## Why

Das Audit-Log-System (Change `audit-log`) bietet eine Hash-Chain zur Manipulationserkennung. Allerdings könnte ein Admin mit direktem Datenbankzugang sowohl die Daten als auch die Hash-Chain gleichzeitig manipulieren und die Kette neu berechnen. Um einen gerichtsfesten, externen Nachweis der Integrität zu haben, muss der aktuelle Hash-Stand regelmäßig bei einem qualifizierten Vertrauensdiensteanbieter (eIDAS Art. 41 Abs. 2) verankert werden. Ein qualifizierter Zeitstempel genießt vor Gericht die gesetzliche Vermutung der Richtigkeit — die Gegenseite müsste die Fälschung beweisen (Beweislastumkehr).

## What Changes

- Periodische Verankerung des aktuellen Audit-Log-Hash bei einem qualifizierten Zeitstempeldienst (RFC 3161) wie DGN (5 Gratis/Monat, danach ~0,09 EUR/Stempel)
- Eigenständiger Timestamp-Worker mit konfigurierbarem Intervall (Default: wöchentlich)
- Manueller Timestamp-Trigger über REST-API und Frontend-Button
- Speicherung der signierten Zeitstempel-Tokens (.tsr-Dateien) lokal in SQLite und optional auf WebDAV/Nextcloud
- Konfiguration des TSA-Endpoints und der Credentials über Config-Store mit Frontend-UI
- Verifizierungs-Endpoint und UI zum Prüfen der externen Zeitstempel

## Capabilities

### New Capabilities
- `qualified-timestamping`: Kern-Integration mit einem RFC 3161 Zeitstempeldienst — Hash abrufen, TSA-Request senden, Token speichern, Token verifizieren
- `timestamp-verification`: REST-API und Frontend-UI zum Anzeigen und Verifizieren der externen Zeitstempel-Tokens

### Modified Capabilities
- (keine — der Timestamp-Worker ist eigenständig und modifiziert den Backup-Worker nicht)

## Impact

- **Service-Layer**: Neuer `TimestampService` für RFC 3161 Kommunikation und Token-Verwaltung
- **Timestamp-Worker**: Eigenständiger periodischer Worker (Default: wöchentlich)
- **Config-Store**: Neue Konfigurationsschlüssel für TSA-URL, Credentials und Intervall
- **REST-Layer**: Neue Endpoints für Timestamp-Listing, -Verifikation und manuellen Trigger (POST)
- **Frontend**: UI zur TSA-Konfiguration, Timestamp-Historie, Verifikation und manuellem Trigger
- **WebDAV**: Neues Verzeichnis `audit-timestamps/` für .tsr-Dateien (optional)
- **Dependencies**: RFC 3161 / ASN.1 Crate für Token-Parsing (z.B. `cms`, `der`, `x509-cert` oder ein dediziertes TSA-Crate)
- **Externe Abhängigkeit**: Account bei einem qualifizierten Zeitstempeldienst (z.B. DGN)
- **Voraussetzung**: Change `audit-log` muss zuerst implementiert sein
