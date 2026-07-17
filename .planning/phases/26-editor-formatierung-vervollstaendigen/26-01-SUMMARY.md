---
phase: 26-editor-formatierung-vervollstaendigen
plan: 01
subsystem: mail-sanitize
status: complete
tags:
  - test-only
  - sanitize
  - ammonia
  - round-trip
  - e2e
  - wysiwyg
requirements:
  - EDIT-06
  - EDIT-07
  - EDIT-08
dependency-graph:
  requires:
    - Phase 23 Plan 04 ammonia sanitize gate (genossi_mail::sanitize_html)
    - Phase 24 Plan 04 WYSIWYG toolbar (baseline e2e vorbild create_template_body_html_sanitized)
  provides:
    - Regression fence: UL/OL/H1/H2/H3 survive ammonia default and full REST round-trip
  affects: []
tech-stack:
  added: []
  patterns:
    - "Ammonia round-trip unit tests (substring-token assertions, no byte-exact compare)"
    - "E2E POST→sanitize→SQLite→GET round-trip via /api/mail/templates for tag preservation"
key-files:
  created: []
  modified:
    - genossi_mail/src/sanitize.rs (mod tests +3 fns; lines 110-166)
    - genossi_bin/tests/e2e_tests.rs (+1 fn; lines 14844-14913)
decisions:
  - "D-04 (Backward-Compat via Nicht-Änderung) mechanically satisfied — production code (sanitize.rs lines 1-37: //! doc + pub fn sanitize_html) is byte-identical to pre-Phase-26 state."
  - "H1 explicitly covered per D-01 (H1 toolbar button stays in wysiwyg_toolbar.rs)."
  - "Substring token assertions (not byte-exact) per RESEARCH Pitfall 4 — ammonia may normalise whitespace."
  - "D-04 Ausstiegs-Klausel NOT triggered: ammonia default preserved all tested tags on first run — no sanitize.rs config change needed."
metrics:
  duration: "~2 min (execution + tests + commit)"
  completed: "2026-07-17"
  tests-added: 4
  loc-added: ~70
---

# Phase 26 Plan 01: Editor-Round-Trip-Tests für Listen und Überschriften — Summary

Test-only phase: bewiesen via 3 Unit-Tests in `genossi_mail::sanitize` und 1 E2E-Round-Trip-Test in `genossi_bin/tests/e2e_tests.rs`, dass die bestehende ammonia-Default-Sanitize-Grenze (Phase 23) die Toolbar-Elemente `<ul><li>`, `<ol><li>` und `<h1><h2><h3>` unverändert durchreicht — sowohl isoliert als auch im vollen HTTP-Pfad POST → sanitize → SQLite → GET über `/api/mail/templates`.

## What Was Built

### Unit tests in `genossi_mail/src/sanitize.rs`

| Test | Line | Coverage |
|------|------|----------|
| `sanitize::tests::sanitize_preserves_unordered_list` | 115 | EDIT-06 |
| `sanitize::tests::sanitize_preserves_ordered_list` | 136 | EDIT-07 |
| `sanitize::tests::sanitize_preserves_headings_h1_h2_h3` | 158 | EDIT-08 (H1/H2/H3) |

Each test uses substring-token assertions with `{output}` format-capture, matching the pattern of `sanitize_preserves_jinja_placeholder_in_text_content`.

### E2E test in `genossi_bin/tests/e2e_tests.rs`

| Test | Line | Coverage |
|------|------|----------|
| `create_template_body_html_lists_and_headings_round_trip` | 14851 | EDIT-06/07/08 full HTTP round-trip |

Placed directly after `create_template_body_html_sanitized` (line 14797) and before the `// ── Phase 24 Plan 04 …` section marker, per D-03. POST payload contains H1/H2/H3 + UL/LI + OL/LI + text fragments; GET response is asserted to contain each of 19 tokens.

## Test Results

| Command | Result |
|---------|--------|
| `cargo test -p genossi_mail --lib sanitize_preserves` | 4 passed, 0 failed (3 new + 1 pre-existing jinja test) |
| `cargo test -p genossi_mail --lib` | **255 passed, 0 failed** (baseline 252 → +3) |
| `cargo test -p genossi_bin --test e2e_tests create_template_body_html_lists_and_headings_round_trip -- --exact` | 1 passed, 0 failed |
| `cargo test -p genossi_bin --test e2e_tests` | **309 passed, 1 failed** (baseline 308+1 → 309+1; delta: +1 green, unchanged failure count — the 1 failure is the Phase-22 pre-existing `test_mail_preview_repayment_no_entries_does_not_default_to_one`, documented in STATE.md) |
| `rustfmt --check` on both modified files | exit 0 (clean) |

## D-04 Backward-Compat Beweis

`jj diff genossi_mail/src/sanitize.rs` zeigt Änderungen ausschließlich innerhalb `#[cfg(test)] mod tests { ... }` (Zeilen 109 f.). Zeilen 1-37 (`//!`-Modul-Doku + `pub fn sanitize_html`) sind byte-identisch zum Vor-Zustand. Success-Criterion #5 (bestehende v1.4-Templates rendern byte-identisch) ist damit als mechanische Konsequenz erfüllt.

**Ausstiegs-Klausel (D-04) NICHT getriggert:** Ammonia-Default hat wie erwartet alle 6 Heading-Tokens (H1/H2/H3 mit schließenden Tags), UL/OL-Wrapper und LI-Items in einem Zug durchgelassen — kein Snapshot-Fallback und keine Ammonia-Config-Anpassung nötig.

## Deviations from Plan

None — plan executed exactly as written. Keine Rule-1/2/3-Fixes nötig; keine Rule-4-Checkpoints.

**Note:** `cargo fmt -p genossi_mail -- --check` und `cargo fmt -p genossi_bin -- --check` zeigen pre-existing Drift in mehreren anderen Dateien (`backfill.rs`, `digest.rs`, `inbox.rs`, `render.rs`, `send.rs`, `template.rs`, `worker.rs`, `inbox_rest.rs`). Diese Drift ist NICHT durch Plan 26-01 verursacht — die beiden modifizierten Dateien (`sanitize.rs`, `e2e_tests.rs`) sind isoliert per `rustfmt --check --edition 2021` verifiziert fmt-clean (exit 0). Per Scope-Boundary (Rule 4 Scope-Guard) nicht mit-gefixt.

## Commit

- **jj change-id:** `nxqsxvprkyvr`
- **jj commit-hash:** `0bb85962 (jj change: nxqsxvpr)`
- **Message:** `test(26): round-trip tests for ul/ol/h1-h3 in sanitize + e2e (26-01)`
- **Files in commit** (exactly 3): `genossi_mail/src/sanitize.rs`, `genossi_bin/tests/e2e_tests.rs`, `.planning/phases/26-editor-formatierung-vervollstaendigen/26-01-SUMMARY.md`

Verify: `jj log -r 0bb85962 (jj change: nxqsxvpr) --summary`

## Self-Check: PASSED

- [x] `genossi_mail/src/sanitize.rs` contains `sanitize_preserves_unordered_list` (line 115), `sanitize_preserves_ordered_list` (line 136), `sanitize_preserves_headings_h1_h2_h3` (line 158) — verified via grep.
- [x] `genossi_bin/tests/e2e_tests.rs` contains `create_template_body_html_lists_and_headings_round_trip` (line 14851) — verified via grep.
- [x] `cargo test -p genossi_mail --lib` → 255 passed, 0 failed.
- [x] `cargo test -p genossi_bin --test e2e_tests create_template_body_html_lists_and_headings_round_trip -- --exact` → 1 passed.
- [x] `jj log -r @- --summary` shows commit `0bb85962 (jj change: nxqsxvpr)` with exactly 3 files (sanitize.rs, e2e_tests.rs, 26-01-SUMMARY.md).
- [x] `sanitize.rs` lines 1-37 (production code) unchanged relative to parent — D-04 backward-compat mechanically satisfied.
- [x] `rustfmt --check --edition 2021` on both modified files → exit 0.
