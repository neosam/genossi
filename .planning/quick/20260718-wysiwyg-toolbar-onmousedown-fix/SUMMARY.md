---
slug: wysiwyg-toolbar-onmousedown-fix
completed: 2026-07-18
status: complete
files_changed:
  - genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs
tests_added: 2
loc_delta: +75/-13
---

# SUMMARY: WYSIWYG-Toolbar Selection-Preserve

## Bug fixed

Alle 13 Toolbar-Buttons haben jetzt `onmousedown: |evt| evt.prevent_default()`. Der `mousedown` feuert vor dem Fokus-Wechsel — das verhindert dass das Contenteditable seinen Fokus verliert und die Selection Range weggeworfen wird. Block-Level-Commands (`formatBlock`, `insertUnorderedList`, `insertOrderedList`) finden jetzt die Range im Contenteditable und wirken auf den selektierten Text.

Inline-Commands (Bold, Italic, Underline, Strike) funktionierten schon vorher, weil `execCommand` diese als State-Flag toggelt.

## Test coverage

Neuer inline `mod grep_gate_tests` in `wysiwyg_toolbar.rs` (analog Phase 26 EDIT-09-Pattern):

- `every_button_has_onmousedown_prevent_default` — asserted `#button-Blöcke == #onmousedown-Handler >= 13`, und `prevent_default()` steht in jedem onmousedown-Handler innerhalb 80 chars.
- `production_region_excludes_test_module` — Meta-Test gegen den self-matching-Hazard.

Self-Reference-Hazard-Schutz: Needles via `format!` zur Laufzeit assembliert; Source-Region wird vor dem `mod grep_gate_tests`-Marker abgeschnitten.

## Verification

- `cargo test grep_gate_tests` → 5 passed (2 neue Toolbar + 3 aus Phase 26).
- `cargo fmt --check` → clean.
- `cargo check` → grün, 37 warnings (pre-existing, none new).
- **Negativbeweis:** Bold-Button-`onmousedown` sed-entfernt → Test schlug fehl mit korrekter Diagnose („13 buttons declared but only 12 have onmousedown handlers"). Nach Restore wieder 5/5 grün.

## Manual test needed

Vorstands-UAT-Checklist Phase 26 (Steps 13-16) sollte jetzt tatsächlich passieren — vorher hätte sie am UL/OL/H2/H3-Klick ohne sichtbare Änderung gefailt. Kein zusätzlicher Step nötig; die bestehenden Checklist-Steps decken die Änderung ab.

## Deviations

Keine.
