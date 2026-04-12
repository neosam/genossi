## MODIFIED Requirements

### Requirement: Shares-Konsistenz prüfen
Das System MUSS für jedes Mitglied mit Status `Normal` prüfen, ob `current_shares` mit der Summe aller `shares_change`-Werte seiner Aktionen übereinstimmt. Mitglieder mit Status `FehlerhaftErfasst` MÜSSEN von dieser Prüfung ausgenommen werden. Abweichungen MÜSSEN als `SharesMismatch` mit member_id, member_number, expected (current_shares) und actual (Summe shares_change) gemeldet werden.

#### Scenario: Shares stimmen überein
- **WHEN** ein Mitglied mit Status `Normal` `current_shares = 5` hat und die Summe seiner shares_change-Werte 5 ergibt
- **THEN** wird kein SharesMismatch gemeldet

#### Scenario: Shares divergieren
- **WHEN** ein Mitglied mit Status `Normal` `current_shares = 5` hat aber die Summe seiner shares_change-Werte 3 ergibt
- **THEN** wird ein SharesMismatch mit expected=5 und actual=3 gemeldet

#### Scenario: Mitglied ohne Aktionen
- **WHEN** ein Mitglied mit Status `Normal` `current_shares = 3` hat aber keine Aktionen besitzt
- **THEN** wird ein SharesMismatch mit expected=3 und actual=0 gemeldet

#### Scenario: FehlerhaftErfasst wird übersprungen
- **WHEN** ein Mitglied mit Status `FehlerhaftErfasst` inkonsistente Shares hat
- **THEN** wird kein SharesMismatch gemeldet

### Requirement: Eintritt-Aktion prüfen
Das System MUSS für jedes aktive Mitglied mit Status `Normal` prüfen, ob genau eine Eintritt-Aktion existiert. Mitglieder mit Status `FehlerhaftErfasst` MÜSSEN von dieser Prüfung ausgenommen werden. Fehlende oder doppelte Eintritte MÜSSEN als `MissingEntryAction` mit member_id, member_number und actual_count gemeldet werden.

#### Scenario: Genau ein Eintritt vorhanden
- **WHEN** ein Mitglied mit Status `Normal` genau eine Eintritt-Aktion hat
- **THEN** wird kein MissingEntryAction gemeldet

#### Scenario: Kein Eintritt vorhanden
- **WHEN** ein Mitglied mit Status `Normal` keine Eintritt-Aktion hat
- **THEN** wird ein MissingEntryAction mit actual_count=0 gemeldet

#### Scenario: Mehrere Eintritte vorhanden
- **WHEN** ein Mitglied mit Status `Normal` zwei Eintritt-Aktionen hat
- **THEN** wird ein MissingEntryAction mit actual_count=2 gemeldet

#### Scenario: FehlerhaftErfasst wird übersprungen
- **WHEN** ein Mitglied mit Status `FehlerhaftErfasst` keine Eintritt-Aktion hat
- **THEN** wird kein MissingEntryAction gemeldet

### Requirement: Exit-Date/Austritt-Konsistenz prüfen
Das System MUSS für Mitglieder mit Status `Normal` prüfen, ob `exit_date` und Austritt-Aktionen konsistent sind. Mitglieder mit Status `FehlerhaftErfasst` MÜSSEN von dieser Prüfung ausgenommen werden. Inkonsistenzen MÜSSEN als `ExitDateMismatch` mit member_id, member_number, has_exit_date und has_austritt_action gemeldet werden.

#### Scenario: Exit-Date und Austritt-Aktion vorhanden
- **WHEN** ein Mitglied mit Status `Normal` `exit_date` gesetzt hat und eine Austritt-Aktion existiert
- **THEN** wird kein ExitDateMismatch gemeldet

#### Scenario: Kein Exit-Date und keine Austritt-Aktion
- **WHEN** ein Mitglied mit Status `Normal` kein `exit_date` hat und keine Austritt-Aktion existiert
- **THEN** wird kein ExitDateMismatch gemeldet

#### Scenario: Exit-Date ohne Austritt-Aktion
- **WHEN** ein Mitglied mit Status `Normal` `exit_date` gesetzt hat aber keine Austritt-Aktion existiert
- **THEN** wird ein ExitDateMismatch mit has_exit_date=true und has_austritt_action=false gemeldet

#### Scenario: Austritt-Aktion ohne Exit-Date
- **WHEN** ein Mitglied mit Status `Normal` eine Austritt-Aktion hat aber kein `exit_date` gesetzt ist
- **THEN** wird ein ExitDateMismatch mit has_exit_date=false und has_austritt_action=true gemeldet

#### Scenario: FehlerhaftErfasst wird übersprungen
- **WHEN** ein Mitglied mit Status `FehlerhaftErfasst` ein `exit_date` ohne Austritt-Aktion hat
- **THEN** wird kein ExitDateMismatch gemeldet

### Requirement: Aktive Mitglieder ohne Anteile erkennen
Das System MUSS aktive Mitglieder mit Status `Normal` (ohne `exit_date`) mit `current_shares <= 0` als `ActiveMemberNoShares` mit member_id und member_number melden. Mitglieder mit Status `FehlerhaftErfasst` MÜSSEN von dieser Prüfung ausgenommen werden.

#### Scenario: Aktives Mitglied mit Anteilen
- **WHEN** ein aktives Mitglied mit Status `Normal` `current_shares = 3` hat
- **THEN** wird kein ActiveMemberNoShares gemeldet

#### Scenario: Aktives Mitglied ohne Anteile
- **WHEN** ein aktives Mitglied mit Status `Normal` `current_shares = 0` hat und kein `exit_date` gesetzt ist
- **THEN** wird ein ActiveMemberNoShares gemeldet

#### Scenario: Ausgetretenes Mitglied ohne Anteile
- **WHEN** ein Mitglied mit `exit_date` `current_shares = 0` hat
- **THEN** wird kein ActiveMemberNoShares gemeldet

#### Scenario: FehlerhaftErfasst wird übersprungen
- **WHEN** ein Mitglied mit Status `FehlerhaftErfasst` ohne `exit_date` `current_shares = 0` hat
- **THEN** wird kein ActiveMemberNoShares gemeldet

### Requirement: Ausgetretene Mitglieder mit verbleibenden Anteilen erkennen
Das System MUSS Mitglieder mit Status `Normal`, `exit_date` und `current_shares > 0` als `ExitedMemberWithShares` mit member_id, member_number und current_shares melden. Mitglieder mit Status `FehlerhaftErfasst` MÜSSEN von dieser Prüfung ausgenommen werden. Dies ist ein Info-Level-Hinweis (z.B. ausstehende Rückerstattung).

#### Scenario: Ausgetretenes Mitglied ohne Anteile
- **WHEN** ein Mitglied mit Status `Normal` und `exit_date` `current_shares = 0` hat
- **THEN** wird kein ExitedMemberWithShares gemeldet

#### Scenario: Ausgetretenes Mitglied mit Anteilen
- **WHEN** ein Mitglied mit Status `Normal` und `exit_date` `current_shares = 3` hat
- **THEN** wird ein ExitedMemberWithShares mit current_shares=3 gemeldet

#### Scenario: Aktives Mitglied mit Anteilen
- **WHEN** ein aktives Mitglied (ohne `exit_date`) `current_shares = 3` hat
- **THEN** wird kein ExitedMemberWithShares gemeldet

#### Scenario: FehlerhaftErfasst wird übersprungen
- **WHEN** ein Mitglied mit Status `FehlerhaftErfasst` und `exit_date` `current_shares > 0` hat
- **THEN** wird kein ExitedMemberWithShares gemeldet

### Requirement: Migrated-Flag-Konsistenz prüfen
Das System MUSS für Mitglieder mit Status `Normal` prüfen, ob das `migrated`-Flag mit dem berechneten Migrationsstatus (`compute_migration_status`) übereinstimmt. Mitglieder mit Status `FehlerhaftErfasst` MÜSSEN von dieser Prüfung ausgenommen werden. Abweichungen MÜSSEN als `MigratedFlagMismatch` mit member_id, member_number, flag_value und computed_status gemeldet werden.

#### Scenario: Flag stimmt mit berechnetem Status überein
- **WHEN** ein Mitglied mit Status `Normal` `migrated = true` hat und `compute_migration_status` den Status "Migrated" liefert
- **THEN** wird kein MigratedFlagMismatch gemeldet

#### Scenario: Flag ist true aber Status ist Pending
- **WHEN** ein Mitglied mit Status `Normal` `migrated = true` hat aber `compute_migration_status` den Status "Pending" liefert
- **THEN** wird ein MigratedFlagMismatch mit flag_value=true und computed_status="Pending" gemeldet

#### Scenario: Flag ist false aber Status ist Migrated
- **WHEN** ein Mitglied mit Status `Normal` `migrated = false` hat aber `compute_migration_status` den Status "Migrated" liefert
- **THEN** wird ein MigratedFlagMismatch mit flag_value=false und computed_status="Migrated" gemeldet

#### Scenario: FehlerhaftErfasst wird übersprungen
- **WHEN** ein Mitglied mit Status `FehlerhaftErfasst` ein nicht übereinstimmendes `migrated`-Flag hat
- **THEN** wird kein MigratedFlagMismatch gemeldet
