## Purpose

Die Mitglieds-Detailseite wird aufgeräumt, indem kontextabhängige UI-Elemente nur bei Bedarf angezeigt werden und Vergleiche gegen typsichere Enum-Werte statt Hardcode-Strings erfolgen.

## Requirements

### Requirement: Generate-Knopf für Eintrittsbestätigung nur ohne vorhandenes Dokument

Auf der Mitglieds-Detailseite SHALL der Knopf „Eintrittsbestätigung generieren" nur sichtbar sein, wenn für das Mitglied noch kein Dokument vom Typ `JoinConfirmation` existiert.

#### Scenario: Kein Dokument vorhanden

- **WHEN** das Mitglied keine Dokumente vom Typ `JoinConfirmation` besitzt
- **THEN** der Knopf „Eintrittsbestätigung generieren" wird im Dokumente-Bereich angezeigt

#### Scenario: Dokument bereits vorhanden

- **WHEN** das Mitglied mindestens ein Dokument vom Typ `JoinConfirmation` besitzt
- **THEN** der Knopf „Eintrittsbestätigung generieren" wird nicht angezeigt
- **AND** das vorhandene Dokument bleibt in der Dokumente-Liste sichtbar

#### Scenario: Nach erfolgreicher Generierung

- **WHEN** der Nutzer den Knopf klickt und die Generierung erfolgreich war
- **AND** die Dokumente-Liste neu geladen wurde
- **THEN** der Knopf wird nicht mehr angezeigt

### Requirement: Migrationsstatus nur bei Auffälligkeit anzeigen

Der Migrationsstatus-Block auf der Mitglieds-Detailseite SHALL nur dann angezeigt werden, wenn der Status nicht `migrated` ist. Im Normalfall (Status `migrated`) wird nichts angezeigt.

#### Scenario: Mitglied ist migriert

- **WHEN** der Migrationsstatus eines Mitglieds `migrated` ist
- **THEN** kein Migrationsstatus-Element wird auf der Detailseite gerendert

#### Scenario: Mitglied ist nicht migriert

- **WHEN** der Migrationsstatus eines Mitglieds nicht `migrated` ist (z. B. `pending`)
- **THEN** der bestehende Statusblock mit erwarteten/tatsächlichen Anteilen, Aktionen und Bestätigungs-Knopf wird unverändert angezeigt

#### Scenario: Migrationsstatus noch nicht geladen

- **WHEN** der Migrationsstatus für ein Mitglied noch nicht abgerufen wurde oder ein Ladefehler vorliegt
- **THEN** kein Migrationsstatus-Element wird gerendert
- **AND** die Detailseite ist sonst voll funktionsfähig

### Requirement: Vergleich gegen Enum-Werte statt Hardcode-Strings

Vergleiche zwischen Frontend-State und Dokumenttyp-Bezeichnern SHALL gegen den serialisierten Wert des `DocumentTypeTO`-Enums (`as_str()`) erfolgen, nicht gegen Hardcode-String-Literale.

#### Scenario: Refactor des Vergleichs

- **WHEN** die Sichtbarkeit des Generate-Knopfes für `JoinConfirmation` ermittelt wird
- **THEN** der Vergleich nutzt `DocumentTypeTO::JoinConfirmation.as_str()` als rechte Seite
