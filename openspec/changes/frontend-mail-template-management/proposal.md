## Why

Die Backend-API für Mail-Templates (`/api/mail/templates`) wird durch den Change `mail-template-crud` bereitgestellt. Ohne ein Frontend können Benutzer die Templates aber nur über Swagger verwalten. Außerdem verwendet der bestehende `TemplateSelector` im Mail-Compose-Formular hardcoded Templates statt der gespeicherten. Benutzer brauchen eine eigene Verwaltungsseite und einen dynamischen Template-Selector, um Mail-Vorlagen erstellen, bearbeiten, löschen und beim Versand auswählen zu können.

## What Changes

- Neue Seite `/mail/templates` mit List-Detail-Layout: Template-Liste links, Editor rechts (Name, Subject, Body)
- `TemplateVarButtons` im Editor verfügbar, damit Benutzer MiniJinja-Variablen einfügen können
- Erstellen, Bearbeiten und Löschen von Templates über die API
- `TemplateSelector`-Komponente auf API umstellen: Templates aus `GET /api/mail/templates` laden statt hardcoded `TEMPLATE_FORMAL` / `TEMPLATE_INFORMAL`
- Hardcoded Template-Konstanten aus `template_selector.rs` entfernen
- "Vorlagen verwalten"-Link im `TemplateSelector` zur neuen Seite
- Navigation: Route `/mail/templates` im Router, Link im TopBar-Menü und auf der Mail-Seite

## Capabilities

### New Capabilities
- `mail-template-management-ui`: Eigene Frontend-Seite zur Verwaltung von Mail-Templates (Erstellen, Bearbeiten, Löschen) mit List-Detail-Layout

### Modified Capabilities
- `predefined-mail-templates`: Template-Auswahl wird von hardcoded auf API-basiert umgestellt; Dropdown lädt Templates dynamisch aus der Datenbank
- `mail-compose-components`: `TemplateSelector` wird erweitert um API-Anbindung und "Vorlagen verwalten"-Link

## Impact

- **Frontend**: `genossi-frontend/src/page/` (neue Seite), `genossi-frontend/src/component/mail_compose/template_selector.rs` (Umbau), `genossi-frontend/src/api.rs` (neue API-Funktionen), `genossi-frontend/src/router.rs` (neue Route)
- **Navigation**: TopBar-Menü bekommt neuen Link, Mail-Seite bekommt "Vorlagen verwalten"-Link
- **i18n**: Neue Übersetzungs-Keys für die Verwaltungsseite
- **Voraussetzung**: Backend-Change `mail-template-crud` muss implementiert sein
