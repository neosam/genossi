---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 02
subsystem: domain-logic
tags: [phase-13, resolver, aggregation, domain-logic, tdd, dry-resolver]

requires:
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    provides: "Phase-Foundation (Plan 01) — keine harte Code-Dependency, separates Modul"
provides:
  - "Trait RepaymentContextResolver (genossi_service::repayment_context) mit zwei Methoden: async resolve + sync aggregate"
  - "Struct RepaymentContext { share_count: i32, payout_amount: String, fiscal_year: i32 } — frozen Felder, PartialEq + Eq + Clone + Debug"
  - "MockRepaymentContextResolver via automock — fuer Letter-Service-Unit-Tests in Plan 04"
  - "Pure-Function aggregate_for_member(phase, entries, member_id) -> Option<RepaymentContext> — mockless direkt testbar"
  - "RepaymentContextResolverImpl + RepaymentContextResolverDeps (genossi_service_impl::repayment_context) — DI-ready, instantiierbar mit RepaymentPhaseDao + RepaymentEntryDao"
affects: [13-03, 13-04, 13-05, 13-06, 13-07]

tech-stack:
  added: []
  patterns:
    - "Trait mit ZWEI Methoden (async resolve + sync aggregate): erlaubt Letter-Service-Loop ohne 1+N DB-Reads (Plan 04 laedt phase+entries einmal vor der Schleife)"
    - "Pure-Function-Extraction aus Worker-Inline-Aggregation: direkte Testbarkeit ohne Mocks (10 pure-fn-Tests covern Filter+SUM+Format-Edge-Cases)"
    - "Trait::resolve delegiert intern via self.aggregate(...) an die pure fn — Single-Source-of-Truth, kein Drift zwischen Worker- und Letter-Pfad moeglich"

key-files:
  created:
    - "genossi_service/src/repayment_context.rs"
    - "genossi_service_impl/src/repayment_context.rs"
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-02-SUMMARY.md"
  modified:
    - "genossi_service/src/lib.rs"
    - "genossi_service_impl/src/lib.rs"

key-decisions:
  - "D-13-04 angewendet: aggregate filtert nach (deleted IS NULL && member_id == X && status IN [Open, Contacted]) — exakter Spiegel der Phase-10-Worker-Logik aus worker.rs:332-360"
  - "D-13-10 angewendet: genossi_mail/src/worker.rs bleibt UNVERAENDERT — Worker-Refactor ist separates Folge-Quick nach Phase 13 (Todo phase-10-worker-refactor-resolver.md)"
  - "Single-Source-of-Truth via Trait::resolve -> self.aggregate -> aggregate_for_member: keine Drift-Moeglichkeit zwischen Worker- und Letter-Pfad"
  - "RepaymentContext-Felder sind frozen (share_count, payout_amount, fiscal_year) — Aenderungen brechen Plan-03-Template-Renderer (D-13-01)"
  - "payout_amount = format!(\"{},{:02}\", cents/100, cents%100) — Phase 10 D-04 deutsche Lokalisierung, KEIN Tausenderpunkt, KEIN Euro-Symbol"
  - "Pure-Function aggregate_for_member returnt Option<RepaymentContext>; Trait::aggregate mapped None auf ServiceError::EntityNotFound(member_id)"
  - "Test-Mocks via hand-rolled mockall::mock!-Bloecke fuer RepaymentPhaseDao + RepaymentEntryDao (5 + 6 Methoden) — Pattern aus repayment_export.rs uebernommen"

patterns-established:
  - "Resolver-mit-zwei-Eingaengen-Trait: async resolve (mit Tx, fuer Worker) + sync aggregate (pure-fn-Wrapper, fuer Multi-Member-Loops ohne 1+N) — Vorbild fuer kuenftige Aggregations-Services"
  - "TDD-RED-Then-GREEN pro Task: erst leerer Modul-Stub + Tests committen (compile-fail), dann Trait/Struct/Impl ergaenzen — analog Plan 13-01 Pattern"
  - "Defense-in-Depth Soft-Delete-Filter: DAO-Default-Impl filtert deleted IS NULL, aggregate filtert NOCHMAL — Resilienz gegen kuenftige DAO-Changes"

requirements-completed: [BRIEF-01]

duration: ~7min
completed: 2026-06-01
---

# Phase 13 Plan 02: RepaymentContextResolver-Service (Domain-Logik-Extraktion) Summary

**Neue `RepaymentContextResolver`-Trait in `genossi_service::repayment_context` mit zwei Methoden (`resolve` + `aggregate`), Impl in `genossi_service_impl::repayment_context` mit Pure-Function `aggregate_for_member` als 1:1-Mirror der Phase-10-Worker-Inline-Aggregation; 19 Unit-Tests gruen, Worker bleibt per D-13-10 unveraendert.**

## Performance

- **Duration:** ~7 min (zwischen `1938eec` Task-1-RED und `a8b41d1` Task-2-GREEN)
- **Tasks:** 2 TDD-Tasks (jeweils RED + GREEN)
- **Files created:** 3 (2 Rust-Module + SUMMARY)
- **Files modified:** 2 (lib.rs in genossi_service + genossi_service_impl)
- **Commits:** 4 Task-Commits (2 RED + 2 GREEN) + 1 SUMMARY-Commit

## Accomplishments

- **Task 1 (RED + GREEN):** Trait `RepaymentContextResolver` mit zwei Methoden (`async resolve(phase_id, member_id, tx)` + `fn aggregate(phase, entries, member_id)`) plus `RepaymentContext`-Struct mit drei frozen Feldern (`share_count: i32`, `payout_amount: String`, `fiscal_year: i32`) in `genossi_service/src/repayment_context.rs`. `automock` generiert `MockRepaymentContextResolver` mit `expect_resolve()` + `expect_aggregate()` (verifiziert via Compile-Test). 4 Tests gruen unter `--features utoipa`.

- **Task 2 (RED + GREEN):** `RepaymentContextResolverImpl<Deps>` + Pure-Function `aggregate_for_member(phase, entries, member_id) -> Option<RepaymentContext>` in `genossi_service_impl/src/repayment_context.rs`. Pure-fn ist 1:1 Spiegel von `genossi_mail/src/worker.rs:332-360`:
  - Filter `deleted IS NULL && member_id == X && status IN [Open, Contacted]` (D-13-10)
  - `share_count = SUM(share_count_to_pay_out)`
  - `cents = share_count * phase.share_value`; `payout_amount = format!("{},{:02}", cents/100, cents%100)` (Phase 10 D-04)
  - Returns `None` wenn keine relevanten Entries
- Trait-Methode `aggregate` ist duenner Wrapper: `aggregate_for_member(...).ok_or(EntityNotFound(member_id))`.
- Trait-Methode `resolve` laedt Phase via `RepaymentPhaseDao::find_by_id` (404 als `EntityNotFound(phase_id)`), Entries via `RepaymentEntryDao::find_by_phase_id`, und delegiert an `self.aggregate(...)` — **eine einzige Wahrheits-Quelle, kein Drift zwischen Worker- und Letter-Pfad moeglich**.
- 15 Unit-Tests gruen, abdeckend alle Edge-Cases aus dem Plan-`<behavior>`-Block:
  - 10 pure-fn-Tests (single open, multi-entry SUM D-13-04, filters PaidOut, filters soft-deleted, cross-member isolation, Contacted included, PaidOut excluded, empty -> None, payout zero-padded, no Euro-symbol/no thousand dot)
  - 2 trait-aggregate-wrapper-Tests (happy path + empty -> EntityNotFound)
  - 3 resolve-Tests via hand-rolled `mockall::mock!`-Bloecke fuer `RepaymentPhaseDao` + `RepaymentEntryDao` (happy path, phase-not-found, no-relevant-entries -> EntityNotFound(member_id))

- **D-13-10 Guard erfuellt:** `genossi_mail/src/worker.rs` ist im gesamten Plan unveraendert geblieben (`git diff f63b97d..HEAD -- genossi_mail/src/worker.rs | wc -l == 0`). Worker-Refactor bleibt als Folge-Quick offen (siehe Todo `phase-10-worker-refactor-resolver.md`).

## Task Commits

1. **Task 1 RED — failing tests for RepaymentContextResolver trait:** `1938eec` (test)
2. **Task 1 GREEN — RepaymentContextResolver trait + RepaymentContext struct:** `2e86478` (feat)
3. **Task 2 RED — failing tests for RepaymentContextResolverImpl:** `386e651` (test)
4. **Task 2 GREEN — RepaymentContextResolverImpl + pure aggregate_for_member:** `a8b41d1` (feat)

## Files Created/Modified

- **Created** `genossi_service/src/repayment_context.rs` — Trait + Struct + automock + 4 Tests (123 Zeilen)
- **Created** `genossi_service_impl/src/repayment_context.rs` — Impl + Pure-fn + Deps-Trait + 15 Tests (~390 Zeilen)
- **Modified** `genossi_service/src/lib.rs` — `pub mod repayment_context;` (1 Zeile)
- **Modified** `genossi_service_impl/src/lib.rs` — `pub mod repayment_context;` (1 Zeile)

## Decisions Made

- **Trait-Methode `aggregate` ist sync (nicht async):** Pure-fn-Wrapper braucht keine `.await`-Punkte; `automock` mit `async_trait` erzeugt korrekt eine sync-mock-Methode. Klares Signal an Caller: "kein DB-Round-Trip" (Plan 04 nutzt das, um 1+N DB-Reads im Letter-Service-Loop zu vermeiden).
- **`resolve` delegiert intern an `self.aggregate(...)`:** statt Filterlogik in `resolve` zu duplizieren, ruft `resolve` `self.aggregate(&phase, &entries, member_id)` auf. So existiert die Filterlogik genau EINMAL in `aggregate_for_member` — kein Drift zwischen Worker- und Letter-Pfad moeglich (mitigiert Threat-Model "Trait::aggregate-Drift vs. Trait::resolve").
- **Resolve-Tests minimal (3 statt mehr):** Pure-fn-Tests decken die gesamte Aggregations-Logik ab; resolve braucht nur den DAO-Glue zu verifizieren (happy-path + phase-not-found + no-relevant-entries). Plan-Discretion akzeptiert minimale resolve-Coverage, wenn pure-fn-Tests exhaustiv sind.
- **Hand-rolled `mockall::mock!`-Bloecke statt `automock` fuer DAO-Mocks:** `automock` an den DAO-Traits selbst wuerde funktionieren, aber das Pattern aus `repayment_export.rs:282-572` ist im Repo etabliert und macht die Test-Setup-Intentions explizit lokal sichtbar.

## Deviations from Plan

**None bei der Domain-Logik selbst** — Plan wurde exakt umgesetzt: Trait-Signaturen, Struct-Felder, Filter-Pattern, Format-Pattern, alle 13+ Test-Behaviors aus dem `<behavior>`-Block.

**Auto-fix Rules waren nicht relevant:**
- Rule 1 (Bug): keine Bugs — alle 19 Tests gruen.
- Rule 2 (Missing Critical Functionality): keine — Plan-2-Scope ist rein domain-logic; Security-relevante Funnel (Permission-Check) liegen in Plan 04 Letter-Service.
- Rule 3 (Blocking Issue): siehe naechster Abschnitt.
- Rule 4 (Architectural): keine — Plan folgt etablierten Phase-10/11-Patterns 1:1.

## Issues Encountered

**Parallel Plan-03-Executor Working-Copy-Kollision (env-spezifisch, nicht Plan-bezogen):**
Der Plan-03-Executor lief parallel im selben jj-Repository (Wave 2, ebenfalls `depends_on: [01]`) und produzierte einen Test-Commit `c6baa8d` zwischen meinem Task-2-RED (`386e651`) und meinem Task-2-GREEN (`a8b41d1`). Plan 03 RED enthielt unstaged Aenderungen in `genossi_service_impl/src/pdf_generation.rs` (Stubs `build_inputs_repayment_letter` + `_bundle`), die `genossi_service::repayment_context::RepaymentContext` importieren — d.h. Plan 03 RED haengt von meinem Trait-Export ab.

**Mitigation:** Nur meine eigene Datei `genossi_service_impl/src/repayment_context.rs` per `git add` selektiv gestaged; `pdf_generation.rs`-Aenderungen blieben unstaged und werden vom Plan-03-Executor in seinem eigenen Commit-Stream uebernommen. Mein Task-2-GREEN-Commit basiert HEAD auf `c6baa8d` (Plan-03-RED), was eine ungewoehnliche aber funktional korrekte Linearitaet ergibt — der Orchestrator kann beide Plans entkoppelt einsammeln.

**Pre-existing Build Issue (NICHT von dieser Aenderung verursacht):** `cargo build -p genossi_service` ohne Feature-Flag scheitert weiterhin an `utoipa::ToSchema`-Imports in `auth_types.rs` — identisch zur Beobachtung in Plan-01-SUMMARY. Workspace-default-Build verwendet implizit `--features utoipa`. Tests laufen mit `--features utoipa` sauber durch fuer `genossi_service`; `genossi_service_impl` braucht das Feature-Flag nicht.

## Self-Check

```
=== Files exist ===
FOUND: genossi_service/src/repayment_context.rs (123 Zeilen)
FOUND: genossi_service_impl/src/repayment_context.rs (~390 Zeilen)
FOUND: .planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-02-SUMMARY.md

=== Commits exist ===
FOUND: 1938eec (Task 1 RED)
FOUND: 2e86478 (Task 1 GREEN)
FOUND: 386e651 (Task 2 RED)
FOUND: a8b41d1 (Task 2 GREEN)

=== Acceptance-Greps gruen ===
Task 1:
- rg 'pub trait RepaymentContextResolver' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'pub struct RepaymentContext\s' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'share_count: i32' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'payout_amount: String' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'fiscal_year: i32' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'automock' genossi_service/src/repayment_context.rs: 4 ✓
- rg '#\[derive\(Clone, Debug, PartialEq, Eq\)\]' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'async fn resolve' genossi_service/src/repayment_context.rs: 1 ✓
- rg 'fn aggregate\(' genossi_service/src/repayment_context.rs: 1 ✓ (sync)
- rg 'pub mod repayment_context' genossi_service/src/lib.rs: 1 ✓

Task 2:
- rg 'pub fn aggregate_for_member' genossi_service_impl/src/repayment_context.rs: 1 ✓
- rg 'pub struct RepaymentContextResolverImpl' genossi_service_impl/src/repayment_context.rs: 1 ✓
- rg 'pub trait RepaymentContextResolverDeps' genossi_service_impl/src/repayment_context.rs: 1 ✓
- rg 'aggregate_for_member' genossi_service_impl/src/repayment_context.rs: 14 (>=2 ✓)
- rg 'pub mod repayment_context' genossi_service_impl/src/lib.rs: 1 ✓
- rg 'RepaymentEntryStatus::Open \| RepaymentEntryStatus::Contacted' genossi_service_impl/src/repayment_context.rs: 1 ✓ (D-13-10 Filter)
- rg 'format!\("\{\},\{:02\}"' genossi_service_impl/src/repayment_context.rs: 2 (>=1 ✓, Phase 10 D-04 Euro-Format)
- rg 'fn test_aggregate_' genossi_service_impl/src/repayment_context.rs: 8 (>=8 ✓)
- rg 'fn test_trait_aggregate_' genossi_service_impl/src/repayment_context.rs: 2 (==2 ✓)

D-13-10 Guard:
- git diff f63b97d..HEAD -- genossi_mail/src/worker.rs | wc -l: 0 ✓ (Worker UNVERAENDERT)

Test-Runs:
- cargo test -p genossi_service --features utoipa --lib repayment_context: 4 passed, 0 failed ✓
- cargo test -p genossi_service_impl --lib repayment_context: 15 passed, 0 failed ✓
- cargo build -p genossi_service_impl: success (2 unrelated warnings aus pdf_generation.rs stub, Plan 03 RED) ✓
```

**Self-Check: PASSED**

## Threat Flags

Keine neuen Threat-Flags ueber das Plan-`<threat_model>` hinaus. Mitigationen:
- **Cross-Member Data Leak**: Test `test_aggregate_cross_member_isolation` verifiziert harter Filter; pure-fn-Test ist direkt+exakt.
- **PaidOut-Doppelauszahlung**: Tests `test_aggregate_filters_paid_out` + `test_aggregate_paid_out_excluded` verifizieren.
- **Soft-Deleted-Leak**: Test `test_aggregate_filters_soft_deleted` + Defense-in-Depth (DAO-Default-Filter + Aggregate-Filter doppelt).
- **Euro-Format Drift Mail vs. Brief**: Pattern-1:1 aus `worker.rs:332-360` + Tests `test_payout_amount_*` verifizieren.
- **D-13-10 Verletzung**: `git diff f63b97d..HEAD -- genossi_mail/src/worker.rs == 0` ✓
- **Trait::aggregate-Drift vs. Trait::resolve**: resolve delegiert intern via `self.aggregate(...)` an die pure fn; eine einzige Wahrheits-Quelle. Test `test_resolve_happy_path` ruft den vollen DAO-Pfad auf und verifiziert dieselbe Aggregation wie die pure-fn-Tests.

## Next Plan Readiness

Plans 03-07 koennen jetzt:
- **Plan 03 (PdfGenerator):** `RepaymentContext` als Input-Typ fuer `build_inputs_repayment_letter(phase, member, ctx)` und `build_inputs_repayment_letters_bundle(phase, recipients: &[(MemberEntity, RepaymentContext)])` importieren (`use genossi_service::repayment_context::RepaymentContext;`). Plan 03 RED-Stubs in `pdf_generation.rs` referenzieren bereits exakt diesen Typ.
- **Plan 04 (Letter-Service):** `RepaymentContextResolverImpl` ueber `RepaymentContextResolverDeps`-Trait in `RepaymentLetterServiceDeps` einbinden. Letter-Service-Loop laedt `phase + entries` EINMAL und ruft `resolver.aggregate(&phase, &entries, member_id)` pro Member — KEIN 1+N DB-Read.
- **Plan 05 (REST):** indirekt ueber Letter-Service.
- **Plan 06 (Frontend) + Plan 07 (E2E):** keine direkte Dependency, aber Aggregation-Korrektheit wird in E2E-Tests verifiziert (Audit-Hashchain + Multi-Entry-Aggregation-Test).
- **Mock-Wiring:** `MockRepaymentContextResolver::new()` + `mock.expect_aggregate().returning(|_, _, _| Ok(RepaymentContext { ... }))` in Plan-04-Letter-Service-Tests verfuegbar.

**Pending Follow-up (NICHT in Phase 13 Scope):**
- `.planning/todos/pending/phase-10-worker-refactor-resolver.md` — Phase-10-Mail-Worker auf `RepaymentContextResolver::resolve` migrieren. Resolver-Trait-Signatur ist stabil und produktiv getestet; Worker-Refactor laeuft als `/gsd-quick` NACH Phase 13.

**Keine Blocker fuer Folge-Plans.**

---
*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-01*
