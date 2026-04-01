## Why

Bei Übertragungs-Aktionen (Empfang/Abgabe) muss aktuell eine UUID manuell eingegeben werden, um das Gegenmitglied zu referenzieren. Das ist nicht benutzerfreundlich — niemand kennt UUIDs auswendig. Eine Suche nach Name oder Mitgliedsnummer ist nötig.

## What Changes

- Neue wiederverwendbare `MemberSearch`-Komponente im Frontend mit Autocomplete/Typeahead-Funktionalität
- Clientseitiges Filtern der bereits geladenen Mitgliederliste (`MEMBERS` GlobalSignal) nach Name und Mitgliedsnummer
- Ersetzen des UUID-Textfelds im Aktions-Formular auf der Member-Details-Seite durch die neue `MemberSearch`-Komponente
- Möglichkeit, das aktuelle Mitglied von der Suche auszuschließen (kein Übertrag an sich selbst)

## Capabilities

### New Capabilities
- `member-search`: Wiederverwendbare Autocomplete-Komponente zur Mitgliedersuche mit clientseitigem Filtern nach Name und Nummer

### Modified Capabilities

## Impact

- `genossi-frontend/src/component/` — neue Datei `member_search.rs`
- `genossi-frontend/src/component/mod.rs` — Export der neuen Komponente
- `genossi-frontend/src/page/member_details.rs` — UUID-Textfeld durch `MemberSearch` ersetzen
- Kein Backend-Impact — es wird die bestehende Mitgliederliste verwendet
