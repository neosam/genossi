---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-04
slug: backfill-worker
type: execute
wave: 4
depends_on: [19-02, 19-03]
files_modified:
  - genossi_mail/src/inbox.rs
  - genossi_bin/src/lib.rs
  - genossi_bin/src/main.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "Free fn `run_attachment_backfill` iterates `InboundMail` rows where `has_attachments=true` AND `count_for_mail==0` (D-05)"
    - "For each candidate: calls `imap.fetch_one_by_uid(uid_validity, imap_uid)`; on Err or Ok(None) → tracing::warn + continue (D-06, T-06)"
    - "Successful refetch: `parse_raw_mail` → `persist_attachment` loop with same 10-MB cap as poll worker (D-02 reuse from Plan 19-02)"
    - "Worker logs `inbox_attachment_backfill: starting (N candidates)` at start and `done (Y persisted, Z skipped)` at end"
    - "Backfill is one-shot tokio::spawn at server start — not a loop; relies on count_for_mail==0 filter for idempotency on restart"
    - "Spawn pattern mirrors existing `start_inbox_worker` — same wiring on RestStateImpl"
  artifacts:
    - path: "genossi_mail/src/inbox.rs"
      provides: "run_attachment_backfill free fn"
      contains: "pub async fn run_attachment_backfill"
    - path: "genossi_bin/src/lib.rs"
      provides: "start_attachment_backfill_worker method on RestStateImpl"
      contains: "fn start_attachment_backfill_worker"
    - path: "genossi_bin/src/main.rs"
      provides: "Spawn call after start_inbox_worker"
      contains: "start_attachment_backfill_worker"
  key_links:
    - from: "main.rs"
      to: "RestStateImpl::start_attachment_backfill_worker"
      via: "method call after start_inbox_worker"
      pattern: "start_attachment_backfill_worker"
    - from: "run_attachment_backfill"
      to: "persist_attachment (re-uses helper from Plan 19-02)"
      via: "inner loop per candidate mail's parsed.attachments"
      pattern: "persist_attachment\\("

---

<objective>
Lege einen einmaligen Backfill-Worker an, der Bestandsmails (vor Phase 19
empfangen) nachträglich um Attachments anreichert — via IMAP-Refetch +
existierende Persist-Pipeline.

Purpose: Ohne Backfill würden alle Bestandsmails dauerhaft den "Anhang vor
Phase 19 empfangen"-Hinweis (Plan 19-05/06) zeigen. Diese Phase ist
best-effort (D-05/D-06) — Mails, die im IMAP nicht mehr existieren oder bei
UIDVALIDITY-Drift unauffindbar werden, bleiben legacy.

Output: 1 neue free fn in `inbox.rs`, 1 neue Methode auf `RestStateImpl`,
1 Spawn-Aufruf in `main.rs`, 1 Unit-Test, der die Skip-Logik bei IMAP-Fehler
verifiziert.
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
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-01-SUMMARY.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-02-SUMMARY.md
@CLAUDE.md

<interfaces>
<!-- Pre-extracted analog APIs — executor mirrors verbatim. -->

From `genossi_bin/src/lib.rs:1344-1351` (existing spawn pattern — mirror exactly):
```rust
pub fn start_inbox_worker(&self) {
    let config_service = self.worker_inbox_config_service.clone();
    let dao = self.worker_inbox_dao.clone();
    let imap_client = self.worker_inbox_imap_client.clone();
    tokio::spawn(async move {
        genossi_mail::inbox::start_inbox_worker(config_service, dao, imap_client).await;
    });
}
```

From `genossi_bin/src/main.rs` (find `rest_state.start_inbox_worker();`) — insert spawn-call right after.

From Plan 19-02 (already implemented):
- `InboxImapClient::fetch_one_by_uid(&self, config, expected_uid_validity, uid) -> Result<Option<FetchedMessage>, MailServiceError>`
- `persist_attachment(storage, dao, mail_id, file_name, mime, bytes) -> Result<InboundMailAttachment, MailServiceError>`
- `parse_raw_mail(raw: &[u8]) -> Result<ParsedMail, MailServiceError>` (already exists; Plan 19-02 just added attachments field)
- `InboundMailAttachmentDao::count_for_mail(mail_id) -> Result<i64, MailDaoError>` (from Plan 19-01)

`InboundMailDao` (already exists in `genossi_mail/src/dao.rs`):
- Has methods to fetch all `InboundMail` rows. Backfill uses `dump_all` (or its equivalent) and filters `has_attachments == true` in-memory. NO new DAO method is added (small dataset, one-shot at startup — see Step 1 LOCKED rationale).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: run_attachment_backfill free fn + skip-on-error unit test</name>
  <files>genossi_mail/src/inbox.rs</files>
  <read_first>
    - genossi_mail/src/inbox.rs (search `pub async fn start_inbox_worker` for spawn-pattern; also find what loads imap config — likely `load_imap_config` or similar inline)
    - genossi_mail/src/dao.rs (verify `InboundMailDao::dump_all` signature — backfill iterates this and filters by `has_attachments` in-memory; NO new DAO method is added)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Pattern 6 (lines 519-541) + §Pitfall 1 (UIDVALIDITY drift)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §4 (last paragraph on `run_attachment_backfill`)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md D-05, D-06
  </read_first>
  <behavior>
    - Free fn signature (mirror generic bounds from `start_inbox_worker`):
      ```rust
      pub async fn run_attachment_backfill<C, D, A, St, I>(
          config_service: Arc<C>,
          mail_dao: Arc<D>,
          attachment_dao: Arc<A>,
          storage: Arc<St>,
          imap_client: Arc<I>,
      )
      where C: ConfigService + Send + Sync + 'static,
            D: InboundMailDao + Send + Sync + 'static,
            A: InboundMailAttachmentDao + Send + Sync + 'static,
            St: DocumentStorage + Send + Sync + 'static,
            I: InboxImapClient + Send + Sync + 'static
      ```
    - Reads IMAP config exactly like `start_inbox_worker` does (mirror the existing call pattern)
    - Iterates candidates: queries mail-dao for all rows where `has_attachments == true`, then filters via `attachment_dao.count_for_mail(mail.id).await? == 0`
    - Logs at start: `tracing::info!("inbox_attachment_backfill: starting ({} candidates)", candidates.len())`
    - Per candidate: `imap_client.fetch_one_by_uid(&imap_cfg, mail.uid_validity, mail.imap_uid)`. On Err OR Ok(None) → `tracing::warn!` + continue + `skipped += 1`
    - On Ok(Some(fetched)): `parse_raw_mail(&fetched.raw)` → loop attachments → `persist_attachment(...)` with `tracing::warn!` on Err + continue
    - Aggregate `persisted: u64` (mails where ≥1 attachment landed) + `skipped: u64`
    - Logs at end: `tracing::info!("inbox_attachment_backfill: done ({} persisted, {} skipped)", persisted, skipped)`
    - One-shot — no `loop {}` body
    - Idempotency relies on the `count_for_mail == 0` filter: re-running on restart skips mails already backfilled
    - Unit test verifies that when `MockInboxImapClient` returns Err / Ok(None), `attachment_dao.create` and `storage.save` are NEVER called for those candidates
  </behavior>
  <action>
    **Step 1 — Discover DAO API.** Run:
    ```bash
    grep -n "pub trait InboundMailDao\|async fn " genossi_mail/src/dao.rs | head -30
    ```
    Identify the method that returns all `InboundMail` rows (e.g. `dump_all` or equivalent).

    **LOCKED — use `dump_all` + in-memory filter.** Reasoning: Backfill ist ein einmaliger Lauf bei Server-Start, das Mengengerüst ist klein (< paar tausend Mails), und ein `has_attachments=true` Filter in Rust ist trivial. Eine neue DAO-Methode `find_with_attachments_flag` würde Trait + SQLite-Impl + Mock-Auto-Extension + Test-Surface erweitern, ohne messbaren Nutzen. Daher KEINE neue DAO-Methode — die Backfill-fn iteriert `mail_dao.dump_all().await?` und filtert per `.iter().filter(|m| m.has_attachments)`. Daher auch keine Edits an `genossi_mail/src/dao.rs` oder `genossi_mail/src/dao_sqlite.rs` (Frontmatter `files_modified` listet sie konsequenterweise NICHT).

    **Step 2 — Implement `run_attachment_backfill`** in `genossi_mail/src/inbox.rs`, placed AFTER `start_inbox_worker`. Use this template (adjust `load_imap_config` to whatever the existing inbox worker uses — read it first):

    ```rust
    pub async fn run_attachment_backfill<C, D, A, St, I>(
        config_service: Arc<C>,
        mail_dao: Arc<D>,
        attachment_dao: Arc<A>,
        storage: Arc<St>,
        imap_client: Arc<I>,
    )
    where
        C: ConfigService + Send + Sync + 'static,
        D: InboundMailDao + Send + Sync + 'static,
        A: InboundMailAttachmentDao + Send + Sync + 'static,
        St: DocumentStorage + Send + Sync + 'static,
        I: InboxImapClient + Send + Sync + 'static,
    {
        // 1. Load IMAP config (mirror start_inbox_worker config-loading code path; on Err → return early)
        let imap_cfg = match /* whatever start_inbox_worker uses */ {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!("inbox_attachment_backfill: config load failed: {:?}", e);
                return;
            }
        };

        // 2. Gather candidates (mails with has_attachments=true AND no existing attachment rows)
        let all = match mail_dao.dump_all().await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("inbox_attachment_backfill: mail dao query failed: {:?}", e);
                return;
            }
        };

        let mut candidates: Vec<crate::dao::InboundMail> = Vec::new();
        for mail in all.iter().filter(|m| m.has_attachments) {
            match attachment_dao.count_for_mail(mail.id).await {
                Ok(0) => candidates.push(mail.clone()),
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("inbox_attachment_backfill: count_for_mail({}) failed: {:?}", mail.id, e);
                }
            }
        }

        tracing::info!("inbox_attachment_backfill: starting ({} candidates)", candidates.len());

        let mut persisted: u64 = 0;
        let mut skipped: u64 = 0;
        for mail in candidates.iter() {
            let fetched = match imap_client.fetch_one_by_uid(&imap_cfg, mail.uid_validity, mail.imap_uid).await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    tracing::warn!(
                        "inbox_attachment_backfill: skip mail={} uid={} (validity={}): no message",
                        mail.id, mail.imap_uid, mail.uid_validity
                    );
                    skipped += 1;
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        "inbox_attachment_backfill: skip mail={} uid={} (validity={}): {:?}",
                        mail.id, mail.imap_uid, mail.uid_validity, e
                    );
                    skipped += 1;
                    continue;
                }
            };
            let parsed = match parse_raw_mail(&fetched.raw) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("inbox_attachment_backfill: parse failed mail={}: {:?}", mail.id, e);
                    skipped += 1;
                    continue;
                }
            };
            let mut any_ok = false;
            for att in parsed.attachments.iter() {
                match persist_attachment(
                    storage.as_ref(),
                    attachment_dao.as_ref(),
                    mail.id,
                    &att.file_name,
                    &att.mime_type,
                    &att.bytes,
                ).await {
                    Ok(_) => { any_ok = true; }
                    Err(e) => {
                        tracing::warn!(
                            "inbox_attachment_backfill: persist failed mail={} file={}: {:?}",
                            mail.id, att.file_name, e
                        );
                    }
                }
            }
            if any_ok { persisted += 1; } else { skipped += 1; }
        }

        tracing::info!("inbox_attachment_backfill: done ({} persisted, {} skipped)", persisted, skipped);
    }
    ```

    Adjust the `load IMAP config` line to whatever existing `start_inbox_worker` calls (could be `config_service.imap_config()` or inline `ImapConfig::from_config_service(...)` — read and mirror). Field-access on `crate::dao::InboundMail` must use actual field names from `dao.rs:222-242` — verify before writing.

    **Step 3 — Add unit test** `test_run_attachment_backfill_silent_skips_imap_error` in `inbox.rs::tests`:
    - Use `MockConfigService`, `MockInboundMailDao` (auto-mock from existing `#[automock]`), `MockInboundMailAttachmentDao`, `MockDocumentStorage`, `MockInboxImapClient`
    - Stub config loading to succeed (mock whatever method `start_inbox_worker` calls)
    - Stub mail dao to return 2 mails with `has_attachments=true`
    - Stub `count_for_mail(_)` → `Ok(0)` for both
    - Stub `expect_fetch_one_by_uid` to be invoked exactly 2 times: first returns `Err(...)`, second returns `Ok(None)`
    - `attachment_dao.expect_create().times(0);` — assertion: persist must NOT happen
    - `storage.expect_save().times(0);` — assertion: save must NOT happen
    - Run `run_attachment_backfill(...).await` to completion
    - mockall verifies all `times(...)` expectations on drop — if `expect_create().times(0)` is violated the test fails

    Test C (optional, if simple to add) — `test_run_attachment_backfill_skips_already_backfilled`:
    - 1 mail with `has_attachments=true`
    - `count_for_mail(...)` → `Ok(2)` (already 2 attachments persisted)
    - `imap_client.expect_fetch_one_by_uid().times(0);` — assertion: must NOT refetch
    - Run completes silently
  </action>
  <verify>
    <automated>cargo test -p genossi_mail inbox::tests::test_run_attachment_backfill_silent_skips_imap_error -- --nocapture 2>&amp;1 | tee /tmp/19-04-task1.log; grep -q "test result: ok" /tmp/19-04-task1.log &amp;&amp; cargo check -p genossi_mail 2>&amp;1 | tee /tmp/19-04-check.log; ! grep -q "^error" /tmp/19-04-check.log</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "pub async fn run_attachment_backfill" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "inbox_attachment_backfill: starting" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "inbox_attachment_backfill: done" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "fetch_one_by_uid" genossi_mail/src/inbox.rs` returns ≥ 2 (trait method + call in backfill)
    - `grep -c "persist_attachment" genossi_mail/src/inbox.rs` returns ≥ 3 (fn def + poll-worker call + backfill call)
    - `grep -c "test_run_attachment_backfill_silent_skips_imap_error" genossi_mail/src/inbox.rs` returns ≥ 2
    - `cargo test -p genossi_mail` exits 0
    - `cargo check -p genossi_mail` exits 0
  </acceptance_criteria>
  <done>
    Backfill-fn lives in `inbox.rs`, silent-skip behaviour test-verified, count_for_mail-Filter macht Restart-Idempotenz, kein Loop, Logging an Start + Ende.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: RestStateImpl::start_attachment_backfill_worker + main.rs spawn</name>
  <files>genossi_bin/src/lib.rs, genossi_bin/src/main.rs</files>
  <read_first>
    - genossi_bin/src/lib.rs:1344-1351 (existing `start_inbox_worker` spawn pattern — mirror precisely)
    - genossi_bin/src/lib.rs:660-680 (RestStateImpl field block — verify `inbound_attachment_dao` field already added by Plan 19-03; `document_storage` field exists at :614 per PATTERNS.md §8)
    - genossi_bin/src/lib.rs (search for `worker_inbox_config_service`, `worker_inbox_dao`, `worker_inbox_imap_client` fields — these are the existing worker dependencies to clone)
    - genossi_bin/src/main.rs:30-65 (find the line `rest_state.start_inbox_worker();`)
  </read_first>
  <behavior>
    - New method `pub fn start_attachment_backfill_worker(&self)` on `RestStateImpl`
    - Mirrors `start_inbox_worker` spawn pattern: clone the 5 Arc dependencies, `tokio::spawn`, call `genossi_mail::inbox::run_attachment_backfill`
    - `main.rs` calls `rest_state.start_attachment_backfill_worker();` after `start_inbox_worker();` + emits `tracing::info!("Attachment backfill worker spawned")`
    - Spawn happens AFTER `sqlx::migrate!()` ran (which is automatic — migrate runs synchronously before `RestStateImpl::new`)
  </behavior>
  <action>
    **Step 1 — Add method on `RestStateImpl`** in `genossi_bin/src/lib.rs` immediately after the existing `start_inbox_worker` method (around `:1344-1351`):
    ```rust
    pub fn start_attachment_backfill_worker(&self) {
        let config_service = self.worker_inbox_config_service.clone();
        let mail_dao = self.worker_inbox_dao.clone();
        let attachment_dao = self.inbound_attachment_dao.clone();
        let storage = self.document_storage.clone();
        let imap_client = self.worker_inbox_imap_client.clone();
        tokio::spawn(async move {
            genossi_mail::inbox::run_attachment_backfill(
                config_service,
                mail_dao,
                attachment_dao,
                storage,
                imap_client,
            ).await;
        });
    }
    ```

    Note: field names (`worker_inbox_config_service`, `worker_inbox_dao`, etc.) must match what already exists in `RestStateImpl` — verify by reading the field block first. `document_storage` field name may differ — use whatever the existing field is called (per PATTERNS.md §8 it's already named `document_storage`; verify).

    **Step 2 — Spawn from main.rs**: open `genossi_bin/src/main.rs`, find the exact line containing `rest_state.start_inbox_worker();`. Immediately after it add:
    ```rust
    rest_state.start_attachment_backfill_worker();
    tracing::info!("Attachment backfill worker spawned");
    ```
  </action>
  <verify>
    <automated>cargo check -p genossi_bin 2>&amp;1 | tee /tmp/19-04-task2-check.log; ! grep -q "^error" /tmp/19-04-task2-check.log &amp;&amp; grep -c "start_attachment_backfill_worker" genossi_bin/src/lib.rs &amp;&amp; grep -c "start_attachment_backfill_worker" genossi_bin/src/main.rs</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "pub fn start_attachment_backfill_worker" genossi_bin/src/lib.rs` returns 1
    - `grep -c "genossi_mail::inbox::run_attachment_backfill" genossi_bin/src/lib.rs` returns 1
    - `grep -c "rest_state.start_attachment_backfill_worker" genossi_bin/src/main.rs` returns 1
    - `grep -c "Attachment backfill worker spawned" genossi_bin/src/main.rs` returns 1
    - The new spawn line in main.rs appears AFTER `rest_state.start_inbox_worker()` (line ordering): verify via `awk '/start_inbox_worker\(\)/{a=NR} /start_attachment_backfill_worker\(\)/{b=NR} END{exit (a && b && b>a) ? 0 : 1}' genossi_bin/src/main.rs`
    - `cargo check -p genossi_bin` exits 0
  </acceptance_criteria>
  <done>
    Spawn-Methode existiert, main.rs ruft sie auf, Compile grün — Backfill startet beim nächsten Server-Boot automatisch einmal.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| IMAP server → backfill | Same boundary as poll worker; UIDVALIDITY drift is the realistic threat |
| DB state at restart | Backfill is idempotent via `count_for_mail == 0` filter |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-06 | Tampering (UID drift fetches wrong mail) | run_attachment_backfill → fetch_one_by_uid | mitigate | `fetch_one_by_uid` (Plan 19-02) re-checks UIDVALIDITY; mismatch → Err → backfill `warn` + skip per D-06. Test `test_run_attachment_backfill_silent_skips_imap_error` verifies that Err / Ok(None) responses do NOT cause `persist_attachment` to run. |

(T-01, T-02, T-03, T-04, T-05, T-07, T-08 are owned by other plans. Backfill re-uses `persist_attachment` so T-01 + T-07 mitigations from Plan 19-02 apply transitively.)
</threat_model>

<verification>
- `cargo check -p genossi_mail` exits 0
- `cargo check -p genossi_bin` exits 0
- `cargo test -p genossi_mail` exits 0 (skip-on-error test passes; existing tests stay green)
- main.rs spawn-call line appears AFTER `start_inbox_worker` (ordering verified by awk gate)
- Backfill is called as one-shot `tokio::spawn` — no `loop {}` in `run_attachment_backfill` (grep `loop \{` should return 0 inside the function body)
</verification>

<success_criteria>
- run_attachment_backfill exists and is callable from genossi_bin
- Silent-skip behaviour verified by unit test
- Idempotency on restart via count_for_mail==0 filter (no state table needed per D-06)
- main.rs spawns the worker once after inbox worker
- Tracing logs at start + end emit candidate count + persisted/skipped tally
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-04-SUMMARY.md`
</output>
