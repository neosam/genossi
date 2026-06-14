---
phase: quick-260614-b1t
plan: 01
subsystem: mail
tags: [backfill, mail, rendering, dry, frontend]
requires:
  - "Quick 260614-9zf (nullable rendered_subject/rendered_body + worker persistence)"
provides:
  - "rendered_reconstructed flag on mail_recipients (DB + DAO + REST + frontend)"
  - "DRY render::resolve_rendered_content shared by worker and backfill"
  - "one-shot idempotent startup backfill for legacy NULL-rendered rows"
  - "frontend amber 'Nachträglich rekonstruiert' badge"
affects:
  - genossi_mail
  - genossi_bin
  - genossi-frontend
tech-stack:
  patterns:
    - "one-shot startup backfill worker (mirrors start_attachment_backfill_worker)"
    - "shared render function as single source of truth (DRY)"
key-files:
  created:
    - migrations/sqlite/20260614010000_mail_recipient_rendered_reconstructed.sql
    - genossi_mail/src/render.rs
    - genossi_mail/src/backfill.rs
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/service.rs
    - genossi_mail/src/inbox.rs
    - genossi_mail/src/lib.rs
    - genossi_mail/src/rest.rs
    - genossi_bin/src/lib.rs
    - genossi_bin/src/main.rs
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/component/mail_recipient_rendered_content.rs
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
    - genossi-frontend/src/page/mail_page.rs
decisions:
  - "Render-Extraktion in eigenes Modul render.rs (statt template.rs) — template.rs ist für pure Render-Bausteine; die generische DAO-Signatur gehört in ein eigenes Modul"
  - "Backfill als one-shot tokio::spawn (kein loop) — Idempotenz folgt aus find_recipients_without_rendered, das gefüllte Zeilen nicht mehr liefert"
metrics:
  duration: ~14min
  completed: 2026-06-14
---

# Phase quick-260614-b1t Plan 01: Backfill rendered_subject/rendered_body Summary

Retroaktive Rekonstruktion von rendered_subject/rendered_body für Alt-Zeilen mit sichtbarer `rendered_reconstructed`-Kennzeichnung, plus DRY-Extraktion der Render-Pipeline (Worker + Backfill teilen eine Funktion) und idempotentem One-Shot-Backfill beim Server-Start.

## What Was Built

- **Migration `20260614010000`**: `rendered_reconstructed INTEGER NOT NULL DEFAULT 0` auf `mail_recipients`. Alt-Zeilen lesen als `false` zurück.
- **DAO**: `MailRecipient.rendered_reconstructed: bool`; neue Trait-Methode `find_recipients_without_rendered` (NULL subject AND body, deleted IS NULL, inkl. status='failed'). SQLite-Impl in create/update/find_by_job_id/next_pending durchgezogen.
- **`render::resolve_rendered_content`** (neu, DRY): die aus dem Worker extrahierte Render-Pipeline (member-only, repayment-merge inkl. EntityNotFound→unmerged, plain-passthrough, fehlender Member → Err). Worker UND Backfill rufen ausschließlich diese Funktion — kein duplizierter Aggregations-Block mehr in worker.rs (verifiziert per grep).
- **Worker**: ruft die Shared-Funktion; setzt `rendered_reconstructed = false` beim Live-Versand.
- **`backfill::run_rendered_backfill`** (neu): One-Shot, idempotent. Füllt NULL-Zeilen mit `rendered_reconstructed=true`; überspringt Zeilen mit fehlendem Member / Render-Fehler / Job-Lookup-Fehler (bleiben NULL → nächster Start retried).
- **Startup-Wiring**: `start_rendered_backfill_worker()` in genossi_bin, aufgerufen in main.rs nach `sqlx::migrate!`.
- **REST**: `MailRecipientTO.rendered_reconstructed` (immer serialisiert, `#[serde(default)]` für Input).
- **Frontend**: api-Feld (`#[serde(default)]`), i18n-Key `MailRenderedReconstructed` (de: "Nachträglich rekonstruiert", en: "Reconstructed afterwards"), amber Badge in `MailRecipientRenderedContent`, beide Aufrufstellen in mail_page.rs.

## Tests

- `genossi_mail`: 190 passed (neu: render-Extraktion member-only/repayment-merge/plain-passthrough/missing-member; DAO-Roundtrip-Flag; find_recipients_without_rendered-Filter; Backfill fill+flag/skip-missing-member/idempotent).
- `genossi_bin`: `cargo build -p genossi_bin` grün.
- `genossi-frontend`: `cargo check` grün; 261 tests passed (bestehende serde-default-Tests intakt).

## Deviations from Plan

None - plan executed exactly as written. (Render-Extraktion ins separate `render.rs`-Modul war vom Plan ausdrücklich als Executor-Entscheidung freigegeben.)

## Commits

- 9bce486: feat(quick-260614-b1t): rendered_reconstructed flag, DRY render extraction, startup backfill
- 438c54d: feat(quick-260614-b1t): expose rendered_reconstructed in MailRecipientTO
- 4ef4768: feat(quick-260614-b1t): frontend badge for reconstructed rendered content

## Self-Check: PASSED

- migrations/sqlite/20260614010000_mail_recipient_rendered_reconstructed.sql: FOUND
- genossi_mail/src/render.rs: FOUND
- genossi_mail/src/backfill.rs: FOUND
- Commits 9bce486, 438c54d, 4ef4768: FOUND
- Render logic single definition (resolve_rendered_content): 1 match in render.rs
- worker.rs duplicate aggregation block: none (grep empty)
