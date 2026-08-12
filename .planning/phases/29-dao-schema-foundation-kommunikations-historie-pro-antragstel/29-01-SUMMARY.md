---
phase: 29-dao-schema-foundation-kommunikations-historie-pro-antragstel
plan: 01
subsystem: database
tags: [sqlx, sqlite, migration, mail, genossi_mail, application_id, dao]

# Dependency graph
requires:
  - phase: v1.5-genossi_mail
    provides: "mail_recipients-Tabelle, MailRecipient/RecipientInput-Structs, create_job-Persistenzpfad, member_id-Linkage-Muster"
provides:
  - "nullable application_id BLOB Spalte + Index idx_mail_recipients_application_id auf mail_recipients"
  - "MailRecipient.application_id: Option<Uuid> (genossi_mail/src/dao.rs)"
  - "RecipientInput.application_id: Option<Uuid> (genossi_mail/src/service.rs) — Phase 31 setzt hier Some(application.id)"
  - "MailRecipientDb.application_id + TryFrom-Parse (dao_sqlite.rs)"
  - "create_job faedelt input.application_id in die INSERT-Bindings"
  - "DAO-Roundtrip-, NULL-Legacy- und Namespace-Gate-Tests"
affects: [30-application-template-kontext, 31-service-rest-versand, 32-frontend-compose-dialog, 29-02]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Additive nullable Geschwisterspalte (application_id spiegelt member_id) — kein DEFAULT/NOT NULL, forward-only, NULL-Legacy byte-identisch"
    - "Namespace-Trennung via zweiter nullable Linkage-Spalte statt Ueberladung von member_id (Pitfall 2)"
    - "Persistenz-Roundtrip-Test als staerkere Absicherung gegen Namespace-Poisoning statt Grep-Gate"

key-files:
  created:
    - "migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql"
  modified:
    - "genossi_mail/src/dao.rs"
    - "genossi_mail/src/dao_sqlite.rs"
    - "genossi_mail/src/service.rs"
    - "genossi_mail/src/rest.rs"
    - "genossi_mail/src/inbox.rs"
    - "genossi_mail/src/worker.rs"
    - "genossi_mail/src/render.rs"
    - "genossi_mail/src/backfill.rs"
    - "genossi_service_impl/src/application.rs"

key-decisions:
  - "application_id ist eine eigene nullable BLOB-Spalte neben member_id — eine Application-UUID landet nie in member_id (Namespace sauber)"
  - "find_sent_member_ids_by_job_id und update() bewusst unveraendert gelassen (selektieren/setzen member_id nicht)"
  - "Namespace-Gate als Persistenz-Roundtrip-Assert (member_id.is_none() bei gesetztem application_id) statt verbatim-Grep"

patterns-established:
  - "Additive Migration-Spiegelung eines getesteten Linkage-Pfads (member_id -> application_id)"
  - "Zweiter nullable Namespace-Slot fuer Subjekt-Zuordnung ohne Ueberladung des bestehenden Slots"

requirements-completed: [APHIST-01]

coverage:
  - id: D1
    description: "application_id ist nullable-Geschwisterspalte von member_id (Schema + Migration + Index)"
    requirement: "APHIST-01"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_recipient_roundtrip_application_id"
        status: pass
      - kind: other
        ref: "test -f migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql && grep ADD COLUMN application_id BLOB / idx_mail_recipients_application_id"
        status: pass
    human_judgment: false
  - id: D2
    description: "application_id-Feld durch MailRecipient, RecipientInput, MailRecipientDb, alle 6 SQL-Spaltenlisten und create_job gefaedelt; Workspace kompiliert"
    requirement: "APHIST-01"
    verification:
      - kind: integration
        ref: "cargo build --workspace"
        status: pass
    human_judgment: false
  - id: D3
    description: "Persistenz-Roundtrip fuer application_id + NULL-Legacy byte-identisch"
    requirement: "APHIST-01"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_recipient_roundtrip_null_legacy_application_id"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_recipient_roundtrip_application_id"
        status: pass
    human_judgment: false
  - id: D4
    description: "member_id-Namespace beweisbar sauber — Application-UUID nie in member_id"
    requirement: "APHIST-01"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_recipient_application_send_keeps_member_id_namespace_clean"
        status: pass
    human_judgment: false

# Metrics
duration: 18min
completed: 2026-08-12
status: complete
---

# Phase 29 Plan 01: DAO/Schema-Foundation application_id-Linkage Summary

**Nullable `application_id BLOB` Geschwisterspalte auf `mail_recipients` (Schema + Index + Struct + Row + alle 6 SQL-Spaltenlisten + create_job-Threading), sodass eine an einen Antragsteller gesendete Mail persistiert werden kann, ohne den member_id-Namespace zu vergiften.**

## Performance

- **Duration:** ~18 min
- **Completed:** 2026-08-12
- **Tasks:** 3
- **Files modified:** 9 modifiziert + 1 Migration erstellt

## Accomplishments
- Additive, forward-only Migration `20260812000000_mail_recipients_add_application_id.sql` (ADD COLUMN application_id BLOB nullable + Index idx_mail_recipients_application_id), spiegelbildlich zu member_id — keine Down-Migration (SQLite < 3.35).
- `application_id: Option<Uuid>` als Feld auf `MailRecipient` und `RecipientInput`, durch `MailRecipientDb` (TryFrom via parse_optional_uuid), alle sechs verbatim SQL-Spaltenlisten (INSERT + 3 SELECT + Test-DDL) und den `create_job`-Persistenzpfad gefaedelt — Workspace kompiliert grün.
- Drei DAO-Tests: application_id-Roundtrip, NULL-Legacy-Roundtrip (Bestandszeilen lesen NULL byte-identisch) und Namespace-Gate (member_id.is_none() bei gesetztem application_id, Pitfall 2 / T-29-01). 282/282 genossi_mail-Tests grün, keine Regression.

## Task Commits

Jede Task wurde atomar committet:

1. **Task 1: Additive Migration application_id + Index** - `3786878` (feat)
2. **Task 2: application_id durch Struct, Row, alle SQL-Spaltenlisten und create_job faedeln (atomar)** - `3248e70` (feat)
3. **Task 3: DAO-Roundtrip-, NULL-Legacy- und Namespace-Tests** - `408a624` (test)

## Files Created/Modified
- `migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql` - ALTER ADD COLUMN application_id BLOB (nullable) + Index, forward-only
- `genossi_mail/src/dao.rs` - `MailRecipient.application_id: Option<Uuid>`
- `genossi_mail/src/service.rs` - `RecipientInput.application_id: Option<Uuid>` + create_job-Threading (`application_id: input.application_id`)
- `genossi_mail/src/dao_sqlite.rs` - `MailRecipientDb.application_id`, TryFrom-Parse, application_id in INSERT + 3 SELECT-Listen + Test-DDL; 3 neue Tests; find_sent_member_ids_by_job_id/update bewusst unveraendert
- `genossi_mail/src/rest.rs` - 2 RecipientInput-Literale um `application_id: None` ergaenzt
- `genossi_mail/src/inbox.rs`, `worker.rs`, `render.rs`, `backfill.rs` - MailRecipient-Literale um `application_id: None` ergaenzt
- `genossi_service_impl/src/application.rs` - send_confirmation_mail RecipientInput um `application_id: None` ergaenzt (Member-Send, kein Application-Send)

## Decisions Made
- application_id ist eine eigene nullable BLOB-Spalte neben member_id; eine Application-UUID landet nie in member_id (Namespace-Trennung, Pitfall 2).
- `find_sent_member_ids_by_job_id` und `update()` bewusst unveraendert — sie selektieren/setzen member_id nicht; im Code als Kommentar notiert.
- Alle Struct-Literale in dieser Phase setzen `application_id: None` (der echte Application-Send kommt erst in Phase 31).
- Namespace-Absicherung als Persistenz-Roundtrip-Assert statt Grep-Gate (staerker, keine Comment-Text-Fallen).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `cargo fmt` (per globaler CLAUDE.md-Konvention aufgerufen) reformatierte ~24 vorbestehende, nicht zu diesem Plan gehoerende Dateien (bestehende Formatierungs-Drift im Repo). Diese Aenderungen wurden NICHT gestaged/committet — jeder Commit staged ausschliesslich die explizit zum Plan gehoerenden Pfade (plus `git read-tree HEAD` vor jedem Stagen im colocated jj+git-Repo). Der Versuch, die fmt-Drift per `git checkout --` zurueckzusetzen, wurde vom Auto-Mode-Classifier blockiert; die betroffenen Dateien verbleiben unstaged im Working Tree und sind kein Teil der 29-01-Commits.

## User Setup Required
None - keine externe Service-Konfiguration noetig. Die Migration laeuft additiv beim naechsten Startup gegen die Live-DB (Bestandszeilen lesen application_id=NULL).

## Next Phase Readiness
- Persistenz-Fundament (APHIST-01) steht: `application_id` existiert, ist persistierbar und wird byte-identisch ausgelesen.
- Bereit fuer Plan 29-02 (`get_application_communications`-Read-Methode, `link_application_recipients_to_member`-Carry-over bei confirm()).
- Bereit fuer Phase 31, die `RecipientInput.application_id = Some(application.id)` setzt.

## Self-Check: PASSED
- `migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql` existiert auf Disk.
- Commits 3786878, 3248e70, 408a624 in git log vorhanden.
- `cargo build --workspace` grün; `cargo test -p genossi_mail` 282/282 grün inkl. der 3 neuen Tests.

---
*Phase: 29-dao-schema-foundation-kommunikations-historie-pro-antragstel*
*Completed: 2026-08-12*
