## 1. DAO-Schicht

- [x] 1.1 `count_active()` Methode zum `MemberDao`-Trait hinzufügen mit Default-Implementierung
- [x] 1.2 SQLite-Implementierung von `count_active()` mit `SELECT COUNT(*)` Query (WHERE deleted IS NULL AND (exit_date IS NULL OR exit_date > ?))
- [x] 1.3 Unit-Tests für `count_active()` — aktive, gelöschte und ausgetretene Mitglieder

## 2. Cache

- [x] 2.1 `PublicStatsCache`-Struct mit `RwLock<Option<(T, Instant)>>` für Config und Count erstellen
- [x] 2.2 Methoden `get_config()`, `set_config()`, `get_count()`, `set_count()` mit TTL-Prüfung (5 Min)
- [x] 2.3 Unit-Tests für Cache-TTL-Verhalten (fresh, expired)

## 3. REST-Endpunkt

- [x] 3.1 `PublicStatsState`-Trait definieren (Zugriff auf Cache, ConfigService, MemberDao/Pool)
- [x] 3.2 Handler für `GET /api/public/member-count` implementieren: Config prüfen → Count abfragen → JSON-Response
- [x] 3.3 `generate_route()` Funktion für Public-Stats-Modul
- [x] 3.4 `PublicStatsState` im `RestStateDef`-Trait und `RestState`-Implementierung ergänzen

## 4. Integration

- [x] 4.1 Route in `create_app()` nach Auth-Layern registrieren
- [x] 4.2 `PublicStatsCache` im `RestState` (genossi_bin) initialisieren
- [x] 4.3 OpenAPI-Dokumentation für den Public-Endpunkt

## 5. Tests

- [x] 5.1 E2E-Test: Endpunkt gibt 403 wenn Config nicht gesetzt
- [x] 5.2 E2E-Test: Endpunkt gibt 403 wenn Config `false`
- [x] 5.3 E2E-Test: Endpunkt gibt korrekten Count wenn Config `true`
- [x] 5.4 E2E-Test: Ausgetretene und gelöschte Mitglieder werden nicht gezählt
- [x] 5.5 E2E-Test: Kein Auth-Header nötig
