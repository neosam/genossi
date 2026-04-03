## 1. Mail-Service: Reached-Member-IDs Methode

- [ ] 1.1 Methode `get_reached_member_ids(job_id: Uuid)` zum `MailService`-Trait hinzufügen, die alle Member-IDs mit status "sent" für einen Job zurückgibt
- [ ] 1.2 Implementierung in `MailServiceImpl` mit Query über `mail_recipients`
- [ ] 1.3 DAO-Methode `find_sent_member_ids_by_job_id(job_id: Uuid)` im `MailRecipientDao`-Trait und SQLite-Implementierung
- [ ] 1.4 Unit-Tests für die neue Service-Methode

## 2. REST-Endpoint: Nicht erreichte Mitglieder

- [ ] 2.1 Neuen Endpoint `GET /api/members/not-reached-by/{job_id}` im Member-REST-Modul registrieren
- [ ] 2.2 Handler implementieren: Mail-Service für reached IDs aufrufen, Member-Service für alle aktiven Mitglieder, Differenz bilden und als `MemberTO`-Liste zurückgeben
- [ ] 2.3 REST-State um Zugriff auf `MailService` erweitern (falls noch nicht vorhanden)
- [ ] 2.4 OpenAPI-Dokumentation für den neuen Endpoint

## 3. E2E-Tests

- [ ] 3.1 E2E-Test: Endpoint gibt nicht erreichte Mitglieder korrekt zurück (Mitglied ohne Eintrag + Mitglied mit failed)
- [ ] 3.2 E2E-Test: Mitglied mit status "sent" wird ausgeschlossen
- [ ] 3.3 E2E-Test: 404 bei ungültiger Job-ID

## 4. Frontend: Mail-Job-Filter in Mitgliederliste

- [ ] 4.1 API-Funktion `get_members_not_reached_by(job_id)` und `get_mail_jobs()` im Frontend-API-Modul hinzufügen
- [ ] 4.2 State um `mail_job_filter: Option<String>` erweitern
- [ ] 4.3 Dropdown-Komponente für Mail-Job-Auswahl in die Mitgliederliste einfügen
- [ ] 4.4 Bei Job-Auswahl den neuen Endpoint aufrufen und Mitgliederliste ersetzen
- [ ] 4.5 i18n-Strings für den neuen Filter (DE + EN)
