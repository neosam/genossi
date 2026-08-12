---
phase: 27-bild-support-backend-editor-upload
verified: 2026-07-23T14:00:00Z
status: human_needed
score: 9/9 must-haves verified
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Bild über Toolbar-Button einfügen und sofortiger Bild-Preview"
    expected: "Vorstand klickt den Toolbar-Button, wählt eine PNG/JPEG/GIF-Datei aus; nach dem Upload erscheint das Bild sofort im Editor via /api/mail/assets/{id}/bytes src."
    why_human: "Browser-WASM-Verhalten (file-picker + insertHTML-Sichtbarkeit) ist nicht per Unit-Test prüfbar."
  - test: "Drag&Drop eines Bildes auf den Editor"
    expected: "Bild wird gedroppt, hochgeladen und identisch zu Toolbar-Pfad eingebettet; Browser öffnet die Datei NICHT direkt."
    why_human: "DragEvent-Dispatch und visuelles Rendering im Browser können nur im echten WASM-Kontext validiert werden."
  - test: "Empfänger sieht Inline-Bild in echtem Mail-Client (Thunderbird/Outlook)"
    expected: "Mail mit <img data-genossi-asset-id> wird versendet; Empfänger sieht das Bild inline (multipart/related CID korrekt aufgelöst)."
    why_human: "Echte SMTP-Zustellung + Client-Rendering (CID-Auflösung in Thunderbird/Outlook) ist per automatisiertem Test nicht prüfbar."
  - test: "Test-Mail an Vorstand selbst zeigt Inline-Bild"
    expected: "Vorstand sendet Test-Mail mit eingebettetem Bild; empfangene Mail enthält das Bild sichtbar."
    why_human: "Echter SMTP-Transport + Posteingang-Validierung ist per Unit-Test nicht prüfbar."
---

# Phase 27: Bild-Support Backend + Editor-Upload — Verification Report

**Phase Goal:** Vorstand kann Inline-Bilder direkt im WYSIWYG-Editor hochladen und in HTML-Mails einbetten; die Empfänger sehen die Bilder in der Mail (inklusive Test-Mail an den Vorstand selbst).

**Verified:** 2026-07-23T14:00:00Z
**Status:** human_needed
**Re-verification:** Nein — erste Verifikation

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Beleg |
|---|-------|--------|-------|
| 1 | POST /api/mail/assets akzeptiert PNG/JPEG/GIF bis 5 MB und gibt die mail_asset-id zurück; Nicht-Admin-Aufrufer werden vor jedem DB-Write abgelehnt. | ✓ VERIFIED | `genossi_rest/src/mail_asset.rs` Route POST "/" mit DefaultBodyLimit; Service CR-02 `check_permission` als erste Anweisung; E2E-Test `test_mail_asset_upload_and_bytes_roundtrip` + `test_mail_asset_upload_svg_rejected_415` — beide grün. |
| 2 | GET /api/mail/assets/{id}/bytes liefert die gespeicherten Bytes mit dem Server-abgeleiteten Content-Type; nur Admins dürfen lesen. | ✓ VERIFIED | `download_mail_asset_bytes` in `genossi_rest/src/mail_asset.rs`; CR-02 in `MailAssetServiceImpl::download`; E2E-Test `test_mail_asset_upload_and_bytes_roundtrip` verifiziert 200 + Content-Type image/png + Byte-Identität. |
| 3 | mail_asset-Bytes werden inline als SQLite BLOB (Vec<u8>) gespeichert, nie im Dateisystem. | ✓ VERIFIED | Migration `20260723000000_create_mail_assets_table.sql`: `bytes BLOB NOT NULL`, kein FOREIGN KEY, kein `relative_path`. Kein `DocumentStorage`-Aufruf in keiner mail_asset-Datei. DAO-Test `test_mail_asset_blob_roundtrip_create_find` grün. |
| 4 | Magic-Byte-MIME-Sniff lehnt SVG, Polyglots und Nicht-PNG/JPEG/GIF-Payloads mit 415 ab, unabhängig von Client-Content-Type oder Dateiname. | ✓ VERIFIED | `sniff_image_mime` in `genossi_service_impl/src/mail_asset.rs`; Service-Tests `test_sniff_accepts_png_jpeg_gif` und `test_sniff_rejects_svg_and_non_images` + `test_upload_svg_rejected_no_dao_call` grün. E2E `test_mail_asset_upload_svg_rejected_415` grün. |
| 5 | sanitize_html erlaubt `<img>` nur mit dem Attribut `data-genossi-asset-id`; src, data:-URIs, externe HTTP-src und SVG werden gestrippt. | ✓ VERIFIED | `genossi_mail/src/sanitize.rs`: `OnceLock<ammonia::Builder>` mit `rm_tag_attributes("img", ...)`, `add_tag_attributes("img", &["data-genossi-asset-id"])`, `rm_url_schemes(&["data"])`. Kein `ammonia::clean()` auf dem Produktionspfad. Tests `sanitize_preserves_img_data_genossi_asset_id`, `sanitize_strips_external_http_img_src_keeps_asset_id`, `sanitize_strips_data_uri_img_src`, `sanitize_strips_svg` — alle 16 Sanitize-Tests grün. |
| 6 | Beim Senden wird `<img data-genossi-asset-id=X>` zu `<img src="cid:asset-N@genossi">` umgeschrieben und als multipart/related Inline-Part (Content-ID passend) angehängt; Struktur: multipart/mixed → related → alternative. | ✓ VERIFIED | `rewrite_img_cids` in `render.rs` + erweitertes `build_message` mit `inline_images`-Parameter in `send.rs`. Tests `build_message_related_structure_matches_cid_and_content_id` und `build_message_mixed_wraps_related_when_attachments_present` grün. 6 `rewrite_img_cids`-Tests grün. |
| 7 | Test-Mail-Versand (send_test_mail_with_body) bettet Bilder identisch ein — kein separater Image-Logik-Zweig. | ✓ VERIFIED | `InlineImageByteLoader` Trait + `image_loader`-Feld in `MailServiceImpl`; `send_test_mail_with_body` ruft `rewrite_img_cids` + Byte-Loader. Service-Tests `send_test_mail_with_body_loads_asset_bytes_when_html_has_image` und `send_test_mail_with_body_no_image_does_not_load_assets` grün. |
| 8 | Gesamtmailgröße (base64-kodiert) wird gegen 25 MB geprüft VOR der Assemblierung/SMTP; Überschreitung liefert klaren App-Fehler. | ✓ VERIFIED | `base64_encoded_len` + `MAX_ENCODED_MAIL_BYTES = 25 * 1024 * 1024` in `send.rs`; Prüfung vor Part-Building. Test `build_message_rejects_when_base64_encoded_size_exceeds_25mb` grün (D-02: Prüfung auf base64-Größe, nicht Rohgröße). |
| 9 | Bestehende Templates ohne Bilder (v1.4) senden weiterhin ohne multipart/related-Wrapper — alle bisherigen build_message-Tests bleiben byte-identisch grün. | ✓ VERIFIED | Leere `inline_images`-Slice → unveränderter 4-Zweig-Matrix. Test `build_message_empty_inline_images_is_byte_identical_no_related` + alle 13 Pre-existing-Tests (nur mechanisches `&[]`-Argument hinzugefügt) grün. |

**Score:** 9/9 Truths VERIFIED (0 present-behavior-unverified)

---

### Anforderungsabdeckung

| Requirement | Plan | Beschreibung | Status | Beleg |
|-------------|------|-------------|--------|-------|
| IMG-01 | 27-01 | mail_asset-Entität mit SQLite BLOB-Storage, DAO/Service/REST | ✓ SATISFIED | Migration, DAO-Trait, SQLite-Impl, BLOB-Roundtrip-Test |
| IMG-02 | 27-01 | POST /api/mail/assets, PNG/JPEG/GIF, max 5 MB, admin-only | ✓ SATISFIED | REST-Handler + Service-Impl + E2E-Test |
| IMG-03 | 27-04 | WYSIWYG-Editor: Drag&Drop + Toolbar-Button, `<img data-genossi-asset-id>` | ✓ SATISFIED (mit UAT-Vorbehalt) | Toolbar-Button + ondrop/ondragover-Handler + Grep-Gate-Tests |
| IMG-04 | 27-01 | GET /api/mail/assets/{id}/bytes, admin-only | ✓ SATISFIED | REST-Handler + Service CR-02 + E2E |
| IMG-05 | 27-02 | sanitize_html: nur data-genossi-asset-id erlaubt, src/data:/SVG gestrippt | ✓ SATISFIED | Ammonia-Builder + 4 neue Img-Tests + alle Phase-23/26-Tests grün |
| IMG-06 | 27-03 | Renderer: data-genossi-asset-id → cid:, multipart/related | ✓ SATISFIED | rewrite_img_cids + build_message multipart/related Branch |
| IMG-07 | 27-03 | Test-Mail-Versand unterstützt Bilder identisch | ✓ SATISFIED | InlineImageByteLoader + service.rs-Test |
| IMG-08 | 27-03 | 25-MB-Limit (base64-encoded) vor SMTP | ✓ SATISFIED | base64_encoded_len + Guard + Test (D-02-Basis) |
| IMG-09 | 27-03 | Backward-Compat: Mails ohne Bilder ohne multipart/related | ✓ SATISFIED | Leer-Slice-Pfad + alle Pre-existing-Tests grün |

Alle 9 Anforderungs-IDs aus den PLAN-Frontmatters abgedeckt. REQUIREMENTS.md bestätigt IMG-01 bis IMG-09 als Phase-27-Scope. Kein Orphan.

---

### Required Artifacts

| Artifact | Erwartet | Status | Details |
|----------|----------|--------|---------|
| `genossi_dao/src/mail_asset.rs` | MailAssetEntity + MailAssetDao-Trait | ✓ VERIFIED | Vorhanden; `bytes: Vec<u8>`, kein Auditable, automock, default all/find_by_id |
| `genossi_dao_impl_sqlite/src/mail_asset.rs` | MailAssetDaoImpl, BLOB-Roundtrip | ✓ VERIFIED | Vorhanden; BLOB-Roundtrip-, Soft-Delete-, Conflict-Tests grün |
| `genossi_service/src/mail_asset.rs` | MailAssetService-Trait + UploadMailAsset + MailAsset | ✓ VERIFIED | Vorhanden; korrekte Signaturen |
| `genossi_service_impl/src/mail_asset.rs` | MailAssetServiceImpl, admin gate, MIME sniff | ✓ VERIFIED | Vorhanden; CR-02 als erste Anweisung; sniff_image_mime; 9 Tests grün |
| `genossi_rest/src/mail_asset.rs` | upload_mail_asset + download_mail_asset_bytes | ✓ VERIFIED | Vorhanden; 415-Mapping; kein Extension-Validation |
| `genossi_rest_types/src/lib.rs::MailAssetTO` | MailAssetTO + From<&MailAsset> | ✓ VERIFIED | Vorhanden; id/filename/mime_type/size_bytes/created |
| `migrations/sqlite/20260723000000_create_mail_assets_table.sql` | mail_assets-Tabelle mit bytes BLOB | ✓ VERIFIED | Vorhanden; `bytes BLOB NOT NULL`; kein FK; kein relative_path |
| `genossi_mail/src/sanitize.rs` | Custom ammonia::Builder (OnceLock) | ✓ VERIFIED | Vorhanden; ammonia::clean() nicht mehr auf Produktionspfad |
| `genossi_mail/src/render.rs::rewrite_img_cids` | Pure Fn → (rewritten_html, Vec<AssetRef>) | ✓ VERIFIED | Vorhanden; De-dup; 6 Tests grün |
| `genossi_mail/src/send.rs::LoadedInlineImage + build_message` | multipart/related + 25-MB-Guard | ✓ VERIFIED | Vorhanden; 18 build_message-Tests grün inkl. CID-Match und Base64-Guard |
| `genossi_mail/src/service.rs::InlineImageByteLoader` | Trait object für Test-Mail-Pfad | ✓ VERIFIED | Vorhanden; with_image_loader-Builder; kein Dao-Generic auf MailServiceImpl |
| `genossi_mail/src/worker.rs` | MailAssetDao-Generic (AS), Byte-Loading | ✓ VERIFIED | Vorhanden; AS-Generic appended last; send_mail_for_recipient lädt Bytes |
| `genossi-frontend/src/api.rs::upload_mail_asset` | FormData "file" POST → MailAssetTO | ✓ VERIFIED | Vorhanden; append_with_blob_and_filename("file", …) |
| `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` | Image-Button + image_insert_html | ✓ VERIFIED | Vorhanden; onmousedown+prevent_default; insertHTML via exec_command_str |
| `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` | ondragover + ondrop | ✓ VERIFIED | Vorhanden; prevent_default FIRST; reuse image_insert_html |
| `genossi-frontend/src/i18n/{mod,de,en}.rs` | MailEditorImage + MailEditorImageUploadError | ✓ VERIFIED | Alle drei Dateien; beide Locales im selben Commit |

---

### Key Link Verification

| Von | Nach | Via | Status | Details |
|-----|------|-----|--------|---------|
| `genossi_bin/src/lib.rs` | `MailAssetServiceImpl` | `mail_asset_dao: Arc<MailAssetDao>` | ✓ WIRED | DI vollständig; `RestStateImpl` hat `mail_asset_service`-Feld; `fn mail_asset_service()` implementiert |
| `genossi_rest/src/lib.rs` | `mail_asset::generate_route()` | `nest("/api/mail/assets", …)` | ✓ WIRED | Route auf Zeile ~669; OpenAPI-Nest auf Zeile ~300 |
| `upload_mail_asset` REST | `MailAssetService::upload` | `rest_state.mail_asset_service().upload(...)` | ✓ WIRED | Vorhanden in `genossi_rest/src/mail_asset.rs:113` |
| `download_mail_asset_bytes` REST | `MailAssetService::download` | `rest_state.mail_asset_service().download(...)` | ✓ WIRED | Vorhanden in `genossi_rest/src/mail_asset.rs:175` |
| `worker.rs::send_mail_for_recipient` | `rewrite_img_cids` + `build_message` | `let (rewritten_html, inline_images) = …; build_message(…, &inline_images, …)` | ✓ WIRED | Worker Zeilen ~704–767 |
| `service.rs::send_test_mail_with_body` | `rewrite_img_cids` + `image_loader` | `rewrite_img_cids(html)` + `image_loader.load_bytes(asset_id)` | ✓ WIRED | Service Zeilen ~573–580 |
| `genossi-frontend/src/api.rs` | `POST /api/mail/assets` | `upload_mail_asset(config, file)` → Backend 27-01 | ✓ WIRED | URL `format!("{}/api/mail/assets", config.backend)`; Parsing zu `MailAssetTO` |
| `wysiwyg_toolbar.rs::image-button` | `upload_mail_asset` + `insertHTML` | `exec_command_str(&doc, "insertHTML", &img_html)` | ✓ WIRED | Zeile ~339–341 |
| `wysiwyg_editor.rs::ondrop` | `upload_mail_asset` + `image_insert_html` | Drop-Handler lädt via spawn + insertHTML | ✓ WIRED | Zeile ~118–132 |

---

### Behavioral Spot-Checks

| Verhalten | Befehl | Ergebnis | Status |
|-----------|--------|----------|--------|
| DAO BLOB-Roundtrip + Soft-Delete + Optimistic-Lock | `cargo test -p genossi_dao_impl_sqlite mail_asset` | 3 passed | ✓ PASS |
| Service: CR-02 zero-side-effects + MIME-Sniff + 5-MB-Limit | `cargo test -p genossi_service_impl mail_asset` | 9 passed | ✓ PASS |
| Ammonia-Hardening: asset-id überlebt, src/svg/data: gestrippt + Phase-23/26-Rückwärtscompat | `cargo test -p genossi_mail sanitize` | 16 passed | ✓ PASS |
| rewrite_img_cids: De-dup, Distinct-IDs, No-Image-Backward-Compat | `cargo test -p genossi_mail rewrite_img_cids` | 6 passed | ✓ PASS |
| build_message: related-Struktur + CID-Match + Base64-Guard + Backward-Compat | `cargo test -p genossi_mail build_message` | 18 passed | ✓ PASS |
| IMG-07: Test-Mail lädt Asset-Bytes wenn HTML Bild enthält | `cargo test -p genossi_mail send_test_mail_with_body_loads_asset` | 1 passed | ✓ PASS |
| plain_from_html: kein cid:/img-Leak in Text-Part | `cargo test -p genossi_mail plain_from_html_strips_image` | 1 passed | ✓ PASS |
| E2E: PNG-Upload + /bytes-Roundtrip (201 + id; 200 + Content-Type + Bytes) | `cargo test --test e2e_tests mail_asset` | 2 passed | ✓ PASS |
| Toolbar-Grep-Gate: Alle Buttons haben onmousedown+prevent_default | Frontend `cargo test wysiwyg_toolbar` | 3 passed | ✓ PASS |
| Editor-Grep-Gate: ondrop + prevent_default vorhanden | Frontend `cargo test wysiwyg_editor` | 11 passed | ✓ PASS |
| genossi_mail gesamt (keine Regressions) | `cargo test -p genossi_mail` | 279 passed, 0 failed | ✓ PASS |
| Workspace gesamt | `cargo test --workspace` | 310 passed, 2 pre-existing failed (deferred, nicht Phase-27) | ✓ PASS (Vorbehalt) |

---

### Pre-existing Failures (nicht Phase 27)

Zwei vorbestehende E2E-Test-Failures wurden im Workspace-Run identifiziert:
- `preview_body_html_round_trips_to_response` — Markdown `**bold**`-Leak in `plain_from_html`
- `test_mail_preview_repayment_no_entries_does_not_default_to_one` — mail-preview/repayment-Aggregation

Diese Fehler existieren auf dem untouched Baseline-Commit `6268e10` (vor Phase 27). Sie sind in `deferred-items.md` dokumentiert und gehören zur Mail-Render-Oberfläche, nicht zum Bild-Support. Sie werden NICHT gegen Phase 27 gewertet.

---

### Anti-Patterns

Keine blockierenden Anti-Patterns gefunden:

- Kein `TBD`, `FIXME` oder `XXX` in den von Phase 27 modifizierten Dateien.
- Kein `DocumentStorage`, `relative_path` oder `.save()` in mail_asset-Dateien (Divergence-Flag respektiert).
- `ammonia::clean()` nicht mehr auf dem Produktionspfad (nur noch in Kommentaren erlaubt).
- Kein `impl Auditable` in `genossi_dao/src/mail_asset.rs` (IMG-01-Anforderung erfüllt).
- `MailAssetDao` kein Generic auf `MailServiceImpl` (RESEARCH Anti-Pattern respektiert; nur Worker-Generic + boxed Trait-Object).

---

### Human Verification Required

#### 1. Bild über Toolbar-Button einfügen (Browser UAT)

**Test:** Vorstand loggt sich ein, öffnet den WYSIWYG-Editor für eine Mail-Vorlage oder Test-Mail-Compose, klickt den neuen "Bild einfügen"-Button, wählt eine PNG/JPEG/GIF-Datei aus.

**Expected:** Datei-Picker öffnet sich; nach Auswahl wird die Datei an `/api/mail/assets` hochgeladen; bei Erfolg erscheint das Bild sofort im Editor sichtbar (via `/api/mail/assets/{id}/bytes`-src).

**Why human:** Browser-WASM-file-picker-Verhalten, exec_command("insertHTML") und das visuelle Rendern des `<img>`-Tags im contenteditable-Editor sind per Unit-Test nicht verifizierbar.

#### 2. Drag&Drop eines Bildes auf den Editor (Browser UAT)

**Test:** Vorstand zieht ein PNG/JPEG/GIF-Bild aus dem Datei-Manager auf den WYSIWYG-Editor-Bereich.

**Expected:** Browser navigiert NICHT zur Bild-Datei; Bild wird hochgeladen und identisch zum Toolbar-Pfad eingebettet; kein Seiten-Reload.

**Why human:** DragEvent-Dispatch, DataTransfer-API und der prevent_default-Effekt auf Browser-Navigation lassen sich nur im echten Browser-WASM-Kontext validieren.

#### 3. Empfänger sieht Inline-Bild in echtem Mail-Client (Vorstand Smoke-Test)

**Test:** Vorstand verfasst eine Mail mit eingebettetem Bild über den Editor, versendet sie (entweder via Job-Send oder Test-Mail-Pfad), öffnet den Posteingang in Thunderbird oder Outlook.

**Expected:** Bild erscheint inline in der empfangenen Mail — kein kaputtes Bild-Icon, kein externer Link, korrekte CID-Auflösung.

**Why human:** Echter SMTP-Transport, Thunderbird/Outlook CID-Rendering und end-to-end Bild-Sichtbarkeit beim Empfänger sind per automatisiertem Test nicht prüfbar.

#### 4. Test-Mail an Vorstand selbst mit Inline-Bild

**Test:** Vorstand klickt "Test-Mail senden" auf einer Vorlage, die ein `<img data-genossi-asset-id>` enthält.

**Expected:** Vorstand empfängt die Test-Mail mit dem Inline-Bild sichtbar.

**Why human:** Wie oben — echte SMTP-Zustellung + Posteingang-Sichtbarkeit.

---

## Zusammenfassung

Phase 27 liefert alle 9 geforderten Anforderungen (IMG-01 bis IMG-09) vollständig und korrekt implementiert:

- **Backend (27-01):** `mail_asset`-Entität mit Inline-BLOB-Storage, admin-gated Upload + Download, Magic-Byte-MIME-Sniff, CR-02-Permission-First-Ordering — vollständig über DAO/Service/REST/DI gewired und durch DAO-Roundtrip-, Service-Unit-, und E2E-Tests abgedeckt.
- **Sanitize-Hardening (27-02):** `sanitize_html` nutzt jetzt einen gecachten Custom-`ammonia::Builder`; nur `data-genossi-asset-id` überlebt auf `<img>`; alle Phase-23/26-Garantien bleiben erhalten.
- **CID-Renderer + Send-Pfad (27-03):** `rewrite_img_cids` (reine Funktion), erweitertes `build_message` mit multipart/related-Branch, 25-MB-Base64-Guard (D-02-Basis), Asset-Byte-Loading in Worker + Test-Mail-Service — alle Tests grün, keine Mail-Regressions, Backward-Compat IMG-09 bewiesen.
- **Frontend (27-04):** `upload_mail_asset`-API-Client, Image-Toolbar-Button (onmousedown-Grep-Gate grün), ondrop/ondragover-Handler, `image_insert_html`-Helfer, i18n in beiden Locales — alle Frontend-Tests grün.

Zwei vorbestehende E2E-Test-Failures (dokumentiert in `deferred-items.md`) sind nicht Bestandteil von Phase 27 und werden nicht angerechnet.

Die vier Human-Verification-Items betreffen ausschliesslich Browser-UAT und echten SMTP-Transport — sie entsprechen dem projektüblichen "Vorstand Smoke Session"-Muster.

---

_Verified: 2026-07-23T14:00:00Z_
_Verifier: Claude (gsd-verifier)_
