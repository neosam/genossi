## 1. API-Funktionen

- [ ] 1.1 `MailTemplateTO`-Struct in `api.rs` definieren (id, name, subject, body, version)
- [ ] 1.2 `list_mail_templates()` Funktion in `api.rs` implementieren (`GET /api/mail/templates`)
- [ ] 1.3 `create_mail_template()` Funktion in `api.rs` implementieren (`POST /api/mail/templates`)
- [ ] 1.4 `update_mail_template()` Funktion in `api.rs` implementieren (`PUT /api/mail/templates/{id}`)
- [ ] 1.5 `delete_mail_template()` Funktion in `api.rs` implementieren (`DELETE /api/mail/templates/{id}`)

## 2. Template-Verwaltungsseite

- [ ] 2.1 Neue Datei `genossi-frontend/src/page/mail_templates.rs` mit `MailTemplatesPage`-Komponente anlegen (Grundstruktur mit TopBar, RequirePrivilege, zwei Spalten)
- [ ] 2.2 Template-Liste in der linken Spalte: Templates laden und als klickbare Liste darstellen, mit "Neu erstellen"-Button
- [ ] 2.3 Editor in der rechten Spalte: Name-, Subject- und Body-Felder mit `TemplateVarButtons` für Variable-Insertion
- [ ] 2.4 Erstellen-Logik: Leeren Editor öffnen, beim Speichern `create_mail_template()` aufrufen, Liste aktualisieren
- [ ] 2.5 Bearbeiten-Logik: Template aus Liste auswählen, Editor befüllen, beim Speichern `update_mail_template()` mit Version aufrufen
- [ ] 2.6 Löschen-Logik: "Löschen"-Button im Editor mit Bestätigungs-Dialog, `delete_mail_template()` aufrufen, Editor leeren
- [ ] 2.7 Fehlerbehandlung: Fehlermeldungen für API-Fehler anzeigen (409 Duplicate Name, 409 Version Conflict, Netzwerkfehler)

## 3. TemplateSelector umbauen

- [ ] 3.1 Hardcoded `TEMPLATE_FORMAL` und `TEMPLATE_INFORMAL` Konstanten aus `template_selector.rs` entfernen
- [ ] 3.2 Templates per `list_mail_templates()` beim Mounten laden und im Signal-State cachen
- [ ] 3.3 Dropdown dynamisch aus geladenen Templates befüllen (Name als Label, Body als Value)
- [ ] 3.4 "Vorlagen verwalten"-Link unter dem Dropdown hinzufügen, der zu `/mail/templates` navigiert

## 4. Routing und Navigation

- [ ] 4.1 `MailTemplatesPage` Route in `router.rs` unter `/mail/templates` registrieren
- [ ] 4.2 `MailTemplatesPage` in `page/mod.rs` exportieren
- [ ] 4.3 "Mail-Vorlagen" Eintrag in der Kommunikation-Gruppe der TopBar hinzufügen (admin-only)

## 5. i18n

- [ ] 5.1 Neue i18n-Keys hinzufügen: `MailTemplates`, `MailTemplateCreate`, `MailTemplateSave`, `MailTemplateDelete`, `MailTemplateManage`, `MailTemplateName`, `MailTemplateSubject`, `MailTemplateBody`, `MailTemplateEmpty`
- [ ] 5.2 Deutsche Übersetzungen hinzufügen
- [ ] 5.3 Englische Übersetzungen hinzufügen
- [ ] 5.4 Tschechische Übersetzungen hinzufügen
