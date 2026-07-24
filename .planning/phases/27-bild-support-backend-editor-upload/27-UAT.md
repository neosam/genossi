---
status: testing
phase: 27-bild-support-backend-editor-upload
source: [27-VERIFICATION.md]
started: 2026-07-23T14:10:00Z
updated: 2026-07-23T15:05:00Z
---

## Current Test

number: 3
name: Empfänger sieht Inline-Bild in echtem Mail-Client (Vorstand Smoke-Test)
expected: |
  Bild erscheint inline in der empfangenen Mail — kein kaputtes Bild-Icon,
  kein externer Link, korrekte CID-Auflösung (multipart/related).
awaiting: user response (blockiert bis Preview-Fix, siehe Gaps)

## Tests

### 1. Bild über Toolbar-Button einfügen (Browser UAT)

- **Test:** Vorstand loggt sich ein, öffnet den WYSIWYG-Editor, klickt den "Bild einfügen"-Button, wählt eine PNG/JPEG/GIF-Datei aus.
- **Expected:** Datei-Picker öffnet sich; nach Auswahl wird die Datei an `/api/mail/assets` hochgeladen; bei Erfolg erscheint das Bild sofort sichtbar im Editor (via `/api/mail/assets/{id}/bytes`-src).
- **Why human:** Browser-WASM-file-picker-Verhalten, `insertHTML` und visuelles Rendern des `<img>` im contenteditable sind per Unit-Test nicht verifizierbar.
- **Status:** issue
- **reported:** "Upload liefert Asset-ID, aber der Bild-Preview via GET /api/mail/assets/{id}/bytes gibt 404 Not Found zurück (auf genossi-beta). Bild erscheint nicht im Editor."
- **severity:** major

### 2. Drag&Drop eines Bildes auf den Editor (Browser UAT)

- **Test:** Vorstand zieht ein PNG/JPEG/GIF-Bild aus dem Datei-Manager auf den WYSIWYG-Editor-Bereich.
- **Expected:** Browser navigiert NICHT zur Bild-Datei; Bild wird hochgeladen und identisch zum Toolbar-Pfad eingebettet; kein Seiten-Reload.
- **Why human:** DragEvent-Dispatch, DataTransfer-API und der prevent_default-Effekt lassen sich nur im echten Browser-WASM-Kontext validieren.
- **Status:** issue
- **reported:** "Drag&Drop reagiert gar nicht (ondrop feuert nicht). Vom Nutzer vorerst zurückgestellt."
- **severity:** minor
- **deferred:** vom Nutzer vorerst ignoriert; separates Problem vom Preview-404.

### 3. Empfänger sieht Inline-Bild in echtem Mail-Client (Vorstand Smoke-Test)

- **Test:** Vorstand verfasst eine Mail mit eingebettetem Bild, versendet sie (Job-Send oder Test-Mail-Pfad), öffnet den Posteingang in Thunderbird oder Outlook.
- **Expected:** Bild erscheint inline in der empfangenen Mail — kein kaputtes Bild-Icon, kein externer Link, korrekte CID-Auflösung (`multipart/related`).
- **Why human:** Echter SMTP-Transport + Client-CID-Rendering (Thunderbird/Outlook) ist per automatisiertem Test nicht prüfbar.
- **Status:** pending

### 4. Test-Mail an Vorstand selbst mit Inline-Bild

- **Test:** Vorstand klickt "Test-Mail senden" auf einer Vorlage, die ein `<img data-genossi-asset-id>` enthält.
- **Expected:** Empfangene Test-Mail enthält das Bild sichtbar inline.
- **Why human:** Echter SMTP-Transport + Posteingang-Validierung ist per Unit-Test nicht prüfbar.
- **Status:** pending

## Summary

total: 4
passed: 0
issues: 2
pending: 2
skipped: 0

## Gaps

- truth: "Nach Toolbar-Upload erscheint das Bild sofort sichtbar im Editor via /bytes-Preview-URL."
  status: failed
  reason: "User reported: Upload liefert Asset-ID, aber GET /api/mail/assets/{id}/bytes gibt 404 (genossi-beta)."
  severity: major
  test: 1
  root_cause: "image_insert_html() (genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs:38) erzeugt eine RELATIVE Preview-src '/api/mail/assets/{id}/bytes'. Alle funktionierenden API-Aufrufe bauen dagegen '{config.backend}/api/...'. Auf beta ist config.backend='https://genossi-beta.nebenan-unverpackt.de/api', d.h. die echten API-Calls laufen über '.../api/api/...' (der Reverse-Proxy mappt das auf den Backend-Router). Die relative Preview-URL umgeht config.backend und landet auf einem Pfad, den der Proxy auf eine nicht existierende Backend-Route abbildet → 404. Bestätigt: /api/api/... liefert 401 (Route existiert, Auth), /api/mail/assets/{id}/bytes liefert authentifiziert 404 (EntityNotFound-Mapping)."
  artifacts:
    - path: "genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs"
      issue: "image_insert_html() emittiert relative Preview-src statt config.backend-basierter URL"
    - path: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs"
      issue: "Drop-Handler-Caller muss backend-Basis an image_insert_html durchreichen"
  missing:
    - "image_insert_html(backend, id) → src = '{backend}/api/mail/assets/{id}/bytes' (konsistent mit allen anderen API-Calls)"
    - "Beide Caller (Toolbar-Button + Editor-Drop) übergeben &config.backend"
    - "Unit-Test: Preview-src verwendet config.backend-Basis (Regression-Guard für diesen 404)"

- truth: "Drag&Drop eines Bildes auf den Editor lädt hoch und bettet ein (kein Seiten-Reload)."
  status: failed
  reason: "User reported: ondrop feuert nicht; Drag&Drop reagiert gar nicht. Vom Nutzer vorerst zurückgestellt."
  severity: minor
  test: 2
  deferred: true

## Notes

- Alle 9 Must-Have-Truths (IMG-01..09) wurden **automatisiert** verifiziert (9/9), siehe `27-VERIFICATION.md` — der 404 ist ein **Deployment-/URL-Konstruktions-Bug im Frontend-Preview-Pfad**, keine Backend-Regression (e2e-Roundtrip `test_mail_asset_upload_and_bytes_roundtrip` grün).
- Test 3 & 4 (echte SMTP-Zustellung) bleiben pending; Test 3 ist bis zum Preview-Fix ohnehin schwer sinnvoll zu prüfen (Bild muss erst im Editor sichtbar sein).
- Test 2 (Drag&Drop) vom Nutzer vorerst zurückgestellt — separates Problem (`ondrop` feuert nicht).
