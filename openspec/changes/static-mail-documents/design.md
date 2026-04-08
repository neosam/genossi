## Context

Das System hat bereits eine `mail-attachments` Capability, die aber strikt an `member-documents` gebunden ist: Anhänge werden pro Empfänger geführt und vom Backend gegen die Mitgliedschaft validiert. Das passt für individuelle Dokumente (z.B. Beitrittserklärungen), aber nicht für wiederverwendbare, globale Dateien wie Satzung oder Flyer.

Gleichzeitig speichert `member-documents` heute bereits Dateien — ein Blick auf dessen Storage-Strategie ist wichtig, um Konsistenz zu wahren. Für statische Dokumente wurde jedoch bewusst **Filesystem** statt BLOB gewählt (siehe Decisions), da es um wenige, eher große und langlebige Dateien geht und eine Trennung zu individuellen Member-Dokumenten gewünscht ist.

Stakeholder: Vorstand (Upload/Verwaltung, Auswahl beim Versand), Empfänger (erhalten Mail mit Anhang). Bestehende Auth- und Middleware-Schicht wird wiederverwendet.

## Goals / Non-Goals

**Goals:**
- Globale, hochladbare Dokumente, unabhängig von Mitgliedern
- Mehrfach-Auswahl beim Bulk-Mail-Versand; alle Empfänger erhalten dieselben Dateien
- Zentrale Admin-Seite zur Verwaltung (Upload, Liste, Löschen)
- Robustes Speicherlayout (Filesystem + Metadaten in DB), konfigurierbarer Pfad
- Keine Störung bestehender `mail-attachments` (member-gebunden); beide Mechanismen koexistieren

**Non-Goals:**
- Keine Koppelung an Mail-Templates (Template ↔ Dokument)
- Keine Versionierung von Dokumenten (Upload = neues Dokument)
- Keine Bearbeitung/Umbenennung nach Upload (v1 reicht Upload + Löschen)
- Keine Feingranulare Berechtigungen (Vorstandsrechte reichen, analog Templates)
- Keine Vorschau im Frontend; nur Download

## Decisions

### Decision: Filesystem statt BLOB-Storage
Statische Dokumente werden im Filesystem unter einem konfigurierbaren Basispfad gespeichert. Jede Datei liegt unter ihrer UUID als Dateiname (ohne Extension), Original-Dateiname und Content-Type stehen in der DB.

**Warum:**
- Trennung von Payload und Metadaten; DB bleibt schlank
- Bewusste Entkopplung von `member-documents` (die ggf. BLOB verwenden) — diese Capability ist eigenständig
- Einfachere Handhabung größerer Dateien (wenige MB) durch direktes Streaming
- User-Präferenz (explizit so entschieden)

**Alternative:** SQLite BLOB. Vorteil: atomare Backups. Nachteil: DB-Wachstum, weniger Streaming-Freundlichkeit.

### Decision: Dateiname auf Platte = UUID ohne Extension
Der Pfad ist `<STATIC_DOCUMENTS_PATH>/<uuid>`. Der Original-Dateiname wird nur in der DB geführt und beim Download als `Content-Disposition: attachment; filename=...` gesetzt.

**Warum:**
- Verhindert Pfad-Traversal durch Nutzer-Eingaben
- Keine Dateinamen-Kollisionen
- Konsistenter, deterministischer Lookup anhand der ID

### Decision: Konfigurierbarer Basispfad über ENV
Neue ENV-Variable `STATIC_DOCUMENTS_PATH` (Default: `./data/static_documents`). Beim Start wird geprüft, dass der Ordner existiert oder angelegt werden kann.

### Decision: Join-Tabelle pro Mail-Job (nicht pro Empfänger)
`mail_job_static_attachments (mail_job_id, static_document_id)` verknüpft einen Bulk-Mail-Job mit den ausgewählten globalen Dokumenten. Das tatsächliche Mail-Modell im Code ist `mail_jobs` → `mail_recipients`; `sent_mails` gibt es nicht mehr. Der Worker liest pro Empfänger-Verarbeitung die statischen Anhänge des Jobs und fügt sie an jede ausgehende Nachricht an.

**Warum:** Statische Dokumente sind per Definition für alle Empfänger gleich — eine Zuordnung pro Empfänger wäre redundant und würde die Tabelle unnötig aufblähen.

**Alternative:** Pro Empfänger (wie `mail_recipient_attachments`). Abgelehnt wegen Redundanz.

### Decision: Koexistenz mit bestehenden `mail-attachments`
Der Bulk-Mail-Request akzeptiert **zusätzlich** `static_document_ids` (pro Request, nicht pro Empfänger). Bestehende `recipient_attachments` bleiben unverändert. Der Worker kombiniert beide Listen beim Bau der Multipart-Nachricht.

### Decision: Größen- und Content-Type-Validierung
- Max. Dateigröße: 10 MB (Default), per ENV `STATIC_DOCUMENTS_MAX_BYTES` überschreibbar
- Erlaubte Content-Types: `application/pdf`, `image/png`, `image/jpeg` (Startset; erweiterbar)
- Validierung im Service-Layer, bevor die Datei persistiert wird

### Decision: Soft-Delete, Datei bleibt auf Platte
Löschen setzt `deleted`-Timestamp (Projekt-Pattern). Die Datei auf Platte bleibt liegen, da aktiv gesendete Mails theoretisch noch darauf verweisen könnten (historisch). Ein späterer Garbage-Collector kann Dateien entfernen, auf die keine `mail_static_attachments` mehr verweisen — für v1 nicht nötig.

### Decision: Axum-Multipart für Upload
Upload-Endpoint `POST /api/static-documents` akzeptiert `multipart/form-data` mit einem Feld `file` und optionalem `name`. Axum unterstützt das nativ.

## Risks / Trade-offs

- **Risk:** Datei auf Platte, Metadaten in DB → Inkonsistenz möglich, wenn einer der beiden Schritte fehlschlägt.
  **Mitigation:** Reihenfolge: erst Datei auf Platte schreiben (tmp-File + rename), dann DB-Insert. Bei DB-Fehler Datei wieder entfernen. Orphaned Files sind harmlos, verwaiste DB-Einträge wären schlimmer.

- **Risk:** Durchreichen großer Dateien bläht Mail-Worker-Speicher auf (jeder Empfänger = Kopie im Speicher).
  **Mitigation:** Dateien werden pro Batch einmal eingelesen und als `lettre` Attachment referenziert. Bei Batchgröße 10 × 10 MB = ~100 MB worst case — akzeptabel.

- **Risk:** Filesystem-Pfad falsch konfiguriert, Startup-Fehler nicht offensichtlich.
  **Mitigation:** Beim Server-Start: Ordner existiert/anlegbar, Schreibrecht prüfen — andernfalls Fail-Fast mit klarer Fehlermeldung.

- **Risk:** Nutzer lädt schädlichen Content-Type hoch.
  **Mitigation:** Whitelist clientseitig und serverseitig. Content-Disposition bei Download auf `attachment` (kein inline), um XSS über Browser-Preview zu vermeiden.

- **Trade-off:** Keine Versionierung — wenn ein Dokument ersetzt werden soll, muss neu hochgeladen und in neuen Mails manuell ausgewählt werden. Akzeptabel für v1.

## Migration Plan

- SQLite-Migration fügt `static_documents` und `mail_static_attachments` Tabellen hinzu
- Beim ersten Start wird der Dokumenten-Ordner angelegt, falls nicht vorhanden
- Keine Datenmigration nötig (neue Capability, keine Altdaten)
- Rollback: Migration ist additiv — Rollback durch Entfernen der Tabellen und des Ordners möglich

## Open Questions

- Erweitertes Content-Type-Set (DOCX, ODT)? Kann nachgezogen werden.
- Frontend-Sortierung der Dokumente (alphabetisch vs. Upload-Datum)? Default: alphabetisch.
