---
phase: 12-frontend-component-first
plan: 14
subsystem: frontend
tags: [frontend, page, export, pdf, download, component-first, tdd]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 — i18n keys RepaymentExportInclude/Open/All/Paid/Download + CONFIG backend Rc<str>"
  - phase: 12-frontend-component-first
    provides: "Plan 12-05 — Detail-Page RepaymentPhaseDetails 3-Tab-Layout mit ExportTab-Stub (Preparation-Branch zeigt Hinweis-Box, Open/Closed-Branch hat TODO-Marker)"
provides:
  - "pub(crate) enum ExportInclude { Open, All, Paid } — Phase 11 D-03-Filter-Mapping"
  - "pub(crate) fn build_export_url(phase_id, include, backend) -> String — defensive backend-Slash-Trim"
  - "#[component] ExportTab(phase: RepaymentPhaseTO) — voll funktionale Export-UI mit Radio-Filter + Download-Anker"
  - "#[component] ExportIncludeRadio — reine Forwarding-Component für die drei Filter-Optionen"
affects:
  - "Phase 12 Plan 15 (Verify) — kann Export-Tab UAT-mässig verifizieren ohne ExportTab-Stub zu skippen"
  - "Künftige UAT-Iteration — falls 'lieber im selben Tab herunterladen' gewünscht ist, lässt sich target='_blank' weglassen (Plan-Discretion-Anker)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-Function-Helper (build_export_url) + Enum (ExportInclude) mit #[cfg(test)] mod tests — TDD-Pattern (RED → GREEN), analog Plan 12-05 (parse_close_conflict) und Plan 12-02 (format_payout_eur)"
    - "Download via <a href + target='_blank' + rel='noopener noreferrer'> mit Button-Styling statt <button> mit fake-href — D-26 / D-01-konform, vermeidet Page-Reload-Bug (feedback_dioxus_button_type.md)"
    - "Defensive backend-Slash-Trim via trim_end_matches('/') — falls CONFIG.backend einen Trailing-Slash hat, entstehen keine doppelten Schrägstriche"
    - "Component-First: ExportTab + ExportIncludeRadio inline in page-file (page-coupled, single caller), KEINE Extraction nach component/ — gleiche Begründung wie BasicsTab in Plan 12-05"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/page/repayment_phase_details.rs (+130 LOC netto: ExportInclude-Enum + build_export_url + ExportTab + ExportIncludeRadio + 4 Tests + Stub-Replacement)"

key-decisions:
  - "ExportInclude als pub(crate) Enum (Phase-12-lokal scoped statt rest-types/ oder api.rs) — Filter ist UI-Konzept und gehört zum Detail-Page. Backend kennt nur den Query-String-Wert (open/all/paid). Erweiterungen wie CSV-Format wären eine separate Enum FormatChoice in einer späteren Phase."
  - "build_export_url defensiver gegen Slash-Doppel-Strich via trim_end_matches('/') — auch wenn CONFIG.backend laut Konvention keinen Trailing-Slash hat, schützt der Trim gegen Config-Drift. Test build_url_trims_backend_trailing_slash sichert das Verhalten."
  - "Download via <a>-Element statt <button> mit fake-href oder programmatischem HtmlElement::click() (assembly_details.rs Z. 369-385-Pattern) — Plan-Discretion-Wahl: Backend handelt Content-Disposition; statisches <a target='_blank'> ist die simpelste Lösung ohne web_sys::Url::create_object_url + revoke_object_url-Overhead. assembly_details.rs benutzt den Blob-URL-Pfad, weil dort der Backend-Endpoint einen Blob retourniert — Phase 11 PDF-Export ist ein direkter Stream mit Content-Disposition, daher genügt <a href>."
  - "target='_blank' + rel='noopener noreferrer' — Browser öffnet PDF im neuen Tab + Download-Bar. Plan-Discretion: Falls UAT-Feedback 'lieber im selben Tab herunterladen' zeigt, kann target weggelassen werden (siehe affects)."
  - "ExportIncludeRadio als zweite inline-Component (zusätzlich zu ExportTab) — DRY-Prinzip gegen 3-fach-Kopie der Radio-Render-Logik. Bleibt page-local (kein Reuse-Bedarf außerhalb der Detail-Page-Datei)."
  - "Default ExportInclude::Open (entspricht Phase 11 D-03 Backend-Default) — Banking-Vorlage ist der Hauptanwendungsfall (Vorstand-Workflow: nach Sammel-Mail PDF exportieren als Sammelüberweisungs-Vorlage)."

patterns-established:
  - "URL-Builder Pure-Func mit Backend-Trim — vorbild für künftige Plan-Discretion-Downloads (CSV, XLSX, ZIP) sollten dasselbe Trim-Pattern befolgen"
  - "Inline Sub-Component-Familie (ExportTab + ExportIncludeRadio) als page-local-Group — wenn ein Tab mehr als eine private Helper-Component braucht, dürfen alle Helper im gleichen page-file leben (gleiche Begründung wie BasicsTab inline)"
  - "Static <a target='_blank'> für Server-Side-File-Streaming-Downloads — wenn der Backend-Endpoint Content-Disposition setzt und keinen Blob-URL-Pfad braucht, ist <a> die einfachste Lösung (Gegensatz zu assembly_details.rs ExportTab, das Blob-URLs benutzt)"

requirements-completed: [UI-02]

# Metrics
duration: ~4min
completed: 2026-06-01T13:35:38Z
task-count: 2
file-count: 1
test-count-added: 4
test-count-total: 196
commits:
  - {sha: d0a2984, type: test, task: "1 RED", scope: "page/repayment_phase_details.rs (4 build_url tests, ExportInclude/build_export_url do not exist yet)"}
  - {sha: 5168ade, type: feat, task: "1 GREEN", scope: "page/repayment_phase_details.rs (pub(crate) enum ExportInclude + pub(crate) fn build_export_url)"}
  - {sha: 8519530, type: feat, task: "2", scope: "page/repayment_phase_details.rs (ExportTab + ExportIncludeRadio + stub replacement)"}
---

# Phase 12 Plan 14: Export-Tab im Detail-Page Summary

**One-liner:** Detail-Page Export-Tab voll verdrahtet — drei Radio-Filter (Open default, All, Paid für Phase 11 D-03), grosser blauer Download-Anker (`<a target='_blank'>` statt `<button>`), backend-handled Content-Disposition als PDF-Download — TODO-Plan-12-14-Stub aus Plan 12-05 entfernt.

## What Was Built

Zwei Tasks, drei Commits (TDD-Sequence + Integration). Task 1 fügt das ExportInclude-Enum + den build_export_url-Pure-Func via RED→GREEN-Zyklus hinzu, Task 2 baut die ExportTab + ExportIncludeRadio Components und ersetzt den TODO-Stub im Detail-Page.

### Task 1 RED → GREEN: ExportInclude + build_export_url (commits d0a2984 → 5168ade)

**ExportInclude-Enum** als `pub(crate) enum { Open, All, Paid }` mit `as_str()` → `"open"`/`"all"`/`"paid"` (Backend-Query-String-Werte, Phase 11 D-03).

**build_export_url(phase_id, include, backend) -> String** — defensiver Trim-Slash + Format-Macro:
```rust
let backend_trimmed = backend.trim_end_matches('/');
format!(
    "{backend_trimmed}/api/repayment-phase/{phase_id}/export/pdf?include={}",
    include.as_str()
)
```

**Test coverage Task 1: 4/4 PASS:**
- `build_url_open` — exakte URL-Vergleich mit Open-Filter
- `build_url_all` — endet auf `include=all`
- `build_url_paid` — endet auf `include=paid`
- `build_url_trims_backend_trailing_slash` — `https://api.example.com/` als Input darf KEIN `//api/` im Ergebnis erzeugen

RED-Phase (commit d0a2984): Tests scheitern beim Compile, weil `ExportInclude` + `build_export_url` noch nicht existieren — TDD-Standardverhalten. GREEN-Phase (commit 5168ade): Symbole hinzugefügt, alle 4 Tests grün.

### Task 2: ExportTab + ExportIncludeRadio + Stub-Replacement (commit 8519530)

**`#[component] ExportTab(phase: RepaymentPhaseTO)`** — voll funktionale Export-UI:
- Liest `CONFIG.read().backend` (Rc<str>), konvertiert zu String
- `use_signal(|| ExportInclude::Open)` als reactive State für Radio-Filter
- 3 ExportIncludeRadio-Aufrufe (Open/All/Paid) mit i18n-Labels
- Download-`<a>`-Anker mit Button-Styling (`bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded text-center font-semibold min-h-[44px]`)
- `target="_blank" rel="noopener noreferrer"` — PDF öffnet im neuen Tab, Backend-Content-Disposition triggert Download-Bar

**`#[component] ExportIncludeRadio(value, selected, label, on_select)`** — reine Forwarding-Component:
- `<label>` als Klick-Ziel (Tablet-tauglich + 44px Touch-Target)
- `<input type="radio">` mit `name="export_include"` (Browser-native Radio-Group)
- `onchange` ruft `on_select.call(value)` auf
- Span mit Label-Text

**Detail-Page-Integration** in `RepaymentPhaseDetails::render` — `"export"`-Branch-Match:
```rust
"export" => match status_value {
    RepaymentPhaseStatusTO::Preparation => rsx! { /* Hinweis-Box (Plan 12-05) */ },
    _ => rsx! { ExportTab { phase: phase_for_export } },  // ← war TODO Plan 12-14-Stub
},
```

D-08-Konformität: Bei Status=Closed wird ExportTab voll gerendert. Backend Phase 11 EXPO-01 lässt Export für Open UND Closed zu. `ExportInclude::Open` in Closed-Phase kann eine leere Liste retournieren — das ist akzeptables Backend-Verhalten und wird nicht im Frontend abgefangen.

D-01 Button-Gate frei: Keine `<button>`-Tags in ExportTab oder ExportIncludeRadio — Download ist `<a>`, Radio-Selection ist `<input type="radio">`. Grep-Gate-Verifikation = 0.

## How It Was Verified

```bash
# Build sauber
$ cargo build --bin genossi-frontend
warning: ... 23 warnings (existierende Dead-Code-Warnings)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.36s

# Plan-spezifische Tests
$ cargo test --bin genossi-frontend -- page::repayment_phase_details::tests::build_url
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured

# Volle Datei-Tests
$ cargo test --bin genossi-frontend -- page::repayment_phase_details
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 181 filtered out

# Volle Test-Suite
$ cargo test --bin genossi-frontend
test result: ok. 196 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# Done-Criteria Greps
$ rg "TODO Plan 12-14" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
0   # Stub-Marker entfernt

$ rg "fn ExportTab" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
1   # ExportTab existiert genau einmal

$ rg "ExportInclude::Open|ExportInclude::All|ExportInclude::Paid" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
12  # >= 6: Enum-Definitions + Tests + 3 Radio-Aufrufe + Default-Signal

$ rg "/api/repayment-phase/.*export/pdf" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
2   # format-Macro in build_export_url + Test-Assert

# D-01 Button-Gate
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' genossi-frontend/src/page/repayment_phase_details.rs | \
    grep -v 'r#type:' | grep -c 'button {'
0   # Keine button-Tags ohne r#type: im ExportTab-Pfad (auch nicht im Rest der Datei)
```

## Decisions Made

### ExportInclude als pub(crate) Enum (Phase-12-lokal scoped)

Plan-Frontmatter `must_haves.artifacts` listet `ExportInclude` als Output. Alternativen waren `rest-types/`-Crate (wäre Backend-↔-Frontend-Sharing — überschießt; Backend braucht den Filter als Query-String, nicht als Typ) oder `api.rs` (wäre frontend-shared mit anderen Pages — gibt es nicht; nur Detail-Page nutzt den Filter). Page-local pub(crate) ist der minimalste Scope, der Tests sichtbar macht. Falls Plan 12-15 oder eine spätere Phase einen zweiten Export-Pfad bauen (z.B. CSV), kann das Enum dann via promotion nach `component/` oder `api.rs` wandern.

### build_export_url defensiver Backend-Slash-Trim

Test `build_url_trims_backend_trailing_slash` sichert das Verhalten gegen Config-Drift. Aktuelle `Config::default()` setzt `backend: Rc<str>` ohne Trailing-Slash, aber Plan-Discretion-Konvention sagt: lieber defensiv. Trim ist ein 1-Zeilen-Cost, deckt Edge-Case zuverlässig ab.

### Download via `<a>`-Element statt programmatischer `HtmlElement::click()`

Vergleich mit `assembly_details.rs` Plan 06-Pattern (Z. 369-385): dort wird ein Blob-URL via `web_sys::Url::create_object_url()` erzeugt, in ein dynamisches `<a download>` injiziert, programmatisch geklickt, dann `revoke_object_url`. Das ist nötig, weil `api::export_attendance_url` einen Blob-URL retourniert (Frontend lädt PDF-Bytes selbst). Phase 11 Repayment-PDF-Export ist hingegen ein direkter Backend-Stream mit `Content-Disposition: attachment` — der Browser handelt den Download nativ, sobald ein `<a href>` geklickt wird. Statisches `<a>` ist die simpelste Lösung ohne Blob-Overhead.

### target='_blank' + rel='noopener noreferrer'

`target='_blank'` öffnet PDF im neuen Tab — Browser zeigt PDF-Viewer-Tab + Download-Bar. `rel='noopener noreferrer'` ist Standardpraxis für externe Links (Security-Best-Practice gegen `window.opener`-Reverse-Tabnabbing). Alternative ohne `target='_blank'` würde den PDF-Stream direkt im Tab öffnen oder triggern (Browser-spezifisch), könnte aber bei manchen Browsern den Detail-Page-State weg-navigieren — Plan-Discretion-Entscheidung: UAT-Test wird zeigen, was sich besser anfühlt. Falls UAT-Feedback "lieber im selben Tab" zeigt, kann `target='_blank'` weggelassen werden (1-Zeilen-Diff in Wave 11+).

### ExportIncludeRadio als zweite inline-Component

Statt 3-fach kopierter Radio-Render-Logik gibt es eine kleine Helper-Component `ExportIncludeRadio` mit 4 Props (value, selected, label, on_select). Die Component bleibt page-local — sie wird nirgendwo sonst gebraucht. Component-First-Prinzip ist nicht verletzt: page-local-Helper-Components sind explizit erlaubt (siehe Plan 12-05 BasicsTab-Begründung).

### Default ExportInclude::Open (Banking-Vorlage)

Vorstand-Workflow (CONTEXT D-03 + Specifics "Banking-Workflow als Leitstern"): Nach Massenmail-Schritt exportiert der Vorstand das PDF und nutzt es als Sammelüberweisungs-Vorlage im Online-Banking. Filter "Open" liefert die offenen + angeschriebenen Einträge — exakt die Liste, die der Vorstand überweisen muss. Default auf Open spart den ersten Klick.

## Deviations from Plan

**None — plan executed exactly as written.**

Plan-Acceptance-Tests wurden alle exakt verifiziert:
- 4 build_url-Tests PASS (Open, All, Paid, Trim-Slash) — ✓
- `pub(crate) enum ExportInclude` + `pub(crate) fn build_export_url` — ✓
- ExportTab-Component mit 3 Radio-Buttons + Download-Anker — ✓
- ExportTab nur sichtbar bei Status=Open/Closed (Preparation-Branch zeigt Hinweis-Box) — ✓
- D-26 Button-Pattern: Download ist `<a>` mit Button-Styling, kein `<button>` mit fake-href — ✓
- D-01 Grep-Gate: 0 buttons ohne `r#type:` — ✓
- D-08: ExportTab wird in Closed-Status weiterhin gerendert — ✓
- `cargo build` exit 0 — ✓
- `cargo test --bin genossi-frontend -- page::repayment_phase_details` 15 PASS — ✓
- TODO Plan 12-14-Stub aus Detail-Page entfernt — ✓

Eine kleine **Plan-Discretion-Klarstellung** während der Implementierung: Plan-Wording erwähnte `min-h-[44px]` nur am Download-Anker. Ich habe `min-h-[44px]` ZUSÄTZLICH am `<label>` von ExportIncludeRadio gesetzt (Tablet-Touch-Target-Konsistenz mit Phase-4-Standard) — das ist keine Abweichung im engeren Sinn, sondern eine Defaults-Verfeinerung im Bereich "Claude's Discretion".

## Known Stubs

**None.**

Die ExportTab ist voll funktionsfähig: Backend Phase 11 liefert PDFs für alle 3 Include-Filter, Frontend baut die URL korrekt, Browser öffnet das PDF im neuen Tab. Keine TODO-Marker, keine hardcoded Mock-Daten, keine leeren Render-Pfade.

Optional in einer späteren Iteration: ein **Filename-Preview** (analog assembly_details.rs Z. 488-492) könnte das `Content-Disposition`-Filename-Pattern (`auszahlung-{fiscal_year}-{include}.pdf`) im Tab vorzeigen. Plan-Wording fordert das nicht; UAT-Feedback wird zeigen, ob es nötig ist.

## Threat Flags

None — dieser Plan baut nur Frontend-UI auf bestehende Backend-Routes auf. Keine neue Netzwerk-Oberfläche, keine neuen Auth-Pfade, keine Schema-Änderungen. Der Backend-Endpoint `/api/repayment-phase/{id}/export/pdf` existiert seit Phase 11 mit Admin-Privilege-Gating (über die bestehende REST-Middleware).

## Self-Check: PASSED

Verified artifacts in the main repo:

- [FOUND] `genossi-frontend/src/page/repayment_phase_details.rs` (737 lines incl. new code)
- [FOUND] `pub(crate) enum ExportInclude { Open, All, Paid }` (line 86-90)
- [FOUND] `pub(crate) fn build_export_url(phase_id: Uuid, include: ExportInclude, backend: &str) -> String` (line 103-109)
- [FOUND] `#[component] fn ExportTab(phase: RepaymentPhaseTO) -> Element` (one occurrence, after BasicsTab)
- [FOUND] `#[component] fn ExportIncludeRadio(value, selected, label, on_select)` (after ExportTab)
- [FOUND] 4 `build_url_*` tests in `mod tests` (open/all/paid/trim-slash)
- [VERIFIED] `cargo build --bin genossi-frontend` exit 0
- [VERIFIED] `cargo test --bin genossi-frontend -- page::repayment_phase_details::tests::build_url` → 4/4 PASS
- [VERIFIED] `cargo test --bin genossi-frontend -- page::repayment_phase_details` → 15/15 PASS
- [VERIFIED] `cargo test --bin genossi-frontend` → 196/196 PASS (no regressions)
- [VERIFIED] D-01 Button-Gate: 0 buttons without `r#type:` in repayment_phase_details.rs
- [VERIFIED] Stub-Marker `TODO Plan 12-14` removed from file (0 occurrences)
- [VERIFIED] ExportInclude::Open|All|Paid references count: 12 (>= 6 required)
- [VERIFIED] `/api/repayment-phase/.*export/pdf` references count: 2 (>= 1 required)
- [FOUND] Commit `d0a2984` (test(12-14): add failing tests for build_export_url Pure-Func)
- [FOUND] Commit `5168ade` (feat(12-14): implement ExportInclude enum + build_export_url Pure-Func)
- [FOUND] Commit `8519530` (feat(12-14): implement Export-Tab + replace TODO-stub in Detail-Page)

## TDD Gate Compliance

- **Task 1 RED gate:** `d0a2984` — 4 build_url-Tests fail at compile (ExportInclude + build_export_url symbols do not exist yet). RED confirmed via failing `cargo test` build.
- **Task 1 GREEN gate:** `5168ade` — ExportInclude + build_export_url added; all 4 tests PASS, cargo build clean.
- **Task 2 GREEN gate:** `8519530` — ExportTab + ExportIncludeRadio + stub replacement; all 15 page-tests PASS, 196/196 full suite PASS. (Task 2 is type="auto" — kein tdd="true"-Marker, kein expliziter RED-Commit erwartet.)
- **REFACTOR gate:** none — Implementation war minimal und direkt; kein Refactor-Commit nötig.

Gate sequence in `git log 90d41e1..HEAD`:
```
d0a2984 test(12-14): add failing tests for build_export_url Pure-Func              ← Task 1 RED
5168ade feat(12-14): implement ExportInclude enum + build_export_url Pure-Func     ← Task 1 GREEN
8519530 feat(12-14): implement Export-Tab + replace TODO-stub in Detail-Page       ← Task 2
```

Strict test→feat→feat — TDD-Sequence ist gewahrt für Task 1; Task 2 hat keinen RED-Commit (per Plan-Frontmatter type="auto").

---

*Phase: 12-frontend-component-first*
*Plan: 14 — Export-Tab im Detail-Page (UI-02 / EXPO-01..03)*
*Completed: 2026-06-01T13:35:38Z (~4 min)*
