## Why

Der Anzeigename (sender_name) eines Users wird weder auf der Berechtigungsseite noch auf der Konfigurationsseite angezeigt, obwohl die API die korrekten Daten liefert. Der Wert wird geladen aber nie im Input-Feld dargestellt.

## What Changes

- Fix: `UserRowComponent` in `permissions.rs` initialisiert `name_input` via `use_signal`, aber der Wert wird bei Re-Renders nicht aktualisiert — das Input-Feld bleibt leer
- Fix: `config_page.rs` setzt `sender_name` async, aber das Input-Feld reflektiert den geladenen Wert nicht korrekt
- Sicherstellen, dass beide Seiten den geladenen `sender_name` korrekt im Input-Feld anzeigen

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

(keine — reiner Bug-Fix im Frontend-Rendering, keine Anforderungsänderung)

## Impact

- `genossi-frontend/src/page/permissions.rs` — `UserRowComponent` Signal-Initialisierung
- `genossi-frontend/src/page/config_page.rs` — sender_name Input-Binding
- Keine API- oder Backend-Änderungen nötig
