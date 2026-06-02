---
phase: 11-export-pdf-csv
plan: 05
subsystem: bin-wiring
tags: [bin, di-wiring, rest-state, repayment-export, pdf]

# Dependency graph
requires:
  - phase: 11-export-pdf-csv
    provides: "Plan 11.03 RepaymentExportServiceImpl + RepaymentExportServiceDeps-Trait"
  - phase: 11-export-pdf-csv
    provides: "Plan 11.04 RepaymentExportRestState-Trait in genossi_rest::repayment_export"
  - phase: 06-attendance-export
    provides: "AttendanceExport DI-Wiring-Pattern (5-Edit-Stellen-Vorlage in genossi_bin/src/lib.rs)"
provides:
  - "RepaymentExportServiceDependencies struct + Send/Sync-Marker + RepaymentExportServiceDeps-Trait-Impl in genossi_bin/src/lib.rs"
  - "type RepaymentExportService Alias auf RepaymentExportServiceImpl<RepaymentExportServiceDependencies>"
  - "RestStateImpl field `repayment_export_service: Arc<RepaymentExportService>`"
  - "Service-Konstruktion in RestStateImpl::new() — Arc::clone-Reuse von pdf_generator, template_storage, alle DAOs (Single-Arc-per-Process)"
  - "impl genossi_rest::repayment_export::RepaymentExportRestState for RestStateImpl"
  - "cargo build (full workspace) clean nach Plan 11.04 hatte den Build broken gelassen (`RepaymentExportRestState not implemented`)"
affects: [11-06-e2e]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "5-Edit-Stellen-Wiring-Pattern fuer neuen Service in RestStateImpl (Type-Aliases + Feld + Construction + Initializer + Trait-Impl) — 1:1 Mirror von AttendanceExport Plan 6"
    - "Single-Arc-per-Process: alle DAOs (repayment_phase_dao/repayment_entry_dao/member_dao/transaction_dao/permission_service) sowie pdf_generator + template_base werden via Arc::clone aus den bereits konstruierten Arcs in new() geteilt — KEIN zweiter DAO-Konstruktor, KEIN zweiter PdfGenerator::new()"
    - "Wave-4-Wiring-Plan separiert mechanisches DI-Wiring von Service-Logik (Plan 11.03) und REST-Layer (Plan 11.04) — laeuft erst, wenn beide Vorgaenger geliefert haben"

key-files:
  created: []
  modified:
    - "genossi_bin/src/lib.rs (+63 Zeilen, 5 additive Edit-Stellen: Z. 291-318 Type-Aliases, Z. 525-527 Feld, Z. 832-852 Construction, Z. 941-942 Initializer, Z. 1485-1493 RestState-Trait-Impl)"

key-decisions:
  - "Type-Aliases NACH `type AttendanceExportService = ...` platziert (Z. 287-289 → 291ff), nicht in den oberen Type-Block neben den anderen Phase-7/8 Type-Aliases. Begruendung: Phase-11-Pattern spiegelt Phase-6 (AttendanceExport) 1:1; Lokalitaet der zusammengehoerigen Items ueberwiegt strikte alphabetische Sortierung."
  - "Konstruktor-Block NACH AttendanceExport-Konstruktor (Z. 821-830) platziert. Wiederverwendet die SAME `pdf_generator` + `template_storage.base_path()` Arcs wie AttendanceExport — pro Plan-Threat-Model T-007 (mehrfache PdfGenerator-Allokationen) vermieden via Arc::clone."
  - "Struct-Initializer NACH `attendance_export_service,` (Z. 942) — gleiche Reihenfolge wie Feld-Deklaration und Konstruktor-Block, damit Code-Review-Diffs zusammenhängen."
  - "RestState-Trait-Impl NACH AttendanceExportRestState (Z. 1485ff), nicht zwischen RepaymentPhase/RepaymentEntry — wieder Lokalitaet zu AttendanceExport-Pattern."
  - "`template_base: Arc::new(template_storage.base_path().to_path_buf())` 1:1 wie AttendanceExport — eine zweite Arc-Allokation des PathBufs ist trivial (kein Performance-Concern); Refactoring zu einer geteilten `attendance_template_base` Arc waere Rule-4-Change (out-of-scope)."

patterns-established:
  - "Wave-4-Wiring-Plan-Vorlage: separater Plan, wenn ein Service-Impl + REST-Layer in unabhaengigen Waves entstehen — DI-Wiring ist mechanisch (5 Edit-Stellen), aber erfordert beide Vorgaenger compilet (Plan 11.04 hat den Build absichtlich broken gelassen, dieser Plan schliesst die Luecke). Pattern fuer kuenftige Phase-12+ Services."
  - "Acceptance-Criterion `grep -c 'type RepaymentExportService =' == 1` ist semantisch zu strikt: matched ZUSAETZLICH zum Type-Alias auch den Trait-Assoc-Type-Setter im RestState-Trait-Impl (`type RepaymentExportService = RepaymentExportService;`). 2 Treffer sind erwartet und gewuenscht. Zukuenftige Plan-Acceptance-Criteria sollten den Trait-Impl-Setter ausschliessen via `grep -E '^type RepaymentExportService =' file == 1` oder zwei separate Greps. Siehe Deviation 1."

requirements-completed: [EXPO-01, EXPO-05]

# Metrics
duration: 3min
completed: 2026-06-01
---

# Phase 11 Plan 05: RepaymentExport DI-Wiring Summary

**5 additive Edit-Stellen in `genossi_bin/src/lib.rs` verdrahten den `RepaymentExportServiceImpl` (Plan 11.03) ueber das `RepaymentExportRestState`-Trait (Plan 11.04) in die `RestStateImpl`. Alle DAOs + `pdf_generator` + `template_storage` werden via Arc::clone aus den bereits konstruierten Arcs geteilt (Single-Arc-per-Process); `cargo build` (full workspace) ist clean.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-06-01T05:49:29Z
- **Completed:** 2026-06-01T05:51:48Z
- **Tasks:** 1
- **Files modified:** 1
- **Commits:** 1 (`084e6c4`)

## Accomplishments

- **5 Edit-Stellen in `genossi_bin/src/lib.rs`** alle additiv (keine Loeschungen, keine Refactors):
  1. **Type-Aliases-Block** (Z. 291-318): `pub struct RepaymentExportServiceDependencies` + `unsafe impl Send/Sync` + `impl RepaymentExportServiceDeps` mit den 5 Sub-Trait-Deps (Context/Transaction/RepaymentPhaseDao/RepaymentEntryDao/MemberDao/PermissionService/TransactionDao) + `type RepaymentExportService = RepaymentExportServiceImpl<RepaymentExportServiceDependencies>`.
  2. **Feld in `RestStateImpl`-Struct** (Z. 525-527): `repayment_export_service: Arc<RepaymentExportService>` direkt nach `attendance_export_service`.
  3. **Service-Konstruktion in `new()`** (Z. 832-852): `Arc::new(RepaymentExportServiceImpl::<RepaymentExportServiceDependencies> { ... })` mit Arc::clone-Reuse von `transaction_dao`, `permission_service`, `repayment_phase_dao`, `repayment_entry_dao`, `member_dao`, `pdf_generator` und neu allokierter `Arc::new(template_storage.base_path().to_path_buf())` (1:1-Mirror von AttendanceExport).
  4. **Struct-Initializer** (Z. 941-942): `repayment_export_service,` direkt nach `attendance_export_service,`.
  5. **RestState-Trait-Impl** (Z. 1485-1493): `impl genossi_rest::repayment_export::RepaymentExportRestState for RestStateImpl { type RepaymentExportService = RepaymentExportService; fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService> { self.repayment_export_service.clone() } }`.
- **`cargo build` (full workspace) clean** — vor Plan 11.04 war der Build aktiv broken durch `RepaymentExportRestState not implemented for RestStateImpl` (erwartete Eingangssituation laut Plan-11.04-Summary §Next-Phase-Readiness); jetzt 0 errors, nur pre-existierende Warnings in fremden Files (unused imports in genossi_backup/worker.rs, genossi_rest/lib.rs, genossi_bin/src/lib.rs — kein unserer Aenderungen verursacht).
- **Single-Arc-per-Process-Pattern eingehalten:** Plan-Threat-Model-Eintraege T-001/T-006/T-007 alle mitigiert via `cargo build` Compile-Time-Gate und Arc::clone (keine neuen DAO-/PdfGenerator-Konstruktoren).
- **REST-Endpoint `GET /api/repayment-phase/{phase_id}/export/{format}` ist nun HTTP-erreichbar.** Plan 11.06 (E2E-Tests) kann die Route exercieren.

## Task Commits

1. **Task 1: 5 Edit-Stellen in `genossi_bin/src/lib.rs`** — `084e6c4` (feat)
   - Alle 5 Stellen in einem atomaren Commit, weil sie strukturell zusammengehoeren und das Tree zwischendurch NICHT compilen wuerde (Struct-Feld ohne Initializer-Eintrag ist ein E0063, Trait-Impl ohne Feld ist ein E0609 — Split-Commits wuerden den Tree rot lassen).
   - `cargo build` final clean (0 errors); 6 von 8 Acceptance-Criteria-Greps exakt erfuellt, 1 mit Deviation (siehe unten).

**Plan metadata commit:** folgt nach diesem SUMMARY (docs-Commit mit STATE.md + ROADMAP.md-Updates).

## Files Created/Modified

- `genossi_bin/src/lib.rs` (MODIFIED, +63 Zeilen) — 5 additive Edit-Stellen wie oben aufgelistet.

## Decisions Made

- **5-Edit-Stellen 1:1-Mirror von AttendanceExport-Wiring (Phase 6 Plan 03):** Struktur, Reihenfolge, Kommentar-Stil, Arc::clone-Patterns sind identisch zu Z. 266-289, 520-522, 815-830, 918-922, 1460-1468 des AttendanceExport-Wiring. Plan 11.06 (E2E) erbt damit die gleiche Architektur-Erwartung.
- **Atomares Commit fuer alle 5 Stellen:** Split-Commits zwischen Type-Alias und Trait-Impl haetten den Tree zwischen Commits rot gelassen (E0063 / E0609). Einzelne logische Aenderung (DI-Wiring eines Services) = 1 Commit. Konsistent mit Phase-7/8 DI-Wiring-Plans.
- **Acceptance-Criterion `grep -c 'type RepaymentExportService =' == 1` umgedeutet als `>= 1`:** der Match-Pattern triff ZWEI Stellen — den Type-Alias auf File-Level (Z. 314) UND den Trait-Assoc-Type-Setter im RestState-Trait-Impl (Z. 1526). Beide sind beabsichtigt; das Plan-Acceptance-Criterion war buchstaeblich zu strikt formuliert. Siehe Deviation 1.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Acceptance-Criterion-Lesart] `grep -c "type RepaymentExportService =" == 1` ergibt 2 Treffer (statt 1)**

- **Found during:** Acceptance-Criteria-Grep-Verifikation nach Commit `084e6c4`.
- **Issue:** Plan-Acceptance-Criterion sagt `grep -cE "type RepaymentExportService =" genossi_bin/src/lib.rs == 1`. Tatsaechlich ergibt der Grep `2` Treffer:
  1. Z. 314: `type RepaymentExportService = genossi_service_impl::repayment_export::RepaymentExportServiceImpl<...>` (File-Level-Type-Alias, vom Plan vorgegeben).
  2. Z. 1526: `type RepaymentExportService = RepaymentExportService;` (Trait-Assoc-Type-Setter im `impl RepaymentExportRestState for RestStateImpl`-Block, ebenfalls vom Plan vorgegeben unter "STELLE 5").
- **Semantik-Check:** Beide Treffer sind im Plan-Text explizit als zu schreibender Code gefordert (SCHRITT 1 fuer Treffer 1, SCHRITT 5 fuer Treffer 2). Der Match-Pattern matched buchstaeblich beide; Plan-Intention ist erfuellt.
- **Disposition:** Plan-Acceptance-Criterion-Lesart wird in zukuenftigen Plan-Erstellungen entweder mit anchoring (`grep -cE '^type RepaymentExportService =' == 1`) oder mit getrennten Counts fuer File-Level-Alias und Trait-Assoc-Type formuliert. Analog zur Phase-11-Plan-03 Acceptance-Lesart-Deviation.
- **Files modified:** None — keine Code-Aenderung noetig, Plan-Intention voll erfuellt.
- **Verification:** `grep -nE "type RepaymentExportService =" genossi_bin/src/lib.rs` zeigt 2 Treffer an Z. 314 + 1526, beide vom Plan vorgegeben.

---

**Total deviations:** 1 (Rule 2 — Acceptance-Criterion-Lesart, keine Code-Aenderung, Plan-Intention erfuellt)
**Impact on plan:** Keine Scope-Aenderung. Alle 2 Requirements (EXPO-01 + EXPO-05) erfuellt; cargo build clean; alle 5 Edit-Stellen vorhanden und semantisch korrekt; Single-Arc-per-Process-Pattern eingehalten.

## Issues Encountered

None — alle 5 Edits erfolgten ohne Konflikte, cargo build ist beim ersten Versuch clean durchgelaufen, keine downstream-Anpassungen in test_server.rs oder anderen Files noetig (test_server.rs wurde bereits in Plan 11.04 als Rule-3-Auto-Fix erweitert; dieser Plan benoetigt dort nichts zusaetzlich).

## User Setup Required

None — keine externen Services, keine ENV-Variablen, keine Dashboard-Konfiguration.

## Next Phase Readiness

**Bereit fuer Plan 11.06** (E2E-Tests):

- `cargo build` (full workspace) ist clean — REST-Endpoint `GET /api/repayment-phase/{phase_id}/export/{format}` ist HTTP-erreichbar.
- `start_test_server` ist bereits korrekt verdrahtet (Plan 11.04 hat den `RepaymentExportRestState`-Bound dort hinzugefuegt).
- `RestStateImpl::new()` haendigt den `RepaymentExportServiceImpl` mit allen Production-Deps an die REST-Handler aus.
- Plan-11.06 E2E-Tests koennen direkt gegen den Pfad asserten (gleiche Infrastruktur wie Phase-6-AttendanceExport-E2E-Tests in `genossi_bin/tests/e2e_tests.rs`).

Keine Blocker.

## Self-Check: PASSED

Verifications run after writing SUMMARY:

- [x] `genossi_bin/src/lib.rs` modified (+63 Zeilen)
- [x] Commit `084e6c4` exists in `git log --oneline -5`
- [x] `cargo build 2>&1 | grep -c "^error"` == 0 (full workspace)
- [x] `cargo build -p genossi_bin 2>&1 | grep -c "^error"` == 0
- [x] `grep -cE "type RepaymentExportService =" genossi_bin/src/lib.rs` == 2 (Plan-Lesart `== 1`, semantisch korrekt — siehe Deviation 1)
- [x] `grep -cE "repayment_export_service: Arc<RepaymentExportService>" genossi_bin/src/lib.rs` == 1 (Feld in Struct)
- [x] `grep -cE "RepaymentExportServiceImpl::<" genossi_bin/src/lib.rs` == 1 (Construction-Stelle)
- [x] `grep -cE "fn repayment_export_service\(&self\)" genossi_bin/src/lib.rs` == 1 (Trait-Impl)
- [x] `grep -cE "impl genossi_rest::repayment_export::RepaymentExportRestState" genossi_bin/src/lib.rs` == 1
- [x] `grep -cE "pdf_generator: pdf_generator.clone\(\)" genossi_bin/src/lib.rs` == 2 (AttendanceExport + RepaymentExport teilen den Arc)
- [x] Keine Loeschungen im Commit (`git diff --diff-filter=D --name-only HEAD~1 HEAD` ist leer)
- [x] Keine untracked Files (`git status --short | grep '^??'` ist leer)
- [x] Plan-Single-Arc-per-Process eingehalten: KEIN `Arc::new(MemberDao::new(pool` und KEIN `PdfGenerator::new()` an einer zweiten Stelle (Grep auf File: jeweils exakt 1 Konstruktor pro Typ in der Workspace)

## TDD Gate Compliance

Plan 11.05 ist NICHT als TDD-Plan deklariert (`type: execute` im Frontmatter, kein `tdd="true"` Attribut auf der Task). Es handelt sich um mechanisches DI-Wiring; die TDD-Cycles laufen in Plan 11.03 (Service-Tests) und Plan 11.04 (REST-Unit-Tests). End-to-End-Tests folgen in Plan 11.06.

Daher kein RED/GREEN/REFACTOR Gate noetig.

---
*Phase: 11-export-pdf-csv*
*Completed: 2026-06-01*
