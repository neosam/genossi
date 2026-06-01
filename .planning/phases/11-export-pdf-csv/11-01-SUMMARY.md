---
phase: 11-export-pdf-csv
plan: 01
subsystem: pdf-rendering
tags: [pdf, typst, template, repayment, render]

requires:
  - phase: 07-repayment-phase
    provides: RepaymentPhaseEntity (fiscal_year, share_value, status, …)
  - phase: 06-attendance-export
    provides: render_attendance_list / build_inputs_attendance pattern (mirror)
provides:
  - templates/defaults/auszahlungsliste.typ (6-Spalten-Tabelle mit Repeat-Header und Summenzeile)
  - DEFAULT_TEMPLATES-Eintrag fuer Fresh-Install-Provisioning (Pitfall #1)
  - PdfGenerator::render_repayment_list-Methode (RepaymentPhaseEntity + RepaymentExportRow[] -> PDF-Bytes)
  - RepaymentExportRow-Struct (pub, 6 Felder mit UTF-8-purpose)
  - build_inputs_repayment-Helper (meta + rows JSON-Strings; total_amount_str ohne .abs())
affects: [11-02, 11-03, 11-04, 11-05, 11-06]

tech-stack:
  added: []
  patterns:
    - "Render-Foundation-Pattern: Template-File + DEFAULT_TEMPLATES-Eintrag + PdfGenerator-Methode + build_inputs_*-Helper (mirror Phase 6)"
    - "Pre-computed-Service-Pattern: Service liefert amount_str und purpose als fertige Strings; Renderer macht keine Lokalisierung"
    - "CARGO_MANIFEST_DIR-Pfad-Pattern: Cargo-Working-Dir-unabhaengige Template-Pfade in Tests"

key-files:
  created:
    - templates/defaults/auszahlungsliste.typ
  modified:
    - genossi_service_impl/src/template_storage.rs
    - genossi_service_impl/src/pdf_generation.rs

key-decisions:
  - "Phase-10-D-04-Pattern (Euro-Formatierung) OHNE .abs() konsistent mit PATTERNS.md §S9 — Domain-Constraint garantiert non-negative cents"
  - "Optionale Summenzeile im Footer implementiert (Planner-Discretion erlaubt; Banking-Vorstand-Nice-to-Have)"
  - "RepaymentPhaseEntity-Konstruktion in Tests via PrimitiveDateTime::new(now.date(), now.time()) statt OffsetDateTime::into() (kein From-Impl in time crate)"
  - "TempDir::new statt env::temp_dir + create_dir_all in Tests (auto-cleanup, kollisionsfrei zwischen parallelen Test-Runs)"

patterns-established:
  - "Render-Methode mirrored render_attendance_list 1:1 (gleiche Error-Wrapping-Stufen NotFound/IO/Compile/Serialise)"
  - "build_inputs_*-Helper sind file-scope fn (nicht impl-Method) — ermoeglicht reine Unit-Tests ohne PdfGenerator-Konstruktion"
  - "RepaymentExportRow.iban als String (D-07: leerer String fuer fehlende IBAN, kein Option<String>) — Service uebergibt unwrap_or_default()"
  - "Test-Pfade via Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"../templates/defaults/…\") fuer deterministischen Cross-Cargo-Run-Pfad"

requirements-completed:
  - EXPO-02

duration: 6min
completed: 2026-06-01
---

# Phase 11 Plan 01: PDF-Render-Foundation Summary

**Typst-basiertes Auszahlungslisten-Rendering mit 6-Spalten-Tabelle (Nr./Name/IBAN/Anteile/Betrag/Verwendungszweck), Repeat-Header, optionaler Summenzeile, UTF-8-Verwendungszweck-Strings und fresh-install Default-Template-Provisioning.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-01T05:01:26Z
- **Completed:** 2026-06-01T05:07:27Z
- **Tasks:** 2
- **Files modified:** 3 (1 created, 2 modified)
- **Tests added:** 2 (`test_render_repayment_list_with_empty_rows`, `test_render_repayment_list_with_two_rows`)
- **Suite size:** 278 tests pass (von vorher 276; +2 neue, 0 Regression)

## Accomplishments

- Neues Typst-Template `templates/defaults/auszahlungsliste.typ` rendert die 6-Spalten-Tabelle mit `table.header(repeat: true, …)` und einer optionalen Summenzeile ("Gesamt: X Eintraege — Summe Y EUR"). Spalten-Verteilung `auto/1fr/auto/auto/auto/1fr` mit Right/Left-Alignment fuer Zahlen-Lesbarkeit.
- DEFAULT_TEMPLATES-Eintrag in `template_storage.rs` registriert per `include_bytes!` — Fresh-Installs schreiben das Template via `provision_defaults()` auf Disk; Pitfall #1 ("template not found" beim ersten PDF-Export) ist verhindert.
- `PdfGenerator::render_repayment_list(template_path, template_base, phase, rows) -> Result<Vec<u8>, ServiceError>` ist eine 1:1-Spiegelung von `render_attendance_list`: same Error-Wrapping (NotFound -> `ServiceError::InternalError("template not found: …")`; IO/Compile/PDF-Serialise getrennt), same TemplateWorld-Konstruktion.
- `RepaymentExportRow` (pub, 6 Felder) ist von 11.02/11.03 importierbar; `purpose` und `amount_str` werden vom Service pre-computed uebergeben — Renderer macht keine Lokalisierung und keine ASCII-Sanitization.
- `build_inputs_repayment` aggregiert `meta.total_amount_str` ueber alle Rows ohne `.abs()` (REVISION-Fix B3, Phase-10-D-04-Konsistenz mit PATTERNS.md §S9).
- 2 neue Unit-Tests verankern beide Pfade: Empty-Rows-Render (Smoke-Test, PDF-Magic) + Two-Rows-Render mit `"Anteilsrückzahlung GJ 2026 1234 Hans Müller"` als purpose (D-04 wortwoertlich, D-05 no-sanitization, Umlauten-Render-Path verifiziert).

## Task Commits

1. **Task 1: Typst-Template + DEFAULT_TEMPLATES** — `60b998a` (feat)
   Neue Datei `auszahlungsliste.typ` + Registry-Eintrag in `template_storage.rs`.

2. **Task 2: Renderer-Methode + Tests (TDD)** — RED `ce08375` + GREEN `6b60111`
   - RED `ce08375` (test): `RepaymentExportRow`-Struct + `render_repayment_list`-Stub mit `todo!()` + zwei Unit-Tests. Tests failen mit `not yet implemented` (verifiziert).
   - GREEN `6b60111` (feat): Echte Implementation von `render_repayment_list` und `build_inputs_repayment`. Beide Tests grün; gesamte `cargo test -p genossi_service_impl --lib`: 278/278 OK.

**Plan metadata:** (folgt im finalen Plan-Metadata-Commit nach STATE/ROADMAP-Updates)

## Files Created/Modified

- `templates/defaults/auszahlungsliste.typ` (CREATED) — Typst-Template, 6-Spalten-Tabelle mit Repeat-Header, optionaler Summenzeile, UTF-8-Inputs via `sys.inputs.at("meta")` / `sys.inputs.at("rows")`.
- `genossi_service_impl/src/template_storage.rs` (MODIFIED) — Neuer `DefaultTemplate { path: "auszahlungsliste.typ", content: include_bytes!(…) }`-Eintrag nach `teilnehmerliste.typ` (chronologische Phase-Sortierung erhalten).
- `genossi_service_impl/src/pdf_generation.rs` (MODIFIED) — Neuer `use genossi_dao::repayment_phase::RepaymentPhaseEntity`-Import, neue pub Struct `RepaymentExportRow`, neue impl-Methode `render_repayment_list`, neuer file-scope Helper `build_inputs_repayment`, 1 Test-Helper `test_repayment_phase()`, 2 neue Unit-Tests.

## Decisions Made

- **Test-Pfade via `CARGO_MANIFEST_DIR`** (REVISION-Fix W5): `Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/defaults/…")` macht die Template-Pfade reproduzierbar unabhaengig vom Cargo-Working-Dir (Workspace-Root vs. Crate-Dir). Pattern fuer kuenftige Tests, die Workspace-relative Pfade brauchen.
- **`TempDir::new()` statt `std::env::temp_dir().join("…")` + `create_dir_all`** (Deviation zur Plan-Text-Form, Rule 1): TempDir kommt mit auto-cleanup und kollisionsfreien Unique-Subdirs — sicherer fuer parallele `cargo test` Runs als ein gemeinsames Sub-Verzeichnis im OS-temp.
- **Summe-Aggregation ohne `.abs()`** (REVISION-Fix B3): Phase-10-D-04-Pattern-Konsistenz. Inline-Kommentar dokumentiert Domain-Constraint.
- **purpose-String als Service-Verantwortung**, nicht Template-Logik (D-04, D-05): Renderer kennt das `Anteilsrückzahlung`-Schema NICHT — Service uebergibt den fertigen String. Verhindert Coupling von Verwendungszweck-Text an Typst-Template-Aenderungen.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `PdfGenerator::new()` gibt `Self` direkt zurueck, nicht `Result<Self, _>`**

- **Found during:** Task 2 (Test-Schreiben)
- **Issue:** Plan-Text-Snippet `let gen_ = PdfGenerator::new().unwrap();` ist falsch — `PdfGenerator::new()` hat Signatur `pub fn new() -> Self` (siehe `pdf_generation.rs:135`); kein `Result`.
- **Fix:** `let generator = PdfGenerator::new();` ohne `.unwrap()` in beiden Tests.
- **Files modified:** `genossi_service_impl/src/pdf_generation.rs` (Test-Bereich).
- **Verification:** Build clean, beide Tests grün.
- **Committed in:** `ce08375` (RED-Commit, da der Bug bereits im Test-Stub auftrat).

**2. [Rule 1 - Bug] `time::OffsetDateTime::now_utc().into()` kompiliert nicht zu `PrimitiveDateTime`**

- **Found during:** Task 2 (Test-Helper `test_repayment_phase()`)
- **Issue:** Plan-Text-Snippet `created: time::OffsetDateTime::now_utc().into()` setzt einen nicht existierenden `From<OffsetDateTime> for PrimitiveDateTime`-Impl voraus. Die `time` 0.3 Crate hat diesen Impl nicht. Vorhandene Service-Konvention: `let now = OffsetDateTime::now_utc(); let pdt = PrimitiveDateTime::new(now.date(), now.time());` (siehe `genossi_service_impl/src/repayment_phase.rs:115-116`).
- **Fix:** Test-Helper `test_repayment_phase()` extrahiert; nutzt explizit `PrimitiveDateTime::new(now.date(), now.time())` und wird in beiden Tests aufgerufen.
- **Files modified:** `genossi_service_impl/src/pdf_generation.rs` (Test-Bereich).
- **Verification:** Build clean, beide Tests grün.
- **Committed in:** `ce08375` (RED-Commit) — der Helper wurde so direkt eingebaut, dass RED nur am `todo!()` panic fail (nicht am Compile-Fehler).

---

**Total deviations:** 2 auto-fixed (beide Rule-1-Bugs in Plan-Text-Snippets, im Test-Setup gefangen)
**Impact on plan:** Keine Scope-Aenderung. Beide Auto-Fixes erhalten die TDD-Sequence (RED-fail wegen `todo!()`, nicht wegen Compile-Fehler) und die Plan-Intention 1:1 (D-04 wortwoertlich, REVISION-Fix W5/W6/B1/B3 alle umgesetzt).

## Issues Encountered

None — Tests grün beim ersten Run nach GREEN-Implementation; keine Compile-Errors nach RED-Commit; vollstaendige `cargo test -p genossi_service_impl --lib` grün (278/278), keine Regression in den 276 bestehenden Tests.

## User Setup Required

None — keine externen Services, keine ENV-Variablen, keine Dashboard-Konfiguration.

## Next Phase Readiness

**Bereit für Plan 11.02** (PATTERNS-Reference / Lower-Level-Helper, falls vorgesehen) und **Plan 11.03** (Service-Impl `RepaymentExportServiceImpl`):

- `PdfGenerator::render_repayment_list` ist `pub` und ueber `crate::pdf_generation::{PdfGenerator, RepaymentExportRow}` importierbar.
- Pre-computed Service-Pattern ist verankert: Plan 11.03 muss aus DAO-Rows die Felder `member_number`, `name` (`"first_name last_name"`), `iban` (`Member.bank_account.unwrap_or_default()`), `share_count`, `amount_str` (Phase-10-D-04-Format), `purpose` (D-04-Wortlaut mit Original-Umlauten) bauen.
- Fresh-Install-Pfad ist abgesichert via `provision_defaults()` — Plan 11.04 (REST-Endpoint) muss keinen Template-Existenz-Check mehr machen.

Keine Blocker.

## Self-Check: PASSED

Verifications run after writing SUMMARY:

- [x] `templates/defaults/auszahlungsliste.typ` exists (1731 bytes).
- [x] `genossi_service_impl/src/template_storage.rs` modified (new DefaultTemplate entry).
- [x] `genossi_service_impl/src/pdf_generation.rs` modified (struct + method + helper + tests).
- [x] Commit `60b998a` exists (Task 1: feat — template + registry).
- [x] Commit `ce08375` exists (Task 2 RED: test — failing tests + stub).
- [x] Commit `6b60111` exists (Task 2 GREEN: feat — full impl).
- [x] `cargo test -p genossi_service_impl --lib`: 278/278 OK, 0 failures.
- [x] `grep -c "fn render_repayment_list" pdf_generation.rs` == 1.
- [x] `grep -c "struct RepaymentExportRow" pdf_generation.rs` == 1.
- [x] `grep -c "fn build_inputs_repayment" pdf_generation.rs` == 1.
- [x] `grep -c "Anteilsrückzahlung" pdf_generation.rs` == 5 (>= 2 required).
- [x] `grep -c "Anteilsrueckzahlung" pdf_generation.rs` == 0 (no ASCII variant — D-05).
- [x] `grep -c "CARGO_MANIFEST_DIR" pdf_generation.rs` == 3 (>= 2 required).
- [x] `.abs()` count in `build_inputs_repayment`-Block == 0 (REVISION-Fix B3).

## TDD Gate Compliance

- **RED gate (`test(...)` commit):** `ce08375` — adds failing tests + `todo!()` stub. Tests panic with `not yet implemented`.
- **GREEN gate (`feat(...)` commit) after RED:** `6b60111` — implements `render_repayment_list` + `build_inputs_repayment`. Both tests pass.
- **REFACTOR gate:** skipped — implementation directly mirrors `render_attendance_list` pattern; no cleanup needed.

Gate sequence verified via `git log --oneline -3`.

---
*Phase: 11-export-pdf-csv*
*Completed: 2026-06-01*
