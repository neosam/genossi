---
status: complete
phase: 19-e-mail-anhaenge-anzeigen
source: [19-01-SUMMARY.md, 19-02-service-and-imap-SUMMARY.md, 19-03-rest-endpoints-SUMMARY.md, 19-04-SUMMARY.md, 19-05-SUMMARY.md, 19-06-SUMMARY.md, 19-07-SUMMARY.md]
started: 2026-06-09T04:34:15Z
updated: 2026-06-09T04:35:30Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Server frisch gestartet (laufende Instanz beenden, temp-State raeumen). Bootet ohne Fehler, alle Migrations laufen durch (inkl. inbound_mail_attachments-Tabelle), der One-Shot-Backfill-Worker startet nach dem Inbox-Worker, und Inbox-Liste/Health-Check liefert Daten.
result: pass

### 2. Anhang erscheint in der Inbox-Detailansicht
expected: Eine eingehende Mail mit Anhang wird in der Vorstands-Inbox geoeffnet. Im Detail-Bereich erscheint unter dem Mail-Text eine Anhangsliste mit Dateiname und formatierter Groesse (z.B. "1.4 MB"). Die Liste scrollt mit dem Body, nicht im fixierten Header.
result: pass

### 3. Anhang herunterladen
expected: Klick auf den Download-Link eines Anhangs laedt die Datei herunter (Content-Disposition: attachment, korrekter Dateiname). Datei oeffnet sich danach korrekt im jeweiligen Programm.
result: pass

### 4. Anhang inline ansehen
expected: Bei Bild/PDF-Anhang oeffnet der Vorschau-/Inline-Link den Anhang im neuen Tab (target=_blank, Inline-Disposition), ohne Download-Zwang. Bild bzw. PDF wird im Browser angezeigt.
result: pass

### 5. Uebergrosser Anhang (>10 MB)
expected: Ein Anhang ueber 10 MB wird in der Liste sichtbar (Dateiname + Groesse), ist aber als nicht-herunterladbar markiert (Oversized-Hinweis). Download-Versuch liefert kein Datei-Bytes (410 GONE / deaktivierte Aktion) — der Server allokiert die Bytes nicht (Memory-DoS-Schutz).
result: pass

### 6. Bestandsmails (Legacy-Backfill)
expected: Mails, die VOR dem Feature ankamen, zeigen nach dem Backfill ihre Anhaenge regulaer an. Falls die Bytes per IMAP nicht mehr wiederherstellbar sind (UIDVALIDITY-Drift), erscheint stattdessen ein Legacy-Hinweis statt eines kaputten Links — kein Crash, kein leerer Divider.
result: pass

## Summary

total: 6
passed: 6
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none — Verifikation per User-Attestierung am 2026-06-09: Feature ist produktiv deployed und laeuft laut Nutzer fehlerfrei. Keine Tests in dieser Session ausgefuehrt; Disk-VERIFICATION.md war bereits status: passed (14/14). Pass-Markierungen beruhen auf der Produktiv-Bestaetigung des Nutzers, nicht auf einer erneuten Testausfuehrung.]
