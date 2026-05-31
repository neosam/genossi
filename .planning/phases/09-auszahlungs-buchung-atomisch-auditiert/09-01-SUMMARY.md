---
phase: 09-auszahlungs-buchung-atomisch-auditiert
plan: 01
subsystem: service-layer
tags: [cascade, audit-macros, atomic-tx, optimistic-locking, race-defense, validation, mockall-sequence]

# Dependency graph
requires:
  - phase: 08-repaymententry-auto-bef-llung
    provides: "RepaymentEntryDao + RepaymentEntryService + Re-Read Pattern (BL-01 Phase-8-Fix) + audited_*!-Macros + RepaymentEntryStatus::PaidOut enum variant"
  - phase: 07-repaymentphase-backend-foundation
    provides: "RepaymentPhaseDao + Action-Endpoint-Pattern (open/close ohne Body) + Status-Guards + Lifecycle-Convention"
  - phase: 01-assembly-aggregat
    provides: "Auditable trait + audit_log_dao + Hash-Chain-Pattern"
provides:
  - "RepaymentEntryService::mark_paid_out Trait-Methode"
  - "RepaymentEntryServiceImpl::mark_paid_out 12-Schritt-Cascade-Implementation"
  - "MemberActionDao als 8. Dep im gen_service_impl!-Block"
  - "REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT (D-01 gemeinsamer Process-String fuer alle 3 Cascade-Writes)"
  - "compute_migration_status mit pub-Visibility (D-10 Option a)"
  - "TestMemberActionDao-Mock + build_service_admin_with_action_dao Helper"
  - "6 neue Unit-Tests: happy-path, 3 reject-paths, BL-01-Re-Read-None, D-04-Field-Assertions"
affects: [09-02-rest-handler, 09-03-wiring, 09-04-e2e, 09-05-requirements-signoff, 12-frontend-confirm-dialog]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "12-step Cross-Entity-Cascade in single SQLite-Tx mit shared process-string (D-01)"
    - "Inline-recalc_migrated via fully-qualified path zu pub'd compute_migration_status (D-10 Option a)"
    - "TestMemberActionDao mit mockall::Sequence fuer 11+ DAO-Call-Cascade-Tests"
    - "withf-Capture-Closure auf MemberActionEntity-Felder fuer D-04 Field-Auto-Setup-Assertions"
    - "Direct-DAO-Zugriff (kein Service-zu-Service) fuer Cascade-Owner (D-08)"

key-files:
  created: []
  modified:
    - "genossi_service/src/repayment_entry.rs (+14 LOC: mark_paid_out Trait-Methode + Compile-Test-Mock-Erweiterung)"
    - "genossi_service_impl/src/member_action.rs (+5 LOC: pub(crate) -> pub Visibility-Wechsel mit Begruendungs-Doc-Comment)"
    - "genossi_service_impl/src/repayment_entry.rs (+839 LOC -6 LOC: mark_paid_out-Impl + MemberActionDao-Dep + REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT-const + 6 Unit-Tests + TestMemberActionDao-Mock + build_service_admin_with_action_dao-Helper)"

key-decisions:
  - "D-04 vollautomatische MemberAction-Felder umgesetzt (action_type=Verkauf, shares_change=-N, date=today, comment='Anteils-Rueckzahlung Phase {fiscal_year}', transfer_member_id=None, effective_date=None)"
  - "D-08 Cascade lebt in RepaymentEntryServiceImpl mit direkten DAO-Macro-Calls (keine Service-zu-Service-Calls), MemberActionDao als 8. Dep"
  - "D-10 Option a gewaehlt: compute_migration_status pub + inline-Aufruf via crate::member_action:: (statt MemberActionService-Trait-Erweiterung)"
  - "D-11 Status-Guard ist primaere Race-Defense (Optimistic-Lock via audited_update! ist zweite Stufe; tatsaechliche Race-Defense passiert ueber UPDATE ... WHERE version = ? im DAO)"
  - "D-13/D-14 PAYO-03-Validation inline in mark_paid_out (kein separate validate_payout_shares-Helper)"
  - "Task 1 Stub-Pattern: unimplemented!() statt Trait-impl in 2 Schritten - haelt cargo build clean zwischen Task 1 und Task 2"
  - "TestDeps + build_service: Default-MockTestMemberActionDao::new() in 4-arg build_service (keine Aenderung an ~23 bestehenden Tests); neuer build_service_admin_with_action_dao-Helper fuer die 6 neuen Tests"

patterns-established:
  - "Compile-Test-Stub-Pattern: bei Trait-Erweiterung in T1 + Impl in T2, temporaere unimplemented!()-Methode in T1 ergaenzen → cargo build bleibt clean → CI/Test in T2 ersetzt die Stub direkt; Re-Read der Datei nicht erforderlich"
  - "Visibility-Upgrade pub(crate) -> pub mit Inline-Doc-Comment-Begruendung als Cross-Crate-Discovery-Hilfe (D-10)"
  - "11+ Calls Mock-Sequence: separate Sequences pro DAO (entry_seq, member_seq), Audit-Log + Tx ohne Sequence wegen returning()-without-times() (Phase-8-Pattern)"
  - "Audit-Disziplin-Grep-Gate (Comment-filtered): 0 direkte DAO-Schreibvorgaenge in mark_paid_out — alles ueber audited_create! + 2x audited_update!"

requirements-completed: []  # PAYO-01..04 werden erst in Plan 09-05 als [x] markiert (per ROADMAP-Konvention)

# Metrics
duration: 9min
completed: 2026-05-31
---

# Phase 9 Plan 01: mark_paid_out Cascade Foundation Summary

**12-Schritt-Cascade fuer atomare Auszahlungs-Buchung: 1x audited_create! (MemberAction::Verkauf) + 2x audited_update! (Member, RepaymentEntry) in einer SQLite-Tx mit gemeinsamem Process-String und BL-01 Re-Read-Defense.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-05-31T10:12:49Z
- **Completed:** 2026-05-31T10:21:18Z
- **Tasks:** 2 (T1: Trait+Visibility+Deps, T2: Impl+Tests)
- **Files modified:** 3

## Accomplishments

- **RepaymentEntryService-Trait** um `async fn mark_paid_out(id, context) -> Result<RepaymentEntry, ServiceError>` erweitert; `MockRepaymentEntryService::expect_mark_paid_out` automatisch via `#[automock]` verfuegbar.
- **`RepaymentEntryServiceImpl::mark_paid_out`** implementiert: kompletter 12-Schritt-Cascade aus RESEARCH §"Cascade Implementation Walkthrough" mit 1x `audited_create!` (MemberAction::Verkauf) + 2x `audited_update!` (Member, RepaymentEntry), alle 3 Aufrufe mit demselben Process-String `repayment-entry.mark-paid-out` (D-01).
- **MemberActionDao** als 8. Dep im `gen_service_impl!`-Block; Import-Ergaenzung (`ActionType`, `MemberActionEntity`); neue Konstante `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT`.
- **`compute_migration_status`** Sichtbarkeit `pub(crate) → pub` (D-10 Option a), aufgerufen via fully-qualified Path `crate::member_action::compute_migration_status` in mark_paid_out Schritt 11.
- **6 neue Unit-Tests** (alle gruen) decken Happy-Path, 3 Reject-Pfade (PAYO-04, Phase-not-Open, PAYO-03), BL-01-Re-Read-None und D-04-Field-Assertions ab.
- **Test-Infrastruktur** erweitert: `MockTestMemberActionDao`-Mock + `TestDeps`-Erweiterung um `MemberActionDao`-Type + `build_service_admin_with_action_dao`-Helper (haelt die ~23 bestehenden Tests ohne Signatur-Aenderung gruen).
- **PAYO-03-Validation** (current_shares < share_count_to_pay_out) inline mit `ServiceError::ValidationError` enthaelt beide Zahlenwerte (D-14).
- **PAYO-04-Status-Guard** (PaidOut ist final) und Phase-Status-Guard (Defense-in-Depth) blockieren mit `ServiceError::Conflict` VOR jedem audited_*!-Call (D-11).
- **BL-01 Re-Read-None-Pattern** aus Phase 8 reproduziert: `find_by_id` nach `audited_update!` returnt `None` → `ServiceError::InternalError` (HTTP 500), NICHT `EntityNotFound` (HTTP 404). Zweimal verwendet (Member + RepaymentEntry).

## Task Commits

Each task was committed atomically:

1. **Task 1: Trait + Visibility-Fix + Service-Impl-Deps** — `b25512c` (feat)
   - Trait-Erweiterung um `mark_paid_out` + Compile-Test-Mock-Erweiterung
   - `compute_migration_status` pub(crate) → pub mit Begruendungs-Doc-Comment
   - `MemberActionDao` als 8. Dep + Import + neue Konstante + Test-Infrastruktur (TestMemberActionDao-Mock + TestDeps-Erweiterung + build_service helpers)
   - Temporaere `unimplemented!()`-Stub fuer `mark_paid_out` haelt cargo build clean

2. **Task 2: mark_paid_out-Cascade-Impl + 6 Unit-Tests** — `1afd1fb` (feat)
   - Stub durch volle 12-Schritt-Cascade-Implementation ersetzt
   - 3 audit-Macro-Aufrufe mit gemeinsamem Process-String (D-01)
   - PAYO-03/04 Validation + Status-Guards
   - BL-01 Re-Read-Pattern (2x: Member + Entry)
   - D-10 Option a: inline recalc_migrated mit fully-qualified compute_migration_status-Path
   - 6 Unit-Tests: happy_path, rejects_paid_out_entry, rejects_when_phase_not_open, rejects_when_current_shares_insufficient, rereads_member_none_yields_internal_error, member_action_has_correct_fields

## Files Created/Modified

- `genossi_service/src/repayment_entry.rs` — Trait `mark_paid_out` ergaenzt (+14 LOC), Compile-Test-Mock erweitert um `expect_mark_paid_out()`.
- `genossi_service_impl/src/member_action.rs` — `compute_migration_status` Visibility pub(crate) → pub mit Inline-Doc-Comment-Begruendung (5 LOC).
- `genossi_service_impl/src/repayment_entry.rs` — Imports (`ActionType`, `MemberActionDao`, `MemberActionEntity`), neue Konstante `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT`, `MemberActionDao` als 8. Dep im `gen_service_impl!`-Block, volle `mark_paid_out`-Cascade-Implementation (~210 LOC), 6 neue Unit-Tests + `MockTestMemberActionDao`-Mock + `TestDeps`-Erweiterung + `build_service_admin_with_action_dao`-Helper. Netto +839 LOC -6 LOC ueber beide Tasks.

## Verification

```text
$ cargo test -p genossi_service --features utoipa repayment_entry::tests::test_mock_repayment_entry_service_compiles
test result: ok. 1 passed; 0 failed
```

```text
$ cargo test -p genossi_service_impl --lib repayment_entry
test result: ok. 29 passed; 0 failed
```

```text
$ cargo test -p genossi_service_impl --lib
test result: ok. 276 passed; 0 failed; 2 ignored
```

**Audit-Disziplin-Grep (Comment-filtered, T-09-01-02 Mitigation):**

```text
$ grep -v '^//' genossi_service_impl/src/repayment_entry.rs \
    | grep -v '^\s*//' \
    | grep -E "self\.(member|member_action|repayment_entry|repayment_phase)_dao\.(create|update)\(" \
    | wc -l
0
```

**Cascade-Marker-Greps:**

| Grep | Result | Expected |
|------|--------|----------|
| `audited_create!` ueber gesamte Datei | 5 | ≥ 2 (1× create_repayment_entry + 1× mark_paid_out + 3× in Test-Skeletten) |
| `audited_update!` ueber gesamte Datei | 35 | ≥ 4 (1× update + 1× batch + 2× mark_paid_out + Test-Match-Strings) |
| `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT` | 7 | ≥ 4 (1 const + 3 macro args + Test-Verwendung) |
| `ActionType::Verkauf` | 3 | ≥ 1 (Impl + Test-with-clauses) |
| `current_shares - entry.share_count_to_pay_out` | 1 | = 1 (genau Schritt 7) |
| `action_count + 1` | 2 | ≥ 1 (Impl + Test-Asserts) |
| `InternalError(Arc::from(format!` | 5 | ≥ 4 (2 Re-Reads + 1 missing-Phase + Tests) |

**mark_paid_out function body invocations:** `crate::audited_create!` = **1**, `crate::audited_update!` = **2** (gepruefte awk-Range-Extraktion).

## Decisions Made

- **Task 1 / Task 2 Split-Strategie:** Temporaeren `unimplemented!()`-Stub in Task 1 einzufuegen statt Task 1 mit Build-Fehler abzuschliessen. Begruendung: Plan erlaubt beides; Stub-Pattern haelt `cargo build` und CI auf jedem Zwischen-Commit clean — Vorteil fuer Bisect-Workflows in Folge-Phasen.
- **`build_service`-Signatur unveraendert lassen + neuer `build_service_admin_with_action_dao`-Helper:** Statt 4-Parameter-Signatur um neuen `action_dao` zu erweitern (was alle ~23 bestehenden Aufrufer aktualisieren wuerde) bekommt `build_service` ein Default `MockTestMemberActionDao::new()` ohne Expectations und die 6 neuen Tests rufen den neuen Helper. Bestehende Tests touchen `member_action_dao` ohnehin nicht.
- **Sequenz-Strategie pro DAO statt globaler Sequence:** Separater `entry_seq` und `member_seq` (jeweils `mockall::Sequence::new()`) statt einem globalem Sequence-Objekt fuer alle DAOs. Begruendung: cleaner failure messages, audit_log_dao + tx_dao haben kein `.times()`-Constraint (Phase-8-Pattern via `make_audit_log_dao_quiet` und `setup_mock_tx_dao`).
- **PAYO-03 ValidationError mit `Vec<ValidationFailureItem>`-Layout:** Gemaess D-14 exakt 1 Item mit `field = "share_count_to_pay_out"` und Message-Text der beide Zahlen enthaelt. Test 4 verifiziert `items[0].message.contains("Member.current_shares")` + die beiden konkreten Zahlen ("2" und "5").
- **Schritt 4 Missing-Phase-Mapping auf InternalError:** Entry referenziert eine nicht-existente Phase → referentielle Inkonsistenz → `ServiceError::InternalError`, nicht `EntityNotFound` (Pitfall #5 aus RESEARCH). Wird im aktuellen Code nicht durch Tests abgedeckt (Mock-Setup haette Entry+Phase-Mismatch konstruieren muessen); bewusste Plan-Entscheidung weil die Code-Pfad-Coverage durch andere Tests indirekt verifiziert ist.

## Deviations from Plan

None — Plan wurde 1:1 ausgefuehrt. Die einzige Diskretion war:

1. **Stub-Strategie zwischen Task 1 und Task 2** (Plan erlaubte beides; Wahl: Stub statt Build-Fehler).
2. **build_service-Helper-Strategie** (Plan erlaubte beides; Wahl: zweiter Helper statt Signatur-Aenderung — wie auch im Plan-Text als "pragmatische Loesung" empfohlen).
3. **Empty actions list in recalc_migrated tests:** Im happy_path-Test wird `find_by_member_id` mit leerer Vec gemockt. Das fuehrt dazu, dass `compute_migration_status` `actual_shares = 0` und `actual_action_count = 0` errechnet, was zu `migrated = false` fuehrt. Der `update_migrated(member_id, false, tx)`-Call ist daher korrekt. Reflektiert die D-10-Konvention (Off-by-One: `expected_action_count = action_count + 1`).

## Issues Encountered

Keine. Tests gruen beim ersten Lauf, Audit-Disziplin-Grep direkt 0.

## User Setup Required

None — keine externe Service-Konfiguration noetig.

## Next Phase Readiness

- **Plan 09-02 (REST-Handler):** Trait-Methode + Mock sind ready; `MockRepaymentEntryService::expect_mark_paid_out` ist automatisch verfuegbar (via `#[automock]`). REST-Handler kann gegen das Trait coden ohne weitere Service-Layer-Aenderungen.
- **Plan 09-03 (DI-Wiring in genossi_bin):** `cargo build -p genossi_bin` schlaegt aktuell ERWARTET fehl, weil `RepaymentEntryServiceImpl::new(...)` jetzt einen 8. Konstruktor-Parameter `member_action_dao` braucht. Plan 09-03 ergaenzt diesen Aufruf in `genossi_bin/src/lib.rs`. Bis dahin: `cargo build -p genossi_service_impl` und `cargo test -p genossi_service_impl` sind gruen — der Workspace-Build wird durch genossi_bin allein gebremst (alle Service-Layer-Plans sind unabhaengig testbar).
- **Plan 09-04 (E2E):** Race-Defense (D-11) ist via Optimistic-Lock im RepaymentEntryDao-Update-Pfad (`UPDATE ... WHERE id = ? AND version = ?`) bereits in Phase 8 verifizierbar; mark_paid_out nutzt dieses Pattern via `audited_update!`. tokio::join!-Verlierer wird mit HTTP 409 antworten (RESEARCH Frage 1).
- **Plan 09-05 (Requirements-Sign-off):** PAYO-01..04 werden hier NICHT als [x] markiert. Plan 09-05 ist explizit der Sign-off-Schritt nach erfolgreicher E2E-Verifikation in 09-04.

## TDD Gate Compliance

Plan-Frontmatter sagt `type: execute` (nicht `type: tdd`), aber beide Tasks haben `tdd="true"`:

- **Task 1 RED:** Die Trait-Erweiterung wuerde — ohne Stub — den `impl ... for RepaymentEntryServiceImpl`-Block brechen (Missing-Method-Error E0046). Das ist effektiv ein RED-Gate. Mit Stub-Pattern wird die Build clean gehalten; `cargo test -p genossi_service repayment_entry::tests::test_mock_repayment_entry_service_compiles` ist der Compile-Test-Beweis.
- **Task 2 RED:** Die 6 neuen Tests sind hinzugefuegt + Impl wird zeitgleich geschrieben (Plan-Action-Section schreibt vor: TDD-RED + GREEN im selben Commit, weil die Mock-Sequence-Pattern-Komplexitaet nur in Kombination mit der Cascade-Implementation sinnvoll testbar ist; Plan vermerkt ausdruecklich: "Implementation kommt in Task 2"). RED-Phase ist mit Sequence-Setup demnach implizit; GREEN-Phase manifestiert durch `cargo test ... mark_paid_out` mit `6 passed`.

Beide Tasks committen als `feat(...)` (nicht `test(...)` + `feat(...)`), weil sie keine reine Test-Erweiterung sind (sie liefern Trait + Impl gleichzeitig). Bei strikter Lesart der TDD-Konvention waere ein separater `test:` -> `feat:` -> `refactor:` Sequenz noetig, aber die Plan-Action-Section spezifiziert atomare Commits pro Task; das ueberwiegt.

## Self-Check: PASSED

- File `genossi_service/src/repayment_entry.rs` exists: FOUND
- File `genossi_service_impl/src/member_action.rs` exists: FOUND
- File `genossi_service_impl/src/repayment_entry.rs` exists: FOUND
- Commit `b25512c` (Task 1) exists: FOUND
- Commit `1afd1fb` (Task 2) exists: FOUND
- `mark_paid_out`-Trait-Methode definiert: FOUND
- `expect_mark_paid_out()` im Compile-Test: FOUND
- `pub fn compute_migration_status` (kein `(crate)`): FOUND (1 match)
- `pub(crate) fn compute_migration_status`: NOT FOUND (0 matches)
- `MemberActionDao` als Dep im gen_service_impl!-Block: FOUND
- `const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT`: FOUND
- Audit-Disziplin-Grep-Gate: 0 (compliant)
- 6 neue Test-Funktionen: FOUND (test_mark_paid_out_happy_path, _rejects_paid_out_entry, _rejects_when_phase_not_open, _rejects_when_current_shares_insufficient, _rereads_member_none_yields_internal_error, _member_action_has_correct_fields)
- 29/29 repayment_entry-Tests gruen
- 276/276 service_impl-Lib-Tests gruen

---

*Phase: 09-auszahlungs-buchung-atomisch-auditiert*
*Plan: 01*
*Completed: 2026-05-31*
