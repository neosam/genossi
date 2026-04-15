## Context

Mail-Templates werden aktuell als hardcoded Strings (`TEMPLATE_FORMAL`, `TEMPLATE_INFORMAL`) in `genossi-frontend/src/component/mail_compose/template_selector.rs` gehalten. Der Backend-Change `mail-template-crud` stellt eine REST-API unter `/api/mail/templates` bereit (CRUD). Dieses Frontend muss die API anbinden und eine Verwaltungsseite sowie einen dynamischen Template-Selector liefern.

Bestehende Patterns im Frontend:
- **List-Detail-Layout**: Wird für Typst-Templates (`templates.rs`) verwendet — Liste links, Editor rechts
- **CRUD-Seiten**: `StaticDocumentsPage` zeigt ein einfacheres Tabellen-Pattern
- **Mail-Compose-Komponenten**: Bereits in `component/mail_compose/` extrahiert (TemplateSelector, TemplateVarButtons, MailBodyEditor, etc.)
- **Navigation**: TopBar verwendet `NavGroup`-Komponenten mit Dropdown-Gruppen (Mitglieder, Kommunikation, Verwaltung)
- **i18n**: Alle UI-Texte über `Key`-Enum in `src/i18n/mod.rs`

## Goals / Non-Goals

**Goals:**
- Eigene Seite `/mail/templates` zum Erstellen, Bearbeiten und Löschen von Mail-Templates
- `TemplateSelector` auf der Mail-Compose-Seite lädt Templates dynamisch aus der API
- Navigation zur neuen Seite über TopBar (Kommunikation-Gruppe) und über "Vorlagen verwalten"-Link im TemplateSelector
- Wiederverwendung bestehender Komponenten (`TemplateVarButtons`, `MailBodyEditor`)

**Non-Goals:**
- Template-Preview mit gerenderten Mitgliederdaten auf der Verwaltungsseite (das bleibt auf der Mail-Compose-Seite)
- Inline-Bearbeitung von Templates auf der Mail-Compose-Seite
- Import/Export von Templates
- Template-Versionierung oder History

## Decisions

### 1. List-Detail-Layout für die Verwaltungsseite

Die Verwaltungsseite verwendet ein zweispaltiges Layout: Template-Liste links, Editor rechts. Beim Klick auf ein Template wird es rechts zum Bearbeiten geöffnet. "Neu erstellen" öffnet einen leeren Editor.

**Warum**: Mail-Templates haben einen mehrzeiligen Body, der nicht in eine Tabellenzeile passt. Das List-Detail-Pattern ist bereits bei der Typst-Template-Seite im Projekt etabliert.

**Alternative**: Tabellen-Layout wie StaticDocumentsPage — abgelehnt, weil der Body-Editor zu viel Platz braucht.

### 2. Eigene Seite statt Modal

Die Template-Verwaltung bekommt eine eigene Route `/mail/templates` statt eines Modals auf der Mail-Compose-Seite.

**Warum**: Mehr Platz für den Editor, saubere URL-Struktur, konsistent mit dem Muster anderer Verwaltungsseiten. Vom User bestätigt.

### 3. TemplateSelector: API-Anbindung mit lokalem Cache

Der `TemplateSelector` auf der Mail-Compose-Seite lädt Templates einmalig beim Mounten per `GET /api/mail/templates` und cached sie im lokalen Signal-State. Kein globaler State nötig.

**Warum**: Templates ändern sich selten während einer Session. Ein einfacher `use_effect` + Signal reicht aus. Globaler State wäre Overengineering.

**Alternative**: Globaler State wie `MEMBERS` — abgelehnt, da Templates nicht von mehreren Seiten gleichzeitig gelesen werden müssen.

### 4. Navigation: Kommunikation-Gruppe + Inline-Link

- TopBar: Neuer Eintrag "Mail-Vorlagen" in der Kommunikation-Gruppe (neben "Mail" und "Posteingang")
- Mail-Compose-Seite: Kleiner "Vorlagen verwalten"-Link unter dem TemplateSelector-Dropdown

**Warum**: Beides wurde vom User gewünscht. Die Kommunikation-Gruppe ist der logische Ort, da Mail-Templates thematisch zu Mail gehören.

### 5. Wiederverwendung von MailBodyEditor und TemplateVarButtons

Der Editor auf der Verwaltungsseite verwendet die bestehenden `MailBodyEditor`- und `TemplateVarButtons`-Komponenten aus `component/mail_compose/`.

**Warum**: Konsistentes Verhalten und Aussehen. Die Komponenten sind bereits als wiederverwendbar extrahiert.

### 6. API-Typen

Neue Typen in `api.rs`:
- `MailTemplateTO` (id, name, subject, body, version)
- API-Funktionen: `list_mail_templates()`, `get_mail_template()`, `create_mail_template()`, `update_mail_template()`, `delete_mail_template()`

**Warum**: Folgt dem bestehenden Pattern in `api.rs` für andere Entitäten.

## Risks / Trade-offs

- **Race Condition beim Bearbeiten**: Wenn zwei Admins gleichzeitig dasselbe Template bearbeiten, gewinnt der letzte Save. → Mitigation: Version-Feld wird beim Update mitgeschickt, Backend lehnt veraltete Versionen ab (409 Conflict). Frontend zeigt Fehlermeldung.
- **Hardcoded Templates entfernen**: Nach der Umstellung fehlen die Templates, wenn die Backend-Migration nicht gelaufen ist. → Mitigation: Change hat explizite Abhängigkeit auf `mail-template-crud`. Die Migration seeded die Formal/Informal-Templates.
- **Leerer Zustand**: Wenn alle Templates gelöscht werden, ist der TemplateSelector leer. → Kein Problem: Der Selector zeigt dann nur "Vorlage wählen..." ohne Optionen. Benutzer können direkt tippen.
