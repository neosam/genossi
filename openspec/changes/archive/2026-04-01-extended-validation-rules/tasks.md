## 1. Service Layer - Neue Typen

- [x] 1.1 Erweitere `genossi_service/src/validation.rs` um neue Structs: `SharesMismatch`, `MissingEntryAction`, `ExitDateMismatch`, `ActiveMemberNoShares`, `DuplicateMemberNumber`, `ExitedMemberWithShares`, `MigratedFlagMismatch`
- [x] 1.2 Erweitere `ValidationResult` um die neuen Felder für alle 7 Regeln

## 2. Service Layer - Implementierung der Regeln

- [x] 2.1 Implementiere `find_shares_mismatches` (current_shares vs. Summe shares_change pro Mitglied)
- [x] 2.2 Implementiere `find_missing_entry_actions` (genau eine Eintritt-Aktion pro Mitglied prüfen)
- [x] 2.3 Implementiere `find_exit_date_mismatches` (exit_date vs. Austritt-Aktion Konsistenz)
- [x] 2.4 Implementiere `find_active_members_no_shares` (aktive Mitglieder mit current_shares <= 0)
- [x] 2.5 Implementiere `find_duplicate_member_numbers` (doppelte Nummern bei aktiven Mitgliedern)
- [x] 2.6 Implementiere `find_exited_members_with_shares` (ausgetretene Mitglieder mit current_shares > 0)
- [x] 2.7 Implementiere `find_migrated_flag_mismatches` (migrated-Flag vs. compute_migration_status)
- [x] 2.8 Integriere alle neuen Regeln in die `validate()`-Methode

## 3. Unit Tests

- [x] 3.1 Tests für Shares-Konsistenz (match, mismatch, keine Aktionen)
- [x] 3.2 Tests für Eintritt-Aktion (vorhanden, fehlt, doppelt)
- [x] 3.3 Tests für Exit-Date-Konsistenz (beide vorhanden, beide fehlen, exit_date ohne Austritt, Austritt ohne exit_date)
- [x] 3.4 Tests für aktive Mitglieder ohne Anteile (mit Anteilen, ohne Anteile, ausgetreten ohne Anteile)
- [x] 3.5 Tests für doppelte Mitgliedsnummern (keine Duplikate, Duplikate, soft-deleted zählt nicht)
- [x] 3.6 Tests für ausgetretene Mitglieder mit Anteilen (ohne Anteile, mit Anteilen, aktiv mit Anteilen)
- [x] 3.7 Tests für Migrated-Flag (stimmt überein, true/Pending, false/Migrated)

## 4. REST Layer

- [x] 4.1 Erweitere Transfer Objects in `genossi_rest_types/src/lib.rs` um die neuen Structs und Konvertierungen
- [x] 4.2 Erweitere Frontend REST-Types in `genossi-frontend/rest-types/src/lib.rs`

## 5. E2E Tests

- [x] 5.1 E2E-Test: Validation erkennt Shares-Mismatch
- [x] 5.2 E2E-Test: Validation erkennt fehlende Eintritt-Aktion

## 6. Frontend

- [x] 6.1 Neue i18n-Keys für alle 7 Regeln (de + en)
- [x] 6.2 Erweitere Validierungsseite um Sektionen für alle neuen Regeln mit Schweregrad-Farben (Rot/Gelb/Blau)
