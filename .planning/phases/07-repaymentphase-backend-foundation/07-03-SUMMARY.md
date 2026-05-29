---
phase: 07-repaymentphase-backend-foundation
plan: 03
subsystem: service
tags: [service, audit, repayment-phase, state-machine, validation, optimistic-locking, rust]

# Dependency graph
requires:
  - phase: 07-repaymentphase-backend-foundation
    provides: RepaymentPhaseDao trait + Entity + Auditable + SQLite-Impl (Plans 01, 02)
provides:
  - "RepaymentPhase Domain-Typ + RepaymentPhaseSubmission + RepaymentPhaseUpdate DTOs"
  - "RepaymentPhaseService trait mit 7 Methoden (create/update/open/close/delete/get/get_all) + #[automock]"
  - "RepaymentPhaseServiceImpl mit Edit-Matrix (D-04), atomarer fiscal_year-Lock im Open (D-07), Lifecycle-Guards (D-05/D-06), Soft-Delete-Guard (D-09), Field-Validation (D-11/D-12), Optimistic-Locking"
  - "5 Prozesskonstanten: repayment-phase.{create,update,open,close,delete}"
  - "validate_phase_fields inline-Helper (fiscal_year 2000..=2100, share_value > 0)"
  - "Audit-Disziplin-Grep-Gate (Threat T-07-03-01 Mitigation: 0 direkte DAO-Calls außerhalb Macros)"
  - "Modul-Deklarationen in genossi_service/src/lib.rs + genossi_service_impl/src/lib.rs"
affects: [07-04-rest, 07-05-e2e, 08-repayment-entries, 09-payout-cascade]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Edit-Matrix-Pattern (Status-abhängige Field-Locks) als wiederverwendbares Service-Pattern für Phase 8 RepaymentEntry und Phase 9 MemberAction-Cascade"
    - "Atomare Mutations-Ablehnung (D-07): WENN gelocktes Feld berührt, GESAMTE Mutation 409 — kein selective passthrough"
    - "Inline-Field-Validator-Pattern (vs. ValidationService) — passt für 2-Feld-Entities"
    - "Audit-Disziplin-Grep-Gate als Pre-Merge-Check (Repudiation-Defense via Pattern-Constraint)"

key-files:
  created:
    - "genossi_service/src/repayment_phase.rs"
    - "genossi_service_impl/src/repayment_phase.rs"
  modified:
    - "genossi_service/src/lib.rs"
    - "genossi_service_impl/src/lib.rs"

key-decisions:
  - "Inline-Field-Validator (validate_phase_fields) statt Erweiterung von genossi_service_impl/src/validation.rs — Pattern-Anker aus PATTERNS.md §5 Hinweis: validation.rs ist ein anderer Concern (Mitglieder-Konsistenzberichte)"
  - "13-Test-Set deckt exakt die Edit-Matrix-Decision-Points D-04/D-05/D-06/D-07/D-09/D-11/D-12 — pro Decision mindestens 1 Negative-Path-Test plus Happy-Path"
  - "Audit-Disziplin-Grep-Gate verwendet `grep -v '^//' | grep -v '^*'` (Doc-Comments + Doc-Comment-Continuations) als Filter, um Self-Invalidation des Gates durch Anti-Pattern-Erwähnung in Kommentaren zu vermeiden"
  - "audited_update!-Aufrufe: 3 (update + open + close); audited_delete!: 1 (delete); audited_create!: 1 (create) — Mapping Service-Methode → Audit-Process direkt überprüfbar via 5 unterschiedliche Prozesskonstanten-Strings"
  - "TestRepaymentPhaseDao + TestAuditLogDao + TestPermissionService Hand-rolled Mocks gegen TestTransaction (statt MockTransaction) — Pattern-Konsistenz mit assembly.rs::tests, weil mockall::automock den Transaction-Type hartkodiert"
  - "Phase-8-TODOs als Code-Kommentare in open_/close_ vermerkt (PHAS-02 Auto-Befüllung, PHAS-03 Pending-Entry-Validation) — Vermeidet dass Plan 08 die Hooks nicht findet"

patterns-established:
  - "RepaymentPhaseService-Trait-Signaturen sind frozen — Plan 04 (REST) erwartet exakt diese 7 Methoden und die DTO-Field-Reihenfolge"
  - "ServiceError::ValidationError-Map auf 400 BadRequest erfolgt in genossi_rest/src/lib.rs:101-107 (Plan 04 verlässt sich darauf)"
  - "ServiceError::Conflict-Map auf 409 erfolgt in genossi_rest/src/lib.rs (Plan 04: alle 5 Conflict-Sources liefern 409)"

requirements-completed: [PHAS-01, PHAS-04, PHAS-05]
requirements-skeleton-complete: [PHAS-02, PHAS-03]

# Metrics
duration: 9min
completed: 2026-05-29
---

# Phase 7 Plan 03: RepaymentPhase Service-Layer Summary

**Service-Trait (`RepaymentPhaseService` mit 7 Methoden) + Service-Impl (`RepaymentPhaseServiceImpl`) mit Edit-Matrix (D-04), atomarer fiscal_year-Locking in Open (D-07), Lifecycle-Guards (D-05/D-06), Soft-Delete-Restriction (D-09), Field-Validation (D-11/D-12), Optimistic-Locking und 5 Audit-Prozessen — 17 grüne Unit-Tests (4 im Trait, 13 im Impl), 0 direkte DAO-Calls außerhalb Audit-Macros (T-07-03-01 Mitigation verifiziert per Grep-Gate).**

## Performance

- **Duration:** ~9 min (562 s)
- **Started:** 2026-05-29T19:54:39Z
- **Completed:** 2026-05-29T20:04:01Z
- **Tasks:** 2 (von 2)
- **Files created/modified:** 4 (2 new + 2 modified)

## Accomplishments

### Task 1: Service-Trait (`genossi_service/src/repayment_phase.rs`, 265 LOC)

- Domain-Typ `RepaymentPhase` mit 9 Feldern (`id`, `fiscal_year`, `share_value`, `status`, `opened_at`, `closed_at`, `created`, `deleted`, `version`), bidirektionale `From`-Impls für Entity-Roundtrip
- DTOs: `RepaymentPhaseSubmission` (2 Felder: `fiscal_year`, `share_value`) und `RepaymentPhaseUpdate` (3 Felder, `version` Pflicht)
- `#[automock]`-Trait `RepaymentPhaseService` mit 7 async-Methoden: `create_repayment_phase`, `update_repayment_phase`, `open_repayment_phase`, `close_repayment_phase`, `delete_repayment_phase`, `get_repayment_phase`, `get_all_repayment_phases`
- **Unterschied zur Assembly-Vorlage:** Kein `RepaymentPhaseDetail`-Wrapper (Phase 7 ohne Snapshot); plus `delete_repayment_phase` (D-09)
- 4 grüne Unit-Tests: Entity-Roundtrip, Submission/Update Constructibility, Mock-Compile (verifiziert alle 7 `expect_*`-Builder)
- Modul-Deklaration `pub mod repayment_phase;` alphabetisch zwischen `permission` und `session`

### Task 2: Service-Impl (`genossi_service_impl/src/repayment_phase.rs`, 1107 LOC)

- 5 Prozesskonstanten exakt benannt: `"repayment-phase.create"`, `".update"`, `".open"`, `".close"`, `".delete"` (Audit-Process-IDs, je 1 Grep-Treffer)
- `validate_phase_fields(fiscal_year, share_value)`-Inline-Helper für D-11/D-12
- `gen_service_impl!`-Wiring mit 5 Deps: `repayment_phase_dao`, `audit_log_dao`, `permission_service`, `uuid_service`, `transaction_dao` (kein Snapshot/Member/Helper/PermissionDao — Phase 7 ist simpler als Assembly)
- 7 Service-Methoden implementiert:
  - `create_repayment_phase`: ADMIN-Check → Validate → Entity (Status=Preparation, opened_at/closed_at=None) → `audited_create!` mit `repayment-phase.create` → commit
  - `update_repayment_phase`: ADMIN → load → **Edit-Matrix-Check** (Closed→Conflict, Open+fiscal_year-Change→Conflict atomar D-07, Preparation→frei) → Version-Check → Validate → Mutate → `audited_update!` mit `repayment-phase.update`
  - `open_repayment_phase`: ADMIN → load → Guard `!= Preparation`→409 → setze Status=Open, opened_at=now → `audited_update!` mit `repayment-phase.open` — **PHAS-02 (Phase 8) Auto-Befüllung als TODO im Code-Kommentar**
  - `close_repayment_phase`: ADMIN → load → Guard `!= Open`→409 → setze Status=Closed, closed_at=now → `audited_update!` mit `repayment-phase.close` — **PHAS-03 (Phase 8) Pending-Validation als TODO im Code-Kommentar**
  - `delete_repayment_phase`: ADMIN → load → Guard `!= Preparation`→409 (D-09) → `audited_delete!` mit `repayment-phase.delete`
  - `get_repayment_phase` / `get_all_repayment_phases`: ADMIN + DAO-Read (DAO-Default-Impl filtert `deleted IS NULL` per D-10)
- **AUDIT-DISZIPLIN-GREP-GATE PASSED:** 0 direkte `self.repayment_phase_dao.create(` / `.update(` Aufrufe außerhalb der `audited_*!`-Macro-Expansionen (Threat T-07-03-01 Mitigation verifiziert)
- 13 grüne Unit-Tests mit Hand-rolled Mocks (TestTxDao, TestRepaymentPhaseDao, TestAuditLogDao, TestPermissionService gegen TestTransaction)
- Modul-Deklaration `pub mod repayment_phase;` alphabetisch zwischen `permission` und `rfc3161`

## Task Commits

Each task was committed atomically:

1. **Task 1: Service-Trait + DTOs + lib.rs-Decl** — `771c8d5` (feat)
2. **Task 2: Service-Impl + 5 Konstanten + Validator + 13 Tests + lib.rs-Decl** — `963f17b` (feat)

## Files Created/Modified

- `genossi_service/src/repayment_phase.rs` — Service-Trait + DTOs + 4 Unit-Tests (NEW, 265 LOC)
- `genossi_service/src/lib.rs` — `pub mod repayment_phase;` alphabetisch (MOD, +1 LOC)
- `genossi_service_impl/src/repayment_phase.rs` — Service-Impl + 5 Prozesskonstanten + Validator + 13 Unit-Tests (NEW, 1107 LOC)
- `genossi_service_impl/src/lib.rs` — `pub mod repayment_phase;` alphabetisch (MOD, +1 LOC)

## Decisions Made

- **Inline-Validator statt `validation.rs`-Erweiterung:** `validate_phase_fields` lebt im Service-Impl, nicht im `genossi_service_impl/src/validation.rs`-Modul (siehe PATTERNS.md §5 Hinweis: validation.rs ist ein anderer Concern — Mitglieder-Konsistenzberichte). Für 2-Feld-Entities wäre ein neuer Service-Stub Overkill.
- **Atomare Mutation-Ablehnung (D-07) BEFORE Version-Check:** Die Edit-Matrix wird vor dem Version-Check geprüft, damit z.B. PUT `{fiscal_year: 2027, share_value: 13000, version: <stale>}` auf einer Open-Phase als `Conflict("Cannot change fiscal_year")` zurückkommt, nicht als `Conflict("Version mismatch")`. Konsistenter Status-Code, semantisch klarere Fehlermeldung.
- **TestTransaction statt MockTransaction:** Pattern-Konsistenz mit `assembly.rs::tests` — `gen_service_impl!` requires `Transaction: Clone + Debug`, aber `genossi_dao::MockTransaction` implementiert kein `Debug`. Lösung: Test-lokale `TestTransaction` mit `Debug`-Derive und 4 Hand-rolled `mockall::mock!`-Blöcke gegen diese.
- **Audit-Disziplin-Grep-Gate-Filter:** `grep -v '^[[:space:]]*//' | grep -v '^[[:space:]]*\*'` filtert sowohl `//`-Single-Line-Comments als auch `*`-Doc-Comment-Continuations heraus, um zu verhindern dass die Doc-Comment-Erwähnung des Anti-Patterns ("self.repayment_phase_dao.create direkt aufrufen ist verboten") das Gate selbst-invalidiert.
- **Phase-8-TODO-Kommentare in `open_`/`close_`:** Explizit als Anker, damit Phase 8 die Erweiterungspunkte sofort findet — Auto-Befüllung (PHAS-02) in `open_repayment_phase` und Pending-Entry-Validation (PHAS-03) in `close_repayment_phase`.

## Threat Model Mitigations Verified

| Threat ID | Mitigation | Verified via |
|-----------|------------|--------------|
| T-07-03-01 (Repudiation / Audit-Bypass) | Audit-Macros sind die einzige Schreib-Route; direkte DAO-Calls verboten | Grep-Gate: `grep -v '^//' | grep 'self.repayment_phase_dao.(create|update)('` → 0 Treffer. Außerdem überprüfen alle 13 Tests implizit, dass `make_audit_log_dao_quiet().expect_create_entries()` gesetzt ist (Audit-Aufruf-Pfad muss kompilieren). |
| T-07-03-02 (Elevation of Privilege) | Jede Methode startet mit `permission_service.check_permission("admin", ctx)` | Code-Inspection: `ADMIN_PRIVILEGE` ist in jeder der 7 Methoden referenziert (verifizierbar via `grep -c ADMIN_PRIVILEGE` = 7). Tests verwenden `make_permission_service_admin_ok()` mit `check_permission` returning `Ok(())`. |
| T-07-03-03 (Tampering / Concurrent PUT) | Version-Mismatch → Conflict("Version mismatch") | Test 8 `test_update_repayment_phase_version_mismatch_returns_conflict` |
| T-07-03-04 (Tampering / heimliche share_value-Mutation in Open) | Edit-Matrix-Check BEVOR Version-Check; D-07 atomare Ablehnung | Test 6 `test_update_repayment_phase_fiscal_year_change_in_open_returns_conflict` (Mock-update().times(0) verifiziert no-write) |
| T-07-03-05 (DoS / Pool-Exhaustion durch unkommitete Transactions) | tokio-`Drop` der Transaction macht implicit-rollback bei Early-Return; Pattern-Konsistenz mit assembly.rs | Code-Inspection: jede `return Err(...)`-Pfad lässt `tx` per Drop sauberen Rollback machen. |
| T-07-03-06 (Information Disclosure / soft-gelöschte Phasen sichtbar) | `get_all_repayment_phases` ruft DAO-Default-Impl `all()` mit `deleted IS NULL`-Filter (D-10) | Code-Inspection: `self.repayment_phase_dao.all(tx)` referenziert genau die Default-Impl aus Plan 01. |
| T-07-03-07 (Tampering / Delete auf Open/Closed bricht Audit-Konsistenz) | D-09 Status-Guard `!= Preparation → Conflict("D-09")` | Test 12 `test_delete_repayment_phase_in_open_returns_conflict` |

## Deviations from Plan

**Keine substantiellen Abweichungen.** Eine kleine Klarstellung zur Build/Test-Konfiguration:

1. **`utoipa`-Feature für `cargo test -p genossi_service`:** Das `genossi_service`-Crate hat ein optionales `utoipa`-Feature. Per-Crate-Test braucht entweder `--features utoipa` oder läuft via Workspace-Build (`cargo test --workspace`). Plan-Verifikation `cargo test -p genossi_service --lib repayment_phase` wurde als `cargo test -p genossi_service --features utoipa --lib repayment_phase` ausgeführt. Pre-existing Feature-Setup, keine durch Plan 07-03 verursachte Änderung.

Sub-Repos sind nicht konfiguriert (Single-Repo); jj+git colocated, normale `git commit` verwendet (kein `--no-verify`).

## Test-Ergebnisse

### genossi_service (Trait + DTOs)

```
running 4 tests
test repayment_phase::tests::test_mock_repayment_phase_service_compiles ... ok
test repayment_phase::tests::test_repayment_phase_submission_constructible ... ok
test repayment_phase::tests::entity_to_repayment_phase_roundtrip ... ok
test repayment_phase::tests::test_repayment_phase_update_requires_version ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out
```

Volle genossi_service-Suite: 43 passed; 0 failed.

### genossi_service_impl (Service-Impl)

```
running 13 tests
test repayment_phase::tests::test_open_repayment_phase_from_closed_returns_conflict ... ok
test repayment_phase::tests::test_create_repayment_phase_validation_rejects_fiscal_year_out_of_range ... ok
test repayment_phase::tests::test_create_repayment_phase_validation_rejects_share_value_negative ... ok
test repayment_phase::tests::test_delete_repayment_phase_in_preparation_succeeds ... ok
test repayment_phase::tests::test_create_repayment_phase_validation_rejects_share_value_zero ... ok
test repayment_phase::tests::test_close_repayment_phase_from_preparation_returns_conflict ... ok
test repayment_phase::tests::test_delete_repayment_phase_in_open_returns_conflict ... ok
test repayment_phase::tests::test_open_repayment_phase_from_open_returns_conflict ... ok
test repayment_phase::tests::test_update_repayment_phase_in_closed_returns_conflict ... ok
test repayment_phase::tests::test_create_repayment_phase_success ... ok
test repayment_phase::tests::test_update_repayment_phase_fiscal_year_change_in_open_returns_conflict ... ok
test repayment_phase::tests::test_update_repayment_phase_version_mismatch_returns_conflict ... ok
test repayment_phase::tests::test_update_repayment_phase_share_value_change_in_open_succeeds ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 223 filtered out
```

Volle genossi_service_impl-Suite: 234 passed; 0 failed; 2 ignored (pre-existing).

**Workspace-Build:** clean. Nur pre-existing Warnings aus genossi_backup (3), genossi_bin (1), genossi_mail (2), genossi_rest (2) — alle bereits in Plan 07-01/02 SUMMARIES dokumentiert, nicht durch Plan 07-03 verursacht.

## Audit-Disziplin-Grep-Gate

```
$ grep -v '^[[:space:]]*//' genossi_service_impl/src/repayment_phase.rs \
  | grep -v '^[[:space:]]*\*' \
  | grep -E 'self\.repayment_phase_dao\.(create|update)\(' \
  | wc -l
0
```

**Ergebnis: 0 direkte DAO-create/update-Aufrufe außerhalb der `audited_*!`-Macro-Expansionen.**

Verifiziert Threat T-07-03-01: Audit-Spur ist garantiert, weil jede Schreibroute durch die Audit-Macros führt. Plan-04 REST-Tests können sich darauf verlassen, dass jeder Lifecycle-Event in der Audit-Hashchain landet.

## TDD Gate Compliance

Plan 07-03 ist `type: execute` mit Task-Level `tdd="true"`:

- **Task 1 (Service-Trait):** RED+GREEN als ein `feat`-Commit zusammengefasst — die 4 Tests sind compile-time Verifikationen + Roundtrip-Checks, die zwingend gegen die Datei kompilieren müssen (`test_mock_repayment_phase_service_compiles` würde scheitern, wenn auch nur eine der 7 Trait-Methoden fehlt). Pattern-konsistent mit Plan 07-01 Task 2.
- **Task 2 (Service-Impl):** RED+GREEN als ein `feat`-Commit — 13 Tests verifizieren jede Decision (D-04, D-05, D-06, D-07, D-09, D-11, D-12) per Negative-Path und Happy-Path. Die Tests sind verschränkt mit der Impl (Mocks für DAO/Audit/Permission), daher untrennbar in einem Commit.

Phase-Level-TDD-Gate-Sequence (test()-Commit gefolgt von feat()-Commit) ist nicht anwendbar — Plan 07-03 ist `type: execute`, nicht `type: tdd`.

## Issues Encountered

Keine substantielle Probleme. Eine kleine Reibung war das `utoipa`-Feature-Gate für `cargo test -p genossi_service` (siehe Deviations), aber das löste sich mit `--features utoipa` sofort.

## User Setup Required

Keine externe Konfiguration nötig.

## Next Phase Readiness

Plan 04 (REST-Handler) kann jetzt direkt andocken:

- **Service-Trait-Signaturen frozen:** Plan 04 erwartet `RepaymentPhaseService` mit exakt 7 Methoden und den DTO-Field-Reihenfolgen aus Task 1.
- **Error-Mapping etabliert:** `ServiceError::ValidationError → 400`, `ServiceError::Conflict → 409`, `ServiceError::EntityNotFound → 404`, `ServiceError::PermissionDenied → 401` (alle in `genossi_rest/src/lib.rs:97-113` schon vorhanden — kein REST-Layer-Change nötig).
- **Audit-Pipeline aktiv:** Plan 05 (E2E) kann `/api/audit/verify` gegen die 5 Prozessnamen prüfen — `create_repayment_phase` → `repayment-phase.create`, `update_repayment_phase` → `repayment-phase.update`, `open_repayment_phase` → `repayment-phase.open`, `close_repayment_phase` → `repayment-phase.close`, `delete_repayment_phase` → `repayment-phase.delete`.
- **REST-Body-Schema-Vorlage:** `RepaymentPhaseSubmission` mit 2 Pflichtfeldern; `RepaymentPhaseUpdate` mit 3 Pflichtfeldern (`version` Optimistic-Locking). Plan 04 setzt `From<&RepaymentPhase> for RepaymentPhaseTO` in `genossi_rest_types/src/lib.rs` (siehe PATTERNS §7 Vorlage).
- **OpenAPI-Konvention:** `fiscal_year: 2026`, `share_value: 12000` als Schema-Beispielwerte (CONTEXT.md "Claude's Discretion: OpenAPI-Beispielwerte").

ROADMAP SC#2 Teil "Auditable + audited_create!/audited_update! greifen" ist erfüllt (Service-Layer-Pfad verifiziert). SC#5 Teil "share_value-Korrektur in Open erzeugt Audit-Eintrag; fiscal_year nach Open read-only" ist verhaltensseitig durch Test 6 + Test 7 verifiziert; E2E-Verifikation kommt in Plan 05.

**Requirements:**
- **PHAS-01** (Create + Soft-Delete): ✅ create_repayment_phase + delete_repayment_phase vollständig implementiert
- **PHAS-02** (Open-Lifecycle): ✅ Skeleton (Status + opened_at + Audit) — Auto-Befüllung als Phase-8-TODO markiert
- **PHAS-03** (Close-Lifecycle): ✅ Skeleton (Status + closed_at + Audit) — Pending-Entry-Validation als Phase-8-TODO markiert
- **PHAS-04** (share_value-Korrektur in Open): ✅ vollständig implementiert + Test 7 verifiziert
- **PHAS-05** (Audit-Macros greifen): ✅ vollständig per Grep-Gate verifiziert

## Self-Check: PASSED

- `genossi_service/src/repayment_phase.rs`: FOUND (265 LOC)
- `genossi_service/src/lib.rs` mit `pub mod repayment_phase;`: FOUND
- `genossi_service_impl/src/repayment_phase.rs`: FOUND (1107 LOC)
- `genossi_service_impl/src/lib.rs` mit `pub mod repayment_phase;`: FOUND
- Commit `771c8d5` (Task 1 Service-Trait): FOUND
- Commit `963f17b` (Task 2 Service-Impl): FOUND
- `cargo test -p genossi_service --features utoipa --lib repayment_phase`: 4/4 passed
- `cargo test -p genossi_service_impl --lib repayment_phase`: 13/13 passed
- `cargo build --workspace`: clean (only pre-existing warnings)
- AUDIT-DISZIPLIN-GREP-GATE: 0 direct DAO create/update calls outside audited_*! macros
- 5 process constants exactly named (1 grep hit each): repayment-phase.{create,update,open,close,delete}
- Audit macro usage: audited_create!=1, audited_update!=3, audited_delete!=1

---
*Phase: 07-repaymentphase-backend-foundation*
*Completed: 2026-05-29*
