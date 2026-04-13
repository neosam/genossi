## ADDED Requirements

### Requirement: Upload-Spalte im Spalten-Picker
Die Mitgliederliste MUSS eine optionale Spezialspalte "Dokument hochladen" im Spalten-Picker anbieten. Die Spalte MUSS visuell von den regulären Datenspalten abgetrennt sein (z.B. durch einen Separator).

#### Scenario: Upload-Spalte einblenden
- **WHEN** der Benutzer im Spalten-Picker die Option "Dokument hochladen" aktiviert
- **THEN** erscheint eine zusätzliche Spalte rechts in der Tabelle
- **AND** die globalen Upload-Einstellungen werden über der Tabelle angezeigt

#### Scenario: Upload-Spalte ausblenden
- **WHEN** der Benutzer im Spalten-Picker die Option "Dokument hochladen" deaktiviert
- **THEN** verschwindet die Upload-Spalte aus der Tabelle
- **AND** die globalen Upload-Einstellungen werden ausgeblendet

#### Scenario: Upload-Spalte wird nicht persistiert
- **WHEN** der Benutzer die Seite neu lädt
- **THEN** ist die Upload-Spalte ausgeblendet, unabhängig vom vorherigen Zustand

### Requirement: Globale Upload-Einstellungen
Wenn die Upload-Spalte aktiv ist, MUSS über der Tabelle ein Einstellungsbereich erscheinen mit einem Dokumenttyp-Dropdown und einem optionalen Beschreibungsfeld. Beide Werte gelten für alle Uploads.

#### Scenario: Dokumenttyp auswählen
- **WHEN** die Upload-Spalte aktiv ist
- **THEN** zeigt das System ein Dropdown mit den Optionen: Beitrittserklärung, Beitrittsbestätigung, Aufstockung, Sonstige
- **AND** es ist kein Typ vorausgewählt

#### Scenario: Beschreibung eingeben
- **WHEN** die Upload-Spalte aktiv ist
- **THEN** zeigt das System ein optionales Freitextfeld für die Beschreibung
- **AND** der eingegebene Wert gilt für alle nachfolgenden Uploads

#### Scenario: Dokumenttyp wechseln
- **WHEN** der Benutzer den Dokumenttyp im Dropdown ändert
- **THEN** werden die Dokumenten-Counts für den neuen Typ geladen
- **AND** der Upload-Status aller Zeilen wird zurückgesetzt

### Requirement: Upload-Zelle pro Mitglied
Jede Zeile in der Upload-Spalte MUSS den aktuellen Status für dieses Mitglied und den gewählten Dokumenttyp anzeigen.

#### Scenario: Kein Dokumenttyp ausgewählt
- **WHEN** die Upload-Spalte aktiv ist aber kein Dokumenttyp gewählt wurde
- **THEN** zeigt jede Zelle einen deaktivierten Zustand (kein File-Input)

#### Scenario: Dokument bereits vorhanden (Singleton-Typ)
- **WHEN** ein Mitglied bereits ein aktives Dokument des gewählten Singleton-Typs hat
- **THEN** zeigt die Zelle "vorhanden" an
- **AND** es wird kein File-Input angezeigt

#### Scenario: Kein Dokument vorhanden
- **WHEN** ein Mitglied kein aktives Dokument des gewählten Typs hat
- **THEN** zeigt die Zelle einen File-Input zum Hochladen

#### Scenario: Upload läuft
- **WHEN** eine Datei ausgewählt wurde und der Upload läuft
- **THEN** zeigt die Zelle einen Lade-Indikator

#### Scenario: Upload erfolgreich
- **WHEN** der Upload erfolgreich abgeschlossen wurde
- **THEN** zeigt die Zelle eine Erfolgsmeldung
- **AND** bei Singleton-Typen wechselt der Status auf "vorhanden"

#### Scenario: Upload fehlgeschlagen
- **WHEN** der Upload fehlschlägt
- **THEN** zeigt die Zelle eine Fehlermeldung
- **AND** der File-Input bleibt verfügbar für einen erneuten Versuch

### Requirement: Upload-Spalte unabhängig vom Bearbeitungsmodus
Die Upload-Spalte MUSS unabhängig vom Bearbeitungsmodus der Mitgliederliste funktionieren. Der Upload MUSS sowohl im normalen Ansichtsmodus als auch im Bearbeitungsmodus möglich sein.

#### Scenario: Upload im Ansichtsmodus
- **WHEN** der Bearbeitungsmodus deaktiviert ist und die Upload-Spalte aktiv ist
- **THEN** kann der Benutzer Dateien hochladen

#### Scenario: Upload im Bearbeitungsmodus
- **WHEN** der Bearbeitungsmodus aktiviert ist und die Upload-Spalte aktiv ist
- **THEN** kann der Benutzer gleichzeitig Mitgliederdaten bearbeiten und Dateien hochladen

### Requirement: Sofortiger Upload bei Dateiauswahl
Wenn der Benutzer eine Datei im File-Input auswählt, MUSS der Upload sofort starten. Es gibt keinen separaten "Hochladen"-Button.

#### Scenario: Datei auswählen löst Upload aus
- **WHEN** der Benutzer eine Datei im File-Input einer Zeile auswählt
- **THEN** startet der Upload sofort mit dem global gewählten Dokumenttyp und der globalen Beschreibung
- **AND** der File-Input wird durch einen Lade-Indikator ersetzt
