# Phase 10: Massenmail-Anbindung + Template-Variablen — Pattern Map

**Mapped:** 2026-05-31
**Files to create/modify:** 13
**Analogs found:** 13 / 13 (1 NEW pattern: Worker-Cross-Crate-Audit)

---

## File Classification

| File | New/Modified | Role | Data Flow | Closest Analog | Match Quality |
|------|--------------|------|-----------|----------------|---------------|
| `migrations/sqlite/2026XXXX_extend_mail_job_template_phase.sql` | NEW | migration | DDL | `20260506000000_add_code_to_helper_token.sql` | exact |
| `migrations/sqlite/2026XXXX_extend_member_document_mail.sql` | NEW | migration | DDL | `20260506000000_add_code_to_helper_token.sql` | exact |
| `genossi_mail/src/dao.rs` (`MailJob` struct) | MODIFIED | DAO entity | data | self (existing `MailJob`) | exact |
| `genossi_mail/src/dao_sqlite.rs` (MailJobDaoSqlite) | MODIFIED | DAO impl SQLite | READ-WRITE | self (existing `MailJobDaoSqlite`) | exact |
| `genossi_dao/src/member_document.rs` (entity + Auditable) | MODIFIED | DAO entity + Auditable | data | `genossi_dao/src/repayment_entry.rs` (FROZEN-Pattern) | exact |
| `genossi_dao_impl_sqlite/src/member_document.rs` | MODIFIED | DAO impl SQLite | READ-WRITE | self | exact |
| `genossi_service/src/member_document.rs` (DocumentType) | MODIFIED | Service trait | enum | self (existing variants) | exact |
| `genossi_mail/src/service.rs` (MailService trait + Impl) | MODIFIED | Service | TX-CRITICAL | self (`create_job`) | exact |
| `genossi_mail/src/rest.rs` (SendBulkMailRequest + handler) | MODIFIED | REST handler | request-response | self (`send_bulk_mail`) | exact |
| `genossi_mail/src/template.rs` (merge helper + tests) | MODIFIED | utility | transform | `member_to_template_context` | exact |
| `genossi_mail/src/worker.rs` (start_mail_worker + render+create cascade) | MODIFIED | worker | READ-WRITE (audited) | self + `genossi_service_impl/src/member_document.rs:120-142` | role-match (cross-crate NEW) |
| `genossi_mail/Cargo.toml` (add `genossi_service_impl` dep) | MODIFIED | config | dependency | self | exact |
| `genossi_bin/src/lib.rs` (`start_mail_worker` wiring) | MODIFIED | binary | DI | self (existing wiring) | exact |

---

## Pattern Assignments

### Migration: `mail_job` + `mail_recipient` (D-12, D-03)

**File:** `migrations/sqlite/2026XXXX_extend_mail_job_template_phase.sql`
**Role:** migration · **Data Flow:** DDL · **Match:** exact

**Analog:** `migrations/sqlite/20260506000000_add_code_to_helper_token.sql`

**Pattern excerpt** (lines 1-19):
```sql
-- ADR 2026-05-06: persist helper_token plaintext code so the Vorstand can
-- re-display QR + manual-code at any time (Phase 2 D-11 / D-21 trade-off).
-- ...
-- Audit-Log MUST exclude this column (parallel rationale to D-06: avoid a
-- second persistent code store in the audit hash chain). See
-- `genossi_dao::helper_token::HelperTokenEntity::audit_fields()`.
-- ...
-- No down-migration: SQLite < 3.35 has no `DROP COLUMN`; the project ships
-- only forward migrations (see migrations/sqlite/*).

ALTER TABLE helper_token ADD COLUMN code TEXT NULL;
```

**Specifics for Phase 10:**
- Forward-only, no DOWN migration
- NULL-able columns (backward-compat with existing rows; D-08)
- FK clauses are DOCUMENTARY only (project does not enable `PRAGMA foreign_keys=ON` per repayment_entry-migration line 4-7)
- Two ALTER statements in one file (template_id, repayment_phase_id)
- ADR-style comment header explaining intent

**Recommended SQL:**
```sql
-- ADR Phase 10 (D-12 / D-03): mail_job gets two optional refs so the worker can
-- (a) record which template was used (template_id → MemberDocument.template_id)
-- and (b) merge job-wide repayment context into per-recipient render.
-- FK clauses are documentary (project does not enable PRAGMA foreign_keys=ON).
-- ON DELETE SET NULL semantics: deleting a template/phase must not break audit.

ALTER TABLE mail_jobs ADD COLUMN template_id BLOB NULL;
ALTER TABLE mail_jobs ADD COLUMN repayment_phase_id BLOB NULL;
```

---

### Migration: `member_document` (D-07)

**File:** `migrations/sqlite/2026XXXX_extend_member_document_mail.sql`
**Role:** migration · **Data Flow:** DDL · **Match:** exact

**Analog:** same as above + `member_document` schema in `20260331000005_create_member_document_table.sql`

**Pattern (same ADR header style):**
```sql
-- ADR Phase 10 (D-07 / D-09): member_document gets three optional fields so a
-- repayment-mail send can be persisted as an audited MemberDocument
-- (status='sent'|'failed', linked to MailRecipient + MailTemplate).
-- All three columns are NULL-able: existing rows (JoinDeclaration etc.) keep
-- NULL values, backward-compat is preserved (D-08).

ALTER TABLE member_document ADD COLUMN template_id BLOB NULL;
ALTER TABLE member_document ADD COLUMN mail_recipient_id BLOB NULL;
ALTER TABLE member_document ADD COLUMN status TEXT NULL;
```

---

### `genossi_dao/src/member_document.rs` — Auditable Extension

**Role:** DAO entity + Auditable trait · **Data Flow:** data · **Match:** exact

**Analog:** `genossi_dao/src/repayment_entry.rs:70-92` (FROZEN-Pattern for hash-chain stability)

**Existing `audit_fields()`** (lines 32-44, the analog to extend):
```rust
fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
    vec![
        ("member_id", Some(self.member_id.to_string())),
        ("document_type", Some(self.document_type.to_string())),
        (
            "description",
            self.description.as_ref().map(|s| s.to_string()),
        ),
        ("file_name", Some(self.file_name.to_string())),
        ("mime_type", Some(self.mime_type.to_string())),
        ("relative_path", Some(self.relative_path.to_string())),
    ]
}
```

**FROZEN-Order pattern reference** (`repayment_entry.rs:79-92`):
```rust
fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
    // FROZEN ORDER (Hash-Chain-Konsistenz, Phase-7-Lektion):
    // member_id, phase_id, share_count_to_pay_out, status
    vec![
        ("member_id", Some(self.member_id.to_string())),
        ("phase_id", Some(self.phase_id.to_string())),
        ("share_count_to_pay_out", Some(self.share_count_to_pay_out.to_string())),
        ("status", Some(self.status.as_str().to_string())),
    ]
}
```

**Specifics for Phase 10 (D-08):**
- Add struct fields: `template_id: Option<Uuid>`, `mail_recipient_id: Option<Uuid>`, `status: Option<Arc<str>>`
- Append to `audit_fields()` **AT END** (existing fields stay at indices 0-5 → backward-compat with existing audit-history; new fields go at 6-8). Comment explaining: "FROZEN — new fields appended; existing rows have NULL → audit-history unaffected"
- Update existing FROZEN-count test (`test_auditable_fields_count` → `assert_eq!(fields.len(), 9)`)
- Mirror conversion fields in `From<&MemberDocumentEntity> for MemberDocument` and reverse (`genossi_service/src/member_document.rs:108-141`)

---

### `genossi_dao_impl_sqlite/src/member_document.rs` — SQLite-Impl Erweiterung

**Role:** DAO impl SQLite · **Data Flow:** READ-WRITE · **Match:** exact

**Analog:** self (lines 29-200) — existing `MemberDocumentDb` + INSERT/UPDATE/SELECT

**Existing `MemberDocumentDb`** (lines 29-41):
```rust
#[derive(Debug, sqlx::FromRow)]
struct MemberDocumentDb {
    id: Vec<u8>,
    member_id: Vec<u8>,
    document_type: String,
    description: Option<String>,
    file_name: String,
    mime_type: String,
    relative_path: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}
```

**Existing `dump_all` SELECT** (lines 80-87):
```rust
let rows = sqlx::query_as::<_, MemberDocumentDb>(
    "SELECT id, member_id, document_type, description, file_name, mime_type, \
     relative_path, created, deleted, version \
     FROM member_document ORDER BY created",
)
```

**Existing INSERT** (lines 116-130):
```rust
sqlx::query(
    "INSERT INTO member_document (id, member_id, document_type, description, file_name, \
     mime_type, relative_path, created, version) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
)
```

**Specifics for Phase 10:**
- Append `template_id: Option<Vec<u8>>`, `mail_recipient_id: Option<Vec<u8>>`, `status: Option<String>` to `MemberDocumentDb`
- Extend SELECT in `dump_all` to include new columns
- Extend INSERT (9 → 12 placeholders) and UPDATE (5 → 8 SET clauses)
- Reuse existing `parse_optional_uuid` helper from `genossi_mail/src/dao_sqlite.rs:46-52` (port as local helper or duplicate)
- `TryFrom<&MemberDocumentDb>` extended with the 3 new fields

---

### `genossi_service/src/member_document.rs` — `DocumentType::RepaymentMail` (D-09)

**Role:** Service trait · **Data Flow:** enum · **Match:** exact

**Analog:** self (lines 48-92) — existing 4-variant enum

**Existing `DocumentType` enum + impls** (lines 48-92):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentType {
    JoinDeclaration,
    JoinConfirmation,
    ShareIncrease,
    Other,
}

impl DocumentType {
    pub fn as_str(&self) -> &str {
        match self {
            DocumentType::JoinDeclaration => "join_declaration",
            DocumentType::JoinConfirmation => "join_confirmation",
            DocumentType::ShareIncrease => "share_increase",
            DocumentType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "join_declaration" => Some(DocumentType::JoinDeclaration),
            "join_confirmation" => Some(DocumentType::JoinConfirmation),
            "share_increase" => Some(DocumentType::ShareIncrease),
            "other" => Some(DocumentType::Other),
            _ => None,
        }
    }

    pub fn is_singleton(&self) -> bool {
        matches!(
            self,
            DocumentType::JoinDeclaration | DocumentType::JoinConfirmation
        )
    }

    pub fn template_path(&self) -> Option<&str> {
        match self {
            DocumentType::JoinConfirmation => Some("join_confirmation.typ"),
            DocumentType::JoinDeclaration => Some("join_declaration.typ"),
            _ => None,
        }
    }
}
```

**Specifics for Phase 10 (D-09):**
- Add variant `RepaymentMail`
- `as_str()` → `"repayment_mail"`
- `from_str()` → `"repayment_mail" => Some(Self::RepaymentMail)`
- `is_singleton()` → stays `false` for `RepaymentMail` (multi-mail per member)
- `template_path()` → `None` for `RepaymentMail` (no typst template; D-09)
- Also extend struct `MemberDocument` (lines 94-106) with `template_id`, `mail_recipient_id`, `status` if pattern is to surface them at service layer (Planner-Discretion: minimal change is keep them only on `MemberDocumentEntity`)

---

### `genossi_mail/src/dao.rs` — `MailJob` Struct (D-03, D-12)

**Role:** DAO entity · **Data Flow:** data · **Match:** exact

**Analog:** self (lines 27-40)

**Existing `MailJob`** (lines 27-40):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailJob {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    pub subject: Arc<str>,
    pub body: Arc<str>,
    pub status: Arc<str>,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
    pub reply_to_inbound_mail_id: Option<Uuid>,
}
```

**Specifics for Phase 10:**
- Add `pub template_id: Option<Uuid>`
- Add `pub repayment_phase_id: Option<Uuid>`
- Position: after `reply_to_inbound_mail_id` (last position — matches helper_token-migration convention of "append-only")
- `MailJob` is **NOT** Auditable (not in the audit-pflicht-list; CLAUDE.md "Adding Audit to New Entities" — Job is operational, not domain)

---

### `genossi_mail/src/dao_sqlite.rs` — `MailJobDaoSqlite` Erweiterung

**Role:** DAO impl SQLite · **Data Flow:** READ-WRITE · **Match:** exact

**Analog:** self (lines 60-184)

**Existing `MailJobDb` + parse-helper pattern** (lines 60-73):
```rust
#[derive(Debug, sqlx::FromRow)]
struct MailJobDb {
    id: Vec<u8>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    subject: String,
    body: String,
    status: String,
    total_count: i64,
    sent_count: i64,
    failed_count: i64,
    reply_to_inbound_mail_id: Option<Vec<u8>>,
}
```

**Existing `TryFrom` w/ `parse_optional_uuid`** (lines 75-94):
```rust
impl TryFrom<&MailJobDb> for MailJob {
    type Error = MailDaoError;

    fn try_from(db: &MailJobDb) -> Result<Self, Self::Error> {
        Ok(MailJob {
            id: parse_uuid(&db.id)?,
            created: parse_datetime(&db.created)...,
            deleted: parse_optional_datetime(&db.deleted)?,
            version: parse_uuid(&db.version)?,
            subject: Arc::from(db.subject.as_str()),
            // ...
            reply_to_inbound_mail_id: parse_optional_uuid(&db.reply_to_inbound_mail_id)?,
        })
    }
}
```

**Existing INSERT** (lines 115-131):
```rust
sqlx::query(
    "INSERT INTO mail_jobs (id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id) \
     VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?)",
)
.bind(id)
.bind(created)
// ...
.bind(reply_to)
```

**Specifics for Phase 10:**
- Append `template_id: Option<Vec<u8>>` and `repayment_phase_id: Option<Vec<u8>>` to `MailJobDb`
- Reuse existing `parse_optional_uuid` helper (lines 46-52) — no new code needed
- Extend INSERT VALUES placeholders (11 → 13), SELECT columns in `find_by_id`/`all` (3 places)
- `update()` (lines 166-183) does NOT need template_id/repayment_phase_id columns — they're immutable after creation, identical to `subject`/`body`/`created` semantics (which existing update also ignores)

---

### `genossi_mail/src/service.rs` — `MailService::create_job` Signature (D-03)

**Role:** Service · **Data Flow:** TX-CRITICAL · **Match:** exact

**Analog:** self (lines 50-82 trait, 234-321 impl)

**Existing trait signature** (lines 50-63):
```rust
#[automock]
#[async_trait]
pub trait MailService: Send + Sync + 'static {
    /// Create a mail job with the given recipients. Returns the created job.
    /// If attachment_inputs is non-empty, recipients must contain exactly one entry.
    /// `static_document_ids` are job-level attachments delivered to every recipient.
    async fn create_job(
        &self,
        subject: &str,
        body: &str,
        recipients: Vec<RecipientInput>,
        attachment_inputs: Vec<AttachmentInput>,
        static_document_ids: Vec<Uuid>,
    ) -> Result<MailJob, MailServiceError>;
```

**Existing `MailJob` construction** (lines 265-280):
```rust
let now = time::OffsetDateTime::now_utc();
let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

let job = MailJob {
    id: Uuid::new_v4(),
    created: now_primitive,
    deleted: None,
    version: Uuid::new_v4(),
    subject: Arc::from(subject),
    body: Arc::from(body),
    status: Arc::from("running"),
    total_count: recipients.len() as i64,
    sent_count: 0,
    failed_count: 0,
    reply_to_inbound_mail_id: None,
};
```

**Specifics for Phase 10:**
- Extend signature with `template_id: Option<Uuid>` and `repayment_phase_id: Option<Uuid>` (positional, append at end)
- All existing call-sites must update: `rest.rs:269-281` (single-send, passes `None, None`), `rest.rs:380-389` (bulk-send, passes parsed UUIDs), plus reply-flow if present
- `MailJob` struct-init gains the two new fields
- **MockMailService**: `#[automock]` auto-regenerates — but downstream test code must pass `None, None` (find via `cargo check`, fix call-sites)

---

### `genossi_mail/src/rest.rs` — `SendBulkMailRequest` + Handler (D-03, D-12)

**Role:** REST handler · **Data Flow:** request-response · **Match:** exact

**Analog:** self (lines 112-123 struct, 305-399 handler)

**Existing `SendBulkMailRequest`** (lines 112-123):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SendBulkMailRequest {
    pub to_addresses: Vec<BulkRecipient>,
    #[schema(example = "Test Subject")]
    pub subject: String,
    #[schema(example = "Hello, this is a test email.")]
    pub body: String,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    #[serde(default)]
    pub static_document_ids: Vec<String>,
}
```

**Existing UUID-Parsing-Pattern in handler** (lines 374-378):
```rust
let mut static_document_ids: Vec<uuid::Uuid> = Vec::new();
for sid in &body.static_document_ids {
    let parsed = uuid::Uuid::parse_str(sid).map_err(|_| MailServiceError::NotFound)?;
    static_document_ids.push(parsed);
}
```

**Existing `create_job` call** (lines 380-389):
```rust
let job = state
    .mail_service()
    .create_job(
        &body.subject,
        &body.body,
        recipients,
        attachment_inputs,
        static_document_ids,
    )
    .await?;
```

**Specifics for Phase 10:**
- Append two `#[serde(default)] pub template_id: Option<String>` and `pub repayment_phase_id: Option<String>` to `SendBulkMailRequest`
- Reuse UUID-parsing pattern: parse both as `Option<Uuid>` before calling `create_job`. On parse error → `MailServiceError::BadRequest`
- Update `ApiDoc` schemas list (line 246) — no change needed if struct is already there
- `send_mail` handler (lines 263-291) passes `None, None` (single-send is not template-bound)

---

### `genossi_mail/src/template.rs` — Merge Helper + Tests (D-04, D-05, D-15)

**Role:** utility · **Data Flow:** transform · **Match:** exact

**Analog:** `member_to_template_context` (lines 15-40) and existing `{% if %}`-pattern tests (lines 188-225)

**Existing `member_to_template_context`** (lines 15-40):
```rust
pub fn member_to_template_context(entity: &MemberEntity) -> Value {
    let salutation_str = entity.salutation.as_ref().map(|s| s.as_str().to_string());
    let join_date_str = entity.join_date.to_string();
    let exit_date_str = entity.exit_date.map(|d| d.to_string());
    context! {
        member_number => entity.member_number,
        first_name => entity.first_name.as_ref(),
        last_name => entity.last_name.as_ref(),
        email => entity.email.as_deref(),
        // ... 16 more fields
        title => entity.title.as_deref(),
    }
}
```

**Existing `{% if optional_field %}` test pattern** (lines 188-205):
```rust
#[test]
fn test_null_field_conditional() {
    let member = make_member("Max", "Mustermann");
    // company is None
    let ctx = member_to_template_context(&member);
    let template = "{% if company %}Firma: {{ company }}{% endif %}Ende";
    let result = render_template(template, &ctx).unwrap();
    assert_eq!(result, "Ende");
}
```

**Specifics for Phase 10:**
- **DO NOT modify `member_to_template_context`** (context.md D-04 / canonical-ref: "Phase 10 NICHT verändern, sondern den Repayment-Context separat mergen")
- Add new function `merge_repayment_context(base: Value, payout_amount: &str, share_count: i32, fiscal_year: i32) -> Value` that creates a new minijinja context with the base fields PLUS the three new ones via `context!` macro composition or `Value::from_serialize`
- Alternative pattern (cleaner): pass values as `Option<...>` and build context with `{% if %}`-safe undefined when None (matches D-05 edge-case — Member has 0 entries → render without these vars; strict-env triggers `mark_recipient_failed`)
- Add unit tests mirroring `test_null_field_conditional`, `test_present_optional_field`:
  - `test_payout_amount_present_renders_value`
  - `test_payout_amount_missing_with_if_guard_renders_empty`
  - `test_payout_amount_missing_without_guard_fails_strict` (asserts `render_template` returns `Err`)
  - `test_fiscal_year_in_context` (numeric type, plain int — analog to `current_shares` line 209-218)
- Optional: extend `validate_template` (lines 71-116) with a probe-render against `(payout_amount="0,00", share_count=0, fiscal_year=2026)` when `repayment_phase_id` is supplied (D-14 — Planner-Discretion)

---

### `genossi_mail/src/worker.rs` — Repayment-Context-Merge + MemberDocument-Create (D-04, D-10, D-11)

**Role:** worker · **Data Flow:** READ-WRITE (audited) · **Match:** role-match + NEW cross-crate audit pattern

**Analog:**
1. Worker render+send loop: self (lines 113-332)
2. `audited_create!` invocation: `genossi_service_impl/src/member_document.rs:120-142`

**Existing render pattern in worker** (lines 181-249):
```rust
let (rendered_subject, rendered_body) = if let Some(member_id) = next.member_id {
    match member_resolver.find_member_by_id(member_id).await {
        Ok(Some(member)) => {
            let ctx = member_to_template_context(&member);
            let subject = match render_template(&job.subject, &ctx) {
                Ok(s) => s,
                Err(e) => {
                    mark_recipient_failed(
                        recipient_dao.as_ref(),
                        job_dao.as_ref(),
                        &next,
                        &mut job,
                        &format!("Template render error (subject): {}", e.message),
                    )
                    .await;
                    let interval = get_send_interval(config_service.as_ref()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    continue;
                }
            };
            // ... same for body
            (subject, body)
        }
        // ... Ok(None) and Err(_) branches also mark_recipient_failed + continue
    }
} else {
    (job.subject.to_string(), job.body.to_string())
};
```

**Existing recipient-update + job-status-completion pattern** (lines 287-327):
```rust
let mut updated_recipient = next.clone();
updated_recipient.version = uuid::Uuid::new_v4();

match send_result {
    Ok(message_id) => {
        updated_recipient.status = Arc::from("sent");
        updated_recipient.sent_at = Some(now_primitive);
        updated_recipient.message_id = message_id.map(Arc::from);
        job.sent_count += 1;
        tracing::info!("Worker: sent mail to {} (job {})", next.to_address, job.id);
    }
    Err(e) => {
        let error_msg = format!("{:?}", e);
        updated_recipient.status = Arc::from("failed");
        updated_recipient.error = Some(Arc::from(error_msg.as_str()));
        job.failed_count += 1;
        tracing::error!(...);
    }
}

// Update recipient
if let Err(e) = recipient_dao.update(&updated_recipient).await {
    tracing::error!("Worker: failed to update recipient {}: {:?}", next.id, e);
}
```

**Existing `audited_create!` invocation in service-layer** (`genossi_service_impl/src/member_document.rs:120-142`):
```rust
let now = time::OffsetDateTime::now_utc();
let new_doc = MemberDocument {
    id: doc_id,
    member_id: upload.member_id,
    document_type: upload.document_type,
    description: upload.description.map(|d| Arc::from(d.as_str())),
    file_name: Arc::from(upload.file_name.as_str()),
    mime_type: Arc::from(upload.mime_type.as_str()),
    relative_path: Arc::from(relative_path.as_str()),
    created: time::PrimitiveDateTime::new(now.date(), now.time()),
    deleted: None,
    version: self.uuid_service.new_v4().await,
};

let doc_entity: genossi_dao::member_document::MemberDocumentEntity = (&new_doc).into();
crate::audited_create!(
    self,
    self.member_document_dao,
    &doc_entity,
    PROCESS,
    &user_id,
    tx
);
```

**Existing `audited_create!` macro contract** (`genossi_service_impl/src/audit_macros.rs:5-36`):
```rust
#[macro_export]
macro_rules! audited_create {
    ($self:expr, $dao:expr, $entity:expr, $process:expr, $user_id:expr, $tx:expr) => {{
        use genossi_dao::audit_log::AuditLogDao;

        $dao.create($entity, $process, $tx.clone()).await?;

        let prev_hash = $self
            .audit_log_dao
            .get_latest_hash($tx.clone())
            .await?
            .unwrap_or_default();

        let entries = $crate::audit_log::build_create_entries(
            $entity,
            $user_id,
            $process,
            &prev_hash,
            &mut || uuid::Uuid::new_v4(),
        );

        if !entries.is_empty() {
            $self
                .audit_log_dao
                .create_entries(&entries, $tx.clone())
                .await?;
        }
    }};
}
```

**Specifics for Phase 10 — NEW PATTERN (cross-crate `audited_create!`):**

**Critical Finding:** `genossi_mail` currently does NOT depend on `genossi_service_impl` (verified in `genossi_mail/Cargo.toml`). The macro uses `$crate::audit_log::build_create_entries` — `$crate` resolves to the **defining crate** (`genossi_service_impl`) inside the macro, so cross-crate invocation IS feasible.

**Required steps for Planner:**
1. Add `genossi_service_impl = { path = "../genossi_service_impl" }` to `genossi_mail/Cargo.toml`
2. Verify no circular dep: `genossi_service_impl` does NOT depend on `genossi_mail` (check — it must not, for this to compile)
3. The macro expects `$self` to have fields `audit_log_dao` and `uuid_service`. The worker is currently a **free function** (`start_mail_worker`), not a struct → wrap the dependencies in a local struct like `MailWorkerContext { audit_log_dao, uuid_service, member_document_dao, mail_template_dao, repayment_entry_dao, repayment_phase_dao, transaction_dao }` and call `audited_create!(worker_ctx, worker_ctx.member_document_dao, &doc_entity, REPAYMENT_MAIL_PROCESS, user_id, tx)`.
4. Macro expects `$tx.clone()` — must be the genossi_dao Transaction trait type (matches `member_document_dao` trait's `type Transaction`). Worker today does not hold a transaction; Planner must add `TransactionDao::transaction()` acquire + `commit()` cycle BEFORE/AFTER the recipient+document writes (Planner-Discretion D-disc-6 — bundle recipient-update and MemberDocument-create in one tx)
5. `process` string: `const REPAYMENT_MAIL_PROCESS: &str = "repayment-mail-worker";` (D-11)
6. `user_id`: Worker has no auth context → use `"SYSTEM"` (matches `genossi_service_impl/src/member_document.rs:212` fallback `unwrap_or_else(|| "SYSTEM".to_string())`)

**Recommended Worker pseudo-code** (insert AFTER send_result line 282, BEFORE final job-status-completion line 318):
```rust
// Phase 10 D-10: persist MemberDocument as Final-State audit anchor (only if recipient has member_id)
if let Some(member_id) = next.member_id {
    let now = time::OffsetDateTime::now_utc();
    let (doc_status, doc_description) = match &send_result {
        Ok(_) => ("sent", job.subject.to_string()),
        Err(e) => {
            let err_truncated: String = format!("{:?}", e).chars().take(200).collect();
            ("failed", format!("{} [FAILED: {}]", job.subject, err_truncated))
        }
    };
    let doc_entity = MemberDocumentEntity {
        id: uuid::Uuid::new_v4(),
        member_id,
        document_type: Arc::from("repayment_mail"),
        description: Some(Arc::from(doc_description.as_str())),
        file_name: Arc::from(""),  // no file — symbolic anchor (specifics §relative_path)
        mime_type: Arc::from("text/plain"),
        relative_path: Arc::from(""),  // Planner-Discretion: "" or "mail/{recipient_id}"
        created: time::PrimitiveDateTime::new(now.date(), now.time()),
        deleted: None,
        version: uuid::Uuid::new_v4(),
        template_id: job.template_id,           // NEW Phase 10 field
        mail_recipient_id: Some(next.id),       // NEW Phase 10 field
        status: Some(Arc::from(doc_status)),    // NEW Phase 10 field
    };
    let tx = transaction_dao.transaction().await?;  // open tx for audit
    genossi_service_impl::audited_create!(
        worker_ctx,
        worker_ctx.member_document_dao,
        &doc_entity,
        REPAYMENT_MAIL_PROCESS,
        "SYSTEM",
        tx
    );
    transaction_dao.commit(tx).await?;
}
```

**Repayment-Context-Merge** (insert BEFORE render_template, after `member_to_template_context` line 184):
```rust
let mut ctx = member_to_template_context(&member);
if let Some(phase_id) = job.repayment_phase_id {
    // Resolve phase + entries (Planner-Discretion: use RepaymentPhaseDao + RepaymentEntryDao
    // directly, or introduce RepaymentMailContextResolver-trait — see specifics §worker-deps)
    if let Some(phase) = repayment_phase_dao.find_by_id(phase_id, tx).await? {
        let entries = repayment_entry_dao.find_by_phase_id(phase_id, tx).await?
            .iter()
            .filter(|e| e.member_id == member_id
                && (e.status == RepaymentEntryStatus::Open
                    || e.status == RepaymentEntryStatus::Contacted))
            .cloned()
            .collect::<Vec<_>>();
        if !entries.is_empty() {
            let share_count: i32 = entries.iter().map(|e| e.share_count_to_pay_out).sum();
            let cents: i64 = share_count as i64 * phase.share_value as i64;
            // German locale "X,YZ" (Planner-Discretion §payout_amount-Format)
            let payout_amount = format!("{},{:02}", cents / 100, cents % 100);
            ctx = merge_repayment_context(ctx, &payout_amount, share_count, phase.fiscal_year);
        }
        // D-05: if no entries, ctx stays without the 3 vars; strict-env will fail
        // render if template references them without `{% if %}` → mark_recipient_failed
    }
}
```

---

### `genossi_mail/Cargo.toml` — Add `genossi_service_impl` Dep

**Role:** config · **Match:** exact

**Existing deps** (verified):
```toml
genossi_config = { path = "../genossi_config" }
genossi_dao = { path = "../genossi_dao" }
genossi_service = { path = "../genossi_service", features = ["utoipa"] }
```

**Add:**
```toml
genossi_service_impl = { path = "../genossi_service_impl" }
```

**Risk check (NEW pattern):** Confirm `genossi_service_impl/Cargo.toml` does NOT depend on `genossi_mail` before adding. Spot check: `genossi_service_impl` typically depends on `genossi_dao` + `genossi_service` only. If a circular dep is found, fallback is to copy the audit macros' inlined body into `genossi_mail/src/worker_audit.rs` and use the macro pattern directly (no macro re-export needed; the build_create_entries function is `pub` from `genossi_service_impl::audit_log` which the Planner can call directly without the macro).

---

### `genossi_bin/src/lib.rs` — `start_mail_worker` Wiring

**Role:** binary (DI) · **Match:** exact

**Existing wiring** (lines 1141-1163):
```rust
pub fn start_mail_worker(&self) {
    let config_service = self.worker_config_service.clone();
    let job_dao = self.worker_job_dao.clone();
    let recipient_dao = self.worker_recipient_dao.clone();
    let attachment_dao = self.worker_attachment_dao.clone();
    let static_attachment_dao = self.worker_static_attachment_dao.clone();
    let document_storage = self.document_storage.clone();
    let member_resolver = Arc::new(PoolMemberResolver::new(self.pool.clone()));
    let inbound_mail_dao = Arc::new(InboundMailDaoType::new(self.pool.clone()));
    tokio::spawn(async move {
        genossi_mail::worker::start_mail_worker(
            config_service,
            job_dao,
            recipient_dao,
            attachment_dao,
            static_attachment_dao,
            document_storage,
            member_resolver,
            inbound_mail_dao,
        )
        .await;
    });
}
```

**Specifics for Phase 10:**
- Append 5 new dependencies to `start_mail_worker` call:
  - `self.member_document_dao.clone()` (already in struct line 589)
  - `self.audit_log_dao.clone()` (already in struct line 567)
  - `self.mail_template_dao.clone()` (verify exists in struct — should, given `MailTemplateService` is wired)
  - `repayment_entry_dao.clone()` + `repayment_phase_dao.clone()` (Phase 7/8 wiring — verify in struct)
  - `transaction_dao.clone()` (verify type matches `MemberDocumentDao::Transaction`)
- All DAOs already exist in `RestStateImpl` — wiring is purely additive
- No new service instantiation needed (Planner-Discretion: 4 DAO-Deps direct, no Resolver-Service wrapper)

---

## Shared Patterns

### Cross-Crate `audited_create!` (NEW for Phase 10)

**Source:** `genossi_service_impl/src/audit_macros.rs:1-36` (definition)
**Used by:** Worker (`genossi_mail/src/worker.rs`)
**Risk:** Pattern is NEW — first cross-crate usage of `audited_*!` from outside `genossi_service_impl`

**Macro contract:**
- `#[macro_export]` exposes globally
- `$crate::audit_log::build_create_entries` resolves to defining crate (`genossi_service_impl::audit_log`) — `pub mod audit_log` confirmed in `genossi_service_impl/src/lib.rs:5`
- `use genossi_dao::audit_log::AuditLogDao;` inside macro hoists the trait import — caller need NOT pre-import
- Requires `$self.audit_log_dao` to be callable: `.get_latest_hash(tx).await?` and `.create_entries(&entries, tx).await?`
- Requires `$dao.create($entity, $process, $tx.clone())` to compile — `$entity` must impl `Auditable` (verified for `MemberDocumentEntity`)

**Planner verification step:** After Cargo.toml change, run `cargo build -p genossi_mail` to confirm no circular dep. Fallback: inline-call `build_create_entries` directly (it is `pub` — line 115 of `audit_log.rs`).

---

### Soft-Delete + Backward-Compat Migrations

**Source:** `migrations/sqlite/20260506000000_add_code_to_helper_token.sql`
**Apply to:** Both Phase-10 migrations
**Pattern:**
- ALTER TABLE ADD COLUMN with NULL default
- ADR-style comment header explaining intent + rationale
- FK clauses are documentary (project does NOT enable `PRAGMA foreign_keys=ON` — see `repayment_entry`-migration line 4-7)
- Forward-only (SQLite < 3.35 no DROP COLUMN)

---

### FROZEN-Order Auditable Extension

**Source:** `genossi_dao/src/repayment_entry.rs:70-92` (FROZEN comment + test)
**Apply to:** `genossi_dao/src/member_document.rs::audit_fields()`
**Pattern:**
- Existing fields stay at their existing indices (hash-chain stability)
- New fields **appended at end** (indices 6, 7, 8 for Phase 10)
- FROZEN-comment block above the function
- Update FROZEN-count test: `assert_eq!(fields.len(), 9)`

---

### Strict-Env `{% if %}` Pattern for Optional Variables

**Source:** `genossi_mail/src/template.rs:188-205` (`test_null_field_conditional`)
**Apply to:** All Phase 10 template tests; documentation of expected user-facing template syntax

```rust
let template = "{% if payout_amount %}Auszahlung: {{ payout_amount }} €{% endif %}";
```

D-05 explicitly relies on `minijinja::UndefinedBehavior::Strict` (template.rs:53-57) so that templates without `{% if %}`-guards FAIL render (→ `mark_recipient_failed`) when a member has no entries. Test pattern mirrors `test_null_field_conditional` + `test_present_optional_field`.

---

### Cross-Crate `pub mod audit_log`

**Source:** `genossi_service_impl/src/lib.rs:5` (`pub mod audit_log;`)
**Implication:** `build_create_entries`, `build_update_entries`, `build_delete_entries` (in `genossi_service_impl/src/audit_log.rs:115-`) are reachable from any downstream crate that depends on `genossi_service_impl`. If the macro pattern is too coupled, planner can call these functions directly without the macro.

---

## No Analog Found

**None.** Every file has an existing analog in the codebase. The one NEW pattern is **cross-crate `audited_create!`-Aufruf vom Worker** — this is structurally identical to the service-layer pattern but Cross-Crate boundary is new. The planner must verify with `cargo build -p genossi_mail` after adding `genossi_service_impl` as dep.

---

## Metadata

**Analog search scope:**
- `genossi_mail/` (rest, service, worker, template, dao, dao_sqlite)
- `genossi_dao/` (member_document, repayment_entry, auditable)
- `genossi_dao_impl_sqlite/` (member_document)
- `genossi_service/` (member_document)
- `genossi_service_impl/` (member_document, audit_macros, audit_log)
- `genossi_bin/src/lib.rs` (wiring, start_mail_worker)
- `migrations/sqlite/` (mail_jobs, member_document, repayment_*, helper_token-add-code)

**Files scanned:** 14
**Pattern extraction date:** 2026-05-31
