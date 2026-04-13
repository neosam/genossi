## Context

Der bestehende Backup-Export umfasst Mitglieder-CSVs, Aktionen-CSV und Dokumente (als ZIP bzw. WebDAV-Sync). Die E-Mail-Kommunikation ist in `genossi_mail` implementiert mit:
- `MailJob` + `MailRecipient` (ausgehend, mit `member_id` Zuordnung)
- `InboundMail` (eingehend, mit `assigned_member_id` Zuordnung)
- `CommunicationDao::get_member_communications()` liefert bereits eine unified Timeline pro Mitglied

Der Export muss in zwei Kontexte integriert werden:
1. REST-Endpoint `/backup/documents` — synchroner ZIP-Download
2. WebDAV-Worker — periodischer Hintergrund-Sync (append-only)

## Goals / Non-Goals

**Goals:**
- Alle zugeordneten Mails (inbound + outbound) als lesbare .txt-Dateien exportieren
- Integration in bestehende ZIP-Struktur (pro Mitglied-Ordner)
- Integration in WebDAV-Worker (append-only, keine Re-Uploads)
- Menschenlesbares Format ohne spezielle Tools

**Non-Goals:**
- Export von nicht zugeordneten Mails
- .eml Format oder maschinenlesbares Austauschformat
- Export von Attachments der Mails (nur der Text-Body)
- Rückimport von exportierten Mails

## Decisions

### 1. Datenquelle: Direkter SQL-Query statt CommunicationDao

**Entscheidung:** Neuer `BackupDao`-Method `all_communications()` mit eigenem SQL-Query statt den bestehenden `CommunicationDao`.

**Begründung:** Der `CommunicationDao` liefert pro Mitglied und enthält nicht alle nötigen Felder (Body fehlt). Ein dedizierter Backup-Query kann beide Richtungen in einem Query JOINen und alle Felder (inkl. Body) liefern, mit Mitgliedsnummer/Name für die Ordner-Zuordnung.

**Alternative:** CommunicationDao erweitern — würde die Interface-Grenze vermischen (Timeline-Display vs. Export).

### 2. Dateiname-Pattern: `{YYYY-MM-DD}_{HHmm}_{richtung}_{betreff}.txt`

**Entscheidung:** Datum + Uhrzeit + Richtung + sanitisierter Betreff.

**Begründung:** Uhrzeit ermöglicht chronologische Nachvollziehbarkeit von Diskussionen an einem Tag. Betreff gibt Kontext ohne Datei öffnen zu müssen.

**Sanitisierung:** Betreff wird auf `[a-zA-Z0-9_-]` reduziert, Umlaute transliteriert (ä→ae etc.), max 50 Zeichen, Leerzeichen → `_`.

### 3. WebDAV-Sync: Append-Only mit ID-Tracking

**Entscheidung:** Eine Tracking-Tabelle `backup_communication_sync` speichert exportierte Mail-IDs (outbound: `recipient_id`, inbound: `inbound_mail_id`). Beim nächsten Sync werden nur neue IDs exportiert.

**Begründung:** Mails ändern sich nicht nachträglich. Ein einfaches "schon exportiert?" reicht. Kein Hash-Vergleich nötig (im Gegensatz zu Dokumenten, die aktualisiert werden können).

**Alternative:** Zeitstempel-basiert (exportiere alles nach letztem Sync) — fragiler bei Lücken oder Zeitzone-Problemen.

### 4. ZIP-Integration: `kommunikation/` Unterordner pro Mitglied

**Entscheidung:** Im bestehenden Mitglied-Ordner wird ein `kommunikation/` Unterordner angelegt.

```
001_Müller_Hans/
├── Beitrittserklärung_beitritt.pdf
└── kommunikation/
    ├── 2026-03-15_1430_ausgehend_Willkommen.txt
    └── 2026-04-01_0915_eingehend_Frage_zu_Anteilen.txt
```

**Begründung:** Hält alles zu einem Mitglied zusammen. Dokumente und Kommunikation sind logisch gruppiert.

### 5. Datei-Inhalt: Einfacher Header + Body

```
Richtung: Ausgehend
Datum: 2026-03-15 14:30:00
Von: verein@example.org
An: hans.mueller@example.com
Betreff: Willkommen bei der Genossenschaft

───────────────────────────────────────

Hallo Hans,
...
```

**Begründung:** Maximale Lesbarkeit. Kein Parser nötig, jeder Texteditor reicht.

## Risks / Trade-offs

- **Große Datenmengen bei vielen Mails** → Bei der ZIP-Generierung könnte der Speicherverbrauch steigen. Mitigation: Streaming wäre ideal, aber die aktuelle ZIP-Implementierung nutzt bereits In-Memory-Buffer. Für den Anfang akzeptabel, da Genossenschaften typischerweise moderate Mailmengen haben.

- **Betreff-Kollisionen im Dateinamen** → Zwei Mails gleiche Minute + gleicher Betreff = Überschreibung. Mitigation: UUID-Suffix (erste 8 Zeichen) anhängen wenn Kollision erkannt wird.

- **Fehlende Absender-Adresse bei Outbound** → Der MailJob speichert keine explizite From-Adresse (kommt aus SMTP-Config). Mitigation: SMTP-Config-Wert (`smtp_from`) zur Laufzeit auslesen, oder "Verein" als Platzhalter.

## Migration Plan

1. Migration: `backup_communication_sync` Tabelle erstellen (nur für WebDAV-Tracking)
2. Deployment: Keine Breaking Changes, rein additiv
3. Erster WebDAV-Zyklus exportiert alle bestehenden Mails (Initialer Lauf)
4. Rollback: Tabelle droppen, Code revertieren — keine Datenbank-Änderungen an bestehenden Tabellen
