## ADDED Requirements

### Requirement: Mitgliederliste als Startseite

Die Anwendung SHALL die Route `/` automatisch auf `/members` umleiten, sodass die Mitgliederliste die effektive Startseite ist.

#### Scenario: Aufruf der Wurzelroute

- **WHEN** ein Nutzer die Anwendung über `/` aufruft
- **THEN** die Anwendung leitet sofort und ohne sichtbaren Zwischenschritt auf `/members` um

#### Scenario: Direkter Aufruf der Mitgliederliste

- **WHEN** ein Nutzer `/members` direkt aufruft
- **THEN** die Mitgliederliste wird ohne weitere Umleitung angezeigt

### Requirement: Statusbalken über der Mitgliederliste

Die Mitgliederliste (`/members`) SHALL oberhalb der bestehenden Toolbar einen kompakten, einzeiligen Statusbalken anzeigen, der zwei klickbare Items enthält: einen Indikator für offene Mitgliedsanträge und einen für offene Mails.

#### Scenario: Anzeige bei vorhandenen offenen Anträgen und Mails

- **WHEN** beim Aufruf der Mitgliederliste 3 Anträge mit Status `Offen` und 12 Mails mit Status `open` existieren
- **THEN** der Statusbalken zeigt „3 offene Anträge" und „12 offene Mails" an
- **AND** beide Texte sind als Link gestaltet

#### Scenario: Anzeige ohne offene Einträge

- **WHEN** beim Aufruf der Mitgliederliste keine Anträge mit Status `Offen` und keine Mails mit Status `open` existieren
- **THEN** der Statusbalken zeigt „Keine offenen Anträge" und „Keine offenen Mails" an
- **AND** beide Texte bleiben als Link gestaltet

#### Scenario: Gemischter Zustand

- **WHEN** beim Aufruf der Mitgliederliste 0 offene Anträge, aber 2 offene Mails existieren
- **THEN** der Statusbalken zeigt „Keine offenen Anträge" und „2 offene Mails" an

### Requirement: Navigation aus dem Statusbalken

Beide Items des Statusbalkens SHALL klickbar sein und auf die jeweils passende Listenseite mit dem Filter „offen" navigieren — auch dann, wenn der Zähler 0 ist.

#### Scenario: Klick auf Anträge-Item

- **WHEN** ein Nutzer auf das Item für offene Anträge klickt
- **THEN** die Anwendung navigiert zu `/applications`
- **AND** der dortige Statusfilter steht auf `Offen`

#### Scenario: Klick auf Mails-Item

- **WHEN** ein Nutzer auf das Item für offene Mails klickt
- **THEN** die Anwendung navigiert zu `/inbox`
- **AND** der dortige Statusfilter steht auf `open`

#### Scenario: Klick bei Zählerwert 0

- **WHEN** ein Nutzer auf „Keine offenen Anträge" klickt
- **THEN** die Anwendung navigiert zu `/applications` mit Statusfilter `Offen`

### Requirement: Laden der Zählerstände

Der Statusbalken SHALL die Anzahlen beim Aufruf der Mitgliederliste einmalig vom Backend laden. Es findet kein automatisches Polling statt.

#### Scenario: Initiales Laden

- **WHEN** die Mitgliederliste aufgerufen wird
- **THEN** die Anwendung ruft die bestehenden Endpoints für offene Anträge und offene Mails ab
- **AND** die Anzahl ergibt sich aus der Länge der jeweiligen Antwortliste

#### Scenario: Aktualisierung nach Bearbeitung

- **WHEN** ein Nutzer einen Antrag oder eine Mail auf einer anderen Seite bearbeitet und anschließend zur Mitgliederliste zurückkehrt
- **THEN** die Zählerstände werden beim erneuten Aufruf neu geladen

#### Scenario: Fehler beim Laden

- **WHEN** der Abruf eines der Zählerstände fehlschlägt
- **THEN** das betroffene Item zeigt anstelle der Zahl einen neutralen Platzhalter (z. B. „—")
- **AND** die Mitgliederliste selbst bleibt voll funktionsfähig

### Requirement: Mehrsprachigkeit

Alle Texte des Statusbalkens SHALL in den drei unterstützten Sprachen Deutsch, Englisch und Tschechisch verfügbar sein.

#### Scenario: Wechsel der Sprache

- **WHEN** die UI-Sprache auf Englisch oder Tschechisch eingestellt ist
- **THEN** die Texte „N offene Anträge", „Keine offenen Anträge", „N offene Mails" und „Keine offenen Mails" erscheinen in der gewählten Sprache
