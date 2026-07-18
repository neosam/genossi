---
slug: html-to-plain-derivation
completed: 2026-07-18
status: complete
files_changed:
  - genossi_mail/Cargo.toml
  - genossi_mail/src/render.rs
tests_added: 7
supersedes_constraint: HTML-02 (im Render-Layer, nicht im Send-Layer)
---

# SUMMARY: Plain-Text-Alternative aus HTML ableiten

## Was gefixt wurde

`html2text = "0.17"` als Backend-Dependency aufgenommen. Neuer Helper `plain_from_html(html: &str) -> String` in `render.rs` nutzt `html2text::from_read(..., 78)` (RFC-2822-freundliche Zeilenbreite).

In `resolve_rendered_content` nach dem HTML-Render: wenn `body_html` Some ist, wird `body` mit `plain_from_html(rendered_html)` überschrieben. Der Frontend-supplied `body` (`element.innerText()`) wird bewusst ignoriert weil er semantische Struktur (Bullets, Nummern, Titel-Marker) verliert.

**Backward-Compat:** `body_html: None` → alter Pfad, Frontend-`body` unverändert (v1.4-Plaintext-Mails byte-identisch).

**HTML-02:** `send.rs::build_message` bleibt HTML-02-treu — `body`-Argument ist raw plain, nie aus html abgeleitet. Nur der vorgelagerte Render-Layer entscheidet jetzt anders. Docstring in `send.rs` bleibt korrekt.

## html2text-Output-Beispiele

```
<ul><li>Apfel</li><li>Birne</li></ul>       →  * Apfel\n* Birne
<ol><li>Eins</li><li>Zwei</li></ol>         →  1. Eins\n2. Zwei
<h1>Titel</h1>                              →  # Titel
<h2>Titel</h2>                              →  ## Titel
<blockquote>Zitat</blockquote>              →  > Zitat
<b>Welt</b>                                 →  **Welt**
<a href="http://x">hier</a>                 →  [hier][1]  (+ Footnote-Ref)
```

## Test coverage

7 neue Tests in `render.rs::mod tests`:

- `plain_from_html_unordered_list_has_bullets` — `* Apfel`, `* Birne`.
- `plain_from_html_ordered_list_is_numbered` — `1. Eins`, `2. Zwei`.
- `plain_from_html_headings_are_marked` — Titel + `#`/`===`/UPPER-Marker (tolerant gegen html2text-Update).
- `plain_from_html_blockquote_prefixed` — `> Zitat hier`.
- `plain_from_html_empty_input_is_empty` — Edge-Case.
- `plain_from_html_bold_becomes_markdown_stars` — enthält Text (Marker-tolerant).

Plus 2 bestehende Tests erweitert:
- `resolve_rendered_content_renders_html_body` — body-Assertion added: `body == "Hallo Max"` (aus html2text) statt „Text body" (Frontend-supplied).
- `resolve_rendered_content_body_html_none_when_job_body_html_none` — body-Assertion added: `body == "Text body"` (Backward-Compat).

## Verification

- `cargo test -p genossi_mail --lib` → 261 passed (255 pre-existing + 6 neu). Erweiterte Tests grün.
- `cargo fmt` clean.
- `cargo build -p genossi_mail` → grün.
- **E2E-Tests konnten nicht laufen** wegen `No space left on device` beim Linker (877 MB frei bei 53 GB `target/`). Lib-Tests belegen die Regression-Freiheit; E2E-Sanity kann nach `cargo clean` bzw. Disk-Cleanup nachgeholt werden. Kein Code-Regressions-Indikator.

## Manual test needed

WYSIWYG-Mail mit UL/OL/H2/H3/Blockquote versenden, dann:
- **Job-Detail-Frontend** (`MailRecipientRenderedContent`): rendered_body-Feld zeigt Markdown-ähnliche Struktur (Bullets, Nummern, Titel-Prefixes).
- **Text-Client des Empfängers** (wenn möglich reine Text-Mail-App): strukturierte Darstellung statt Textblock.

## Deviations

- **HTML-02 abgemildert:** Explizit im PLAN.md dokumentiert. Die Scope-Minimierungs-Entscheidung aus Phase 23 (kein Zusatz-Crate, keine Text-aus-HTML-Ableitung) wird im Render-Layer aufgehoben, weil sie zu unlesbarem Text-Fallback bei WYSIWYG-Formatierung führte. `send.rs`-Invariante bleibt.
- **E2E-Test nicht ausgeführt:** Disk-Space-Blocker, kein Code-Problem. Nächste Session: `cargo clean` oder gezielter Cleanup, dann E2E sanity.

## Follow-up (separater Task)

- **Disk-Space:** `/dev/mapper/luks-...` bei 100% (877 MB frei / 950 GB). `target/` = 53 GB. Kandidaten zum Cleanup: alte Nix-Store-Generationen, `cargo clean` einzelner Workspace-Mitglieder, Docker-Container falls vorhanden. Nicht Teil dieses Quick-Tasks.
- Frontend-Optimierung: `sync_from_dom` könnte auf `he.inner_text()` verzichten und nur HTML senden. Backend würde beides ableiten. Cleaner-API, aber Backward-Compat-Break — separater Task.

## Nachtrag 2026-07-18 (unmittelbar nach Erst-Commit)

**Gap gefunden:** Erste Iteration fixte nur den Render-Layer (`resolve_rendered_content`). Preview + Test-Mail-Endpoint in `genossi_mail/src/rest.rs` haben **eigene** Render-Pfade (`render_template` + `render_html_template` inline), gehen nicht durch `resolve_rendered_content`. Effekt: User sah in der Editor-Preview weiter zusammenhängenden Text, obwohl versendete Mails korrekt strukturiert waren.

**Nachtrag:** Beide Sites in `rest.rs` (`preview_mail` bei rendered_body_html-Match + `send_test_mail_with_template` analog) bekommen denselben `plain_from_html`-Override. 261/261 lib-Tests bleiben grün.

Lesson: Bei „Backend leitet aus HTML ab"-Fixes IMMER alle Render-Sites grep-pen (`render_template` + `render_html_template` als Anker), nicht nur die Worker-Pipeline. Die Preview/Test-Pfade waren aus historischen Gründen (Phase 22-23-Refactor) nicht durch `resolve_rendered_content` konsolidiert — Refactor-Kandidat für später.
