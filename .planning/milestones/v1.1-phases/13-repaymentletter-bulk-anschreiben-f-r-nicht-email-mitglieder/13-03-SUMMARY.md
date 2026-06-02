---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 03
subsystem: render
tags: [phase-13, pdf, typst, render, sys-inputs, bundle, tdd]

requires:
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "01"
    provides: "auszahlungs_anschreiben.typ + _bundle.typ Templates + DEFAULT_TEMPLATES-Eintraege"
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "02"
    provides: "genossi_service::repayment_context::RepaymentContext (parallel in Wave 2)"
provides:
  - "PdfGenerator::render_repayment_letter(template_path, template_base, phase, member, ctx) -> Result<Vec<u8>>"
  - "PdfGenerator::render_repayment_letter_bundle(template_path, template_base, phase, recipients) -> Result<Vec<u8>>"
  - "build_inputs_repayment_letter + build_inputs_repayment_letters_bundle Helper (sys.inputs JSON-Pattern)"
affects: [13-04, 13-05, 13-07]

tech-stack:
  added: []
  patterns:
    - "Synchrone Render-Methode auf PdfGenerator analog render_repayment_list — fuer Letter+Bundle"
    - "build_inputs_*-Helper bauen serde_json::json! → typst::foundations::Value::Str (sys.inputs)"
    - "Compat-Layer in build_inputs_repayment_letters_bundle: setzt zusaetzlich `member` + `repayment` aus dem ersten Recipient, damit der Plan-13-01-Bundle-Template-#import nicht crasht (Plan-13-01 Single-Template hat Top-Level `#let member = json.decode(sys.inputs.at(\"member\"))` Side-Effects, die beim Import evaluiert werden)"
    - "Smoke-Test mit echtem Template + Logo-Asset → Pattern fuer kuenftige Letter-Typst-Render-Tests"

key-files:
  created:
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-03-SUMMARY.md"
  modified:
    - "genossi_service_impl/src/pdf_generation.rs"

key-decisions:
  - "Smoke-Test-Heuristik fuer Bundle-Size: absolute Delta `bundle - single >= 5kB` STATT 1.5x-Ratio (threat-model #6) — Logo-Embed dominiert die single-PDF-Groesse (44k von 60k im 2-Recipient-Bundle), Ratio waere false-positive. Strikt-groesser-als plus 5k-Delta deckt 'zweiter Brief tatsaechlich gerendert' robust ab."
  - "Compat-Workaround fuer Plan-13-01 Bundle-Template-Bug statt Template-Fix: parallel_execution-Note erlaubt nur pdf_generation.rs-Touching. build_inputs_repayment_letters_bundle setzt `member` + `repayment` Keys mit first-recipient-Daten als Top-Level-Compat. Bundle-Loop liest aus `recipients[]` und ignoriert die Compat-Keys. Plan-13-01-Template sollte spaeter via `sys.inputs.at(\"member\", default: none)`-Pattern defensiv gemacht werden (siehe deferred-items)."
  - "RepaymentContext-Import aus genossi_service::repayment_context (Plan-13-02-Output) statt lokaler Duplikat-Definition — Plan 13-02 lief parallel in Wave 2 und committed seine GREEN-Phase zwischen meinen RED-/GREEN-Commits; Build-Konsistenz beibehalten."

patterns-established:
  - "Smoke-Test fuer Typst-Brief-Templates: provision_letter_templates() kopiert auszahlungs_anschreiben.typ + _bundle.typ + nebenan-unverpackt-logo.svg in TempDir — Logo MUSS mitkopiert werden, sonst Compile-Error. Vorlage fuer Plan 13-04+ Service-Render-Tests."
  - "Bundle-Size-Heuristik: absolute Delta statt Ratio bei Templates mit Logo-Embed (logo embed wird im Bundle nur 1x referenziert, Ratio waere unter 1.5x trotz mehrerer Recipients)"

requirements-completed: []

duration: 10min
completed: 2026-06-01
---

# Phase 13 Plan 03: PdfGenerator-Erweiterung (Single + Bundle Render) Summary

**PdfGenerator bekommt zwei neue synchrone Render-Methoden — `render_repayment_letter` (1 Member → 1 PDF) und `render_repayment_letter_bundle` (N Members → 1 PDF mit `#pagebreak()`) — plus zwei build_inputs-Helper (sys.inputs JSON-Pattern). TDD RED-GREEN-Cycle pro Task; 9 Helper-Tests + 4 Smoke-Tests; Plan-13-01-Bundle-Template-Bug per Compat-Layer abgefangen.**

## Performance

- **Duration:** ~10 min (zwischen `c6baa8d` 23:53:15 und `445e2df` 00:03:23)
- **Tasks:** 2 (beide TDD: Task 1 RED+GREEN fuer Helper, Task 2 RED+GREEN fuer Render-Methoden)
- **Files modified:** 1 (`genossi_service_impl/src/pdf_generation.rs`)
- **Commits:** 4 (2 RED + 2 GREEN)
- **Tests added:** 13 (9 Helper-Unit-Tests + 4 Render-Smoke-Tests)

## Accomplishments

**Task 1 — Helper-Funktionen (RED `c6baa8d` → GREEN `31c0642`):**
- `build_inputs_repayment_letter(phase, member, ctx) -> Dict`: setzt drei Top-Level-Keys (`member`, `repayment`, `today`); `member`-JSON enthaelt alle 10 Felder (member_number, salutation via `.as_str()`, title, first_name, last_name, street, house_number, postal_code, city, bank_account); optional fields → JSON null (Pitfall #5 fuer `bank_account`).
- `build_inputs_repayment_letters_bundle(phase, recipients) -> Dict`: setzt `recipients[]`-Array mit `{member, repayment}` Sub-JSON pro Recipient; zusaetzlich `today` und `meta` (fiscal_year, recipient_count, phase_id).
- 9 Unit-Tests verifizieren JSON-Shape inkl. NULL-IBAN-Edge-Case (Pitfall #5) in beiden Pfaden.

**Task 2 — Render-Methoden (RED `0905d43` → GREEN `445e2df`):**
- `PdfGenerator::render_repayment_letter`: Pipeline 1:1 wie `render_repayment_list` — Read template (mit `template not found`-Substring im InternalError), `build_inputs_repayment_letter`, `TemplateWorld::new`, `typst::compile::<PagedDocument>`, `typst_pdf::pdf(...)`.
- `PdfGenerator::render_repayment_letter_bundle`: gleiche Pipeline, nutzt `build_inputs_repayment_letters_bundle`.
- 4 Smoke-Tests: real-render gegen Plan-13-01-Templates (single + bundle) mit Logo-Asset → PDF magic + Size-Heuristik; NULL-IBAN-Edge-Case rendert ohne Crash (D-13-06 Baustein 3); template-not-found-Error-Pfad; Bundle-Size > Single + 5kB (Delta-Heuristik).

**Scope-Gate verifiziert:** Alle 4 Commits modifizieren AUSSCHLIESSLICH `genossi_service_impl/src/pdf_generation.rs` (kein Touch an Plan-13-01-Templates, kein Touch an `lib.rs` — keine Merge-Konflikt-Gefahr mit dem parallelen Plan-13-02-Agenten).

## Task Commits

1. **Task 1 RED — failing helper tests** — `c6baa8d` (test)
2. **Task 1 GREEN — implement build_inputs helpers** — `31c0642` (feat)
3. **Task 2 RED — failing render tests** — `0905d43` (test)
4. **Task 2 GREEN — implement render methods + compat-layer** — `445e2df` (feat)

## Files Created/Modified

- `genossi_service_impl/src/pdf_generation.rs` — +416 LOC: Imports `MemberEntity` + `RepaymentContext`; 2 neue Helper-Funktionen (`build_inputs_repayment_letter` + `build_inputs_repayment_letters_bundle`); 2 neue PdfGenerator-Methoden (`render_repayment_letter` + `render_repayment_letter_bundle`); 13 neue Tests + Test-Helper (`sample_member_with_iban`, `sample_member_without_iban`, `sample_ctx`, `extract_str_input`, `provision_letter_templates`).

## Decisions Made

### Plan-Discretion: Smoke-Test-Heuristik

Plan-Text empfahl `bundle.len() > single.len() * 1.5` als "mehrere Seiten"-Indikator. Echte Messung ergab Single=44 728 Bytes, Bundle (2 Recipients)=60 839 Bytes — Ratio nur ~1.36x. Ursache: das `nebenan-unverpackt-logo.svg`-Embed (vermutlich ~30+kB) dominiert die Single-PDF-Groesse; das Bundle bettet das Logo nur 1x. **Plan threat-model bullet #6 hatte das vorausgesehen** ("Bundle-Size-Heuristik (>1.5x Single) ist false-positive-anfaellig") und Plan-Discretion fuer alternative Page-Count-Verifikation gewaehrt. Stattdessen umgesetzt: strikt-groesser-als (`bundle > single`) PLUS absolute Delta (`bundle - single >= 5 000 Bytes`). 5kB Delta entspricht klar mehreren Seiten Brief-Text-Content (Falzmarken, Reference-Tabelle, IBAN-Block, Vorstands-Signatur — ohne Logo). Robust gegen False-Positives und nicht von Font-Compression-Drift abhaengig.

### Plan-Discretion: Logo-Asset im Smoke-Test mitkopieren

Plan-Text-Snippet sah `provision_letter_templates`-Helper nicht vor. Bei naiver Implementierung (nur die zwei Templates kopieren) bricht der Render mit "file not found"-Fehler fuer `nebenan-unverpackt-logo.svg`. Loesung: Helper kopiert ZUSAETZLICH `templates/nebenan-unverpackt-logo.svg` in den TempDir. **Pattern-Anker fuer Plan 13-04+ Service-Render-Tests** und Production-Provisioning: das Logo muss ebenfalls bereitgestellt werden (entweder via DEFAULT_TEMPLATES include_bytes! oder via separater Asset-Provisioning).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan-13-01-Bundle-Template `#import`-Side-Effect**

- **Found during:** Task 2 GREEN — `test_render_repayment_letter_bundle_smoke` crashte mit `dictionary does not contain key "member" and no default value was specified`.
- **Issue:** Das Plan-13-01-`auszahlungs_anschreiben_bundle.typ` macht `#import "auszahlungs_anschreiben.typ": render-letter`. Typst evaluiert beim Import alle Top-Level-Statements des importierten Moduls, darunter das `#let member = json.decode(sys.inputs.at("member"))` im Single-Template — das fail-t, wenn nur `recipients` in `sys.inputs` steht (Bundle-Use-Case).
- **Fix:** Compat-Layer in `build_inputs_repayment_letters_bundle` — setzt zusaetzlich `member` + `repayment` Top-Level-Keys vom ersten Recipient (oder Dummy-Daten bei leerem Bundle). Der Bundle-Loop liest aus `recipients[]` und ignoriert die Compat-Keys. Bundle-Render durchlaeuft jetzt sauber.
- **Files modified:** `genossi_service_impl/src/pdf_generation.rs` (build_inputs_repayment_letters_bundle)
- **Commit:** `445e2df`
- **Scope-Decision:** Plan-13-01-Template selbst NICHT modifiziert (parallel_execution-Note: "ONLY pdf_generation.rs"). Production-Templates muessen via `sys.inputs.at("member", default: none)`-Pattern defensive gemacht werden — siehe deferred-items.

### Threat-Model-Note vs. Bundle-Size-Heuristik

Plan threat-model bullet #6 dokumentierte das Risiko der `>1.5x`-Heuristik vorab und gewaehrte Plan-Discretion. Umgesetzte Alternative (absolute Delta) ist robuster — keine Test-Flakiness erwartet.

## Issues Encountered

### Pre-Existing — Untracked typst-packages/ im genossi_service_impl/-Folder

Beim Test-Run wird `./typst-packages/`-Cache **relativ zum Test-CWD** angelegt, was zu einem `genossi_service_impl/typst-packages/`-Folder fuehrt (das Package-Cache-Default ist `PackageCache::new()` mit `./typst-packages`). Top-Level `typst-packages/` ist im Repo committed mit den letter-pro-Files; der nested Folder ist nur Test-Side-Effect. **Out-of-scope fuer Plan 13-03** — sollte via `.gitignore` (z.B. `**/typst-packages/`) gefangen werden, ist aber kein Funktionsproblem. Logged als deferred-item.

## Deferred Items

1. **Plan-13-01-Template defensive machen:** `auszahlungs_anschreiben.typ` sollte Top-Level `#let member = json.decode(sys.inputs.at("member"))` zu `#let member = if "member" in sys.inputs { json.decode(sys.inputs.at("member")) } else { none }` umstellen, damit das Bundle-Template ohne Compat-Layer importieren kann. Aktuell wird die Symptom-Behandlung in `build_inputs_repayment_letters_bundle` durchgefuehrt. Klares Refactoring-Target fuer eine spaetere Wartungs-Phase.

2. **`.gitignore` fuer nested typst-packages:** `**/typst-packages/` ergaenzen, damit Test-Runs keinen untracked-Output mehr hinterlassen.

3. **Logo-Asset-Provisioning fuer Production:** Plan 13-04+ Service-Wiring muss klaeren, wie `nebenan-unverpackt-logo.svg` aus `templates/` auf den DEPLOYED `TEMPLATE_PATH` kommt. Heutige `provision_defaults` schreibt nur die Templates aus `DEFAULT_TEMPLATES` (include_bytes! im genossi_service_impl-Crate). Optionen: (a) zusaetzlicher Default-Asset-Eintrag im template_storage; (b) Production-File-Copy-Hook im genossi_bin-Startup. Plan-13-04 Discretion.

## Self-Check

```
=== Files exist ===
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/pdf_generation.rs
FOUND: /home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-03-SUMMARY.md

=== Commits exist ===
FOUND: c6baa8d (Task 1 RED)
FOUND: 31c0642 (Task 1 GREEN)
FOUND: 0905d43 (Task 2 RED)
FOUND: 445e2df (Task 2 GREEN)

=== Acceptance-Greps gruen ===
- rg 'fn build_inputs_repayment_letter\b' genossi_service_impl/src/pdf_generation.rs: 1 (>=1 ✓)
- rg 'fn build_inputs_repayment_letters_bundle' genossi_service_impl/src/pdf_generation.rs: 1 (>=1 ✓)
- rg 'recipients' genossi_service_impl/src/pdf_generation.rs: 34 (>=1 ✓)
- rg 'bank_account' genossi_service_impl/src/pdf_generation.rs: 21 (>=2 ✓)
- rg 'pub fn render_repayment_letter\b' genossi_service_impl/src/pdf_generation.rs: 1 (==1 ✓)
- rg 'pub fn render_repayment_letter_bundle' genossi_service_impl/src/pdf_generation.rs: 1 (==1 ✓)
- rg 'typst::compile' genossi_service_impl/src/pdf_generation.rs: 6 (>=3 ✓, existing 4 + neue 2)
- rg 'fn test_build_inputs_repayment_letter' genossi_service_impl/src/pdf_generation.rs: 5 (>=3 ✓)
- rg 'fn test_build_inputs_bundle' genossi_service_impl/src/pdf_generation.rs: 4 (>=2 ✓)
- cargo test -p genossi_service_impl --lib pdf_generation: 29 passed, 0 failed, 1 ignored

=== Scope-Gate (KRITISCH) ===
- git diff --name-only c6baa8d^ 445e2df (Plan-13-03-Commits): nur genossi_service_impl/src/pdf_generation.rs ✓
- KEIN Touch an templates/defaults/*.typ ✓ (Plan-13-01-Output unveraendert)
- KEIN Touch an template_storage.rs ✓ (Plan-13-01-Output unveraendert)
- KEIN Touch an lib.rs ✓ (keine Merge-Konflikt-Gefahr mit Plan-13-02)

=== TDD Gate Compliance ===
- Task 1: RED commit c6baa8d (test) → GREEN commit 31c0642 (feat) ✓
- Task 2: RED commit 0905d43 (test) → GREEN commit 445e2df (feat) ✓
- Keine REFACTOR-Commits noetig (Implementation bereits minimal-clean)
```

**Self-Check: PASSED**

## Next Plan Readiness

Plan 13-04 (RepaymentLetter-Service-Impl) kann jetzt:
- `PdfGenerator::render_repayment_letter(template_path, template_base, phase, member, ctx)` aufrufen — synchron, nach Tx-Commit (RESEARCH Pitfall #2).
- `PdfGenerator::render_repayment_letter_bundle(template_path, template_base, phase, &recipients)` mit sortiertem `Vec<(MemberEntity, RepaymentContext)>` aufrufen — eine Compile, alle N Briefe inkl. `#pagebreak()` zwischen ihnen.
- Service muss beim Aufruf von `render_repayment_letter_bundle` keine zusaetzlichen Inputs vorbereiten — `build_inputs_repayment_letters_bundle` macht den Plan-13-01-Bundle-Template-Compat selbst.

Plan 13-07 (E2E-Tests) braucht ggf. zusaetzliches Logo-Asset-Provisioning (siehe deferred-item 3).

**Keine Blocker fuer Folge-Plans.**

---
*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-01*
