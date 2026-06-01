---
phase: 11-export-pdf-csv
plan: 04
subsystem: api
tags: [rest-handler, openapi, format-whitelist, axum, utoipa, repayment-export, pdf]

# Dependency graph
requires:
  - phase: 11-export-pdf-csv
    provides: "Plan 11.02 RepaymentExportService trait + ExportFormat + ExportInclude + RepaymentExport bundle"
  - phase: 11-export-pdf-csv
    provides: "Plan 11.03 RepaymentExportServiceImpl mit Permission-Funnel"
  - phase: 06-attendance-export
    provides: "AttendanceExport REST-Pattern (attendance_export.rs, 1:1-Vorlage)"
provides:
  - "REST-Handler `export_repayment` unter GET /api/repayment-phase/{phase_id}/export/{format}"
  - "ExportQuery + ExportIncludeQuery (Default = Open per D-03) als REST-Layer-Mirror"
  - "RepaymentExportRestState-Trait fuer DI in Plan 11.05"
  - "Lokales map_export_error (PermissionDenied -> 403 per D-11)"
  - "Format-Whitelist mit GENAU einem Match-Arm `pdf` (D-12, Pitfall #3)"
  - "ApiDoc fuer utoipa-OpenAPI-Schema-Registry"
  - "generate_export_route() Router-Generator"
  - "lib.rs-Wiring (Module-Decl, ApiDoc-Nest, RestStateDef-Bound an 2 Stellen, Router-Mount)"
affects: [11-05-bin-wiring, 11-06-e2e]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "REST-Layer-Mirror-Enum (ExportIncludeQuery) damit utoipa nicht ins Service-Domain-Crate leakt — analog AttendanceExport Plan 6"
    - "Lokales map_export_error fuer Endpoint-spezifische Status-Code-Policy (PermissionDenied -> 403, andere -> globales From)"
    - "Format-Whitelist mit explizitem Match-Arm pro erlaubtem Format und `other => BadRequest` Catch-All"
    - "REVISION-Fix W2 deterministisch: RestStateDef-Bound-Count fuer neuen Trait MUSS == Bound-Count fuer Vorbild-Trait sein (grep-Count-Check vor Commit)"
    - "Test-Server-Generic-Bound-Sync: cargo build deckt fehlende Bounds in test_server.rs auf (Rule-3-Auto-Fix-Pattern)"

key-files:
  created:
    - "genossi_rest/src/repayment_export.rs (249 LOC: Handler + Query-Params + map_export_error + RestState-Trait + Router-Generator + ApiDoc + 7 Unit-Tests)"
  modified:
    - "genossi_rest/src/lib.rs (+13 Zeilen: pub mod, ApiDoc-Nest, 2x RestStateDef-Bound, .nest()-Router-Mount)"
    - "genossi_rest/src/test_server.rs (+1 Zeile: start_test_server Generic-Bound — Rule-3-Auto-Fix)"

key-decisions:
  - "TDD-RED-Strategie fuer REST-Handler: wrong defaults (ExportIncludeQuery::default()=All, map_export_error->Unauthorized, Format-Whitelist als `_ => Pdf` Catch-All) statt todo!()-Stub. 3 von 7 Tests scheitern semantisch korrekt (default_is_open, default_via_serde, permission_denied_to_403) — saubere RED-Verifikation ohne Runtime-Panic-Noise."
  - "1:1-Mirror von attendance_export.rs (269 LOC -> repayment_export.rs 249 LOC) ohne strukturelle Abweichungen — gleiches Trait-Pattern (Context=crate::ContextType), gleiche map_*_error-Override-Form, gleiches generate_route-Pattern."
  - "Format-Whitelist mit GENAU einem `pdf`-Match-Arm und Catch-All Else-Branch — kein toter Code (keine ungenutzten Csv/Xlsx-Match-Arms), D-12 (CSV gestrichen) auf Code-Pfad-Ebene codifiziert. Re-Add ist additiv (neuer Arm)."
  - "Rule-3 Auto-Fix in test_server.rs: Plan-Spec zaehlte nur 2 Bound-Stellen in lib.rs (create_app + start_server); cargo build deckte eine 3. Stelle in test_server.rs (start_test_server-Generic) auf. Auto-Fix erweitert dort ebenfalls — sonst E2E-Tests via TestServer wuerden nicht compilen."

patterns-established:
  - "Pattern 1: REST-TDD-RED via wrong-defaults — semantisch-korrekte Compile-Time-Failures statt todo!()-Panics. Pattern-Vorlage fuer kuenftige REST-Handler-Plans mit klar definierten Default-Werten und Mapping-Tabellen."
  - "Pattern 2: Bound-Count-Sync-Check als Verify-Snippet — grep-Counts der alten und neuen Trait-Bounds MUESSEN identisch sein (deterministisch). Verhindert 'trait bound not satisfied'-Build-Errors an genau einer vergessenen Stelle."
  - "Pattern 3: cargo build -p {downstream-crate} als Discovery-Mechanismus fuer fehlende Bound-Stellen — wenn der Plan eine bestimmte Anzahl Bounds zaehlt aber das Build in anderen Files (test_server, e2e_tests) erst spaeter aufdeckt, ist Rule-3-Auto-Fix der korrekte Pfad."

requirements-completed: [EXPO-01, EXPO-03, EXPO-05]

# Metrics
duration: 6min
completed: 2026-06-01
---

# Phase 11 Plan 04: RepaymentExport REST-Handler Summary

**REST-Handler `export_repayment` unter `GET /api/repayment-phase/{phase_id}/export/{format}` mit Format-Whitelist NUR `pdf` (D-12), Default-Include `Open` (D-03), lokales `map_export_error` (PermissionDenied -> 403 per D-11), OpenAPI-Schema mit allen 6 Status-Codes, Router-Generator und vollstaendigem lib.rs-Wiring inkl. RestStateDef-Bound-Count-Sync (REVISION-Fix W2 deterministisch).**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-01T05:40:00Z (ca. nach Plan-11.03-Completion)
- **Completed:** 2026-06-01T05:46:00Z
- **Tasks:** 2 (Task 1 TDD-RED→GREEN, Task 2 Wiring)
- **Files modified:** 3 (1 created, 2 modified)
- **Tests added:** 7 (alle 7 Unit-Tests gruen)
- **Suite size:** 69 Tests pass im `genossi_rest --lib` (vorher 62, +7 neue, 0 Regression)

## Accomplishments

- **`genossi_rest::repayment_export`-Modul** mit Handler-Funktion `export_repayment`, lokalem `map_export_error`, Query-Param-Struct `ExportQuery` mit Default-Open, REST-Mirror-Enum `ExportIncludeQuery`, `From<ExportIncludeQuery> for ExportInclude` Mapping, `RepaymentExportRestState`-Trait, `generate_export_route` Router-Generator, `ApiDoc` mit utoipa-OpenAPI-Schema.
- **Format-Whitelist** auf Code-Pfad-Ebene: GENAU `"pdf" => ExportFormat::Pdf`, Catch-All `other => RestError::BadRequest("unknown export format: {}", other)`. Keine toten `ExportFormat::Csv` oder `Xlsx`-Match-Arms — D-12 (CSV gestrichen) compiletime-enforced ueber die `ExportFormat`-Enum aus Plan 11.02 (NUR `Pdf`-Variante).
- **Lokales `map_export_error`** mappt `ServiceError::PermissionDenied -> RestError::Forbidden(403)`; alle anderen Varianten delegieren ans globale `From<ServiceError>` (EntityNotFound -> 404, Conflict -> 409, etc.). Frontend kann "kein Admin" (403) klar von "Session ungueltig" (401) unterscheiden — D-11 / Phase-6-D-13-Pattern 1:1.
- **OpenAPI-Schema** dokumentiert alle 6 Status-Codes (200/400/401/403/404/409) mit Beschreibungen und content_type="application/pdf" fuer 200. Tag "RepaymentExport".
- **lib.rs-Wiring** (4 Stellen): `pub mod repayment_export;`, ApiDoc-Nest im `merged_openapi`-Block, 2x `+ repayment_export::RepaymentExportRestState`-Bound (auf `create_app` UND `start_server`-Generic), `.nest("/api/repayment-phase", repayment_export::generate_export_route::<RestState>())` neben dem existierenden `.nest("/api/repayment-phase", repayment_phase::generate_route::<RestState>())` — Axum 0.8.3 merged die zwei Mounts unter demselben Prefix, Pfade `/{phase_id}` und `/{phase_id}/export/{format}` kollidieren nicht.
- **test_server.rs-Wiring** (1 Stelle, Rule-3-Auto-Fix): Auch `start_test_server`-Generic in `genossi_rest::test_server` braucht den neuen Bound — sonst kompilieren keine E2E-Tests, die `start_test_server` aufrufen.
- **7 Unit-Tests** verifizieren alle Domain-Invarianten: Default=Open (D-03), serde-Deserialisierung "open"/"all"/"paid" (lowercase), From-Mapping ExportIncludeQuery -> ExportInclude, map_export_error fuer PermissionDenied (-> Forbidden) / EntityNotFound (-> NotFound) / Conflict (-> Conflict), `ExportQuery`-Default via serde-JSON-Round-Trip.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Failing Tests + Stub mit wrong defaults** — `ee64d88` (test)
   - Erstellt `genossi_rest/src/repayment_export.rs` mit Handler-Skelett, 7 Unit-Tests, und absichtlich falscher Default-Konfiguration (ExportIncludeQuery::default()=All, map_export_error->Unauthorized, Format-Whitelist als Catch-All `_ => Pdf`).
   - Deklariert `pub mod repayment_export` in `genossi_rest/src/lib.rs`.
   - RED-State verifiziert: 3 Tests scheitern semantisch korrekt (default_is_open mit Open!=All, default_via_serde dito, permission_denied_to_403 mit `Forbidden(_) != Unauthorized`). 4 Tests laufen bereits gruen (Deserialisierung lowercase, From-Mapping, entity_not_found, conflict — sind unabhaengig von den Stub-Bodies).

2. **Task 1 GREEN: Flip defaults zu D-03/D-11/D-12-konform** — `cf9ce76` (feat)
   - Flipped `#[default]` von `All` auf `Open` (D-03).
   - Flipped `map_export_error`: `PermissionDenied -> Forbidden("forbidden".to_string())` (D-11).
   - Ersetzt Catch-All-Format-Match-Arm durch explizites `"pdf" => Pdf` + `other => BadRequest("unknown export format: {}", other)` (D-12 / Pitfall #3).
   - Alle 7 Tests gruen.

3. **Task 2: lib.rs + test_server.rs Wiring** — `04d8d72` (feat)
   - lib.rs: ApiDoc-Nest fuer `/api/repayment-phase/{phase_id}/export`, 2x `+ repayment_export::RepaymentExportRestState`-Bound (REVISION-Fix W2: Bound-Count == AttendanceExportRestState-Count == 2 in lib.rs), `.nest("/api/repayment-phase", repayment_export::generate_export_route::<RestState>())` direkt nach dem `attendance_export`-Mount.
   - test_server.rs: 3. Bound-Stelle (Rule-3-Auto-Fix) im `start_test_server`-Generic; ohne diesen Fix bricht `cargo build -p genossi_rest` mit `trait bound RepaymentExportRestState is not satisfied`.

**REFACTOR phase:** Skipped — Code spiegelt `attendance_export.rs`-Pattern 1:1, kein offensichtlicher Cleanup notwendig.

**Plan metadata commit:** folgt nach diesem SUMMARY.

## Files Created/Modified

- `genossi_rest/src/repayment_export.rs` (NEW, 249 LOC) — Handler `export_repayment`, lokales `map_export_error`, `ExportQuery`/`ExportIncludeQuery`-Types mit From-Impl, `RepaymentExportRestState`-Trait, `generate_export_route` Router-Generator, `ApiDoc` utoipa-Struct, 7 Unit-Tests.
- `genossi_rest/src/lib.rs` (+13 Zeilen) — Modul-Deklaration `pub mod repayment_export;`, ApiDoc-Nest-Eintrag im `merged_openapi`-Block, `+ repayment_export::RepaymentExportRestState` an create_app- und start_server-Generic, `.nest("/api/repayment-phase", repayment_export::generate_export_route::<RestState>())` Router-Mount mit erklaerendem Kommentar.
- `genossi_rest/src/test_server.rs` (+1 Zeile, Rule-3-Auto-Fix) — `+ crate::repayment_export::RepaymentExportRestState` an `start_test_server`-Generic, sonst kompilieren keine E2E-Tests.

## Decisions Made

- **TDD-RED-Strategie fuer REST-Handler:** Statt `todo!()`-Stubs (Plan-11.01-Pattern fuer Funktions-Bodies) wurden absichtlich falsche Defaults gewaehlt — `#[default] All` statt `#[default] Open`, `Unauthorized` statt `Forbidden`, Catch-All-Match-Arm statt Whitelist. Resultat: 3 von 7 Tests scheitern semantisch korrekt mit assertion-Failures (`left: All, right: Open`), 4 Tests greifen Code-Pfade die unabhaengig von den Stub-Bodies sind und laufen schon gruen. Keine Runtime-Panic-Noise.
- **REST-Layer-Mirror-Enum ExportIncludeQuery** statt direkter Wiederverwendung von `genossi_service::repayment_export::ExportInclude`: Spiegelt Phase-6-Pattern 1:1 — utoipa-Derives bleiben im REST-Crate, Service-Domain-Crate bleibt utoipa-frei. Cost: eine zusaetzliche `From<ExportIncludeQuery> for ExportInclude`-Impl (3 Zeilen).
- **REVISION-Fix W2 deterministisch via grep-Count:** Statt Plan-Text-Statisch zu zaehlen ("an beiden Stellen") wird die Anzahl der RestStateDef-Bound-Stellen aus dem Code per `grep -c "attendance_export::AttendanceExportRestState"` abgeleitet, und der neue Bound muss an EXAKT der gleichen Anzahl Stellen erscheinen. Vor Commit verifiziert: ATT_COUNT=2 == REP_COUNT=2 in `lib.rs`. Verhindert deterministisch das Vergessen-einer-Stelle-Risiko (das im Plan-Audit als REVISION-Fix W2 explizit benannt war).
- **test_server.rs ist eigenstaendige Bound-Stelle ausserhalb lib.rs:** Plan-Spec zaehlte nur Bounds in `lib.rs` (2 Stellen: create_app + start_server); `cargo build -p genossi_rest` deckte eine 3. Stelle in `test_server.rs` auf. Rule-3-Auto-Fix (Blocking) erweitert den Bound dort ebenfalls — ohne den Fix wuerde Plan 11.06 (E2E-Tests via `start_test_server`) nicht compilen, und auch der `cargo test -p genossi_rest --lib` haette weiter funktioniert nur weil `test_server` ein eigenes Modul ist und der Generic-Bound-Check erst beim Aufruf greift. Pattern-Anker fuer kuenftige Trait-Erweiterungen: ALLE Stellen mit RestStateDef-Bound-Listen suchen, nicht nur in `lib.rs`.
- **1:1-Mirror-Disziplin:** Strukturelle Abweichungen vom Phase-6-Pattern wurden NICHT eingefuehrt — gleiche Module-Struktur, gleiche `map_*_error`-Form, gleiches `generate_route`-Pattern. Plan 11.05 (DI-Wiring) und Plan 11.06 (E2E) koennen dieselbe Architektur-Erwartung uebernehmen.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] test_server.rs Bound-Stelle nachgezogen**

- **Found during:** Task 2 (lib.rs-Wiring) Build-Check `cargo build -p genossi_rest`
- **Issue:** Plan-Spec zaehlte nur 2 RestStateDef-Bound-Stellen in `genossi_rest/src/lib.rs` (Zeilen 444 und 767, `create_app` + `start_server`). `cargo build -p genossi_rest` schlug nach dem lib.rs-Edit mit 3 Errors fehl: `error[E0277]: the trait bound RestState: RepaymentExportRestState is not satisfied --> genossi_rest/src/test_server.rs:40:30`. `start_test_server`-Generic in `test_server.rs` listet ebenfalls alle RestStateDef-Bounds explizit.
- **Fix:** `+ crate::repayment_export::RepaymentExportRestState` an `start_test_server`-Generic in `test_server.rs` Zeile 28 hinzugefuegt (direkt nach `crate::attendance_export::AttendanceExportRestState`).
- **Files modified:** `genossi_rest/src/test_server.rs` (+1 Zeile)
- **Verification:** `cargo build -p genossi_rest 2>&1 | grep -c "^error"` == 0 nach dem Fix; `cargo test -p genossi_rest --lib` 69/69 gruen.
- **Committed in:** `04d8d72` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 3 — Blocking, test_server.rs Bound-Stelle)
**Impact on plan:** Keine Scope-Aenderung. Alle 3 Requirements (EXPO-01/03/05) erfuellt; alle 7 Tests gruen; lib.rs-Wiring + Bound-Count-Sync verifiziert; cargo build -p genossi_rest clean. Plan-11.06-E2E-Tests via `start_test_server` koennen jetzt direkt das neue Endpoint exercieren.

## Issues Encountered

None — RED- und GREEN-Build kompilieren beide ohne Errors (modulo der bekannten Rule-3-Bound-Stelle in test_server.rs, die im selben Task gefixed wurde). Alle 7 Tests laufen wie erwartet (RED: 3 fail / 4 pass; GREEN: 7 pass). Volle Crate-Test-Suite 69/69 OK ohne Regression in den 62 bestehenden Tests. Alle Acceptance-Criteria-Greps verifiziert.

## User Setup Required

None — keine externen Services, keine ENV-Variablen, keine Dashboard-Konfiguration.

## Next Phase Readiness

**Bereit fuer Plan 11.05** (DI-Wiring `impl RepaymentExportRestState for RestStateImpl` in `genossi_bin/src/lib.rs`):

- `RepaymentExportRestState`-Trait ist `pub` und importierbar via `genossi_rest::repayment_export::RepaymentExportRestState`.
- Trait-Signatur erwartet `type RepaymentExportService: RepaymentExportService<Context = crate::ContextType>` und `fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService>` — Plan 11.05 muss exakt dieses Trait implementieren.
- `genossi_bin` builds aktuell mit Error `the trait RepaymentExportRestState is not implemented for RestStateImpl` — Plan 11.05 schliesst diese Luecke und macht `cargo build -p genossi_bin` wieder clean.

**Bereit fuer Plan 11.06** (E2E-Tests):

- `start_test_server` Generic-Bound ist bereits korrekt (Rule-3-Auto-Fix in diesem Plan).
- REST-Route `/api/repayment-phase/{phase_id}/export/{format}` und Query-Param `?include=` sind registriert und ueber OpenAPI dokumentiert — E2E-Tests koennen direkt gegen den Pfad asserten.

Keine Blocker.

## Self-Check: PASSED

Verifications run after writing SUMMARY:

- [x] `genossi_rest/src/repayment_export.rs`: FOUND (249 LOC)
- [x] `genossi_rest/src/lib.rs` mit `pub mod repayment_export;`: FOUND
- [x] `genossi_rest/src/test_server.rs` mit `+ crate::repayment_export::RepaymentExportRestState`: FOUND
- [x] Commit `ee64d88` (Task 1 RED test): FOUND in git log
- [x] Commit `cf9ce76` (Task 1 GREEN feat): FOUND in git log
- [x] Commit `04d8d72` (Task 2 lib.rs+test_server wiring): FOUND in git log
- [x] `cargo build -p genossi_rest 2>&1 | grep -c "^error"` == 0
- [x] `cargo test -p genossi_rest --lib repayment_export 2>&1 | grep "test result: ok"` == 1 (7 passed)
- [x] `cargo test -p genossi_rest --lib`: 69/69 OK, 0 failures
- [x] `grep -cE "pub async fn export_repayment"` == 1
- [x] `grep -cE "fn map_export_error"` == 1
- [x] `grep -cE "\"pdf\" => ExportFormat::Pdf"` == 1
- [x] `grep -cE "ExportFormat::Csv"` == 0 (D-12: kein Csv-Match-Arm)
- [x] `grep -cE "ExportFormat::Xlsx"` == 0 (D-12: kein Xlsx-Match-Arm)
- [x] `grep -cE "pub trait RepaymentExportRestState"` == 1
- [x] `grep -cE "content_disposition_attachment"` == 1
- [x] `grep -cE "pub fn generate_export_route"` == 1
- [x] `grep -cE "pub struct ApiDoc"` == 1
- [x] `grep -cE "^pub mod repayment_export"` in lib.rs == 1
- [x] `grep -cE "repayment_export::ApiDoc"` in lib.rs == 1
- [x] `grep -c "repayment_export::RepaymentExportRestState"` in lib.rs == `grep -c "attendance_export::AttendanceExportRestState"` in lib.rs (BEIDE 2; REVISION-Fix W2 deterministisch erfuellt)
- [x] `grep -cE "repayment_export::generate_export_route"` in lib.rs == 1

## TDD Gate Compliance

- **RED gate (`test(...)` commit):** `ee64d88` — fuegt 7 failing Tests + Stub mit wrong defaults (ExportIncludeQuery::default()=All, map_export_error->Unauthorized, Format-Whitelist als Catch-All). 3 Tests scheitern semantisch korrekt; 4 Tests greifen Code-Pfade unabhaengig von Stub-Bodies und sind im RED bereits gruen.
- **GREEN gate (`feat(...)` commit) nach RED:** `cf9ce76` — flippt Defaults zu D-03/D-11/D-12-konform; alle 7 Tests gruen. Volle Suite 69/69.
- **REFACTOR gate:** skipped — Implementation spiegelt `attendance_export.rs`-Pattern 1:1; kein Cleanup notwendig.

Gate-Sequence verified via `git log --oneline -3`: `ee64d88` (test) -> `cf9ce76` (feat) -> `04d8d72` (feat lib.rs wiring, second feat-Commit ist Task 2 ohne TDD-Cycle).

---
*Phase: 11-export-pdf-csv*
*Completed: 2026-06-01*
