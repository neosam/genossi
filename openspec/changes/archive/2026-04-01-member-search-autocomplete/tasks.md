## 1. MemberSearch-Komponente erstellen

- [x] 1.1 Neue Datei `genossi-frontend/src/component/member_search.rs` erstellen mit `MemberSearch`-Komponente (Props: `on_select`, `selected_id`, `exclude_id`)
- [x] 1.2 Suchlogik implementieren: clientseitiges Filtern auf `MEMBERS.items` nach `first_name`, `last_name`, `member_number` (case-insensitive, Teilstring)
- [x] 1.3 Dropdown-Ergebnisliste mit max. 10 Einträgen, sortiert nach Mitgliedsnummer, Format "#Nr Vorname Nachname"
- [x] 1.4 Ausgewähltes Mitglied anzeigen (Name + Nummer) mit ✕-Button zum Zurücksetzen
- [x] 1.5 Dropdown schließen bei Klick außerhalb (onfocusout)
- [x] 1.6 Komponente in `genossi-frontend/src/component/mod.rs` exportieren

## 2. Integration in Aktions-Formular

- [x] 2.1 UUID-Textfeld in `member_details.rs` durch `MemberSearch`-Komponente ersetzen
- [x] 2.2 `exclude_id` auf die ID des aktuellen Mitglieds setzen
- [x] 2.3 `on_select`-Callback mit `action_transfer_member_id`-Signal verbinden (UUID als String setzen)

## 3. Tests

- [x] 3.1 Tests für die Filterlogik der MemberSearch-Komponente (Namenssuche, Nummernsuche, Exclude, Limit)
