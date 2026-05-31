---
phase: 08-repaymententry-auto-bef-llung
verified: 2026-05-31T18:00:00Z
status: gaps_found
score: 5/5 ROADMAP success criteria verified (with quality gaps inside SC#3/SC#4)
overrides_applied: 0
gaps:
  - truth: "Optimistic-Locking-Vertrag: PUT/Batch/Lifecycle-Response liefert verwendbare `version` für Folge-PUT (CR-01)"
    status: failed
    reason: |
      RepaymentEntryServiceImpl::update_repayment_entry (Z. 266),
      RepaymentEntryServiceImpl::batch_toggle_status (Z. 444),
      RepaymentPhaseServiceImpl::update_repayment_phase (Z. 221),
      RepaymentPhaseServiceImpl::open_repayment_phase (Z. 355),
      RepaymentPhaseServiceImpl::close_repayment_phase (Z. 459) und
      RepaymentPhaseServiceImpl::create_repayment_phase (Z. 140) liefern alle
      `RepaymentEntry::from(&entity)` bzw. `RepaymentPhase::from(&entity)` mit
      der PRE-UPDATE-Version. Das DAO (`repayment_entry.rs:136` und Phase-Pendant)
      generiert via `let new_version = Uuid::new_v4()` eine neue Version und
      schreibt sie in die DB, propagiert sie aber nie zurück. Service liest die
      Entity nicht erneut. Folge: Jeder Client, der `response.version` für ein
      Folge-PUT verwendet, erhält 409 "Version mismatch". Das in
      `MemberServiceImpl::update` (member.rs:343-348) etablierte Re-Read-Pattern
      ist die kanonische Lösung und fehlt hier.
      E2E-Tests fangen den Bug nicht ab, weil kein Test einen zweiten PUT mit
      der vom ersten PUT zurückgelieferten Version macht (siehe IN-04 im REVIEW).
      Funktional bricht das Phase-8 ROADMAP-SC #3 ("manuelles create_entry
      funktioniert") und SC #4 ("Status-Toggle multi-select-fähig") für jede
      realistische Frontend-Sequenz, in der dieselbe Entity mehrfach editiert
      wird (z.B. zuerst Mail-Toggle dann Korrektur).
    artifacts:
      - path: "genossi_service_impl/src/repayment_entry.rs"
        issue: "update_repayment_entry (Z. 266) + batch_toggle_status (Z. 444) returnen entity vor DAO-update; kein find_by_id-Reread"
      - path: "genossi_service_impl/src/repayment_phase.rs"
        issue: "update_repayment_phase (Z. 221), open_repayment_phase (Z. 355), close_repayment_phase (Z. 459), create_repayment_phase (Z. 140) returnen entity vor DAO-update; gleiche Bug-Klasse aus Phase 7 fortgepflanzt"
      - path: "genossi_bin/tests/e2e_tests.rs"
        issue: "Kein E2E-Regressionstest, der zwei aufeinanderfolgende PUTs mit der version aus dem 1. PUT als Input für den 2. PUT macht (IN-04 im REVIEW)"
    missing:
      - "Nach jedem audited_update! in RepaymentEntryServiceImpl::update_repayment_entry: find_by_id(id, tx.clone()) re-read und das Ergebnis statt der pre-update-entity returnen"
      - "Nach jedem audited_update! in RepaymentEntryServiceImpl::batch_toggle_status: refreshed entry pro Iteration sammeln (oder nach der Loop alle nochmal laden) statt aktueller pre-update-entity"
      - "Gleiche Re-Read-Korrektur in RepaymentPhaseServiceImpl::update_repayment_phase, open_repayment_phase, close_repayment_phase, create_repayment_phase"
      - "E2E-Regressionstest: PUT auf RepaymentEntry, returned version für 2. PUT verwenden, 200 erwarten (nicht 409)"

  - truth: "REST-API-Konsistenz: 'entry not found' im Batch-Toggle liefert 404 (Aggregat-Konvention) statt 409 (CR-02)"
    status: failed
    reason: |
      `RepaymentEntryServiceImpl::batch_toggle_status` (Z. 416) mappt eine
      fehlende/soft-gelöschte Entry-ID auf einen 409-Conflict mit
      `failure_reason: "entry not found"`, statt auf `ServiceError::EntityNotFound`
      (→ 404). Alle anderen Methoden im Aggregat (get/update/delete) returnen
      404 für denselben Zustand. Die OpenAPI-Doku des Batch-Endpoints
      (`genossi_rest/src/repayment_entry.rs:260-265`) listet bewusst nur
      200/400/401/409 — der 404-Pfad ist undokumentiert. Das
      `BatchFailureResponse`-Schema spezifiziert nicht, dass `failure_reason`
      auch "entry not found" enthalten kann. Folge: Ein Frontend, das 409 als
      "Domain-Konflikt (Status hat sich geändert — bitte Liste neu laden)"
      klassifiziert, klassifiziert eine UI-Race-Condition (stale ID nach
      Soft-Delete in anderem Tab) silent falsch.
    artifacts:
      - path: "genossi_service_impl/src/repayment_entry.rs"
        issue: "Z. 416: .ok_or_else(|| conflict_body(idx, *entry_id, \"entry not found\"))? statt .ok_or(ServiceError::EntityNotFound(*entry_id))?"
      - path: "genossi_rest/src/repayment_entry.rs"
        issue: "Z. 260-265: OpenAPI-Doku für POST /batch-status listet keinen 404-Response; BatchFailureResponse-Doku nennt 'entry not found' nicht als möglichen failure_reason"
    missing:
      - "Variante (a) (preferred): Service-Layer mappt Not-Found-Fall auf ServiceError::EntityNotFound(*entry_id) (entsprechend dem Rest des Aggregats); REST-Layer liefert dann 404"
      - "Oder Variante (b): OpenAPI-Doku um (status = 404) ergänzen UND BatchFailureResponse-doc-comment um 'entry not found'-Fall erweitern, plus explizite Test-Erwartung"
      - "Unit-Test der das verifizierte Verhalten (404 oder dokumentierter 409) gegen einen nicht-existenten Entry-ID-Eintrag in der Batch-Liste prüft"

human_verification:
  - test: "Realistisches Folge-PUT mit der von einem vorherigen PUT zurückgelieferten Version"
    expected: "PUT /api/repayment-entry/{id} mit response.version aus der 1. PUT-Response sollte 200 ergeben"
    why_human: "E2E-Tests prüfen das nicht; manuelle Verifikation via curl oder Frontend zeigt sofort, ob CR-01 in realer Nutzung Auswirkungen hat. Ohne Fix gibt es im Frontend einen sichtbaren Optimistic-Locking-Konflikt nach jedem Edit."
  - test: "Batch-Toggle mit einer in einem anderen Tab gerade gelöschten Entry-ID"
    expected: "Antwort-Status (409 oder 404) + Body-Format sollte das Frontend eindeutig erkennen lassen, ob es ein Stale-ID-Problem ist (Listing aktualisieren) oder ein Domain-Konflikt (Bestätigung anfordern)"
    why_human: "UX-Defekt durch CR-02 nur in realer Multi-Tab- oder Multi-User-Nutzung sichtbar"

---

# Phase 8: RepaymentEntry + Auto-Befüllung — Verification Report

**Phase Goal:** RepaymentEntry-Aggregat mit Auto-Befüllung beim Phase-Öffnen,
manueller Ergänzung, und Status-Toggle `offen ↔ angeschrieben` (ohne `ausbezahlt` —
kommt in Phase 9).

**Verified:** 2026-05-31T18:00:00Z
**Status:** gaps_found (2 BLOCKER bugs aus Code-Review im Code bestätigt)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Migration legt `repayment_entry`-Tabelle an (kein Composite-PK, eigene UUID; `member_id`, `phase_id`, `share_count_to_pay_out INTEGER`, `status TEXT`, `created`, `deleted`, `version`) | ✓ VERIFIED | `migrations/sqlite/20260530203550_create_repayment_entry_table.sql` enthält CREATE TABLE mit 8 Spalten + 3 Indizes; `CHECK(share_count_to_pay_out > 0)`; einziges "UNIQUE"-Vorkommen ist in einem Kommentar, der die ABSENZ dokumentiert (Z. 21); `cargo test -p genossi_dao_impl_sqlite repayment_entry` 6 Tokio-Tests grün (Roundtrip + Optimistic-Locking) |
| 2 | Phase-Öffnen (`open_phase`) befüllt atomar Einträge für alle Mitglieder mit `exit_date BETWEEN ? AND ?` (Geschäftsjahres-Range) — `share_count_to_pay_out = Member.current_shares`-Snapshot | ✓ VERIFIED | `genossi_service_impl/src/repayment_phase.rs:280-355` Auto-Fill-Block in derselben Tx wie `audited_update!`; `from_calendar_date(fiscal_year, Month::January, 1)` + `Month::December, 31` als Range-Filter; `share_count_to_pay_out = member.current_shares` (Z. 327); 6 Unit-Tests für Auto-Fill (zero_members/matching/skip_zero_shares/skip_outside_FY/skip_no_exit_date/atomic_on_failure) grün; 4 E2E-Tests (auto_fill_triggers/zero_members/skips_no_exit_date/skips_outside_FY) grün |
| 3 | Manuelles `create_entry` über REST funktioniert; mehrere Einträge pro Mitglied+Phase im selben State verifiziert durch Integration-Test | ⚠️ VERIFIED mit Quality-Gap (CR-01) | POST `/api/repayment-entry` Handler in `genossi_rest/src/repayment_entry.rs:60-90` mit `extract_auth_context` + Service-Call; Schema-Migration hat KEIN UNIQUE-Constraint auf (member_id, phase_id) (verified); E2E-Tests `test_manual_add_entry_happy_path`, `test_manual_add_entry_phase_not_open_returns_409`, `test_manual_add_entry_share_count_exceeds_returns_400` grün. **ABER:** Die Response liefert stale `version` zurück (CR-01) — siehe Gap unten |
| 4 | Status-Toggle `offen ↔ angeschrieben` ist multi-select-fähig (Batch-Endpoint); Audit-Eintrag pro Toggle | ⚠️ VERIFIED mit Quality-Gaps (CR-01, CR-02) | POST `/api/repayment-entry/batch-status` Handler vorhanden (Z. 254-293), Router-Reihenfolge `/batch-status` VOR `/{id}` (Z. 304); `batch_toggle_status` ruft `audited_update!` N-mal in 1 Tx (Z. 433-443); Unit-Test `test_batch_toggle_success` verifiziert 3 audited_update-Calls; E2E `test_batch_toggle_happy_path` grün. **ABER:** stale version pro Entry in Response (CR-01/WR-01) + "entry not found" als 409 statt 404 (CR-02) — siehe Gaps |
| 5 | `close_phase` (PHAS-03) blockt mit 409 Conflict wenn mindestens ein Eintrag nicht `ausbezahlt` ODER `deleted IS NULL` ist — E2E-Test deckt Negative-Path | ✓ VERIFIED | `genossi_service_impl/src/repayment_phase.rs:380-446` Pending-Validation-Block; Pending-Definition `e.status != PaidOut AND e.deleted.is_none()`; Body als `serde_json::json!({error, pending_count, pending_member_numbers})` in Arc<str>-Wrap; 0-Entry-Close erlaubt (D-14); E2E-Tests `test_close_phase_with_pending_entries_returns_409_with_member_numbers` und `test_close_phase_with_zero_entries_succeeds` grün |

**Score:** 5/5 ROADMAP Success Criteria sind grundsätzlich erfüllt (Migration, Auto-Fill, Manueller Create, Batch-Toggle, Close-Validation). **2 Quality-Gaps** innerhalb SC#3 und SC#4 sind als BLOCKER-Bugs im Code-Review identifiziert und im Codebase bestätigt — sie machen die API in realistischen Folge-PUT-Sequenzen unbrauchbar (CR-01) bzw. inkonsistent für Stale-ID-Fälle (CR-02).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/sqlite/20260530203550_create_repayment_entry_table.sql` | DDL + 3 Indizes + FK-Doku | ✓ VERIFIED | 8 Spalten, CHECK > 0, 3 Indizes, FK-Doku-Kommentar, KEIN UNIQUE (außer dokumentierende Negation in Comment) |
| `genossi_dao/src/repayment_entry.rs` | Enum + Entity + Auditable + Trait + Mock | ✓ VERIFIED | 8 Unit-Tests grün; frozen audit_fields (member_id/phase_id/share_count_to_pay_out/status), entity_type="repayment_entry"; find_by_phase_id Default-Impl |
| `genossi_dao_impl_sqlite/src/repayment_entry.rs` | SQLite-DaoImpl mit Pre-Exists + Optimistic-Locking | ✓ VERIFIED | 6 Tokio-Tests grün; Pre-Exists-Check (`SELECT COUNT(*)`) + atomic `UPDATE...WHERE id=? AND version=? AND deleted IS NULL`; guarded i32-Cast vorhanden |
| `genossi_service/src/repayment_entry.rs` | Trait + DTOs | ✓ VERIFIED | 5 Unit-Tests grün; 6-Methoden-Trait mit #[automock]; RepaymentEntrySubmission/Update/BatchStatusInput vorhanden |
| `genossi_service_impl/src/repayment_entry.rs` | Service-Impl mit Audit-Macros + Edit-Matrix | ⚠️ STUB-ARTIG (CR-01/CR-02) | 19 Unit-Tests grün; Audit-Disziplin Grep-Gate sauber; ADMIN_PRIVILEGE-Check vorhanden; aber `update_repayment_entry` (Z. 266) und `batch_toggle_status` (Z. 444) returnen entity mit STALE version statt nach DAO-update re-zu-lesen; `batch_toggle_status` (Z. 416) mappt Not-Found auf 409 statt 404 |
| `genossi_service_impl/src/repayment_phase.rs` | Auto-Fill + Pending-Validation | ⚠️ TEILWEISE (CR-01) | 23 Tests grün (14 Phase-7 + 9 neue Phase-8); Auto-Fill + Pending-Validation funktional korrekt; Deps um RepaymentEntryDao + MemberDao erweitert; ABER update_repayment_phase/open/close/create haben gleiche Stale-Version-Bug-Klasse wie RepaymentEntry-Service |
| `genossi_rest_types/src/lib.rs` | 7 TOs inkl. BatchFailureResponse + CloseConflictResponse | ✓ VERIFIED | 7 Unit-Tests grün; alle 7 TOs vorhanden (RepaymentEntryStatusTO + RepaymentEntryTO + 3 Requests + 2 ConflictResponses) mit ToSchema + bidirektionalen From-Impls |
| `genossi_rest/src/repayment_entry.rs` | 6 Handler + Router + ApiDoc | ⚠️ TEILWEISE (CR-02) | 3 Unit-Tests grün; 6 Handler mit `#[utoipa::path]`-Annotation; Router `/batch-status` VOR `/{id}` korrekt; ApiDoc kompiliert; **ABER:** OpenAPI für POST /batch-status listet keine 404-Response, obwohl Service "entry not found" durchreicht (CR-02-Symptom) |
| `genossi_rest/src/lib.rs` | Modul + Nest + Trait-Bounds | ✓ VERIFIED | `pub mod repayment_entry` (Z. 20); OpenAPI-Nest (Z. 272); Router-Mount (Z. 617); 2 Trait-Bounds (Z. 441 + 764) |
| `genossi_rest/src/test_server.rs` | Trait-Bound erweitert | ✓ VERIFIED | RepaymentEntryRestState ergänzt — E2E-Tests starten erfolgreich |
| `genossi_bin/src/lib.rs` | DI-Wiring + RestState-Bridge | ✓ VERIFIED | `let repayment_phase_dao = Arc::new(...)` exakt 1× (W-02 sauber); `let repayment_entry_dao = Arc::new(...)` 1×; impl RepaymentEntryRestState for RestStateImpl vorhanden |
| `genossi_bin/tests/e2e_tests.rs` | 15 neue E2E-Tests + 2 Helper | ✓ VERIFIED (mit Test-Coverage-Lücke IN-04) | Alle 15 erforderlichen Test-Namen vorhanden; create_member_with_exit_date + create_open_repayment_phase Helper vorhanden; `cargo test --test e2e_tests --features mock_auth` zeigt 270/270 grün (255 Phase-7-Baseline + 15 neue Phase-8) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `RestStateImpl.repayment_entry_service` | RepaymentEntryServiceImpl mit 7 Arc-shared Deps | gen_service_impl! + Arc::clone | ✓ WIRED | `cargo build --workspace` clean; DI-Wiring liefert beim Server-Start ein funktionales Service |
| Router POST /api/repayment-entry/batch-status | batch_toggle_status Handler | `.route("/batch-status", post(...))` | ✓ WIRED | Router-Reihenfolge `/batch-status` VOR `/{id}` mit Inline-Doc-Kommentar fixiert; E2E `test_batch_toggle_happy_path` grün |
| `open_repayment_phase` Auto-Fill | `member_dao.all` + N audited_create! auf repayment_entry_dao | gleiche tx, sortiert nach member_number | ✓ WIRED | E2E `test_open_phase_triggers_auto_fill` legt 3 Members an, öffnet Phase, GET liefert genau 2 Einträge (cs=0 wird gefiltert) |
| `close_repayment_phase` Pending-Check | `repayment_entry_dao.find_by_phase_id` + `member_dao.all` | filter + JSON-Body | ✓ WIRED | E2E `test_close_phase_with_pending_entries_returns_409_with_member_numbers` zeigt Body enthält "pending_count" und "42" |
| `update_repayment_entry` → Client | RepaymentEntryTO mit version | RepaymentEntry::from(&entity) | ✗ NOT_WIRED (CR-01) | Service returnt stale `entity.version`; DAO-generierte neue version wird nie ausgelesen; verstößt gegen den dokumentierten Optimistic-Locking-Vertrag |
| `batch_toggle_status` → Client | Vec<RepaymentEntryTO> mit versions | updated.push(RepaymentEntry::from(&entity)) | ✗ NOT_WIRED (CR-01/WR-01) | N stale versions pro Batch — CR-01 N-mal repliziert |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `RepaymentEntryDaoImpl::dump_all` | rows | sqlx::query_as gegen repayment_entry | Ja (echte Tabelle, 6 Tokio-Tests bestätigen) | ✓ FLOWING |
| Auto-Fill-Block | targets | member_dao.all() + filter on exit_date/current_shares | Ja (E2E verifiziert 2 von 3 Members in einer Tx) | ✓ FLOWING |
| Pending-Validation-Block | pending | repayment_entry_dao.find_by_phase_id(id) + filter | Ja (E2E zeigt Body mit konkreter Mitgliedsnummer) | ✓ FLOWING |
| `update_repayment_entry` Response | entity.version | pre-update value | Nein — DAO schreibt neue UUID, Service liest sie nicht zurück | ⚠️ HOLLOW (CR-01) |
| `batch_toggle_status` Response | updated[i].version | pre-update value | Nein — N stale UUIDs (siehe oben) | ⚠️ HOLLOW (CR-01/WR-01) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace baut clean | `cargo build --workspace` | Finished `dev` profile (1 unused-imports-Warning in genossi_bin pre-existing) | ✓ PASS |
| DAO-Trait + Migration tests grün | `cargo test -p genossi_dao repayment_entry` | 8 passed | ✓ PASS |
| DAO-SQLite tests grün | `cargo test -p genossi_dao_impl_sqlite repayment_entry` | 6 passed | ✓ PASS |
| Service-Trait tests grün | `cargo test -p genossi_service repayment_entry` | (siehe Service-Impl) | ✓ PASS |
| Service-Impl tests grün (RepaymentEntry) | `cargo test -p genossi_service_impl --lib repayment_entry` | 19 passed | ✓ PASS |
| Service-Impl tests grün (RepaymentPhase Phase-7+8) | `cargo test -p genossi_service_impl --lib repayment_phase` | 23 passed | ✓ PASS |
| REST-Types tests | `cargo test -p genossi_rest_types repayment_entry` | 7 passed | ✓ PASS |
| REST-Handler smoke tests | `cargo test -p genossi_rest repayment_entry` | 3 passed | ✓ PASS |
| E2E-Tests komplett (Phase-7-Baseline + neue Phase-8) | `cargo test --test e2e_tests --features mock_auth` | 270 passed; 0 failed | ✓ PASS |

### Requirements Coverage

Phase 8 Plan-Frontmatter `requirements` aggregiert: ENTR-01, ENTR-02, ENTR-03, ENTR-04, ENTR-05, ENTR-06, PHAS-02, PHAS-03.
REQUIREMENTS.md Z. 86-90 + 93-98 mappt exakt diese 8 IDs auf Phase 8.

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| ENTR-01 | 08-01, 08-02, 08-04, 08-06 | Auto-Fill beim Phase-Öffnen; current_shares-Snapshot | ✓ SATISFIED | Migration + Auto-Fill-Block + E2E `test_open_phase_triggers_auto_fill` |
| ENTR-02 | 08-03, 08-05, 08-06 | Manueller Add via REST | ⚠️ SATISFIED mit Quality-Gap (CR-01) | POST-Handler + Service-Validation; aber stale version in Response |
| ENTR-03 | 08-01, 08-03 | Mehrere Einträge pro Mitglied+Phase | ✓ SATISFIED | Migration ohne UNIQUE-Constraint verified; logisch durch Service ungehindert |
| ENTR-04 | 08-03, 08-05, 08-06 | share_count-Edit nur in {Open, Contacted} | ✓ SATISFIED | Unit-Test test_update_entry_paid_out_returns_conflict; PaidOut-Doppel-Guard in update_repayment_entry |
| ENTR-05 | 08-03, 08-05, 08-06 | Soft-Delete nur wenn Status != PaidOut | ✓ SATISFIED | Pre-Check vor audited_delete!; E2E test_delete_entry_in_open_succeeds; Unit-Tests grün |
| ENTR-06 | 08-03, 08-05, 08-06 | Multi-select Status-Toggle (offen↔angeschrieben) | ⚠️ SATISFIED mit Quality-Gaps (CR-01, CR-02) | Batch-Endpoint + audited_update! N-mal in 1 Tx; aber stale versions in Response + "entry not found" als 409 |
| PHAS-02 | 08-04, 08-05, 08-06 | Open-Phase-Auto-Fill (Phase-7 Skeleton + Phase-8 voll) | ✓ SATISFIED | Auto-Fill-Block in derselben Tx; 6 Unit-Tests + 4 E2E-Tests |
| PHAS-03 | 08-04, 08-05, 08-06 | Close blockt bei pending entries | ✓ SATISFIED | Pending-Validation-Block; 3 Unit-Tests + 2 E2E-Tests (positive + negative path) |

Keine orphaned Requirements: alle in REQUIREMENTS.md auf Phase 8 gemappten IDs (8 IDs) sind in den Plan-Frontmatter-`requirements`-Feldern reflektiert.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `genossi_service_impl/src/repayment_entry.rs` | 266, 444 | Service returnt entity vor DAO-update (stale version) | 🛑 Blocker (CR-01) | API-Response-Vertrag gebrochen; jedes Folge-PUT mit Response-version liefert 409 |
| `genossi_service_impl/src/repayment_phase.rs` | 140, 221, 355, 459 | Gleiche stale-version-Bug-Klasse (Phase-7-Erbe) | 🛑 Blocker (CR-01, gleiche Klasse) | Wie oben für Phase-Lifecycle-Endpoints |
| `genossi_service_impl/src/repayment_entry.rs` | 416 | "entry not found" als Conflict (statt EntityNotFound) | 🛑 Blocker (CR-02) | 404-Semantik in 409 versteckt; Frontend kann Stale-ID nicht von Domain-Konflikt unterscheiden |
| `genossi_rest/src/repayment_entry.rs` | 260-265 | OpenAPI für /batch-status listet 404 nicht | ⚠️ Warning (CR-02-Symptom) | API-Doku unvollständig; Frontend-Devs erkennen den Stale-ID-Fall nicht aus der Doc |
| `genossi_service_impl/src/repayment_entry.rs` | 391-403 | Zweiter target_status-Match nach exhaustivem PaidOut-Check ist dead code | ℹ️ Info (WR-02) | Harmlos heute; rotting risk bei neuer Status-Variante |
| `genossi_service_impl/src/repayment_phase.rs` | 393-401 | Redundanter `.filter(|e| e.deleted.is_none())` nach DAO-Default-Impl | ℹ️ Info (WR-03) | Harmlos; leicht irreführend für Reader |
| `genossi_dao_impl_sqlite/src/repayment_entry.rs` | 102, 139 | `as i64`-Widening ohne debug_assert | ℹ️ Info (WR-04) | Harmlos heute; brüchig falls nicht-validierter Schreibpfad entsteht |
| `genossi_service_impl/src/repayment_entry.rs` | Multiple | `current_user_id` vor `check_permission` | ℹ️ Info (WR-05) | Minor Perf-Inefficiency; rare Identity-Resolution-Failure leakt vor Permission-Reject |
| `genossi_bin/tests/e2e_tests.rs` | 11334-11421 | Kein 2nd-PUT-Test mit version aus 1. PUT | 🛑 Blocker (IN-04, würde CR-01 fangen) | Test-Coverage-Lücke verbirgt CR-01 |

### Human Verification Required

Siehe `human_verification`-Block im Frontmatter.

### Gaps Summary

Die Phase liefert grundsätzlich alle 5 ROADMAP-Success-Criteria und alle 8
Requirements (ENTR-01..06 + PHAS-02 + PHAS-03). 270/270 E2E-Tests sind grün,
DI-Wiring ist sauber (W-02 verifiziert), Auto-Fill + Pending-Validation
funktionieren atomar in der Tx wie spezifiziert, alle Audit-Macros werden
diszipliniert verwendet.

**Aber:** Der Code-Review hat 2 BLOCKER-Bugs identifiziert, die im Codebase
zweifelsfrei bestätigt sind:

1. **CR-01 (stale version response):** Eine systematische Bug-Klasse, die in 6
   Service-Methoden auftritt (RepaymentEntry: update + batch_toggle; RepaymentPhase:
   create + update + open + close). Funktional bricht das den Optimistic-Locking-Vertrag
   für JEDE Folge-PUT-Sequenz. Das ist nicht durch die E2E-Tests gefangen, weil kein
   Test einen 2. PUT mit der Response-version macht (IN-04 dokumentiert den
   Test-Coverage-Mangel). Phase 7 hatte denselben Bug — Phase 8 erbt + propagiert
   ihn in 2 neue Methoden.

2. **CR-02 (404 vs 409 in Batch):** Eine Aggregat-Konsistenz-Verletzung. Im selben
   Aggregat returnen alle anderen Methoden 404 für die "Entry nicht gefunden"-Bedingung,
   der Batch-Toggle aber 409. Das ist konsistent in OpenAPI nicht-dokumentiert.
   Folge: Frontends klassifizieren Stale-ID-Fälle falsch.

Beide Bugs sind kleine Punkt-Fixes (~10 LOC pro Methode für CR-01 nach
MemberServiceImpl-Vorlage, ~1 LOC für CR-02). Beide haben minimal-invasive
Lösungen, die das REVIEW.md detailliert beschreibt. Empfehlung: Fix-Phase
mit Re-Read-Pattern für alle 6 betroffenen Methoden + Mapping-Korrektur in
batch_toggle_status + ergänzender E2E-Regressionstest (siehe `missing`-Listen).

---

*Verified: 2026-05-31T18:00:00Z*
*Verifier: Claude (gsd-verifier)*
