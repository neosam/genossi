## 1. Frontend: Mail-Compose-Komponenten extrahieren

- [x] 1.1 Erstelle `component/mail_compose/mod.rs` mit Modul-Deklarationen
- [x] 1.2 Extrahiere `MailSubjectInput` Komponente (Label + Input, Props: value, on_change)
- [x] 1.3 Extrahiere `MailBodyEditor` Komponente (Label + Textarea, Props: value, on_change)
- [x] 1.4 Extrahiere `TemplateVarButtons` Komponente (Primary/secondary vars, "Mehr"-Toggle, Props: on_insert)
- [x] 1.5 Extrahiere `TemplateSelector` Komponente (Formal/Informal Dropdown, Props: on_select)
- [x] 1.6 Extrahiere `TemplatePreview` Komponente (Member-Auswahl + API-Preview, Props: subject, body, member_ids)

## 2. Frontend: MailPage Refactoring

- [x] 2.1 Refactore `mail_page.rs` — ersetze inline-Code durch die neuen Komponenten
- [x] 2.2 Verifiziere, dass die MailPage nach Refactoring identisch funktioniert (manueller Test oder build-check)

## 3. Backend: DB-Migration und DAO-Erweiterungen

- [x] 3.1 Migration: `ALTER TABLE mail_jobs ADD COLUMN reply_to_inbound_mail_id BLOB`
- [x] 3.2 Migration: `ALTER TABLE inbound_mails ADD COLUMN message_id TEXT`
- [x] 3.3 Erweitere `MailJob` DAO-Struct um `reply_to_inbound_mail_id: Option<Uuid>`
- [x] 3.4 Erweitere `InboundMail` DAO-Struct um `message_id: Option<Arc<str>>`
- [x] 3.5 Aktualisiere SQLite DAO-Implementierungen (create/update/read) für beide neuen Felder
- [x] 3.6 Erweitere `InboundMailDao` um `find_by_id` für Worker-Zugriff auf Message-ID

## 4. Backend: Message-ID auf Inbound-Mails parsen

- [x] 4.1 Erweitere `ParsedMail` Struct um `message_id: Option<String>`
- [x] 4.2 Erweitere `parse_raw_mail()` — extrahiere und normalisiere `Message-ID` Header
- [x] 4.3 Setze `message_id` beim Erstellen von `InboundMail` im Inbox-Worker
- [x] 4.4 Tests für `parse_raw_mail` mit und ohne Message-ID Header

## 5. Backend: Reply-Endpoint und Service

- [x] 5.1 Erweitere `InboxService` Trait um `reply(id, subject, body) -> Result<MailJob>`
- [x] 5.2 Implementiere `reply` in `InboxServiceImpl` — erstelle MailJob + MailRecipient, setze Status auf `replied`
- [x] 5.3 Erstelle REST-Handler `POST /api/inbox/{id}/reply` in `inbox_rest.rs`
- [x] 5.4 Registriere Route und OpenAPI-Docs
- [x] 5.5 Unit-Tests für `InboxService::reply` (Happy Path, Not Found, bereits replied)

## 6. Backend: In-Reply-To Header im Worker

- [x] 6.1 Erweitere `send_mail_for_recipient` — prüfe `reply_to_inbound_mail_id` auf dem Job
- [x] 6.2 Lade Inbound-Mail Message-ID wenn vorhanden
- [x] 6.3 Setze `In-Reply-To` und `References` Header auf der ausgehenden lettre::Message
- [x] 6.4 Tests: Reply-Mail mit bekannter Message-ID enthält korrekte Header
- [x] 6.5 Tests: Reply-Mail ohne Message-ID sendet ohne Threading-Header

## 7. Frontend: Inbox Reply UI

- [x] 7.1 Erweitere `status_label` und `status_color` in `inbox_page.rs` um `replied`
- [x] 7.2 Füge "Antworten"-Button in der Detail-Ansicht hinzu (klappt Reply-Formular auf)
- [x] 7.3 Baue Reply-Formular mit `MailSubjectInput`, `MailBodyEditor`, `TemplateSelector`, `TemplateVarButtons` (wenn Member zugeordnet), `TemplatePreview` (wenn Member zugeordnet)
- [x] 7.4 Implementiere Reply-Absende-Logik (`POST /api/inbox/{id}/reply`)
- [x] 7.5 Nach erfolgreichem Reply: Status in Liste aktualisieren, Erfolgs-Meldung anzeigen
- [x] 7.6 Füge API-Funktion `reply_inbox_mail` in `api.rs` hinzu

## 8. E2E-Tests

- [ ] 8.1 E2E-Test: Reply auf Inbound-Mail erstellt Job mit korrektem `reply_to_inbound_mail_id`
- [ ] 8.2 E2E-Test: Inbound-Mail Status wird auf `replied` gesetzt nach Reply
