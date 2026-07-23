---
status: testing
phase: 27-bild-support-backend-editor-upload
source: [27-VERIFICATION.md]
started: 2026-07-23T14:10:00Z
updated: 2026-07-23T14:10:00Z
---

## Current Test

number: 1
name: Bild über Toolbar-Button einfügen und sofortiger Bild-Preview
expected: |
  Vorstand klickt den Toolbar-Button, wählt eine PNG/JPEG/GIF-Datei aus; nach dem
  Upload an /api/mail/assets erscheint das Bild sofort im Editor via
  /api/mail/assets/{id}/bytes src.
awaiting: user response

## Tests

### 1. Bild über Toolbar-Button einfügen (Browser UAT)

- **Test:** Vorstand loggt sich ein, öffnet den WYSIWYG-Editor, klickt den "Bild einfügen"-Button, wählt eine PNG/JPEG/GIF-Datei aus.
- **Expected:** Datei-Picker öffnet sich; nach Auswahl wird die Datei an `/api/mail/assets` hochgeladen; bei Erfolg erscheint das Bild sofort sichtbar im Editor (via `/api/mail/assets/{id}/bytes`-src).
- **Why human:** Browser-WASM-file-picker-Verhalten, `insertHTML` und visuelles Rendern des `<img>` im contenteditable sind per Unit-Test nicht verifizierbar.
- **Status:** pending

### 2. Drag&Drop eines Bildes auf den Editor (Browser UAT)

- **Test:** Vorstand zieht ein PNG/JPEG/GIF-Bild aus dem Datei-Manager auf den WYSIWYG-Editor-Bereich.
- **Expected:** Browser navigiert NICHT zur Bild-Datei; Bild wird hochgeladen und identisch zum Toolbar-Pfad eingebettet; kein Seiten-Reload.
- **Why human:** DragEvent-Dispatch, DataTransfer-API und der prevent_default-Effekt lassen sich nur im echten Browser-WASM-Kontext validieren.
- **Status:** pending

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

## Notes

- Alle 9 Must-Have-Truths (IMG-01..09) wurden **automatisiert** verifiziert (9/9), siehe `27-VERIFICATION.md`.
- Diese 4 UAT-Items sind genuin nicht-automatisierbar (Browser-WASM + echte SMTP-Zustellung).
- Konsistent mit dem v1.5-Muster (Phase 26 UAT ebenfalls deferred): abzuarbeiten in der Vorstands-Smoke-Session vor dem v1.5-Milestone-Close.
