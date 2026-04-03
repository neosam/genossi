## ADDED Requirements

### Requirement: Checkbox-Spalte in der Mitgliedertabelle
Die Mitgliederliste MUSS eine Checkbox-Spalte als erste Spalte in der Tabelle anzeigen. Jede Zeile MUSS eine Checkbox enthalten, mit der das Mitglied selektiert/deselektiert werden kann.

#### Scenario: Mitglied per Checkbox selektieren
- **WHEN** der Benutzer auf die Checkbox eines Mitglieds klickt
- **THEN** wird das Mitglied zur Selektion hinzugefügt und die Checkbox als angehakt dargestellt

#### Scenario: Mitglied per Checkbox deselektieren
- **WHEN** der Benutzer auf die Checkbox eines bereits selektierten Mitglieds klickt
- **THEN** wird das Mitglied aus der Selektion entfernt und die Checkbox als nicht angehakt dargestellt

#### Scenario: Zeilen-Klick navigiert weiterhin zur Detailseite
- **WHEN** der Benutzer auf den Nicht-Checkbox-Bereich einer Zeile klickt
- **THEN** wird zur Mitglieder-Detailseite navigiert (bestehendes Verhalten bleibt erhalten)

#### Scenario: Checkbox-Klick propagiert nicht zur Zeile
- **WHEN** der Benutzer auf die Checkbox oder deren Zelle klickt
- **THEN** wird NICHT zur Detailseite navigiert

### Requirement: Mobiltaugliches Touch-Target für Checkboxen
Die Checkbox-Zelle MUSS ein Touch-Target von mindestens 44x44 Pixel haben, damit die Selektion auf mobilen Geräten zuverlässig funktioniert.

#### Scenario: Touch auf Checkbox-Zelle auf Mobilgerät
- **WHEN** der Benutzer auf einem Mobilgerät die Checkbox-Zelle antippt
- **THEN** wird die Checkbox umgeschaltet, ohne dass versehentlich die Zeilennavigation ausgelöst wird

### Requirement: Alle gefilterten Mitglieder auswählen
Der Tabellenkopf MUSS eine Checkbox enthalten, mit der alle aktuell gefilterten Mitglieder gleichzeitig selektiert oder deselektiert werden können.

#### Scenario: Alle gefilterten auswählen
- **WHEN** keine oder nur einige Mitglieder selektiert sind und der Benutzer die Kopf-Checkbox anklickt
- **THEN** werden alle aktuell sichtbaren (gefilterten) Mitglieder zur Selektion hinzugefügt

#### Scenario: Alle gefilterten abwählen
- **WHEN** alle gefilterten Mitglieder selektiert sind und der Benutzer die Kopf-Checkbox anklickt
- **THEN** werden alle gefilterten Mitglieder aus der Selektion entfernt

#### Scenario: Kopf-Checkbox Zustand bei Teilselektion
- **WHEN** einige aber nicht alle gefilterten Mitglieder selektiert sind
- **THEN** zeigt die Kopf-Checkbox einen Indeterminate-Zustand (oder unchecked)

### Requirement: Selektions-Aktionsleiste
Die Mitgliederliste MUSS eine Aktionsleiste anzeigen, wenn mindestens ein Mitglied selektiert ist. Die Leiste MUSS die Anzahl der selektierten Mitglieder und einen "Mail senden"-Button enthalten.

#### Scenario: Aktionsleiste erscheint bei Selektion
- **WHEN** der Benutzer mindestens ein Mitglied selektiert
- **THEN** erscheint eine Aktionsleiste mit der Anzahl der selektierten Mitglieder und dem "Mail senden"-Button

#### Scenario: Aktionsleiste verschwindet ohne Selektion
- **WHEN** alle Mitglieder deselektiert werden
- **THEN** verschwindet die Aktionsleiste

#### Scenario: Mail senden navigiert zur Mail-Seite
- **WHEN** der Benutzer den "Mail senden"-Button in der Aktionsleiste klickt
- **THEN** werden die selektierten Member-IDs in den GlobalSignal `SELECTED_MEMBER_IDS` geschrieben
- **AND** der Benutzer wird zur Mail-Seite navigiert

### Requirement: GlobalSignal für Selektions-Übergabe
Das System MUSS einen `GlobalSignal<Vec<Uuid>>` namens `SELECTED_MEMBER_IDS` bereitstellen, über den selektierte Member-IDs zwischen der Mitgliederliste und der Mail-Seite übergeben werden.

#### Scenario: IDs werden beim Navigieren gesetzt
- **WHEN** der Benutzer "Mail senden" in der Mitgliederliste klickt
- **THEN** enthält `SELECTED_MEMBER_IDS` die UUIDs aller selektierten Mitglieder

#### Scenario: Signal wird nach Übernahme geleert
- **WHEN** die Mail-Seite die IDs aus `SELECTED_MEMBER_IDS` übernommen hat
- **THEN** wird das Signal auf einen leeren Vec geleert
