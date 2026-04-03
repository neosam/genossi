## Why

Genossenschaftsvorstände müssen regelmäßig Dokumente wie Beitrittsbestätigungen, Austrittserklärungen oder Anteilserhöhungen erstellen. Aktuell geschieht das manuell außerhalb der Software. Durch Template-basierte PDF-Generierung mit Typst können diese Dokumente direkt aus Genossi heraus generiert werden — mit den korrekten Mitgliedsdaten, konsistentem Layout und ohne manuellen Aufwand.

## What Changes

- Typst als Rust-Library (Crate) einbinden zur PDF-Generierung
- Template-Verwaltung im Dateisystem unter einem konfigurierbaren `TEMPLATE_PATH`
- REST-API-Endpunkte zum Auflisten, Lesen, Erstellen, Bearbeiten und Löschen von Typst-Templates (Dateien und Ordner)
- REST-API-Endpunkt zum Rendern eines Templates mit Mitgliedsdaten zu PDF
- Default-Templates (z.B. `join_confirmation.typ`, `_layout.typ`) via `include_bytes!` in die Binary eingebettet, werden beim Start pro Datei angelegt falls nicht vorhanden
- Frontend: Template-Verwaltungsseite mit Dateibaum-Navigation, CodeMirror-Editor, Vorschau-Funktion, sowie Möglichkeit neue Dateien und Ordner anzulegen
- Frontend: Button auf der Mitglieds-Detailseite zum Generieren von Dokumenten aus Templates

## Capabilities

### New Capabilities
- `document-templates`: Verwaltung von Typst-Templates im Dateisystem (CRUD für Dateien/Ordner, Dateibaum-API, Default-Template-Provisioning)
- `pdf-generation`: Rendern von Typst-Templates mit Mitgliedsdaten zu PDF (Typst-Engine-Integration, Platzhalter-Auflösung, `#import`-Support)
- `template-editor`: Frontend-Seite zur Template-Verwaltung mit CodeMirror-Editor, Dateibaum, Vorschau und Datei-/Ordnererstellung

### Modified Capabilities
- `member-documents`: Generierte PDFs können optional als Member Document gespeichert werden (spätere Erweiterung, nicht im initialen Scope)

## Impact

- **Neue Dependencies**: `typst` Crate (oder `typst-library`), ggf. Font-Dateien im Repo
- **Frontend**: Neue Seite für Template-Verwaltung, CodeMirror-Integration, neuer Button auf Member-Details
- **API**: Neue Endpunkte unter `/templates/` und `/templates/{path..}/render/{member_id}`
- **Konfiguration**: Neue Umgebungsvariable `TEMPLATE_PATH` (Default: `./templates`)
- **Deployment**: Default-Templates werden in die Binary eingebettet, keine zusätzlichen Dateien nötig
- **Dateisystem**: Schreibzugriff auf `TEMPLATE_PATH` erforderlich
