# Phase 17: service-rest-uebertrag-cascade - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 17 implementiert die **komplexeste v1.2-Operation**: atomare 2-Action-Cascade `MembershipAdjustService::transfer_shares(from_id, to_id, n, transfer_date, ctx)` mit optionalem 3.-Action-Branch für Voll-Übertrag. Sofort wirksam (kein H1/H2-Stichtag, da kein Geldfluss aus der Genossenschaft). Single-Tx-Pattern mit shared `tx.clone()`, gemeinsamer Process-String `"member-adjust.transfer"` für AUDT-02-Verlinkung.

**In scope:**

- **Trait-Erweiterung `MembershipAdjustService::transfer_shares`** in `genossi_service/src/membership_adjust.rs`. Signatur: `async fn transfer_shares(&self, from_id: Uuid, to_id: Uuid, shares: i64, transfer_date: Date, context: Authentication<Self::Context>, tx: Option<Self::Transaction>) -> Result<(Vec<MemberAction>, Member, Member), ServiceError>`. Inkrementelles Wachsen analog Phase 15 D-15-13.
- **Impl in `genossi_service_impl/src/membership_adjust.rs`** — neue Methode auf bestehendem `MembershipAdjustServiceImpl<Deps>`. TRSF-01..05, TRSF-07 + AUDT-02 + PERM-03.
- **Pure-Function `validate_transfer_inputs(from_id, to_id, n, from_current_shares) -> Vec<ValidationFailureItem>`** — analog Phase 15 D-15-05 `validate_willensbekundung_date`. Bounds: `n >= 1`, `n <= from_current_shares`, `from_id != to_id`. Edge-Case-Tests deterministisch ohne DB-Mock.
- **Cascade-Pipeline (Pre-write-Detection, Single-Tx):**
  1. `permission_service.check_permission(ADMIN_PRIVILEGE, context).await?`
  2. `let tx = transaction_dao.use_transaction(tx).await?`
  3. Pure-Function `validate_transfer_inputs(from_id, to_id, shares, from.current_shares)` → 400 BadRequest bei Fehler
  4. `validate_willensbekundung_date(transfer_date, today())?` (re-use Phase-15-Pure-Function)
  5. Load `from = member_dao.find_by_id(from_id, tx.clone())` → 404 EntityNotFound bei None
  6. Load `to = member_dao.find_by_id(to_id, tx.clone())` → 404 EntityNotFound bei None
  7. PERM-03-Check: `if to.exit_date.is_some() { return Err(ServiceError::Conflict("recipient already cancelled")) }` → 409 Conflict
  8. `let will_become_zero = (from.current_shares - shares == 0)`
  9. `audited_create!(self, member_action_dao, &abgabe_entity, TRANSFER_PROCESS, &user_id, tx.clone())` für `UebertragungAbgabe(A, shares_change=-n, transfer_member_id=Some(B.id), date=transfer_date, effective_date=None)`
  10. `audited_create!(self, member_action_dao, &empfang_entity, TRANSFER_PROCESS, &user_id, tx.clone())` für `UebertragungEmpfang(B, shares_change=+n, transfer_member_id=Some(A.id), date=transfer_date, effective_date=None)`
  11. `audited_update!(self, member_dao, from.id, &from_updated, TRANSFER_PROCESS, &user_id, tx.clone())` — `from.current_shares -= shares`, `from.version = uuid_service.new_v4().await`
  12. `audited_update!(self, member_dao, to.id, &to_updated, TRANSFER_PROCESS, &user_id, tx.clone())` — `to.current_shares += shares`, `to.version = uuid_service.new_v4().await`
  13. **If `will_become_zero`**: `audited_create!(self, member_action_dao, &austritt_entity, TRANSFER_PROCESS, &user_id, tx.clone())` für `Austritt(A, shares_change=0, transfer_member_id=Some(B.id), date=transfer_date, effective_date=Some(transfer_date))`
  14. `recalc_dates(&*member_dao, &*member_action_dao, from.id, tx.clone()).await?` — leitet `from.exit_date` aus dem neuen Austritt ab (nur Voll-Übertrag) bzw. lässt es unverändert (Teil-Übertrag)
  15. `transaction_dao.commit(tx).await?`
- **REST-Endpoint** `POST /api/members/{from_id}/transfer-shares` in `genossi_rest/src/membership_adjust.rs` (existing Datei aus Phase 15) oder `genossi_rest/src/member.rs`. Sub-Route **vor** `/{id}`-catch-all (D-14-08-Lesson). Permission via Auth-Middleware → Service-Layer-Funnel.
- **Request-DTO `TransferSharesRequestTO { to_member_id: Uuid, shares: i64, transfer_date: Date }`** in `genossi_rest_types/src/lib.rs` mit `iso8601_date`-Serde. ToSchema für OpenAPI.
- **Response-DTO**: `{ actions: Vec<MemberActionTO>, from: MemberTO, to: MemberTO }` (anonymes JSON oder benannter `TransferSharesResponseTO`-Wrapper — Planner-Discretion). Frontend (Phase 18) braucht Single-Round-Trip-Update für from + to.
- **DI-Wiring** unverändert — `MembershipAdjustServiceImpl` ist bereits in `RestStateImpl` verdrahtet (Phase 15). Keine neuen Deps.
- **8 E2E-Tests** (Roadmap-SC #5):
  1. Teil-Übertrag Happy-Path (n < from.current_shares, kein Austritt, kein exit_date)
  2. Voll-Übertrag mit exit_date-Cascade (n == from.current_shares, 3. Action + exit_date gesetzt)
  3. Self-Transfer 400 (from_id == to_id → ValidationError)
  4. Empfänger-gekündigt 409 Conflict (PERM-03)
  5. Empfänger-soft-deleted 404 NotFound
  6. Audit-Pair-Verlinkung-Verify: Doppel-Assertion (a) eine transaction_id pro Vorgang, (b) MemberAction-Row-Count = 2 (Teil) / 3 (Voll)
  7. **Race-Test Same-Direction-Parallel** (analog v1.1 Phase 9): 2x identischer POST via `tokio::join!`, sortiert auf `[200, 409|500]`; NIE `[200, 200]` (Double-Cascade)
  8. **Race-Test Cross-Direction-Parallel**: A→B simultan B→A; akzeptiert `[(200, 200), (200, 409|500)]`, aber **nicht** `[409|500, 409|500]` (Total-Deadlock); Post-Check: `A.current_shares + B.current_shares == Start-Summe`, Audit-Chain valid
- **Mock-Unit-Tests** für `MembershipAdjustServiceImpl::transfer_shares` (~10, analog Phase 16):
  - Happy-Path Teil-Übertrag (2 audited_create + 2 audited_update + 1 recalc_dates aufgerufen)
  - Happy-Path Voll-Übertrag (3 audited_create + 2 audited_update + 1 recalc_dates; Austritt-Entity verifiziert mit `effective_date=Some(transfer_date)`, `transfer_member_id=Some(B.id)`)
  - Validation: n<1, n>from.current_shares, self-transfer (alle → ValidationError, kein DAO-Aufruf)
  - PERM-03: Empfänger-cancelled → Conflict, kein audited_*! Aufruf
  - PermissionDenied: kein check_permission-Success → Service-Methode bricht ab vor jedem DAO-Call
  - PaidOut-Exclusion: dev-check, dass current_shares-Update nicht auf bereits-PaidOut-Members trifft (n/a — TRSF nicht im PaidOut-Pfad; trotzdem assert für Documentation)
- **Unit-Tests für `validate_transfer_inputs`** (mindestens 6 Edge-Cases): n=0, n=-1, n=1 valid, n=from.current_shares valid (Voll-Übertrag-Boundary), n>from.current_shares invalid, from_id==to_id invalid.

**Out of scope (deferred / explizit nicht):**

- **Keine Frontend-Integration** — `MembershipAdjustModal` ist Phase 18.
- **Keine v2-Übertrag-an-Antragsteller** — bleibt Seed `transfer-to-applicant.md`, explizit deferred (PROJECT.md Out-of-Scope-Tabelle).
- **Kein `MemberAction::Verkauf`** — v1.1-PaidOut-Cascade-Single-Source-of-Truth; Übertrag hat ohnehin keinen Geldfluss.
- **Kein `RepaymentEntry`-Insert** — Übertrag schreibt nicht in RepaymentPhase.
- **Keine H1/H2-Berechnung** — Übertrag ist sofort wirksam (kein `compute_effective_date`-Aufruf).
- **Keine Storno-Knopf-UI** — bleibt manuelle MemberAction (PROJECT.md Out-of-Scope).

</domain>

<decisions>
## Implementation Decisions

### Voll-Übertrag-Detection (Area 1)

- **D-17-01:** **Pre-write Service-Check für Voll-Übertrag.** Service rechnet vor allen Writes: `will_become_zero = (from.current_shares - shares == 0)`. **Why:** Deterministisch, Mock-Tests können `audited_create!`-Aufrufe für die 3. Action direkt verifizieren (assertion: 2 vs. 3 Action-Creates je nach Branch). Spart einen DAO-Read vs. Post-Write-Re-Read-Variante. Konsistent zum Phase-9-PaidOut-Cascade-Muster (single-tx, branch im Service). **How to apply:** Service-Methode bestimmt `will_become_zero` direkt nach `from`-Load (Schritt 8 der Pipeline); branched bei Schritt 13. Test asserted Branch-Pfad anhand `audited_create!`-Call-Count.

- **D-17-02:** **`recalc_dates` wird genau einmal aufgerufen, nur für `from.id`, am Ende vor `commit`.** **Why:** Nur Member A kann durch Voll-Übertrag exit_date ändern; B bleibt aktiv, kein exit_date-Move. Symmetrisch zu Phase 15 D-15-04 (1x nach Austritt-Create). Bei Teil-Übertrag ist es ein No-Op (kein Austritt-Action in actions-List → exit_date bleibt None), aber für Konsistenz immer aufrufen. **How to apply:** `recalc_dates(&*self.member_dao, &*self.member_action_dao, from.id, tx.clone()).await?` einmal nach allen `audited_*!`-Calls.

- **D-17-03:** **`MemberAction::Austritt(A)` beim Voll-Übertrag bekommt `transfer_member_id = Some(B.id)`.** **Why:** Verlinkt den Austritt mit der Cascade. Audit-Story: bei `/api/audit/verify` + Process-Filter `member-adjust.transfer` sieht man drei verlinkte Einträge — Abgabe(A→B), Empfang(B←A), Austritt(A→B). Klar erkennbar im Audit-Log, dass A NICHT über Phase-15-CANC ausgetreten ist, sondern durch Voll-Übertrag. Divergiert bewusst von Phase 15 CANC-Austritt (dort `transfer_member_id=None`). **How to apply:** Austritt-Entity-Builder: `MemberActionEntity { action_type: ActionType::Austritt, member_id: from.id, shares_change: 0, transfer_member_id: Some(to.id), date: transfer_date, effective_date: Some(transfer_date), ... }`.

### Process-String + Audit-Trail (Area 2)

- **D-17-04:** **Shared Process-String `"member-adjust.transfer"` für ALLE Writes der Cascade** (2x bzw. 3x `audited_create!` + 2x `audited_update!`). **Why:** AUDT-02 verlangt gemeinsamen Process-String für das Pair (Abgabe + Empfang). Wir erweitern bewusst auf das Triple (inkl. optional Austritt) und auf die Member-current_shares-Updates: eine Audit-Story = eine User-Aktion. Filter via `WHERE process = 'member-adjust.transfer'` findet ALLE Writes eines Übertrag-Vorgangs. **How to apply:** `const TRANSFER_PROCESS: &str = "member-adjust.transfer";` am Top von `genossi_service_impl/src/membership_adjust.rs`. Alle Macro-Calls innerhalb `transfer_shares` benutzen diesen Konstanten-String.

- **D-17-05:** **AUDT-02-Test als Doppel-Assertion** — (a) alle Audit-Log-Einträge des Vorgangs teilen sich genau **eine** `transaction_id` (Atomarität); (b) `COUNT(*) FROM audit_log WHERE process='member-adjust.transfer' AND entity_type='MemberAction' AND transaction_id=?` ist genau **2** (Teil-Übertrag) bzw. **3** (Voll-Übertrag). **Why:** Defensive Verifikation der Cascade-Vollständigkeit UND Atomarität. (a) deckt Phase-9-Lesson ab (single-tx-cascade darf nicht in zwei transaction_ids zersplittern), (b) deckt Branch-Korrektheit ab (Voll- vs. Teil-Übertrag-Action-Count). Vorbild: Phase 15 Audit-Chain-Verify-Tests. **How to apply:** Test-Helper-Funktion `assert_transfer_audit_trail(tx_id, expected_action_count: usize)` für Wiederverwendung in mehreren E2E-Tests.

### Race-Pattern (Area 3)

- **D-17-06:** **Zwei separate Race-Tests** in den 8 E2E-Tests aus SC #5:
  - **Race-Test Same-Direction-Parallel** (analog v1.1 Phase 9): 2x identischer `POST /api/members/{A}/transfer-shares` (Body: `{to: B, shares: 2, transfer_date: today}`) via `tokio::join!`. Sortiert auf `[200, 409|500]`. NIE `[200, 200]` (wäre Double-Cascade), NIE `[4xx/5xx, 4xx/5xx]` (wäre Total-Deadlock).
  - **Race-Test Cross-Direction-Parallel**: A→B (shares=2) simultan B→A (shares=2) via `tokio::join!`. Akzeptiert `[(200, 200), (200, 409|500)]` (SQLite kann orthogonale Member-Locks unter Umständen serialisieren), aber **nicht** `[409|500, 409|500]`. Post-Test-Konsistenz-Check: `A.current_shares + B.current_shares` == Start-Summe; `/api/audit/verify` → `valid=true`.
  **Why:** Same-Direction ist der kanonische Race (zwei Vorstand-Klicks auf denselben Button hintereinander). Cross-Direction ist der nicht-triviale Deadlock-Probe (zwei Vorstandsmitglieder, die gleichzeitig verschiedene Übertrags initiieren — selten, aber möglich; muss NICHT in Total-Deadlock münden). **How to apply:** Übernimm `tokio::time::sleep(Duration::from_millis(1)).await` als Pool-Warm-up (Phase-9-Pitfall #11). Status-Vergleich via `let mut statuses = [r_a.status(), r_b.status()]; statuses.sort_by_key(|s| s.as_u16())`.

### Error-Status-Mapping (Area 4)

- **D-17-07:** **PERM-03 (Empfänger gekündigt) → HTTP 409 Conflict.** **Why:** Analog Phase 15 D-15-12 Already-Cancelled-Pattern: Resource-State des Empfängers verhindert die Operation, das ist kein Input-Fehler. Konsistenz mit Phase-15-Audit-Story wichtiger als pixel-genaue ROADMAP-SC-#4-Lesart (die "HTTP 400" sagt — diese Notiz entstand vor der Phase-15-Lesson). Mapping: `ServiceError::Conflict(Arc::from("recipient already cancelled"))` → `RestError::Conflict` → 409. **How to apply:** PERM-03-Check nach Empfänger-Load (Pipeline-Schritt 7); Error-Body trägt Klartext-Hinweis für Frontend-i18n-Lookup.

- **D-17-08:** **Self-Transfer (TRSF-07) → HTTP 400 BadRequest via ValidationError.** **Why:** Self-Transfer ist Input-Fehler (Vorstand hätte das nicht klicken sollen, Frontend-Validierung sollte greifen), nicht Resource-Conflict. Mapping: `validate_transfer_inputs` returns `ValidationFailureItem { field: "to_member_id", message: "cannot transfer to self" }` → `ServiceError::ValidationError(Vec<_>)` → `RestError::BadRequest` → 400. Konsistent mit Phase 15 D-15-08 Pattern. **How to apply:** In `validate_transfer_inputs`-Pure-Function als erste Check.

- **D-17-09:** **Validierung in Pure-Function `validate_transfer_inputs`** (Pipeline-Schritt 3). **Why:** Analog Phase 15 D-15-05 `validate_willensbekundung_date`. Edge-Case-Tests deterministisch ohne DB-Mock. PERM-03 (recipient cancelled) bleibt **separat** als Service-Branch (Pipeline-Schritt 7), weil dafür ein DAO-Read auf `to.exit_date` nötig ist. **How to apply:** Signatur `pub(crate) fn validate_transfer_inputs(from_id: Uuid, to_id: Uuid, shares: i64, from_current_shares: i64) -> Vec<ValidationFailureItem>`. Service ruft sie nach Permission-Check, **vor** DAO-Loads auf. Vorteil: Self-Transfer + n-Range schlagen früher fehl, ohne Members zu laden.

- **D-17-10:** **Error-Reihenfolge / Mapping-Tabelle:**

  | Bedingung | ServiceError | RestError | HTTP |
  |-----------|--------------|-----------|------|
  | Auth fehlt | `Unauthorized` | `Unauthorized` | 401 |
  | Kein ADMIN_PRIVILEGE | `PermissionDenied` | `Unauthorized` (codebase-mapping) oder `BadRequest` | 403 / siehe Phase 15 |
  | `n < 1` oder `n > from.current_shares` oder `from_id == to_id` | `ValidationError(Vec)` | `BadRequest` | 400 |
  | `transfer_date` außerhalb [today.year(), today.year()+1] | `ValidationError(Vec)` | `BadRequest` | 400 |
  | from oder to nicht gefunden / soft-deleted | `EntityNotFound(uuid)` | `NotFound` | 404 |
  | `to.exit_date IS NOT NULL` (PERM-03) | `Conflict(msg)` | `Conflict` | 409 |
  | Optimistic-Locking-Mismatch im audited_update | `Conflict(msg)` | `Conflict` | 409 |
  | SQLITE_BUSY mid-cascade | (DaoError::DatabaseError) → `DataAccess` | `InternalError` | 500 |

  **Why:** Klare Mapping-Tabelle für Planner und E2E-Test-Autoren; deckt alle 8 SC-#5-Tests ab. **How to apply:** Planner bezieht diese Tabelle in jeden Test-Case ein.

### Carry-Forward (locked aus Phase 14/15)

- **C-15-13** → **C-17-CF-01:** Trait `MembershipAdjustService` wächst inkrementell — Phase 17 fügt `transfer_shares` hinzu, kein neues Trait.
- **C-15-01/02** → **C-17-CF-02:** Direkter `audited_create!` (kein Delegieren an `MemberActionService::create`); shared Process-Konstante.
- **C-15-03** → **C-17-CF-03:** `Member.current_shares`-Mutation via generic `MemberDao::update` + `audited_update!`; Version-Bump via `uuid_service.new_v4()`.
- **C-15-04** → **C-17-CF-04:** `recalc_dates` ist bereits Free-Function — Phase 17 nutzt sie ohne weiteren Refactor.
- **C-15-05..08** → **C-17-CF-05:** `validate_willensbekundung_date` Pure-Function wird wiederverwendet für `transfer_date`-Bounds.
- **C-14-08** → **C-17-CF-06:** REST-Sub-Route `POST /api/members/{id}/transfer-shares` registriert **vor** `/{id}`-catch-all.
- **C-15-10/11** → **C-17-CF-07:** Request-DTO in `genossi_rest_types/src/lib.rs` mit ISO8601-Serde; Response-Shape `{actions: Vec<_>, from, to}` für Single-Round-Trip.
- **C-15-15** → **C-17-CF-08:** Return-Tuple `(Vec<MemberAction>, Member, Member)` — domain values, kein DTO-Wrapping im Service.

### Claude's Discretion

- **Response-DTO-Naming**: anonymes JSON `{"actions": [...], "from": {...}, "to": {...}}` oder benannter `TransferSharesResponseTO` in `genossi_rest_types`. Planner-Discretion — bevorzugt benannt für OpenAPI-Klarheit.
- **Handler-Datei-Placement**: weiterhin `genossi_rest/src/membership_adjust.rs` (existing aus Phase 15) erweitern. Falls Datei >600 LOC: Planner darf Split in `membership_adjust/{cancel,partial_repayment,transfer}.rs`-Submodule erwägen.
- **`recalc_migrated`-Aufruf**: Phase 15 D-15-04 hat NUR `recalc_dates` als Free-Function. `recalc_migrated` bleibt private Methode auf `MemberActionServiceImpl`. Phase 17 ruft `recalc_migrated` **NICHT** auf — Übertrag berührt `migrated`-Flag nicht (kein Eintritts-Action involviert). Planner verifiziert.
- **Test-Setup-Helper für PaidOut-Member als from**: Edge-Case "from hat bereits einen `RepaymentEntry::PaidOut`, also reduzierte current_shares" — TRSF-04 sagt nur `current_shares -= n`. Planner darf Mini-Test ergänzen, der absichert, dass eine durch v1.1 reduzierte `current_shares` korrekt als Basis für n-Validation dient.
- **OpenAPI-Annotationen**: Utoipa `#[utoipa::path(...)]` mit 200, 400, 401, 403, 404, 409, 500. Klar dokumentierte Response-Shapes pro Status.
- **`audited_*!`-Macro-Reihenfolge**: Pipeline-Schritte 9–13 in der Reihenfolge Abgabe → Empfang → from-Update → to-Update → optional Austritt. Planner darf Reihenfolge minimal umstellen (z.B. Abgabe → from-Update → Empfang → to-Update), solange Atomarität in einer Tx und finale recalc_dates(from) erhalten bleibt.
- **`tokio::time::sleep(1ms)` Pool-Warm-up vor `tokio::join!`** (v1.1 Phase 9 Pitfall #11) — Planner kopiert dieses Muster für beide Race-Tests.
- **`busy_timeout` PRAGMA** — Genossi-DB-Pool nutzt SQLite-Default. Falls Race-Tests intermittierend rot werden, könnte Planner PRAGMA setzen — bleibt aber Phase-17-out-of-scope falls grün ohne Anpassung.

### Folded Todos

None — die 2 pending Todos (`backend-pre-flight-check-attach-repayment-letter.md`, `frontend-bulk-no-repayment-letter-action.md`) sind v1.1-Tech-Debt und gehören **nicht** in Phase-17-Scope (Übertrag betrifft keine RepaymentEntry-Erzeugung).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Projekt-Foundation

- `.planning/PROJECT.md` — v1.2-Milestone, Constraints (Layered DAO/Service/REST, Audit-Pflicht, ADMIN_PRIVILEGE für v1.2-Ops, Single-Tx-Pattern, Übertrag-Konsistenz-Story mit Voll-Übertrag-Austritt).
- `.planning/REQUIREMENTS.md` §TRSF-01..05, §TRSF-07, §AUDT-02, §PERM-03 — Phase-17-Requirements.
- `.planning/ROADMAP.md` §Phase 17 — Phase-Goal, 4 Success-Criteria, 8 E2E-Test-Liste.
- `.planning/ROADMAP.md` §Constraints (Phase 17 spezifisch) — Single-Tx-Pattern mit shared `tx.clone()`, shared Process-String, Pre-Tx-Rollback-Test.
- `.planning/ROADMAP.md` §Discuss-Phase-Decisions §Phase 17 — Voll-Übertrag-Detection + Race-Pattern-Test (beide hier in D-17-01..03 + D-17-06 geklärt).

### Domain & Architektur

- `.planning/notes/membership-adjust-design.md` — Master-Design-Doc: "Voll-Übertrag an Mitglied = sofortiger Austritt; Austrittsdatum wird automatisch auf das Übertragsdatum gesetzt"; "H1/H2-Regel gilt nur wenn Genossenschaft Geld auszahlt — Übertrag fließt nichts aus der Kasse → sofort wirksam".
- `.planning/research/ARCHITECTURE.md` §1 (Placement-Decision `genossi_service_impl/src/membership_adjust.rs`), §7 (Permission-Funnel `ADMIN_PRIVILEGE`).
- `.planning/research/PITFALLS.md` §Kat 5/6 (Audit-Macro-Wiring, Cascade-Atomarität).

### Phase-15+16-Decisions (carried forward, hier wiederverwendet)

- `.planning/phases/15-service-rest-kuendigung-aufstockung/15-CONTEXT.md` — D-15-01..16 vollständig. Insbesondere:
  - D-15-01 (direkter audited_create vs. MemberActionService::create-Delegation)
  - D-15-02 (Process-String-Konvention `member-adjust.X`)
  - D-15-03 (Member-Update via generic + audited_update!)
  - D-15-04 (`recalc_dates` als Free-Function)
  - D-15-05..08 (validate_willensbekundung_date Pure-Function)
  - D-15-09..12 (REST-Sub-Route-Shape, Status-Codes incl. 409 für Conflict)
  - D-15-13 (Trait wächst inkrementell)
- `.planning/phases/16-service-rest-teil-rueckgabe-auto-anlegen-phase/16-CONTEXT.md` — D-16-X Single-Tx-Cascade-Pattern, Sum-Check + audited-Calls vor commit. Pattern für Pre-write-Detection (analog D-17-01).

### Vorbild-Phasen (Pattern-Quelle)

- `.planning/milestones/v1.1-phases/09-paid-out-cascade-toggle/` — Atomare Multi-DAO-Cascade in einer Tx mit shared Process-String. **Vorbild für D-17-04** und Race-Test-Pattern (D-17-06).
- `genossi_bin/tests/e2e_tests.rs:12474-12555` (Phase 9 D-12 Race-Test) — wortgenaues Vorbild für Same-Direction-Race-Test.

### Code-Referenzen (Files, die berührt werden)

- `genossi_service/src/membership_adjust.rs` — bestehende Trait-Datei (Phase 15). **Erweitern:** Trait-Methode `transfer_shares(from_id, to_id, shares, transfer_date, context, tx)`.
- `genossi_service/src/lib.rs` — keine Änderung (Modul bereits registriert).
- `genossi_service_impl/src/membership_adjust.rs` — bestehende Impl-Datei (Phase 14 angelegt, Phase 15/16 erweitert). **Erweitern:**
  - `validate_transfer_inputs` Pure-Function (D-17-09)
  - `const TRANSFER_PROCESS: &str = "member-adjust.transfer";` (D-17-04)
  - `MembershipAdjustServiceImpl::transfer_shares` Methode mit 15-Schritt-Pipeline
  - `#[cfg(test)] mod tests` für validate_transfer_inputs Edge-Cases + Mock-Service-Tests
- `genossi_service_impl/src/member_action.rs:155` (`compute_dates`) — Auto-Derive von `exit_date` aus `Austritt.effective_date.unwrap_or(date)`. **Wichtig:** Voll-Übertrag-Austritt setzt `effective_date = Some(transfer_date)` (D-17-03), also exit_date = transfer_date.
- `genossi_service_impl/src/member_action.rs:184` (`recalc_dates` Free-Function) — wiederverwendet (kein Refactor mehr nötig).
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!` + `audited_update!` (kein Anpassungsbedarf).
- `genossi_dao/src/member_action.rs` — `ActionType::UebertragungAbgabe`, `UebertragungEmpfang`, `Austritt` existieren bereits. `MemberActionEntity.transfer_member_id: Option<Uuid>` existiert.
- `genossi_dao/src/member.rs:111` (`update`) — generischer Update-Pfad für current_shares-Mutation via audited_update!.
- `genossi_service/src/permission.rs:28` (`ADMIN_PRIVILEGE = "admin"`) — Permission-Konstante.
- `genossi_service/ServiceError` — Variants `ValidationError`, `Conflict`, `EntityNotFound`, `PermissionDenied`.
- `genossi_rest/src/member.rs:28-74` (`generate_route`) — Sub-Route `/transfer-shares` MUSS vor `/{id}` registriert werden (D-14-08-Lesson).
- `genossi_rest/src/membership_adjust.rs` — existierende Datei aus Phase 15. **Erweitern:** Handler `post_transfer_shares(State<RestState>, Extension<Context>, Path<Uuid>, Json<TransferSharesRequestTO>)`.
- `genossi_rest/src/error.rs` — `RestError::Conflict` → 409, `RestError::BadRequest` → 400, `RestError::NotFound` → 404 (existing Mapping; D-17-10 verlässt sich darauf).
- `genossi_rest_types/src/lib.rs` — **neue DTOs:** `TransferSharesRequestTO`, optional `TransferSharesResponseTO`.
- `genossi_rest_types/src/lib.rs:10` (`iso8601_date`-Modul) — wiederverwendet für `transfer_date`.
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — keine Änderung (Service ist bereits verdrahtet aus Phase 15).
- `genossi_bin/tests/e2e_tests.rs` — **neue 8 E2E-Tests** für TRSF-01..05 + TRSF-07 + AUDT-02 + PERM-03 + Race-Patterns. Vorbild für Race-Tests: Z.12474-12555 (Phase 9 D-12).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`MembershipAdjustServiceImpl<Deps>`** (existing aus Phase 15+16): bereits verdrahtet in `RestStateImpl`, mit allen benötigten DAOs (member, member_action, audit_log, permission, uuid, transaction). Phase 17 fügt nur eine weitere Methode hinzu — kein DI-Wiring-Change.
- **`validate_willensbekundung_date`** (`genossi_service_impl/src/membership_adjust.rs`, Phase 15 D-15-05): Pure-Function für Datum-Bounds. Phase 17 ruft sie für `transfer_date` auf (selbe Bounds: aktuelles + nächstes Kalenderjahr).
- **`recalc_dates`** (`genossi_service_impl/src/member_action.rs:184`, Free-Function): liest member + actions, ruft `compute_dates` → setzt `update_dates(join_date, exit_date)`. Phase 17 ruft sie für `from.id` auf — sie sieht die neue Austritt-Action (bei Voll-Übertrag) und setzt `from.exit_date`.
- **`compute_dates`** (`genossi_service_impl/src/member_action.rs:155`): findet `Austritt`-Action und nutzt `effective_date.unwrap_or(date)`. Phase 17 Austritt setzt beides auf `transfer_date` → exit_date = transfer_date deterministisch.
- **`audited_create!` + `audited_update!` Macros** (`genossi_service_impl/src/audit_macros.rs`): erwarten `self.audit_log_dao` + `self.uuid_service`. Phase 17 nutzt sie für 2x bzw. 3x Action-Create + 2x Member-Update mit shared Process-String.
- **`ActionType::UebertragungAbgabe` + `UebertragungEmpfang`** (`genossi_dao/src/member_action.rs`): Varianten existieren bereits, kein Enum-Erweitern.
- **`MemberActionEntity.transfer_member_id: Option<Uuid>`**: Field existiert bereits — für D-17-03 ohne Migration nutzbar.
- **ISO8601-Date-Serde** (`genossi_rest_types/src/lib.rs:10`): wiederverwendet für `transfer_date`.
- **`MemberTO` + `MemberActionTO`**: bestehende TOs für Response-Komposition.
- **`error_handler`** (`genossi_rest/src/error.rs`): REST-Wrapper für `Result<Response, RestError>`.
- **v1.1 Phase 9 Race-Test-Pattern** (`genossi_bin/tests/e2e_tests.rs:12474-12555`): `tokio::join!` mit Mini-Sleep-Pool-Warm-up, sortiertes Status-Tupel, NIE-Klauseln für Double-Cascade.

### Established Patterns

- **`audited_*!`-Macro-Compliance** (AUDT-01-Grep-Gate aus v1.2-Constraints): 0 direkte DAO-create/update-Calls außerhalb der Macros in v1.2-Code. Phase 17 nutzt für ALLE Writes Macros.
- **Permission-Funnel am Service-Methoden-Eintritt**: `permission_service.check_permission(ADMIN_PRIVILEGE, context).await?` als erste Op.
- **Transaction-Lifecycle**: `let tx = transaction_dao.use_transaction(tx).await?` am Start, `transaction_dao.commit(tx).await?` am Ende. Alle DAO-Calls dazwischen mit `tx.clone()`.
- **Soft-Delete-Filter**: `find_by_id` returnt `Option` (None = nicht-gefunden ODER soft-deleted); Service mapped → 404.
- **Pure-Function-Konvention `pub(crate)` + `#[cfg(test)] mod tests`**: für `validate_transfer_inputs`.
- **`Vec<ValidationFailureItem>`**: für Field-Level-Validation-Errors.
- **`ServiceError → RestError → HTTP-Status`-Mapping** (siehe D-17-10).
- **Single-Tx-Cascade-Pattern**: Phase 9 PaidOut-Cascade als Vorbild — alle Writes in einer Tx mit shared Process-String, commit am Ende.

### Integration Points

- **REST-Mount**: Sub-Route `POST /api/members/{id}/transfer-shares` in `member::generate_route` (oder `membership_adjust::extend_member_routes`), **vor** `/{id}`-catch-all (D-14-08-Lesson).
- **Service-Layer-Wiring**: keine Änderung — `membership_adjust_service`-Slot existiert seit Phase 15.
- **OpenAPI**: Utoipa-Annotationen analog v1.1 Phase 13; Schemas für `TransferSharesRequestTO` (+ optional `TransferSharesResponseTO`).
- **Audit-Layer**: `audit_log_dao` ist bereits in `MembershipAdjustServiceDeps` verdrahtet (Phase 15).
- **Test-Server**: `genossi_rest/src/test_server.rs::start_test_server` für E2E; In-Memory-DB-Setup wie v1.1-Phase-9 (für Race-Tests genug Pool-Connections).

</code_context>

<specifics>
## Specific Ideas

- **Audit-Process-String-Naming**: `member-adjust.transfer` (Konsistente Dot-Hierarchy: `cancel`, `upgrade`, `partial-repayment`, `transfer`).
- **Voll-Übertrag-Austritt-Action-Fields**:
  ```rust
  MemberActionEntity {
      id: uuid_service.new_v4().await,
      member_id: from.id,
      action_type: ActionType::Austritt,
      shares_change: 0,             // Austritt-Konvention (Phase 15 CANC analog)
      transfer_member_id: Some(to.id),  // D-17-03
      date: transfer_date,
      effective_date: Some(transfer_date),  // TRSF-05 — sofort wirksam, kein H1/H2
      ...
  }
  ```
- **Edge-Case-Tests für `validate_transfer_inputs`**:
  - `test_validate_transfer_n_zero_invalid` (n=0 → ValidationError)
  - `test_validate_transfer_n_negative_invalid` (n=-1 → ValidationError)
  - `test_validate_transfer_n_equal_current_shares_valid` (n=5, from.current_shares=5 → Voll-Übertrag valid)
  - `test_validate_transfer_n_exceeds_current_shares_invalid` (n=6, from.current_shares=5 → ValidationError)
  - `test_validate_transfer_self_invalid` (from_id == to_id → ValidationError)
  - `test_validate_transfer_n_one_valid` (n=1, from.current_shares=5 → Teil-Übertrag valid)
- **E2E-Test-Naming-Konvention** (analog Phase 15):
  - `test_transfer_shares_partial_happy_path`
  - `test_transfer_shares_full_with_exit_date_cascade`
  - `test_transfer_shares_self_transfer_400`
  - `test_transfer_shares_recipient_cancelled_409`
  - `test_transfer_shares_recipient_not_found_404`
  - `test_transfer_shares_audit_pair_verify_doppel_assertion`
  - `test_transfer_shares_race_same_direction_sqlite_busy`
  - `test_transfer_shares_race_cross_direction_consistency_check`
- **Test-Doppel-Assertion-Helper**:
  ```rust
  fn assert_transfer_audit_trail(
      pool: &SqlitePool, 
      tx_id: Uuid, 
      expected_action_count: usize,  // 2 für Teil, 3 für Voll
  ) {
      // (a) Atomarität: alle Rows teilen sich tx_id
      // (b) MemberAction-Rows zählen: COUNT(*) WHERE entity_type='MemberAction' AND tx_id=?
  }
  ```
- **Race-Test-Pool-Setup**: kopiere `tokio::time::sleep(Duration::from_millis(1)).await` als Mini-Warm-up.
- **Cross-Direction-Konsistenz-Check** (Test #8):
  ```rust
  let total_after = a_after.current_shares + b_after.current_shares;
  assert_eq!(total_after, a_start + b_start, "Cross-Race: Anteile-Summe muss erhalten bleiben");
  let verify = client.get("/api/audit/verify").send().await.unwrap().json::<VerifyResponse>().await.unwrap();
  assert!(verify.valid, "Cross-Race: Audit-Hashchain muss valid bleiben");
  ```

</specifics>

<deferred>
## Deferred Ideas

- **`MembershipAdjustService` Voll-Verschmelzung mit `MemberActionService`** — beide Services haben Audit-Cascade-Pattern; in einer v2-Refaktor-Phase könnten sie konsolidiert werden. Phase 17 erweitert nur inkrementell, kein Refactor.
- **Storno-Knopf für Übertrag** — bleibt manuelle MemberAction (PROJECT.md Out-of-Scope für v1.2).
- **Bulk-Übertrag (mehrere n auf mehrere Empfänger atomar)** — kein User-Case in v1.2, könnte v2 sein.
- **Voll-Übertrag-Austritt mit `transfer_member_id = None`** als zukünftiger Vereinheitlichungs-Pfad falls Audit-Trail-Linking via shared Process-String + tx_id ausreicht und das Field nicht nötig ist. Heute D-17-03 Some(B.id) → kann später revisited werden.
- **`busy_timeout` PRAGMA setzen** — falls Race-Tests intermittierend rot werden. Heute bleibt Default; nur if-needed.
- **`recalc_migrated` Free-Function-Refactor** — Phase 15 D-15-04 hat es absichtlich nicht gemacht. Phase 17 braucht es nicht (Übertrag berührt `migrated`-Flag nicht). Bleibt deferred bis ein Service es wirklich braucht.
- **Frontend-Vorschau-Dialog für Voll-Übertrag-Edge-Case** — Phase 18; UI muss anzeigen "Voll-Übertrag: A wird austreten am DD.MM.YYYY".

</deferred>

---

*Phase: 17-service-rest-uebertrag-cascade*
*Context gathered: 2026-06-06*
