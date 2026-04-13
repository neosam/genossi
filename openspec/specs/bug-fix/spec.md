## ADDED Requirements

### Requirement: Anzeigename wird nach dem Laden korrekt dargestellt

Die Berechtigungsseite SHALL den geladenen `sender_name` eines Users im Eingabefeld anzeigen, sobald die Daten von der API geladen wurden.

Die Konfigurationsseite SHALL den eigenen `sender_name` des eingeloggten Users im Eingabefeld anzeigen, sobald die Daten von der API geladen wurden.

#### Scenario: Anzeigename auf Berechtigungsseite sichtbar
- **WHEN** ein Admin die Berechtigungsseite öffnet und ein User einen gesetzten `sender_name` hat
- **THEN** wird der `sender_name` im Eingabefeld der entsprechenden Zeile angezeigt

#### Scenario: Anzeigename auf Konfigurationsseite sichtbar
- **WHEN** ein User die Konfigurationsseite öffnet und einen gesetzten `sender_name` hat
- **THEN** wird der eigene `sender_name` im Absendername-Eingabefeld angezeigt

#### Scenario: Leerer Anzeigename bei neuem User
- **WHEN** ein User noch keinen `sender_name` gesetzt hat
- **THEN** bleibt das Eingabefeld leer (kein Fehler)
