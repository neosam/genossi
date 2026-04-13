## Context

Genossi ist eine REST-API mit Axum, die standardmäßig alle Endpunkte hinter OIDC-Authentifizierung schützt. Für die WordPress-Integration wird ein einzelner öffentlicher Endpunkt benötigt, der die aktive Mitgliederanzahl liefert. Das Config-System speichert Key-Value-Paare in SQLite und unterstützt bool-Typen.

## Goals / Non-Goals

**Goals:**
- Öffentlicher Endpunkt für aktive Mitgliederanzahl
- Feature-Flag über bestehendes Config-System
- Caching um DB-Last zu minimieren
- Minimale Angriffsfläche (nur eine Zahl, kein Auth-Bypass)

**Non-Goals:**
- Allgemeines Public-API-Framework
- Weitere Statistiken oder aggregierte Daten
- Cache-Invalidierung bei Mitgliederänderungen (TTL reicht)
- Neue externe Dependencies

## Decisions

### 1. Routing: Route nach Auth-Layern registrieren

Die Public-Route wird in `create_app()` **nach** den Auth-Middleware-Layern (`.layer(forbid_unauthenticated)`, `.layer(context_extractor)`) und nach den OIDC-Layern registriert. In Axum gilt `.layer()` nur für bereits registrierte Routen — spätere Routen durchlaufen diese Layer nicht.

```rust
// In create_app(), nach OIDC-Setup und vor app-Return:
let app = app
    .nest("/api/public", public_stats::generate_route())
    .with_state(rest_state.clone());
```

**Alternative**: Separater Router mit `.merge()` — unnötig komplex für einen einzelnen Endpunkt.

### 2. Cache: `tokio::sync::RwLock` mit TTL

Ein einfacher In-Memory-Cache mit `RwLock<Option<(T, Instant)>>` und 5 Minuten TTL. Zwei Cache-Einträge: Config-Wert und Member-Count.

```rust
struct PublicStatsCache {
    config_enabled: RwLock<Option<(bool, Instant)>>,
    member_count: RwLock<Option<(u64, Instant)>>,
}
```

**Alternative**: `moka` oder `dashmap` — Overkill für zwei Werte. Keine neue Dependency nötig.

### 3. Count-Query: Dedizierte DAO-Methode

Neue `count_active()` Methode im `MemberDao`-Trait mit SQL `SELECT COUNT(*)` statt alle Mitglieder zu laden und in Rust zu zählen.

```sql
SELECT COUNT(*) FROM member
WHERE deleted IS NULL
AND (exit_date IS NULL OR exit_date > ?)
```

Der Zeitparameter ist das aktuelle Datum/Uhrzeit zum Zeitpunkt der Query.

**Alternative**: `all().len()` — funktioniert, lädt aber unnötig alle Datensätze.

### 4. Cache-Struct als Teil des RestState

Das `PublicStatsCache` wird im `RestState` gehalten und über ein neues Trait `PublicStatsState` zugänglich gemacht. So folgt es dem bestehenden Pattern für State-Zugriff.

### 5. Request-Flow

```
Request: GET /api/public/member-count
    │
    ├─ Config gecached und gültig?
    │   ├─ Ja, enabled=false → 403
    │   ├─ Ja, enabled=true  → weiter
    │   └─ Nein → Config aus DB laden, cachen
    │       ├─ Nicht gefunden oder false → 403
    │       └─ true → weiter
    │
    ├─ Count gecached und gültig?
    │   ├─ Ja → count zurückgeben
    │   └─ Nein → count_active() aus DB, cachen
    │
    └─ Response: 200 { "count": 142 }
```

## Risks / Trade-offs

- **[Stale Data]** Count kann bis zu 5 Minuten veraltet sein → Akzeptabel für WordPress-Anzeige
- **[Config-Cache]** Config-Änderung braucht bis zu 5 Minuten um zu wirken → Akzeptabel, Admin kann warten
- **[Erster öffentlicher Endpunkt]** Neue Angriffsfläche → Minimiert durch: nur eine Zahl, Feature-Flag, kein Auth-Kontext
- **[Kein Rate-Limiting]** Endpunkt kann häufig aufgerufen werden → Cache federt DB-Last ab; Rate-Limiting kann später bei Bedarf ergänzt werden
