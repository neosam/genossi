## Why

`join_date` und `exit_date` auf dem Member existieren doppelt: als direkte Felder und implizit als Datum der Eintritt/Austritt/Todesfall-Aktionen. Die Aktionen sind die eigentliche Source of Truth, aber die Member-Felder werden nicht automatisch synchronisiert. Außerdem ist `join_date` im Frontend bei bestehenden Mitgliedern nicht editierbar und `exit_date` wird gar nicht angezeigt — Änderungen an Aktionen spiegeln sich nicht im Member wider.

## What Changes

- Neue `recalc_dates()`-Funktion im Backend, die `join_date` und `exit_date` aus Aktionen ableitet — aufgerufen nach jedem Action create/update/delete
- `effective_date` wird bei Austritt-Aktionen zum **Pflichtfeld** (bisher optional), da der Austritt laut Satzung immer erst am Ende des Geschäftsjahres wirksam wird
- `join_date` und `exit_date` werden im Frontend bei bestehenden Mitgliedern als **read-only** angezeigt
- `join_date` bleibt beim Neuanlegen eines Mitglieds editierbar (bestimmt das Datum der automatischen Eintritt-Aktion)

## Capabilities

### New Capabilities

### Modified Capabilities
- `member-management`: `join_date` und `exit_date` werden automatisch aus Aktionen abgeleitet statt manuell gesetzt; Frontend zeigt beide Felder read-only bei bestehenden Mitgliedern
- `member-actions`: `effective_date` wird bei Austritt zum Pflichtfeld; neue `recalc_dates()`-Logik nach Action create/update/delete

## Impact

- **Backend**: `genossi_service_impl/src/member_action.rs` (recalc_dates, Validierung), `genossi_service_impl/src/member.rs` (recalc_dates nach Create)
- **Frontend**: `genossi-frontend/src/page/member_details.rs` (join_date/exit_date Anzeige)
- **DAO**: Evtl. neue Methode zum gezielten Update von `join_date`/`exit_date`, analog zu `update_migrated`
- **API**: Keine Breaking Changes — Felder bleiben im Response, werden nur serverseitig überschrieben
