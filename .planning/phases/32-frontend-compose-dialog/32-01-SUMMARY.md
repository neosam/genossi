---
phase: 32-frontend-compose-dialog
plan: 01
subsystem: api
tags: [rust, sqlx, sqlite, dioxus, rest-types, communication-timeline, mail]

# Dependency graph
requires:
  - phase: 29-application-communications
    provides: get_application_communications DAO + outbound-only Antragsteller-Timeline
  - phase: 23-html-mail
    provides: mail_recipients.rendered_html_body (per-Empfaenger persistierter, sanitisierter HTML-Body)
provides:
  - CommunicationEntry traegt rendered_body/rendered_html_body durch die gesamte DAO->TO-Kette (Backend + Frontend)
  - GET /api/applications/{id}/communications liefert den echten gespeicherten Render-Body je Outbound-Eintrag
affects: [32-04, frontend-compose-dialog, body-detail-panel]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Additive DAO->TO-Feldkette ueber beide rest-types-Crates (Landmine 1: doppelte handgepflegte Frontend-Crate)"
    - "Geteilte FromRow-Struct: beide UNION-Zweige (member NULL-Platzhalter, application r.rendered_*) muessen Spaltenzahl konsistent halten"

key-files:
  created: []
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs
    - genossi_mail/src/communication_rest.rs
    - genossi-frontend/rest-types/src/lib.rs

key-decisions:
  - "Test seedet rendered_body via direktes SQL-UPDATE (create() schreibt hardcodiert NULL) — spiegelt den realen Versand-Backfill"
  - "Member-SELECT nur um SQL-Konsistenz gleichgezogen (NULL im inbound-Zweig, r.rendered_* im outbound-Zweig); Member-Body-View bleibt Scope spaeterer Plaene"

patterns-established:
  - "rendered_*-Durchreichung ohne Re-Render: Body kommt byte-genau aus mail_recipients, keine Schema-Migration, kein Audit-Feld"

requirements-completed: [APUI-03]

coverage:
  - id: D1
    description: "get_application_communications liefert je Outbound-Eintrag den persistierten rendered_body/rendered_html_body (Some) und None fuer Legacy-Zeilen"
    requirement: "APUI-03"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_exposes_rendered_body"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#test_application_communications_rendered_body_none_for_legacy_row"
        status: pass
    human_judgment: false
  - id: D2
    description: "Frontend-rest-types CommunicationEntryTO traegt rendered_body/rendered_html_body wire-kompatibel (skip_serializing_if), deserialisiert mit + ohne Felder"
    requirement: "APUI-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/rest-types/src/lib.rs#communication_entry_deserializes_rendered_body_fields"
        status: pass
      - kind: unit
        ref: "genossi-frontend/rest-types/src/lib.rs#communication_entry_missing_rendered_body_is_none"
        status: pass
    human_judgment: false
  - id: D3
    description: "Member-Communications-Pfad bleibt im Verhalten unveraendert (Query kompiliert, Felder additiv)"
    requirement: "APUI-03"
    verification:
      - kind: integration
        ref: "nix develop --command cargo test -p genossi_mail (311 passed, bestehende member-timeline-Tests gruen)"
        status: pass
    human_judgment: false

# Metrics
duration: ~20min
completed: 2026-08-21
status: complete
---

# Phase 32 Plan 01: D-06 Backend-Kette + Wire-Typ Summary

**Der bereits in `mail_recipients` persistierte, per-Empfaenger gerenderte Mail-Body wird additiv durch die gesamte CommunicationEntry-Kette (dao.rs -> dao_sqlite.rs -> communication_rest.rs) bis in beide rest-types-Crates sichtbar gemacht — ohne Schema-Migration und ohne Re-Render.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-08-21
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- `CommunicationEntry` (dao.rs) traegt `rendered_body`/`rendered_html_body: Option<Arc<str>>` bei den Outbound-Feldern.
- `CommunicationEntryDb` + `TryFrom` mappen die Felder; **beide** SQL-Queries (member + application) selektieren die Spalten in allen UNION-Zweigen konsistent (member-inbound `NULL AS ...`, outbound/application `r.rendered_body`/`r.rendered_html_body`).
- Backend-`CommunicationEntryTO` + `From`-Impl (communication_rest.rs) reichen die Felder mit `skip_serializing_if` durch — Handler `application.rs:599` unveraendert.
- Frontend-`CommunicationEntryTO` (genossi-frontend/rest-types) gespiegelt (Landmine 1: separate handgepflegte Crate) — wire-kompatibel.
- 4 neue Tests (2 DAO Some/None, 2 serde-Roundtrip mit/ohne Felder), alle gruen.

## Task Commits

1. **Task 1 (RED): failing DAO tests** - `b651345` (test)
2. **Task 1 (GREEN): expose rendered_body in application comms chain** - `8ec2f04` (feat)
3. **Task 2: mirror rendered_body in frontend rest-types** - `4a32153` (feat)

_TDD-Task 1: RED (test) -> GREEN (feat). Kein Refactor noetig._

## Files Created/Modified
- `genossi_mail/src/dao.rs` - `CommunicationEntry` um zwei Outbound-Felder erweitert.
- `genossi_mail/src/dao_sqlite.rs` - `CommunicationEntryDb` + `TryFrom` + beide SELECTs (member/application) + 2 DAO-Tests.
- `genossi_mail/src/communication_rest.rs` - Backend-`CommunicationEntryTO` + `From`-Mapping.
- `genossi-frontend/rest-types/src/lib.rs` - Frontend-`CommunicationEntryTO` gespiegelt + 2 serde-Tests.

## Decisions Made
- **Test-Seeding via direktes SQL-UPDATE:** `MailRecipientDaoSqlite::create()` schreibt `rendered_body`/`rendered_html_body` hardcodiert als NULL (Render-Werte entstehen erst beim Versand). Der Positive-Path-Test setzt sie daher per `UPDATE mail_recipients SET rendered_body = ?, rendered_html_body = ?` — spiegelt den realen Persistenz-Zeitpunkt (Muster wie die bestehenden soft-delete-Tests). Das erste RED-Testdesign nutzte `recipient.rendered_body = Some(...)` vor `create()` und schlug fehl (Wert wird von create verworfen); korrigiert im GREEN-Schritt.
- **Member-Pfad nur um SQL-Konsistenz gleichgezogen:** Da beide Queries dieselbe FromRow-Struct teilen, musste `get_member_communications` die zwei Spalten ebenfalls liefern (inbound `NULL`, outbound `r.rendered_*`). Der Member-Body-View selbst bleibt Scope spaeterer Plaene.

## Deviations from Plan

None - plan executed exactly as written. Die Test-Seeding-Korrektur (SQL-UPDATE statt create-Feld) ist eine Anpassung innerhalb des geplanten TDD-Flows von Task 1, keine Abweichung vom Plan-Scope.

## Issues Encountered
- **`create()` persistiert rendered_* nicht:** Der erste GREEN-Run schlug fehl (rendered_body las als `None` zurueck), weil `MailRecipientDaoSqlite::create()` diese Spalten immer NULL schreibt. Behoben durch Seeding via direktem SQL-UPDATE im Test. Danach 311/311 genossi_mail-Tests und 24/24 rest-types-Tests gruen.

## User Setup Required
None - keine externe Service-Konfiguration erforderlich (reine additive Feld-Erweiterung, keine neuen Pakete, keine Migration).

## Next Phase Readiness
- Wave-1-Voraussetzung fuer das Body-Detail-Panel (Plan 32-04) ist erfuellt: der Application-Communications-Endpoint liefert den echten gespeicherten Body.
- Beide `rest-types`-Crates tragen die Felder wire-kompatibel (Landmine 1 geschlossen).
- Threat-Register: T-32-01/T-32-03 bleiben `mitigate` — Feld reist nur ueber den bereits admin-gated Handler, kein neuer Endpoint, kein Re-Render, keine neue Injektionsflaeche.

## Self-Check: PASSED

Alle geaenderten Dateien und Task-Commits (b651345, 8ec2f04, 4a32153) auf Platte verifiziert.

---
*Phase: 32-frontend-compose-dialog*
*Completed: 2026-08-21*
