## Why

Das Security-Review hat mehrere Schwachstellen im Session-Lifecycle aufgedeckt, die zusammen einen realen Session-Hijack-Vektor mit bis zu einem Jahr Gültigkeitsdauer öffnen. Der ernsteste Einzelbefund: `tracing::info!()`-Aufrufe in `genossi_rest/src/session.rs:72-85` schreiben die aktive Session-ID bei **jedem authentifizierten Request** in die Logs. Kombiniert mit einer 365-Tage-Session-Lebensdauer und der Tatsache, dass nach OIDC-Login die OIDC-Middleware nicht mehr konsultiert wird (WordPress-Account-Deaktivierung wirkt nicht), ist jeder Log-Zugriff eine Eintrittskarte für Session-Hijack über 365 Tage. Da die App öffentlich im Internet erreichbar ist, muss dieser Vektor geschlossen werden.

## What Changes

- **BREAKING** Reduktion der Session-Lebensdauer von 365 Tagen auf 14 Tage (absolut) mit zusätzlichem 24-Stunden-Inaktivitäts-Timeout. Bestehende Sessions werden beim Deployment ungültig — User müssen sich neu einloggen.
- Entfernen aller Session-ID-Ausgaben aus `tracing`-Aufrufen in `genossi_rest/src/session.rs` (keine `{:?}` auf Cookies, Session-Entities oder Session-IDs mehr). Stattdessen: Logging des User-IDs (pseudonym, unkritisch) auf `debug`-Level.
- Ersetzen von `.expect("Failed to create session for OIDC user")` in `register_session` durch sauberes Error-Handling (HTTP 500 mit Log statt Panic). Verhindert DoS bei DB-Wackeln im Auth-Path.
- Neuer Endpunkt `POST /api/session/revoke-all` — erlaubt dem authentifizierten User, alle seine eigenen aktiven Sessions auf dem Server zu beenden. Ermöglicht Reaktion auf "mein Laptop wurde gestohlen" auch ohne WordPress-Admin-Zugriff.
- Inactivity-Update-Mechanismus: Bei jedem authentifizierten Request wird `last_used_at` der Session aktualisiert. Sessions, deren `last_used_at` älter als 24h ist, werden von `verify_user_session` abgelehnt und gelöscht.

## Capabilities

### New Capabilities

- `session-auth`: Definiert Session-Erstellung, -Verifikation, -Lebensdauer (absolut + Inaktivität), -Revocation und -Logging-Politik. Enthält explizite Nicht-Leak-Anforderungen für Session-IDs in Logs und Error-Messages.

### Modified Capabilities

_(keine bestehenden Specs betroffen — `session-auth` ist die erste formalisierte Session-Spec)_

## Impact

**Code:**
- `genossi_rest/src/session.rs` — Logging-Aufrufe umgeschrieben, `.expect` ersetzt, Session-Lifetime-Konstante, `last_used_at`-Update
- `genossi_service/src/session.rs` + `genossi_service_impl/src/session.rs` — neue Methoden `touch_session` und `revoke_all_for_user`, Inactivity-Check in `verify_user_session`
- `genossi_dao/src/permission.rs` + `genossi_dao_impl_sqlite/src/permission.rs` — neues Feld `last_used_at` auf `SessionEntity`, neue Methoden `touch_session` und `delete_sessions_for_user`
- `genossi_rest/src/lib.rs` — Neuer Route-Mount für `/api/session/revoke-all`
- Neuer REST-Handler für Session-Revoke (vermutlich neue Datei `genossi_rest/src/session_management.rs`)

**Datenbank:**
- Neue Migration: `last_used_at INTEGER` Spalte auf `sessions` Tabelle hinzufügen, initialisiert mit `created`-Wert für Bestandszeilen

**Benutzer:**
- **BREAKING**: Bei Deployment werden alle bestehenden 365-Tage-Sessions ungültig (sobald die neue Migration `last_used_at` initialisiert und der Inactivity-Check greift; Sessions älter als 24h ohne Traffic werden abgelehnt). Angekündigter Re-Login über OIDC/WordPress ist nötig.
- Kein sichtbarer UI-Change außer dem neuen Revoke-All-Button (optional im Frontend, kann nachgezogen werden).

**Dependencies:**
- Keine neuen Crates nötig.
