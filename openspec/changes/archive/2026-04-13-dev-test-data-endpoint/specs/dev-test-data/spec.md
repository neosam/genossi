## ADDED Requirements

### Requirement: Testdaten-Endpunkt nur in Debug-Builds verfuegbar

Der Endpunkt `POST /api/dev/generate-test-data` SHALL nur kompiliert werden, wenn `debug_assertions` aktiv ist. In Release-Builds (`--release`) SHALL der Endpunkt nicht existieren.

#### Scenario: Debug-Build enthaelt den Endpunkt
- **WHEN** der Server mit `cargo run` (ohne `--release`) gestartet wird
- **THEN** antwortet `POST /api/dev/generate-test-data` mit einem HTTP-Statuscode (nicht 404)

#### Scenario: Release-Build enthaelt den Endpunkt nicht
- **WHEN** der Server mit `cargo run --release` gestartet wird
- **THEN** antwortet `POST /api/dev/generate-test-data` mit 404

### Requirement: Testdaten erzeugen

Der Endpunkt SHALL mindestens 5 Member-Datensaetze mit realistischen deutschen Testdaten erzeugen. Die Members SHALL unterschiedliche Eigenschaften haben (mit/ohne Email, mit/ohne Company, unterschiedliche Adressen, verschiedene Join-Dates und Share-Werte).

#### Scenario: Erstmaliger Aufruf erzeugt Testdaten
- **WHEN** `POST /api/dev/generate-test-data` aufgerufen wird und keine Members existieren
- **THEN** werden Member-Datensaetze in der Datenbank erzeugt und HTTP 201 zurueckgegeben

#### Scenario: Mindestens ein Member mit allen optionalen Feldern
- **WHEN** Testdaten erzeugt werden
- **THEN** hat mindestens ein Member Email, Company, Adresse und Bank-Account gesetzt

#### Scenario: Mindestens ein ausgetretenes Mitglied
- **WHEN** Testdaten erzeugt werden
- **THEN** hat mindestens ein Member ein gesetztes `exit_date`

### Requirement: Idempotenz

Der Endpunkt SHALL idempotent sein. Wenn bereits Members in der Datenbank existieren, SHALL er keine weiteren erzeugen.

#### Scenario: Wiederholter Aufruf erzeugt keine Duplikate
- **WHEN** `POST /api/dev/generate-test-data` aufgerufen wird und bereits Members existieren
- **THEN** werden keine neuen Members erzeugt und HTTP 200 zurueckgegeben

### Requirement: Kein Auth-Check

Der Endpunkt SHALL ohne Authentifizierung aufrufbar sein, da er nur in Debug-Builds existiert.

#### Scenario: Unauthentifizierter Aufruf
- **WHEN** `POST /api/dev/generate-test-data` ohne Auth-Header aufgerufen wird
- **THEN** wird die Anfrage trotzdem verarbeitet (kein 401)
