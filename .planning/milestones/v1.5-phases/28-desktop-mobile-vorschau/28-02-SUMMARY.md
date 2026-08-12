---
phase: 28-desktop-mobile-vorschau
plan: 02
subsystem: frontend
tags: [dioxus, iframe, sandbox, css-isolation, uuid-validation, i18n, grep-gate]

# Dependency graph
requires:
  - phase: 24-wysiwyg-frontend-editor
    provides: "`wysiwyg_toolbar.rs` mit `image_insert_html`, Grep-Gate-Muster inkl. Self-Reference-Abwehr in `wysiwyg_editor.rs`"
  - phase: 27-bild-support-backend-editor-upload
    provides: "`/api/mail/assets/{id}/bytes`-Route, `config.backend`-basierte Asset-URL, gehärtete `<img>`-ammonia-Policy"
  - phase: 28-desktop-mobile-vorschau
    plan: 01
    provides: "sanitisiertes `body_html` in der `POST /api/mail/preview`-Response — der Input von `inject_asset_src`"
provides:
  - "`PreviewMode`-Enum mit `width_px()` (640/360/None) und `is_preview()`"
  - "`preview_needs_fetch(from, to)` — Request nur beim Übergang Bearbeiten → Vorschau (D-05)"
  - "`inject_asset_src(html, backend)` — ergänzt `src` aus `asset_bytes_url`, behält `data-genossi-asset-id`"
  - "`preview_srcdoc(body_html)` — self-contained Dokument mit inline Baseline-Stylesheet und utf-8-Angabe"
  - "`MailPreviewFrame`-Component — sandboxed iframe fester Breite, Device-Rahmen, Fehler-Block"
  - "`asset_bytes_url(backend, id)` in `wysiwyg_toolbar.rs` als einzige Quelle der Asset-Bytes-URL"
  - "sieben `MailEditorMode*`-i18n-Keys in beiden Locales"
  - "21 neue Tests: 17 Verhaltenstests + 4 Source-Invarianten-Gates"
affects: [28-03-editor-verkabelung, 28-04, 28-05-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure Funktionen vor RSX: Geometrie, Asset-Rewrite und Dokument-Aufbau leben als reine Funktionen, die RSX-Hülle bleibt dünn — dadurch nativ unit-testbar ohne wasm32-Target und ohne Browser"
    - "Eine URL-Quelle, zwei Markup-Formen: `asset_bytes_url` wird von `image_insert_html` (kompletter Tag) und `inject_asset_src` (Attribut-Einschub) geteilt"
    - "String-Scan statt DOM-Parsing für Attribut-Rewrites im Frontend — spiegelt `genossi_mail::render::rewrite_img_cids`"
    - "Modul-Level `#![allow(dead_code)]` mit TODO-Rückbau-Hinweis für Primitive, deren Konsument erst im Folgeplan entsteht"
    - "Umschreibende Kommentar-Formulierung, damit Negativ-Grep-Gates keine False-Positives auf der eigenen Rationale erzeugen"

key-files:
  created:
    - "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs"
  modified:
    - "genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs"
    - "genossi-frontend/src/component/mail_compose/mod.rs"
    - "genossi-frontend/src/i18n/mod.rs"
    - "genossi-frontend/src/i18n/de.rs"
    - "genossi-frontend/src/i18n/en.rs"

key-decisions:
  - "Baseline-Stylesheet bewusst NICHT als Kopie von `.mail-html-render`: sähe die Vorschau exakt wie der Editor aus, wäre der Phasenzweck (Diskrepanzen sichtbar machen) unterlaufen — nackte Browser-Defaults wären andererseits Times New Roman 16 px"
  - "Kein Escaping des Vorschau-Dokuments (D-09): Dioxus setzt Attribute per `setAttribute` ohne HTML-Quelltext-Parsing; Escaping würde im iframe sichtbaren Escape-Text erzeugen und wäre ein Bug, keine Härtung"
  - "`Uuid::parse_str` vor der URL-Interpolation als eigenständige Sicherheitsschicht, unabhängig von ammonia und Sandbox — beides sind Umgebungsannahmen, die eine spätere Änderung aushebeln kann"
  - "Modul-Level `#![allow(dead_code)]` statt fünf Einzelattribute, mit explizitem TODO für Plan 28-03 — alle Symbole der Datei sind Primitive ohne Produktions-Konsument in diesem Plan"
  - "`rustfmt --edition 2021 <datei>` statt `cargo fmt` crate-weit, damit die vorbestehende `api.rs`-Drift nicht in den Plan-Diff gezogen wird (repo-spezifisches Git-Protokoll)"

patterns-established:
  - "Negativ-Nachweis eines Sicherheits-Grep-Gates als Pflichtschritt: das verbotene Token wird einmalig eingebaut, der Fehlschlag samt Meldung protokolliert, die Änderung zurückgenommen und die Suite erneut grün gefahren"

requirements-completed: [PREV-01, PREV-03, PREV-04, PREV-05]

coverage:
  - id: D1
    description: "PreviewMode meldet 640 px für Desktop, 360 px für Mobile und keine Breite für Edit; is_preview() ist genau für die Device-Modi wahr"
    requirement: "PREV-01"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_mode_widths_are_640_and_360"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_mode_edit_has_no_width"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_mode_is_preview_only_for_device_modes"
        status: pass
    human_judgment: false
  - id: D2
    description: "Ein <img> mit gültiger UUID im data-Attribut bekommt zusätzlich ein src, das mit der backend-Basis beginnt; das data-Attribut bleibt unverändert stehen"
    requirement: "PREV-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_adds_src_and_keeps_asset_id"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_uses_backend_base_not_relative"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_handles_multiple_and_duplicate_images"
        status: pass
    human_judgment: false
  - id: D3
    description: "Ein Nicht-UUID-Wert im data-Attribut lässt den Tag byte-identisch und erzeugt kein src — auch wenn der Wert ein Anführungszeichen und ein Event-Handler-Fragment enthält (T-28-07)"
    requirement: "PREV-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_ignores_non_uuid_value"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_rejects_quote_injection_payload"
        status: pass
    human_judgment: false
  - id: D4
    description: "HTML ohne <img> kommt byte-identisch heraus (v1.4-Backward-Compat); ein unvollständiger <img>-Tag ohne schließendes > führt nicht zu Panic oder Endlosschleife (T-28-10)"
    requirement: "PREV-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_leaves_html_without_images_untouched"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_handles_unterminated_tag_without_panic"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#inject_asset_src_preserves_surrounding_markup"
        status: pass
    human_judgment: false
  - id: D5
    description: "Das Vorschau-Dokument trägt sein Stylesheet inline, referenziert kein externes Stylesheet und keine App-CSS-Klasse, bettet den Body ohne Escaping ein und deklariert utf-8 vor dem Stylesheet (T-28-08, Pitfall 8)"
    requirement: "PREV-05"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#srcdoc_is_self_contained_no_external_css"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#srcdoc_embeds_body_html_verbatim"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#srcdoc_declares_utf8_charset"
        status: pass
    human_judgment: false
  - id: D6
    description: "Die MailPreviewFrame-Produktionsregion setzt ein Sandbox-Attribut mit Same-Origin-Erlaubnis und enthält niemals die Script-Erlaubnis; sie füttert das Dokument über das iframe-Attribut statt über Roh-HTML-Einbettung (T-28-06)"
    requirement: "PREV-05"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_frame_sets_sandbox_attribute"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_frame_never_allows_scripts"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_frame_uses_iframe_srcdoc_not_inner_html"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#production_region_excludes_test_module"
        status: pass
    human_judgment: false
  - id: D7
    description: "Der Request läuft nur beim Übergang Bearbeiten → Vorschau; der Wechsel Desktop ↔ Mobile löst keinen neuen Request aus (D-05)"
    requirement: "PREV-01"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_needs_fetch_on_edit_to_preview"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs#preview_needs_fetch_false_between_device_modes"
        status: pass
    human_judgment: false
  - id: D8
    description: "Der Device-Rahmen mit grauem Backdrop und Breiten-Label grenzt die Vorschau visuell klar vom Bearbeiten-Modus ab; Links in der Vorschau sind nicht klickbar"
    requirement: "PREV-04"
    verification: []
    human_judgment: true
    rationale: "Die Sandbox-Invariante und die Rahmen-Klassen sind am Quelltext nachgewiesen, aber ob der Vorstand die Abgrenzung tatsächlich sofort als 'das ist eine Vorschau, kein Editor' liest, ist eine visuelle Beurteilungsfrage. Gehört in die UAT in Plan 28-05 — zusammen mit dem tatsächlichen Laden der Bilder im iframe (Cookie-Verhalten, Assumption A2) und der CSS-Bleed-Gegenprobe."

# Metrics
duration: 21min
completed: 2026-07-28
status: complete
---

# Phase 28 Plan 02: Device-Vorschau-Primitive Summary

**`mail_preview_frame.rs` liefert die komplette Vorschau-Mechanik als pure Funktionen plus eine dünne RSX-Hülle: Geometrie, Asset-`src`-Injektion mit UUID-Validierung, ein self-contained Vorschau-Dokument und ein sandboxed iframe — alles nativ testbar, ohne Browser und ohne wasm32-Target.**

## Performance

- **Duration:** ~21 min
- **Tasks:** 3/3
- **Commits:** 3 (`6f6b08d`, `9c45fc9`, `4539d84`)
- **Dateien:** 1 neu, 5 modifiziert — kein Cargo-Manifest, keine neue Dependency, kein neues `web-sys`-Feature

## Testergebnis

`cd genossi-frontend && cargo test` → **322 passed, 0 failed** (Baseline 301 + 21 neue).
Vorgegebenes Minimum aus dem Plan: 318.

| Filter | Ergebnis |
|---|---|
| `cargo test inject_asset_src` | 8 passed |
| `cargo test srcdoc_` | 4 passed |
| `cargo test preview_frame_sets_sandbox_attribute` | 1 passed |
| `cargo test preview_frame_never_allows_scripts` | 1 passed |
| `cargo test preview_frame_uses_iframe_srcdoc_not_inner_html` | 1 passed |
| `cargo test production_region_excludes_test_module` | 4 passed (jetzt vier Dateien mit dem Muster) |

`cargo build` exit 0. `cargo clippy --all-targets` exit 0, **null** Warnungen mit Bezug auf
`mail_preview_frame.rs`. `cargo fmt -- --check` meldet für die neue Datei nichts (siehe
Abweichung 1 zur vorbestehenden `api.rs`-Drift).

## Negativ-Nachweis für den Sandbox-Grep-Gate

Einmalig geführt, wie vom Plan gefordert. Der Sandbox-Wert wurde testweise von
`allow-same-origin` auf `allow-same-origin allow-scripts` geändert.
`cargo test preview_frame_never_allows_scripts` schlug daraufhin fehl — Wortlaut der
Panic-Meldung:

```
thread '…::grep_gate_tests::preview_frame_never_allows_scripts' panicked at
src/component/mail_compose/mail_preview_frame.rs:563:9:
Grep gate FAILED: allow-scripts appeared in mail_preview_frame.rs (production region).
Combined with the same-origin token it lets the embedded document remove the sandbox
attribute from itself, which voids the isolation entirely. This is a security invariant
(T-28-06), not a style check — do not silence this test, remove the token.

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 321 filtered out
```

Die Änderung wurde unmittelbar zurückgenommen; `cd genossi-frontend && cargo test` läuft
seitdem wieder mit 322 passed / 0 failed. Der committete Stand enthält den Token nicht —
`grep -c 'allow-scripts' mail_preview_frame.rs` ergibt 0.

Damit ist T-28-06 nicht nur behauptet, sondern die Schutzwirkung des Gates bewiesen: Das
verbotene Token ist tatsächlich detektierbar, der Gate ist kein wirkungsloser Platzhalter.

## Gewählte Werte des Baseline-Stylesheets (D-10)

`MAIL_PREVIEW_BASELINE_CSS` ist ein privater `&str` mit genau diesen Regeln — inline im
Vorschau-Dokument, kein externes Stylesheet, kein `@import`:

| Selektor | Deklarationen |
|---|---|
| `html,body` | `margin:0;padding:0` |
| `body` | `font-family:Arial,Helvetica,sans-serif; font-size:14px; line-height:1.45; color:#222; padding:12px; word-wrap:break-word` |
| `p` | `margin:0 0 1em` |
| `h1` | `font-size:22px; margin:.5em 0` |
| `h2` | `font-size:18px; margin:.6em 0` |
| `h3` | `font-size:16px; margin:.7em 0` |
| `ul,ol` | `margin:0 0 1em; padding-left:24px` |
| `li` | `margin:.15em 0` |
| `blockquote` | `margin:0 0 1em; padding-left:12px; border-left:3px solid #ccc; color:#555` |
| `a` | `color:#1155cc` |
| `img` | `max-width:100%; height:auto` |
| `table` | `border-collapse:collapse` |

Bewusst **nicht** identisch mit `.mail-html-render` aus `input.css` (D-10): Die Editor-Regeln
arbeiten mit `rem`-Größen, `font-style:italic` auf `blockquote` und Tailwind-Slate-Tönen; die
Baseline orientiert sich stattdessen an dem, was Thunderbird/Outlook/Gmail für HTML-Mails
ohne eigene Styles rendern. Sähe die Vorschau exakt wie der Editor aus, wäre der Zweck der
Phase — Diskrepanzen zwischen Editor-DOM und Empfänger-Sicht sichtbar zu machen (PREV-02) —
unterlaufen. Nackte Browser-Defaults wären andererseits Times New Roman 16 px, was kein
realer Mail-Client so zeigt.

## Exportierte Symbole für Plan 28-03

Plan 28-03 muss die Datei nicht erneut lesen. Alle Symbole liegen in
`crate::component::mail_compose::mail_preview_frame`, sofern nicht anders vermerkt.

| Symbol | Signatur / Form | Sichtbarkeit | Import-Weg |
|---|---|---|---|
| `PREVIEW_WIDTH_DESKTOP_PX` | `u32 = 640` | `pub` | vollqualifizierter Modulpfad |
| `PREVIEW_WIDTH_MOBILE_PX` | `u32 = 360` | `pub` | vollqualifizierter Modulpfad |
| `PREVIEW_HEIGHT_PX` | `u32 = 640` | `pub` | vollqualifizierter Modulpfad |
| `PreviewMode` | `enum { Edit, Desktop, Mobile }`, `Clone + Copy + PartialEq + Eq + Debug` | `pub` | vollqualifizierter Modulpfad |
| `PreviewMode::width_px` | `fn(self) -> Option<u32>` | `pub` | Methode |
| `PreviewMode::is_preview` | `fn(self) -> bool` | `pub` | Methode |
| `preview_needs_fetch` | `fn(from: PreviewMode, to: PreviewMode) -> bool` | `pub(crate)` | vollqualifizierter Modulpfad |
| `inject_asset_src` | `fn(html: &str, backend: &str) -> String` | `pub(crate)` | vollqualifizierter Modulpfad |
| `preview_srcdoc` | `fn(body_html: &str) -> String` | `pub(crate)` | vollqualifizierter Modulpfad |
| `MailPreviewFrame` | Component, Props `mode: PreviewMode`, `srcdoc: String`, `#[props(default)] errors: Vec<String>` | `pub` | **re-exportiert**: `crate::component::mail_compose::MailPreviewFrame` |
| `asset_bytes_url` | `fn(backend: &str, id: &str) -> String` | `pub(crate)` | `crate::component::mail_compose::wysiwyg_toolbar::asset_bytes_url` |
| `MAIL_PREVIEW_BASELINE_CSS` | `&str` | privat | nicht importierbar (Absicht) |
| `extract_asset_uuid` | `fn(tag: &str, attr: &str) -> Option<Uuid>` | privat | nicht importierbar (Absicht) |

Nur die Component ist über `mod.rs` re-exportiert. Die pure Funktionen bleiben `pub(crate)`
und werden über den vollqualifizierten Modulpfad importiert — dieselbe Konvention wie bei
`wysiwyg_toolbar`.

**Erwartete Aufrufreihenfolge in 28-03:**
`preview_needs_fetch(alt, neu)` entscheidet über den Request →
`inject_asset_src(response.body_html, &config.backend)` →
`preview_srcdoc(…)` → `MailPreviewFrame { mode, srcdoc, errors }`.

**Namenskollision beachten:** In `crate::page::templates` existiert ein gleichnamiger, aber
vollkommen unverwandter privater `PreviewMode` (Member-/Application-Umschaltung). Immer den
vollqualifizierten Pfad verwenden.

## Neue i18n-Keys

Alle sieben in `mod.rs`, `de.rs` und `en.rs` im selben Commit (`6f6b08d`) — kein Locale-Drift.

| Key | de | en |
|---|---|---|
| `MailEditorModeEdit` | Bearbeiten | Edit |
| `MailEditorModeDesktop` | Desktop-Vorschau | Desktop preview |
| `MailEditorModeMobile` | Mobile-Vorschau | Mobile preview |
| `MailEditorModeDesktopFrameLabel` | Desktop-Vorschau (640 px) | Desktop preview (640 px) |
| `MailEditorModeMobileFrameLabel` | Mobile-Vorschau (360 px) | Mobile preview (360 px) |
| `MailEditorModeSelectMember` | Mitglied für die Vorschau wählen | Select a member for the preview |
| `MailEditorModeLoading` | Vorschau wird geladen … | Loading preview … |

Verwendet wird in diesem Plan nur das Frame-Label-Paar (im `MailPreviewFrame`). Die übrigen
fünf konsumiert Plan 28-03.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blockierend] Modul-Level `#![allow(dead_code)]` statt fehlender Konsumenten**

- **Gefunden während:** Task 2
- **Problem:** Das Akzeptanzkriterium fordert `cargo clippy --all-targets` ohne neue
  Warnungen für `mail_preview_frame.rs`. Da dieser Plan bewusst KEINE Editor-Integration
  enthält, hat kein Symbol der Datei einen Produktions-Konsumenten: `cargo build` meldete
  fünf `never used`-Warnungen (drei Konstanten, das Enum samt Varianten, `width_px`) — die
  Dead-Code-Analyse propagiert von der ungenutzten Component nach unten.
- **Fix:** Ein Modul-Level `#![allow(dead_code)]` mit ausführlichem Rationale-Kommentar und
  explizitem `TODO (Plan 28-03): nach der Verkabelung wieder entfernen`. Bewusst gewählt
  gegenüber fünf verstreuten Einzelattributen: eine Stelle, eine Begründung, ein klarer
  Rückbau-Auftrag.
- **Datei:** `genossi-frontend/src/component/mail_compose/mail_preview_frame.rs`
- **Commit:** `9c45fc9`

**2. [Rule 3 - Blockierend] Gezieltes `rustfmt` statt crate-weitem `cargo fmt`**

- **Gefunden während:** Task 3
- **Problem:** Nach dem Anlegen der Testmodule meldete `cargo fmt -- --check` fünf
  Fundstellen in `mail_preview_frame.rs`. Ein `cargo fmt` über den Crate hätte zugleich die
  vorbestehende Drift in `src/api.rs:405` mitverändert und damit eine unbeteiligte Datei in
  den Plan-Diff gezogen — was das repo-spezifische Git-Protokoll ausdrücklich verbietet.
- **Fix:** `rustfmt --edition 2021 src/component/mail_compose/mail_preview_frame.rs`.
  Anschließend ist die neue Datei fmt-sauber; die einzige verbleibende `--check`-Fundstelle
  im Crate ist die vorbestehende `api.rs`-Drift.
- **Nachkontrolle:** Suite nach dem Formatieren erneut grün (322 passed) — insbesondere die
  Grep-Gates, deren Needles auf exakte Byte-Sequenzen im RSX zielen.
- **Datei:** `genossi-frontend/src/component/mail_compose/mail_preview_frame.rs`
- **Commit:** `4539d84`

**3. [Rule 3 - Blockierend] Stale `genossi-frontend/Cargo.lock` nicht committet**

- **Gefunden während:** Task 1
- **Problem:** Jeder Build schreibt `Cargo.lock` neu, weil die committete Fassung noch die
  Vorgänger-Version des datumsbasierten Dev-Version-Strings trägt
  (`2026.196.1-dev` → `2026.207.1-dev`). Der Plan verlangt aber ausdrücklich, dass
  `git diff --name-only` **kein** Manifest listet (T-28-SC, Package-Legitimacy).
- **Fix:** Diff geprüft (genau eine Zeile, reiner Version-String, **keine**
  Dependency-Änderung), Datei per `git checkout --` zurückgesetzt und in keinem Commit
  gestaged. Es wurde kein Paket installiert.
- **Datei:** `genossi-frontend/Cargo.lock` (unverändert im Repo)

### Bewusst NICHT gefixt (Scope Boundary)

- **`cargo fmt`-Drift in `genossi-frontend/src/api.rs:405`** — vorbestehend, unberührte
  Datei, kein Phase-28-Bezug. Nach `deferred-items.md` Punkt 5 ausgelagert.
- **`warning: unused import: mail_preview_frame::MailPreviewFrame` in `mod.rs`** — der
  Re-Export ist ein Pflicht-Artefakt dieses Plans; `component/mod.rs` trägt bereits acht
  identische Warnungen für denselben Vorgriffs-Fall (etablierte Repo-Konvention). Verschwindet
  mit der Verkabelung in Plan 28-03. Nach `deferred-items.md` Punkt 6 ausgelagert.
- **Die beiden vorbestehenden e2e-Fehlschläge im Backend** — von diesem Plan nicht berührt
  (er fasst kein Backend-File an), bereits als Punkt 1 und 2 in `deferred-items.md`.

## Authentication Gates

Keine.

## Known Stubs

Keine. Alle Symbole sind vollständig implementiert und durch Tests belegt. Dass sie in diesem
Plan noch keinen Produktions-Konsumenten haben, ist der ausdrückliche Plan-Zuschnitt („Dieser
Plan enthält bewusst KEINE Editor-Integration — er liefert Primitive, die Plan 28-03 nur noch
verkabelt") und kein Stub: Es gibt keine hartkodierten Leerwerte, keine Platzhalter-Texte und
keine Component ohne Datenquelle.

## Threat Flags

Keine neue Angriffsfläche außerhalb des Threat Models. Die drei `mitigate`-Dispositionen
dieses Plans sind umgesetzt und automatisiert belegt:

| Threat | Umsetzung | Beweis |
|---|---|---|
| T-28-06 (Sandbox-Escape) | Sandbox mit ausschließlich der Same-Origin-Erlaubnis; Script-Erlaubnis nie gesetzt | `preview_frame_never_allows_scripts` + einmaliger Negativ-Nachweis oben |
| T-28-07 (Attribut-Injektion) | `Uuid::parse_str` vor der URL-Interpolation | `inject_asset_src_ignores_non_uuid_value`, `inject_asset_src_rejects_quote_injection_payload` |
| T-28-08 (externe Ressourcen) | Stylesheet inline, kein `<link>`, kein `@import` | `srcdoc_is_self_contained_no_external_css` |
| T-28-10 (DoS bei unvollständigem Tag) | Rest wird angehängt, Schleife verlassen | `inject_asset_src_handles_unterminated_tag_without_panic` |
| T-28-SC (Package-Legitimacy) | kein Install, kein Manifest im Diff | `git diff --name-only` listet nur die sechs Quelldateien |

## Offene Punkte für die UAT (Plan 28-05)

- Tatsächliches Laden der Bilder im iframe inklusive Cookie-Verhalten (Assumption A2 der
  Research: `SameSite=Strict` mit Same-Origin-Erlaubnis).
- Visuelle Abgrenzung des Device-Rahmens (PREV-04).
- CSS-Bleed-Gegenprobe in beide Richtungen mit einem Konflikt-Selektor.
- Zweimaliges Hin- und Herschalten Bearbeiten ↔ Vorschau (Assumption A3: Dokument-Attribut
  aktualisiert sich reaktiv).

## Self-Check: PASSED

Alle sieben behaupteten Dateien existieren auf der Platte, alle drei Commit-Hashes
(`6f6b08d`, `9c45fc9`, `4539d84`) sind in `git log` auffindbar.
