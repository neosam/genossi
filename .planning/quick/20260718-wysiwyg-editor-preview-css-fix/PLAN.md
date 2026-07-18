---
slug: wysiwyg-editor-preview-css-fix
created: 2026-07-18
type: quick-task
scope: bugfix/css
files_modified:
  - genossi-frontend/input.css
  - genossi-frontend/tailwind.config.js
  - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
  - genossi-frontend/src/component/mail_compose/template_preview.rs
  - genossi-frontend/assets/tailwind.css (regenerated)
---

# Quick-Task: WYSIWYG-Editor + Preview CSS-Fix

## Bug

Im WYSIWYG-Editor UND im TemplatePreview werden `<h1>`, `<h2>`, `<h3>`, `<ul>`, `<ol>`, `<blockquote>` visuell wie normaler Text dargestellt — keine Größe, keine Bullets, keine Einrückung. Der User sieht in DevTools dass die Elemente korrekt im DOM stehen, aber optisch tut sich nichts.

## Root Cause

Beide Container haben `class: "prose prose-sm max-w-none ..."`. Diese Klassen kommen aus Tailwind Typography — das Plugin ist aber NICHT installiert (`plugins: []` in `tailwind.config.js`, `.prose`-Selektor-Count in `assets/tailwind.css` = 0).

Ohne Plugin greift nur Tailwind's Preflight (Base-Reset), der u.a. setzt:
- `h1..h6 { font-size: inherit; font-weight: inherit; margin: 0 }`
- `ul, ol { list-style: none; padding: 0; margin: 0 }`
- `blockquote { margin: 0 }`

→ semantische HTML-Elemente sehen aus wie plain Text.

## Fix (Option (b) per User-Entscheidung 2026-07-18)

Editor+Preview-scoped Custom-CSS, kein Typography-Plugin, kein npm-Install, kein Flake-Rebuild-Risiko.

1. **Neue Custom-Klasse `.mail-html-render` in `input.css`**:
   - Reaktiviert Browser-Default-artiges Styling für h1-h6, ul, ol, li, blockquote im Scope dieser Klasse.
   - Werte matchen ungefähr was ein normaler Mail-Client (Gmail, Thunderbird) ohnehin rendert — WYSIWYG-treu.

2. **`wysiwyg_editor.rs:70`** und **`template_preview.rs:192`**: `prose prose-sm max-w-none` → `mail-html-render` (plus die anderen Layout-Klassen bleiben).

3. **`tailwind.config.js` `safelist`**: `mail-html-render` hinzufügen als safety-net gegen Purge (die Klasse ist zwar in .rs-Sources referenziert, `mode: "all"` + `content: ["./src/**/*.{rs,html,css}"]` sollte ausreichen — safelist ist gürtel-und-hosenträger).

4. **`assets/tailwind.css` regenerieren** via `npx tailwindcss -i input.css -o assets/tailwind.css`.

## Test

Grep-Gate-Test in `wysiwyg_editor.rs::grep_gate_tests` (bestehendes Modul erweitern) UND analog in `template_preview.rs` (neues Modul):

- Assert: der jeweilige Container hat die `mail-html-render`-Klasse.
- Assert: die alte `prose`-Klasse ist raus (Regression-Schutz gegen Rückkehr zu no-op).

Plus: Snapshot-artiger CSS-Test — grep in `assets/tailwind.css`, dass die neuen `.mail-html-render h1..h6, ul, ol, blockquote`-Regeln kompiliert sind (bestätigt: `npx tailwindcss` wurde nach Änderung ausgeführt).

## Success Criteria

1. `cargo test grep_gate` → grün (bestehende 5 + 2 neue).
2. `grep -c 'mail-html-render' assets/tailwind.css` → > 0 (bestätigt Rebuild).
3. Manueller Browser-Test: `<h1>Titel</h1>` im Editor sichtbar größer/fetter; `<ul><li>A</li><li>B</li></ul>` mit Bullets + Einrückung; Preview zeigt das gleiche.

## Out of Scope

- Bug C (Plain-Text-Alternative aus HTML rendern) — separater Quick-Task, kommt danach.
- Typography-Plugin (Option a) — bewusst nicht gewählt, um npm/Flake-Overhead zu vermeiden und WYSIWYG-Treue zum Mailclient zu wahren.
- Andere `dangerous_inner_html`-Uses (qr_card.rs = SVG, nicht relevant).
