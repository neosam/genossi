## 1. Frontend REST-Types

- [x] 1.1 `ActionTypeTO` Enum in `genossi-frontend/rest-types/src/lib.rs` definieren (Eintritt, Austritt, Todesfall, Aufstockung, Verkauf, UebertragungEmpfang, UebertragungAbgabe)
- [x] 1.2 `MemberActionTO` Struct mit Serde und Datumsformatierung
- [x] 1.3 `MigrationStatusTO` Struct

## 2. API-Funktionen

- [x] 2.1 `get_member_actions(member_id)` in `api.rs`
- [x] 2.2 `create_member_action(member_id, action)` in `api.rs`
- [x] 2.3 `update_member_action(member_id, action_id, action)` in `api.rs`
- [x] 2.4 `delete_member_action(member_id, action_id)` in `api.rs`
- [x] 2.5 `get_migration_status(member_id)` in `api.rs`

## 3. i18n

- [x] 3.1 Neue Keys in `i18n/mod.rs`: Aktionstypen, Aktionen-UI-Labels (Actions, ActionType, Date, SharesChange, TransferMember, EffectiveDate, MigrationStatus, Migrated, Pending, NewAction, EditAction, ExpectedShares, ActualShares, ExpectedActionCount, ActualActionCount)
- [x] 3.2 Deutsche Uebersetzungen in `i18n/de.rs`
- [x] 3.3 Englische Uebersetzungen in `i18n/en.rs`

## 4. Member-Detail-Seite: Aktionen

- [x] 4.1 Aktionen-State (Vec<MemberActionTO>) und Migrations-Status-State als Signals auf der Member-Detail-Seite
- [x] 4.2 Aktionen und Migrations-Status beim Laden des Members parallel abrufen (use_effect)
- [x] 4.3 Aktionen-Liste unterhalb des Member-Formulars rendern (Tabelle mit action_type, date, shares_change, comment)
- [x] 4.4 Inline-Formular fuer Aktion erstellen/bearbeiten mit konditionalen Feldern je nach ActionType
- [x] 4.5 Aktion loeschen mit Bestaetigung
- [x] 4.6 Migrations-Status-Badge oben auf der Seite (gruen=migriert, orange=ausstehend mit Details)
- [x] 4.7 Aktionen-Sektion und Migrations-Status nur fuer bestehende Mitglieder anzeigen (nicht bei "new")
