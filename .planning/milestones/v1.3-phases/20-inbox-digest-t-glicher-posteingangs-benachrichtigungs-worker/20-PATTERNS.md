# Phase 20: Inbox-Digest — täglicher Posteingangs-Benachrichtigungs-Worker - Pattern Map

**Mapped:** 2026-06-26
**Files analyzed:** 7 (1 migration, 1 DAO trait, 1 DAO SQLite-Impl, 1 Worker, 1 Worker-Wiring, 1 main.rs-Spawn, 1 Frontend-Config-Abschnitt)
**Analogs found:** 7 / 7 (alle haben einen direkten, im CONTEXT.md benannten Analog)

> Hinweis: Es gibt KEIN RESEARCH.md (bewusst übersprungen). Alle Patterns stammen aus
> realer Codebase. Excerpts sind copy-ready: echte Struct-Felder, echte Signaturen,
> echte SQL/SQLx-Bind-Reihenfolge, echte Loop-Struktur.

---

## File Classification

| Neue/geänderte Datei | Rolle | Data Flow | Closest Analog | Match |
|----------------------|-------|-----------|----------------|-------|
| `migrations/sqlite/<ts>_create_digest_state_table.sql` | migration | state-persist | `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` + `..._create_audit_timestamp_table.sql` | exact (single-table) |
| `genossi_mail/src/dao.rs` (neuer `DigestStateDao`-Trait + Struct) | model/dao | CRUD (get/set state) | `genossi_mail/src/dao.rs` → `InboundMailAttachmentDao`, `ConfigDao` (`genossi_config/src/dao.rs`) | exact |
| `genossi_mail/src/dao_sqlite.rs` (neuer `DigestStateDaoSqlite`) | dao-impl | CRUD/upsert | `ConfigDaoSqlite` (`genossi_config/src/dao_sqlite.rs`, upsert) + `InboundMailDaoSqlite` (`genossi_mail/src/dao_sqlite.rs`, UUID-BLOB/datetime) | exact |
| `genossi_mail/src/digest.rs` (neuer Digest-Worker) | service/worker | event-driven (poll-loop) | `genossi_service_impl/src/timestamp_worker.rs` (config-read im Loop) + `genossi_mail/src/inbox.rs::start_inbox_worker`/`poll_once` | exact |
| `genossi_bin/src/lib.rs` (neue `start_digest_worker`-Methode) | config/DI-wiring | request-response (spawn) | `genossi_bin/src/lib.rs::start_inbox_worker` (Zeile 1377) + `start_timestamp_worker` (Zeile 1443) | exact |
| `genossi_bin/src/main.rs` (Spawn-Aufruf) | entry-point | request-response | `genossi_bin/src/main.rs:54` (`rest_state.start_inbox_worker()`) | exact |
| `genossi-frontend/src/page/config_page.rs` (neuer Abschnitt "Posteingangs-Benachrichtigung") | component/page | request-response (form save) | IMAP-Block in `config_page.rs` (Zeilen 47–58, 123–130, 646–778) | exact |

---

## Pattern Assignments

### `migrations/sqlite/<ts>_create_digest_state_table.sql` (migration, state-persist)

**Analog:** `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` (Form/Stil)
+ `migrations/sqlite/20260416000000_create_audit_timestamp_table.sql` (State-Tabellen-Semantik).

**Bestehende Konventionen (verbindlich):**
- Dateiname: `<UTC-timestamp>_create_<name>_table.sql` (siehe `ls migrations/sqlite/` —
  z.B. `20260608000000_...`, `20260625000000_...`). Timestamp > letzte Migration
  (`20260625000000`).
- UUID-PKs sind `BLOB PRIMARY KEY NOT NULL`. Datetimes sind `TEXT NOT NULL` (ISO8601).
- `CREATE TABLE IF NOT EXISTS` (Pattern aus inbound_mail_attachments).

**Excerpt — Struktur-Vorbild (inbound_mail_attachments, Zeilen 1-12):**
```sql
CREATE TABLE IF NOT EXISTS inbound_mail_attachments (
    id BLOB PRIMARY KEY NOT NULL,
    inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
    created TEXT NOT NULL,
    ...
);
CREATE INDEX idx_inbound_mail_attachments_mail ON inbound_mail_attachments(inbound_mail_id);
```

**Excerpt — State-Tabellen-Semantik-Vorbild (audit_timestamp, Zeilen 1-12):**
```sql
CREATE TABLE audit_timestamp (
    id BLOB NOT NULL PRIMARY KEY,
    timestamp TEXT NOT NULL,
    audit_hash TEXT NOT NULL,
    ...
    status TEXT NOT NULL
);
```

**Empfehlung für Digest-State (D-03 — Singleton-State, hält letztes Versanddatum):**
Da nur EIN letztes Versanddatum existiert, ist eine Singleton-Row sinnvoll. Zwei
copy-ready Varianten (Planning entscheidet):
- **Variante A (KV-artig, upsert-freundlich, spiegelt `ConfigDaoSqlite`):**
  ```sql
  CREATE TABLE IF NOT EXISTS digest_state (
      key TEXT PRIMARY KEY,        -- z.B. 'last_sent_date'
      value TEXT NOT NULL          -- ISO-Datum 'YYYY-MM-DD' (kein KV-Config-Store, eigene Tabelle!)
  );
  ```
- **Variante B (Singleton-Row mit fester PK):**
  ```sql
  CREATE TABLE IF NOT EXISTS digest_state (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      last_sent_date TEXT          -- NULL = noch nie gesendet
  );
  ```
> Tabellen-/Spaltennamen liegen laut CONTEXT.md ("Claude's Discretion") im Ermessen.

---

### `genossi_mail/src/dao.rs` — neuer `DigestStateDao`-Trait (model/dao, CRUD)

**Analog:** `InboundMailAttachmentDao` (`genossi_mail/src/dao.rs:139-153`) für Trait-Form;
`ConfigDao` (`genossi_config/src/dao.rs:18-25`) für die get/set-Semantik.

**Imports/Konventionen (dao.rs Kopf, Zeilen 1-4):**
```rust
use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;
```

**Fehlertyp (wiederverwenden, dao.rs:21-25):**
```rust
#[derive(Debug, Clone)]
pub enum MailDaoError {
    DatabaseError(Arc<str>),
    NotFound,
}
```

**Trait-Form-Vorbild (`InboundMailAttachmentDao`, dao.rs:139-153) — `#[automock]` ist Pflicht
(jeder Trait wird mit mockall mockbar gemacht):**
```rust
#[automock]
#[async_trait]
pub trait InboundMailAttachmentDao: Send + Sync + 'static {
    async fn create(&self, attachment: &InboundMailAttachment) -> Result<(), MailDaoError>;
    async fn count_for_mail(&self, mail_id: Uuid) -> Result<i64, MailDaoError>;
}
```

**Get/Set-Semantik-Vorbild (`ConfigDao`, genossi_config/src/dao.rs:18-25):**
```rust
#[automock]
#[async_trait]
pub trait ConfigDao: Send + Sync + 'static {
    async fn get(&self, key: &str) -> Result<Option<ConfigEntry>, ConfigDaoError>;
    async fn set(&self, entry: &ConfigEntry) -> Result<(), ConfigDaoError>;
}
```

**Empfohlene neue Trait-Form (copy-ready):**
```rust
#[automock]
#[async_trait]
pub trait DigestStateDao: Send + Sync + 'static {
    /// Letztes Versanddatum (None = noch nie gesendet).
    async fn get_last_sent_date(&self) -> Result<Option<time::Date>, MailDaoError>;
    /// Setzt (upsert) das letzte Versanddatum.
    async fn set_last_sent_date(&self, date: time::Date) -> Result<(), MailDaoError>;
}
```
> Alternativ `String`/`time::PrimitiveDateTime` statt `time::Date`, je nach Migration-Variante.

---

### `genossi_mail/src/dao_sqlite.rs` — neuer `DigestStateDaoSqlite` (dao-impl, CRUD/upsert)

**Analog:** `ConfigDaoSqlite` (`genossi_config/src/dao_sqlite.rs`) für upsert-Pattern;
`InboundMailDaoSqlite` (`genossi_mail/src/dao_sqlite.rs:849-948`) für Struct-Form,
Pool-Handling, datetime/UUID-Konvertierung und In-Memory-Test-Setup.

**Struct + `new` (dao_sqlite.rs:849-857, identisch für jeden DAO):**
```rust
pub struct InboundMailDaoSqlite {
    pool: Arc<SqlitePool>,
}
impl InboundMailDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
```

**Upsert-Pattern (ConfigDaoSqlite::set, genossi_config/src/dao_sqlite.rs:63-76) — exakt für
`set_last_sent_date` übernehmen:**
```rust
sqlx::query(
    "INSERT INTO config_entries (key, value, value_type) VALUES (?, ?, ?)
     ON CONFLICT(key) DO UPDATE SET value = excluded.value, value_type = excluded.value_type",
)
.bind(entry.key.as_ref())
.bind(entry.value.as_ref())
.bind(entry.value_type.as_ref())
.execute(self.pool.as_ref())
.await
.map_err(|e| ConfigDaoError::DatabaseError(Arc::from(e.to_string())))?;
Ok(())
```

**get-Pattern (ConfigDaoSqlite::get, genossi_config/src/dao_sqlite.rs:51-61) für
`get_last_sent_date`:**
```rust
let row = sqlx::query_as::<_, ConfigEntryDb>(
    "SELECT key, value, value_type FROM config_entries WHERE key = ?",
)
.bind(key)
.fetch_optional(self.pool.as_ref())
.await
.map_err(|e| ConfigDaoError::DatabaseError(Arc::from(e.to_string())))?;
Ok(row.as_ref().map(ConfigEntry::from))
```

**Datetime-Helper (wenn `TEXT`-Datum/Datetime gespeichert wird) — aus
genossi_mail/src/dao_sqlite.rs:14-37 wiederverwenden (`parse_datetime` /
`format_datetime`). Für reine `time::Date` ggf. `Date::parse`/`format` mit
`[year]-[month]-[day]`-Maske analog `sqlite_simple` (Zeile 27).**

**In-Memory-Test-Setup-Pattern (ConfigDaoSqlite-Tests, genossi_config/src/dao_sqlite.rs:93-128):**
```rust
async fn setup_db() -> Arc<SqlitePool> {
    let pool = SqlitePool::connect("sqlite::memory:").await.expect("...");
    sqlx::query("CREATE TABLE digest_state ( ... )").execute(&pool).await.expect("...");
    Arc::new(pool)
}
#[tokio::test]
async fn test_set_and_get() {
    let pool = setup_db().await;
    let dao = DigestStateDaoSqlite::new(pool);
    dao.set_last_sent_date(...).await.unwrap();
    assert_eq!(dao.get_last_sent_date().await.unwrap(), Some(...));
}
```
> Module-Export: `dao_sqlite.rs` re-exportiert seine Daos via `pub struct ...Sqlite`;
> prüfe `genossi_mail/src/lib.rs` ob `pub mod dao_sqlite;` schon vorhanden ist (ja, ist es).

---

### `genossi_mail/src/digest.rs` — neuer Digest-Worker (service/worker, event-driven poll-loop)

**Analog (Primär):** `genossi_service_impl/src/timestamp_worker.rs` — config-getriebene
`loop {}`-Struktur (get_all() lesen → Werte ableiten → Aktion → sleep). CONTEXT.md D-04
verlangt diese Struktur explizit (periodisches Polling, KEIN sleep-bis-Uhrzeit).
**Analog (Sekundär):** `genossi_mail/src/inbox.rs::start_inbox_worker` (764-792) +
`poll_once` (798-888) für ConfigMissing-Skip, best-effort-Fehlerbehandlung und `Arc`-Deps.

**Loop-Skelett (timestamp_worker.rs:24-69) — exakt nachbauen:**
```rust
pub async fn start_timestamp_worker<T, C>(timestamp_service: Arc<T>, config_service: Arc<C>)
where T: TimestampService, C: ConfigService,
{
    tracing::info!("Timestamp worker started");
    loop {
        let entries = match config_service.get_all().await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("...: failed to read config: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(DEFAULT_INTERVAL_HOURS * 3600)).await;
                continue;
            }
        };
        let interval_hours = get_interval_hours(&entries);
        if is_tsa_enabled(&entries) {
            match timestamp_service.create_timestamp().await { /* ... */ }
        } else {
            tracing::debug!("...: not enabled, skipping");
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval_hours * 3600)).await;
    }
}
```

**Config-Key-Reader-Pattern (timestamp_worker.rs:8-22) — für `digest_recipients` /
`digest_send_time` übernehmen (free-Funktion + `.iter().find(|e| e.key.as_ref() == "...")`):**
```rust
fn get_interval_hours(entries: &[ConfigEntry]) -> u64 {
    entries.iter()
        .find(|e| e.key.as_ref() == "tsa_interval_hours")
        .and_then(|e| e.value.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_HOURS)
}
```
Diese reinen Funktionen sind die **Test-Oberfläche** (timestamp_worker.rs:72-119 testet sie
ohne Worker-Loop). Für Phase 20: `parse_recipients(entries) -> Vec<String>` (komma-split, D-05),
`parse_send_time(entries) -> Option<(u8,u8)>` (HH:MM), `is_due(now, send_time, last_sent_date)`
sollten reine, unit-getestete Funktionen sein.

**Offene Mails laden + Filter (inbox.rs:582-584 + dao.rs `archived`-Feld):**
```rust
// InboxService::list() liefert Arc<[InboundMail]> (list_active, sortiert received_at DESC)
let mails = inbox_service.list().await?;       // bereits DESC sortiert (dao.rs:298)
let offen: Vec<&InboundMail> = mails.iter().filter(|m| !m.archived).collect();
```
> WICHTIG: `InboundMail.received_at` (dao.rs:279), `.subject` (278), `.from_address` (277) sind
> die Felder für Titel/Absender/Eingangszeit. `list_active` ist schon `ORDER BY received_at DESC`
> → D-10 ("neueste zuerst") ist bereits erfüllt, keine erneute Sortierung nötig.

**Versand pro Empfänger (D-06/D-07) — `send_test_mail_with_body` (service.rs:447-488)
wiederverwenden, in Schleife, Fehler loggen+weiter:**
```rust
for recipient in &recipients {
    match mail_service.send_test_mail_with_body(recipient, &subject, &body).await {
        Ok(()) => tracing::info!("digest: sent to {}", recipient),
        Err(e) => tracing::error!("digest: send to {} failed: {:?}", recipient, e), // weiter (D-07)
    }
}
// Versanddatum trotzdem setzen (D-07): digest_state_dao.set_last_sent_date(today).await
```
> `send_test_mail_with_body(to, subject, body)` baut SMTP-Transport selbst via
> `load_smtp_config` + `build_transport` (service.rs:127-211) → keine SMTP-Logik im
> Worker duplizieren. Plain-Text passt zu D-08.

**Server-Lokalzeit (D-02):** `time::OffsetDateTime::now_local()` (NICHT `now_utc`) für die
Uhrzeit-Prüfung; CONTEXT D-02 sagt explizit Server-Lokalzeit, kein chrono-tz.
> Achtung: andere Stellen im mail-Code nutzen `now_utc()` (z.B. service.rs:310) für
> Persistenz-Timestamps — das ist OK; nur der Uhrzeit-Vergleich braucht Lokalzeit.

**Deep-Link (D-11) — exakt aus helper_token.rs:38-41:**
```rust
let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string());
let deep_link = format!("{}/inbox", app_url.trim_end_matches('/'));
```

**Worker-Generics-Bounds-Vorbild (inbox.rs:764-776, `Arc`-Deps + `where`-Klausel):**
```rust
pub async fn start_inbox_worker<C, D, I, A, St>(
    config_service: Arc<C>, dao: Arc<D>, /* ... */
) where C: ConfigService, D: InboundMailDao, /* ... */
```
> Für Digest: `start_digest_worker<C, I, M, S>(config_service: Arc<C>, inbox_service: Arc<I>,
> mail_service: Arc<M>, digest_state_dao: Arc<S>)` mit Bounds
> `C: ConfigService, I: InboxService, M: MailService, S: DigestStateDao`.

**Test-Pattern für reine Funktionen (timestamp_worker.rs:72-119) — `make_entry`-Helper +
`#[test]` für jede Config-Ableitung; für `is_due` mehrere `#[test]`-Fälle (vor/nach Uhrzeit,
schon-heute-gesendet, nie-gesendet → D-01 Nachhol-Garantie).**

---

### `genossi_bin/src/lib.rs` — neue `start_digest_worker`-Methode (config/DI-wiring)

**Analog:** `start_inbox_worker` (lib.rs:1377-1393) und `start_timestamp_worker`
(lib.rs:1443-1454). Letztere zeigt, wie ein Worker einen ConfigService **ad-hoc** baut
(ohne vorab gespeichertes Worker-Feld) — relevant, falls keine neuen Worker-Felder
angelegt werden sollen.

**Spawn-Pattern mit vorhandenen Worker-Feldern (start_inbox_worker, lib.rs:1377-1393):**
```rust
pub fn start_inbox_worker(&self) {
    let config_service = self.worker_inbox_config_service.clone();
    let dao = self.worker_inbox_dao.clone();
    let imap_client = self.worker_inbox_imap_client.clone();
    let attachment_dao = self.worker_inbox_attachment_dao.clone();
    let storage = self.worker_inbox_storage.clone();
    tokio::spawn(async move {
        genossi_mail::inbox::start_inbox_worker(config_service, dao, imap_client, attachment_dao, storage).await;
    });
}
```

**Spawn-Pattern mit ad-hoc gebautem ConfigService (start_timestamp_worker, lib.rs:1443-1454):**
```rust
pub fn start_timestamp_worker(&self) {
    let timestamp_service = self.timestamp_service.clone();
    let config_dao = ConfigDao::new(self.pool.clone());
    let config_service = Arc::new(ConfigService::new(config_dao));
    tokio::spawn(async move {
        genossi_service_impl::timestamp_worker::start_timestamp_worker(timestamp_service, config_service).await;
    });
}
```

**DI-Hinweise (verbindlich):**
- `InboxServiceImpl` wird in `new()` bereits gebaut (lib.rs:1107) → existierender
  `inbox_service` Arc kann an den Digest-Worker weitergereicht werden (oder ein
  Worker-Klon-Feld analog `worker_inbox_*` anlegen).
- Neuer `DigestStateDaoSqlite` wird via `DigestStateDaoSqlite::new(self.pool.clone())`
  gebaut — gleiches Muster wie `InboundMailDaoType::new(self.pool.clone())` (lib.rs:1464).
- MailService-Arc existiert ebenfalls bereits in `new()` (für Mail-Worker).
- Typ-Aliase oben in lib.rs (z.B. `type InboundMailDaoType = ...`, lib.rs:580) — ggf.
  analogen Alias `type DigestStateDaoType = genossi_mail::dao_sqlite::DigestStateDaoSqlite;`
  ergänzen.

---

### `genossi_bin/src/main.rs` — Spawn-Aufruf (entry-point)

**Analog:** `genossi_bin/src/main.rs:51-67` — jeder Worker wird mit
`rest_state.start_*_worker();` + `tracing::info!("... started");` gestartet.

**Excerpt (main.rs:54-55):**
```rust
rest_state.start_inbox_worker();
tracing::info!("Inbox worker started");
```
> Neue Zeile analog: `rest_state.start_digest_worker();` + `tracing::info!("Digest worker started");`
> — sinnvoll direkt nach `start_inbox_worker()` (thematisch zusammengehörig).

---

### `genossi-frontend/src/page/config_page.rs` — Abschnitt "Posteingangs-Benachrichtigung" (component/page, form save)

**Analog:** IMAP-Block in derselben Datei. CONTEXT.md D-12 verlangt eigenen Abschnitt
im Stil der SMTP/IMAP-Blöcke, NICHT in IMAP integriert.

**Helper (config_page.rs:12-22) — wiederverwenden, NICHT neu schreiben:**
```rust
fn get_config_value(entries: &[ConfigEntryTO], key: &str) -> String {
    entries.iter().find(|e| e.key == key).map(|e| e.value.clone()).unwrap_or_default()
}
fn has_config_key(entries: &[ConfigEntryTO], key: &str) -> bool {
    entries.iter().any(|e| e.key == key)
}
```

**Signal-State-Pattern (IMAP-Block, config_page.rs:48-58) — analoge Signals anlegen:**
```rust
let mut imap_host = use_signal(|| String::new());
let mut imap_poll_interval = use_signal(|| "300".to_string());
let mut imap_saving = use_signal(|| false);
```
> Für Digest: `digest_recipients = use_signal(|| String::new())` (ein Textfeld, D-05),
> `digest_send_time = use_signal(|| "08:00".to_string())`, `digest_saving = use_signal(|| false)`.

**Populate-aus-Entries-Pattern (config_page.rs:108-130, im `spawn`-Load-Block):**
```rust
imap_host.set(get_config_value(&data, "imap_host"));
imap_poll_interval.set(... get_config_value(&data, "imap_poll_interval_seconds") ...);
```
> Für Digest: `digest_recipients.set(get_config_value(&data, "digest_recipients"));`
> `digest_send_time.set(get_config_value(&data, "digest_send_time"));`

**Save-Button + Speicher-Flow (IMAP-Block, config_page.rs:721-777) — exakt nachbauen
(read Signals → `spawn` → `set_config_entry` in Schleife → success_msg/reload):**
```rust
button {
    class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
    disabled: *imap_saving.read() || imap_host.read().is_empty(),
    onclick: move |_| {
        let host = imap_host.read().clone();
        // ... weitere Felder lesen ...
        spawn(async move {
            imap_saving.set(true);
            error.set(None);
            success_msg.set(None);
            let config = CONFIG.read().clone();
            let mut all_ok = true;
            let entries_to_save: Vec<(&str, String, &str)> = vec![
                ("imap_host", host, "string"),
                ("imap_poll_interval_seconds", poll, "int"),
            ];
            for (key, value, vtype) in &entries_to_save {
                if let Err(e) = api::set_config_entry(&config, key, value, vtype).await {
                    error.set(Some(e)); all_ok = false; break;
                }
            }
            if all_ok {
                success_msg.set(Some("IMAP-Einstellungen gespeichert".to_string()));
                reload();
            }
            imap_saving.set(false);
        });
    },
    if *imap_saving.read() { "Speichere…" } else { "Speichern" }
}
```
> Für Digest: zwei Einträge speichern — `("digest_recipients", recipients, "string")` und
> `("digest_send_time", send_time, "string")`. D-14: leeres Empfänger-Feld DEAKTIVIERT —
> trotzdem speichern (leerer String), Worker prüft selbst (parse_recipients → leer → skip).

**Validierung vor Speichern (D-13):** Im `onclick`-Block VOR dem `spawn`/vor dem Loop
prüfen: jede komma-getrennte Adresse grobes E-Mail-Format (`contains('@')` + nicht-leer
links/rechts reicht laut "grobes Format"), Uhrzeit `HH:MM` (split ':' → 2 Teile,
0–23 / 0–59). Bei Fehler `error.set(Some(...))` und früh zurück. Dieser inline-Check hat
keinen exakten Analog im IMAP-Block — IMAP validiert serverseitig via `value_type`.

**RSX-Wrapper (CollapsibleSection, config_page.rs:783) — Abschnitt in eine
`CollapsibleSection { title: "Posteingangs-Benachrichtigung", div { class: "space-y-4", ... } }`
hüllen (gleicher Container wie WebDAV/IMAP).**

> Component-First (CLAUDE.md): Falls Empfänger-Feld + Uhrzeit + Save als eigenständig
> wiederverwendbarer Block entsteht, prüfen, ob er als Component nach
> `genossi-frontend/src/component/` gehört. Für einen einmaligen Config-Abschnitt im
> Stil der bestehenden inline-Blöcke (SMTP/IMAP sind aktuell inline in config_page.rs)
> ist inline-im-config_page konsistent mit dem Bestand.

---

## Shared Patterns

### Config-Key-Lesen (Backend, im Worker)
**Source:** `genossi_service_impl/src/timestamp_worker.rs:8-22`, `genossi_mail/src/inbox.rs:46-100`
**Apply to:** `digest.rs`
Reine Funktion `fn parse_x(entries: &[ConfigEntry]) -> T` mit
`.iter().find(|e| e.key.as_ref() == "key").and_then(|e| e.value.parse().ok()).unwrap_or(default)`.
Macht den Worker ohne Loop unit-testbar.

### ConfigMissing / leerer-Wert = no-op (kein Fehler)
**Source:** `genossi_mail/src/inbox.rs:812-819` (poll_once skip), `timestamp_worker.rs:64-66`
**Apply to:** `digest.rs` — kein Empfänger / keine SMTP-Config ⇒ `tracing::debug!` + skip,
KEIN Error. Deckt D-14 (Deaktivierung über leeres Feld) und DIGEST-07 direkt ab.

### Fehlertyp-Wiederverwendung
**Source:** `genossi_mail/src/dao.rs:21-25` (`MailDaoError`), `service.rs:13-21` (`MailServiceError`)
**Apply to:** Neuer `DigestStateDao` verwendet `MailDaoError` (kein neuer Fehlertyp nötig,
da im `genossi_mail`-Crate). Worker propagiert `MailServiceError`.

### `#[automock]` auf jedem Trait
**Source:** Alle Traits in `dao.rs` / `service.rs` / `inbox.rs`
**Apply to:** `DigestStateDao` MUSS `#[automock] #[async_trait]` tragen → `MockDigestStateDao`
für Worker-Unit-Tests (User-CLAUDE.md: "Always make sure you have tests").

### SQLite Pool + datetime/UUID-Konvertierung
**Source:** `genossi_mail/src/dao_sqlite.rs:14-57` (parse/format-Helper),
`genossi_config/src/dao_sqlite.rs:63-76` (upsert)
**Apply to:** `DigestStateDaoSqlite` — `pool: Arc<SqlitePool>`, upsert via
`INSERT ... ON CONFLICT(...) DO UPDATE`, Fehler via `.map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))`.

### Worker-Spawn (DI)
**Source:** `genossi_bin/src/lib.rs:1377-1454`, `main.rs:51-67`
**Apply to:** Neue `start_digest_worker(&self)` in lib.rs + Aufruf in main.rs mit `tracing::info!`.

### APP_URL Deep-Link
**Source:** `genossi_rest/src/helper_token.rs:38-41`
**Apply to:** `digest.rs` — `std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string())`
+ `.trim_end_matches('/')` + `format!("{}/inbox", ...)`. KEIN eigener Config-Key (D-11).

---

## No Analog Found

| Aspekt | Begründung |
|--------|-----------|
| Inline E-Mail-/Uhrzeit-Validierung im Frontend (D-13) | Kein bestehender Frontend-Block validiert clientseitig vor `set_config_entry`; IMAP/SMTP verlassen sich auf serverseitiges `value_type`. Planner muss leichten inline-Check neu schreiben (grobes `@`-Format je komma-getrennter Adresse; `HH:MM`-Parse). Pattern-Quelle für die Speicher-Mechanik bleibt der IMAP-Block. |

> Alle übrigen Dateien haben einen exakten, im CONTEXT.md benannten Analog.

---

## Metadata

**Analog-Suchscope:** `genossi_service_impl/`, `genossi_mail/`, `genossi_config/`,
`genossi_rest/`, `genossi_bin/`, `genossi-frontend/src/page/`, `migrations/sqlite/`
**Gelesene Schlüsseldateien:** `timestamp_worker.rs`, `mail/service.rs`, `mail/inbox.rs`,
`mail/dao.rs`, `mail/dao_sqlite.rs`, `config/dao.rs`, `config/service.rs`,
`config/dao_sqlite.rs`, `helper_token.rs`, `bin/lib.rs` (1370-1529), `bin/main.rs`,
`config_page.rs`, 2 Migrationen
**Pattern-Extraktionsdatum:** 2026-06-26
