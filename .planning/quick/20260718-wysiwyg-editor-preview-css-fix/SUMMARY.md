---
slug: wysiwyg-editor-preview-css-fix
completed: 2026-07-18
status: complete
files_changed:
  - genossi-frontend/input.css
  - genossi-frontend/tailwind.config.js
  - genossi-frontend/assets/tailwind.css (regenerated)
  - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
  - genossi-frontend/src/component/mail_compose/template_preview.rs
tests_added: 3
---

# SUMMARY: Editor + Preview CSS-Scope

## Was gefixt wurde

Neue `.mail-html-render`-Klasse in `input.css` mit Browser-Default-artigem Styling für h1-h6/ul/ol/li/blockquote/p/a/b/strong/i/em/u/s. Ersetzt die nutzlose `prose prose-sm max-w-none`-Klasse in `wysiwyg_editor.rs:70` und `template_preview.rs:192`. Tailwind Typography Plugin wurde nicht installiert (User-Entscheidung Option b, WYSIWYG-Treue).

Nach dem Rebuild (`nix-store-tailwindcss -i input.css -o assets/tailwind.css`) hat `assets/tailwind.css` 17 `.mail-html-render`-Selektoren; `safelist` in `tailwind.config.js` hält die Klasse als Belt-and-Suspenders am Leben.

## Test coverage

3 neue Grep-Gate-Tests analog Phase 26 EDIT-09 Pattern:

- `wysiwyg_editor::grep_gate_tests::editor_uses_mail_html_render_scope` — assert Editor-Klasse enthält `mail-html-render` UND enthält kein `prose ` (Regressions-Sperre).
- `template_preview::grep_gate_tests::preview_uses_mail_html_render_scope` — dito für Preview.
- `template_preview::grep_gate_tests::production_region_excludes_test_module` — Meta-Test.

Self-Reference-Hazard-Schutz: `production_region()`-Slice + `format!`-assembled Needles. Ursprünglich hatte das template_preview.rs-Modul einen doc-comment über `mod grep_gate_tests` mit literal `prose prose-sm` — der landete im production_region und triggerte False-Positive. Fix: Docstring in die einzelne Test-fn verschoben, Modul-Kommentar auf `//` reduziert.

## Verification

- `cargo test grep_gate_tests` → 8 passed (5 pre-existing + 3 neu).
- `cargo fmt` clean.
- `grep -c mail-html-render assets/tailwind.css` → 17.
- Vorherige Fail-Iteration mit literal `prose prose-sm` im doc-comment bestätigte dass der Regressions-Check biteed.

## Manual test needed

`dx serve`-Session mit Browser-Hard-Reload:
- Editor: `<h2>Titel</h2>` beim Klick auf H2-Button sichtbar größer/fetter; `<ul><li>A</li><li>B</li></ul>` beim UL-Button mit Bullets und Einrückung; Blockquote mit linkem Border.
- Preview: dieselbe visuelle Darstellung.

## Deviations

- **Doc-comment-Fall in template_preview.rs:** Literal `prose prose-sm` in module-doc über `mod grep_gate_tests` triggerte den eigenen Regressions-Check. Docstring in die Test-fn verschoben — kein Verlust an Erklärung, aber `production_region()` bleibt sauber.

## Out of Scope

- Bug C (Plain-Text-Alternative aus HTML rendern) — separater Quick-Task, kommt danach.
- Andere `dangerous_inner_html`-Uses (qr_card.rs = SVG, nicht relevant).
