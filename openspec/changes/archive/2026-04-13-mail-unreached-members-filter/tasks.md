## 1. Mail-Service: Reached-Member-IDs Methode

- [x] 1.1 Methode `get_reached_member_ids(job_id: Uuid)` zum `MailService`-Trait hinzufügen, die alle Member-IDs mit status "sent" für einen Job zurückgibt
- [x] 1.2 Implementierung in `MailServiceImpl` mit Query über `mail_recipients`
- [x] 1.3 DAO-Methode `find_sent_member_ids_by_job_id(job_id: Uuid)` im `MailRecipientDao`-Trait und SQLite-Implementierung
- [x] 1.4 Unit-Tests für die neue Service-Methode

## 2. REST-Endpoint: Nicht erreichte Mitglieder

- [x] 2.1 Neuen Endpoint `GET /api/members/not-reached-by/{job_id}` im Member-REST-Modul registrieren
- [x] 2.2 Handler implementieren: Mail-Service für reached IDs aufrufen, Member-Service für alle aktiven Mitglieder, Differenz bilden und als `MemberTO`-Liste zurückgeben
- [x] 2.3 REST-State um Zugriff auf `MailService` erweitern (falls noch nicht vorhanden)
- [x] 2.4 OpenAPI-Dokumentation für den neuen Endpoint

## 3. E2E-Tests

- [x] 3.1 E2E-Test: Endpoint gibt nicht erreichte Mitglieder korrekt zurück (Mitglied ohne Eintrag + Mitglied mit failed)
- [x] 3.2 E2E-Test: Mitglied mit status "sent" wird ausgeschlossen
- [x] 3.3 E2E-Test: 404 bei ungültiger Job-ID

## 4. Frontend: Mail-Job-Filter in Mitgliederliste

- [x] 4.1 API-Funktion `get_members_not_reached_by(job_id)` und `get_mail_jobs()` im Frontend-API-Modul hinzufügen
- [x] 4.2 State um `mail_job_filter: Option<String>` erweitern
- [x] 4.3 Dropdown-Komponente für Mail-Job-Auswahl in die Mitgliederliste einfügen
- [x] 4.4 Bei Job-Auswahl den neuen Endpoint aufrufen und Mitgliederliste ersetzen
- [x] 4.5 i18n-Strings für den neuen Filter (DE + EN)
