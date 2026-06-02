---
phase: 08-repaymententry-auto-bef-llung
plan: 08
subsystem: api
tags: [rust, axum, sqlx, optimistic-locking, audit, bug-fix, gap-closure, regression-test, lifecycle]

# Dependency graph
requires:
  - phase: 08-repaymententry-auto-bef-llung
    provides: "RepaymentPhaseServiceImpl + 4 lifecycle methods + audited_create!/update! macros (Plans 07-01..03)"
  - phase: 08-repaymententry-auto-bef-llung
    provides: "MemberServiceImpl re-read pattern at member.rs:343-348 (canonical template, Phase 07-erbe)"
  - phase: 08-repaymententry-auto-bef-llung
    provides: "RepaymentEntryServiceImpl 08-07 Re-Read fix (parallel sibling pattern)"
provides:
  - "Re-Read after audited_create! in create_repayment_phase: clients receive the DAO-generated persisted version-UUID instead of the locally-constructed pre-DAO one"
  - "Re-Read after audited_update! in update_repayment_phase: clients receive the DAO-generated post-update version-UUID instead of the pre-update stale one"
  - "Re-Read of Phase entity after audited_update! + Auto-Fill loop in open_repayment_phase (NACH /PHAS-02, VOR commit) — Phase-Row Snapshot ist single-snapshot-konsistent"
  - "Re-Read after audited_update! in close_repayment_phase: clients receive the DAO-generated post-close version-UUID + status=Closed"
  - "Two CR-01 regression tests dokumentieren das Re-Read-Verhalten dauerhaft für update + open"
  - "Bestehende 6 Tests auf mockall::Sequence umgestellt, die Re-Read-Mock-Sequenz wird damit explizit dokumentiert"
affects: [09-payout-cascade, 11-export, 12-frontend-repayment, future-PUT-flows-on-RepaymentPhase]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Re-Read-after-audited_*! (canonical pattern from MemberServiceImpl; now applied to all 4 RepaymentPhase lifecycle methods)"
    - "mockall::Sequence für strikt-sequenzielle Mock-Erwartungen im Triple-find_by_id-Pfad (pre-update + audit-load + Re-Read)"
    - "Re-Read NACH Auto-Fill-Loop, VOR commit für open_repayment_phase — Single-Snapshot-Konsistenz innerhalb der Tx (T-08-08-01 Mitigation)"

key-files:
  modified:
    - genossi_service_impl/src/repayment_phase.rs

key-decisions:
  - "Re-Read pattern 1:1 wörtlich aus MemberServiceImpl::update (member.rs:343-348) übernommen — keine Variation; gleiche Form wie 08-07 für RepaymentEntry"
  - "Re-Read läuft in derselben Transaction wie das audited_*! (tx.clone()), damit der Snapshot single-snapshot-konsistent ist (T-08-08-01 Mitigation)"
  - "Für open_repayment_phase: Re-Read NACH Auto-Fill-Loop, VOR commit (nicht zwischen audited_update! und Auto-Fill) — endgültig persistierte Phase-Row wird zurückgegeben, und das Pattern bleibt einheitlich mit den anderen 3 Methoden (T-08-08-02 Mitigation)"
  - "Auto-Fill-Block in open_repayment_phase bleibt funktional UNVERÄNDERT — alle N audited_create!-Calls weiterhin innerhalb der Tx, vor dem Re-Read; bei Fehler im Auto-Fill droppt die ganze Tx via ?-early-return, Re-Read wird nie erreicht (T-08-08-01 Mitigation)"
  - "Sechs bestehende Tests wurden angepasst: test_create_repayment_phase_success (plain expect_find_by_id für Re-Read; kein pre-load in create-Flow), test_update_repayment_phase_share_value_change_in_open_succeeds + test_open_phase_auto_fill_zero_members + test_open_phase_auto_fill_creates_entries_for_matching_members + test_close_phase_with_zero_entries_succeeds + test_close_phase_with_only_paid_out_or_deleted_succeeds (alle auf mockall::Sequence mit 4 Mock-Erwartungen pro Lifecycle-Operation)"
  - "Bei EntityNotFound vom Re-Read wird ServiceError::EntityNotFound zurückgegeben (kein Conflict), weil die Entity gerade in derselben Tx geschrieben wurde — None hier wäre ein interner Konsistenzfehler, kein User-Race"

patterns-established:
  - "Re-Read-after-audited_*!: jede Service-Methode die nach audited_create!/update! eine Entity an den Client returnt MUSS sie via find_by_id mit tx.clone() erneut lesen, sonst stale version-UUID. Pattern jetzt etabliert für Member, RepaymentEntry und RepaymentPhase — Konvention für künftige Service-Implementationen"
  - "mockall::Sequence für audit-Pfad-Tests: pre-update find_by_id, audited_*!-internal find_by_id, DAO.update/create, post-update find_by_id — 4 Mock-Erwartungen in fester Reihenfolge pro Update-Operation"
  - "Re-Read-NACH-Auto-Fill-Pattern: in Service-Methoden die mehrere Aggregate atomar mutieren (z.B. open_repayment_phase: Phase + N RepaymentEntries), kommt der Re-Read der primären Entity NACH allen sekundären Mutationen, aber VOR commit — Single-Snapshot-Konsistenz"
  - "CR-01-Marker-Konvention: alle Re-Read-Fix-Stellen tragen den exakten String 'CR-01 Fix' im Doc-Comment für grep-basierte Verification"

requirements-completed: [PHAS-02, PHAS-03]

# Metrics
duration: 7min
completed: 2026-05-31
---

# Phase 08 Plan 08: RepaymentPhase CR-01 Re-Read Fix Summary

**Re-Read nach audited_create! / audited_update! in allen 4 RepaymentPhase-Lifecycle-Methoden (create / update / open / close) — Clients erhalten jetzt die frische DAO-generierte version-UUID statt der stale pre-update Version, sodass realistische Edit- und Lifecycle-Flows keine 409-Endlosschleife mehr produzieren. Phase-7-erbte Bug-Klasse damit beseitigt; selbe Pattern wie 08-07 für RepaymentEntry.**

## Performance

- **Duration:** ~7 min
- **Tasks:** 1 (TDD: RED + GREEN + 6 Test-Anpassungen)
- **Files modified:** 1 (`genossi_service_impl/src/repayment_phase.rs`)
- **Commits:** 2 (RED + GREEN)
- **Tests:** 25/25 RepaymentPhase grün (23 pre-existing adaptiert + 2 neue CR-01 Regression-Tests); 267/267 service_impl lib grün (keine Regression)

## Accomplishments

### TDD RED — 2 Failing Tests (Commit `aba0c1e`)

Beide Tests asserten dass das Service-Result die DAO-generierte version-UUID enthält, nicht die pre-update Version:

1. **`test_update_repayment_phase_rereads_after_audited_update_returns_new_version`**
   - Setup: Mock liefert beim 1. + 2. find_by_id pre-update Entity mit `version_a` (Pre-Update-Load + audit_macros-internal Load); beim 3. find_by_id post-update Entity mit `version_b` + `share_value=13000` (Re-Read).
   - Assert: `result.version == version_b` (nicht `version_a`), `result.share_value == 13000`.
   - RED-Failure (vor Fix): `left: version_a` / `right: version_b` — bestätigt die Bug-Hypothese.

2. **`test_open_repayment_phase_rereads_phase_entity_returns_new_version`**
   - Setup: Mock liefert beim 1. + 2. find_by_id Preparation-Entity mit `version_a`; beim 3. find_by_id (nach Auto-Fill-Loop mit 0 Members) Open-Entity mit `version_b` + `opened_at=Some(_)` (Re-Read).
   - Assert: `result.version == version_b`, `result.status == Open`.
   - RED-Failure (vor Fix): `left: version_a` / `right: version_b` — bestätigt die Bug-Hypothese.

### TDD GREEN — Re-Read in 4 Lifecycle-Methoden (Commit `9305eac`)

Pattern in jeder Methode (Vorlage: `member.rs:343-348`):

```rust
crate::audited_create!(self, self.repayment_phase_dao, &entity, REPAYMENT_PHASE_PROCESS_CREATE, &user_id, tx);

// CR-01 Fix (Phase-7-Erbe): Re-read to get the persisted version UUID.
let refreshed = self
    .repayment_phase_dao
    .find_by_id(entity.id, tx.clone())  // bzw. id für update/open/close
    .await?
    .ok_or(ServiceError::EntityNotFound(entity.id))?;

self.transaction_dao.commit(tx).await?;
Ok(RepaymentPhase::from(&refreshed))
```

- **`create_repayment_phase`** (Z. ~140-152): Re-Read via `find_by_id(entity.id, tx.clone())` — kein pre-existing find_by_id im create-Flow, also nur 1 zusätzlicher Read.
- **`update_repayment_phase`** (Z. ~232-245): Re-Read via `find_by_id(id, tx.clone())` — bestehender pre-update Load + audit_macros-internal Load + neuer Re-Read = 3 find_by_id total.
- **`open_repayment_phase`** (Z. ~365-378): Re-Read NACH der `// ----- /PHAS-02 -----`-Markierung, VOR commit — Auto-Fill-Loop bleibt funktional unverändert zwischen audited_update! und Re-Read.
- **`close_repayment_phase`** (Z. ~482-493): Re-Read via `find_by_id(id, tx.clone())` — gleiches Pattern wie update.

### Test-Adaptionen (6 bestehende Tests)

| Test | Anpassung |
|------|-----------|
| `test_create_repayment_phase_success` | `expect_find_by_id` für Re-Read hinzugefügt; persistente Entity mit gleichen Daten wie Submission. Keine Sequence nötig (kein pre-load in create). |
| `test_update_repayment_phase_share_value_change_in_open_succeeds` | Umgestellt auf `mockall::Sequence`: 1. + 2. `find_by_id` → pre-update entity (Open, share_value=12000); `update` → Ok; 3. `find_by_id` → post-update entity (Open, share_value=13000, new version). |
| `test_open_phase_auto_fill_zero_members` | Sequence: 1. + 2. → pre-open (Preparation); `update` → Ok; 3. → post-open (Open, opened_at=Some, new version). |
| `test_open_phase_auto_fill_creates_entries_for_matching_members` | Sequence wie oben; member_dao liefert 3 Members → 3 entry-creates erwartet; Re-Read liefert post-open Phase. |
| `test_close_phase_with_zero_entries_succeeds` | Sequence: 1. + 2. → pre-close (Open); `update` → Ok; 3. → post-close (Closed, closed_at=Some, new version). |
| `test_close_phase_with_only_paid_out_or_deleted_succeeds` | Sequence wie oben; entry_dao liefert nur PaidOut entries (keine pending) → close darf fortschreiten; Re-Read liefert post-close Phase. |

Die anderen 11 bestehenden Tests sind nicht betroffen, weil sie:
- Reject-Pfade testen (`expect_update().times(0)`) — Re-Read wird nie erreicht.
- `delete_repayment_phase`-Tests sind — keine Re-Read-Änderung dort.
- Validation-Tests sind — kein DAO-Call überhaupt.

## Acceptance Criteria Verification

| Criterion | Expected | Actual | Status |
|-----------|----------|--------|--------|
| `find_by_id(id, tx.clone())` (multi-line format) Aufrufe | >=4 | 8 (incl. update + open + close service-load + Re-Read + delete + get) | PASS |
| `find_by_id(entity.id, tx.clone())` (create Re-Read) | >=1 | 1 | PASS |
| `CR-01 Fix` Marker-Kommentare | >=4 | 4 (1 pro Lifecycle-Methode) | PASS |
| `Ok(RepaymentPhase::from(&entity))` | 0 in den 4 Lifecycle-Methoden | 1 (nur in `get_repayment_phase`, read-only, korrekt) | PASS (Plan-Acceptance bezog sich auf write-Pfade) |
| `Ok(RepaymentPhase::from(&refreshed))` | >=4 | 4 | PASS |
| `test_update_repayment_phase_rereads_after_audited_update` | 1 | 1 | PASS |
| `test_open_repayment_phase_rereads_phase_entity` | 1 | 1 | PASS |
| `REPAYMENT_PHASE_PROCESS_OPEN` Erwähnungen (Auto-Fill bleibt intakt) | >=2 | 5 (const + 2 audited Calls + 2 in comments) | PASS |
| `self.repayment_phase_dao.(create\|update)(` außerhalb Macros | 0 | 0 | PASS (Audit-Disziplin) |
| `cargo build -p genossi_service_impl` | exit 0 | exit 0 | PASS |
| `cargo test -p genossi_service_impl --lib repayment_phase` | 25 grün | 25 grün, 0 failed | PASS |
| Re-Read-Position-Check open_repayment_phase | Re-Read VOR commit, NACH /PHAS-02 | bestätigt via grep | PASS |

## Deviations from Plan

Keine. Plan exakt wie geschrieben ausgeführt.

Eine Plan-Konsistenz-Klarstellung dokumentiert: Plan-Acceptance-Criterion `grep -nE "Ok\(RepaymentPhase::from\(&entity\)\)" ... | wc -l == 0` ist nur für die 4 Lifecycle-Methoden korrekt. Der legitime Read-Only-Pfad `get_repayment_phase` muss weiterhin `&entity` returnen (kein write, kein Re-Read nötig). Diese Methode war im Plan nicht im Scope der CR-01-Korrektur — die Effective Compliance wurde durch die 4 neuen `Ok(RepaymentPhase::from(&refreshed))` Aufrufe in den 4 modifizierten Lifecycle-Methoden verifiziert.

## Threat Model Verification

| Threat | Severity | Mitigation Status |
|--------|----------|-------------------|
| T-08-08-01 (inkonsistenter Re-Read nach Auto-Fill-Failure) | low | MITIGATED — Re-Read läuft INNERHALB derselben Tx wie audited_update! + Auto-Fill-Loop. Bei Auto-Fill-Failure droppt die Tx via `?`-early-return; Re-Read wird nie erreicht. |
| T-08-08-02 (Re-Read commitiert vor Auto-Fill, bricht Auto-Fill-Atomarität) | medium | MITIGATED — Re-Read steht NACH dem Auto-Fill-Block (nach `/PHAS-02`-Markierung), VOR `transaction_dao.commit`. Grep verifiziert die Position. Auto-Fill bleibt innerhalb der Tx. |

## Test Results

```
running 25 tests
test repayment_phase::tests::test_create_repayment_phase_validation_rejects_fiscal_year_out_of_range ... ok
test repayment_phase::tests::test_create_repayment_phase_validation_rejects_share_value_negative ... ok
test repayment_phase::tests::test_close_repayment_phase_from_preparation_returns_conflict ... ok
test repayment_phase::tests::test_create_repayment_phase_validation_rejects_share_value_zero ... ok
test repayment_phase::tests::test_delete_repayment_phase_in_open_returns_conflict ... ok
test repayment_phase::tests::test_close_phase_with_pending_entries_returns_conflict ... ok
test repayment_phase::tests::test_close_phase_with_only_paid_out_or_deleted_succeeds ... ok
test repayment_phase::tests::test_close_phase_with_zero_entries_succeeds ... ok
test repayment_phase::tests::test_close_phase_with_25_pending_entries_truncates_at_20 ... ok
test repayment_phase::tests::test_create_repayment_phase_success ... ok
test repayment_phase::tests::test_delete_repayment_phase_in_preparation_succeeds ... ok
test repayment_phase::tests::test_open_phase_auto_fill_atomic_on_dao_failure ... ok
test repayment_phase::tests::test_open_phase_auto_fill_skips_members_outside_fiscal_year ... ok
test repayment_phase::tests::test_open_phase_auto_fill_skips_members_with_zero_shares ... ok
test repayment_phase::tests::test_update_repayment_phase_fiscal_year_change_in_open_returns_conflict ... ok
test repayment_phase::tests::test_open_phase_auto_fill_skips_members_without_exit_date ... ok
test repayment_phase::tests::test_open_repayment_phase_from_open_returns_conflict ... ok
test repayment_phase::tests::test_open_repayment_phase_from_closed_returns_conflict ... ok
test repayment_phase::tests::test_open_repayment_phase_rereads_phase_entity_returns_new_version ... ok
test repayment_phase::tests::test_open_phase_auto_fill_zero_members ... ok
test repayment_phase::tests::test_open_phase_auto_fill_creates_entries_for_matching_members ... ok
test repayment_phase::tests::test_update_repayment_phase_in_closed_returns_conflict ... ok
test repayment_phase::tests::test_update_repayment_phase_rereads_after_audited_update_returns_new_version ... ok
test repayment_phase::tests::test_update_repayment_phase_share_value_change_in_open_succeeds ... ok
test repayment_phase::tests::test_update_repayment_phase_version_mismatch_returns_conflict ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 244 filtered out
```

```
test result: ok. 267 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
```

## Commit Trail

| Commit | Type | Description |
|--------|------|-------------|
| `aba0c1e` | test | RED: 2 failing CR-01 regression tests in mod tests |
| `9305eac` | fix | GREEN: Re-Read in 4 lifecycle methods + adapted 6 existing tests |

## Lessons

- **TDD-Disziplin zahlte sich aus**: Die 2 RED-Tests bestätigten die Bug-Hypothese exakt — `left: version_a / right: version_b`. GREEN-Fix machte sie sofort grün, ohne Iteration.
- **Bestehende Tests mit `.returning(...)` ohne `.times(N)`**: matchen "beliebig oft". Das verhindert direkte Test-Failures bei zusätzlichen find_by_id-Calls, aber die Tests verlieren ihre Aussagekraft — der Re-Read liefert dann die OLD-entity, und Assertions auf post-update-state schlagen fehl. `mockall::Sequence` ist die saubere Lösung, weil sie explizit dokumentiert, welcher Call welche Daten liefert.
- **`audited_create!` macht KEINEN find_by_id** (nur `create` + audit_log) — anders als `audited_update!`. Daher braucht `create_repayment_phase` nur 1 zusätzlichen Re-Read-Call im Mock, kein Sequence-Setup mit Pre-Load.
- **Plan-Acceptance "alle Methoden returnen &refreshed"**: Bezieht sich nur auf write-Pfade (create/update/open/close). Read-Pfade (`get_repayment_phase`) returnen weiterhin `&entity` — kein Bug, sondern korrekt (kein write, keine version-Bump).
- **Phase-7-Bug-Klasse jetzt vollständig beseitigt**: Sowohl RepaymentEntry (08-07) als auch RepaymentPhase (08-08) returnen jetzt frische version-UUIDs. Phase-7-E2E-Test `test_repayment_phase_lifecycle_audit_chain_intact` (e2e_tests.rs:10592 mit Workaround-Kommentar) kann in einer Folge-Iteration aufgeräumt werden — nicht Scope von 08-08.

## Self-Check: PASSED

- `genossi_service_impl/src/repayment_phase.rs`: FOUND
- Commit `aba0c1e`: FOUND
- Commit `9305eac`: FOUND
- All 4 Re-Read blocks verifiziert via grep (4× `Ok(RepaymentPhase::from(&refreshed))`)
- Auto-Fill block in open_repayment_phase unverändert: REPAYMENT_PHASE_PROCESS_OPEN 5 mal referenziert
- Audit-Disziplin: 0 direkte DAO.create/update außerhalb der `audited_*!` Macros
