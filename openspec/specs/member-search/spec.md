### Requirement: Mitglieder-Suche per Textfeld
Die `MemberSearch`-Komponente MUSS ein Textfeld bereitstellen, in das der Benutzer einen Suchbegriff eingeben kann. Die Ergebnisse MÜSSEN clientseitig aus der bestehenden `MEMBERS`-Liste gefiltert werden.

#### Scenario: Suche nach Nachname
- **WHEN** der Benutzer "müll" in das Suchfeld eingibt
- **THEN** werden alle Mitglieder angezeigt, deren Vor- oder Nachname "müll" enthält (case-insensitive)

#### Scenario: Suche nach Mitgliedsnummer
- **WHEN** der Benutzer "42" in das Suchfeld eingibt
- **THEN** werden alle Mitglieder angezeigt, deren Mitgliedsnummer "42" enthält

#### Scenario: Keine Ergebnisse
- **WHEN** der Benutzer einen Suchbegriff eingibt, der auf kein Mitglied passt
- **THEN** wird kein Dropdown angezeigt

#### Scenario: Leeres Suchfeld
- **WHEN** das Suchfeld leer ist
- **THEN** wird kein Dropdown angezeigt

### Requirement: Ergebnisliste mit begrenzter Anzeige
Die Ergebnisliste MUSS maximal 10 Einträge anzeigen, sortiert nach Mitgliedsnummer.

#### Scenario: Mehr als 10 Treffer
- **WHEN** die Suche mehr als 10 Treffer liefert
- **THEN** werden nur die ersten 10 Ergebnisse (nach Mitgliedsnummer sortiert) angezeigt

### Requirement: Ergebnisanzeige mit Nummer und Name
Jeder Eintrag in der Ergebnisliste MUSS die Mitgliedsnummer und den vollen Namen (Vorname + Nachname) anzeigen.

#### Scenario: Ergebnisdarstellung
- **WHEN** Ergebnisse im Dropdown angezeigt werden
- **THEN** zeigt jeder Eintrag das Format "#<Nummer> <Vorname> <Nachname>"

### Requirement: Mitglied auswählen
Der Benutzer MUSS ein Mitglied aus der Ergebnisliste durch Klick auswählen können.

#### Scenario: Klick auf Ergebnis
- **WHEN** der Benutzer auf einen Eintrag in der Ergebnisliste klickt
- **THEN** wird das Suchfeld durch eine Anzeige des gewählten Mitglieds ersetzt (Name + Nummer)
- **AND** der `on_select`-Callback wird mit der UUID des Mitglieds aufgerufen

### Requirement: Auswahl zurücksetzen
Der Benutzer MUSS die Auswahl über einen Zurücksetzen-Button löschen können.

#### Scenario: Klick auf Zurücksetzen
- **WHEN** ein Mitglied ausgewählt ist und der Benutzer den ✕-Button klickt
- **THEN** wird die Auswahl gelöscht und das Suchfeld wieder angezeigt
- **AND** der `on_select`-Callback wird mit `None` aufgerufen

### Requirement: Aktuelles Mitglied ausschließen
Wenn eine `exclude_id` gesetzt ist, MUSS das entsprechende Mitglied aus den Suchergebnissen ausgeschlossen werden.

#### Scenario: Eigenes Mitglied ausgeschlossen
- **WHEN** die Komponente mit einer `exclude_id` konfiguriert ist
- **THEN** erscheint das Mitglied mit dieser ID nie in den Suchergebnissen

### Requirement: Integration in Übertragungs-Formular
Das UUID-Textfeld für das Gegenmitglied im Aktions-Formular MUSS durch die `MemberSearch`-Komponente ersetzt werden.

#### Scenario: Übertrag-Aktion erstellen
- **WHEN** der Benutzer eine Übertragungs-Aktion (Empfang oder Abgabe) erstellt
- **THEN** kann er das Gegenmitglied über die Suchkomponente auswählen statt eine UUID einzugeben
