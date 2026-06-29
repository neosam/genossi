---
phase: 17-service-rest-uebertrag-cascade
plan: 02
subsystem: service/membership-adjust
tags: [rust, service-impl, cascade, audit, single-transaction, transfer]
requires:
  - "genossi_service::membership_adjust::MembershipAdjustService::transfer_shares trait method (Plan 17-01)"
  - "TRANSFER_PROCESS constant from Plan 17-01"
  - "validate_transfer_inputs pure function from Plan 17-01"
  - "validate_willensbekundung_date pure function from Phase 15 (D-15-05)"
  - "recalc_dates Free-Function from genossi_service_impl::member_action (D-15-04)"
  - "audited_create! + audited_update! macros from genossi_service_impl::audit_macros"
provides:
  - "MembershipAdjustService::transfer_shares full Single-Tx Cascade-Pipeline impl (frozen behavior; 10 Mock-Tests gate)"
  - "Service-level test scaffolding (allow_admin_perms_transfer, allow_audit_log_transfer, transfer_test_date helpers)"
affects:
  - "Plan 17-03 (REST): kann auf das fertige Service-Verhalten aufsetzen ohne weitere Spec-Validierung"
  - "Plan 17-04 (E2E): kann sich auf Integration-Aspekte konzentrieren (Audit-Trail-Hashchain, Race-Conditions, DB-Transaktions-Atomarität)"
tech-stack:
  added: []
  patterns:
    - "Pre-write Voll-Uebertrag-Detection via will_become_zero before any audited_*! call (D-17-01)"
    - "Single-Tx Cascade: use_transaction once, tx.clone() for ALL intermediate steps, plain tx on commit (D-17-04 / AUDT-02)"
    - "Mock-Capture via Arc<Mutex<Vec<MemberActionEntity>>> for verstaerkte D-17-03 field-level assertions"
    - "Re-Read both members after recalc_dates for response (Schritt 15)"
key-files:
  created: []
  modified:
    - genossi_service_impl/src/membership_adjust.rs
decisions:
  - "Pipeline-Reihenfolge minimal von CONTEXT.md angepasst: validate_transfer_inputs läuft NACH from-Load, da from.current_shares benötigt wird. validate_willensbekundung_date läuft trotzdem VOR to-Load (Plan-Vorgabe behalten)."
  - "audited_update! lädt intern die Old-Entity via find_by_id (siehe audit_macros.rs:43-50). Führt zu zusätzlichen find_by_id-Mock-Calls auf member_dao: 4x from + 3x to im Happy-Path."
  - "recalc_dates wird nur für from.id aufgerufen — to.exit_date kann durch Empfang nicht negativ beeinflusst werden, weil PERM-03 schon vor allen Writes sichergestellt hat, dass to nicht gekuendigt ist (D-17-02)."
  - "Voll-Uebertrag-Branch erzeugt den Austritt-Eintrag NACH den Member-Updates, damit der Audit-Trail (Action-Abgabe, Action-Empfang, Member-from-Update, Member-to-Update, Action-Austritt) in chronologisch konsistenter Reihenfolge geschrieben wird."
  - "Tests nutzen .withf(|id, _| *id == from_id) + .times(N) zur Disambiguierung mehrerer find_by_id-Expectations auf demselben Mock (Lessons-Learned aus Phase-15-Mock-Tests)."
metrics:
  duration: "~30min"
  completed: "2026-06-06"
  tasks: 2
  files_modified: 1
---

# Phase 17 Plan 02: 15-Schritt-Cascade-Pipeline + 10 Mock-Tests Summary

Ersetzt den `unimplemented!()`-Stub aus Plan 17-01 durch die volle 15-Schritt-Single-Tx-Cascade-Pipeline und friert das Verhalten mit 10 Mock-basierten Unit-Tests ein. TRSF-01..05, AUDT-02 und PERM-03 sind vollständig in der Service-Layer abgedeckt; Plan 17-03 (REST) und Plan 17-04 (E2E) können direkt aufbauen ohne Aengste vor Service-Drift.

## Was wurde gebaut

### Task 1 — 15-Schritt-Pipeline implementiert

- `transfer_shares` Methoden-Body in `genossi_service_impl/src/membership_adjust.rs` Zeilen 493-694 (war vorher 16-Zeilen-Stub).
- Pipeline-Reihenfolge (D-17-01..10):
  1. `transaction_dao.use_transaction(tx).await?` (Tx-Begin)
  2. `permission_service.current_user_id` mit SYSTEM-Fallback
  3. `permission_service.check_permission(ADMIN_PRIVILEGE, context)` (D-17 Schritt 1 / PERM-01)
  4. `member_dao.find_by_id(from_id)` (EntityNotFound bei None)
  5. `validate_transfer_inputs(from_id, to_id, shares, from.current_shares)` — Pure-Function-Call aus Plan 17-01
  6. `validate_willensbekundung_date(transfer_date, today)` — Reuse Phase 15
  7. `member_dao.find_by_id(to_id)` (EntityNotFound bei None)
  8. PERM-03: `if to.exit_date.is_some() return Err(ServiceError::Conflict("recipient already cancelled"))` (D-17-07)
  9. `let will_become_zero = from.current_shares - shares == 0` (D-17-01 Pre-write-Detection)
  10. `audited_create!(member_action_dao, abgabe_entity, TRANSFER_PROCESS, user_id, tx)`
  11. `audited_create!(member_action_dao, empfang_entity, TRANSFER_PROCESS, user_id, tx)`
  12. `audited_update!(member_dao, from.id, from_updated{current_shares -= shares}, TRANSFER_PROCESS, user_id, tx)`
  13. `audited_update!(member_dao, to.id, to_updated{current_shares += shares}, TRANSFER_PROCESS, user_id, tx)`
  14. Optional `audited_create!(member_action_dao, austritt_entity, ...)` mit `transfer_member_id=Some(to_id)` + `effective_date=Some(transfer_date)` (D-17-03 / TRSF-05)
  15. `crate::member_action::recalc_dates(member_dao, member_action_dao, from.id, tx.clone())` (D-17-02 — EXAKT EINMAL, nur für from)
  16. Re-Read `from_final` + `to_final` via `find_by_id`
  17. `transaction_dao.commit(tx)`
  18. Return `(Vec<MemberAction>, Member, Member)` Tuple
- `#[allow(dead_code)]` Marker auf `TRANSFER_PROCESS` (Zeile 43) und `validate_transfer_inputs` (Zeile 606 alt) entfernt, weil die Pipeline beide jetzt nutzt.
- Imports nicht geändert — `MemberActionEntity` + `ActionType` waren bereits am Top-of-File.

**Commit:** `5ae8bc9` — feat(17-02): transfer_shares 15-Schritt-Cascade-Pipeline implementieren

### Task 2 — 10 Mock-Unit-Tests

Alle Tests im bestehenden `#[cfg(test)] mod service_tests` als Append nach dem letzten partial_repayment-Test eingefügt (membership_adjust.rs Zeilen 2523-3142):

| #   | Test                                                              | Was wird geprüft                                                                                                              |
| --- | ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_transfer_shares_partial_happy_path_calls_two_creates_two_updates`   | Teil-Uebertrag (5/2): 2 Actions in Result, 2 `audited_create`, 2 `audited_update`, alle mit TRANSFER_PROCESS         |
| 2   | `test_transfer_shares_full_branch_creates_austritt`               | Voll-Uebertrag (3/3): 3 Actions, 3. ist `ActionType::Austritt` mit `transfer_member_id=Some(to_id)` + `effective_date=Some(transfer_date)` (Arc-Mutex-Capture) |
| 3   | `test_transfer_shares_validation_n_zero_rejected_no_dao_writes`   | shares=0 → `ValidationError(field=shares)`; KEIN `create`/`update` Call (mockall `.times(0)` self-validating)                |
| 4   | `test_transfer_shares_validation_n_negative_rejected`             | shares=-1 → `ValidationError`; KEIN write                                                                                       |
| 5   | `test_transfer_shares_validation_n_exceeds_rejected`              | shares=6 > current=5 → `ValidationError(message contains "exceeds")`; KEIN write                                                |
| 6   | `test_transfer_shares_validation_self_transfer_rejected`          | from_id == to_id → `ValidationError(field=to_member_id)`; KEIN write                                                            |
| 7   | `test_transfer_shares_recipient_cancelled_returns_conflict`       | to.exit_date.is_some() → `Conflict("recipient already cancelled")` (PERM-03); KEIN write nach from+to-Load                       |
| 8   | `test_transfer_shares_permission_denied_no_dao_load`              | `check_permission` Err → `PermissionDenied`; **KEIN** `find_by_id` Call                                                          |
| 9   | `test_transfer_shares_from_not_found_returns_entity_not_found`    | from-Load Ok(None) → `EntityNotFound(from_id)`; KEIN write                                                                       |
| 10  | `test_transfer_shares_to_not_found_returns_entity_not_found`      | to-Load Ok(None) → `EntityNotFound(to_id)`; from wurde 1x geladen, dann KEIN write                                              |

**Mock-Counts pro Test** (zur Referenz für Plan 17-04 E2E-Cross-Check, falls real-DB-Pfad andere Interaktionen sieht):

| Test                              | member_dao.find_by_id | member_dao.update | member_dao.update_dates | member_action_dao.create | member_action_dao.find_by_member_id | audit_log_dao.get_latest_hash / create_entries |
| --------------------------------- | --------------------- | ----------------- | ----------------------- | ------------------------ | ----------------------------------- | ---------------------------------------------- |
| Test 1 (Teil happy)               | 4 from + 3 to = 7     | 2                 | 1                       | 2                        | 1                                   | any                                            |
| Test 2 (Voll happy)               | 4 from + 3 to = 7     | 2                 | 1                       | 3 (mit Capture)          | 1                                   | any                                            |
| Test 3-5 (Validation, post-from)  | 1 from                | 0                 | 0                       | 0                        | 0                                   | any                                            |
| Test 6 (Self-Transfer)            | 1                     | 0                 | 0                       | 0                        | 0                                   | any                                            |
| Test 7 (PERM-03 Conflict)         | 1 from + 1 to = 2     | 0                 | 0                       | 0                        | 0                                   | any                                            |
| Test 8 (Permission Denied)        | **0**                 | 0                 | 0                       | 0                        | 0                                   | none (MockAuditLogDao::new)                    |
| Test 9 (from not found)           | 1                     | 0                 | 0                       | 0                        | 0                                   | any                                            |
| Test 10 (to not found)            | 1 from + 1 to = 2     | 0                 | 0                       | 0                        | 0                                   | any                                            |

Warum 4x find_by_id(from)? Schritt 4 (existence) + Schritt 12 audited_update! internal old-load + Schritt 15 recalc_dates internal load + Schritt 16 re-read.
Warum 3x find_by_id(to)? Schritt 7 (existence) + Schritt 13 audited_update! internal old-load + Schritt 16 re-read.

**Commit:** `180e500` — test(17-02): Mock-Unit-Tests fuer transfer_shares-Pipeline (10 Faelle)

## Referenz-Punkte für Plan 17-03 (REST) und Plan 17-04 (E2E)

- **Service-Trait-Signatur** (eingefroren in Plan 17-01): `genossi_service/src/membership_adjust.rs` Zeilen 83-91.
- **Service-Impl** (jetzt fertig): `genossi_service_impl/src/membership_adjust.rs` Zeilen 493-694 (Method-Body inkl. Doc-Kommentar).
- **TRANSFER_PROCESS-Konstante**: `genossi_service_impl/src/membership_adjust.rs` Zeile 43 (jetzt ohne `#[allow(dead_code)]`).
- **validate_transfer_inputs**: gleiches Modul Zeile 605+ (jetzt ohne `#[allow(dead_code)]`).
- **Mock-Test-Scaffolding**: gleicher Modul, `service_tests` submod, Helper `allow_admin_perms_transfer` Zeile 2550, `allow_audit_log_transfer` Zeile 2561, `transfer_test_date` Zeile 2570.

## Pipeline-Reihenfolge-Abweichung von CONTEXT.md

CONTEXT.md listet 15 Pipeline-Schritte; meine Implementierung hat einen kleinen Mehrwert in der Reihenfolge:

| CONTEXT.md Schritt | Implementierung                                              | Grund                                                                                                                  |
| ------------------ | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| 1. Permission      | OK                                                           |                                                                                                                        |
| 2. validate inputs | Nach from-Load                                               | `validate_transfer_inputs` braucht `from.current_shares` als Parameter (Plan-17-01-Pure-Function-Signatur)               |
| 3. validate date   | Vor to-Load                                                  | Plan-Vorgabe behalten — kein Grund, das Datum erst nach to zu prüfen                                                |
| 4. from + to load  | from VOR validate-inputs; to NACH validate-inputs + validate-date | Sequenziert um die Pure-Function-Dependency aus Plan 17-01                                                              |
| 5. PERM-03         | Nach to-Load                                                 | Prüft `to.exit_date` — to muss vorher geladen sein                                                                  |
| 6-15 Cascade       | OK 1:1 wie CONTEXT.md                                        |                                                                                                                        |

Diese Anpassung steht **nicht** in Konflikt mit CONTEXT.md, die explizit erlaubt "validate_transfer_inputs muss nach from-Load laufen" (must-haves Truth #4).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] Worktree-Path nicht git-tracked**

- **Found during:** Task 1 commit attempt
- **Issue:** Der Agent-Arbeitsverzeichnis (`/home/neosam/programming/rust/projects/genossi3/.claude/worktrees/agent-a3555c691635fc4fc/`) ist eine separate Datei-Kopie ohne eigenes `.git`. `git status` operiert gegen den Parent-Repo (`/home/neosam/programming/rust/projects/genossi3/`), aber meine `Edit`-Aufrufe haben Dateien in der Worktree-Kopie modifiziert.
- **Fix:** Nach `Edit` jeweils `cp <worktree-Pfad> <main-repo-Pfad>` ausgeführt; danach `git add` + `git commit` im main-repo. Sowohl Tasks 1 als auch 2 wurden so committed (5ae8bc9 + 180e500).
- **Files modified:** keine Code-Änderung; nur Bash-Workflow-Anpassung.
- **Impact:** Keine; die Commits sind sauber in der detached-HEAD-History und enthalten die kompletten Änderungen.

**2. [Rule 3 - Blocker] `SQLX_OFFLINE=true` weiterhin nötig**

- **Found during:** Beide Tasks Build+Test
- **Issue:** Wie in Plan 17-01 — `genossi_dao_impl_sqlite` braucht eine SQLite-DB für SQLx-Compile-Time-Checks; im Worktree existiert keine `genossi.db`.
- **Fix:** `SQLX_OFFLINE=true` als Env-Variable bei `cargo build`/`cargo test`/`cargo clippy`. Kein Code-Change.
- **Impact:** Plan 17-03 (REST) und Plan 17-04 (E2E) werden ebenfalls `SQLX_OFFLINE=true` brauchen, sofern die Worktree-Umgebung gleich bleibt.

## Verification Results

| Check                                                                                              | Erwartung               | Result        |
| -------------------------------------------------------------------------------------------------- | ----------------------- | ------------- |
| `cargo build --workspace --all-features`                                                           | exit 0                  | exit 0      |
| `cargo test -p genossi_service_impl --lib membership_adjust`                                       | 0 fail                  | 55 pass / 0 fail / 0 ignored |
| `cargo test -p genossi_service_impl --lib membership_adjust::service_tests::test_transfer_shares`  | 10 pass                 | 10 pass / 0 fail |
| `cargo clippy -p genossi_service_impl --all-targets` errors                                        | 0                       | 0           |
| `grep -c 'TRANSFER_PROCESS' membership_adjust.rs`                                                  | >= 6                    | 8           |
| `grep -c 'will_become_zero' membership_adjust.rs`                                                  | >= 2                    | 3           |
| `grep -c 'unimplemented!' membership_adjust.rs`                                                    | 0                       | 0           |
| `grep -c 'ActionType::UebertragungAbgabe' membership_adjust.rs`                                    | >= 1                    | 1           |
| `grep -c 'ActionType::UebertragungEmpfang' membership_adjust.rs`                                   | >= 1                    | 1           |
| `grep -c 'recipient already cancelled' membership_adjust.rs`                                       | >= 1                    | 1 (Service) + 1 (Test) = matches; counted as 1 in Service-Code |
| `grep -c 'effective_date: Some(transfer_date)' membership_adjust.rs`                               | >= 1                    | 1           |
| `grep -c 'recalc_dates' membership_adjust.rs`                                                      | >= 1                    | 13 (inkl. Doc + Tests + Macro-Aufrufe) |
| Task-2 grep: `test_transfer_shares_partial_happy_path`                                             | >= 1                    | 1           |
| Task-2 grep: `test_transfer_shares_full_branch_creates_austritt`                                   | >= 1                    | 1           |
| Task-2 grep: `test_transfer_shares_validation_self_transfer`                                       | >= 1                    | 1           |
| Task-2 grep: `test_transfer_shares_recipient_cancelled`                                            | >= 1                    | 1           |
| Task-2 grep: `test_transfer_shares_permission_denied`                                              | >= 1                    | 1           |
| Task-2 grep: `test_transfer_shares_from_not_found`                                                 | >= 1                    | 1           |
| Task-2 grep: `test_transfer_shares_to_not_found`                                                   | >= 1                    | 1           |
| WARNING #2 D-17-03: `transfer_member_id == Some(to`                                                | >= 1                    | 2           |
| WARNING #2 D-17-03: `effective_date == Some(transfer_date)`                                        | >= 1                    | 2           |

**AUDT-02-Grep-Gate**: alle `audited_create!`/`audited_update!`-Macro-Calls referenzieren genau eine der bekannten Process-String-Konstanten (`TRANSFER_PROCESS`, `CANCEL_PROCESS`, `UPGRADE_PROCESS`, `PARTIAL_REPAYMENT_PROCESS`, `REPAYMENT_PHASE_CREATE_PROCESS`). Die Phase-17-Calls nutzen ausschließlich `TRANSFER_PROCESS`.

## Success Criteria Status

- [x] 15-Schritt-Pipeline implementiert, `unimplemented!()` entfernt
- [x] 10 Mock-Unit-Tests covern alle Branches (Teil, Voll, 4x Validation, PERM-03, PermissionDenied, 2x NotFound)
- [x] TRSF-01 (atomare 2-Action-Cascade) verifiziert in Test 1
- [x] TRSF-02 (gemeinsamer Process-String) verifiziert via `withf(|_, process, _| process == "member-adjust.transfer")` Mock-Filter in Tests 1+2
- [x] TRSF-03 (Voll-Uebertrag-Austritt) verifiziert in Test 2 mit D-17-03-verstaerkter Field-Assertion
- [x] TRSF-04 (current_shares-Update) verifiziert via 2x `member_dao.expect_update().times(2)` in Test 1+2
- [x] TRSF-05 (sofort wirksam) verifiziert via `effective_date == Some(transfer_date)` in Test 2
- [x] AUDT-02 (shared Audit-Process) durch grep-gate + Mock-Filter verifiziert
- [x] PERM-03 (Empfänger-Aktiv-Check) verifiziert in Test 7
- [x] Plan 17-03 (REST) und Plan 17-04 (E2E) können aufbauen — Service-Trait-Signatur ist eingefroren, Service-Verhalten durch 10 Mock-Tests gepinnt

## Self-Check: PASSED

- File `genossi_service_impl/src/membership_adjust.rs` → FOUND (mit `transfer_shares` Body Zeilen 493-694, 10 neue Tests ab Zeile 2581)
- Commit `5ae8bc9` → FOUND in git log (feat(17-02): transfer_shares 15-Schritt-Cascade-Pipeline implementieren)
- Commit `180e500` → FOUND in git log (test(17-02): Mock-Unit-Tests fuer transfer_shares-Pipeline)
