---
phase: 23-html-mail-backend
plan: 01
subsystem: database
tags: [rust, sqlite, sqlx, dao, migration, schema, html-mail]

requires:
  - phase: 22-8bit-shared-mail-body-helper
    provides: shared build_message helper that will gain multipart/alternative wiring in later plans of Phase 23
provides:
  - Three forward-only ADD COLUMN … TEXT NULL migrations for body_html / rendered_html_body
  - MailJob.body_html, MailTemplate.body_html, MailRecipient.rendered_html_body DAO fields
  - Wired INSERT / SELECT / UPDATE statements + NULL-legacy roundtrip tests
affects: [23-02-render, 23-03-mime-build, 23-04-worker-persist, 24-wysiwyg-editor]

tech-stack:
  added: []
  patterns:
    - Forward-only ADD COLUMN … TEXT NULL migration (Phase-22 vorbild)
    - Optional string DAO field as Option<Arc<str>> (mirroring rendered_body)

key-files:
  created:
    - migrations/sqlite/20260702000000_mail_templates_add_body_html.sql
    - migrations/sqlite/20260702000001_mail_jobs_add_body_html.sql
    - migrations/sqlite/20260702000002_mail_recipients_add_rendered_html_body.sql
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs
    - genossi_mail/src/inbox.rs
    - genossi_mail/src/mail_template_service.rs
    - genossi_mail/src/service.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/render.rs
    - genossi_mail/src/backfill.rs
    - genossi_mail/src/rest.rs

key-decisions:
  - "NULL-legacy semantics upheld — legacy rows read back body_html=None (never Some(\"\")), verified by dedicated roundtrip test"
  - "body_html on both mail_templates (D-06) and mail_jobs (D-07) so ad-hoc jobs can carry HTML independently of a template"
  - "rendered_html_body persisted per-recipient (D-08) to preserve the byte-identical bytes actually sent"
  - "mail_template_service.update preserves prior body_html on text-only update (until service+REST wire lands in later plans)"

patterns-established:
  - "Optional HTML-body field on DAO structs: Option<Arc<str>> next to the sibling text field"
  - "UPDATE bind order mirrors SET-clause order — rendered_html_body binds between rendered_body and rendered_reconstructed"

requirements-completed: [HTML-03]

coverage:
  - id: D1
    description: "Three forward-only ADD COLUMN migrations (mail_templates.body_html, mail_jobs.body_html, mail_recipients.rendered_html_body) apply cleanly on a fresh SQLite DB and leave existing rows as NULL"
    requirement: HTML-03
    verification:
      - kind: manual_procedural
        ref: "sqlx migrate run --database-url sqlite:target/23-migration-test.db --source migrations/sqlite (executed during Task 1; verified via sqlite3 .schema)"
        status: pass
    human_judgment: false
  - id: D2
    description: "DAO structs (MailJob, MailTemplate, MailRecipient) carry the new optional HTML fields via Option<Arc<str>>, and INSERT/SELECT/UPDATE code paths persist them without changing legacy text-only behavior"
    requirement: HTML-03
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#mail_job_body_html_roundtrip"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#mail_job_body_html_null_roundtrip"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#mail_recipient_update_persists_rendered_html_body"
        status: pass
    human_judgment: false

duration: 42min
completed: 2026-07-02
status: complete
---

# Phase 23 Plan 01: HTML Mail Backend — Schema Foundation Summary

**Three forward-only ADD COLUMN … TEXT NULL migrations plus DAO wiring add body_html (mail_templates + mail_jobs) and rendered_html_body (mail_recipients) with byte-identical NULL-legacy roundtrip guarantees**

## Performance

- **Duration:** ~42 min
- **Started:** 2026-07-02T19:56:56Z
- **Completed:** 2026-07-02T20:38:58Z
- **Tasks:** 3
- **Files modified:** 9 (+ 3 new migrations)

## Accomplishments

- Three forward-only migrations added and verified against a scratch SQLite DB (`.schema` confirms all three columns present).
- `MailJob.body_html`, `MailTemplate.body_html`, `MailRecipient.rendered_html_body` fields wired into DAO structs, INSERT / SELECT / UPDATE column lists, in-file test scaffolds, and every construction site across `genossi_mail`.
- Three new roundtrip tests prove the persistence contract:
  - `mail_job_body_html_roundtrip` — `Some("<b>Hallo</b>")` survives byte-identical.
  - `mail_job_body_html_null_roundtrip` — legacy NULL reads back as `None`, never `Some("")` (closes RESEARCH Pitfall 4 for `body_html`).
  - `mail_recipient_update_persists_rendered_html_body` — worker UPDATE path persists `<p>Rendered</p>` alongside `rendered_body` / `rendered_subject`.
- All **227** lib tests in `genossi_mail` pass; `cargo build` for the full workspace succeeds.

## Task Commits

Each task was committed atomically via `jj commit`:

1. **Task 1: Add forward-only migrations for body_html / rendered_html_body** — `f0a4d284` (feat)
2. **Task 2: Add body_html / rendered_html_body fields to DAO structs** — `2f9c3dd8` (feat)
3. **Task 3: Wire body_html / rendered_html_body into dao_sqlite.rs + roundtrip tests** — `cdcbbf73` (feat)

## Files Created/Modified

### Created (migrations)
- `migrations/sqlite/20260702000000_mail_templates_add_body_html.sql` — D-06
- `migrations/sqlite/20260702000001_mail_jobs_add_body_html.sql` — D-07
- `migrations/sqlite/20260702000002_mail_recipients_add_rendered_html_body.sql` — D-08

### Modified (DAO + downstream construction sites)
- `genossi_mail/src/dao.rs` — added the three new `Option<Arc<str>>` fields, positioned next to their sibling text field
- `genossi_mail/src/dao_sqlite.rs` — extended `MailJobDb`/`MailTemplateDb`/`MailRecipientDb`, all `TryFrom` impls, all INSERT/SELECT/UPDATE queries, in-file `CREATE TABLE` test scaffolds, sample-factory defaults, and 3 new roundtrip tests
- `genossi_mail/src/inbox.rs` — MailJob/MailRecipient literal in inbox reply path
- `genossi_mail/src/mail_template_service.rs` — MailTemplate literals in `create`/`update` service methods + two test mocks (update preserves prior `body_html`)
- `genossi_mail/src/service.rs` — MailJob literal in `create_job` + MailRecipient literal in the recipient loop + three test-scope MailJob/MailRecipient literals
- `genossi_mail/src/worker.rs` — two test-scope MailJob factories (`sample_job`, `make_test_job`)
- `genossi_mail/src/render.rs` — test-scope `make_job` / `make_recipient`
- `genossi_mail/src/backfill.rs` — test-scope `make_job` / `make_recipient`
- `genossi_mail/src/rest.rs` — test-scope `make_mail_job`

## Decisions Made

- **body_html defaulted to `None` at every construction site** — service, REST, template and inbox paths stay text-only until later Phase-23 plans wire the API and render path. Documented at each site.
- **`mail_template_service.update` preserves prior `body_html`** rather than clobbering to `None`. This keeps the through-path idempotent for text-only edits once Phase 23 Plan 04+ (or Phase 24) adds an HTML-aware update.
- **No `sqlx prepare` needed** — `genossi_mail` uses runtime `sqlx::query` (not compile-time `sqlx::query!`), so `.sqlx/` cache is untouched by this plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Extend body_html/rendered_html_body defaults to every construction site in genossi_mail**
- **Found during:** Task 3 (initial `cargo build -p genossi_mail`)
- **Issue:** The plan pointed at `dao.rs` + `dao_sqlite.rs` only, but the new mandatory struct fields broke every `MailJob { .. }` / `MailRecipient { .. }` / `MailTemplate { .. }` literal in `genossi_mail` (production paths in `inbox.rs`, `mail_template_service.rs`, `service.rs`, plus test factories in `worker.rs`, `render.rs`, `backfill.rs`, `rest.rs`).
- **Fix:** Added `body_html: None` / `rendered_html_body: None` to every literal. Production sites keep behavior identical (they were text-only before this plan and stay text-only until later plans wire the API); test factories default to `None` so existing assertions remain unchanged.
- **Files modified:** `genossi_mail/src/inbox.rs`, `mail_template_service.rs`, `service.rs`, `worker.rs`, `render.rs`, `backfill.rs`, `rest.rs`.
- **Verification:** `cargo build -p genossi_mail && cargo test -p genossi_mail --lib` both green; full workspace `cargo build` succeeds; 227 lib tests pass.
- **Committed in:** `cdcbbf73` (Task 3 commit).

**2. [Rule 3 — Blocking] Preserve `body_html` in `mail_template_service.update`**
- **Found during:** Task 3 (thinking through the update path).
- **Issue:** The plan phrased Task 3 in terms of the DAO wiring only, but the service `update` reconstructs a new `MailTemplate` value and would have needed a value for `body_html`. Defaulting to `None` would silently wipe any HTML body every time a text-only field was edited.
- **Fix:** Set `body_html: existing.body_html` in the update reconstruction so the current stored HTML is preserved through text-only edits until a later plan extends the service signature.
- **Files modified:** `genossi_mail/src/mail_template_service.rs`
- **Verification:** All 227 lib tests still pass; existing template tests (which use `body_html: None`) unaffected.
- **Committed in:** `cdcbbf73` (Task 3 commit).

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking build/behavior).
**Impact on plan:** Both auto-fixes are surgical and stay strictly within the "text-only unchanged, HTML wire deferred" contract of this plan. No scope creep — no service or REST-layer HTML wiring, no MIME changes.

## Issues Encountered

- The initial `cargo build` after Task 2 failed at 6 sites and after Task 3's build the tests failed at 6 more sites. Both were expected consequences of extending struct fields; fixed each site by adding the `None` default. Documented as Rule-3 auto-fixes above.
- Clippy printed one pre-existing warning in `worker.rs:105` (`unnecessary_sort_by`) that predates this plan and is out of scope. Logged for future cleanup, not fixed here.

## User Setup Required

None — DB schema changes are forward-only ADD COLUMN with NULL semantics; the next `cargo run --bin genossi` will auto-apply them at startup with no data touching.

## Next Phase Readiness

Foundation is in place for:
- **Plan 02 (HTML render env):** consume `body_html` and produce a rendered HTML body via a new autoescaping minijinja env.
- **Plan 03 (multipart/alternative in build_message):** emit the HTML part when `body_html.is_some()`.
- **Plan 04 (worker persistence):** write `rendered_html_body` alongside `rendered_body` in the same UPDATE — the UPDATE column order is already in the shape the worker needs.
- Phase 24 (WYSIWYG editor) has the DAO surface it needs for the API wire.

**Blockers:** none.

## Self-Check: PASSED

- ✓ `migrations/sqlite/20260702000000_mail_templates_add_body_html.sql` exists
- ✓ `migrations/sqlite/20260702000001_mail_jobs_add_body_html.sql` exists
- ✓ `migrations/sqlite/20260702000002_mail_recipients_add_rendered_html_body.sql` exists
- ✓ Migrations apply cleanly on scratch DB; `.schema` shows all three columns
- ✓ `grep -c 'pub body_html: Option<Arc<str>>' genossi_mail/src/dao.rs` → 2
- ✓ `grep -c 'pub rendered_html_body: Option<Arc<str>>' genossi_mail/src/dao.rs` → 1
- ✓ `grep -c 'body_html' genossi_mail/src/dao_sqlite.rs` → 28 (≥ 12 required)
- ✓ `grep -c 'rendered_html_body' genossi_mail/src/dao_sqlite.rs` → 16 (≥ 8 required)
- ✓ All three new tests exist and pass by name
- ✓ 3 jj commits present: `f0a4d284`, `2f9c3dd8`, `cdcbbf73`
- ✓ Full workspace `cargo build` succeeds; `cargo test -p genossi_mail --lib` → 227 passed / 0 failed

---
*Phase: 23-html-mail-backend*
*Completed: 2026-07-02*
