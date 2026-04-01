## ADDED Requirements

### Requirement: Shares-Konsistenz prüfen
Das System MUSS für jedes Mitglied prüfen, ob `current_shares` mit der Summe aller `shares_change`-Werte seiner Aktionen übereinstimmt. Abweichungen MÜSSEN als `SharesMismatch` mit member_id, member_number, expected (current_shares) und actual (Summe shares_change) gemeldet werden.

#### Scenario: Shares stimmen überein
- **WHEN** ein Mitglied `current_shares = 5` hat und die Summe seiner shares_change-Werte 5 ergibt
- **THEN** wird kein SharesMismatch gemeldet

#### Scenario: Shares divergieren
- **WHEN** ein Mitglied `current_shares = 5` hat aber die Summe seiner shares_change-Werte 3 ergibt
- **THEN** wird ein SharesMismatch mit expected=5 und actual=3 gemeldet

#### Scenario: Mitglied ohne Aktionen
- **WHEN** ein Mitglied `current_shares = 3` hat aber keine Aktionen besitzt
- **THEN** wird ein SharesMismatch mit expected=3 und actual=0 gemeldet

### Requirement: Eintritt-Aktion prüfen
Das System MUSS für jedes aktive Mitglied prüfen, ob genau eine Eintritt-Aktion existiert. Fehlende oder doppelte Eintritte MÜSSEN als `MissingEntryAction` mit member_id, member_number und actual_count gemeldet werden.

#### Scenario: Genau ein Eintritt vorhanden
- **WHEN** ein Mitglied genau eine Eintritt-Aktion hat
- **THEN** wird kein MissingEntryAction gemeldet

#### Scenario: Kein Eintritt vorhanden
- **WHEN** ein Mitglied keine Eintritt-Aktion hat
- **THEN** wird ein MissingEntryAction mit actual_count=0 gemeldet

#### Scenario: Mehrere Eintritte vorhanden
- **WHEN** ein Mitglied zwei Eintritt-Aktionen hat
- **THEN** wird ein MissingEntryAction mit actual_count=2 gemeldet

### Requirement: Exit-Date/Austritt-Konsistenz prüfen
Das System MUSS prüfen, ob `exit_date` und Austritt-Aktionen konsistent sind. Inkonsistenzen MÜSSEN als `ExitDateMismatch` mit member_id, member_number, has_exit_date und has_austritt_action gemeldet werden.

#### Scenario: Exit-Date und Austritt-Aktion vorhanden
- **WHEN** ein Mitglied `exit_date` gesetzt hat und eine Austritt-Aktion existiert
- **THEN** wird kein ExitDateMismatch gemeldet

#### Scenario: Kein Exit-Date und keine Austritt-Aktion
- **WHEN** ein Mitglied kein `exit_date` hat und keine Austritt-Aktion existiert
- **THEN** wird kein ExitDateMismatch gemeldet

#### Scenario: Exit-Date ohne Austritt-Aktion
- **WHEN** ein Mitglied `exit_date` gesetzt hat aber keine Austritt-Aktion existiert
- **THEN** wird ein ExitDateMismatch mit has_exit_date=true und has_austritt_action=false gemeldet

#### Scenario: Austritt-Aktion ohne Exit-Date
- **WHEN** ein Mitglied eine Austritt-Aktion hat aber kein `exit_date` gesetzt ist
- **THEN** wird ein ExitDateMismatch mit has_exit_date=false und has_austritt_action=true gemeldet

### Requirement: Aktive Mitglieder ohne Anteile erkennen
Das System MUSS aktive Mitglieder (ohne `exit_date`) mit `current_shares <= 0` als `ActiveMemberNoShares` mit member_id und member_number melden.

#### Scenario: Aktives Mitglied mit Anteilen
- **WHEN** ein aktives Mitglied `current_shares = 3` hat
- **THEN** wird kein ActiveMemberNoShares gemeldet

#### Scenario: Aktives Mitglied ohne Anteile
- **WHEN** ein aktives Mitglied `current_shares = 0` hat und kein `exit_date` gesetzt ist
- **THEN** wird ein ActiveMemberNoShares gemeldet

#### Scenario: Ausgetretenes Mitglied ohne Anteile
- **WHEN** ein Mitglied mit `exit_date` `current_shares = 0` hat
- **THEN** wird kein ActiveMemberNoShares gemeldet

### Requirement: Doppelte Mitgliedsnummern erkennen
Das System MUSS prüfen, ob aktive Mitglieder (nicht soft-deleted) mit identischer `member_number` existieren. Duplikate MÜSSEN als `DuplicateMemberNumber` mit member_number und member_ids (Liste der betroffenen IDs) gemeldet werden.

#### Scenario: Keine Duplikate
- **WHEN** alle aktiven Mitglieder eindeutige Nummern haben
- **THEN** werden keine DuplicateMemberNumber gemeldet

#### Scenario: Zwei aktive Mitglieder mit gleicher Nummer
- **WHEN** zwei aktive Mitglieder die Nummer 42 haben
- **THEN** wird ein DuplicateMemberNumber mit member_number=42 und beiden IDs gemeldet

#### Scenario: Soft-deleted Duplikat zählt nicht
- **WHEN** ein aktives Mitglied und ein soft-deleted Mitglied die gleiche Nummer haben
- **THEN** wird kein DuplicateMemberNumber gemeldet

### Requirement: Ausgetretene Mitglieder mit verbleibenden Anteilen erkennen
Das System MUSS Mitglieder mit `exit_date` und `current_shares > 0` als `ExitedMemberWithShares` mit member_id, member_number und current_shares melden. Dies ist ein Info-Level-Hinweis (z.B. ausstehende Rückerstattung).

#### Scenario: Ausgetretenes Mitglied ohne Anteile
- **WHEN** ein Mitglied mit `exit_date` `current_shares = 0` hat
- **THEN** wird kein ExitedMemberWithShares gemeldet

#### Scenario: Ausgetretenes Mitglied mit Anteilen
- **WHEN** ein Mitglied mit `exit_date` `current_shares = 3` hat
- **THEN** wird ein ExitedMemberWithShares mit current_shares=3 gemeldet

#### Scenario: Aktives Mitglied mit Anteilen
- **WHEN** ein aktives Mitglied (ohne `exit_date`) `current_shares = 3` hat
- **THEN** wird kein ExitedMemberWithShares gemeldet

### Requirement: Migrated-Flag-Konsistenz prüfen
Das System MUSS prüfen, ob das `migrated`-Flag jedes Mitglieds mit dem berechneten Migrationsstatus (`compute_migration_status`) übereinstimmt. Abweichungen MÜSSEN als `MigratedFlagMismatch` mit member_id, member_number, flag_value und computed_status gemeldet werden.

#### Scenario: Flag stimmt mit berechnetem Status überein
- **WHEN** ein Mitglied `migrated = true` hat und `compute_migration_status` den Status "Migrated" liefert
- **THEN** wird kein MigratedFlagMismatch gemeldet

#### Scenario: Flag ist true aber Status ist Pending
- **WHEN** ein Mitglied `migrated = true` hat aber `compute_migration_status` den Status "Pending" liefert
- **THEN** wird ein MigratedFlagMismatch mit flag_value=true und computed_status="Pending" gemeldet

#### Scenario: Flag ist false aber Status ist Migrated
- **WHEN** ein Mitglied `migrated = false` hat aber `compute_migration_status` den Status "Migrated" liefert
- **THEN** wird ein MigratedFlagMismatch mit flag_value=false und computed_status="Migrated" gemeldet
