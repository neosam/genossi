---
slug: wysiwyg-toolbar-onmousedown-fix
created: 2026-07-18
type: quick-task
scope: bugfix
files_modified:
  - genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs
---

# Quick-Task: WYSIWYG-Toolbar Selection-Preserve via onmousedown

## Bug

Block-Level-Formatierungs-Buttons (Unordered List, Ordered List, H1, H2, H3, Paragraph, Blockquote) machen sichtbar nichts, wenn der User Text im Editor selektiert und einen dieser Buttons klickt. Inline-Buttons (Bold, Italic, Underline, Strike) funktionieren.

## Root Cause

Alle Toolbar-Buttons haben nur `onclick: |evt| evt.prevent_default()`. Der Klick läuft in der Reihenfolge **mousedown → blur (contenteditable verliert Fokus) → mouseup → click**. Wenn das Contenteditable seinen Fokus verliert, wird die `Selection.type` auf `"None"` gesetzt und die Range geht verloren.

- Inline-Commands (bold/italic/underline/strike) tolerieren das, weil `execCommand` einen State-Flag toggelt.
- Block-Commands (`insertUnorderedList`, `insertOrderedList`, `formatBlock`) brauchen die Range, um zu wissen welchen Block sie transformieren sollen. Ohne Range → No-Op.

Der spätere `onclick`-Handler ruft zwar `focus_editor()` auf, aber `HTMLElement.focus()` stellt nur den Fokus wieder her, nicht die Selection Range.

## Fix

Für jeden Toolbar-Button in `wysiwyg_toolbar.rs`: `onmousedown: |evt| evt.prevent_default()` hinzufügen. `mousedown` feuert vor `blur` — wenn wir dort `preventDefault()` machen, verhindert der Browser den Fokus-Wechsel vom Contenteditable auf den Button. Selection bleibt intakt, `execCommand` findet die Range.

Betrifft alle 13 Buttons in `wysiwyg_toolbar.rs`:
Bold, Italic, Underline, Strike, UL, OL, H1, H2, H3, Paragraph, Blockquote, Link, Remove-Formatting.

**Nicht betroffen:** `wysiwyg_link_dialog.rs` — die Buttons dort sind in einem Modal, kein Contenteditable in der Nähe, kein Selection-Loss-Problem.

## Test

Grep-Gate-Test in `wysiwyg_toolbar.rs::grep_gate_tests` (analog Phase 26 EDIT-09):

- Zählt `button {`-Vorkommen (soll 13 sein).
- Zählt `onmousedown:`-Vorkommen (soll ≥ 13 sein).
- Assert: `#button-Blöcke == #onmousedown-Handler` — keiner darf fehlen.

Selben Self-Reference-Hazard-Schutz wie Phase 26 (`production_region()` schneidet ab dem Test-Modul-Marker).

## Success Criteria

1. `cargo test -p genossi-frontend grep_gate` (aus genossi-frontend/) → alle Tests grün.
2. Manueller Browser-Test: User selektiert Text, klickt UL/OL/H2/H3 → Element wird gewrappt/transformiert.

## Out of Scope

- `wysiwyg_link_dialog.rs`-Buttons (kein Selection-Kontext).
- Refactor der 12 `editor_id_X`-Clones (kein Bug, nur Kosmetik).
- Automatischer Browser-E2E-Test (Vorstands-UAT-Checklist deckt das ab; Phase 26 hat es als Ship-Gate vermerkt).
