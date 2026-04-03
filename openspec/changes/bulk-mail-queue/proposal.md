## Why

Bulk-Mails an 600+ Mitglieder werden aktuell synchron im HTTP-Request verarbeitet. Das führt zu HTTP-Timeouts, keinem Fortschritts-Feedback, und ist nicht restart-sicher. Außerdem ist das flache `sent_mails`-Modell (eine Zeile pro Empfänger) im Frontend unübersichtlich, da zusammengehörige Mails nicht gruppiert sind. IONOS Basic Mail erlaubt ~100 Mails/Stunde im sicheren Betrieb, was bei 600 Empfängern ~6 Stunden Versanddauer bedeutet.

## What Changes

- **Neues Datenmodell**: `mail_job` (ein Mailing) + `mail_recipient` (je Empfänger) ersetzt die flache `sent_mails`-Tabelle
- **DB-basierte Mail-Queue**: Jobs werden als "pending" in der DB gespeichert, HTTP-Request kehrt sofort zurück (202 Accepted)
- **Background-Worker**: Tokio-Task beim Server-Start, pollt DB nach offenen Recipients, sendet mit konfigurierbarem Intervall (Default: 36s ≈ 100/Stunde)
- **Restart-Sicherheit**: Worker ist zustandslos, nimmt nach Neustart offene Jobs wieder auf
- **Konfigurierbares Sende-Intervall**: `mail_send_interval_seconds` im Config-Store
- **BREAKING**: `sent_mails`-Tabelle und zugehörige Endpoints werden entfernt
- **BREAKING**: `POST /api/mail/send` und `POST /api/mail/send-bulk` Response-Format ändert sich (gibt Mail-Job statt SentMail zurück)
- Test-Mail (`POST /api/mail/test`) bleibt synchron und direkt — kein Job

## Capabilities

### New Capabilities
- `mail-queue`: Background-Worker für asynchronen, restart-sicheren Mail-Versand mit konfigurierbarem Intervall

### Modified Capabilities
- `mail-sending`: Datenmodell wechselt von flacher `sent_mails`-Tabelle zu `mail_job` + `mail_recipient`. Bulk-Endpoint gibt sofort zurück statt synchron zu senden. Einzelversand wird ebenfalls über Job-Queue abgewickelt.

## Impact

- **Datenbank**: Migration löscht `sent_mails`, erstellt `mail_jobs` und `mail_recipients`
- **Backend-Crates**: `genossi_mail` (Service + DAO), `genossi_dao` (neue Traits), `genossi_dao_impl_sqlite` (neue Implementierungen), `genossi_rest` (geänderte Endpoints), `genossi_bin` (Worker-Start)
- **Frontend**: Mail-Seite muss auf Job-basierte Anzeige umgebaut werden (Fortschrittsbalken, aufklappbare Empfängerliste)
- **REST-API**: Geänderte Response-Typen für Send-Endpoints, neuer Endpoint für Job-Status/Fortschritt
- **Config-Store**: Neuer Key `mail_send_interval_seconds`
