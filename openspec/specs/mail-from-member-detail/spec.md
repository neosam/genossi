# Mail From Member Detail

## Purpose

Ermöglicht es Nutzern, direkt aus der Mitglieds-Detailseite einen Mail-Versand an das betrachtete Mitglied zu starten. Der Mail-Versand wird mit dem aktuellen Mitglied als vorausgewähltem Empfänger geöffnet, indem der globale `SELECTED_MEMBER_IDS`-State entsprechend gesetzt wird.

## Requirements

### Requirement: Knopf „Mail senden" auf der Mitglieds-Detailseite

Die Mitglieds-Detailseite SHALL einen Knopf „Mail senden" im Aktionsbereich der Seite zeigen, mit dem der Nutzer direkt in den Mail-Versand mit diesem Mitglied als Empfänger springen kann.

#### Scenario: Mitglied mit E-Mail-Adresse

- **WHEN** ein Mitglied mit hinterlegter E-Mail-Adresse betrachtet wird
- **THEN** der Knopf „Mail senden" wird angezeigt und ist klickbar

#### Scenario: Mitglied ohne E-Mail-Adresse

- **WHEN** ein Mitglied ohne hinterlegte E-Mail-Adresse betrachtet wird
- **THEN** der Knopf „Mail senden" wird angezeigt
- **AND** der Knopf ist disabled
- **AND** ein Hinweis zeigt an, dass keine E-Mail-Adresse hinterlegt ist

#### Scenario: Mitglied im Anlegen-Modus (noch nicht gespeichert)

- **WHEN** die Detailseite im Anlegen-Modus für ein neues Mitglied geöffnet ist
- **THEN** der Knopf „Mail senden" wird nicht angezeigt

### Requirement: Vorauswahl des Empfängers via SELECTED_MEMBER_IDS

Beim Klick auf „Mail senden" SHALL der globale `SELECTED_MEMBER_IDS`-State auf genau dieses eine Mitglied gesetzt werden, und die Anwendung navigiert zu `/mail`.

#### Scenario: Klick auf den Knopf

- **WHEN** der Nutzer den Knopf „Mail senden" klickt
- **THEN** der globale `SELECTED_MEMBER_IDS`-State enthält ausschließlich die ID des aktuellen Mitglieds
- **AND** die Anwendung navigiert auf `/mail`
- **AND** auf `/mail` ist das Mitglied bereits als Empfänger ausgewählt

#### Scenario: Vorherige Auswahl wird ersetzt

- **WHEN** vor dem Klick bereits andere Mitglieder im `SELECTED_MEMBER_IDS`-State waren
- **WHEN** der Nutzer den Knopf „Mail senden" klickt
- **THEN** der State enthält nur noch das aktuelle Mitglied; die vorherige Auswahl ist verworfen
