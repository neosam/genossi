## Why

Der `PUT /api/templates/{path}`-Endpoint akzeptiert nur Plain-Text-Bodies. Binärdateien wie Bilder (PNG, JPG, SVG) können nicht über die API in das Template-Verzeichnis hochgeladen werden. Benutzer brauchen diese Dateien z.B. für Firmenlogos in Typst-Briefvorlagen.

## What Changes

- Der bestehende `PUT /api/templates/{path}`-Endpoint wird erweitert, um sowohl Text- als auch Binärdateien zu akzeptieren
- Erkennung anhand des `Content-Type`-Headers: `text/*` wird als Text behandelt, alles andere als Binärdaten
- Das Frontend bekommt einen Datei-Upload-Button im Template-Editor

## Capabilities

### New Capabilities

Keine neuen Capabilities nötig — die Änderung erweitert bestehende Funktionalität.

### Modified Capabilities

- `document-templates`: Der Write-Endpoint wird erweitert, um Binärdateien (Bilder etc.) neben Textdateien zu unterstützen

## Impact

- **Code**: `genossi_rest/src/template.rs` (Endpoint-Handler), `genossi_service/src/template.rs` (Service-Trait), Service-Implementierung
- **API**: `PUT /api/templates/{path}` akzeptiert zusätzlich `application/octet-stream` und Bild-Content-Types
- **Frontend**: Template-Seite braucht einen Upload-Button für Dateien
