---
phase: quick-260614-9zf
plan: 01
subsystem: mail
tags: [mail, dao, rest, frontend, audit]
requires:
  - mail_recipients table (existing)
  - mail worker per-recipient render (existing)
provides:
  - persisted per-recipient rendered_subject/rendered_body
  - rendered content exposed via MailRecipientTO (backend + frontend)
  - MailRecipientRenderedContent frontend component
affects:
  - genossi_mail (dao/worker/rest)
  - genossi-frontend (mail page)
tech-stack:
  added: []
  patterns:
    - "runtime sqlx::query (no compile-checked macros) — no sqlx prepare needed"
    - "Component-First display component reused across two recipient tables"
key-files:
  created:
    - migrations/sqlite/20260614000000_mail_recipient_rendered_subject_body.sql
    - genossi-frontend/src/component/mail_recipient_rendered_content.rs
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/service.rs
    - genossi_mail/src/inbox.rs
    - genossi_mail/src/rest.rs
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/component/mod.rs
    - genossi-frontend/src/page/mail_page.rs
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
decisions:
  - "rendered_subject/rendered_body are nullable: legacy rows + not-yet/failed-before-render recipients have no rendered content"
  - "serde(default) on the frontend api model so responses without the fields still deserialize"
  - "rendered_* written in the worker for BOTH sent and send-failed paths (render precedes send); pre-render failures keep None via mark_recipient_failed + continue"
metrics:
  duration: ~18min
  completed: 2026-06-14
---

# Quick 260614-9zf: Mail-Versand gerenderten Betreff und Text Summary

Persist the per-recipient rendered subject and body (template-interpolated content the worker actually sent) on `MailRecipient` and surface it end-to-end up to a reusable Dioxus display component, so the Vorstand can see exactly what each member received.

## What Was Built

**Task 1 — Backend persistence (commit 1bfd772):**
- Migration `20260614000000_mail_recipient_rendered_subject_body.sql`: two nullable `TEXT` columns `rendered_subject`, `rendered_body` on `mail_recipients` (no down-migration, SQLite convention).
- `MailRecipient` entity gains `rendered_subject: Option<Arc<str>>` and `rendered_body: Option<Arc<str>>`.
- SQLite DAO: `MailRecipientDb` + `TryFrom` mapping, `create` (NULL on insert), `find_by_job_id` and `next_pending` SELECT lists, and `update` SET-clause all extended.
- Worker writes both rendered values onto `updated_recipient` before `recipient_dao.update()` — covers the sent and the send-failed path; pre-render failures (`mark_recipient_failed` + `continue`) correctly leave them None.
- Test schema (`setup_db`) + `sample_recipient` extended. New roundtrip test `test_recipient_update_persists_rendered_subject_body` (persistence + preservation of existing fields) and `test_recipient_next_pending_maps_rendered_fields_as_none`.

**Task 2 — REST output (commit 4c0fac9):**
- `MailRecipientTO` gains `rendered_subject`/`rendered_body` (skip-serialized when None); `From<&MailRecipient>` maps them.

**Task 3 — Frontend (commit b073642):**
- `api::MailRecipientTO` gains the two fields with `#[serde(default)]` for backward-compatible deserialization.
- New Component-First `MailRecipientRenderedContent` (renders nothing when both None) registered in `component/mod.rs`.
- Both recipient tables in `mail_page.rs` (expandable job list + `MailJobDetail`) render it via a `colspan` detail row — no inline RSX duplication.
- i18n `Key::MailRenderedContent` added in `mod.rs`/`de.rs`/`en.rs` ("Gesendeter Inhalt" / "Sent content"); existing `MailSubject`/`MailBody` reused as labels.
- serde-default deserialization tests for legacy and full responses.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Additional `MailRecipient` constructions updated**
- **Found during:** Task 1
- **Issue:** The struct field addition broke direct `MailRecipient { .. }` constructions in `genossi_mail/src/service.rs` (3 sites) and `genossi_mail/src/inbox.rs` (1 site) — not listed in the plan.
- **Fix:** Added `rendered_subject: None, rendered_body: None` to all four.
- **Files modified:** genossi_mail/src/service.rs, genossi_mail/src/inbox.rs
- **Commit:** 1bfd772

**2. [Rule 3 - Blocking] Frontend is a separate cargo project, not a workspace member**
- **Found during:** Task 3 verify
- **Issue:** `cargo check -p genossi-frontend` failed ("did not match any packages") — `genossi-frontend` is `exclude`d in the root `Cargo.toml` and has its own `Cargo.lock`.
- **Fix:** Verified via `cargo check --manifest-path genossi-frontend/Cargo.toml` (0 errors) and `cargo test --manifest-path ...`.
- **Commit:** n/a (tooling adjustment)

## Verification

- `cargo test -p genossi_mail`: 181 passed, 0 failed (includes new DAO roundtrip tests).
- `cargo build -p genossi_mail`: green.
- `cargo check --manifest-path genossi-frontend/Cargo.toml`: 0 errors (only pre-existing dead-code warnings, out of scope).
- `cargo test --manifest-path genossi-frontend/Cargo.toml`: new serde-default tests + existing mail component tests pass.

## Commits

- 1bfd772: feat(quick-260614-9zf): persist per-recipient rendered subject + body
- 4c0fac9: feat(quick-260614-9zf): expose rendered_subject/rendered_body in MailRecipientTO
- b073642: feat(quick-260614-9zf): show per-recipient rendered subject + body in frontend

## Self-Check: PASSED

- migrations/sqlite/20260614000000_mail_recipient_rendered_subject_body.sql — FOUND
- genossi-frontend/src/component/mail_recipient_rendered_content.rs — FOUND
- Commits 1bfd772, 4c0fac9, b073642 — all FOUND
