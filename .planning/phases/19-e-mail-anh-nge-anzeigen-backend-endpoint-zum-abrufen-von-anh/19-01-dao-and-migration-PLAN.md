---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-01
slug: dao-and-migration
type: execute
wave: 1
depends_on: []
files_modified:
  - migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql
  - genossi_mail/src/dao.rs
  - genossi_mail/src/dao_sqlite.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "DB table `inbound_mail_attachments` exists with columns id/inbound_mail_id/created/file_name/mime_type/size_bytes/relative_path (NULL)/oversized (D-01, D-02, D-04)"
    - "Trait `InboundMailAttachmentDao` exposes create, find_by_inbound_mail_id, find_by_id_and_mail, count_for_mail (D-07, T-03)"
    - "Entity `InboundMailAttachment` carries `oversized: bool` + `relative_path: Option<Arc<str>>` to encode D-02 oversized rows"
    - "Entity does NOT implement Auditable trait (D-10) — direct DAO calls only, no `audited_*!` macros"
  artifacts:
    - path: "migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql"
      provides: "SQLite DDL for inbound_mail_attachments + FK + index"
      contains: "CREATE TABLE IF NOT EXISTS inbound_mail_attachments"
    - path: "genossi_mail/src/dao.rs"
      provides: "InboundMailAttachment struct + InboundMailAttachmentDao trait"
      contains: "pub struct InboundMailAttachment"
    - path: "genossi_mail/src/dao_sqlite.rs"
      provides: "InboundMailAttachmentDaoSqlite + TryFrom + test-only CREATE TABLE"
      contains: "pub struct InboundMailAttachmentDaoSqlite"
  key_links:
    - from: "genossi_mail/src/dao_sqlite.rs (test schema)"
      to: "inbound_mail_attachments (in-memory DB)"
      via: "CREATE TABLE inside the unit-test bootstrap"
      pattern: "CREATE TABLE inbound_mail_attachments"
    - from: "genossi_mail/src/dao_sqlite.rs (InboundMailAttachmentDaoSqlite::create)"
      to: "sqlite INSERT"
      via: "sqlx::query bind chain"
      pattern: "INSERT INTO inbound_mail_attachments"

---

<objective>
Lege das DAO-Fundament (Migration + Entity + Trait + SQLite-Impl) für die neue
read-only-Entität `InboundMailAttachment` an. Spiegelt vollständig das bestehende
`MailRecipientAttachment`-Pattern.

Purpose: Persistenz-Schicht muss zuerst stehen, damit Service (Plan 19-02) +
REST (Plan 19-03) + Backfill (Plan 19-04) drauf bauen können. Die Migration läuft
beim nächsten `cargo run`/`cargo test` automatisch via `sqlx::migrate!()` in
`genossi_bin/src/main.rs`.

Output: Eine neue Migration, ein neues Entity-Struct + Trait in `dao.rs`, eine
neue SQLite-Impl in `dao_sqlite.rs` (inkl. test-only Schema-Bootstrap). Alle Unit-Tests
in `genossi_mail` bleiben grün; ein neuer Test verifiziert Insert+Find-Roundtrip.
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md
@CLAUDE.md

<interfaces>
<!-- Pre-extracted analog API shapes from PATTERNS.md §1, §2, §3 + RESEARCH.md Code Examples. -->
<!-- Executor must mirror these shapes — do not invent new field orders or method signatures. -->

From `genossi_mail/src/dao.rs:88-105` (analog `MailRecipientAttachment` block):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailRecipientAttachment {
    pub recipient_id: Uuid,
    pub document_id: Uuid,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
}

#[automock]
#[async_trait]
pub trait MailRecipientAttachmentDao: Send + Sync + 'static {
    async fn create(&self, attachment: &MailRecipientAttachment) -> Result<(), MailDaoError>;
    async fn find_by_recipient_id(
        &self,
        recipient_id: Uuid,
    ) -> Result<Arc<[MailRecipientAttachment]>, MailDaoError>;
}
```

Helpers in `genossi_mail/src/dao_sqlite.rs:32` (use these — do not re-implement):
- `format_datetime(&PrimitiveDateTime) -> Result<String, MailDaoError>`
- `parse_datetime(&str) -> Result<PrimitiveDateTime, time::error::Parse>`
- `parse_uuid(&[u8]) -> Result<Uuid, MailDaoError>`

Migration filename rule (PATTERNS.md §3): timestamp must be strictly after
`20260603100000_mail_job_attach_repayment_letter.sql` → use `20260608000000_…`.

Analog migration in `migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql`
(whole file). Index pattern from `migrations/sqlite/20260409000001_create_inbound_mails_table.sql`:
`CREATE INDEX idx_inbound_mails_status ON inbound_mails(status);`
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Migration + Entity + Trait</name>
  <files>
    migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql,
    genossi_mail/src/dao.rs
  </files>
  <read_first>
    - migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql (whole file — DDL analog)
    - migrations/sqlite/20260409000001_create_inbound_mails_table.sql (whole file — FK target + index pattern)
    - genossi_mail/src/dao.rs:80-250 (existing `MailRecipientAttachment` block at :88-105 + `InboundMail` at :222-242 — copy struct/trait shape)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §1, §3 (line-exact patterns)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md decisions D-01, D-02, D-04, D-07, D-10
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Migration SQL (lines 810-826) — exact DDL
  </read_first>
  <behavior>
    - Migration is idempotent (`CREATE TABLE IF NOT EXISTS`) and creates 8 columns + 1 index
    - `relative_path` is nullable (encodes D-02 oversized rows: oversized=1 ⇒ relative_path=NULL)
    - `oversized` is INTEGER NOT NULL DEFAULT 0 (SQLite-bool convention from analog)
    - Index targets `inbound_mail_id` (sole query predicate per RESEARCH §Migration SQL notes)
    - Entity carries `oversized: bool` and `relative_path: Option<Arc<str>>` — both round-trip through DAO
    - Trait exposes exactly 4 methods: `create`, `find_by_inbound_mail_id`, `find_by_id_and_mail`, `count_for_mail` (no update/delete/dump_all — read-only)
    - Trait is `#[automock]` decorated so `MockInboundMailAttachmentDao` exists for downstream tests
    - Entity does NOT implement `Auditable` (D-10) — no `genossi_dao::auditable::Auditable` impl block, no use-import for it
  </behavior>
  <action>
    **Step 1 — Create migration** `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` with verbatim content:
    ```sql
    CREATE TABLE IF NOT EXISTS inbound_mail_attachments (
        id BLOB PRIMARY KEY NOT NULL,
        inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
        created TEXT NOT NULL,
        file_name TEXT NOT NULL,
        mime_type TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        relative_path TEXT,
        oversized INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX idx_inbound_mail_attachments_mail ON inbound_mail_attachments(inbound_mail_id);
    ```

    **Step 2 — In `genossi_mail/src/dao.rs`**, insert the new struct + trait immediately AFTER the existing `MailRecipientAttachmentDao` trait block (around line 105). Use the existing `use` statements (Uuid, Arc, async_trait, automock, time::PrimitiveDateTime — verify they are imported; if not, extend the use-block).

    Add (verbatim):
    ```rust
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct InboundMailAttachment {
        pub id: Uuid,
        pub inbound_mail_id: Uuid,
        pub created: time::PrimitiveDateTime,
        pub file_name: Arc<str>,
        pub mime_type: Arc<str>,
        pub size_bytes: i64,
        pub relative_path: Option<Arc<str>>, // NULL when oversized=true (D-02)
        pub oversized: bool,                 // D-02 hard 10 MB cap marker
    }

    #[automock]
    #[async_trait]
    pub trait InboundMailAttachmentDao: Send + Sync + 'static {
        async fn create(&self, attachment: &InboundMailAttachment) -> Result<(), MailDaoError>;
        async fn find_by_inbound_mail_id(
            &self,
            inbound_mail_id: Uuid,
        ) -> Result<Arc<[InboundMailAttachment]>, MailDaoError>;
        async fn find_by_id_and_mail(
            &self,
            mail_id: Uuid,
            attachment_id: Uuid,
        ) -> Result<Option<InboundMailAttachment>, MailDaoError>;
        async fn count_for_mail(&self, mail_id: Uuid) -> Result<i64, MailDaoError>;
    }
    ```

    Notes you MUST follow:
    - Do NOT add `version: Uuid` or `deleted: Option<PrimitiveDateTime>` — read-only entity, no soft-delete (RESEARCH §Project Constraints — note Soft-Delete-Cascade-Sonderfall)
    - Do NOT add `Auditable` impl for `InboundMailAttachment` (D-10) — direct DAO calls only
    - Trait method count is exactly 4 — no `update`, no `delete`, no `dump_all`
  </action>
  <verify>
    <automated>cargo check -p genossi_mail 2>&amp;1 | tee /tmp/19-01-task1.log; ! grep -q "^error" /tmp/19-01-task1.log &amp;&amp; grep -q "trait InboundMailAttachmentDao" genossi_mail/src/dao.rs &amp;&amp; grep -q "pub struct InboundMailAttachment" genossi_mail/src/dao.rs &amp;&amp; test -f migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql &amp;&amp; grep -q "CREATE TABLE IF NOT EXISTS inbound_mail_attachments" migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql</automated>
  </verify>
  <acceptance_criteria>
    - File `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` exists
    - Migration file contains exact substrings: `CREATE TABLE IF NOT EXISTS inbound_mail_attachments`, `id BLOB PRIMARY KEY NOT NULL`, `inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id)`, `relative_path TEXT,`, `oversized INTEGER NOT NULL DEFAULT 0`, `CREATE INDEX idx_inbound_mail_attachments_mail`
    - `grep -c "pub struct InboundMailAttachment" genossi_mail/src/dao.rs` returns 1
    - `grep -c "pub trait InboundMailAttachmentDao" genossi_mail/src/dao.rs` returns 1
    - `grep -c "#\[automock\]" genossi_mail/src/dao.rs` returns ≥ 2 (analog + new trait)
    - `grep -F "Auditable for InboundMailAttachment" genossi_mail/src/dao.rs` returns nothing (D-10 — must NOT be auditable)
    - `grep -F "pub version:" genossi_mail/src/dao.rs | grep -A2 InboundMailAttachment` returns nothing (no optimistic locking on read-only entity)
    - `cargo check -p genossi_mail` exits 0
  </acceptance_criteria>
  <done>
    Migration file exists with correct DDL. Entity + Trait compiled and `automock`-derived. No Auditable impl. cargo check green.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: SQLite DAO Impl + Roundtrip Test</name>
  <files>genossi_mail/src/dao_sqlite.rs</files>
  <read_first>
    - genossi_mail/src/dao_sqlite.rs:32 (find `format_datetime` + `parse_datetime` + `parse_uuid` helpers — DO NOT re-implement)
    - genossi_mail/src/dao_sqlite.rs:359-435 (analog `MailRecipientAttachmentDaoSqlite` — struct + TryFrom + impl)
    - genossi_mail/src/dao_sqlite.rs:1130-1192 (existing test-only CREATE TABLE bootstrap — insert new test schema HERE)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Code Examples → DAO: SQLite Implementation Pattern (lines 711-805) — copy verbatim
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §2 — exact analog references
  </read_first>
  <behavior>
    - `InboundMailAttachmentDb` is `sqlx::FromRow`-derived with 8 fields matching column order: id BLOB → Vec<u8>, inbound_mail_id BLOB → Vec<u8>, created TEXT → String, file_name TEXT → String, mime_type TEXT → String, size_bytes INTEGER → i64, relative_path TEXT NULL → Option<String>, oversized INTEGER → i64
    - `TryFrom<&InboundMailAttachmentDb> for InboundMailAttachment` converts Vec<u8> → Uuid (via `parse_uuid`), String → Arc<str>, oversized != 0 → bool
    - `create` INSERTs all 8 columns in same order (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized); oversized is bound as `if a.oversized { 1i64 } else { 0i64 }`
    - `find_by_inbound_mail_id` SELECTs all 8 columns WHERE inbound_mail_id = ? ORDER BY created ASC
    - `find_by_id_and_mail` SELECTs WHERE id = ? AND inbound_mail_id = ? (both bind args required — IDOR mitigation per T-03)
    - `count_for_mail` returns SELECT COUNT(*) WHERE inbound_mail_id = ?
    - Test-only schema bootstrap creates the new table inside the existing in-memory-test setup block at `:1167-1192` so unit tests can roundtrip
    - One new unit test `test_inbound_mail_attachment_roundtrip` inserts an entity (with oversized=false), reads via `find_by_inbound_mail_id`, asserts equality, then a second insert with oversized=true / relative_path=None and verifies the NULL column round-trips
    - Second unit test `test_find_by_id_and_mail_wrong_mail_returns_none` verifies that requesting (mail_B_id, attachment_A_id) returns None — T-03 IDOR guard
  </behavior>
  <action>
    **Step 1 — Insert DAO struct + impl** in `genossi_mail/src/dao_sqlite.rs` immediately AFTER the existing `MailRecipientAttachmentDaoSqlite` impl block (after line ~435).

    Use the exact code block from `19-RESEARCH.md` §Code Examples → DAO: SQLite Implementation Pattern (lines 711-805). Specifically copy:

    - `#[derive(Debug, sqlx::FromRow)] struct InboundMailAttachmentDb { … 8 fields … }`
    - `impl TryFrom<&InboundMailAttachmentDb> for InboundMailAttachment { … }` (uses `parse_uuid`, `parse_datetime`)
    - `pub struct InboundMailAttachmentDaoSqlite { pool: Arc<SqlitePool> }`
    - `impl InboundMailAttachmentDaoSqlite { pub fn new(pool: Arc<SqlitePool>) -> Self { Self { pool } } }`
    - `#[async_trait] impl InboundMailAttachmentDao for InboundMailAttachmentDaoSqlite { … 4 methods … }`

    The SQL strings to bind must match exactly:
    - INSERT: `"INSERT INTO inbound_mail_attachments (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"`
    - find_by_inbound_mail_id: `"SELECT id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized FROM inbound_mail_attachments WHERE inbound_mail_id = ? ORDER BY created ASC"`
    - find_by_id_and_mail: `"SELECT id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized FROM inbound_mail_attachments WHERE id = ? AND inbound_mail_id = ?"`
    - count_for_mail: `"SELECT COUNT(*) FROM inbound_mail_attachments WHERE inbound_mail_id = ?"`

    **Step 2 — Add test-only CREATE TABLE** inside the existing test bootstrap block at `dao_sqlite.rs:1167-1192` (the same place the `inbound_mails` test schema lives). Insert AFTER the `inbound_mails` block:

    ```rust
    sqlx::query(
        "CREATE TABLE inbound_mail_attachments (
            id BLOB PRIMARY KEY NOT NULL,
            inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
            created TEXT NOT NULL,
            file_name TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            relative_path TEXT,
            oversized INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create inbound_mail_attachments table");
    ```

    **Step 3 — Add 2 unit tests** in the existing `#[cfg(test)] mod tests` of `dao_sqlite.rs`:

    Test A — `test_inbound_mail_attachment_roundtrip`:
    1. Boot in-memory DB via existing helper
    2. Seed parent `InboundMail` row (use existing test helper or direct INSERT)
    3. Build `InboundMailAttachment` (id=Uuid::new_v4(), file_name="invoice.pdf", mime_type="application/pdf", size_bytes=12345, relative_path=Some(Arc::from("inbound_mail_attachments/<mid>/<aid>")), oversized=false)
    4. `dao.create(&a).await.unwrap()`
    5. `let list = dao.find_by_inbound_mail_id(parent_mail_id).await.unwrap()` → assert `list.len() == 1`, `list[0].file_name.as_ref() == "invoice.pdf"`, `list[0].oversized == false`, `list[0].relative_path.is_some()`
    6. Build second `InboundMailAttachment` with `oversized=true, relative_path=None`
    7. `dao.create(&b).await.unwrap()`
    8. Re-fetch → assert `list.len() == 2`, find the oversized one, assert `relative_path.is_none() && oversized == true`
    9. `assert_eq!(dao.count_for_mail(parent_mail_id).await.unwrap(), 2)`

    Test B — `test_find_by_id_and_mail_wrong_mail_returns_none`:
    1. Seed parent mail A + attachment A1 under mail A
    2. Seed parent mail B (no attachments)
    3. `dao.find_by_id_and_mail(mail_b_id, attachment_a1_id).await.unwrap()` → assert `is_none()` (T-03 cross-mail IDOR mitigation)
    4. `dao.find_by_id_and_mail(mail_a_id, attachment_a1_id).await.unwrap()` → assert `is_some()` (positive control)
  </action>
  <verify>
    <automated>cargo test -p genossi_mail dao_sqlite::tests::test_inbound_mail_attachment_roundtrip dao_sqlite::tests::test_find_by_id_and_mail_wrong_mail_returns_none -- --nocapture 2>&amp;1 | tee /tmp/19-01-task2.log; grep -q "test result: ok. 2 passed" /tmp/19-01-task2.log</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "pub struct InboundMailAttachmentDaoSqlite" genossi_mail/src/dao_sqlite.rs` returns 1
    - `grep -c "impl InboundMailAttachmentDao for InboundMailAttachmentDaoSqlite" genossi_mail/src/dao_sqlite.rs` returns 1
    - `grep -c "INSERT INTO inbound_mail_attachments" genossi_mail/src/dao_sqlite.rs` returns ≥ 1
    - `grep -c "CREATE TABLE inbound_mail_attachments" genossi_mail/src/dao_sqlite.rs` returns 1 (test-only schema)
    - `grep -c "test_inbound_mail_attachment_roundtrip" genossi_mail/src/dao_sqlite.rs` returns ≥ 2 (declaration + #[tokio::test] block)
    - `grep -c "test_find_by_id_and_mail_wrong_mail_returns_none" genossi_mail/src/dao_sqlite.rs` returns ≥ 2
    - `cargo test -p genossi_mail` exits 0 (all genossi_mail tests still pass, including the 2 new ones)
  </acceptance_criteria>
  <done>
    SQLite-Impl kompiliert, beide neuen Tests grün, Bestandstests grün, Test-Schema bootstrapped in-Memory.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| DAO → SQLite | Untrusted file paths could leak into `relative_path` if upstream callers don't sanitize; this plan does NOT enforce that — Plan 19-02 owns generation, DAO simply persists the string |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-03 | Information Disclosure | `find_by_id_and_mail` | mitigate | DAO query requires BOTH `attachment_id` AND `inbound_mail_id` to match → cross-mail enumeration returns None. Unit test `test_find_by_id_and_mail_wrong_mail_returns_none` enforces this gate in Task 2. |

(T-01, T-02, T-04..T-08 are owned by other plans — see 19-02 / 19-03 / 19-04 / 19-05.)
</threat_model>

<verification>
- `cargo check -p genossi_mail` exits 0
- `cargo test -p genossi_mail` exits 0 (all tests including 2 new ones pass)
- File `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` exists
- `grep -c "Auditable for InboundMailAttachment" genossi_mail/src/dao.rs` returns 0 (D-10 enforcement gate)
</verification>

<success_criteria>
- Migration file present with expected DDL (8 columns + 1 index)
- `InboundMailAttachment` struct + `InboundMailAttachmentDao` trait exist with exactly 4 methods
- `InboundMailAttachmentDaoSqlite` implements all 4 trait methods
- `MockInboundMailAttachmentDao` auto-generated via `#[automock]` (downstream plans depend on this)
- Test-only schema bootstrap present in test module
- 2 new unit tests pass: roundtrip + IDOR
- Bestandstests grün, kein Audit-Trait-Impl angelegt (D-10)
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-01-SUMMARY.md`
</output>
