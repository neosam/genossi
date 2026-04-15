## Context

Genossi verwaltet Mitgliederdaten einer Genossenschaft. Änderungen an Member, MemberAction, MemberDocument und Application müssen revisionssicher dokumentiert werden. Aktuell gibt es keine Änderungshistorie. Der `process`-Parameter in DAO-Methoden (`create`, `update`) wird bereits durchgereicht, aber ignoriert (`_process`).

Die Architektur folgt einem klaren Layer-Modell: REST → Service → DAO. Der Service-Layer hat Zugriff auf `Authentication<Context>` (user_id) und den `process`-String. Alle Schreiboperationen laufen durch den Service-Layer.

## Goals / Non-Goals

**Goals:**
- Jede Feldänderung an Member, MemberAction, MemberDocument und Application wird geloggt
- Nachvollziehbar: wer (user_id), wann (timestamp), was (Feld + alter/neuer Wert), welcher Service (process)
- Hash-Chain (SHA256) zur Erkennung von Manipulationen
- `transaction_id` zum Gruppieren zusammengehöriger Änderungen
- Audit-Macros, die DAO-Call und Logging atomar zusammenfassen
- REST-API und Frontend-UI zum Lesen und Filtern
- Verifizierungs-Endpoint für Hash-Chain-Integrität

**Non-Goals:**
- Event Sourcing (kein Rebuild des Zustands aus Events)
- SQLite-Trigger (Logging passiert im Service-Layer, nicht auf DB-Ebene)
- Externe Signierung oder Backup der Hash-Chain (Stufe 3)
- Audit-Log für Permission/User-Management-Operationen (kann später ergänzt werden)
- Retention-Policy / automatisches Löschen alter Einträge

## Decisions

### 1. Audit-Log im Service-Layer, nicht im DAO

**Entscheidung**: Diff-Berechnung und Logging im Service-Layer.

**Rationale**: Der Service-Layer kennt sowohl die `user_id` (aus `Authentication<Context>`) als auch den `process`-String. Im DAO-Layer müsste man den gesamten Auth-Context durchreichen, was die DAO-Schnittstelle verkompliziert.

**Alternative**: DAO-Layer-Logging — hätte den Vorteil, dass es nicht vergessen werden kann, erfordert aber Context-Durchreichung und vermischt Verantwortlichkeiten.

**Mitigation**: Audit-Macros (`audited_create!`, `audited_update!`, `audited_delete!`) kapseln den DAO-Call + Logging und verhindern, dass das Logging vergessen wird.

### 2. Eine Zeile pro Feld, nicht pro Entity

**Entscheidung**: Jede Feldänderung wird als separate Zeile in `audit_log` gespeichert.

**Rationale**: Ermöglicht einfaches SQL-Filtering ("zeig alle Namensänderungen"), klare UI-Darstellung und granulare Nachvollziehbarkeit.

**Alternative**: Eine Zeile pro Entity-Änderung mit JSON-Diff — weniger Zeilen, aber schwerer filterbar und erfordert JSON-Parsing.

**Gruppierung**: Zusammengehörige Feldänderungen werden über `transaction_id` (UUID) gruppiert.

### 3. Hash-Chain mit SHA256

**Entscheidung**: Jeder Audit-Log-Eintrag enthält `prev_hash` (Hash des vorherigen Eintrags) und `entry_hash` (SHA256 über alle Felder + prev_hash).

**Hash-Input** (deterministisch, Felder in fester Reihenfolge):
```
SHA256(timestamp | user_id | process | transaction_id | entity_type | entity_id | action | field_name | old_value | new_value | prev_hash)
```

**Ketten-Ordnung**: Strikt sequentiell. Bei mehreren Feldern in einer Transaction werden die Einträge alphabetisch nach `field_name` sortiert und sequentiell verkettet.

**Erster Eintrag**: `prev_hash` ist ein leerer String, `entry_hash` wird über alle Felder + leerer prev_hash berechnet.

**Alternative**: Baum-Struktur pro Transaction (Transaction-Hash fasst Feld-Hashes zusammen) — komplexer, marginaler Vorteil.

### 4. Auditable Trait für Feld-Extraktion

**Entscheidung**: Ein `Auditable`-Trait auf DAO-Entities mit Methoden für Feld-Extraktion und automatische Diff-Berechnung.

```rust
pub trait Auditable {
    fn entity_type() -> &'static str;
    fn entity_id(&self) -> Uuid;
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)>;
    
    // Default-Implementierung: vergleicht audit_fields paarweise
    fn diff(&self, other: &Self) -> Vec<AuditFieldChange> { ... }
}
```

**Geloggte Felder**: Nur echte Nutzdaten. Ausgeschlossen sind: `id`, `version`, `created`, `deleted` (interne Verwaltungsfelder).

**Alle Werte als String**: `i32` → `"42"`, `Option<Arc<str>>` → `Some("wert")` oder `None` → `None`, `time::Date` → `"2026-04-15"`, Enums → `.as_str()`.

### 5. Audit-Macros statt manuellem Logging

**Entscheidung**: Drei Macros in `genossi_service_impl` kapseln das Pattern:

- `audited_create!(self, dao, entity, process, user_id, tx)` — loggt alle Felder als neu (old_value = None)
- `audited_update!(self, dao, entity_id, new_entity, process, user_id, tx)` — lädt alte Entity, führt Update durch, berechnet Diff, loggt nur geänderte Felder
- `audited_delete!(self, dao, entity_id, process, user_id, tx)` — lädt Entity, setzt deleted-Timestamp, loggt als Delete

Die Macros erwarten, dass `self` ein `audit_log_dao`-Feld hat (wird über `gen_service_impl!` als neue Dependency injiziert).

### 6. AuditLogDao — schlankes Interface

```rust
pub trait AuditLogDao {
    type Transaction: Transaction;
    
    async fn create_entries(
        &self, entries: &[AuditLogEntry], tx: Self::Transaction
    ) -> Result<(), DaoError>;
    
    async fn get_latest_hash(
        &self, tx: Self::Transaction
    ) -> Result<Option<String>, DaoError>;
    
    async fn get_by_entity(
        &self, entity_type: &str, entity_id: Uuid, tx: Self::Transaction
    ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
    
    async fn get_all_ordered(
        &self, tx: Self::Transaction
    ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
}
```

### 7. SQLite-Tabelle

```sql
CREATE TABLE audit_log (
    id BLOB NOT NULL PRIMARY KEY,
    timestamp TEXT NOT NULL,
    user_id TEXT NOT NULL,
    process TEXT NOT NULL,
    transaction_id BLOB NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    action TEXT NOT NULL,        -- 'create', 'update', 'delete'
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    prev_hash TEXT NOT NULL,
    entry_hash TEXT NOT NULL
);

CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_transaction ON audit_log(transaction_id);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_user ON audit_log(user_id);
```

### 8. user_id aus Authentication<Context>

**Entscheidung**: Die `user_id` für den Audit-Log wird über `PermissionService::current_user_id()` geholt, das bereits im Service-Layer verfügbar ist.

Bei `Authentication::Full` (interner Aufruf ohne User-Context) wird `"SYSTEM"` als user_id verwendet.

## Risks / Trade-offs

**[Wachstum der audit_log-Tabelle]** → Kein Retention-Limit vorgesehen. Bei hoher Änderungsfrequenz kann die Tabelle groß werden. Mitigation: SQLite-Indizes; Retention kann später ergänzt werden.

**[Performance bei Hash-Chain-Verifikation]** → `verify_chain` muss alle Einträge sequentiell lesen und hashen. Mitigation: Verifikation ist ein Admin-only Vorgang, kein Hot-Path. Kann bei Bedarf paginiert werden.

**[Macro-Komplexität]** → Macros können schwerer zu debuggen sein als reguläre Funktionen. Mitigation: Macros sind einfach gehalten (5-10 Zeilen Expansion), klar dokumentiert und getestet.

**[Admin kann DB + Log gleichzeitig manipulieren]** → Hash-Chain erkennt Manipulationen in der Mitte, aber ein Admin könnte theoretisch die gesamte Kette neu berechnen. Mitigation: Regelmäßige Hash-Exports oder externe Backups (Out of Scope, kann als Follow-Up ergänzt werden).

**[Vergessenes Logging bei neuen Entities]** → Wenn eine neue Entity hinzukommt, muss der Auditable-Trait implementiert und die Macros verwendet werden. Mitigation: Dokumentation im CLAUDE.md; Tests können prüfen, ob die Trait-Implementierung existiert.
