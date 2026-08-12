---
phase: 29-dao-schema-foundation-kommunikations-historie-pro-antragstel
plan: 02
subsystem: database
tags: [sqlx, sqlite, mail, genossi_mail, application_id, dao, carry-over, timeline, confirm]

# Dependency graph
requires:
  - phase: 29-01
    provides: "mail_recipients.application_id BLOB-Spalte + MailRecipient/RecipientInput.application_id + create_job-Threading + Index"
provides:
  - "CommunicationDao::get_application_communications(application_id) — outbound-only Antragsteller-Timeline (DAO-Ebene, kein REST)"
  - "MailRecipientDao::link_application_to_member(application_id, member_id) — Carry-over Back-fill UPDATE"
  - "MailService::link_application_recipients_to_member(application_id, member_id) — Service-Fassade fuer confirm()"
  - "confirm() post-commit best-effort Carry-over-Hook (genossi_service_impl/src/application.rs)"
  - "e2e-Beweis: Antragsteller-Erinnerung erscheint nach confirm() in Member-Timeline via genuiner new_member_id"
affects: [31-service-rest-versand, 32-frontend-compose-dialog]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Outbound-only Timeline-Query als reduzierter Klon der Member-Timeline (inbound/UNION-Zweig bewusst weggelassen — Antragsteller haben keine assigned_member_id)"
    - "Carry-over via gefiltertem UPDATE (WHERE application_id=? AND member_id IS NULL) — schreibt nur genuine member_id zurueck, ueberschreibt nie bereits zugeordnete Zeilen"
    - "Post-commit best-effort Hook (nach commit(tx), tracing::warn! statt ?) fuer nicht-atomare Cross-Connection-Konsistenz — Muster analog zum Datei-Cleanup"

key-files:
  created: []
  modified:
    - "genossi_mail/src/dao.rs"
    - "genossi_mail/src/dao_sqlite.rs"
    - "genossi_mail/src/service.rs"
    - "genossi_service_impl/src/application.rs"
    - "genossi_bin/tests/e2e_tests.rs"

key-decisions:
  - "D2 = Option A (Back-fill echte member_id): kein Audit-Ripple, kein Umbau der getesteten Member-Timeline-Query — im Gegensatz zu Option B/C"
  - "Carry-over-Hook laeuft post-commit best-effort auf separater Mail-Pool-Connection; Fehler loggt tracing::warn!, rollt confirm() NICHT zurueck"
  - "Mock-Erwartung fuer link_application_recipients_to_member permissiv (ohne times()-Untergrenze) zentral in build_service statt per Test — deckt Erfolgspfade, laesst perm-denied/rollback unberuehrt"

patterns-established:
  - "Antragsteller-Timeline als outbound-only Query neben der bestehenden inbound+outbound Member-Timeline"
  - "Namespace-sauberer Carry-over: genuine member_id auf application_id-Zeilen zurueckschreiben statt Application-UUID zu ueberladen (Pitfall 2)"

requirements-completed: [APHIST-01, APHIST-03]

coverage:
  - id: D1
    description: "get_application_communications liefert die als Antragsteller gesendete Mail outbound-only; fremde application_id → leer; Soft-Delete (r.deleted/j.deleted) ausgeschlossen; kein inbound-Eintrag"
    requirement: "APHIST-01"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_returns_outbound_entry"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_empty_for_foreign_application"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_excludes_soft_deleted_recipient"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_excludes_soft_deleted_job"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_is_outbound_only"
        status: pass
    human_judgment: false
  - id: D2
    description: "link_application_to_member schreibt genuine member_id gefiltert zurueck (WHERE member_id IS NULL); fremde application_id unangetastet; danach in Member-Timeline sichtbar"
    requirement: "APHIST-03"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_link_application_to_member_backfills_and_is_visible_in_member_timeline"
        status: pass
    human_judgment: false
  - id: D3
    description: "MailService::link_application_recipients_to_member delegiert an DAO-Back-fill; confirm() post-commit best-effort Hook ruft ihn ohne Rollback"
    requirement: "APHIST-03"
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_confirm_with_document_creates_audited_member_doc_and_soft_deletes"
        status: pass
      - kind: integration
        ref: "cargo test -p genossi_mail -p genossi_service_impl"
        status: pass
    human_judgment: false
  - id: D4
    description: "e2e: als Antragsteller (application_id, member_id None) geseedete Erinnerung erscheint nach confirm() in Timeline des neuen Mitglieds via genuiner new_member_id (Pitfall 2), ohne HTTP-Send-Endpoint"
    requirement: "APHIST-03"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_communication_carries_over_to_member_on_confirm"
        status: pass
    human_judgment: false

# Metrics
duration: 15min
completed: 2026-08-12
status: complete
---

# Phase 29 Plan 02: Antragsteller-Historie Read + confirm()-Carry-over Summary

**Outbound-only `get_application_communications`-DAO-Read fuer die Antragsteller-Timeline plus D2-Option-A-Carry-over: nach `confirm()` schreibt ein post-commit best-effort Hook die genuine neue `member_id` auf die als Antragsteller (`application_id`) gesendeten `mail_recipients`-Zeilen zurueck, sodass die Erinnerung automatisch in der UNVERAENDERTEN Mitglieds-Timeline erscheint.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-08-12T15:28:37Z
- **Completed:** 2026-08-12T15:44:33Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments
- `CommunicationDao::get_application_communications` — reduzierter, outbound-only Klon der Member-Timeline (`WHERE r.application_id = ?1`, Soft-Delete-Filter `r.deleted/j.deleted IS NULL`, kein inbound/UNION-Zweig), abgesichert durch 5 DAO-Tests (Roundtrip, fremd-application_id → leer, Recipient- und Job-Soft-Delete → ausgeschlossen, outbound-only trotz kollidierender assigned_member_id).
- `MailRecipientDao::link_application_to_member` — gefilterter `UPDATE mail_recipients SET member_id = ? WHERE application_id = ? AND member_id IS NULL`; schreibt nur die genuine neue member_id zurueck, ueberschreibt keine bereits zugeordneten Zeilen, laesst fremde application_id unangetastet (Pitfall 2). `MailService::link_application_recipients_to_member` delegiert an den DAO.
- confirm()-Post-Commit best-effort Carry-over-Hook (nach `commit(tx)`, neben dem Datei-Cleanup): `tracing::warn!` statt `?`, kein Rollback — Mail-DAO laeuft auf separater Pool-Connection. e2e beweist end-to-end: Erinnerung (application_id, member_id None) → confirm() → sichtbar in Member-Timeline des neuen Mitglieds via genuiner new_member_id.

## Task Commits

Jede Task wurde atomar committet:

1. **Task 1: DAO get_application_communications (outbound-only) + link_application_to_member** - `ac1397c` (feat)
2. **Task 2: MailService::link_application_recipients_to_member + confirm() post-commit Carry-over-Hook** - `1598f2e` (feat)
3. **Task 3: e2e Carry-over — Erinnerung → confirm → sichtbar in Member-Timeline** - `4d696e3` (test)

## Files Created/Modified
- `genossi_mail/src/dao.rs` - `CommunicationDao::get_application_communications` + `MailRecipientDao::link_application_to_member` Trait-Methoden (`#[automock]` erweitert Mocks automatisch)
- `genossi_mail/src/dao_sqlite.rs` - Impl beider Methoden (outbound-only SELECT + gefilterter UPDATE) + 6 DAO-Tests
- `genossi_mail/src/service.rs` - `MailService::link_application_recipients_to_member` Trait + Impl (Delegation an recipient_dao)
- `genossi_service_impl/src/application.rs` - confirm() post-commit best-effort Carry-over-Hook + permissive Mock-Erwartung in build_service
- `genossi_bin/tests/e2e_tests.rs` - `test_application_communication_carries_over_to_member_on_confirm` (Pool-Seeding, kein HTTP-Send-Endpoint)

## Decisions Made
- D2 = Option A (Back-fill echte member_id): die einzige Variante ohne Audit-Ripple und ohne Umbau der getesteten Member-Timeline-Query (Option C brauchte ein Feld auf der auditierten ApplicationEntity und briche `test_auditable_fields_count == 11`; Option B faktisch denselben Stored-Link).
- Carry-over-Hook post-commit best-effort auf separater Mail-Pool-Connection; nie atomar in confirm()s Transaktion. Fehler loggt `tracing::warn!`, Mitglied bleibt korrekt erstellt (T-29-06 accept, recoverable per Re-Run).
- Mock-Erwartung `expect_link_application_recipients_to_member` permissiv (0..n, keine times()-Untergrenze) zentral in `build_service` gesetzt statt pro Test — deckt beide confirm()-Erfolgspfade und laesst perm-denied (commit times(0)) und rollback (kein commit) unberuehrt, da mockall nur bei ungesetzt-aber-aufgerufen panickt.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Mock-Erwartung zentral in build_service statt per-Test**
- **Found during:** Task 2 (confirm()-Hook Unit-Tests)
- **Issue:** Der `MockMailService` wird im Test-Setup ausschliesslich innerhalb von `build_service` konstruiert (application.rs:1012) — die einzelnen confirm()-Erfolgs-Tests haben keinen direkten Zugriff, um `.expect_link_application_recipients_to_member()` wie im Plan-Wortlaut ("ihrem MockMailService je ... hinzufuegen") pro Test zu setzen, ohne die build_service-Signatur um einen mail_service-Parameter zu erweitern (Ripple ueber alle Aufrufer).
- **Fix:** Die permissive Erwartung (`.returning(|_, _| Ok(()))`, ohne times()-Untergrenze) einmalig zentral in `build_service` gesetzt. Ergebnis ist funktional identisch zum Plan-Ziel: Erfolgspfade gruen, perm-denied/rollback unberuehrt.
- **Files modified:** genossi_service_impl/src/application.rs
- **Verification:** `cargo test -p genossi_service_impl` 437/437 gruen; die confirm()-Erfolgs-Tests erreichen den Hook und passieren.
- **Committed in:** 1598f2e (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Minimal — identisches Testverhalten mit weniger Boilerplate, kein Scope-Creep, keine Signatur-Aenderung an build_service-Callern.

## Issues Encountered
- **Zwei pre-existing e2e-Failures (out of scope, nicht durch 29-02 verursacht):** `preview_body_html_round_trips_to_response` (Markdown-Bold `**Max**` wird im Preview-Body nicht zu Plain gestrippt) und `test_mail_preview_repayment_no_entries_does_not_default_to_one` (`errors`-Feld der `/api/mail/preview`-Response ist kein Array). Beide betreffen `/api/mail/preview`-Rendering (genossi_mail render/rest), Code den 29-02 nicht anfasst. In einem isolierten Worktree auf dem Baseline-Commit `4f7940e` (vor 29-02) reproduzieren beide identisch → nachweislich pre-existing. Nach Scope-Boundary-Regel NICHT gefixt; dokumentiert in `deferred-items.md`. Alle 29-02-Tests + die restlichen 315 e2e-Tests sind gruen.

## User Setup Required
None - keine externe Service-Konfiguration noetig. Kein neues DI-Wiring, keine neue Dependency, keine Migration in diesem Plan.

## Next Phase Readiness
- APHIST-01 (Read) + APHIST-03 (Carry-over) auf DAO/Service-Ebene fertig und getestet.
- Bereit fuer Phase 31: `RecipientInput.application_id = Some(application.id)` beim Antragsteller-Send setzen und den REST-Endpoint `GET /api/applications/{id}/communications` auf `get_application_communications` mounten (bewusst NICHT in Phase 29).
- Offene, unabhaengige Vorlast: zwei pre-existing `/api/mail/preview`-e2e-Failures (siehe deferred-items.md) — betreffen 29-02 nicht.

## Self-Check: PASSED
- Commits `ac1397c`, `1598f2e`, `4d696e3` in git log vorhanden.
- `cargo build --workspace` gruen.
- `cargo test -p genossi_mail -p genossi_service_impl` gruen (288 + 437), inkl. 6 neuer DAO-Tests, MailService-Methode und confirm()-Hook.
- `cargo test -p genossi_bin --test e2e_tests` 315 passed inkl. neuem `test_application_communication_carries_over_to_member_on_confirm`; die einzigen 2 Failures sind nachweislich pre-existing (baseline 4f7940e).

---
*Phase: 29-dao-schema-foundation-kommunikations-historie-pro-antragstel*
*Completed: 2026-08-12*
