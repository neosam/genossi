## Why

Die Validierung prüft Mitgliedsdaten auf Konsistenz (Eintritts-Aktionen, Anteile, Austrittsdaten usw.). Diese Prüfungen setzen voraus, dass ein Mitglied tatsächlich beigetreten ist. Mitglieder mit Status `FehlerhaftErfasst` waren jedoch nie echte Mitglieder — sie entstanden durch Importfehler aus der alten Excel-Liste. Die Validierung meldet für diese Mitglieder falsche Fehler, was die echten Probleme verschleiert. Zusätzlich werden beim Anlegen eines `FehlerhaftErfasst`-Mitglieds automatisch Eintritts- und Aufstockungsaktionen erzeugt, die manuell gelöscht werden müssen.

## What Changes

- **Validierung**: 6 Validierungschecks überspringen `FehlerhaftErfasst`-Mitglieder:
  - `find_shares_mismatches` — Anteile-Konsistenz
  - `find_missing_entry_actions` — Fehlende Eintritts-Aktion
  - `find_exit_date_mismatches` — Austrittsdatum vs. Austritts-Aktion
  - `find_active_members_no_shares` — Aktives Mitglied ohne Anteile
  - `find_exited_members_with_shares` — Ausgetretenes Mitglied mit Anteilen
  - `find_migrated_flag_mismatches` — Migrations-Flag-Inkonsistenz
- **Mitglied-Erstellung**: Keine automatischen Eintritts- und Aufstockungsaktionen bei `FehlerhaftErfasst`-Status. `current_shares` wird auf 0 gesetzt.
- Nummern-Lücken und Duplikat-Prüfungen bleiben unverändert (Mitgliedsnummer existiert trotzdem).
- Kein automatischer Statuswechsel bei Update — bestehende Actions werden nicht berührt.

## Capabilities

### New Capabilities

_(keine)_

### Modified Capabilities

- `data-validation`: Validierungschecks filtern `FehlerhaftErfasst`-Mitglieder heraus
- `member-management`: Erstellung überspringt automatische Actions bei `FehlerhaftErfasst`

## Impact

- `genossi_service_impl/src/validation.rs` — 6 Funktionen erhalten zusätzlichen Filter
- `genossi_service_impl/src/member.rs` — `create` Methode: bedingte Action-Erstellung
- Bestehende Tests müssen um `FehlerhaftErfasst`-Szenarien erweitert werden
- Keine API-Änderungen, keine Datenbankänderungen, keine Breaking Changes
