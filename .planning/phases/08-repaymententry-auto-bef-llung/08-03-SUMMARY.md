---
phase: 08-repaymententry-auto-bef-llung
plan: 03
subsystem: service
tags: [rust, service, audit, validation, batch-tx, optimistic-locking, repayment-entry]

requires:
  - phase: 08-repaymententry-auto-bef-llung
    plan: 01
    provides: "RepaymentEntryDao-Trait, RepaymentEntryEntity, RepaymentEntryStatus-Enum, Auditable-Impl"
  - phase: 08-repaymententry-auto-bef-llung
    plan: 02
    provides: "RepaymentEntryDaoImpl SQLite mit Pre-Exists-Check + Optimistic-Locking"

provides:
  - "RepaymentEntryService-Trait mit 6 Methoden (create/update/delete/get/list_by_phase/batch_toggle_status) + #[automock]"
  - "RepaymentEntryServiceImpl mit 7 Deps via gen_service_impl!"
  - "Edit-Matrix: D-05 (PaidOut final), D-06 (Open ↔ Contacted bidirektional), D-07 (PaidOut nicht via PUT/Batch), ENTR-04 (share_count nur in Open/Contacted), ENTR-05 (delete blockt PaidOut)"
  - "Batch-Toggle all-or-nothing in 1 Tx (D-08) mit strukturiertem 409-Body { failure_index, failure_id, failure_reason }"
  - "Validation: Phase-Status=Open (D-11.1), Member existiert (D-11.2), Range >0 AND ≤ current_shares (D-11.3)"
  - "Audit-Disziplin: ALLE Schreib-Ops via audited_create!/update!/delete! — Grep-Gate sauber"
  - "ADMIN_PRIVILEGE-Check als erste DAO-touchende Aktion in jeder Methode (T-08-03-02)"

affects:
  - "08-04 (RepaymentPhase-Service-Erweiterung): open_phase Auto-Fill nutzt RepaymentEntryDao direkt mit audited_create!; close_phase nutzt find_by_phase_id"
  - "08-05 (REST-Handler): TOs leiten von RepaymentEntry-Domain ab; Batch-Conflict-Body wird im REST-Layer als BatchFailureResponse-JSON ausgeliefert"
  - "08-06 (E2E-Tests): Service-Impl wird via DI in RestStateImpl gewired und gegen reale SQLite getestet"
  - "09 (PAYO): mark_paid_out-Endpoint hängt sich an UPDATE-Pfad an; nutzt selbe Status-Enum + Audit-Macro-Konventionen"

tech-stack:
  added: []
  patterns:
    - "Audit-Macros 6/7/6-Arg-Signaturen (audit_macros.rs); audited_delete! lädt Entity intern, daher Pre-Check vor Macro für Status-Guard"
    - "Inline-Validator (Phase-7-Plan-07-03-D-04-Lektion: kein validation.rs-Refactor)"
    - "Hand-rolled mock! statt cross-modul automock-Sharing (Phase-3-Plan-03-Lektion)"
    - "Strukturierter 409-JSON-Body als Arc<str>-Wrap im Conflict-Error (analog CloseConflictResponse, Plan 04/05)"
    - "All-or-nothing Batch-Pattern: Drop-on-Error = Tx-Rollback (Phase-1-assembly.rs Pattern-Anker)"

key-files:
  created:
    - "genossi_service/src/repayment_entry.rs"
    - "genossi_service_impl/src/repayment_entry.rs"
  modified:
    - "genossi_service/src/lib.rs (Modul-Deklaration alphabetisch vor repayment_phase)"
    - "genossi_service_impl/src/lib.rs (Modul-Deklaration alphabetisch vor repayment_phase)"

key-decisions:
  - "PaidOut-Doppel-Guard im update_repayment_entry: erstens Source-Status (entity.status == PaidOut → blockiert), zweitens Target-Status (update.status == Some(PaidOut) → blockiert). Vorteile: explizit getrennte Fehlermeldungen; semantisch klar."
  - "Range-Check im update wird gegen aktuelle Member.current_shares geprüft (nicht gegen alte Member-Snapshot). Begründung: Korrektur eines fehlerhaft erfassten Entries soll gegen den aktuellen Stand validieren; alternative wäre Snapshot-at-create-time, was D-11.3-Semantik bricht."
  - "Test-Mocks hand-rolled mit mock! statt automock-Sharing — Phase-3-Plan-03-Lektion: automock-generierte Mocks aus genossi_dao können nicht über Modul-Grenzen serialisiert werden, hand-rolled mocks haben volle Kontrolle über Lifetimes."
  - "find_by_id für Member im Test direkt gemockt (nicht via dump_all). Grund: mockall überschreibt die DAO-Default-Impl von find_by_id, sodass dump_all-mocks ignoriert werden. Erste Iteration nutzte dump_all → 4 Tests rot mit 'No matching expectation found'; gefixt durch direkte expect_find_by_id-Aufrufe (siehe Issues Encountered)."
  - "Strukturierter 409-Body als serde_json::json!() in Arc<str>-Wrap. REST-Layer (Plan 05) wird diesen JSON-Body 1:1 als Body der 409-Response ausliefern; Frontend (Phase 12) kann failure_index/failure_id/failure_reason strukturiert lesen."

patterns-established:
  - "Service-Impl-Vorlage für Phase-8-Folgepläne (Plan 04 nutzt selbe gen_service_impl!-7-Dep-Struktur + audited_*!-Macros)"
  - "Strukturiertes 409-JSON-Body-Pattern (D-08) wird in Plan 05 vom REST-Layer als BatchFailureResponse-TO formalisiert"

requirements-completed: [ENTR-02, ENTR-03, ENTR-04, ENTR-05, ENTR-06]

duration: ~12min
completed: 2026-05-31
---

# Phase 08 Plan 03: RepaymentEntry-Service-Trait + Service-Impl Summary

**Service-Layer für RepaymentEntry-CRUD plus Batch-Toggle: Validation gegen Phase/Member, Edit-Matrix mit PaidOut-Doppel-Guard, all-or-nothing Batch-Tx mit strukturiertem 409-JSON-Body — 19 grüne Unit-Tests; Plan forderte mind. 14.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-31T04:11:02Z
- **Completed:** 2026-05-31T04:23:11Z
- **Tasks:** 2/2 abgeschlossen
- **Files created:** 2 (Service-Trait, Service-Impl)
- **Files modified:** 2 (lib.rs Modul-Deklarationen)
- **Tests:** 5 (Trait) + 19 (Impl) = 24 Unit-Tests grün

## Accomplishments

- **`RepaymentEntryService`-Trait** mit 6 Methoden (create/update/delete/get/list_by_phase/batch_toggle_status) plus `#[automock]` für MockRepaymentEntryService
- **3 DTOs:** `RepaymentEntrySubmission` (3 Felder), `RepaymentEntryUpdate` (Optional + Pflicht-Version), `RepaymentEntryBatchStatusInput` (Arc<[Uuid]> + target_status)
- **`RepaymentEntryServiceImpl`** mit 7 Deps via `gen_service_impl!`: RepaymentEntryDao, RepaymentPhaseDao, MemberDao, AuditLogDao, PermissionService, UuidService, TransactionDao
- **Inline-Validator `validate_entry_create`** für D-11.3 Range-Check (>0 AND ≤ Member.current_shares); wiederverwendet in create und update
- **6 Service-Methoden komplett:**
  - **create**: D-11.1 (Phase Open) + D-11.2 (Member existiert) + D-11.3 (Range) + audited_create! → Status = Open
  - **update**: PaidOut-Doppel-Guard (D-05 source, D-07 target) + Optimistic-Locking + ENTR-04 share_count-Edit-Window + D-06 bidirektionaler Toggle + audited_update!
  - **delete**: Pre-Check ENTR-05 (Status != PaidOut) + 6-Arg audited_delete! (Macro lädt Entity intern)
  - **get / list_by_phase**: read-only via find_by_id / find_by_phase_id Default-Impl
  - **batch_toggle_status**: D-07 PaidOut-Target → 400 ValidationError; D-08 all-or-nothing in 1 Tx; strukturierter 409-JSON-Body `{ failure_index, failure_id, failure_reason }`
- **Audit-Disziplin:** ALLE Schreib-Ops via `audited_*!`-Macros — Grep-Gate vor Commit verifiziert (0 direkte DAO-write-Aufrufe ausserhalb Macro-Expansion)
- **Permission-Check** als erste DAO-touchende Aktion in jeder Methode (T-08-03-02 mitigiert)
- **19 Unit-Tests grün** in genossi_service_impl::repayment_entry::tests:
  - 5 Create-Tests (alle 4 Validation-Pfade + Happy)
  - 5 Update-Tests (PaidOut source/target, Open↔Contacted, version mismatch)
  - 2 Delete-Tests (PaidOut blockiert, Open succeeds)
  - 3 Batch-Tests (PaidOut target rejected, all-or-nothing JSON-Body, 3x update happy)
  - 4 Permission-Tests (create/update/delete/batch_toggle verlangen admin)

## Task Commits

Jede Task atomar committed:

1. **Task 1: Service-Trait + DTOs in genossi_service** — `9b81e2d` (feat)
2. **Task 2: Service-Impl mit Validation + Batch-Tx in genossi_service_impl** — `af575c4` (feat)

**Plan metadata:** *(folgt mit dem nächsten Commit)*

## Files Created/Modified

- `genossi_service/src/repayment_entry.rs` (285 LOC): Domain-Typ + From-Impls + 3 DTOs + Trait-Definition + 5 Unit-Tests
- `genossi_service_impl/src/repayment_entry.rs` (~1500 LOC nach rustfmt): Doc-Header + Imports + Process-Konstanten + gen_service_impl! + validate_entry_create + 6 Service-Methoden + Test-Modul mit 6 hand-rolled mocks + 19 Tokio-Tests
- `genossi_service/src/lib.rs`: +1 LOC `pub mod repayment_entry;` alphabetisch vor `pub mod repayment_phase;`
- `genossi_service_impl/src/lib.rs`: +1 LOC `pub mod repayment_entry;` alphabetisch vor `pub mod repayment_phase;`

## Decisions Made

Alle wesentlichen Decisions kamen aus `08-CONTEXT.md` (D-05/D-06/D-07/D-08/D-11/D-12), `08-PATTERNS.md §4-§5` und dem PLAN-Block, und wurden 1:1 umgesetzt.

Klarstellungen während der Implementierung:

- **PaidOut-Doppel-Guard im update:** Erstens Source-Status-Guard (`entity.status == PaidOut` → blockiert mit "Cannot update: entry is PaidOut; final per PAYO-04"), zweitens Target-Status-Guard (`update.status == Some(PaidOut)` → blockiert mit "PaidOut transition must use Phase-9 mark_paid_out endpoint"). Vorteile: explizite Fehlermeldungen; semantisch klar getrennte Fehlerursachen für REST-Layer.
- **Range-Check im update gegen aktuelle Member.current_shares** (nicht gegen Snapshot zum Zeitpunkt der ursprünglichen Anlage). Begründung: Korrekturen müssen gegen den aktuellen Member-Stand validieren; Snapshot-at-create-time würde D-11.3-Semantik brechen.
- **Strukturierter 409-Body als serde_json::json!() in Arc<str>:** Im Service als `serde_json::json!({...}).to_string()` gebaut und in `Arc<str>` verpackt; REST-Layer (Plan 05) wird diesen JSON-Body 1:1 in der 409-Response ausliefern. Test `test_batch_toggle_all_or_nothing_on_failure` parst den Body als `serde_json::Value` und prüft `failure_index == 1`, `failure_id == id2.to_string()`, `failure_reason.contains("source status")`.
- **Hand-rolled `mock!`-Mocks statt cross-modul `automock`:** Wegen Phase-3-Plan-03-Lektion (automock-Mocks aus genossi_dao können nicht über Modul-Grenzen geteilt werden). Voller Aufbau aller 6 Mocks (TestTxDao, TestRepaymentEntryDao, TestRepaymentPhaseDao, TestMemberDao, TestAuditLogDao, TestPermissionService) plus TestTransaction + StaticUuidService analog `repayment_phase.rs::tests`.

## Deviations from Plan

None — plan executed exactly as written.

Drei Hinweise zur Vollständigkeit:

1. **19 Tests statt 14:** Plan forderte „mindestens 14 grüne Unit-Tests". Implementiert wurden 15 Functional-Tests + 4 Permission-Tests = 19. Die 4 Permission-Tests (`test_*_requires_admin_privilege` für create/update/delete/batch_toggle) sind als T-08-03-02-Verteidigung gelistet — explizit für jede der 4 mutating Methoden.
2. **rustfmt angewendet:** Datei wurde mit `rustfmt --edition 2021` aus `/nix/store/...rustfmt-preview-1.90.0...` formatiert (cargo fmt ist auf dem System nicht installiert; Memory-Notiz "Nix-Toolchain nicht sofort aufgeben"). Kein Verhaltens-Impact, nur Code-Style. Tests blieben nach Format grün.
3. **Workspace-Build durchgeführt:** Zusätzlich zur in den Acceptance Criteria geforderten `cargo build -p genossi_service && cargo build -p genossi_service_impl` habe ich `cargo build --workspace --all-features` ausgeführt, um sicherzustellen, dass das neue Modul nicht versehentlich downstream-Crates bricht. Ergebnis: clean, nur pre-existing Warnings in genossi_mail, genossi_rest, genossi_bin (alle ausserhalb des Plan-Scopes).

## Issues Encountered

- **mockall überschreibt DAO-Default-Impl:** Erste Iteration nutzte `member_dao.expect_dump_all().returning(...)` für die Tests, in der Annahme dass die `find_by_id`-Default-Impl auf dem `MemberDao`-Trait (die `dump_all` aufruft) auch via Mock funktioniert. **Tatsächlich überschreibt das `mock!`-Macro die Default-Impl** — der gemockte `find_by_id`-Stub (auch wenn nicht explizit gesetzt) wird statt der Default-Impl verwendet, und ohne `expect_find_by_id` fehlt die Erwartung. 4 Tests (test_create_entry_rejects_when_member_not_found, test_create_entry_validation_rejects_share_count_zero_or_negative, test_create_entry_validation_rejects_share_count_exceeds_member_current_shares, test_create_entry_success) waren rot mit "MockTestMemberDao::find_by_id(...): No matching expectation found". **Fix (Rule 1 — Bug):** alle 4 Tests umgestellt auf `member_dao.expect_find_by_id().returning(...)`. Test_create_entry_rejects_when_member_not_found returnt `Ok(None)`, die 3 Range-Tests returnen `Ok(Some(member_with_current_shares(N)))`. Alle 19 Tests grün nach Fix.

## User Setup Required

None — Service-Impl integriert sich automatisch über Plan 04 (Phase-Service-Erweiterung) und Plan 06 (DI-Wiring im RestStateImpl::new()) beim Server-Start. Keine externen Service-Konfigurationen, keine Environment-Variablen, keine manuellen Schritte.

## Next Phase Readiness

- **Plan 04 (RepaymentPhase-Service-Erweiterung):** Foundation komplett. `RepaymentEntryDao` mit `find_by_phase_id`-Default-Impl ist verfügbar; `RepaymentEntryServiceImpl`-Konstanten (REPAYMENT_ENTRY_PROCESS_CREATE) können von open_phase Auto-Fill direkt verwendet werden — alternativ nutzt Plan 04 die Phase-Konvention `REPAYMENT_PHASE_PROCESS_OPEN` (CONTEXT D-03).
- **Plan 05 (REST-Handler):** TOs müssen vom Domain-Typ `RepaymentEntry` abgeleitet werden; `BatchFailureResponse`-TO im `genossi_rest_types` formalisiert das strukturierte 409-Body-Schema. PaidOut-409-Conflicts werden im REST-Layer 1:1 mit der serialisierten ServiceError-Message ausgeliefert.
- **Keine Blocker.**

## Threat Coverage

| Threat ID | Mitigation | Verified-by |
|-----------|------------|-------------|
| T-08-03-01 (Direct DAO write bypassing audit chain) | Audit-Disziplin-Grep-Gate `grep -nE "repayment_entry_dao\.(create\|update)\b" genossi_service_impl/src/*.rs` ergab 0 Treffer; Inline-Comment "audited via macro" an jeder Macro-Call-Stelle | Grep-Gate post-commit + Code-Inspektion |
| T-08-03-02 (Privilege escalation) | `permission_service.check_permission(ADMIN_PRIVILEGE, ...)` als erste DAO-touchende Aktion in jeder der 6 Methoden | Tests test_create_entry_requires_admin_privilege, test_update_entry_requires_admin_privilege, test_delete_entry_requires_admin_privilege, test_batch_toggle_requires_admin_privilege (4 mutating; get/list verifiziert per Code-Review) |
| T-08-03-03 (Validation bypass on share_count_to_pay_out) | validate_entry_create läuft VOR audited_create!; Tests verifizieren times(0) auf DAO.create bei ValidationError | test_create_entry_validation_rejects_share_count_zero_or_negative + test_create_entry_validation_rejects_share_count_exceeds_member_current_shares |
| T-08-03-04 (Invalid state transition: PaidOut via PUT/Batch) | PUT-Pfad: update_repayment_entry rejects target=PaidOut mit Conflict; Batch-Pfad: rejects target=PaidOut mit ValidationError 400 | test_update_entry_status_to_paid_out_via_put_returns_conflict + test_batch_toggle_paid_out_target_returns_validation_error |
| T-08-03-05 (Batch partial-success leaves entries inconsistent) | All-or-nothing in 1 Tx; erster Fehler triggert Drop = Rollback; strukturierter JSON-Body identifiziert Failing-Entry | test_batch_toggle_all_or_nothing_on_failure (Body-Assertion auf failure_index/failure_id/failure_reason) |

## Self-Check: PASSED

**Verified files exist:**
- `genossi_service/src/repayment_entry.rs`: FOUND
- `genossi_service_impl/src/repayment_entry.rs`: FOUND
- `genossi_service/src/lib.rs` (modified): FOUND
- `genossi_service_impl/src/lib.rs` (modified): FOUND

**Verified commits exist:**
- `9b81e2d` (Task 1): FOUND in git log
- `af575c4` (Task 2): FOUND in git log

**Verified tests pass:**
- 5/5 in `genossi_service::repayment_entry::tests`: passed
- 19/19 in `genossi_service_impl::repayment_entry::tests`: passed
- `cargo build --workspace --all-features`: clean (nur pre-existing warnings in genossi_mail/genossi_rest/genossi_bin, ausserhalb Plan-Scope)

**Verified acceptance criteria (grep counts):**
- `pub trait RepaymentEntryService` == 1 ✓
- `async fn create_repayment_entry` == 1 ✓ (in trait)
- `async fn batch_toggle_status` == 1 ✓ (in trait)
- `async fn list_repayment_entries_by_phase` == 1 ✓
- `pub struct RepaymentEntryBatchStatusInput` == 1 ✓
- `pub mod repayment_entry;` in genossi_service/src/lib.rs == 1 ✓
- `const REPAYMENT_ENTRY_PROCESS_CREATE` == 1 ✓
- `const REPAYMENT_ENTRY_PROCESS_BATCH_TOGGLE` == 1 ✓
- `fn validate_entry_create` == 1 ✓
- `gen_service_impl!` == 1 ✓
- `audited_create!` == 2 (Code + Comment) ✓ (>= 1)
- `audited_update!` == 6 (Code + Comments) ✓ (>= 2)
- `audited_delete!` == 6 (Code + Comments) ✓ (>= 1)
- `failure_index` == 4 ✓ (W-05 strukturierter Body)
- `failure_reason` == 6 ✓
- `use genossi_service::{ServiceError` == 1 ✓ (B-02 fix: aus genossi_service, NICHT genossi_dao)
- `as ServiceErr` == 0 ✓ (kein Alias)
- `pub mod repayment_entry;` in genossi_service_impl/src/lib.rs == 1 ✓
- Audit-Grep-Gate: 0 direkte DAO-write-Aufrufe ausserhalb audited_*!-Macros ✓

---

*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
