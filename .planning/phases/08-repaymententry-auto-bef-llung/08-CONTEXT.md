# Phase 8: RepaymentEntry + Auto-Befüllung - Context

**Gathered:** 2026-05-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 8 liefert das auditpflichtige `RepaymentEntry`-Aggregat auf Basis der in Phase 7 etablierten `RepaymentPhase` und vervollständigt die in Phase 7 als Skeleton angelegten Übergänge **PHAS-02** (Auto-Befüllung beim `open_phase`) und **PHAS-03** (Close-Validation gegen pending Entries). Eingänge stehen mit Lifecycle `Open ↔ Contacted` (reversibel) und einer dritten, in Phase 8 nicht togglebaren Enum-Variante `PaidOut` (Toggle kommt erst Phase 9). Phase 8 schreibt KEINE `MemberAction::Verkauf` und reduziert KEIN `Member.current_shares` — beides ist explizit Phase 9.

**In scope:**
- Migration `repayment_entry`-Tabelle: BLOB-UUID `id`, `member_id` BLOB FK, `phase_id` BLOB FK, `share_count_to_pay_out INTEGER NOT NULL` (>0), `status TEXT NOT NULL`, `created`, `deleted`, `version` BLOB
- DAO-Trait + SQLite-Impl + Service-Trait + `*Impl` + REST-Handler + DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()`
- `Auditable`-Impl auf `RepaymentEntryEntity` mit `audit_fields` für `member_id`, `phase_id`, `share_count_to_pay_out`, `status` (NICHT `id`/`version`/`created`/`deleted` per Auditable-Konvention)
- Auto-Befüllung beim Phase-Open (`open_phase`-Erweiterung): in **derselben** Transaktion wie der Status-Übergang Preparation→Open werden alle Mitglieder mit `deleted IS NULL AND exit_date BETWEEN {fy}-01-01 AND {fy}-12-31 AND current_shares > 0` als RepaymentEntries angelegt. **N einzelne `audited_create!`-Calls**, share_count_to_pay_out = `Member.current_shares` zum Zeitpunkt von `open_phase`. Auto-Fill läuft **genau einmal** — Nachzügler ausschließlich über manuellen `POST /api/repayment-entry`.
- Manueller Add: `POST /api/repayment-entry` mit `{ phase_id, member_id, share_count_to_pay_out }`, drei Validierungen: Phase-Status=Open, Member existiert+aktiv, share_count_to_pay_out ∈ (0, Member.current_shares]
- Edit: `PUT /api/repayment-entry/{id}` darf `share_count_to_pay_out` nur ändern wenn Eintragsstatus ∈ {Open, Contacted}; Status-Toggle Open↔Contacted via PUT erlaubt oder via Batch-Endpoint (siehe unten); ENTR-04
- Soft-Delete: `DELETE /api/repayment-entry/{id}` erlaubt wenn Status ≠ PaidOut, sonst 409; ENTR-05
- Batch-Toggle: `POST /api/repayment-entry/batch-status` mit Body `{ entry_ids: [uuid…], target_status: 'Open' | 'Contacted' }`, **all-or-nothing**, eine Transaktion, erster Fehler rollt zurück + 409 mit Detail welche IDs scheiterten warum
- Listing: `GET /api/repayment-entry?phase_id=<uuid>` — nur Phase-Filter, Status/Member-Filter client-side
- Phase-Close-Validation (PHAS-03 vervollständigen): `close_phase` blockt mit 409 wenn mindestens ein Eintrag `status != PaidOut AND deleted IS NULL`; Body enthält `pending_count` + Mitgliedsnummern-Liste (max 20, sonst `+N weitere`); 0-Entry-Fall ist erlaubt (Close geht durch)
- E2E-Tests: Auto-Fill-Lifecycle, manueller Add, Edit, Soft-Delete, Batch-Toggle, Close-Validation Negative-Path, Audit-Chain bleibt via `/api/audit/verify` valide

**Out of scope (gehört in Phase 9-12 oder explizit nicht gewollt):**
- `PaidOut`-Toggle inklusive `MemberAction::Verkauf`-Cascade und `Member.current_shares`-Reduktion → Phase 9 (PAYO-01..04)
- Massenmail-Versand + Template-Variablen `{{ payout_amount }}` → Phase 10
- PDF/CSV-Export der Auszahlungsliste → Phase 11
- Frontend / Dioxus-Komponenten / `RepaymentEntryList` → Phase 12
- Re-Fill-Endpoint zum erneuten Auto-Fill nach Phase-Open → bewusst deferred (würde Phase-Open-Atomarität verletzen)
- Sum-Check über mehrere Einträge pro `(member_id, phase_id)` gegen `Member.current_shares` → Phase 9 fängt das beim `mark_paid_out` ab
- Listing-Filter `?member_id=`, `?status=`, `?include_deleted=true` → nicht jetzt; client-side bzw. spätere Phasen
- Reverse-Transition `Closed → Open` für Phase → bleibt verboten (Phase 7 D-06)

</domain>

<decisions>
## Implementation Decisions

### Auto-Befüllung beim Phase-Open (PHAS-02 / ENTR-01)

- **D-01:** `fiscal_year` definiert als **Kalenderjahr 1.1.–31.12.**: SQL-Filter `exit_date BETWEEN {fiscal_year}-01-01 AND {fiscal_year}-12-31`. Begründung: deutsche Genossenschaften nutzen überwiegend Kalenderjahr als GJ; einfachste Semantik; abweichende GJs sind nicht angefragt. Bei Bedarf nachzuziehen über zusätzliche Phase-Felder.
- **D-02:** **Strikter Member-Filter** für Auto-Fill: `deleted IS NULL AND exit_date BETWEEN ? AND ? AND current_shares > 0`. Member mit 0 Anteilen werden ausgeschlossen (verhindert leere Einträge, die in Phase 9 sowieso `ValidationError` werfen würden). `Member.status` wird NICHT zusätzlich gefiltert — Ausgeschiedene haben oft Status != `Normal` nach `exit_date`, und genau die sind die Zielgruppe.
- **D-03:** **N einzelne `audited_create!`-Calls** pro RepaymentEntry, alle in derselben DB-Transaktion wie der Phase-Status-Übergang. Audit-Hash-Chain enthält jeden Eintrag mit allen Feldern; Pattern-konsistent mit Member-/MemberAction-Audit.
  - **KLARSTELLUNG (Revision Iteration 1 — W-06):** Die Audit-Macro-Konvention (verified per Read auf `genossi_service_impl/src/audit_log.rs:65`) generiert **pro `audited_create!`-Call** eine NEUE `transaction_id` via `uuid_fn()`. N Aufrufe = N transaction_ids. Eine "gemeinsame transaction_id für den Phase-Open-Akt" ist OHNE Macro-Refactor NICHT erreichbar.
  - **Pragmatischer Identifikations-Pfad:** Die N RepaymentEntry-Einträge werden als Folge des Phase-Open-Akts identifiziert über:
    1. Gemeinsamer `process`-String: alle N Calls verwenden `REPAYMENT_PHASE_PROCESS_OPEN = "repayment-phase.open"` (identisch zum `audited_update!` des Phase-Status-Übergangs)
    2. Zeitgleicher `timestamp`-Range: alle Einträge liegen in derselben DB-Commit-Sekunde (Single-Transaction-Atomarität)
    3. Identische `entity_type = "repayment_entry"` + gemeinsamer `phase_id`-Wert in jedem Entry
  - **Audit-Query-Konvention für Phase-Open-Block-Lookup:** `GROUP BY (process, timestamp_minute, entity_type)` filtert nach `process = 'repayment-phase.open' AND entity_type = 'repayment_entry'`; sortiert nach `timestamp` ASC gibt die N Entry-Creations + den 1 Phase-Update in lesbarer Reihenfolge zurück.
  - Anti-Pattern: kein Manual-Hash-Chain-Hack, kein audited_create-Macro-Variante mit shared transaction_id (würde Macro-API ausweiten ohne klaren Nutzen). Phase 9 bekommt die transaction_id-Verkettung MemberAction ↔ RepaymentEntry-Update automatisch, weil EIN `audited_update!`-Call innerhalb desselben Macro-Aufrufs konsistent eine transaction_id verwendet.
- **D-04:** **Auto-Fill läuft genau einmal beim Phase-Open**, keine erneute Re-Fill-Action. Nachträglich gemeldete Austritte werden per `POST /api/repayment-entry` manuell hinzugefügt (ENTR-02). Vermeidet implizite Side-Effects nach Open; klare Audit-Story: ein Phase-Open-Block enthält genau das, was zum Zeitpunkt T existierte. Soft-Delete + Neu-Anlage der Phase ist der Escape-Hatch für massiv fehlerhaftes Auto-Fill.

### Status-Lifecycle & Toggle-API (ENTR-06 / PAYO-04 vorbereiten)

- **D-05:** Status-Enum `RepaymentEntryStatus { Open, Contacted, PaidOut }` **von Anfang an alle drei Varianten** in DAO/Service/REST/TO/DB. Phase 8 implementiert nur die Toggles `Open ↔ Contacted` (per PUT und Batch); jeder Versuch in Phase 8 auf `PaidOut` zu togglen → **409 Conflict** mit Hinweis: „Auszahlung muss über Phase-9-Endpoint `POST /mark-paid-out` laufen". Vermeidet DB-Schema-Migration in Phase 9 und Enum-Erweiterung, hält Status-String-Konvention stabil. Statusstrings analog Phase 7 D-01 in Englisch (Frontend i18n).
- **D-06:** **Reversibilität bidirektional**: `Open ↔ Contacted` ist beide Richtungen erlaubt (z.B. Mail-Korrektur nach falscher E-Mail-Adresse). Jeder Toggle ist eigener `audited_update!`-Eintrag mit Feldänderung `status: Contacted → Open` etc. — Audit-Trail zeigt die komplette Kontakt-Historie. **`PaidOut` bleibt einseitig** (PAYO-04 Phase 9: final).
- **D-07:** Batch-Toggle als **dedizierter Endpoint** `POST /api/repayment-entry/batch-status` mit Body `{ entry_ids: [Uuid, …], target_status: 'Open' | 'Contacted' }`. **PaidOut ist als `target_status` nicht erlaubt** (400). Ein Roundtrip vom Frontend (gegenüber N parallelen PUTs).
- **D-08:** **All-or-nothing-Semantik** für Batch-Toggle: alle Updates in einer Transaktion. Erster Fehler (Conflict bei falschem Quell-Status, version-mismatch, NotFound, soft-deleted) rollt komplett zurück; Response 409 mit Body `{ failure_index, failure_id, failure_reason }`. Vorstand sieht eindeutig was nicht ging und kann nachbessern. Pattern-Bruch zum Phase-10-Mail-Best-Effort ist bewusst: Status-Toggles sind atomare State-Machine-Übergänge, Mail-Versand ist I/O-pro-Empfänger.

### REST-Pfad-Schema & Validation

- **D-09:** **Flat REST-Pfade**: `POST/GET/PUT/DELETE /api/repayment-entry/{id}`, `phase_id` im Create-Body und als Listing-Filter `?phase_id=<uuid>`. Pattern-konsistent mit `/api/member`, `/api/application`, `/api/member-action`, `/api/attendance`. Vereinfacht Audit-Lookups (entity_type='repayment_entry' + entity_id=uuid führt direkt zum Audit-Trail ohne Sub-Pfad-Parsing).
- **D-10:** **Listing nur mit `?phase_id=<uuid>`**. Kein `?status=`, kein `?member_id=`, kein `?include_deleted=`. Frontend filtert client-side (Phase 12); Audit-Tools können über `dump_all` gehen. Hält das REST-Surface schmal; weitere Filter später nachziehbar.
- **D-11:** **Create-Validation (POST /api/repayment-entry)** prüft drei Bedingungen, jede Verletzung blockt:
  1. **Phase-Status = Open** (sonst 409 Conflict). Manuelle Einträge in `Preparation` (Auto-Fill noch nicht passiert) oder `Closed` (Phase final) verboten.
  2. **Member existiert** und `deleted IS NULL` (sonst 400/404).
  3. **`share_count_to_pay_out > 0`** und **≤ `Member.current_shares`** (sonst `ServiceError::ValidationError`).
  Sum-Check über mehrere Einträge pro `(member_id, phase_id)` ist **kein** Phase-8-Concern — Phase 9 fängt das beim `mark_paid_out` über `current_shares`-Check.
- **D-12:** **REST-Endpoints Total Phase 8** (einer pro Operation, keine Nesting-Duplikate):
  - `POST /api/repayment-entry` (manueller Add)
  - `GET /api/repayment-entry?phase_id={uuid}` (Listing, gefiltert)
  - `GET /api/repayment-entry/{id}` (Detail)
  - `PUT /api/repayment-entry/{id}` (Update: `share_count_to_pay_out` und/oder `status` einzeln, Optimistic Locking via `version`)
  - `DELETE /api/repayment-entry/{id}` (Soft-Delete, ENTR-05)
  - `POST /api/repayment-entry/batch-status` (Batch-Toggle, D-07)
  Plus die in **Phase 7** existierenden Phase-Endpoints, an die in Phase 8 erweitert wird:
  - `POST /api/repayment-phase/{id}/open` — erweitert um Auto-Fill (PHAS-02)
  - `POST /api/repayment-phase/{id}/close` — erweitert um pending-Entry-Validation (PHAS-03)

### Close-Validation (PHAS-03)

- **D-13:** **„Pending entry" = `status != PaidOut AND deleted IS NULL`**. Sowohl `Open` als auch `Contacted` blocken den Close. Soft-Delete ist die Konvention für „Eintrag verworfen, Auszahlung findet nicht statt" (z.B. Member hat seine Austrittsmeldung zurückgezogen). Verbandskonform: Phase abschließen heißt „alle Auszahlungen erledigt oder explizit verworfen".
- **D-14:** **0-Entry-Close ist erlaubt**. Phase mit 0 Eintragen kann Open → Closed übergehen ohne Warnung. Realer Use-Case: FY ohne Austritte. Audit-Log zeigt `open` + `close` ohne dazwischenliegende Buchungen. Vorstand kann Phase auch wieder soft-deleten wenn er die ganze Phase verwerfen will (Soft-Delete einer `Open`-Phase ist via Phase 7 D-09 verboten — er muss erst zu `Closed` durch oder die Phase über die DB recovern; das ist beabsichtigt).
- **D-15:** **409-Conflict-Body** beim Close mit pending Entries enthält:
  ```json
  {
    "error": "Cannot close phase: N entries are not paid out and not deleted.",
    "pending_count": N,
    "pending_member_numbers": ["M-001", "M-042", "M-097", "…+N weitere"]
  }
  ```
  Maximal 20 Mitgliedsnummern in der Liste, danach `+N weitere`. Vorstand sieht direkt, wo das Problem liegt, ohne im Frontend filtern zu müssen. Mitgliedsnummern (statt UUIDs oder Entry-IDs) sind die für den Vorstand vertraute Identifikation.

### Claude's Discretion

- **PUT-Body-Schema (D-12)**: Planner darf das genaue Schema festlegen — typisch `{ share_count_to_pay_out?: Option<i32>, status?: Option<RepaymentEntryStatus>, version: Uuid }` mit Edit-Matrix-Check im Service. Status `PaidOut` als PUT-Target → 409 (analog Batch-D-07). Wenn ein Feld nicht im Body steht, bleibt es unverändert.
- **Auto-Fill-Reihenfolge**: Planner wählt die Sortierung der `audited_create!`-Calls für deterministische Audit-Reihenfolge (z.B. `ORDER BY member_number ASC`); nicht strikt nötig für Korrektheit, hilft bei Testing.
- **Batch-Toggle-Größenlimit**: Optional ein Max-Batch-Size (z.B. 500 IDs) zur DoS-Schutz. Planner entscheidet — bestehende Genossi-Massenmail-Patterns geben evtl. Anker.
- **Indizes auf `repayment_entry`**: Migration darf Indizes auf `phase_id`, `(phase_id, status)`, `member_id` anlegen wenn Planner sieht dass Listing-Filter Phase-12 daraus profitiert. Pattern-konsistent mit `attendance`-Indizes.
- **`MemberAction::Verkauf`-Audit-Coupling vorbereiten**: Phase 8 muss noch nichts wiren. Phase 9 wird die `transaction_id`-Verkettung MemberAction ↔ RepaymentEntry-Update über die existierende `audited_update!`-Macro-Tx-Group automatisch bekommen.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Anforderungen
- `.planning/ROADMAP.md` §"Phase 8: RepaymentEntry + Auto-Befüllung" — Goal, 5 Success Criteria (Migration / Phase-Open-Auto-Fill / Manueller Create / Batch-Toggle / Close-Validation)
- `.planning/REQUIREMENTS.md` §"RepaymentEntry Management" (ENTR-01..06) + §"RepaymentPhase Lifecycle" PHAS-02, PHAS-03 (zu vervollständigen)
- `.planning/PROJECT.md` §"Current Milestone: v1.1 Anteile-Rückzahlungsphase" — Target features, Trigger vor GV 2027

### Phase-7-Vorgänger (direkter Bauteil-Lieferant)
- `.planning/phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — Phase-7-Entscheidungen, die Phase 8 ÜBERNIMMT: D-01 (Englisch + i18n), D-02 (Action-Endpoints für Lifecycle), D-05 (409 Conflict für ungültige Transitionen), D-06 (keine Reverse), D-14 (Singular REST-Pfade), D-09 (Soft-Delete nur in `Preparation` für Phase — bleibt; gilt nur für Phase, nicht Entry)
- `.planning/phases/07-repaymentphase-backend-foundation/07-PATTERNS.md` — falls vorhanden, Code-Pattern-Anker
- `genossi_dao/src/repayment_phase.rs` — existierende `RepaymentPhaseStatus`-Enum, `RepaymentPhaseEntity` mit `Auditable`-Impl, DAO-Trait `RepaymentPhaseDao` (ist da, Phase 8 fügt eigenständigen `RepaymentEntryDao` daneben)
- `genossi_dao_impl_sqlite/src/repayment_phase.rs` — SQLite-Impl als Anker für `repayment_entry`-DAO-Impl
- `genossi_service/src/repayment_phase.rs` — Service-Trait `RepaymentPhaseService` mit `open_phase`/`close_phase` — Phase 8 erweitert diese um Auto-Fill und Pending-Validation
- `genossi_service_impl/src/repayment_phase.rs` — `RepaymentPhaseServiceImpl` mit `gen_service_impl!`-Deps; Phase 8 fügt `RepaymentEntryDao`-Dep + Member-DAO falls noch nicht da
- `genossi_rest/src/repayment_phase.rs` — REST-Handler für Phase-Lifecycle, Anker für `repayment_entry`-REST-Handler

### Pattern-Anker: Assembly-Aggregat (Auto-Fill-Snapshot ähnlich)
- `genossi_service_impl/src/assembly.rs:181-259` — `open_assembly` mit single-Tx + Auto-Snapshot-Pattern; direkte Vorlage für `open_phase`-Erweiterung (Auto-Fill in derselben Tx wie Status-Update)
- `genossi_service_impl/src/assembly.rs:261-` — `close_assembly` als Vorlage für `close_phase`-Pending-Validation (Pattern: Status-Guard + DAO-Lookup + 409 mit Detail)
- `genossi_dao/src/assembly_member_snapshot.rs` — Batch-Insert-Pattern (`create_batch`); **NICHT 1:1 für RepaymentEntry**, weil Phase 8 D-03 N einzelne audited_create-Calls fordert (kein Batch-without-Audit)
- `genossi_dao_impl_sqlite/src/assembly.rs` — SQLite-Impl-Vorlage, UPDATE...RETURNING-Pattern

### Member-Filter-Logik (Auto-Fill-Quelle)
- `genossi_dao/src/member.rs:94` — `exit_date: Option<time::Date>` (das Feld, gegen das gefiltert wird)
- `genossi_dao/src/member.rs:90` — `current_shares: i32` (Snapshot-Wert für `share_count_to_pay_out`)
- `genossi_dao/src/member.rs:172-185` — `count_active`-Filter-Logik als semantisches Beispiel; Phase 8 fordert eigene Filter (exit_date IN FY + current_shares > 0); `is_normal()`-Filter NICHT übernehmen (D-02)
- `genossi_service_impl/src/assembly.rs:230-249` — Snapshot-Filter-Code mit `is_normal() + join_date + exit_date`-Kombination als syntaktische Vorlage (Filter-Bedingungen sind in Phase 8 anders, Struktur ist gleich)

### Audit-Infrastruktur (Phase-7-Pattern, in Phase 8 wiederverwendet)
- `genossi_dao/src/auditable.rs` — `Auditable`-Trait-Definition; `RepaymentEntryEntity` muss implementieren (`entity_type() = "repayment_entry"`, `entity_id()`, `audit_fields()` für `member_id`, `phase_id`, `share_count_to_pay_out`, `status`)
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!` (beim manuellen POST + N-fach beim Auto-Fill in `open_phase`), `audited_update!` (PUT, Batch-Toggle, Soft-Delete via D-08-Pattern). **REAL signatures (verified): `audited_create!` 6 Args, `audited_update!` 7 Args, `audited_delete!` 6 Args (lädt entity intern)**.
- `genossi_service_impl/src/audit_log.rs` — Hash-Chain-Berechnung (NICHT manuell re-implementieren — Anti-Pattern in ARCHITECTURE.md). **Wichtig: `build_create_entries` ruft `uuid_fn()` pro Call → neue transaction_id pro audited_create! (siehe D-03 Klarstellung)**.
- `CLAUDE.md` §"Audit Log System" — 4-Schritt-Checkliste für neue auditierte Entities

### MemberAction-Coupling (Phase 9 vorbereitet, hier nur Schema-Awareness)
- `genossi_dao/src/member_action.rs` — `ActionType::Verkauf` als Enum-Variante existiert (`MemberActionEntity`); Phase 8 schreibt NICHT, aber `RepaymentEntryEntity` muss feldlich kompatibel sein (z.B. `share_count_to_pay_out` korrespondiert mit Phase-9-`shares_change=-N`)

### Service-Layer-Patterns
- `genossi_service_impl/src/macros.rs` — `gen_service_impl!` Macro für Service-Boilerplate
- `genossi_service_impl/src/validation.rs` — `ValidationService` für `share_count_to_pay_out`-Range (vs. `Member.current_shares`)
- `genossi_service/src/permission.rs` — `Authentication<Context>` für OIDC-Vorstand-Check; alle Endpoints sind admin-only (analog Phase 7 + ADMIN_PRIVILEGE-Pattern aus `assembly.rs:50`)
- **Imports-Konvention (verified per Phase-7-Pattern in `repayment_phase.rs:28-37`):** `ServiceError` + `ValidationFailureItem` aus `genossi_service` importieren — **NICHT aus `genossi_dao`** (dort nicht definiert).

### REST-Layer-Patterns
- `genossi_rest/src/lib.rs` — Router-Registration; `.merge(repayment_entry::generate_route())` ergänzen. **Member-Route ist PLURAL `/api/members` (verified Z. 559)** — E2E-Helper müssen entsprechend posten.
- `genossi_rest_types/src/lib.rs` — neuer `RepaymentEntryTO` + `From<&RepaymentEntryEntity>` + Utoipa-Schema-Derive; **`RepaymentEntryBatchStatusRequest`-TO**, **`CloseConflictResponse`-TO** mit `pending_member_numbers`-Field (D-15), **`BatchFailureResponse`-TO** mit `failure_index`/`failure_id`/`failure_reason` (D-08 — strukturierter Body)
- `genossi_rest/src/assembly.rs:142,224,279,315` — `#[utoipa::path]`-Annotation-Patterns für POST/PUT/POST(action) inkl. Response-Doc und Schema (insbesondere 409-Response-Doc)
- `genossi_rest/src/repayment_phase.rs` — Phase-7-Handler als Anker; Phase-8 fügt `repayment_entry.rs` als neuen Sub-Router daneben

### Binary-Layer (Dependency-Injection)
- `genossi_bin/src/lib.rs` (`RestStateImpl::new()`) — neuer `RepaymentEntryDaoImpl` + `RepaymentEntryServiceImpl`; `Arc::clone(&audit_log_dao)` für Audit-Wiring; `Arc::clone(&member_dao)` für Member-Lookups; `RepaymentPhaseServiceImpl`-Deps müssen erweitert werden (`RepaymentEntryDao` + `MemberDao` für Auto-Fill in `open_phase`)

### Testing-Patterns
- `genossi_bin/tests/e2e_tests.rs` — E2E-Pattern; Phase 8 ergänzt Tests: Phase-Open mit Auto-Fill (verifiziert N Einträge erzeugt + Audit-Chain), manueller Add (Happy + 3 Validation-Fails), Edit/Soft-Delete-Edit-Matrix, Batch-Toggle (Happy + All-or-Nothing-Fail), Close mit pending → 409 mit pending_member_numbers
- `genossi_rest/src/test_server.rs` — `start_test_server` Helper
- `genossi_service_impl/src/repayment_phase.rs` (Phase 7) — `test_*` als Vorlage für RepaymentEntry-Service-Unit-Tests

### Architekturelle Constraints
- `.planning/codebase/ARCHITECTURE.md` — Anti-Patterns (Hard Delete, Manual Hash Chain, Service-creates-its-own-Transaction); Layer-Verantwortlichkeiten
- `CLAUDE.md` §"Entity Structure" — UUID/BLOB, ISO8601, optimistic locking via `version`
- `.planning/PROJECT.md` §"Constraints" — Audit-Pflicht für neue auditpflichtige Entities (gilt für RepaymentEntry — D-03)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`RepaymentPhaseServiceImpl::open_phase` (Phase 7)** — bereits da als Skeleton (Status-Übergang Preparation→Open mit `audited_update!`). Phase 8 erweitert die Methode innerhalb der bestehenden Transaktion um den Auto-Fill-Block (Member-Query, N audited_create!-Calls). Pattern-Anker: `assembly.rs:181-259`.
- **`RepaymentPhaseServiceImpl::close_phase` (Phase 7)** — Skeleton ohne Validation. Phase 8 fügt den Pending-Entry-Check vor dem Status-Übergang ein (DAO-Listing der Phase-Entries, Filter, bei `len > 0` Sammeln der Mitgliedsnummern, 409-Response).
- **`audited_create!`/`audited_update!`** — komplette Audit-Macro-Infrastruktur; `audit_log_dao`+ `uuid_service` werden via `gen_service_impl!`-Deps gewired (Pattern aus `RepaymentPhaseServiceImpl`-Phase 7 1:1 übernehmen).
- **`MemberDao::all` + `MemberDao::find_by_id`** — bereits da; `all` filtert `deleted IS NULL`. Auto-Fill nutzt `all` + In-Memory-Filter auf `exit_date BETWEEN ?` + `current_shares > 0`. Performance OK für Genossi-Größenordnung; alternative SQL-Where-Klausel im DAO ist Premature-Optimization.
- **`ValidationService`** — `validate_share_count_to_pay_out(count: i32, current_shares: i32)` reiht sich neben Phase-7-Validatoren ein.
- **`PermissionService::check_permission(ADMIN_PRIVILEGE, ...)`** — OIDC-Vorstand-Check; alle Phase-8-Endpoints admin-only.

### Established Patterns

- **Layered Implementierungs-Reihenfolge** (Backend-First, identisch zu Phase 7): Migration → DAO-Trait → SQLite-Impl → Service-Trait → Service-Impl (inkl. Phase-7-Erweiterungen) → REST-Handler + TOs → DI-Wiring → E2E-Tests.
- **`gen_service_impl!`-Macro** für Service-Impl-Boilerplate.
- **`Auditable`-Trait** auf neuer Entity; `audit_fields` enthält genau die geschäftsrelevanten Felder, **nicht** `id`/`version`/`created`/`deleted` (Konvention).
- **ISO8601-Datetime + BLOB-UUID** in Entities; SQLite-Storage als TEXT/BLOB; Custom-Serde-Module in `genossi_rest_types/src/lib.rs`.
- **DAO-Minimal-Interface** (`create`, `update`, `dump_all`, `find_by_id`) plus Domain-Methoden — Phase 8 braucht `find_by_phase_id(phase_id) -> Arc<[RepaymentEntryEntity]>` für Listing + Close-Validation (DAO-Method oder Service-Layer-Filter über `all`; Planner entscheidet).
- **Batch-Endpoint-Pattern in einer Tx**: analog `assembly_member_snapshot_dao.create_batch` (siehe Phase-1-Code) — aber Phase 8 NICHT batch-without-audit, sondern N einzelne audited_create-Calls in einer Tx (D-03). Batch-Toggle ist N einzelne audited_update-Calls in einer Tx (D-08).
- **Single-Transaction-Multi-DAO-Methode** (`open_phase` und `close_phase`): Pattern aus `assembly.rs:181-259` mit `tx.clone()` für sub-DAO-Calls, ein einziges `commit` am Ende, Rollback durch Drop bei Error.

### Integration Points

- **`genossi_bin/src/lib.rs::RestStateImpl::new()`** — neue DAO (`RepaymentEntryDaoImpl`) und Service (`RepaymentEntryServiceImpl`) aufbauen; **`RepaymentPhaseServiceImpl`-Deps erweitern** um `RepaymentEntryDao` und `MemberDao` (Auto-Fill in `open_phase`). Wiring-Reihenfolge: zuerst RepaymentEntryDao bauen, dann es als `Arc::clone` an RepaymentPhaseService und RepaymentEntryService weitergeben.
- **`genossi_rest/src/lib.rs`** — `.merge(repayment_entry::generate_route())` ergänzen; OpenAPI-Schema-Registry erweitern.
- **`genossi_service/src/lib.rs`** und **`genossi_service_impl/src/lib.rs`** — `pub mod repayment_entry;` Modul-Deklaration + selektives Re-Export von `RepaymentEntryServiceImpl`/`MockRepaymentEntryServiceImpl`.
- **`genossi_dao/src/lib.rs`** + **`genossi_dao_impl_sqlite/src/lib.rs`** — analog Modul-Deklarationen.
- **`genossi_rest_types/src/lib.rs`** — `RepaymentEntryTO` + `RepaymentEntryCreateRequest` + `RepaymentEntryUpdateRequest` + `BatchStatusRequest` + `CloseConflictResponse` + `BatchFailureResponse` (W-05) + Utoipa-Schema-Derives.
- **Migration** in `migrations/sqlite/`: nächste Sequenz-Nummer nach `20260529190437_create_repayment_phase_table.sql`. Planner setzt finale Datum-Sequenz (vermutlich `20260530…_create_repayment_entry_table.sql`).

</code_context>

<specifics>
## Specific Ideas

- **Auto-Fill genau einmal** (D-04) — User-Vision: „die Phase ist ein Bilanz-Stichtag". Was zum Zeitpunkt T des Phase-Open im System ist, wird gefroren; alles danach ist explizite Vorstand-Aktion. Macht Audit-Story klar lesbar im Protokoll.
- **Mitgliedsnummern (nicht UUIDs/Names) im 409-Body** (D-15) — Vorstand denkt in Mitgliedsnummern, nicht in UUIDs. Phase-12-Frontend kann mit der Liste sofort ein Toast oder einen Filter auf die Tabelle setzen. Max 20 mit `+N weitere`-Annotation reicht für den Use-Case (typische Genossenschaft hat <100 Austritte/Jahr).
- **`PaidOut` als 3-Wert-Enum schon in Phase 8** (D-05) — Vermeidet die Phase-9-Migration und ist verbandskonform sauber: Status-String-Konvention bleibt stabil über Milestones hinweg.
- **All-or-nothing Batch-Toggle** (D-08) — Gegenpol zum Phase-10-Mail-Best-Effort. Begründet, weil Status-Toggle eine atomare State-Machine-Transition ist, kein I/O-Resilience-Problem.
- **N einzelne `audited_create!` statt Batch** (D-03) — Bewusst andere Wahl als Assembly-Member-Snapshot, weil RepaymentEntries Lifecycle-Träger sind (Phase-9-Cascade hängt direkt an entity_id+version) — nicht „nur Daten". Identifikation via gemeinsamer `process`-String + Timestamp-Range (siehe D-03 Klarstellung).

</specifics>

<deferred>
## Deferred Ideas

- **`?member_id=<uuid>` Listing-Filter** — sinnvoll für Phase-12-Member-Detail-Page (Auszahlungs-Historie pro Mitglied); kann nachgezogen werden ohne API-Breaking-Change
- **`?status=<...>` Listing-Filter** — kann später dazukommen; jetzt client-side
- **`?include_deleted=true` Listing-Filter** — für Audit-Tooling, später; default-off-Konvention (Phase 7 D-10) bleibt
- **Re-Fill-Endpoint `POST /api/repayment-phase/{id}/refill`** — explizit verworfen (D-04); Escape-Hatch ist Soft-Delete + Neuanlage der Phase oder manueller `POST /api/repayment-entry` pro Nachzügler
- **Member-Status-Filter (`is_normal()`)** in Auto-Fill — bewusst NICHT angewendet (D-02); Ausgeschiedene haben oft Status != Normal, das ist genau die Zielgruppe
- **Sum-Check über mehrere Entries pro `(member_id, phase_id)`** gegen `Member.current_shares` — Phase 9 fängt das beim `mark_paid_out` via `current_shares`-Check; in Phase 8 würde das ein Pattern-Bruch sein (Service-Layer-Aggregat-Validation)
- **DB-Unique-Index auf `(phase_id, member_id, status='Open')`** — würde ENTR-03 brechen (mehrere Einträge pro Mitglied+Phase erlaubt). Nicht.
- **PUT-Body-Status-Toggle vs. dedizierter Sub-Endpoint** — Phase 7 D-02 sagt „Lifecycle-Transition via Action-Endpoint, kein Status im PUT-Body". Phase 8 erlaubt Status im PUT (Open↔Contacted) **zusätzlich** zum Batch-Endpoint, weil Einzel-Toggle ergonomischer ist. Planner-Discretion ob das explizit dokumentiert wird oder ob nur Batch geht.
- **Max-Batch-Size für Batch-Toggle** — DoS-Schutz, Planner-Discretion
- **Shared-transaction-id für Auto-Fill-Block** — würde Audit-Macro-API erweitern (`audited_create_with_tx_id!`-Variante oder TxScope-Wrapper); bewusst NICHT in Phase 8, weil pragmatischer `(process, timestamp)`-Pfad ausreicht (D-03 Klarstellung). Reopen wenn echter Audit-Query-Use-Case auftaucht.

### Reviewed Todos (not folded)
Keine — `gsd-sdk query todo.match-phase 8` lieferte keine Matches (nicht ausgeführt; keine pending Todos im System).

</deferred>

---

*Phase: 8-repaymententry-auto-bef-llung*
*Context gathered: 2026-05-30*
*Revised: 2026-05-30 (Iteration 1 — D-03 Klarstellung, audit_macros-Signaturen, Plural-Member-Route, strukturierter Batch-Conflict-Body)*
