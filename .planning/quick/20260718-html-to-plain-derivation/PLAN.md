---
slug: html-to-plain-derivation
created: 2026-07-18
type: quick-task
scope: bugfix/backend
files_modified:
  - genossi_mail/Cargo.toml
  - genossi_mail/src/render.rs
supersedes_constraint: HTML-02 (Phase 23 scope-min: kein Text-aus-HTML-Ableitung, kein Zusatz-Crate)
---

# Quick-Task: Plain-Text-Alternative aus HTML ableiten

## Bug

Wenn ein Sender im WYSIWYG-Editor Listen (`<ul>`, `<ol>`), Überschriften (`<h1..h3>`) oder Blockquotes erzeugt, verliert der Plain-Text-Teil in der ausgehenden `multipart/alternative`-Mail die semantische Formatierung. Grund: das Frontend leitet `body` aus `element.innerText()` ab — das gibt bei `<ul><li>Foo</li><li>Bar</li></ul>` nur `"Foo\nBar"` (keine Bullets), bei `<h2>Titel</h2>` nur `"Titel"` (keine Unterstreichung/Prefix).

Empfänger mit reinem Text-Client (Terminal-Mail, Screen-Reader, HTML-abgeschaltete Mail-App) sehen also einen strukturlosen Textblock — Lesbarkeit leidet.

## Root Cause / History

Phase 23 (HTML-02) hat *explizit* keine Text-aus-HTML-Ableitung eingebaut, um Scope zu minimieren und Bewegungsteile zu vermeiden. Phase 24 löste den WYSIWYG-Konflikt (HTML-02 vs. EDIT-01) über Frontend-`inner_text()`. Das funktioniert für Bold/Italic (keine semantische Struktur), degradiert aber bei Block-Level-Formatierung.

Bug C dreht die HTML-02-Entscheidung an einer präzisen Stelle wieder um: **im Render-Layer**, nicht im Send-Layer. `build_message`'s HTML-02-Invariante („`body` ist raw plain, nie aus html abgeleitet") bleibt intakt — nur der vorgelagerte Renderer entscheidet, was er als `body` liefert.

## Fix

1. **Dependency:** `html2text = "0.17"` zu `genossi_mail/Cargo.toml` hinzufügen. Server-side only, kein WASM-Impact. Mature Crate (~5 Jahre, ähnlich zu `lynx -dump`).

2. **`genossi_mail/src/render.rs`:** neuer Helper `pub(crate) fn plain_from_html(html: &str) -> String` — nutzt `html2text::from_read(html.as_bytes(), 78)` (78 = klassische Mail-Breite, RFC 2822-freundlich).

3. **`resolve_rendered_content`** (nach Zeile 187, wo `body_html` gerendert wird): wenn `body_html.is_some()`, `body` mit `plain_from_html(body_html.as_ref().unwrap())` überschreiben. Backward-Compat: `body_html: None` → alter Pfad, Frontend-supplied `body` unverändert (v1.4-Plaintext-Mails bleiben byte-identisch).

## Test

Neue Unit-Tests in `render.rs::mod tests`:
- `plain_from_html_ul_lists_have_bullets` — `<ul><li>A</li><li>B</li></ul>` → enthält `"* A"` und `"* B"` (oder `- A`/`- B`, je nach html2text-Default).
- `plain_from_html_ol_lists_are_numbered` — `<ol><li>A</li><li>B</li></ol>` → enthält `"1. A"` und `"2. B"`.
- `plain_from_html_h1_h2_h3_are_visually_marked` — enthält Titel-Text plus irgendein Marker (===, ---, oder ähnlich, je nach html2text).
- `plain_from_html_blockquote_prefixed` — `<blockquote>Zitat</blockquote>` → enthält `"> Zitat"`.
- `resolve_rendered_content_body_derived_from_html_when_body_html_some` — E2E im render-Layer: job mit body_html führt zu body ≠ übergebenem body, sondern zu html2text-Output.
- `resolve_rendered_content_body_unchanged_when_body_html_none` — Backward-Compat.

**Anpassung bestehender Tests:** Zwei Tests in `render.rs` (`resolve_rendered_content_body_html_some_when_job_body_html_some` und Verwandte) assertieren aktuell body-Werte, die aus dem übergebenen `body`-Template kommen. Wenn `body_html` gesetzt ist, wird body jetzt aus dem HTML abgeleitet — Erwartung anpassen.

## Success Criteria

1. `cargo test -p genossi_mail --lib` → alle Tests grün (neue + angepasste).
2. `cargo test -p genossi_bin --test e2e_tests` → keine neue Regression (die bekannte Phase-22-Altlast bleibt).
3. Manueller Browser-Test: WYSIWYG-Mail mit UL/OL/H2 versenden, im rendered_body-Feld (Job-Detail-Frontend) sichtbar strukturierten Text sehen (Bullets, Nummern, Titel-Marker).

## Out of Scope

- Frontend-Änderungen (`sync_from_dom`, `he.inner_text()`) bleiben — der Frontend-supplied `body` wird ohnehin überschrieben wenn body_html Some. Als saubere Follow-up-Task könnte Frontend nur noch html senden (nicht plain), aber das ist API-Break und Backfill-relevant → separat.
- Andere Send-Pfade (`send_test_mail`, Digest) — falls diese `body_html` benutzen, greift der Fix automatisch (alle gehen durch `resolve_rendered_content` oder analog). Falls sie plain-only sind, unbetroffen.
- HTML-02 aus `send.rs` docstring formal deprecaten — Docstring bleibt korrekt: `build_message`'s `body`-Argument IST raw plain; nur der Renderer davor liefert es jetzt aus HTML. Kein Kommentar-Update nötig.
