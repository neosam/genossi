---
phase: 26-editor-formatierung-vervollstaendigen
plan: 02
subsystem: frontend-editor-invariants
tags:
  - test-only
  - grep-gate
  - source-invariant
  - frontend
  - include_str
  - self-reference-hazard-fix
dependency_graph:
  requires:
    - Phase 24 WysiwygEditor invariants (styleWithCSS-false guard + onpaste prevent_default)
  provides:
    - Mechanical regression detection for the two ammonia-gate source invariants
    - Meta-test protecting the grep-gate defence pattern itself
  affects:
    - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs (added mod grep_gate_tests)
tech_stack:
  added:
    - include_str! + runtime-assembled needle strings (self-reference-hazard-safe pattern)
  patterns:
    - "Grep-Gate v2: production_region slicing + format!-assembled needles"
key_files:
  created: []
  modified:
    - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
decisions:
  - "D-DEV-1: RESEARCH pattern was self-matching (false-positive gate) — replaced with two-defence variant (region-slice + runtime-assembled needles)"
  - "D-DEV-2: Added third meta-test production_region_excludes_test_module to guard the defence pattern itself against regression"
metrics:
  duration_minutes: 20
  tasks_completed: 1
  files_modified: 1
  tests_added: 3
  baseline_tests: 290
  final_tests: 293
  completed_date: "2026-07-17"
requirements:
  - EDIT-09
status: complete
---

# Phase 26 Plan 02: Grep-Gate für styleWithCSS + Paste-plain-Invarianten Summary

Mechanischer Rust-`include_str!`-Grep-Gate schützt die zwei Phase-24-Source-Invarianten des WYSIWYG-Editors (`styleWithCSS=false`-Guard und `onpaste`→`prevent_default()`) via 3 neue Tests in `wysiwyg_editor.rs::mod grep_gate_tests`; Baseline 290 Tests → 293, alle grün, manueller Negativ-Beweis für beide Invarianten bestätigt.

## What Was Built

Neuer `#[cfg(test)] mod grep_gate_tests` in `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` (ab Zeile 226; separater Block hinter dem bestehenden `mod tests` mit `plain_to_html`-Tests, kein Konflikt). Der Block enthält:

| # | Test | Lokation | Guard |
|---|------|----------|-------|
| 1 | `style_with_css_false_guard_present` | Zeile ~274 | Asserts `exec_command_bool(&doc, "styleWithCSS", false)` existiert in der Production-Region der Datei |
| 2 | `paste_handler_calls_prevent_default_before_read` | Zeile ~291 | Asserts `evt.prevent_default()` erscheint innerhalb von 400 Zeichen nach `onpaste:` im Production-Region |
| 3 | `production_region_excludes_test_module` | Zeile ~310 | Meta-Test: beweist dass `production_region()` das Test-Modul aus dem Search-Range ausschließt (Self-Reference-Hazard-Schutz) |

Ausführung: `cargo test grep_gate_tests` — beide Domain-Tests grün, Meta-Test grün. Kein WASM-Target nötig — `include_str!` + `str::contains/find` + `format!` sind pure-Rust ohne wasm-bindgen-Dependency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Self-Reference-Hazard im RESEARCH-Pattern**

- **Found during:** Task 1 (manueller Negativ-Beweis nach initialer Implementation)
- **Issue:** Der RESEARCH.md-Pattern-3-Code (Zeilen 324-361) implementiert die Grep-Assertions direkt mit `EDITOR_SRC.contains(r#"exec_command_bool(&doc, "styleWithCSS", false)"#)`. Weil `EDITOR_SRC = include_str!("wysiwyg_editor.rs")` genau die Datei einliest in der der Test steht, wird die Literal-Byte-Sequenz aus der `assert!`-Zeile SELBST Teil von `EDITOR_SRC`. Ergebnis: Der Test ist immer grün, auch wenn die Production-Zeile 77 komplett entfernt wird — reproduziert und bestätigt via 1. Iteration (`cargo test style_with_css_false_guard_present` blieb grün nach Auskommentieren von Zeile 77).
- **Fix:** Zwei-Verteidigungs-Muster eingeführt:
  1. **Region-Slicing:** Neue Helper-Funktion `production_region()` schneidet `EDITOR_SRC` am ersten Vorkommen von `"mod grep_gate_tests"` ab; die Tests suchen nur in Bytes davor. Die Marker-String (`TEST_MODULE_MARKER`) ist harmlos, weil sie kein Teil der Guard-Substrings ist und selbst außerhalb des Search-Range wieder auftaucht.
  2. **Runtime-Assembled Needles:** Die zu-suchenden Strings werden via `format!("exec_command_bool(&doc, {q}styleWithCSS{q}, false)", q = "\"")` und `format!("onpast{tail}", tail = "e:")` / `format!("evt.prevent_defaul{tail}", tail = "t()")` zur Laufzeit zusammengesetzt. Als Konsequenz existiert keine einzelne Byte-Sequenz im Test-Source, die dem Match-Target entsprechen würde — selbst wenn (a) versagt.
- **Meta-Test added:** `production_region_excludes_test_module` prüft (a) dass `production_region()` den Marker nicht mehr enthält (sonst Slice-Bug) und (b) dass die Slice-Länge < Gesamt-Länge (sonst Marker wurde per `find()` nicht gefunden — was ohnehin schon paniked). Damit ist die Verteidigung selbst gegen zukünftiges Refactoring (z. B. Umbenennung des Test-Moduls) geschützt.
- **Files modified:** `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`
- **Commit:** `41a3714b` (change-id `snrmvytn`)

### Auth Gates

Keine.

## Manueller Negativ-Beweis

Der Grep-Gate wurde experimentell auf beide Invarianten geprüft (nicht committet; nur zur Validierung dass die Tests nicht false-positive grün sind):

### Test 1: `style_with_css_false_guard_present`

- **Aktion:** Zeile 77 (`let _ = crate::js::exec_command_bool(&doc, "styleWithCSS", false);`) durch Kommentar-Zeile ersetzt (`// NEGATIVE-PROOF: line removed for gate test`).
- **Erwartung:** Test schlägt fehl mit selbst-erklärender Diagnose.
- **Beobachtung:** ✓ FAILED — Fehlermeldung (aus cargo-test-Output):
  > `Grep gate FAILED: expected literal call exec_command_bool(&doc, "styleWithCSS", false) in wysiwyg_editor.rs (production region, before the test module). This guard is Pitfall 1 of 24-RESEARCH.md — removing it means Bold emits <span style=…> instead of <b>, which ammonia strips silently.`
- **Restore:** Zeile 77 wiederhergestellt → Test wieder grün.

### Test 2: `paste_handler_calls_prevent_default_before_read`

- **Aktion:** Zeile 89 (`evt.prevent_default();`) durch Kommentar-Zeile ersetzt.
- **Erwartung:** Test schlägt fehl und zeigt das 400-Zeichen-Fenster als Diagnose.
- **Beobachtung:** ✓ FAILED — Fehlermeldung enthielt das erwartete Diagnose-Fenster mit den umgebenden `onpaste`-Closure-Zeilen und dem Ersatz-Kommentar `// NEGATIVE-PROOF: line removed for gate test`, dazu die selbst-erklärende Message über Pitfall 3.
- **Restore:** Zeile 89 wiederhergestellt → Test wieder grün.

Beide Negativ-Beweise bestätigen: die Grep-Gate greift auf echte Source-Regressionen und liefert präzise Diagnose-Messages, die einem Refactorer die korrekte Fix-Location zeigen.

## Verification Results

```
cargo fmt -- --check                                            → clean
cargo test                                                      → 293 passed; 0 failed (+3 vs. baseline 290)
cargo test grep_gate_tests                                      → 3 passed
cargo test style_with_css_false_guard_present                   → 1 passed
cargo test paste_handler_calls_prevent_default_before_read      → 1 passed
cargo test production_region_excludes_test_module               → 1 passed
```

Bestehendes `mod tests` mit `plain_to_html`-Tests (6 Tests) unverändert grün — keine Kollision.

## Threat Flags

Keine neuen Threat-Flags. Der Gate ist reine Regressions-Detektion für die T-26-02-Mitigation aus dem Plan-Threat-Model — kein neuer Angriffs-Vektor.

## Commit

- **Change-ID (jj):** `snrmvytn`
- **Commit-Hash (git-side):** `41a3714b41cc`
- **Message:** `test(26): grep-gate for styleWithCSS + paste-plain invariants (26-02)`
- **Scope:** Nur `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` (119 insertions, 6 deletions — die Deletions sind der `cargo fmt`-Reflow der bestehenden `WysiwygEditor`-Signatur und der `let-else`-Blöcke in `sync_from_dom`, ausgelöst durch `cargo fmt` nach dem Anhängen des neuen Moduls; kein semantischer Code-Change am bestehenden Bestand).

## Self-Check: PASSED

- `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` — FOUND (117 Zeilen `mod grep_gate_tests`, 293 Tests im Package)
- Commit `41a3714b` — FOUND via `jj log -r @-`
- Baseline-Test-Count 290 → Neu 293 (Erwartet: 292 laut Plan Success-Criterion, geliefert: 293 wegen des zusätzlichen Meta-Tests aus der Deviation-Fix — dokumentiert, kein Regression)
