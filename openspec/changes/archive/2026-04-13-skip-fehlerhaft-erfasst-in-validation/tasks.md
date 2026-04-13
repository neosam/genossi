## 1. Validierung: FehlerhaftErfasst-Filter

- [x] 1.1 `find_shares_mismatches` um `.filter(|m| m.status.is_normal())` erweitern
- [x] 1.2 `find_missing_entry_actions` um `.filter(|m| m.status.is_normal())` erweitern
- [x] 1.3 `find_exit_date_mismatches` um `.filter(|m| m.status.is_normal())` erweitern
- [x] 1.4 `find_active_members_no_shares` um `.filter(|m| m.status.is_normal())` erweitern
- [x] 1.5 `find_exited_members_with_shares` um `.filter(|m| m.status.is_normal())` erweitern
- [x] 1.6 `find_migrated_flag_mismatches` um `.filter(|m| m.status.is_normal())` erweitern

## 2. Mitglied-Erstellung: Bedingte Actions

- [x] 2.1 Eintritt- und Aufstockungsaktionen in `create` mit `if item.status.is_normal()` umschließen
- [x] 2.2 `current_shares` auf 0 setzen, wenn Status `FehlerhaftErfasst` ist

## 3. Tests: Validierung

- [x] 3.1 Test: `find_shares_mismatches` überspringt `FehlerhaftErfasst`
- [x] 3.2 Test: `find_missing_entry_actions` überspringt `FehlerhaftErfasst`
- [x] 3.3 Test: `find_exit_date_mismatches` überspringt `FehlerhaftErfasst`
- [x] 3.4 Test: `find_active_members_no_shares` überspringt `FehlerhaftErfasst`
- [x] 3.5 Test: `find_exited_members_with_shares` überspringt `FehlerhaftErfasst`
- [x] 3.6 Test: `find_migrated_flag_mismatches` überspringt `FehlerhaftErfasst`

## 4. Tests: Mitglied-Erstellung

- [x] 4.1 E2E-Test: Erstellung mit `FehlerhaftErfasst` erzeugt keine Actions und setzt `current_shares = 0`
- [x] 4.2 E2E-Test: Erstellung mit `Normal` erzeugt weiterhin Eintritt- und Aufstockungsaktionen
