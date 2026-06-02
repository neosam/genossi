# Phase 9: Auszahlungs-Buchung (atomisch + auditiert) - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 9 liefert die atomare, audit-konsistente Auszahlungs-Buchung für `RepaymentEntry`. Der bisher in Phase 8 blockierte Status-Übergang nach `PaidOut` wird über einen dedizierten Action-Endpoint exposed und erzeugt in genau einer SQLite-Transaktion drei verkettete Schreibvorgänge: einen neuen `MemberAction::Verkauf` mit `shares_change = -share_count_to_pay_out`, eine `Member.current_shares`-Reduktion um `share_count_to_pay_out` plus `action_count += 1`, und die finale `RepaymentEntry.status = PaidOut`-Setzung. Race-sicher gegen `tokio::join!`-Parallelaufrufe via Entry-Status-Guard und SQLite-Tx-Serialisierung. Final per Phase 8 D-05 (Toggle-Back über PUT/Batch bereits in Phase 8 blockiert).

**In scope:**
- Neue `mark_paid_out`-Methode auf `RepaymentEntryService`-Trait + `RepaymentEntryServiceImpl`
- `RepaymentEntryServiceImpl`-Deps-Erweiterung um `MemberActionDao` (zusätzlich zu bestehendem `MemberDao`)
- REST-Endpoint `POST /api/repayment-entry/{id}/mark-paid-out` (kein Body, kein Version-Body-Feld) — Phase-7-D-02/D-03-Pattern für Action-Endpoints
- Drei `audited_*!`-Calls in einer SQLite-Tx, alle mit gemeinsamem `process = "repayment-entry.mark-paid-out"`:
  1. `audited_create!` für `MemberActionEntity { action_type: Verkauf, shares_change: -N, date: today, comment: "Anteils-Rückzahlung Phase {fiscal_year}", effective_date: None, transfer_member_id: None }`
  2. `audited_update!` für `MemberEntity { current_shares: old - N, action_count: old + 1, ... }`
  3. `audited_update!` für `RepaymentEntryEntity { status: PaidOut, ... }`
- Pre-Conditions (alle in der Tx, alle mit eindeutigen HTTP-Mappings):
  - Entry existiert & nicht soft-deleted → sonst 404
  - Entry.status ∈ {Open, Contacted} → sonst 409 (PaidOut → "already paid out, final per PAYO-04")
  - Phase.status == Open (Defense-in-Depth) → sonst 409
  - Member.current_shares ≥ Entry.share_count_to_pay_out (PAYO-03) → sonst `ServiceError::ValidationError`
- Post-Cascade: expliziter `recalc_migrated`-Call (Pattern-Konsistenz mit `MemberActionServiceImpl::create` und `MemberServiceImpl::update`)
- Re-Read nach `audited_update!`-Calls (Phase 8 BL-01-Pattern): bei `None` → `ServiceError::InternalError` → HTTP 500, NICHT `EntityNotFound`→404
- OpenAPI/Utoipa-Schema-Doku für den Endpoint mit allen Status-Codes (200/400/404/409/500)
- E2E-Tests:
  - Happy-Path: Open-Entry → 200, Member.current_shares reduziert, MemberAction::Verkauf vorhanden, Entry.status=PaidOut, Audit-Chain via `/api/audit/verify` valide
  - Negative-Path PAYO-03: ValidationError wenn `current_shares < share_count_to_pay_out`
  - Negative-Path PAYO-04: Toggle-Back über mark_paid_out auf PaidOut-Entry → 409
  - Phase-Status-Guard: mark_paid_out auf Entry einer Preparation/Closed-Phase → 409
  - Race-Test (`tokio::join!`): zwei parallele mark_paid_out auf demselben Entry → genau einer Erfolg, einer Conflict
- Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()`: `MemberActionDao` an `RepaymentEntryServiceImpl`-Deps anhängen

**Out of scope (gehört in Phase 10-12 oder explizit nicht gewollt):**
- Batch-`mark_paid_out`-Endpoint — bewusst deferred zu Phase 12 (siehe `<deferred>`)
- Vorstand-Input für `MemberAction.date` oder `comment` — vollständig auto, kein Body
- `RepaymentPhase`-Auto-Close wenn letzter Entry PaidOut wird — nicht im SC; Vorstand schließt manuell über `POST /api/repayment-phase/{id}/close`
- Audit-Macro-Erweiterung für gemeinsame `transaction_id`-UUID — Phase-8-D-03-Pragma reicht für SC #3
- Reverse-Transition `PaidOut → Open/Contacted` — Phase 8 D-05 bereits blockiert; keine zusätzliche Logik nötig
- Frontend-Komponenten / Confirm-Dialog UI-05 → Phase 12
- Massenmail-Anbindung → Phase 10

</domain>

<decisions>
## Implementation Decisions

### Audit-Gruppierung (PAYO-01..03, ROADMAP SC #3)

- **D-01:** **Phase-8-D-03-Pragma übernehmen.** Drei `audited_*!`-Calls (einmal `audited_create!` für MemberAction, zweimal `audited_update!` für Member + RepaymentEntry) laufen in EINER SQLite-Tx. Alle drei verwenden denselben `process`-String: `const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT: &str = "repayment-entry.mark-paid-out"`. SC #3 wird interpretiert als „Identifikation als ein Geschäftsvorfall via (process + same-tx-commit + sequentielle Hash-Chain)" — NICHT als buchstäblich gemeinsamer `transaction_id`-UUID-Wert. Begründung: Phase 8 D-03 hat das Pattern bereits etabliert (Auto-Fill), die Hash-Chain beweist Atomarität und Reihenfolge, kein konkreter Audit-Query-Use-Case fordert UUID-Gleichheit. Macro-Erweiterung `audited_*_with_tx_id!` bleibt als deferred Idee bestehen, falls echter Bedarf auftaucht.

### REST-Endpoint-Schema (PAYO-01, UI-05 Backend-Anker)

- **D-02:** REST-Pfad: `POST /api/repayment-entry/{id}/mark-paid-out` (Action-Endpoint, Phase-7-D-02-Pattern für Lifecycle-Transitions). Singular `/repayment-entry`-Konvention (Phase 7 D-14, Phase 8 D-09).
- **D-03:** **Kein Request-Body, kein Version-Body-Feld.** Pattern aus Phase 7 D-03 (Open/Close-Endpoints): Concurrency-Defense läuft über Entry-Status-Guard. Vorstand-Input ist explizit nicht vorgesehen (siehe D-04).
- **D-04:** **MemberAction-Felder vollautomatisch:**
  - `action_type = ActionType::Verkauf` (PAYO-01)
  - `shares_change = -(entry.share_count_to_pay_out)` (PAYO-01)
  - `date = OffsetDateTime::now_utc().date()` (server-set, kein Backdating in v1.1)
  - `comment = "Anteils-Rückzahlung Phase {fiscal_year}"` (mit `phase.fiscal_year` substituiert)
  - `effective_date = None` (validate_action erlaubt es nur für Austritt — `member_action.rs:140-145`)
  - `transfer_member_id = None`
  - `id`, `version` via `uuid_service.new_v4()`; `created = now()`
- **D-05:** **Response-Body:** aktualisierter `RepaymentEntryTO` (status=PaidOut + neue version aus Re-Read). KEIN `MemberTO`, KEIN `MemberActionTO` in der Response — Phase 12 ruft bei Bedarf separat ab. Hält das Response-Schema schmal.
- **D-06:** **HTTP-Status-Codes** (alle in OpenAPI-Doc):
  - `200 OK` + RepaymentEntryTO — Happy Path
  - `400 Bad Request` — `ServiceError::ValidationError` (PAYO-03: `current_shares < share_count_to_pay_out`)
  - `401/403` — OIDC ohne Admin-Privileg
  - `404 Not Found` — Entry nicht existiert oder soft-deleted (vor Re-Read; aggregat-konsistent mit Phase 8 CR-02-Fix)
  - `409 Conflict` — Entry.status nicht Open/Contacted; Phase.status nicht Open
  - `500 Internal Server Error` — Re-Read `None` nach `audited_*!` in same-Tx (Phase 8 BL-01-Pattern)
- **D-07:** **Single-only in Phase 9.** Keine Batch-Variante. Cascade ist sicherheitskritisch (irreversibel + audit-pflichtig + Cross-Entity-Konsistenz); Confirm-Dialog UI-05 ist explizit pro Eintrag konzipiert; SC #5 testet single + Race. Batch wäre Phase-12-Ergänzung falls UI-Flow es will (deferred).

### Cascade-Mechanik (PAYO-01, PAYO-02, PAYO-04)

- **D-08:** **Owner-Service:** `RepaymentEntryServiceImpl::mark_paid_out`. `RepaymentEntryService`-Trait wird um die Methode erweitert. Deps-Erweiterung: `MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao` neben dem bestehenden `MemberDao`. Pattern-konsistent mit Phase 8 D-03 Auto-Fill (Service-zu-Service-Calls bewusst vermieden — direkter DAO-Zugriff via Macros hält die Tx-Atomarität deterministisch und vermeidet Permission-Re-Checks).
- **D-09:** **Cascade-Reihenfolge** (deterministisch, Planner darf abweichen wenn Audit-Argument gut ist):
  1. Tx beginnen (`use_transaction(None)`)
  2. Permission-Check (`ADMIN_PRIVILEGE`)
  3. Load Entry + Status-Guards (Entry.status ∈ {Open, Contacted}, sonst 409)
  4. Load Phase + Status-Guard (Phase.status == Open, sonst 409)
  5. Load Member + Validation (`current_shares >= share_count_to_pay_out`, sonst PAYO-03 ValidationError)
  6. `audited_create!` MemberAction::Verkauf
  7. `audited_update!` Member (current_shares -= N, action_count += 1)
  8. Re-Read Member (BL-01-Pattern, `None` → InternalError → 500)
  9. `audited_update!` RepaymentEntry (status = PaidOut)
  10. Re-Read RepaymentEntry (BL-01-Pattern)
  11. `recalc_migrated` (D-10)
  12. `commit`
- **D-10:** **Expliziter `recalc_migrated`-Call nach Cascade.** Pattern-Konsistenz mit `MemberActionServiceImpl::create` (`member_action.rs:346-348`) und `MemberServiceImpl::update` (`member.rs:341`). Begründung: `compute_migration_status` hat eine subtile `expected_action_count = member.action_count + 1`-Off-by-one-Konvention (semantisch wohl Initial-Migration-Stub-Tracking) — Konsistenz-by-Construction wäre fragil, expliziter Recalc ist Defense-in-Depth + Pattern-Treue. Kosten: 1 `find_by_member_id` + 1 `update_migrated` in der Tx (<1ms bei <100 Actions/Member).
  - **Planner-Discretion:** `compute_migration_status` ist aktuell `pub(crate)` in `genossi_service_impl::member_action`. Optionen: (a) `pub` machen + Inline-Aufruf; (b) `MemberActionService::recalc_migrated`-Trait-Methode + Service-Dep; (c) Logik in `RepaymentEntryServiceImpl` duplizieren. Empfehlung: (a), weil minimal-invasiv und das Verhalten ohnehin Cross-Service-relevant ist.

### Race-Sicherheit (ROADMAP SC #5)

- **D-11:** **Status-Guard ist die primäre Race-Defense.** Beide Parallel-Aufrufe lesen Entry, beide sehen Status=Open. Erste Tx commited (Entry-Update setzt PaidOut + neue version). Zweite Tx hat `audited_update!`-internes `find_by_id` (Macro lädt Entity), Macro liest committed Entry mit Status=PaidOut → tatsächlich wird der Macro-Guard nicht greifen, weil die zweite Tx innerhalb derselben Connection ein konsistentes Snapshot hat... → **wichtige Klarstellung für Researcher/Planner:** SQLite mit WAL serialisiert Schreib-Tx; die zweite Tx wird beim ersten `audited_update!`-Schreibversuch entweder (a) auf einen SQLITE_BUSY laufen (DAO mappt das auf `Conflict`), oder (b) sieht den neuen Status nach Tx-Begin durch READ-Snapshot — beide Pfade enden in 409. Researcher soll prüfen, welches Verhalten der bestehende `transaction_dao_impl_sqlite` tatsächlich produziert (vermutlich SQLITE_BUSY, weil exklusiver Write-Lock).
- **D-12:** **Race-Test im E2E** mit `tokio::join!(mark_paid_out, mark_paid_out)` auf demselben Entry-ID. Erfolgs-Assertion: exakt ein 200 + ein 409, NICHT zweimal 200 und NICHT zweimal 409. Pattern aus Phase 2 HLPR-04 (Helfer-Token-Race) und Phase 1 ASSY-Atomarität.

### Validation-Strategie (PAYO-03)

- **D-13:** `current_shares >= share_count_to_pay_out`-Check lebt in `mark_paid_out` selbst (inline, nach Member-Load). Reuse von Phase-8-`validate_entry_create` (`repayment_entry.rs:67-91`) ist nicht 1:1 passend — dort prüft die Funktion `> 0` UND `<= current_shares`. Phase 9 braucht NUR den `<=`-Teil. Planner darf entweder (a) eine schlanke `validate_payout_shares`-Helper-Funktion einführen, oder (b) inline-Check schreiben.
- **D-14:** `ServiceError::ValidationError(vec![ValidationFailureItem { field: "share_count_to_pay_out", message: "Member.current_shares ({current}) is less than entry.share_count_to_pay_out ({requested})" }])` — REST-Mapping ist 400 Bad Request. Frontend-Toast in UI-05 zeigt die Message.

### Claude's Discretion

- **OpenAPI-Beispielwerte** für mark_paid_out — Planner darf realistische Defaults setzen (z.B. `share_count_to_pay_out: 5`, `payout_amount: 60000` in Cent).
- **Race-Defense-Pfad** (D-11) — Planner verifiziert via Code-Lese, ob Phase-2-Pattern (`UPDATE...RETURNING`) wiederverwendbar ist oder ob SQLITE_BUSY-Konkurrenz das ausreichend abdeckt.
- **`compute_migration_status`-Pub-Machen vs Trait-Methode vs Duplikation** (D-10) — Planner wählt.
- **Reihenfolge der drei `audited_*!`-Calls** (D-09) — Planner darf abweichen, wenn ein Argument gut ist (z.B. „Entry zuerst setzen, damit recalc_migrated den Verkauf bereits sieht"). Empfohlene Reihenfolge MemberAction → Member → Entry ist chronologisch lesbar im Audit-Log.
- **Audit-Log-Sortierung im Test** — beim Audit-Chain-Test darf Planner entscheiden, ob nach `process`-String gefiltert oder nach `transaction_id`-Sequence gegruppt wird.
- **Tests:** Phase 9 sollte Mocks reuse von Phase 8 (`TestRepaymentEntryDao`, `TestMemberDao` aus `repayment_entry.rs:556+`). Neuer Mock `TestMemberActionDao` muss dazu (kann analog zu MemberActionService-Mocks aus `member_action.rs` gebaut werden).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Anforderungen
- `.planning/ROADMAP.md` §"Phase 9: Auszahlungs-Buchung (atomisch + auditiert)" — Goal + 5 Success Criteria (atomic Cascade, Validation, Audit-Chain-Group, Final-Status, Race-Test)
- `.planning/REQUIREMENTS.md` §"Auszahlung" PAYO-01..04 — Anforderungs-IDs vollständig in Phase 9
- `.planning/PROJECT.md` §"Current Milestone: v1.1 Anteile-Rückzahlungsphase" — Auszahlung-Toggle erzeugt MemberAction::Verkauf

### Phase 7/8 Vorgänger (direkter Bauteil-Lieferant)
- `.planning/phases/08-repaymententry-auto-bef-llung/08-CONTEXT.md` — Phase 8 D-03 (Audit-Pragma, von Phase 9 übernommen), D-05 (PaidOut final, Toggle-Back-Blocks bereits da)
- `.planning/phases/08-repaymententry-auto-bef-llung/08-REVIEW.md` BL-01 — Re-Read None → InternalError/500-Pattern (Phase 9 muss dasselbe Pattern verwenden)
- `.planning/phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — D-02 (Lifecycle-Action-Endpoints), D-03 (kein Body, kein Version-Field bei Action-Endpoints), D-05 (409 für ungültige Transitions)

### Code-Anker: Cascade-Owner
- `genossi_service/src/repayment_entry.rs` — `RepaymentEntryService`-Trait; Phase 9 ergänzt `mark_paid_out`-Methode
- `genossi_service_impl/src/repayment_entry.rs:48-58` — `gen_service_impl!`-Block, Phase 9 ergänzt `MemberActionDao`-Dep
- `genossi_service_impl/src/repayment_entry.rs:169-291` — `update_repayment_entry` als Re-Read-Pattern-Anker (BL-01-Fix Z. 277-287)
- `genossi_service_impl/src/repayment_entry.rs:379-512` — `batch_toggle_status` als Multi-Step-Cascade-Pattern (Tx-Atomarität, structured 409, Re-Read-Loop)

### Code-Anker: MemberAction-Erzeugung
- `genossi_dao/src/member_action.rs:8-50` — `ActionType::Verkauf` als Enum-Variante; `as_str`/`from_str`
- `genossi_dao/src/member_action.rs:53-97` — `MemberActionEntity`-Struktur + `Auditable`-Impl (`entity_type="member_action"`, audit_fields umfassen `member_id`, `action_type`, `date`, `shares_change`, `transfer_member_id`, `effective_date`, `comment`)
- `genossi_dao/src/member_action.rs:101-130` — `MemberActionDao`-Trait-Definition (`create`, `update`, `dump_all`); `find_by_member_id` für recalc_migrated
- `genossi_service_impl/src/member_action.rs:91-97` — `validate_action` für `Verkauf`: `shares_change < 0` (validates negativ); Phase 9 muss `shares_change = -N` setzen
- `genossi_service_impl/src/member_action.rs:284-352` — `MemberActionServiceImpl::create`-Vorlage (Pattern für audited_create + recalc_dates + recalc_migrated). Phase 9 bypassed Service-Layer und ruft `audited_create!` direkt; Recalc-Logik wird in mark_paid_out repliziert (D-10)

### Code-Anker: Member-Update + Migration
- `genossi_dao/src/member.rs:80-100` — `MemberEntity`-Felder, insbesondere `current_shares: i32`, `action_count: i32`, `version: Uuid`
- `genossi_dao/src/member.rs:116-122` — `MemberDao::update`-Signatur (entity, process, tx)
- `genossi_dao/src/member.rs:145-150` — `MemberDao::update_migrated` für recalc
- `genossi_service_impl/src/member.rs:295-352` — `MemberServiceImpl::update` als Re-Read-Pattern-Anker; Phase 9 reproduziert Re-Read-Logik
- `genossi_service_impl/src/member_action.rs:32-69` — `compute_migration_status` — aktuell `pub(crate)`, Phase 9 braucht Access (siehe D-10 Planner-Discretion)
- `genossi_service_impl/src/member_action.rs:174-225` — `recalc_dates` und `recalc_migrated` als Vorlagen

### Code-Anker: Audit-Macros + Hash-Chain
- `genossi_service_impl/src/audit_macros.rs:5-36` — `audited_create!` (6 Args)
- `genossi_service_impl/src/audit_macros.rs:42-80` — `audited_update!` (7 Args, lädt old intern)
- `genossi_service_impl/src/audit_log.rs:55-113` — `build_audit_entries`: jeder Aufruf erzeugt EINE neue `transaction_id` via `uuid_fn()` (Z. 65). Phase-8-D-03-Klarstellung bestätigt; Phase 9 D-01 lebt mit dieser Eigenschaft.
- `genossi_dao/src/audit_log.rs:8-23` — `AuditLogEntry`-Felder, `prev_hash` + `entry_hash` für Hash-Chain
- `genossi_dao/src/audit_log.rs:26-33` — `AuditQueryFilter` (kein `transaction_id`-Field aktuell; Phase 9 erweitert das NICHT — siehe `<deferred>`)

### Code-Anker: REST + OpenAPI + Wiring
- `genossi_rest/src/repayment_entry.rs` — Phase 8 REST-Handler-Datei; Phase 9 ergänzt einen neuen Sub-Route `/{id}/mark-paid-out`
- `genossi_rest/src/repayment_phase.rs` — Phase-7-Anker für Action-Endpoints (`/open`, `/close`); strukturell wiederverwendbar für mark-paid-out
- `genossi_rest_types/src/lib.rs` — `RepaymentEntryTO` existiert, Phase 9 reuse für Response; ggf. ein `MarkPaidOutResponse`-Alias oder direkt `RepaymentEntryTO` zurückgeben
- `genossi_rest/src/lib.rs` — Router-Komposition; Phase 9 muss nichts neues mergen (Endpoint kommt in `repayment_entry::generate_route()` dazu)
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — DI-Wiring; Phase 9 verbindet `MemberActionDao` an `RepaymentEntryServiceImpl`

### Testing-Anker
- `genossi_bin/tests/e2e_tests.rs` — E2E-Pattern; Phase 9 ergänzt: Happy-Cascade, PAYO-03-ValidationError, PAYO-04-Double-mark-paid-out (409), Phase-Status-Guard, Race-Test (tokio::join!)
- `genossi_rest/src/test_server.rs` — `start_test_server` Helper
- `genossi_service_impl/src/repayment_entry.rs:515+` — Test-Mock-Setup (TestRepaymentEntryDao, TestMemberDao etc.) als Vorlage für Phase-9-Unit-Tests; neuer `TestMemberActionDao`-Mock nötig
- Phase 2 HLPR-04 (Helfer-Token-Race) — historisches Race-Test-Pattern mit `tokio::join!`, Researcher findet via `genossi_service_impl/src/helper_token*` (Phase 2 v1.0)

### Architektur-Constraints
- `.planning/codebase/ARCHITECTURE.md` — Anti-Patterns (Hard Delete, Manual Hash Chain, Service-creates-its-own-Transaction)
- `CLAUDE.md` §"Audit Log System" — 4-Schritt-Checklist (Phase 9 nutzt bestehende Auditable-Impls von MemberAction, Member, RepaymentEntry; keine neue Auditable-Impl nötig)
- `CLAUDE.md` §"Entity Structure" — UUID/BLOB, ISO8601, optimistic locking
- `.planning/PROJECT.md` §"Constraints" — Audit-Pflicht; Phase 9 ist die zentrale Cross-Entity-Cascade des v1.1-Milestones

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`RepaymentEntryServiceImpl`-Deps-Struktur** (Phase 8): bereits da mit `RepaymentEntryDao`, `RepaymentPhaseDao`, `MemberDao`, `AuditLogDao`, `PermissionService`, `UuidService`, `TransactionDao`. Phase 9 ergänzt EIN Dep: `MemberActionDao`. Der `gen_service_impl!`-Block in `repayment_entry.rs:48-58` ist die einzige Stelle, die wachsen muss.
- **Re-Read-Pattern mit `InternalError`-Mapping** (Phase 8 BL-01-Fix, `repayment_entry.rs:265-291`): exakte Vorlage für die zwei Re-Read-Stellen in mark_paid_out (nach Member-Update, nach Entry-Update). Source-Comments mitkopieren — sie dokumentieren das Same-Tx-Invariant.
- **`audited_*!`-Macros** mit 6/7-Arg-Signatur: keine Anpassung nötig, Phase 9 ruft sie 3x in derselben Methode auf.
- **`MemberAction::Verkauf`-Validation** (`member_action.rs:91-97`): wenn Phase 9 direkt das Entity baut UND `audited_create!` aufruft (statt `MemberActionService::create`), läuft `validate_action` NICHT automatisch — Phase 9 muss sicherstellen, dass `shares_change < 0` korrekt gesetzt ist. Das ist trivial (`-N` mit `N > 0` aus PAYO-03-Validation), aber Researcher sollte das im Kopf haben.
- **`Auditable`-Impls** für MemberAction (`member_action.rs:67-97`), Member (`member.rs:198-251`), RepaymentEntry (Phase 8): vorhanden, Phase 9 nutzt sie automatisch über die Macros.

### Established Patterns

- **Service-Layer-direct-DAO-Access für Cross-Entity-Cascades** (Phase 8 D-03 Auto-Fill in `open_phase`): vermeidet Service-zu-Service-Calls, hält Tx-Atomarität deterministisch, vermeidet doppelte Permission-Checks. Phase 9 folgt diesem Pattern.
- **Action-Endpoint ohne Body für Lifecycle-Transitions** (Phase 7 D-02/D-03): `POST /api/.../{id}/open`, `/close` — Phase 9 erweitert um `/mark-paid-out`.
- **Re-Read-nach-`audited_update!`** mit `InternalError`-Fallback (Phase 8 BL-01): einheitliches Pattern an allen Schreibstellen.
- **Race-Test via `tokio::join!`** (Phase 2 HLPR-04): zwei parallele Service-Calls, Assert genau ein Erfolg + ein Conflict.
- **OpenAPI-Annotation mit allen Status-Codes** (Phase 8 `repayment_entry.rs` REST-Handler): `200`, `400`, `404`, `409`, `500` doc-strings + Schema-Referenz.

### Integration Points

- **`genossi_service/src/repayment_entry.rs`** — Trait `RepaymentEntryService` wird um `async fn mark_paid_out(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<RepaymentEntry, ServiceError>` erweitert.
- **`genossi_service_impl/src/repayment_entry.rs`** — `gen_service_impl!`-Block bekommt neue Dep `MemberActionDao`; `RepaymentEntryService for RepaymentEntryServiceImpl<Deps>`-Impl bekommt `mark_paid_out`-Implementierung; neuer `const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT: &str`.
- **`genossi_rest/src/repayment_entry.rs`** — neuer Sub-Route `/{id}/mark-paid-out` per POST registriert via `.route(...)` in `generate_route()`; neuer `#[utoipa::path]`-Handler.
- **`genossi_rest_types/src/lib.rs`** — keine neuen TOs nötig (Response = bestehender `RepaymentEntryTO`).
- **`genossi_bin/src/lib.rs::RestStateImpl::new()`** — `RepaymentEntryServiceImpl::new(...)`-Aufruf bekommt `member_action_dao.clone()` als zusätzliches Argument.
- **`MockRepaymentEntryService`** (in `repayment_entry.rs` automocks): mockall generiert die `mark_paid_out`-Mock automatisch; Tests in REST-Layer können das nutzen.

</code_context>

<specifics>
## Specific Ideas

- **Auto-Comment-Format `"Anteils-Rückzahlung Phase {fiscal_year}"`** (D-04): konsistent mit der semantischen Namensgebung des Vorgangs; Audit-Reader sieht direkt den Bezug zur Phase. Optional kann der Planner stattdessen den Phase-Namen / Geschäftsjahres-Bereich verwenden, falls das in der Verbandsakte üblich ist.
- **`MemberAction.date = today`** (D-04): pragmatischer Standard. In v1.1 wird die Cascade direkt vom Vorstand ausgelöst, kurz nachdem die Bank-Überweisung läuft — der Unterschied zwischen „Buchungsdatum" und „Auszahlungs-Tag" beträgt typischerweise <24h. Wenn das in der Verbands-Realität problematisch wird, wird ein optionaler `date`-Body-Parameter nachgezogen (deferred).
- **Defense-in-Depth `Phase.status == Open`-Check** (D-09 Schritt 4): kostet einen `find_by_id`, schützt aber gegen DB-direkte Manipulation und macht die Pre-Condition explizit. Verbandskonform sauber: „Auszahlung nur in offener Phase".
- **`MemberAction::Verkauf` ohne `transfer_member_id`/`effective_date`**: Verkauf an die Genossenschaft selbst, kein Mitglied-zu-Mitglied-Transfer. Audit-Trail bleibt klar.

</specifics>

<deferred>
## Deferred Ideas

- **Batch-`mark_paid_out`-Endpoint** (`POST /api/repayment-entry/batch-mark-paid-out`) — bewusst nicht in Phase 9; UI-05 Confirm-Dialog ist pro Entry konzipiert, Cascade ist sicherheitskritisch. Phase 12 oder eine spätere Phase kann nachziehen, falls UI-Flow Bulk-Bestätigung verlangt.
- **Vorstand-Input für `MemberAction.comment` / `date`** — Body `{ comment?, date? }` kann nachgezogen werden, wenn Auto-Comment in der Praxis zu generisch ist oder Backdating nötig wird.
- **Audit-Macro-Erweiterung für gemeinsame `transaction_id`-UUID** (`audited_*_with_tx_id!`) — wenn echter Audit-Query-Use-Case auftaucht (z.B. Verband-Prüfung will „zeig mir alle Audit-Einträge dieses einen Vorgangs"). Additive Macro-Variante, kein Breaking-Change. SC #3 wird in Phase 9 ohne diese Erweiterung erfüllt.
- **`AuditQueryFilter.transaction_id`** + REST-Endpoint `GET /api/audit/transaction/{tx_id}` — passt zur Macro-Erweiterung oben.
- **Auto-Close der Phase wenn letzter Entry PaidOut wird** — bewusst NICHT; Vorstand schließt manuell, weil er ggf. nachträgliche Einträge noch hinzufügen will (Soft-Delete + Manual-Add via Phase 8 ENTR-02).
- **`MemberAction::Verkauf`-Updates / Korrekturen** — Phase 8 D-05 macht `PaidOut`-Entry final; Verkauf-Action ist über bestehenden `MemberActionService` editierbar, aber das ist Vorstand-manual-Korrektur, kein neuer Endpoint in Phase 9.

### Reviewed Todos (not folded)

Keine — `gsd-sdk query todo.match-phase 9` lieferte 0 Matches.

</deferred>

---

*Phase: 9-auszahlungs-buchung-atomisch-auditiert*
*Context gathered: 2026-05-31*
