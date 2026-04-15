## Why

Das Audit-Log-System (Change `audit-log`) bietet eine Hash-Chain zur Manipulationserkennung. Allerdings könnte ein Admin mit direktem Datenbankzugang sowohl die Daten als auch die Hash-Chain gleichzeitig manipulieren und die Kette neu berechnen. Um einen gerichtsfesten, externen Nachweis der Integrität zu haben, muss der aktuelle Hash-Stand regelmäßig bei einem qualifizierten Vertrauensdiensteanbieter (eIDAS Art. 41 Abs. 2) verankert werden. Ein qualifizierter Zeitstempel genießt vor Gericht die gesetzliche Vermutung der Richtigkeit — die Gegenseite müsste die Fälschung beweisen (Beweislastumkehr).

## What Changes

- Periodische Verankerung des aktuellen Audit-Log-Hash bei einem qualifizierten Zeitstempeldienst (RFC 3161) wie DGN (~0,06 EUR/Stempel, ~22 EUR/Jahr bei täglichem Stempel)
- Speicherung der signierten Zeitstempel-Tokens (.tsr-Dateien) auf WebDAV/Nextcloud
- Konfiguration des TSA-Endpoints und der Credentials über den bestehenden Config-Store
- Verifizierungs-Endpoint und UI zum Prüfen der externen Zeitstempel
- Integration in den bestehenden Backup-Worker (periodischer Ablauf)

## Capabilities

### New Capabilities
- `qualified-timestamping`: Kern-Integration mit einem RFC 3161 Zeitstempeldienst — Hash abrufen, TSA-Request senden, Token speichern, Token verifizieren
- `timestamp-verification`: REST-API und Frontend-UI zum Anzeigen und Verifizieren der externen Zeitstempel-Tokens

### Modified Capabilities
- `webdav-backup`: Backup-Worker erhält zusätzlichen Schritt zum Hochladen der .tsr-Dateien auf WebDAV

## Impact

- **Service-Layer**: Neuer `TimestampService` für RFC 3161 Kommunikation und Token-Verwaltung
- **Backup-Worker**: Zusätzlicher Schritt im Backup-Zyklus
- **Config-Store**: Neue Konfigurationsschlüssel für TSA-URL und Credentials
- **REST-Layer**: Neue Endpoints für Timestamp-Status und -Verifikation
- **Frontend**: UI zur Anzeige der Timestamp-Historie und Verifikation
- **WebDAV**: Neues Verzeichnis `audit-timestamps/` für .tsr-Dateien
- **Dependencies**: RFC 3161 / ASN.1 Crate für Token-Parsing (z.B. `cms`, `der`, `x509-cert` oder ein dediziertes TSA-Crate)
- **Externe Abhängigkeit**: Account bei einem qualifizierten Zeitstempeldienst (z.B. DGN)
- **Voraussetzung**: Change `audit-log` muss zuerst implementiert sein
