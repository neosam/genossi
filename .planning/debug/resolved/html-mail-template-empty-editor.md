---
slug: html-mail-template-empty-editor
status: resolved
trigger: "HTML-Mail-Templates werden im Frontend leer angezeigt, obwohl sie als Text hinterlegt sind"
created: 2026-07-17
updated: 2026-07-17
---

## Symptoms

- Editor auf `/mail-templates` ist leer, sobald ein Template ausgewählt wird
- Nutzer sieht keinen Weg, den Plain-Text-Body anzuzeigen
- Aufgetreten seit Einführung HTML-Mails (Phase 24)

## Current Focus

- hypothesis: `on_select_template` seedet `edit_body_html` aus `tpl.body_html.unwrap_or_default()` — Legacy-Templates ohne `body_html` liefern `""`, Editor bleibt leer. Fallback auf `tpl.body` fehlt.
- next_action: Fix implementieren (plain_to_html Helper + Fallback + Editor-key + mail_page-Prüfung + Test)

## Evidence

- timestamp: 2026-07-17
  file: genossi-frontend/src/page/mail_templates.rs:63
  content: `edit_body_html.set(tpl.body_html.clone().unwrap_or_default());`
  finding: Kein Fallback auf `tpl.body` bei fehlendem `body_html`.

- timestamp: 2026-07-17
  file: genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs:76-85
  content: `onmounted` ruft `set_inner_html(&initial_value)` — Prop-Änderung nach Mount aktualisiert DOM nicht.
  finding: Template-Wechsel füllt Editor nicht neu, weil kein Remount.

- timestamp: 2026-07-17
  file: migrations/sqlite/20260702000000_mail_templates_add_body_html.sql
  finding: `body_html` wurde in Phase 24 nachträglich hinzugefügt — bestätigt, dass Legacy-Templates `body_html = NULL` haben.

## Root Cause

Beim Merge von HTML-Templates (Phase 24) wurde der Load-Pfad im Frontend so verändert, dass der Editor nur noch aus `body_html` gespeist wird. Es fehlt ein Fallback für Templates, die vor Phase 24 angelegt wurden und daher nur `body` haben. Zusätzlich remountet der WysiwygEditor nicht bei Prop-Änderungen — ein pre-existing Design-Issue, das durch diesen Bug sichtbar wird.

## Fix

1. `pub fn plain_to_html(&str) -> String` in `wysiwyg_editor.rs` — HTML-Entities escapen, `\n` → `<br>`.
2. `mail_templates.rs on_select_template`: Fallback via `plain_to_html(&tpl.body)`, wenn `body_html` leer/None.
3. `key` auf `WysiwygEditor`-Aufruf → Remount bei Template-Wechsel (in mail_templates.rs, mail_page.rs und reply_form.rs).
4. `mail_page.rs`: TemplateSelector setzt `body`, wiped `body_html` → gleicher Fix per plain_to_html.
5. `reply_form.rs`: gleicher Fix (Template-Select seedet body_html jetzt via plain_to_html; editor_reset_counter triggert Remount).
6. Unit-Tests für `plain_to_html` (6 Cases, alle grün).

## Verification

- `cargo check` sauber
- `cargo test --bin genossi-frontend`: 290 pass, keine Regressionen
- 6 neue Tests in `wysiwyg_editor::tests` grün
