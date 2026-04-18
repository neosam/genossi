## Meta
- **Priority:** medium
- **Category:** security

## Why

Genossi verwaltet personenbezogene Daten (Namen, Adressen, Bankverbindungen, E-Mail-Adressen) von Vereinsmitgliedern. Drei Findings aus dem Security Audit (2026-04-18) betreffen den Datenschutz:

1. **M3 — PII in API-Responses:** Alle authentifizierten Nutzer mit Leseberechtigung sehen sämtliche Mitgliederdaten inkl. Bankverbindung (`genossi_rest_types/src/lib.rs`, MemberTO). Es gibt keine Feld-Level-Zugriffskontrolle.
2. **M5 — Kein Hard-Delete:** Nur Soft-Deletes implementiert. Audit-Logs bewahren PII unbegrenzt. Art. 17 DSGVO ("Recht auf Löschung") kann nicht vollständig erfüllt werden.
3. **I3 — PII in Logs:** Member-Daten könnten bei Debug-Logging in Log-Dateien erscheinen.

## What Changes

- **PII-Zugriffskontrolle evaluieren:** Prüfen, ob sensible Felder (Bankverbindung, Adresse) nur für Nutzer mit bestimmten Privilegien sichtbar sein sollen, oder ob die aktuelle Berechtigung ("manage_members" = alles sehen) für den Vereinskontext ausreicht.
- **Anonymisierungs-Mechanismus:** Endpoint oder Admin-Funktion, die PII eines gelöschten Mitglieds in DB und Audit-Log anonymisiert (z.B. Name → "GELÖSCHT", E-Mail → Hash). Audit-Log-Einträge bleiben erhalten, aber ohne identifizierbare Daten.
- **Log-PII-Schutz:** Sicherstellen, dass `tracing`-Aufrufe keine Member-Felder mit PII loggen, z.B. durch `#[instrument(skip(member))]` oder gezielte Field-Selection.

## Capabilities

### New Capabilities

- `gdpr-member-anonymization`: Anonymisierung personenbezogener Daten gelöschter Mitglieder in DB und Audit-Log

### Modified Capabilities

_(zu evaluieren: ggf. `member-documents` für Dokument-Löschung, `audit-logging` für Anonymisierung)_

## Impact

**Code:**
- Neuer Service-Layer-Endpoint für Anonymisierung
- Audit-Log: Anonymisierungs-Logik für historische Einträge
- REST-Handler: ggf. PII-Filterung nach Berechtigung
- Logging: `#[instrument]`-Annotationen prüfen

**Datenbank:**
- Ggf. Migration für Anonymisierungs-Status-Feld

**Rechtlich:**
- Ermöglicht Compliance mit Art. 17 DSGVO
