---
phase: 30-application-template-kontext-antragsteller-vorlagen
plan: 02
subsystem: database
tags: [mail-templates, sqlite, migration, sqlx, axum, seed, template_type]

# Dependency graph
requires:
  - phase: 30-01
    provides: format_eur_de (Euro-Formatierung — im selben Phase-Scope, hier nicht direkt genutzt)
provides:
  - "mail_templates.template_type TEXT NOT NULL DEFAULT 'member' (Pool-Diskriminator, additive Migration)"
  - "template_type auf Entity/DB-Row/TO/Create-Request/Service-Create-Pfad durchgefädelt"
  - "MailTemplateService::create nimmt template_type: &str entgegen (unveränderlich nach Anlegen)"
  - "Seed-Vorlage 'Zahlungserinnerung' (UUID …0003, template_type 'application')"
affects: [30-03, phase-32-mitglieder-selektor, mail-template-crud]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Additive, forward-only ALTER TABLE ADD COLUMN mit NOT NULL DEFAULT für Legacy-Roundtrip"
    - "Immutable-after-create Spalte: INSERT + alle SELECTs tragen sie, UPDATE lässt sie bewusst aus"
    - "In-Memory-DAO-Roundtrip-Test aus echten Migrations-Dateien via include_str! + sqlx::raw_sql"

key-files:
  created:
    - migrations/sqlite/20260820000000_mail_templates_add_template_type.sql
    - migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs
    - genossi_mail/src/rest_templates.rs
    - genossi_mail/src/mail_template_service.rs
    - genossi_bin/tests/e2e_tests.rs

key-decisions:
  - "template_type ist nach dem Anlegen unveränderlich (Pitfall 3, Option a): UPDATE-SQL schreibt die Spalte nicht, update() trägt existing.template_type.clone() weiter"
  - "CreateMailTemplateRequest.template_type ist optional mit #[serde(default = default_template_type)] → 'member' (Open Question Q1: bestehende JSON-Posts bleiben abwärtskompatibel)"
  - "Kein SQLx-offline-prepare nötig — mail_templates-Queries sind Runtime-query/query_as, keine query!-Makros"
  - "DAO-Roundtrip-Test baut den Pool aus den echten Migrations-Dateien (kein handgepflegtes Test-DDL), prüft damit die Migration selbst"

patterns-established:
  - "Pool-Diskriminator-Spalte: additive NOT NULL DEFAULT + immutable-after-create + Read-Pfad-Exposure als Grundierung für einen späteren Selektor-Filter"

requirements-completed: [APTPL-01, APTPL-03]

# Coverage metadata
coverage:
  - id: D1
    description: "mail_templates trägt template_type (DEFAULT 'member'); Legacy-Zeile liest 'member', 'application' roundtrippt"
    requirement: "APTPL-01"
    verification:
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#mail_template_type_tests::mail_template_legacy_row_reads_back_member"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/dao_sqlite.rs#mail_template_type_tests::mail_template_application_roundtrip"
        status: pass
    human_judgment: false
  - id: D2
    description: "Create-Request akzeptiert optionales template_type (default 'member'); TO exponiert template_type; 'application' vs 'member' auf dem Draht"
    requirement: "APTPL-01"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#mail_template_type_application_roundtrips_on_wire"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/mail_template_service.rs#tests::test_create_success"
        status: pass
    human_judgment: false
  - id: D3
    description: "Seed 'Zahlungserinnerung' (UUID …0003, template_type 'application') existiert nach echten Migrationen; Body nutzt nur Antragsteller-Kontext-Keys"
    requirement: "APTPL-03"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#mail_template_predefined_present_after_migration"
        status: pass
    human_judgment: false

# Metrics
duration: 25 min
completed: 2026-08-20
status: complete
---

# Phase 30 Plan 02: template_type-Pooltrennung + Zahlungserinnerung-Seed Summary

**Additive `template_type`-Spalte (DEFAULT 'member', immutable nach Anlegen) durch mail_templates-Migration, DAO, TO, Create-Request und Service gefädelt plus deutscher Antragsteller-Seed 'Zahlungserinnerung' (UUID …0003, 'application').**

## Performance

- **Duration:** 25 min
- **Started:** 2026-08-20T09:00:00Z
- **Completed:** 2026-08-20T09:25:38Z
- **Tasks:** 3
- **Files modified/created:** 7 (2 Migrationen neu, 4 Rust-Quellen, 1 e2e-Testdatei)

## Accomplishments
- Additive Migration `20260820000000` fügt `mail_templates.template_type TEXT NOT NULL DEFAULT 'member'` hinzu — Legacy-Zeilen (2 Seeds + Vorstands-Vorlagen) lesen verlustfrei 'member' zurück.
- `template_type` durch alle 8 Produktionsstellen gefädelt: Entity (`MailTemplate`), DB-Row (`MailTemplateDb`), TryFrom, INSERT und 4 SELECTs; UPDATE bleibt bewusst ohne die Spalte (unveränderlich nach Anlegen, Pitfall 3).
- Read-/Create-Pfad exponiert/akzeptiert `template_type`: `MailTemplateTO` + `CreateMailTemplateRequest` (optional, default 'member'), `MailTemplateService::create` nimmt den Wert entgegen und fädelt ihn in die Create-Entität.
- Seed 'Zahlungserinnerung' (UUID `00000000-0000-0000-0000-000000000003`, `template_type='application'`) mit formellem deutschen Sie-Body, der ausschließlich Antragsteller-Kontext-Keys mit guarded `bank_bic` verwendet.

## Task Commits

Jede Aufgabe wurde atomar committet:

1. **Task 1: template_type-Spalte + Migration + DAO-Threading** - `b299266` (feat)
2. **Task 2: TO + Create-Request + Service-Threading** - `0f4b4b4` (feat)
3. **Task 3: Seed 'Zahlungserinnerung' Antragsteller-Vorlage** - `03245ea` (feat)

## Files Created/Modified
- `migrations/sqlite/20260820000000_mail_templates_add_template_type.sql` - additive Spalte, forward-only
- `migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql` - Antragsteller-Seed …0003
- `genossi_mail/src/dao.rs` - `MailTemplate.template_type: Arc<str>`
- `genossi_mail/src/dao_sqlite.rs` - `MailTemplateDb.template_type: String`, TryFrom, INSERT/SELECTs, 2 Roundtrip-Tests
- `genossi_mail/src/rest_templates.rs` - `MailTemplateTO`/`CreateMailTemplateRequest`-Feld, `default_template_type`, From-Impl, Handler
- `genossi_mail/src/mail_template_service.rs` - `create(template_type: &str)`, update trägt bestehenden Wert, Unit-Tests
- `genossi_bin/tests/e2e_tests.rs` - Wire-Assertions (application vs default member) + Seed-Präsenz

## Decisions Made
- **Immutable-after-create:** `template_type` wird nie per UPDATE geändert; `update()` trägt `existing.template_type.clone()` weiter, damit die zurückgegebene Entität konsistent bleibt.
- **Optional mit Default auf dem Draht:** `CreateMailTemplateRequest.template_type` ist `#[serde(default = "default_template_type")]` → 'member'; bestehende Clients ohne das Feld bleiben abwärtskompatibel (Open Question Q1).
- **Kein SQLx-offline-prepare:** die mail_templates-Zugriffe sind Runtime-`query`/`query_as`, keine `query!`-Makros — keine Metadaten-Regeneration nötig.
- **Test aus echten Migrationen:** der DAO-Roundtrip-Test baut den In-Memory-Pool via `include_str!` der echten Migrations-SQL + `sqlx::raw_sql`, prüft also die Migration selbst statt eines Test-DDL (D-02: kein mail_templates-Test-DDL vorhanden).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] template_type-Feld in Service-Literalen bereits in Task 1**
- **Found during:** Task 1 (Entity-Erweiterung)
- **Issue:** Das Hinzufügen des Nicht-Option-Feldes `template_type` zur `MailTemplate`-Entity bricht sofort die Kompilierung aller `MailTemplate { ... }`-Literale in `mail_template_service.rs` (Create-, Update- und 3 Test-Mock-Literale). Die Plan-`<files>`-Liste ordnete `mail_template_service.rs` erst Task 2 zu, aber ohne diese Literale kompiliert genossi_mail nach Task 1 nicht und die Task-1-Verifikation (`cargo test -p genossi_mail mail_template`, die Kompilierung voraussetzt) kann nicht laufen.
- **Fix:** In Task 1 die Service-Literale ergänzt — Create-Literal setzte vorläufig `template_type: Arc::from("member")`, Update-Literal direkt die Finalform `existing.template_type.clone()`, Test-Mocks `Arc::from("member")`. Task 2 hat das Create-Literal dann auf den durchgefädelten `template_type`-Parameter umgestellt.
- **Files modified:** genossi_mail/src/mail_template_service.rs
- **Verification:** `nix develop --command cargo test -p genossi_mail mail_template` grün nach Task 1 (11 Tests) und nach Task 2.
- **Committed in:** `b299266` (Task-1-Commit) bzw. `0f4b4b4` (Task-2-Umstellung)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Notwendige Cross-Task-Kompilierungsfolge des Entity-Feldes; kein Scope-Creep. Jeder Commit kompiliert und besteht die Tests.

## Issues Encountered
- Erster Kompilierlauf des DAO-Roundtrip-Tests scheiterte an `pool.as_ref()` (Pool war an der Stelle noch nicht Arc-gewrappt) → auf `&pool` korrigiert. Sofort behoben, kein Folgeeffekt.

## User Setup Required
None - keine externe Service-Konfiguration nötig. Die Migrationen laufen beim Serverstart automatisch (sqlx migrate).

## Next Phase Readiness
- Datenfundament für die Pooltrennung steht: Plan 30-03 kann `application_to_template_context` bauen und den Render-Sicherheits-Nachweis (`validate_application_template`) gegen den seeded Body führen.
- Der Mitglieder-Selektor-Filter (`template_type = 'member'`) ist bewusst nicht Teil dieses Plans (Phase 32, D-03) — die Grundierung (Read-Pfad exponiert template_type) ist vorhanden.

## Self-Check: PASSED
- `migrations/sqlite/20260820000000_mail_templates_add_template_type.sql` — FOUND
- `migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql` — FOUND
- Commits `b299266`, `0f4b4b4`, `03245ea` — vorhanden im git log
- `nix develop --command cargo test -p genossi_mail` — 290 passed, 0 failed
- `nix develop --command cargo test -p genossi_bin --test e2e_tests mail_template` — 5 passed, 0 failed

---
*Phase: 30-application-template-kontext-antragsteller-vorlagen*
*Completed: 2026-08-20*
