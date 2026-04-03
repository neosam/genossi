## 1. Backend: Validierung effective_date Pflicht bei Austritt

- [x] 1.1 In `validate_action()` in `genossi_service_impl/src/member_action.rs` Validierung hinzufügen: Wenn `action_type == Austritt` und `effective_date.is_none()`, Validierungsfehler zurückgeben
- [x] 1.2 Unit-Tests für die neue Validierungsregel: Austritt ohne effective_date schlägt fehl, Austritt mit effective_date ist valide

## 2. Backend: recalc_dates() Implementierung

- [x] 2.1 Neue Funktion `recalc_dates(member_id, tx)` in `genossi_service_impl/src/member_action.rs` implementieren: Eintritt-Aktion → `join_date`, Austritt → `exit_date = effective_date`, Todesfall → `exit_date = date`, kein Exit → `exit_date = None`, kein Eintritt → `join_date` unverändert
- [x] 2.2 `recalc_dates()` in `MemberActionServiceImpl::create()`, `update()`, `delete()` aufrufen (neben bestehendem `recalc_migrated()`)
- [x] 2.3 `recalc_dates()` in `MemberServiceImpl::create()` aufrufen (nach Eintritt-Aktion-Erstellung)
- [x] 2.4 Unit-Tests für `recalc_dates()`: join_date aus Eintritt, exit_date aus Austritt effective_date, exit_date aus Todesfall date, exit_date None ohne Exit-Aktion, join_date unverändert ohne Eintritt

## 3. Frontend: join_date und exit_date Anzeige

- [x] 3.1 `join_date`-Feld im Member-Detailformular bei bestehenden Mitgliedern (nicht "new") als read-only rendern
- [x] 3.2 `exit_date`-Feld im Member-Detailformular bei bestehenden Mitgliedern als read-only anzeigen (leer wenn None)
- [x] 3.3 `join_date`-Feld beim Neuanlegen editierbar belassen

## 4. E2E-Tests

- [x] 4.1 E2E-Test: Member anlegen → prüfen dass join_date aus Eintritt-Aktion abgeleitet wird
- [x] 4.2 E2E-Test: Austritt-Aktion mit effective_date anlegen → prüfen dass exit_date am Member gesetzt wird
- [x] 4.3 E2E-Test: Austritt-Aktion ohne effective_date → prüfen dass Validierungsfehler zurückkommt
- [x] 4.4 E2E-Test: Austritt-Aktion löschen → prüfen dass exit_date wieder None wird
