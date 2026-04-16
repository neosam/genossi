## ADDED Requirements

### Requirement: Wiederverwendbare CollapsibleSection-Komponente

Das Frontend SHALL eine wiederverwendbare Dioxus-Komponente `CollapsibleSection` bereitstellen, die einen klickbaren Header mit Titel und einen darunter liegenden Inhaltsbereich kapselt, der ein- und ausgeklappt werden kann.

#### Scenario: Komponente rendert eingeklappt

- **WHEN** die Komponente mit dem Standard-Initialzustand „eingeklappt" gemountet wird
- **THEN** nur der Header mit Titel und einem Pfeil-Icon wird angezeigt
- **AND** der Inhaltsbereich ist nicht sichtbar

#### Scenario: Klick auf den Header öffnet die Sektion

- **WHEN** der Nutzer auf den Header einer eingeklappten Sektion klickt
- **THEN** der Inhaltsbereich wird sichtbar
- **AND** das Pfeil-Icon zeigt den geöffneten Zustand an

#### Scenario: Klick auf den Header schließt die Sektion

- **WHEN** der Nutzer auf den Header einer geöffneten Sektion klickt
- **THEN** der Inhaltsbereich wird verborgen
- **AND** das Pfeil-Icon zeigt wieder den geschlossenen Zustand an

#### Scenario: Initialzustand konfigurierbar

- **WHEN** die Komponente mit `default_open: true` instanziiert wird
- **THEN** der Inhaltsbereich ist beim ersten Rendern sichtbar

### Requirement: Konfigurationsseite mit eingeklappten Sektionen

Beim Aufruf der Konfigurationsseite (`/config`) SHALL jede der bestehenden Bereiche (SMTP, Mail-Footer, IMAP-Posteingang, WebDAV-Backup, TSA, WordPress-Integration, generische Config-Entries) als eigene `CollapsibleSection` gerendert werden, wobei alle Sektionen standardmäßig eingeklappt sind.

#### Scenario: Erstes Aufrufen der Seite

- **WHEN** der Nutzer `/config` aufruft
- **THEN** alle Sektionen werden mit Headern angezeigt
- **AND** der Inhalt jeder Sektion ist eingeklappt

#### Scenario: Mehrere Sektionen gleichzeitig öffnen

- **WHEN** der Nutzer eine Sektion aufklappt und danach eine zweite
- **THEN** beide Sektionen sind gleichzeitig geöffnet
- **AND** keine Sektion wird automatisch geschlossen

#### Scenario: Inhalte der Sektionen unverändert

- **WHEN** eine Sektion aufgeklappt wird
- **THEN** der gezeigte Inhalt entspricht exakt den heutigen Eingabefeldern, Knöpfen und Anzeigen der jeweiligen Sektion
- **AND** das Verhalten der enthaltenen Steuerelemente bleibt unverändert
