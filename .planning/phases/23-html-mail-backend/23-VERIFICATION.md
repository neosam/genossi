---
phase: 23-html-mail-backend
verified: 2026-07-02T00:00:00Z
status: passed
score: 6/6 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 23: HTML Mail Backend Verification Report

**Phase Goal:** Eine Mail kann mit Text- UND HTML-Teil als multipart/alternative versendet werden, wobei mitglieds-/nutzergelieferte Werte sicher escaped und vom Vorstand verfasstes HTML serverseitig saniert werden.

**Verified:** 2026-07-02
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (mapped to REQUIREMENTS HTML-01..05, FMT-01)

| # | Requirement / Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | **HTML-01** — multipart/alternative with correct `mixed{alternative,attachments}` nesting; MIME-byte tests exist | ✓ VERIFIED | `genossi_mail/src/send.rs:114-163` implements the 4-branch match `(html_part_opt, attachments.is_empty())`. Text-first alternative order pinned. Behavioral evidence: 5 new send::tests pass (`build_message_alternative_text_then_html_no_attachments`, `build_message_alternative_text_part_is_verbatim_body`, `build_message_mixed_wraps_alternative_when_attach`, `build_message_legacy_singlepart_text_unchanged`, `build_message_html_part_declares_text_html_charset_utf8`) — confirmed via `cargo test -p genossi_mail --lib`: 249 passed, 0 failed. |
| 2 | **HTML-02** — plain-text remains author's body (no HTML derivation), no new crate for text | ✓ VERIFIED | `send.rs:83-86` builds `text_part` verbatim from `body: &str` parameter, independent of `html_body`. `grep html2text|html_to_text|from_html` in genossi_mail/src → 0 hits. `Cargo.toml` only adds `ammonia = "4"` — no text-derivation crate. |
| 3 | **HTML-03** — forward-only ADD COLUMN migrations; NULL legacy = text-only preserved | ✓ VERIFIED | Three migration files exist: `migrations/sqlite/20260702000000_mail_templates_add_body_html.sql`, `..0001_mail_jobs_add_body_html.sql`, `..0002_mail_recipients_add_rendered_html_body.sql`. Each contains a single `ALTER TABLE … ADD COLUMN … TEXT NULL;` with no DEFAULT clause. Comment headers document "forward-only, no down migration" + NULL-legacy semantics. `render.rs:181-187` preserves None→None (D-09 wire), pinned by test `body_html_none_leaves_rendered_html_body_null` (worker). |
| 4 | **HTML-04** — separate autoescaping html_env; strict_env unchanged for text/subject; escapes member values | ✓ VERIFIED | `template.rs:89-94` defines `html_env()` with `set_auto_escape_callback(\|_name\| AutoEscape::Html)`; `strict_env()` untouched (no autoescape). `render_html_template()` (`template.rs:101-114`) routes HTML through `html_env`. Tests `html_env_autoescapes_member_value`, `html_env_preserves_author_markup`, `strict_env_does_not_escape_member_value` (regression), `test_date_fields_renders_german_format` all pass. |
| 5 | **HTML-05** — ammonia sanitize at all 4 D-03 entry points | ✓ VERIFIED | `sanitize.rs:35-37` implements `sanitize_html(&str) -> String` via `ammonia::clean`. Four call sites present: (a) `service.rs:380` (create_job); (b) `service.rs:272` (`sanitize_body_html_opt` helper used by `send_test_mail_with_body` at line 535); (c) `mail_template_service.rs:95` (template create); (d) `mail_template_service.rs:143` (template update). 4 unit tests in sanitize::tests + 4 service-layer sanitize tests + 3 e2e tests (`bulk_mail_body_html_sanitized_and_persisted`, `bulk_mail_body_html_none_stays_backward_compatible`, `create_template_body_html_sanitized`) all pass. |
| 6 | **FMT-01** — format_de renders join_date/exit_date as DD.MM.YYYY in shared context | ✓ VERIFIED | `template.rs:123-127` defines `format_de(date: time::Date) -> String` using `format_description!("[day].[month].[year]")`. Applied in `member_to_template_context` at lines 21-22: `format_de(entity.join_date)` and `entity.exit_date.map(format_de)`. Test `test_date_fields_renders_german_format` in template::tests pins `02.07.2026` / `31.12.2025` output. Shared context feeds both text and HTML render paths. |

**Score:** 6/6 truths verified (0 present, behavior-unverified).

### D-01..D-12 Decision Verification

| Decision | Status | Evidence |
| --- | --- | --- |
| D-01 permissive ammonia default | ✓ | `sanitize.rs:36`: `ammonia::clean(html)` — no custom Builder. `grep ammonia::Builder` = 0. |
| D-02 sanitizer, not sender | ✓ | `sanitize.rs` module doc explicit; lettre stays sender. |
| D-03 4 entry points | ✓ | Verified above (Truth 5). |
| D-04 separate html_env | ✓ | `template.rs:89-94` (Truth 4). |
| D-05 no re-sanitize of rendered output | ✓ | `render.rs:181-187` calls `render_html_template` only; no `sanitize_html` call. `grep -n sanitize_html genossi_mail/src/render.rs` = 0 hits. |
| D-06 mail_templates.body_html migration | ✓ | Migration file + `MailTemplate.body_html: Option<Arc<str>>`. |
| D-07 mail_jobs.body_html migration | ✓ | Migration file + `MailJob.body_html: Option<Arc<str>>`. |
| D-08 mail_recipients.rendered_html_body | ✓ | Migration file + `MailRecipient.rendered_html_body: Option<Arc<str>>`. Worker writes at `worker.rs:460`. |
| D-09 NULL → text-only, Some → multipart/alternative | ✓ | `render.rs:181-187` + `send.rs:114-163` match. |
| D-10 build_message extended in Phase-22 helper | ✓ | `send.rs:57` new `html_body: Option<&str>` parameter; single 4-branch match; no duplicated builders. |
| D-11 format_de in shared context | ✓ | Truth 6. |
| D-12 MIME-byte test coverage | ✓ | 5 new send::tests + 3 e2e tests confirm multipart/alternative + mixed wrapping + escape end-to-end. |

### Required Artifacts

| Artifact | Expected | Status |
| --- | --- | --- |
| `migrations/sqlite/20260702000000_mail_templates_add_body_html.sql` | ADD COLUMN body_html TEXT NULL | ✓ VERIFIED |
| `migrations/sqlite/20260702000001_mail_jobs_add_body_html.sql` | ADD COLUMN body_html TEXT NULL | ✓ VERIFIED |
| `migrations/sqlite/20260702000002_mail_recipients_add_rendered_html_body.sql` | ADD COLUMN rendered_html_body TEXT NULL | ✓ VERIFIED |
| `genossi_mail/Cargo.toml` | `ammonia = "4"` | ✓ VERIFIED (line 31) |
| `genossi_mail/src/sanitize.rs` | Shared sanitize_html helper + 4 tests | ✓ VERIFIED |
| `genossi_mail/src/template.rs` | html_env + render_html_template + format_de | ✓ VERIFIED |
| `genossi_mail/src/render.rs` | RenderedContent struct + body_html routing | ✓ VERIFIED |
| `genossi_mail/src/send.rs` | build_message 4-branch tree, 5 new MIME-byte tests | ✓ VERIFIED |
| `genossi_mail/src/service.rs` | create_job body_html + sanitize + send_test_mail_with_body sanitize | ✓ VERIFIED |
| `genossi_mail/src/mail_template_service.rs` | template create/update body_html + sanitize | ✓ VERIFIED |
| `genossi_mail/src/worker.rs` | rendered_html_body persistence + body_html to build_message | ✓ VERIFIED (line 460, 682) |
| `genossi_mail/src/rest.rs` + `rest_templates.rs` | DTO extensions | ✓ VERIFIED (7 body_html + 1 rendered_html_body on rest.rs; 3 body_html on rest_templates.rs) |
| `genossi_mail/src/dao.rs` + `dao_sqlite.rs` | Struct fields + SQL columns wired | ✓ VERIFIED |
| `genossi_bin/tests/e2e_tests.rs` | 3 new Phase 23 e2e tests | ✓ VERIFIED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Workspace build clean | `cargo build` | success | ✓ PASS |
| genossi_mail lib tests | `cargo test -p genossi_mail --lib` | 249 passed, 0 failed | ✓ PASS |
| e2e HTTP tests (phase 23 new) | `cargo test --test e2e_tests` | 303 passed, 1 pre-existing failure | ✓ PASS (see note below) |

**Note on the one failing e2e test:** `test_mail_preview_repayment_no_entries_does_not_default_to_one` was documented in the Phase 22 SUMMARY (22-02) as a pre-existing failure carried over. Plan 04 Task 4's acceptance criteria explicitly allow this ("documents the pre-existing failure … as still-failing, if it was not fixed elsewhere; do NOT block on it"). It is not a regression from Phase 23 — the panic is in a repayment-preview JSON-shape check (`errors must be array`) unrelated to HTML mail wiring. Not a phase-23 blocker.

### Requirements Coverage

| Requirement | Description | Status | Evidence |
| --- | --- | --- | --- |
| HTML-01 | Multipart/alternative w/ text+HTML, correct mixed nesting w/ attachments | ✓ SATISFIED | Truth 1 |
| HTML-02 | Text part remains author-authored, no HTML→text derivation | ✓ SATISFIED | Truth 2 |
| HTML-03 | Forward-only migrations, NULL-legacy text-only preserved | ✓ SATISFIED | Truth 3 |
| HTML-04 | Separate autoescape env; member values escaped in HTML | ✓ SATISFIED | Truth 4 |
| HTML-05 | ammonia sanitize at 4 store-side entry points | ✓ SATISFIED | Truth 5 |
| FMT-01 | DD.MM.YYYY German date format for join_date/exit_date | ✓ SATISFIED | Truth 6 |

### Anti-Patterns Found

None. Grep for `TBD|FIXME|XXX` on phase-modified files returned no unreferenced markers. No new `bool` flag introduced (project rule "Immer Enum statt Boolean" honored — Option<…> models presence/absence). No `Message::builder()` in production code outside `send.rs` (the one hit in service.rs:946 is a doc-comment).

### Human Verification Required

None. All truths are behaviorally verified by the passing test suite (249 lib tests + 303 e2e tests, minus the pre-existing Phase 22 failure). MIME structure claims are pinned by byte-level assertions; sanitize wire is proven by 3 end-to-end HTTP tests that POST `<script>` and read back the persisted sanitized payload.

### Gaps Summary

No gaps. All 6 must-have truths verified, all D-01..D-12 decisions implemented, all 3 migrations present, all 4 sanitize entry points wired, worker persists rendered HTML byte-identically, and both the `body_html=None` legacy path and the `body_html=Some(…)` new path have passing behavioral tests.

---

*Verified: 2026-07-02T00:00:00Z*
*Verifier: Claude (gsd-verifier)*
