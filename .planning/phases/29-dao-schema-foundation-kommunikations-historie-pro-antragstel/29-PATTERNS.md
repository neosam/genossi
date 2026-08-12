# Phase 29: DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller) — Pattern Map

**Mapped:** 2026-08-12
**Files analyzed:** 6 zu ändern + 1 neu (Migration) + Tests
**Analogs found:** 7 / 7 (alle exakt — reine Spiegelung des bestehenden `member_id`-Pfads)

> Keine phasen-spezifische CONTEXT.md — verbindliche Entscheidungen aus ROADMAP.md (Phase-29-Detail, D2) + REQUIREMENTS.md (APHIST-01/03). RESEARCH.md liefert die verifizierte Ist-Kartierung; dieses Dokument bestätigt alle Datei:Zeile-Anker gegen den echten Code und extrahiert die konkreten Copy-Vorlagen.

Diese Phase ist reine **additive Spiegelung**: `mail_recipients` bekommt eine zweite nullable Linkage-Spalte `application_id BLOB`, Geschwister zu `member_id`. Für jede Zieldatei ist das Analog das direkt daneben liegende `member_id`-Konstrukt in derselben Datei. Match-Qualität durchgehend **exakt**.

## File Classification

| Neu/Geändert | Rolle | Datenfluss | Nächstes Analog | Match |
|--------------|-------|-----------|-----------------|-------|
| `migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql` (NEU) | migration | schema-additiv | `migrations/sqlite/20260702000002_mail_recipients_add_rendered_html_body.sql` + `20260411000001_add_member_communication_indexes.sql` | exakt |
| `genossi_mail/src/dao.rs` — `MailRecipient` + `CommunicationDao`-Trait | model + DAO-trait | CRUD / read | `member_id`-Feld (:62) + `get_member_communications` (:281) daneben | exakt |
| `genossi_mail/src/dao_sqlite.rs` — `MailRecipientDb`, `TryFrom`, alle SQL-Listen, Timeline-Impl, Test-DDL | DAO-impl | CRUD / read | `member_id`-Bindungen + `get_member_communications`-Impl (:1042) | exakt |
| `genossi_mail/src/service.rs` — `RecipientInput` + `create_job` | service | CRUD | `member_id`-Feld (:56) + `member_id`-Threading (:440) | exakt |
| `genossi_service_impl/src/application.rs` — Carry-over-Hook in `confirm()` | service | event-driven (post-commit) | Best-effort Datei-Cleanup nach `commit(tx)` (:547-555) | exakt |
| `genossi_mail/src/service.rs` — neue `MailService`-Methode `link_application_recipients_to_member` | service | CRUD (UPDATE) | bestehende `MailService`-Trait-Methoden (:68-149) | role-match |
| Tests (dao_sqlite Roundtrip, Namespace-Grep-Gate, e2e Carry-over) | test | — | `test_create_job` (service.rs:685), Grep-Gates Phase 26 (`26-02-PLAN`), Confirm-e2e (e2e_tests.rs:6740/6867) | role-match |

## Shared Pattern: „Geschwisterspalte zu `member_id`"

Die zentrale, über alle Dateien gültige Regel: **überall wo `member_id` steht, kommt spiegelbildlich `application_id` daneben** — als `Option<Uuid>` / `Option<Vec<u8>>`, nullable, ohne DEFAULT, ohne NOT NULL, ohne Audit. NIEMALS Application-UUID in `member_id` (Pitfall 2). Die Binär-Kodierung ist immer `.map(|x| x.as_bytes().to_vec())` beim Schreiben, `parse_optional_uuid(&db.field)?` beim Lesen.

---

## Pattern Assignments

### `migrations/sqlite/20260812000000_...add_application_id.sql` (migration, schema-additiv)

**Analog (Spaltenform + NULL-Legacy-Kommentar):** `migrations/sqlite/20260702000002_mail_recipients_add_rendered_html_body.sql` (vollständig gelesen):
```sql
-- NULL-legacy semantics: legacy recipients (pre-migration) read back as
-- rendered_html_body=NULL, matching the text-only contract.
-- Forward-only. SQLite < 3.35 cannot drop columns; no down migration is provided.
ALTER TABLE mail_recipients ADD COLUMN rendered_html_body TEXT NULL;
```

**Analog (Index):** `20260411000001_add_member_communication_indexes.sql`:
```sql
CREATE INDEX IF NOT EXISTS idx_mail_recipients_member_id ON mail_recipients(member_id);
```

**Aktion:** Neue Datei mit fortlaufendem Timestamp (jüngste ist `20260723000000` → z.B. `20260812000000`). Inhalt:
```sql
ALTER TABLE mail_recipients ADD COLUMN application_id BLOB;
CREATE INDEX IF NOT EXISTS idx_mail_recipients_application_id ON mail_recipients(application_id);
```
Kein DEFAULT, kein NOT NULL (spiegelt `member_id BLOB` aus Basistabelle `20260403000004`). **Kein `cargo sqlx prepare` nötig** — Mail-DAOs nutzen ausschließlich Runtime-`sqlx::query`/`query_as`, keine compile-time-Makros (verifiziert in RESEARCH.md:243).

---

### `genossi_mail/src/dao.rs` — `MailRecipient` (model) + `CommunicationDao` (DAO-trait)

**Analog Feld** (`MailRecipient`, verifiziert dao.rs:54-80):
```rust
pub struct MailRecipient {
    pub id: Uuid,
    // ...
    pub member_id: Option<Uuid>,     // :62  ← application_id: Option<Uuid> additiv DANEBEN
    pub status: Arc<str>,
    // ...
}
```

**Analog Trait-Methode** (verifiziert dao.rs:278-285):
```rust
#[automock]
#[async_trait]
pub trait CommunicationDao: Send + Sync + 'static {
    async fn get_member_communications(
        &self,
        member_id: Uuid,
    ) -> Result<Arc<[CommunicationEntry]>, MailDaoError>;
    // NEU: async fn get_application_communications(&self, application_id: Uuid)
    //         -> Result<Arc<[CommunicationEntry]>, MailDaoError>;
}
```

**Aktion:**
1. `pub application_id: Option<Uuid>` in `MailRecipient` direkt nach `member_id` (:62).
2. `get_application_communications`-Methode in `CommunicationDao`-Trait ergänzen.
3. **`CommunicationEntry` NICHT ändern** — es ist subjekt-agnostisch (kein `member_id`-Feld, verifiziert dao.rs:258-276). Auch `#[automock]` bleibt — MockCommunicationDao wird automatisch um die neue Methode erweitert.

---

### `genossi_mail/src/dao_sqlite.rs` — DAO-impl (CRUD + Timeline-read + Test-DDL)

Diese Datei ist der **Kern von Pitfall 6** (Spaltenlisten-Vollständigkeit). Alle folgenden Stellen in EINEM Commit anfassen.

**a) Row-Struct** `MailRecipientDb` (verifiziert :207-225) — Feld ergänzen neben `member_id: Option<Vec<u8>>` (:215):
```rust
#[derive(Debug, sqlx::FromRow)]
struct MailRecipientDb {
    // ...
    member_id: Option<Vec<u8>>,       // :215  ← application_id: Option<Vec<u8>> DANEBEN
    status: String,
    // ...
}
```

**b) `TryFrom<&MailRecipientDb>`** (verifiziert :227-250) — spiegelt `member_id`-Parse (:239):
```rust
member_id: parse_optional_uuid(&db.member_id)?,   // :239
// NEU: application_id: parse_optional_uuid(&db.application_id)?,
```

**c) `create()` INSERT** (verifiziert :264-289) — Bind-Vorbild `member_id` (:269 + :280). Die VALUES-Liste mischt `?`-Binds mit Literalen:
```rust
let member_id = recipient.member_id.map(|m| m.as_bytes().to_vec());   // :269
sqlx::query(
    "INSERT INTO mail_recipients (id, created, deleted, version, mail_job_id, to_address, member_id, status, error, sent_at, message_id, rendered_subject, rendered_body, rendered_reconstructed, rendered_html_body) \
     VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?, NULL)",   // :272-273
)
// ...
.bind(member_id)   // :280
```
Aktion: `application_id`-Spalte in Spaltenliste, weiteres `?` in VALUES, `.bind(recipient.application_id.map(|a| a.as_bytes().to_vec()))` an passender Position.

**d) SELECT-Spaltenlisten** — an jeder Stelle `application_id` in die Liste (nach `member_id`), sonst `FromRow`-Mismatch:
- `find_by_job_id` :294-296
- `next_pending` :311-312 (`r.member_id` → auch `r.application_id`)
- `find_recipients_without_rendered` :379-380
- `find_sent_member_ids_by_job_id` :362-364 selektiert NUR `member_id` → **keine Änderung nötig**, aber bewusst notieren (RESEARCH.md:265 — nach Back-fill tragen Carry-over-Zeilen `member_id IS NOT NULL` und werden hier mitgezählt; bei Einzel-Application-Send unkritisch).
- `update()` :337-338 setzt `member_id` NICHT → keine Änderung nötig.

**e) Neue Timeline-Query** `get_application_communications` — Vorlage = Outbound-Zweig aus `get_member_communications` (verifiziert :1042-1097). **Inbound-`UNION`-Zweig (:1049-1063) WEGLASSEN** (Antragsteller haben keine `assigned_member_id`). Nur der Outbound-Block, aber **alle 12 SELECT-Spalten** (inkl. der NULL-Inbound-Spalten) müssen bleiben, damit `CommunicationEntryDb`-`FromRow` passt:
```rust
async fn get_member_communications(&self, member_id: Uuid) -> ... {
    let member_bytes = member_id.as_bytes().to_vec();
    let rows = sqlx::query_as::<_, CommunicationEntryDb>(r#"
        SELECT 'inbound' AS direction, ... FROM inbound_mails i WHERE i.assigned_member_id = ?1
        UNION ALL
        SELECT 'outbound' AS direction, COALESCE(r.sent_at, r.created) AS date, j.subject,
            NULL AS inbox_id, NULL AS from_address, NULL AS inbound_done, NULL AS inbound_replied,
            NULL AS inbound_archived, j.id AS mail_job_id, r.id AS recipient_id, r.to_address,
            r.status AS outbound_status
        FROM mail_recipients r JOIN mail_jobs j ON j.id = r.mail_job_id
        WHERE r.member_id = ?1 AND r.deleted IS NULL AND j.deleted IS NULL   -- :1082-1084
        ORDER BY date DESC
    "#).bind(&member_bytes).fetch_all(self.pool.as_ref()).await ...
}
```
Aktion neue Methode: nur der zweite SELECT-Block, `WHERE r.application_id = ?1`, Soft-Delete-Filter `r.deleted IS NULL AND j.deleted IS NULL` **beibehalten**.

**f) Test-DDL** (verifiziert :1341-1361) — die 6. leicht übersehene Stelle; Tests bauen eigenes In-Memory-Schema (NICHT über Migrationsdateien):
```sql
CREATE TABLE mail_recipients (
    id BLOB PRIMARY KEY, created TEXT NOT NULL, deleted TEXT, version BLOB NOT NULL,
    mail_job_id BLOB NOT NULL REFERENCES mail_jobs(id), to_address TEXT NOT NULL,
    member_id BLOB,                    -- :1348  ← application_id BLOB DANEBEN
    status TEXT NOT NULL, error TEXT, sent_at TEXT, message_id TEXT,
    rendered_subject TEXT, rendered_body TEXT,
    rendered_reconstructed INTEGER NOT NULL DEFAULT 0, rendered_html_body TEXT
)
```
Aktion: `application_id BLOB` ergänzen, sonst brechen DAO-Roundtrip-Tests am INSERT.

---

### `genossi_mail/src/service.rs` — `RecipientInput` (service-input) + `create_job` (CRUD)

**Analog `RecipientInput`** (verifiziert :54-57):
```rust
pub struct RecipientInput {
    pub address: String,
    pub member_id: Option<Uuid>,     // ← application_id: Option<Uuid> additiv
}
```

**Analog Threading in `create_job`** (verifiziert :432-450):
```rust
for input in &recipients {
    let recipient = MailRecipient {
        // ...
        member_id: input.member_id,        // :440  ← application_id: input.application_id, DANEBEN
        // ...
    };
    self.recipient_dao.create(&recipient).await?;
```

**Aktion + Nebenwirkung:** `RecipientInput` hat **kein** `#[derive(Default)]` und wird an mehreren Stellen als Literal gebaut. Ein neues Pflichtfeld zwingt jede Stelle zu `application_id: None`. Verifizierte Literale:
- `genossi_service_impl/src/application.rs:132-135` (`send_confirmation_mail`):
  ```rust
  let recipient = genossi_mail::service::RecipientInput { address: email, member_id: None };
  ```
- `genossi_mail/src/service.rs:708-711` und `:712-715` (Test-Literale, `member_id: None`).

Empfehlung (RESEARCH.md A3): Feld hinzufügen, alle Literale mechanisch um `application_id: None` ergänzen (Compiler findet sie). Kein Builder-Pattern im Codebase.

---

### `genossi_mail/src/service.rs` — NEU: `link_application_recipients_to_member` (D2 Option A)

**Analog:** bestehende `MailService`-Trait-Methoden (verifiziert :68-149, z.B. `get_reached_member_ids` :149). Neue Signatur:
```rust
// im MailService-Trait (~nach :149):
async fn link_application_recipients_to_member(
    &self, application_id: Uuid, member_id: Uuid,
) -> Result<(), MailServiceError>;
```
Impl führt ein `UPDATE mail_recipients SET member_id = ?new_member_id WHERE application_id = ?app_id AND member_id IS NULL` aus (neue `MailRecipientDao`-Methode dahinter, z.B. `link_application_to_member`). Setzt **ausschließlich die genuine neue `member_id`** — NIE die Application-UUID (Success-Kriterium 2). `#[automock]` auf `MailService`-Trait erzeugt automatisch die Mock-Methode.

---

### `genossi_service_impl/src/application.rs` — Carry-over-Hook (D2, event-driven post-commit)

**Analog (Best-effort post-commit):** Datei-Cleanup nach `commit(tx)` (verifiziert :541-555):
```rust
self.transaction_dao.commit(tx).await?;                              // :541

// Phase 25 best-effort: A failure here does NOT roll back.
if let Some(old_path) = old_app_doc_path_for_cleanup {
    if let Err(e) = self.document_storage.delete(&old_path).await {
        tracing::warn!(old_path = %old_path, error = ?e,
            "Failed to delete old application document file after confirm (best-effort)");  // :549-553
    }
}
Ok(Application::from(&entity))
```

**Kontext:** `member_id = self.uuid_service.new_v4().await` (verifiziert :326) steht **unabhängig von `application.id`** und ist zum Hook-Zeitpunkt bereits fest. `ApplicationServiceDeps` hat bereits `MailService` (verifiziert application.rs:39 `MailService: ... = mail_service`) — **keine neue DAO-Dep nötig**.

**Aktion:** Nach `commit(tx)` (:541), als weiterer best-effort-Schritt (Muster identisch zum Datei-Cleanup):
```rust
if let Err(e) = self.mail_service
    .link_application_recipients_to_member(id /* application_id */, member_id)
    .await
{
    tracing::warn!(application_id = %id, member_id = %member_id, error = ?e,
        "Failed to carry over applicant communications to member (best-effort)");
}
```
**NICHT** in die `confirm()`-Transaktion einbinden — Mail-DAOs laufen auf separater Pool-Connection (`self.pool.as_ref()`, verifiziert dao_sqlite.rs:285), nicht auf `tx` (RESEARCH.md:204-208).

---

## Shared Patterns

### UUID ↔ BLOB Kodierung
**Quelle:** `dao_sqlite.rs` — Schreiben `member_id.map(|m| m.as_bytes().to_vec())` (:269), Lesen `parse_optional_uuid(&db.member_id)?` (:239).
**Anwenden auf:** `application_id` in `create()`-Bind, `TryFrom`, Timeline-Bind.

### Best-effort Post-Commit
**Quelle:** `application.rs:547-555` (Datei-Cleanup) — `if let Err(e) = ... { tracing::warn!(...) }`, kein Rollback.
**Anwenden auf:** D2-Carry-over-Hook.

### `#[automock]`-Trait-Erweiterung
**Quelle:** `CommunicationDao` (:278) und `MailService` (:68) tragen `#[automock]`. Neue Trait-Methoden erzeugen automatisch Mock-Erweiterungen — bestehende `Mock*`-Setups in Tests müssen ggf. `.expect_...()` für die neuen Methoden ergänzen (nur wo aufgerufen).

### Test-Vorbilder
- **DAO-Roundtrip:** `service.rs:685 test_create_job` (RecipientInput→create→assert). Neu: `application_id=Some` Roundtrip + NULL-Legacy (`application_id` weggelassen → liest `None`).
- **Namespace-Grep-Gate (Success-Kriterium 2 / Pitfall 2):** `include_str!`-Source-Invariant-Test analog `26-02-PLAN` — sichert, dass kein Produktivcode `member_id: Some(` mit Application-UUID koppelt; Application-Sends setzen `member_id: None`.
- **e2e Carry-over:** `genossi_bin/tests/e2e_tests.rs:6740 test_confirm_application_creates_member` + `:6867 ...carryover_audited`. Neu: Application `Offen` → `mail_recipients`-Zeile mit `application_id` (in Phase 29 direkt via DAO/Service seeden, da `POST /api/applications/{id}/mail` erst Phase 31) → `confirm()` → `get_member_communications(new_member_id)` enthält Eintrag.

## No Analog Found

Keine. Jede Zieldatei hat ein exaktes `member_id`-Analog in derselben Datei. Die einzige „neue" Konstruktion (`get_application_communications`, `link_application_recipients_to_member`) ist ein reduzierter Klon bestehender Methoden.

## Scope-Grenzen (aus ROADMAP/REQUIREMENTS — Planner beachten)
- **KEIN Audit** auf `application_id`-Linkage; `mail_recipients` bleibt nicht-auditiert (Pitfall 10). Kein neues Feld auf `ApplicationEntity`/`MemberEntity` (bricht sonst `test_auditable_fields_count == 11`, application.rs:176).
- **D2 = Option A** (Back-fill echte `member_id`, post-commit best-effort). Member-Timeline-Query bleibt **unverändert**.
- **REST-Endpoint `GET /api/applications/{id}/communications` ist Phase 31** — Phase 29 verifiziert auf DAO-Ebene (`get_application_communications`). Kein HTTP-Handler in Phase 29 (Scope-Creep-Gefahr, RESEARCH.md Offene Frage 2).
- **Kein `cargo sqlx prepare`** für diese Migration nötig.

## Metadata
**Analog-Suchraum:** `genossi_mail/src/{dao.rs,dao_sqlite.rs,service.rs}`, `genossi_service_impl/src/application.rs`, `migrations/sqlite/`, `genossi_bin/tests/e2e_tests.rs`
**Dateien gescannt:** 6 (alle Datei:Zeile-Anker aus RESEARCH.md gegen echten Code re-verifiziert)
**Extraktions-Datum:** 2026-08-12

## PATTERN MAPPING COMPLETE

**Phase:** 29 - DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller)
**Files classified:** 6 geändert + 1 neu (Migration) + Test-Ebenen
**Analogs found:** 7 / 7

### Coverage
- Files mit exaktem Analog: 5
- Files mit role-match Analog: 2 (neue MailService-Methode, Tests)
- Files ohne Analog: 0

### Key Patterns Identified
- „Geschwisterspalte zu `member_id`": `application_id` spiegelt überall das nullable `member_id`-Konstrukt — Struct-Feld, Row-Struct, alle 6 SQL-Stellen (inkl. Test-DDL :1341), Service-Input, `create_job`-Threading.
- Neue Timeline-Query = reduzierter Klon von `get_member_communications` (nur Outbound-Zweig, `WHERE r.application_id = ?1`, Soft-Delete-Filter beibehalten, alle 12 SELECT-Spalten für `FromRow`).
- D2-Carry-over = best-effort post-commit-Hook nach `commit(tx)` (application.rs:541), exakt nach dem bestehenden Datei-Cleanup-Muster (:547-555); Mail-DAO auf separater Pool-Connection, nie atomar in `tx`.

### File Created
`.planning/phases/29-dao-schema-foundation-kommunikations-historie-pro-antragstel/29-PATTERNS.md`

### Ready for Planning
Pattern-Mapping abgeschlossen. Alle Anker gegen echten Code verifiziert. Der Planner kann `read_first`- und `action`-Werte direkt aus den Datei:Zeile-Auszügen oben formulieren.
