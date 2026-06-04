# Phase 16: Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 16 erweitert den `MembershipAdjustService` (Phase 15 etabliert) um die **Teil-Rückgabe-Operation** — multi-step Service-Layer-Operation, die in der via H1/H2-Stichtag berechneten Ziel-`RepaymentPhase` einen `RepaymentEntry` mit Status `Open` erzeugt. Bei fehlender Ziel-Phase wird sie **automatisch in Status `Open`** angelegt; ein neues **Auto-Fill-Skip-Pattern** in v1.1's `open_repayment_phase` (Z. 319–395) verhindert Duplikate mit der späteren Phase-Open-Auto-Befüllung. Erfüllt PART-01..06 + PITFALLS-Kategorie-1 (Doppelbuchungs-Prävention) + PITFALLS-Kategorie-2 (Variante B).

**In scope:**
- **Trait-Erweiterung** `MembershipAdjustService::partial_repayment(member_id: Uuid, shares: i64, willensbekundung_date: Date, context, tx) -> Result<(RepaymentEntry, Member, Option<RepaymentPhase>), ServiceError>` in `genossi_service/src/membership_adjust.rs` (D-15-13 inkrementelles Wachstum). Member-Daten unverändert zurück (PART-06: keine `current_shares`-Reduktion), `Option<RepaymentPhase>` nur bei Auto-Anlegen gesetzt.
- **Impl** in `genossi_service_impl/src/membership_adjust.rs` — Permission-Funnel `ADMIN_PRIVILEGE`, dann `validate_willensbekundung_date` (Phase 15 wiederverwenden), Range-Validation `1 <= shares < member.current_shares`, gekündigt-Block (`exit_date IS NOT NULL` → HTTP 409), Auto-Anlegen-Ziel-Phase via `RepaymentPhaseService::create_repayment_phase`, Sum-Check, `audited_create!(RepaymentEntry)`. Alles in einer Tx.
- **Audit-Process-String** `const PARTIAL_REPAYMENT_PROCESS: &str = "member-adjust.partial-repayment"` (folgt D-15-02-Konvention).
- **Neue DAO-Query** `RepaymentEntryDao::find_by_member_and_phase(member_id: Uuid, phase_id: Uuid, tx) -> Result<Vec<RepaymentEntryEntity>, DaoError>` — Filter `deleted IS NULL` (default). Foundation für **Sum-Check** (Service summiert in Code) UND **Auto-Fill-Skip-Pattern** (Existenz-Check).
- **Auto-Fill-Skip-Pattern** Erweiterung in `genossi_service_impl/src/repayment_phase.rs:319–395` (`open_repayment_phase`-Auto-Fill-Loop): pro Member-Iteration ein `find_by_member_and_phase`-Call; wenn `non-empty` → skip diesen Member (statt zusätzlichen Entry erzeugen). Pattern verhindert Duplikate mit v1.2-erzeugten Entries.
- **Default-Konstante** `const DEFAULT_SHARE_VALUE_CENT: i64 = 10000;` in `membership_adjust.rs` — Fallback bei Auto-Anlegen, wenn keine Vorgänger-RepaymentPhase existiert.
- **REST-Endpoint** `POST /api/members/{id}/partial-repayment` (Sub-Route vor `/{id}`-catch-all, D-14-08-Lesson). Handler-Datei: `genossi_rest/src/membership_adjust.rs` erweitern (von Phase 15 angelegt).
- **Request-DTO** `PartialRepaymentRequestTO { willensbekundung_date: Date, shares: i64 }` in `genossi_rest_types/src/lib.rs` (ISO8601-Date-Serde wiederverwenden).
- **Response-Body** `{ entry: RepaymentEntryTO, member: MemberTO, phase: Option<RepaymentPhaseTO> }` — `phase` nur bei Auto-Anlegen befüllt, sonst null. Frontend (Phase 18) kann Hinweis "Phase für FY 2027 wurde automatisch angelegt" anzeigen.
- **DI-Wiring** in `genossi_bin/src/lib.rs::RestStateImpl::new()`: `MembershipAdjustServiceImpl` bekommt neue Dependency `repayment_phase_service` (für Auto-Anlegen-Delegation) UND `repayment_entry_dao` (für Sum-Check + Skip-Lookup). DI-Map via `gen_service_impl!`.
- **HTTP-Status-Codes**: 200 (Success), 400 (Validation: shares <= 0, shares >= current_shares = Voll-Rückgabe-Block, willensbekundung_date out-of-bounds), 401 (No Auth + No Admin — siehe Hinweis unten), 404 (Member not found), 409 (Conflict: gekündigtes Mitglied, oder keine Vorgänger-Phase + Default-Fallback wurde NICHT gewählt — siehe D-16-06).
  - **403/401-Reclassification (post-discuss-phase, plan-phase-Resolution):** Phase 15 hat in D-15-12 entschieden, dass `ServiceError::PermissionDenied` global auf `RestError::Unauthorized` → HTTP 401 mappt (nicht 403). Phase 16 folgt dieser Konvention; OpenAPI-Annotationen listen kein 403. Wenn künftige Phasen ein dediziertes 403 brauchen, muss D-15-12 zuerst überarbeitet werden.
- **8 E2E-Tests** (ersetzt Roadmap-Test #5 'Phase-not-existent-without-auto-create' durch zwei Variante-B-Tests) + **8 Service-Unit-Tests**.

**Out of scope (deferred / explizit nicht):**
- **Keine `MemberAction::Verkauf`** (PART-06, PITFALLS Z. 11–16) — v1.1-PaidOut-Cascade erzeugt sie beim Ausbezahlt-Toggle.
- **Keine `Member.current_shares`-Reduktion** (PART-06) — PaidOut-Cascade reduziert beim Ausbezahlt-Toggle.
- **Kein `recalc_dates`/`recalc_migrated`-Aufruf** (PITFALLS-Kat-10) — Teil-Rückgabe erzeugt KEINE MemberAction.
- **Kein Übertrag** (Phase 17), **kein Voll-Rückgabe-Pfad** (auf cancel_membership verweisen, D-16-11).
- **Keine UI** (Frontend ist Phase 18).
- **Kein Variante-A (Vorbereitung) oder Variante-C (Explicit Error)** — bewusst gegen die anderen Auto-Anlegen-Strategien entschieden (D-16-01).
- **Keine D-11.1-Guard-Aufweichung** — Phase-Open-Status bleibt einzige Bedingung für `create_repayment_entry`. Variante B macht das obsolet.
- **Keine Member-spezifische Locking** (PITFALLS-Kat-6: weiterhin optimistic via `version`-UUID).
- **Kein neuer `update_current_shares`-DAO-Targeted-Call** — `current_shares` wird in Phase 16 nicht angefasst.

</domain>

<decisions>
## Implementation Decisions

### Auto-Anlegen-Strategie

- **D-16-01:** **Variante B — Auto-Create in Status `Open` + Auto-Fill-Skip-Pattern.** Wenn `RepaymentPhase` für berechneten `fiscal_year` nicht existiert, wird sie via `RepaymentPhaseService::create_repayment_phase` mit Status `Open` angelegt. Doppelbuchung mit dem späteren v1.1-Phase-Open-Auto-Fill wird durch das Skip-Pattern verhindert. **Why:** PITFALLS-Empfehlung; D-11.1-Status-Guard bleibt unangetastet (keine v1.1-Invarianten-Berührung); Direkt-Insert funktioniert sofort ohne zweistufigen Vorstand-Workflow; das Skip-Pattern wird ohnehin für Success-Criterion-4 gebraucht — eine Foundation, zwei Use-Cases. Varianten A und C bewusst verworfen (siehe Out-of-Scope). **How to apply:** Im Service: `let phase = match self.repayment_phase_dao.find_by_fiscal_year(target_year, tx).await { Some(p) => p, None => self.repayment_phase_service.create_repayment_phase(submission, context, tx).await? };` (Pseudocode — Planner adaptiert auf existing DAO-Signatures).

- **D-16-02:** **Phase-Auto-Create via existing `RepaymentPhaseService::create_repayment_phase`** (Delegation, nicht direkter DAO+`audited_create!`). **Why:** Existing Service-Methode kapselt Audit-Macro, Validierung (`share_value > 0`), Permission-Check und korrekten Audit-Process-String. Direkt-DAO würde Validierungs-Logik duplizieren und Layered-Architektur brechen. **How to apply:** `MembershipAdjustServiceImpl` bekommt neue Dependency `repayment_phase_service: Arc<dyn RepaymentPhaseService<...>>` via `gen_service_impl!`. Aufruf in `partial_repayment` mit demselben `context` (User durchläuft beide Permission-Funnels, was OK ist — beide verlangen `ADMIN_PRIVILEGE`).

- **D-16-03:** **Auto-Fill-Skip-Lookup per-Member im Loop** (nicht Bulk-Prefetch). In v1.1's `open_repayment_phase`-Auto-Fill-Loop (Z. 319–395) wird pro iterierten Member ein `repayment_entry_dao.find_by_member_and_phase(member_id, phase_id, tx)`-Call gemacht; wenn `non-empty` → `continue` (skip Member, kein neuer Entry). **Why:** N+1 wäre bei Massenanwendung problematisch, aber GV-typische Genossi-Größe ist <200 Members; Konsistenz mit existing Loop-Pattern; vermeidet Bulk-State-Tracking-Komplexität. Foundation `find_by_member_and_phase`-DAO-Query existiert sowieso für Sum-Check (D-16-08), also keine zusätzliche Maintenance. **How to apply:** Innerhalb der existing Loop in `repayment_phase.rs:319–395` direkt vor dem `audited_create!(RepaymentEntry)`-Call den Skip-Check einfügen. Inline-Comment auf den Anchor (D-16-03 + PART-04 + PITFALLS-Kat-1).

- **D-16-04:** **Single-Tx atomar für Phase-Auto-Create + Entry-Create.** Beide Operationen teilen sich denselben `tx`-Handle. Wenn der Entry-Create fehlschlägt (z.B. Sum-Check-Verletzung oder DAO-Conflict), wird die auto-angelegte Phase mit-rollbacked. **Why:** Konsistenter State garantiert, kein verwaister Phase-Eintrag im Audit-Log möglich. Analog v1.1-Phase-9-Multi-DAO-Cascade-Pattern. **How to apply:** `let tx = self.transaction_dao.use_transaction(tx).await?;` am Eintritt; sowohl `repayment_phase_service.create_repayment_phase(...)` als auch `audited_create!(self, self.repayment_entry_dao, ...)` mit `tx.clone()`; finaler `self.transaction_dao.commit(tx).await?` am Ende.

### share_value bei Auto-Anlegen

- **D-16-05:** **`share_value` aus letzter existierender `RepaymentPhase` übernehmen** (unabhängig vom Status). Lookup-Strategie: Planner wählt zwischen `repayment_phase_dao.dump_all()` + Sort-by-fiscal_year-descending (existing Default-Impl) oder einer neuen targeted Query `find_latest_by_fiscal_year`. **Why:** In der Praxis ist `share_value` über Jahre konstant; Genossenschaftsverband-Konvention. Vorgänger-Phase-Übernahme reflektiert die letzte vom Vorstand beschlossene Höhe. Vorstand kann nach Auto-Anlegen die neue Phase im "Vorbereitung"-Stadium editieren (existing v1.1-UI), falls Wert abweicht. **How to apply:** Service-Methode-interner Helper `fn resolve_share_value(latest_phase: Option<RepaymentPhaseEntity>) -> i64` — bei `Some` nimm `.share_value`, bei `None` `DEFAULT_SHARE_VALUE_CENT`.

- **D-16-06:** **Fallback bei keiner Vorgänger-RepaymentPhase: hardcoded Default.** Edge-Case: in v1.1-frischem System ohne RepaymentPhase wird der Default genutzt; HTTP 200 (kein 409). **Why:** In der produktiven Genossi-Installation ist v1.1 bereits ausgerollt und Phasen existieren — der Edge-Case trifft realistisch nie zu. Hardcoded Default macht den Test-Path determiniert und vermeidet doppel-stufige Fehlerbehandlung im Frontend. Vorstand sieht im Audit-Log die neue Phase und kann den Wert nachträglich korrigieren. **How to apply:** `let share_value = latest_phase.map(|p| p.share_value).unwrap_or(DEFAULT_SHARE_VALUE_CENT);`. Inline-Comment auf D-16-06 + Hinweis, dass Vorstand den Wert nach Auto-Anlegen prüfen soll.

- **D-16-07:** **`DEFAULT_SHARE_VALUE_CENT = 10000`** (= 100 EUR pro Anteil). Konstante in `genossi_service_impl/src/membership_adjust.rs` neben `PARTIAL_REPAYMENT_PROCESS`. **Why:** Standardwert vieler Genossenschaften; entspricht der typischen Genossi-Konfiguration. **How to apply:** `pub(crate) const DEFAULT_SHARE_VALUE_CENT: i64 = 10000;` am Top des Moduls. Inline-Doc-Comment mit Bezug zu D-16-06.

### Sum-Check

- **D-16-08:** **Service-Layer-Sum-Check mit `find_by_member_and_phase`**. Neue DAO-Query auf `RepaymentEntryDao`: `find_by_member_and_phase(member_id: Uuid, phase_id: Uuid, tx) -> Result<Vec<RepaymentEntryEntity>, DaoError>`. Filter im DAO: `deleted IS NULL` (default). Service filtert in Code zusätzlich auf `status != PaidOut`, summiert `share_count_to_pay_out`, prüft `sum + new.shares <= member.current_shares` — bei Verletzung `ServiceError::ValidationError → HTTP 400`. **Why:** PITFALLS-Prevention-Strategy explizit empfohlen; eine DAO-Query liefert Foundation für BEIDE Use-Cases (Sum-Check + Auto-Fill-Skip); Default-Impl-Pattern konsistent (kein targeted SUM-Aggregat); Service-Code testbar mit Mock-DAOs. **How to apply:** `let existing = self.repayment_entry_dao.find_by_member_and_phase(member_id, phase.id, tx.clone()).await?; let sum: i64 = existing.iter().filter(|e| e.status != RepaymentEntryStatus::PaidOut).map(|e| e.share_count_to_pay_out).sum(); if sum + shares > member.current_shares { return Err(ServiceError::ValidationError(vec![...])); }`. Field: `"shares"`, message: `format!("sum of open repayments ({}) plus new ({}) exceeds current_shares ({})", sum, shares, member.current_shares)`.

- **D-16-09:** **Status-Filter: `status != PaidOut`** (nicht nur `status == Open`). **Why:** PaidOut-Entries haben über v1.1-PaidOut-Cascade bereits `current_shares` reduziert — sie sind also NICHT mehr im `current_shares`-Counter enthalten. Wenn wir sie trotzdem in die Summe einbeziehen würden, käme es zu fälschlichem Block bei legitimen Folge-Operationen. Open + alle anderen Nicht-Final-Status (falls v1.1 weitere hinzufügt) zählen. **How to apply:** Service-Layer-Filter: `existing.iter().filter(|e| e.status != RepaymentEntryStatus::PaidOut)`. Planner verifiziert die existing `RepaymentEntryStatus`-Variants in `genossi_dao/src/repayment_entry.rs`.

### Edge-Cases & Validierung

- **D-16-10:** **Gekündigtes Mitglied (`exit_date IS NOT NULL`) → HTTP 409 Conflict, blocken.** Service-Methode prüft nach `find_by_id(member_id)` direkt nach Permission-Check: `if member.exit_date.is_some() { return Err(ServiceError::Conflict(...)); }`. Fehlertext: `"Cannot start partial repayment for cancelled member (exit_date={})"`. **Why:** Gekündigte Members gehen via v1.1-PaidOut-Cascade in die nächste Auszahlungsphase — separater Workflow. Zusätzliche Teil-Rückgabe würde Doppelbuchungs-Risiko erhöhen. Konsistent mit Phase-15-UPGD-04-Pattern (Aufstockung blockt gekündigte Members). **How to apply:** Block ist der erste Member-State-Check nach `find_by_id` und VOR `validate_willensbekundung_date` / Sum-Check.

- **D-16-11:** **Voll-Rückgabe (`shares == current_shares`) → HTTP 400 Bad Request, blocken.** Range-Validation greift; Fehlertext: `"shares must be strictly less than current_shares — use cancel_membership for full return"`. **Why:** Voll-Rückgabe ohne Kündigung ist semantisch verwirrend (Mitglied bleibt aktiv mit 0 Anteilen). Klarere Audit-Story: Voll-Rückgabe = Austritt via `cancel_membership`. **How to apply:** Im Range-Check `if shares >= member.current_shares { return Err(ServiceError::ValidationError(vec![ValidationFailureItem { field: "shares", message: "..." }])); }`.

- **D-16-12:** **Range-Validation: `1 <= shares < current_shares` (strikt).** Bei `shares <= 0` HTTP 400 ("shares must be at least 1"), bei `shares >= current_shares` HTTP 400 (siehe D-16-11). Validation läuft im Service-Layer VOR Sum-Check und VOR Auto-Anlegen. **Why:** Klare Pre-Conditions → klare Fehlermeldungen; trennt billige Range-Checks von teureren DAO-Queries. **How to apply:** Helper-Funktion `validate_partial_repayment_shares(shares: i64, current_shares: i64) -> Result<(), Vec<ValidationFailureItem>>` als Pure-Function (testbar), analog `validate_willensbekundung_date`.

### Implizite Decisions (carry-forward aus Phase 14/15)

- **D-16-13:** **Audit-Process-String `"member-adjust.partial-repayment"`** (`const PARTIAL_REPAYMENT_PROCESS`). Folgt D-15-02-Konvention. Dot-Hierarchy `member-adjust.*` macht Audit-Log-Filter `WHERE process LIKE 'member-adjust.%'` möglich. **Auto-Anlegen-Audit-Trail:** die separat erzeugte `RepaymentPhase` bekommt den Process-String des `RepaymentPhaseService::create_repayment_phase` (eigener Service-internal-String); separate Audit-Einträge sind erwartet und semantisch korrekt (zwei Operationen — Phase angelegt + Entry angelegt — werden zwei separate Audit-Transactions).

- **D-16-14:** **REST-Endpoint `POST /api/members/{id}/partial-repayment`** (Sub-Route, Pattern D-15-09). MUSS **vor** `/{id}` in `member::generate_route` registriert werden (D-14-08-Lesson).

- **D-16-15:** **Request-DTO `PartialRepaymentRequestTO { willensbekundung_date: Date, shares: i64 }`** in `genossi_rest_types/src/lib.rs`. ISO8601-Date-Serde wiederverwenden (existing Modul). `#[derive(Debug, Serialize, Deserialize, ToSchema)]`.

- **D-16-16:** **Response-Body `{ entry: RepaymentEntryTO, member: MemberTO, phase: Option<RepaymentPhaseTO> }`**. `phase` nur befüllt, wenn Auto-Anlegen passierte. Frontend (Phase 18) zeigt Hinweis "Phase für FY YYYY automatisch angelegt". Wenn Phase bereits existierte: `phase: null`. **Planner-Discretion:** anonymes JSON-Struct oder benannter `PartialRepaymentResponseTO`-Typ in `genossi_rest_types`.

- **D-16-17:** **Trait wächst inkrementell** um `partial_repayment` (D-15-13). Nach Phase 16 hat `MembershipAdjustService` drei Methoden: `cancel_membership`, `increase_shares`, `partial_repayment`. Phase 17 fügt `transfer_shares` hinzu.

- **D-16-18:** **`validate_willensbekundung_date` (Phase 15) wird wiederverwendet** — die Datum-Bounds-Pure-Function aus D-15-05..08 deckt Phase 16 automatisch ab (heutiges Kalenderjahr + nächstes Kalenderjahr).

- **D-16-19:** **KEINE direkte MemberAction-Erzeugung, KEINE `Member.current_shares`-Mutation** (PART-06, PITFALLS-Linie). v1.1-PaidOut-Cascade übernimmt das beim späteren Ausbezahlt-Toggle. Konsequenz: `recalc_dates`/`recalc_migrated`-Hooks werden NICHT aufgerufen (PITFALLS-Kat-10).

### Claude's Discretion

- **`find_by_member_and_phase`-DAO-Methode-Lokation:** auf `RepaymentEntryDao` (Trait + SQLite-Impl). Standard-Default-Impl wäre `dump_all` + Filter — aber für Performance hier eine echte targeted Query mit SQL `WHERE member_id = ? AND phase_id = ? AND deleted IS NULL`.
- **`share_value`-Lookup-Mechanik:** Planner wählt zwischen `repayment_phase_dao.dump_all()` + Sort vs. neue targeted Query `find_latest_by_fiscal_year`. Beide funktionieren; targeted ist effizienter, aber dump_all reicht für realistic Phasen-Anzahl (<10).
- **Handler-Datei-Placement:** `genossi_rest/src/membership_adjust.rs` erweitern (Phase 15 sollte sie angelegt haben — falls nicht, dann neu). Falls > 600 LOC: in `member.rs` verschieben oder Sub-Module.
- **Plan-File-Aufteilung:** Planner-Discretion. Empfehlung: `plan_01_dao_query_and_trait.md` (neue DAO-Query + Trait-Erweiterung), `plan_02_partial_repayment_impl.md` (Service-Impl + Unit-Tests), `plan_03_autofill_skip_pattern.md` (open_repayment_phase-Erweiterung), `plan_04_rest_endpoint_and_e2e.md` (REST + DI-Wiring + E2E).
- **Response-DTO-Naming:** anonymes `serde_json::json!({...})` oder benannter `PartialRepaymentResponseTO`. Benannt für OpenAPI-Schema bevorzugt.
- **Auto-Anlegen-Reihenfolge im Code:** Planner darf entscheiden, ob `find_by_fiscal_year` + Fallback-`create_repayment_phase` als `match`/`if let` strukturiert oder als Helper-Methode `ensure_repayment_phase(target_year, tx) -> Result<(RepaymentPhase, bool /* was_created */), ServiceError>` extrahiert wird.
- **Permission-Doppel-Check:** `MembershipAdjustServiceImpl::partial_repayment` ruft `permission_service.check_permission(ADMIN_PRIVILEGE, context)?` am Eintritt; danach delegierter `repayment_phase_service.create_repayment_phase(...)` macht denselben Check erneut. Zwei Permission-Checks ist OK (gleiche Berechtigung), Planner darf nicht versuchen, den zweiten zu umgehen — Service-Boundary-Sauberkeit.
- **E2E-Tests:** Roadmap-Test #5 `Phase-not-existent-without-auto-create-Fallback` wird durch Variante B obsolet — ersetzen durch zwei Variante-B-Tests:
  - `test_partial_repayment_auto_creates_phase_with_previous_share_value` (Vorgänger-Phase existiert → übernehmen)
  - `test_partial_repayment_auto_creates_phase_with_default_share_value` (keine Vorgänger-Phase → `DEFAULT_SHARE_VALUE_CENT`)

### Folded Todos

None — `gsd-sdk query todo.match-phase 16` ergab keine Matches.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Projekt-Foundation
- `.planning/PROJECT.md` — v1.2-Milestone, Layered DAO/Service/REST-Architecture-Constraint, Audit-Pflicht für Member/MemberAction/MemberDocument/Application (Teil-Rückgabe erzeugt keine MemberAction, also keine Auditable-Trait-Implementation nötig), ADMIN_PRIVILEGE für v1.2-Ops.
- `.planning/REQUIREMENTS.md` §PART-01..06 — Phase-16-Requirements (1..current_shares, H1/H2-Stichtag, RepaymentEntry-Erzeugung, Sum-Check, Auto-Anlegen, KEINE MemberAction/current_shares-Mutation).
- `.planning/ROADMAP.md` §Phase 16 — Goal, 5 Success-Criteria, Plans-Liste.
- `.planning/ROADMAP.md` §Constraints (Phase 15+16+17) — `audited_*!`-Macro-Grep-Gate (0 direkte DAO-create/update außerhalb Macros).
- `.planning/ROADMAP.md` §Discuss-Phase-Decisions §Phase 16 — Auto-Anlegen-Strategie (gelöst: D-16-01 = B), Sum-Check-Service vs. DAO-Query (gelöst: D-16-08 = Service-Layer mit find_by_member_and_phase).

### Domain & Architektur
- `.planning/notes/membership-adjust-design.md` — Master-Design-Doc; **Achtung**: Z. 23 ("Teil-Rückgabe → MemberAction (−n)") ist obsolet/widerspruchsvoll — PITFALLS und ROADMAP überschreiben mit PART-06-Linie (kein MemberAction, kein current_shares).
- `.planning/research/PITFALLS.md` §Kat 1 — Doppelbuchung Auto-Fill + v1.2 (Prevention via Auto-Fill-Skip-Pattern, D-16-03 + find_by_member_and_phase-DAO-Query).
- `.planning/research/PITFALLS.md` §Kat 2 — Auto-Anlegen-Strategie A/B/C; Variante B (Open + Auto-Fill-Skip) gewählt → D-16-01.
- `.planning/research/PITFALLS.md` §Kat 4 — H1/H2-Edge-Cases (durch `compute_effective_date` aus Phase 14 abgedeckt; `validate_willensbekundung_date` Phase 15 deckt Kalenderjahr-Bounds ab).
- `.planning/research/PITFALLS.md` §Kat 6 — Optimistic Locking `current_shares`-Race (in Phase 16 NICHT akut, weil PART-06 current_shares unverändert lässt; relevant erst bei Phase 17/PaidOut-Cascade).
- `.planning/research/PITFALLS.md` §Kat 9 — SQLITE_BUSY in v1.2-E2E-Tests (E2E-Pool mit `busy_timeout(5000)` analog v1.1-Phase-9).
- `.planning/research/PITFALLS.md` §Kat 10 — `recalc_migrated`-Konsistenz: für Teil-Rückgabe IRRELEVANT (keine MemberAction → kein Aufruf).

### Phase-Vorgänger-Decisions (carried forward)
- `.planning/phases/14-dao-domain-foundation/14-CONTEXT.md` — Phase-14-Decisions (compute_effective_date Pure-Function, ADMIN_PRIVILEGE, ISO8601-Date-Serde, Sub-Routes-vor-/{id}-Lesson D-14-08).
- `.planning/phases/15-service-rest-kuendigung-aufstockung/15-CONTEXT.md` — Phase-15-Decisions vollständig: D-15-01..16, insbesondere D-15-02 (Audit-Process-String-Konvention `"member-adjust.*"`), D-15-09 (Sub-Routes), D-15-11 (Response-Body-Shape), D-15-13 (Trait wächst inkrementell), D-15-14 (Trait-Datei genossi_service/src/membership_adjust.rs).

### Vorbild-Phasen aus v1.1 (Pattern-Quelle)
- `.planning/milestones/v1.1-phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — RepaymentPhase-Service-Foundation (Pattern für Service-Delegation in D-16-02).
- `.planning/milestones/v1.1-phases/09-paid-out-cascade-toggle/` — Multi-DAO-Atomare-Tx-Pattern (Vorbild für D-16-04 Single-Tx; gemeinsamer Process-String — nicht relevant hier, weil zwei semantisch verschiedene Operationen).
- `.planning/milestones/v1.1-phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-CONTEXT.md` — Permission-Funnel-Pattern, E2E-Test-Setup-Vorbild.

### Code-Referenzen (Files, die berührt werden)
- `genossi_service/src/membership_adjust.rs` — **erweitern** (Phase 15 hat die Datei mit zwei Methoden angelegt; Phase 16 fügt `partial_repayment` hinzu).
- `genossi_service_impl/src/membership_adjust.rs` — **erweitern** (`PARTIAL_REPAYMENT_PROCESS`-Const, `DEFAULT_SHARE_VALUE_CENT`-Const, `partial_repayment`-Method-Impl, `validate_partial_repayment_shares`-Pure-Helper).
- `genossi_service_impl/src/repayment_phase.rs:319-395` — **erweitern** (Auto-Fill-Loop um Skip-Check via `find_by_member_and_phase`, D-16-03).
- `genossi_service_impl/src/repayment_entry.rs:101-170` (`create_repayment_entry`) — **NICHT anfassen** (Status-Guard `Phase.status == Open` bleibt; D-11.1 unangetastet).
- `genossi_service_impl/src/repayment_entry.rs:517-723` (`mark_paid_out`) — **NICHT anfassen** (PaidOut-Cascade übernimmt MemberAction-Verkauf + current_shares-Reduktion bei Toggle, nicht jetzt).
- `genossi_dao/src/repayment_entry.rs` — **erweitern** (Trait `RepaymentEntryDao` um `find_by_member_and_phase(member_id, phase_id, tx) -> Vec<RepaymentEntryEntity>`).
- `genossi_dao_impl_sqlite/src/repayment_entry.rs` — **erweitern** (SQL-Impl `SELECT * FROM repayment_entries WHERE member_id = ? AND phase_id = ? AND deleted IS NULL`).
- `genossi_dao/src/repayment_phase.rs` — `find_latest_by_fiscal_year` evtl. ergänzen (Planner-Discretion) oder `dump_all` + Sort nutzen.
- `genossi_rest_types/src/lib.rs` — `PartialRepaymentRequestTO` + optional `PartialRepaymentResponseTO`. ISO8601-Date-Serde wiederverwenden (`iso8601_date`-Modul).
- `genossi_rest/src/membership_adjust.rs` (Phase 15 angelegt) ODER `genossi_rest/src/member.rs` — `partial_repayment`-Handler hinzufügen.
- `genossi_rest/src/member.rs:28` (`generate_route`) — neue Sub-Route `/partial-repayment` **vor** `/{id}` registrieren (D-14-08).
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — `MembershipAdjustServiceImpl` bekommt neue Dependencies `repayment_phase_service` + `repayment_entry_dao`.
- `genossi_service/src/permission.rs:28` (`ADMIN_PRIVILEGE = "admin"`) — Permission-Konstante.
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!`-Macro (kein Anpassungsbedarf).
- `genossi_dao/src/repayment_entry.rs` (`RepaymentEntryStatus`-Enum) — Status-Variants verifizieren (Status != PaidOut, D-16-09).
- `genossi_bin/tests/` — neue E2E-Test-Datei oder Erweiterung der existing v1.2-Test-Datei.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`compute_effective_date` Pure-Function** (`genossi_service_impl/src/membership_adjust.rs:21`, Phase 14): liefert `EffectiveDate { fiscal_year, effective_date }` — Phase 16 nutzt `fiscal_year` für Ziel-Phase-Lookup.
- **`validate_willensbekundung_date` Pure-Function** (Phase 15, D-15-05..08): Datum-Bounds-Check (heutiges + nächstes Kalenderjahr). Phase 16 ruft sie unverändert auf — keine Re-Implementation nötig.
- **`MembershipAdjustService` Trait** (`genossi_service/src/membership_adjust.rs`, Phase 15): zwei existing Methoden (cancel_membership, increase_shares). Phase 16 erweitert um `partial_repayment`.
- **`MembershipAdjustServiceImpl<Deps>`** (`genossi_service_impl/src/membership_adjust.rs`, Phase 15): existing Service-Impl mit DI über `gen_service_impl!`. Phase 16 fügt zwei Dependencies hinzu (`repayment_phase_service`, `repayment_entry_dao`).
- **`RepaymentPhaseService::create_repayment_phase`** (`genossi_service_impl/src/repayment_phase.rs:94`): existing Service-Methode für Phase-Anlegen. Phase 16 delegiert für Auto-Anlegen (D-16-02).
- **`audited_create!` Macro** (`genossi_service_impl/src/audit_macros.rs`): RepaymentEntry-Audit-Logging unverändert; erwartet `self` mit `audit_log_dao` + `uuid_service`-Feldern.
- **`RepaymentPhaseDao`** (`genossi_dao/src/repayment_phase.rs`): existing `find_by_fiscal_year` (verifizieren!) oder `dump_all` für share_value-Lookup. Planner prüft existing Query-Set.
- **ISO8601-Date-Serde** (`genossi_rest_types/src/lib.rs:10`): wiederverwendbar für `PartialRepaymentRequestTO.willensbekundung_date`.
- **`error_handler`** (`genossi_rest/src/error.rs`): REST-Wrapper für `Result<Response, RestError>`.

### Established Patterns
- **`audited_*!`-Macro-Compliance**: AUDT-01 Grep-Gate; 0 direkte DAO-create/update außerhalb Macros.
- **Permission-Funnel am Service-Methoden-Eintritt**: `permission_service.check_permission(ADMIN_PRIVILEGE, context).await?` (D-14-11 + D-15-01).
- **Transaction-Lifecycle**: `use_transaction → all DAO calls with tx.clone() → commit`. Single Tx atomar (D-16-04).
- **Soft-Delete-Filter `deleted IS NULL`**: DAO-Layer-Default; neue `find_by_member_and_phase`-Query befolgt das Pattern.
- **Pure-Function-Konvention `pub(crate)` + `#[cfg(test)] mod tests`**: für Helpers (`validate_partial_repayment_shares`).
- **`Vec<ValidationFailureItem>`** für Field-Level-Validation-Errors.
- **`ServiceError → RestError → HTTP-Status`-Mapping**: ValidationError → 400, Conflict → 409, EntityNotFound → 404, PermissionDenied → 403, Unauthorized → 401.
- **Sub-Routes vor `/{id}`-Catch-All** (D-14-08-Lesson): `partial-repayment` MUSS vor `/{id}` in `member::generate_route` registriert werden.
- **Audit-Process-String-Dot-Hierarchy** `member-adjust.partial-repayment` (D-15-02).

### Integration Points
- **REST-Mount** unverändert (`lib.rs:582` `.nest("/api/members", member::generate_route())`); neue Sub-Route innerhalb `generate_route`.
- **Service-Layer-Wiring**: existing `membership_adjust_service`-Slot in `RestStateImpl` (Phase 15 hat ihn angelegt); Phase 16 erweitert nur die Dependencies via `gen_service_impl!`-Macro.
- **OpenAPI**: Utoipa-Annotationen für Endpoint mit allen Status-Codes (200, 400, 401, 403, 404, 409).
- **Audit-Layer**: `audit_log_dao` ist schon in `MembershipAdjustServiceDeps` (Phase 15); kein neuer Wiring nötig.
- **Test-Server**: `genossi_rest/src/test_server.rs::start_test_server` für E2E-Tests; In-Memory-DB-Setup wie Phase 15.
- **Pool-Setup für E2E**: `busy_timeout(5000)` analog v1.1-Phase-9 (PITFALLS-Kat-9-Mitigation).

</code_context>

<specifics>
## Specific Ideas

- **Audit-Process-String**: `const PARTIAL_REPAYMENT_PROCESS: &str = "member-adjust.partial-repayment";` (D-16-13).
- **Default-Konstante**: `pub(crate) const DEFAULT_SHARE_VALUE_CENT: i64 = 10000;` (D-16-07; Kommentar: "Fallback für Auto-Anlegen-Phase wenn keine Vorgänger-RepaymentPhase existiert; entspricht 100 EUR pro Anteil — Standardwert in Genossi-Installationen").
- **E2E-Test-Liste (Anpassung an Variante B):**
  - `test_partial_repayment_happy_path_h1` — Willensbekundung im H1 (z.B. 2026-03-15), Ziel-Phase 2026 existiert, Entry erzeugt.
  - `test_partial_repayment_happy_path_h2_with_auto_create_phase` — Willensbekundung im H2 (z.B. 2026-11-15), Ziel-Phase 2027 existiert NICHT, Auto-Anlegen + Entry, Response enthält `phase: { ... }`.
  - `test_partial_repayment_sum_check_block_400` — Member hat schon Open-Entry mit X Anteilen, neue Teil-Rückgabe würde Sum > current_shares ergeben → HTTP 400.
  - `test_partial_repayment_auto_fill_skip_after_v12` — Nach v1.2-Teilrückgabe (Entry existiert in Phase), Phase-Open-Auto-Fill in v1.1 erzeugt KEINEN Duplikat-Entry für denselben Member.
  - `test_partial_repayment_full_return_block_400` — `shares == current_shares` → HTTP 400 mit Hinweis auf cancel_membership.
  - `test_partial_repayment_cancelled_member_block_409` — gekündigtes Mitglied → HTTP 409.
  - `test_partial_repayment_audit_chain_verify` — `/api/audit/verify.valid == true` nach Teil-Rückgabe; Audit-Log enthält `process="member-adjust.partial-repayment"`-Eintrag.
  - `test_partial_repayment_auto_creates_phase_with_default_share_value` — keine Vorgänger-Phase im System → Phase wird mit `DEFAULT_SHARE_VALUE_CENT = 10000` angelegt.
- **Service-Unit-Tests (mit MockDaos):**
  - `test_partial_repayment_happy_path`
  - `test_partial_repayment_n_zero_invalid`
  - `test_partial_repayment_n_negative_invalid`
  - `test_partial_repayment_n_equals_current_shares_blocked` (Voll-Rückgabe-Block)
  - `test_partial_repayment_n_greater_than_current_shares_blocked` (Sum-Check explizit)
  - `test_partial_repayment_permission_denied`
  - `test_partial_repayment_cancelled_member_blocked`
  - `test_partial_repayment_sum_check_violation`
  - `test_partial_repayment_auto_create_uses_previous_share_value`
  - `test_partial_repayment_auto_create_fallback_default_share_value`
- **Range-Validation-Pure-Function**: `pub(crate) fn validate_partial_repayment_shares(shares: i64, current_shares: i64) -> Result<(), Vec<ValidationFailureItem>>` mit Unit-Tests (mindestens 4 Cases: shares=0, shares=-5, shares=current_shares, shares=current_shares+1).
- **Member-State-Check-Reihenfolge** (Service-Methode):
  1. `permission_service.check_permission(ADMIN_PRIVILEGE, context)`
  2. `member = member_dao.find_by_id(member_id).await?.ok_or(EntityNotFound)?`
  3. `if member.exit_date.is_some() { return Err(Conflict("cancelled member")) }` (D-16-10)
  4. `validate_partial_repayment_shares(shares, member.current_shares)?` (D-16-12 + D-16-11)
  5. `validate_willensbekundung_date(willensbekundung_date, today)?`
  6. `let effective = compute_effective_date(willensbekundung_date)`
  7. `let target_phase = ensure_repayment_phase(effective.fiscal_year, ctx, tx)?` (Auto-Create wenn nötig)
  8. `let existing_entries = repayment_entry_dao.find_by_member_and_phase(member_id, target_phase.id, tx)?`
  9. Sum-Check: `if sum(filter(status != PaidOut, existing)) + shares > current_shares { return Err(ValidationError) }`
  10. `audited_create!(RepaymentEntry { member_id, phase_id, share_count_to_pay_out: shares, status: Open, ... }, PARTIAL_REPAYMENT_PROCESS, user_id, tx)`
  11. `transaction_dao.commit(tx)`
  12. Return `(entry, member, Some(phase_if_created))`

</specifics>

<deferred>
## Deferred Ideas

- **`MembershipAdjustService::transfer_shares`** — Phase 17.
- **AUDT-02 (shared Process-String für Übertrag-Pair)** — Phase 17.
- **Frontend-Modal mit Vorschau** (PART-Pendant zu CANC-06) — Phase 18.
- **Variante A (Phase in Vorbereitung anlegen)** — bewusst gegen entschieden; falls v2 Auto-Anlegen-UX Variante A wünscht, ist die Migration: D-11.1-Phase-Status-Guard auf `Preparation | Open` aufweichen + Auto-Fill-Dedup beim Phase-Open einbauen.
- **Variante C (Explicit Error + manuell anlegen)** — bewusst gegen entschieden; falls v2 strenger werden soll (Vorstand muss bewusst Phase-Wechsel-Entscheidung treffen), ist die Migration: Auto-Anlegen entfernen + HTTP-409-Response mit Frontend-Deep-Link zur Phase-Anlegen-UI.
- **Bulk-Prefetch für Auto-Fill-Skip-Pattern** — bei realistic GV-Größe (<200 Members) nicht nötig; bei Skalierung auf >1000 Members wäre Bulk-Lookup performanter.
- **Targeted `sum_open_shares_by_member_and_phase`-DAO-Query** — wenn `find_by_member_and_phase`-Vec-Transport Performance-Problem wird (heute nicht). Migration: `RepaymentEntryDao::sum_open_shares` als SQL-Aggregat-Query hinzufügen, Service-Layer-Sum auf SQL verlagern.
- **Audit-Macro-Erweiterung um targeted-Methoden** (`audited_update_with!`) — heute nicht nötig; Bedarf könnte in v2 entstehen.
- **`share_value`-Default als Config-Setting** — wenn unterschiedliche Genossi-Installationen unterschiedliche Default-Werte brauchen, kann der Hardcode `10000` in eine ConfigService-Setting (`config.default_share_value_cent`) wandern. Heute YAGNI.
- **Pessimistic-Lock auf Member während v1.2-Dialog** — PITFALLS-Kat-6-Empfehlung: nein, das ist v2-Architektur.
- **`current_shares == 0` defensiver Check** — durch Range-Validation `1 <= n < current_shares` automatisch abgedeckt (bei `current_shares == 0` ist kein valider Wert von `n` möglich).
- **Phase 17 Übertrag** — die Auto-Fill-Skip-Erweiterung in `open_repayment_phase` (D-16-03) wird Phase 17 nicht ins Gehege kommen (Übertrag erzeugt keinen RepaymentEntry; sieht v1.1's Auto-Fill nicht). Aber: Plan-Phase 17 muss Skip-Pattern-Existenz dokumentieren.

</deferred>

---

*Phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase*
*Context gathered: 2026-06-04*
