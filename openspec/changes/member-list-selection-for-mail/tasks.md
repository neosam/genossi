## 1. GlobalSignal und State

- [x] 1.1 `SELECTED_MEMBER_IDS: GlobalSignal<Vec<Uuid>>` erstellen (neues Modul oder in bestehendem State-Modul)
- [x] 1.2 Hilfsfunktionen: `toggle_member_selection(id)`, `select_all(ids)`, `clear_selection()`

## 2. Mitgliederliste: Checkbox-Spalte

- [x] 2.1 Checkbox-Spalte als erste Spalte im Tabellenkopf hinzufügen (Select-All Checkbox)
- [x] 2.2 Checkbox pro Zeile mit mobilfreundlichem Touch-Target (min 44x44px), `stop_propagation` auf der Checkbox-Zelle
- [x] 2.3 Indeterminate/Checked/Unchecked-Zustand der Kopf-Checkbox basierend auf gefilterter Selektion

## 3. Mitgliederliste: Aktionsleiste

- [x] 3.1 Aktionsleiste einbauen, die bei `selected_count > 0` erscheint (Anzahl + "Mail senden"-Button)
- [x] 3.2 "Mail senden"-Button setzt `SELECTED_MEMBER_IDS` und navigiert zu `/mail`

## 4. Mail-Seite: Vorauswahl übernehmen

- [x] 4.1 Beim Mount `SELECTED_MEMBER_IDS` lesen, als initiale `selected_member_ids` setzen, Signal leeren

## 5. Mail-Seite: Skalierbare Empfänger-Darstellung

- [x] 5.1 Bestehende Chip-Darstellung durch eingeklappte Zusammenfassung ersetzen (Zähler + Warnung für fehlende E-Mails)
- [x] 5.2 Aufklappbare scrollbare Liste mit Name, E-Mail und Entfernen-Button pro Zeile
- [x] 5.3 Autocomplete-Suche für manuelles Hinzufügen beibehalten

## 6. i18n

- [x] 6.1 Neue Übersetzungsschlüssel für Selektions-UI (SelectAll, SelectedCount, SendMailToSelected, Recipients, NoEmail, ShowRecipients, HideRecipients) in allen Sprachen (en, de, cs)

## 7. Tests

- [x] 7.1 Tests für Selektions-Logik (toggle, select-all, clear)
- [x] 7.2 Sicherstellen, dass bestehende Tests weiterhin bestehen (cargo test)
