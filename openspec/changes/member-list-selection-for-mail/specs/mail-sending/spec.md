## MODIFIED Requirements

### Requirement: Empfänger-Darstellung auf der Mail-Seite
Die Mail-Seite MUSS Empfänger in einer skalierbaren Darstellung anzeigen, die sowohl für wenige als auch für hunderte Empfänger funktioniert. Standardmäßig wird eine eingeklappte Zusammenfassung angezeigt, die auf Klick zu einer scrollbaren Detail-Liste aufgeklappt werden kann.

#### Scenario: Eingeklappte Ansicht bei Empfängern
- **WHEN** Empfänger ausgewählt sind
- **THEN** zeigt die Mail-Seite eine eingeklappte Zusammenfassung mit Anzahl der Empfänger und einem Aufklapp-Button

#### Scenario: Warnung bei Empfängern ohne E-Mail
- **WHEN** Empfänger ohne E-Mail-Adresse in der Auswahl sind
- **THEN** zeigt die Zusammenfassung eine Warnung mit der Anzahl der Empfänger ohne E-Mail

#### Scenario: Aufgeklappte Detail-Liste
- **WHEN** der Benutzer die Empfänger-Liste aufklappt
- **THEN** erscheint eine scrollbare Liste (max-h-60) mit Mitgliedsnummer, Name, E-Mail-Adresse und Entfernen-Button pro Zeile

#### Scenario: Einzelnen Empfänger entfernen
- **WHEN** der Benutzer den Entfernen-Button bei einem Empfänger klickt
- **THEN** wird der Empfänger aus der Liste entfernt und die Zusammenfassung aktualisiert

#### Scenario: Leere Empfänger-Liste
- **WHEN** alle Empfänger entfernt wurden
- **THEN** verschwindet die Empfänger-Darstellung und der Senden-Button wird deaktiviert

### Requirement: Vorauswahl aus GlobalSignal übernehmen
Die Mail-Seite MUSS beim Laden prüfen, ob `SELECTED_MEMBER_IDS` Einträge enthält, und diese als initiale Empfänger übernehmen. Nach der Übernahme MUSS das GlobalSignal geleert werden.

#### Scenario: Mail-Seite mit vorausgewählten Empfängern
- **WHEN** die Mail-Seite geladen wird und `SELECTED_MEMBER_IDS` nicht leer ist
- **THEN** werden die IDs als initiale `selected_member_ids` übernommen
- **AND** `SELECTED_MEMBER_IDS` wird auf leer gesetzt

#### Scenario: Mail-Seite ohne Vorauswahl
- **WHEN** die Mail-Seite geladen wird und `SELECTED_MEMBER_IDS` leer ist
- **THEN** startet die Mail-Seite ohne vorausgewählte Empfänger (bestehendes Verhalten)

#### Scenario: Manuelle Suche bleibt verfügbar
- **WHEN** Empfänger aus der Mitgliederliste vorausgewählt wurden
- **THEN** kann der Benutzer weiterhin über die Autocomplete-Suche weitere Empfänger hinzufügen
