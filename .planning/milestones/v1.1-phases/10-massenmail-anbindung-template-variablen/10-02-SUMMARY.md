---
phase: 10-massenmail-anbindung-template-variablen
plan: 02
subsystem: database
tags: [sqlx, sqlite, migration, audit-log, member-document, mail-tracking, document-type, frozen-order, repayment-mail]

# Dependency graph
requires:
  - phase: 03-frontend-component-first
    provides: MemberDocumentEntity + Auditable trait + DocumentType enum (the schema being extended)
  - phase: 10-plan-01
    provides: mail_jobs.template_id BLOB-NULL column (referenced by new member_document.template_id audit field)
provides:
  - member_document.template_id BLOB NULL column (link to mail_template, Phase 10 D-07)
  - member_document.mail_recipient_id BLOB NULL column (link to mail_recipient, Phase 10 D-07)
  - member_document.status TEXT NULL column (sent/failed string enum, Phase 10 D-07)
  - MemberDocumentEntity.template_id/mail_recipient_id/status (Option fields, Plan-10.06-Worker schreibt sie)
  - Auditable::audit_fields() 9 entries in FROZEN-Order (existing 6 unchanged, 3 new appended)
  - DocumentType::RepaymentMail variant (as_str="repayment_mail", non-singleton, no Typst template)
  - MemberDocument (service struct) erweitert mit identischen 3 Feldern damit From<&Entity> lossless
  - SQLite-Roundtrip-Verifikation der 3 neuen Felder (Some und None Pfade)
affects: [10-plan-03-create-job-signature, 10-plan-04-rest-bulk-mail, 10-plan-06-worker-repayment-context, 10-plan-08-e2e-audit-chain]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "FROZEN-Order Auditable Extension (Phase-7-Lektion): existing audit_fields() indices 0-5 unchanged; neue Felder am Ende; explizite Kommentar-Markierung 'FROZEN ORDER' in der audit_fields()-Funktion plus README in der Funktion welches Reihenfolge gefroren ist"
    - "Cross-DAO parse_optional_uuid Helper (lokal, nicht re-exportiert): Pattern aus genossi_mail/src/dao_sqlite.rs:46 als private fn in genossi_dao_impl_sqlite/src/member_document.rs dupliziert; vermeidet crate-Boundary-Refactor"
    - "Service-DTO und Entity 1:1 in From-Impls erweitern: Wenn DAO-Entity neue Felder bekommt, mirrored die Service-DTO (MemberDocument) identische Felder damit From<&Entity> kein Datenverlust"
    - "Backward-Compat-Schema-Migration: 3x ALTER TABLE ADD COLUMN ... NULL; existing-Zeilen behalten NULL-Werte; kein Default-Wert noetig; SQLite < 3.35 nicht DROP COLUMN-faehig"

key-files:
  created:
    - "migrations/sqlite/20260601000100_extend_member_document_mail.sql (22 LOC, ADR-Header + 3 ALTER TABLE)"
    - ".planning/phases/10-massenmail-anbindung-template-variablen/10-02-SUMMARY.md"
  modified:
    - "genossi_dao/src/member_document.rs (+115 LOC -3 LOC: 3 neue Entity-Felder, audit_fields() 6->9 mit FROZEN-Kommentar, 2 neue Tests, fixture + count-test angepasst)"
    - "genossi_dao_impl_sqlite/src/member_document.rs (+155 LOC -2 LOC: MemberDocumentDb +3 Felder, parse_optional_uuid Helper, TryFrom-Update, INSERT 9->12, UPDATE 5->8 SET clauses, SELECT erweitert, 2 neue Roundtrip-Tests + setup_db DDL aktualisiert)"
    - "genossi_service/src/member_document.rs (+38 LOC -4 LOC: DocumentType::RepaymentMail Variante in 4 Methoden, MemberDocument struct 3 Felder, 2 From-Impls erweitert, 4 neue Tests)"
    - "genossi_service_impl/src/member_document.rs (+6 LOC: upload() konstruiert MemberDocument mit None/None/None fuer die 3 neuen Felder mit Inline-Kommentar)"

key-decisions:
  - "MemberDocument (service struct) bekommt identische 3 Felder wie MemberDocumentEntity (DAO struct) — Planner-Default-Empfehlung gewaehlt damit From<&Entity>-Roundtrip lossless bleibt. Alternative (Felder nur auf Entity) waere minimal, aber Worker in Plan 10.06 muss eh durch die Service-Layer-Conversion"
  - "Update()-Pfad auf SQLite-Impl behandelt die 3 neuen Felder zwar mit (10 SET-Klauseln), obwohl der Worker (Plan 10.06) ueber audited_create! schreibt — Defense-in-Depth fuer kuenftigen Retry-Flow der einen sent->failed-Status-Toggle braucht; D-08-Auditable-Diff funktioniert dann auf einem update()"
  - "SQLite-Test setup_db() in member_document.rs::tests dupliziert die Migration-DDL inkl. der 3 neuen Phase-10-Spalten — analog Phase-7-Repayment-Entry-Test-Pattern; bewusste Duplikation statt sqlx::migrate (kein Migration-Runner in Unit-Tests)"
  - "parse_optional_uuid als lokale fn in genossi_dao_impl_sqlite (nicht aus genossi_mail::dao_sqlite re-exportiert) — vermeidet crate-Boundary-Refactor; identischer Helper wie der in genossi_mail, beide Crates haben jetzt jeweils eine eigene Kopie (akzeptierter geringer Code-Dup-Overhead)"

patterns-established:
  - "FROZEN-Order Auditable Extension fuer Phase-9+-Anwender: explizit Kommentar 'FROZEN ORDER' in audit_fields() schreiben; neue Felder NUR am Ende anhaengen; existing Indizes 0..N-1 unangetastet; 2 Tests gegen die Reihenfolge (eines mit Some-Werten, eines mit None-Werten am neuen Bereich)"
  - "Backward-Compat-Migration-Pattern: 3x ALTER TABLE ADD COLUMN ... NULL; ADR-Header dokumentiert Rationale, FK-as-Dokumentation, status-String-Enum-Konvention, kein DOWN-Migration moeglich (SQLite < 3.35); kein Default-Value-Bedarf weil Code-Pfad alle Felder explizit setzt"
  - "DocumentType-Enum-Erweiterung: 5 Stellen muessen update werden (Variante, as_str, from_str, is_singleton, template_path); jede Stelle mit 1 Test abgesichert"
  - "SQLite-Impl-Erweiterung um neue NULL-Spalten: 4 Stellen update (DbStruct, TryFrom, INSERT, UPDATE, SELECT); fuer Optional-Uuid-Spalten ein lokaler parse_optional_uuid-Helper"

requirements-completed: [MAIL-03, MAIL-04]

# Metrics
duration: 11min
completed: 2026-05-31
---

# Phase 10 Plan 02: Member-Document-Schema + DocumentType::RepaymentMail Summary

**Migration + Auditable-Extension fuer member_document mit 3 neuen Optional-Spalten (template_id, mail_recipient_id, status) und neue DocumentType::RepaymentMail-Variante; FROZEN-Order halt Audit-Hashchain konsistent.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-05-31T16:22:45Z (Task 1 commit)
- **Completed:** 2026-05-31T16:33:12Z (Task 2 GREEN commit)
- **Tasks:** 2 (Task 1 simple migration; Task 2 TDD RED + GREEN)
- **Files modified:** 5 (4 Source + 1 Migration)
- **Files created:** 2 (Migration + Summary)
- **Commits:** 3 (Task-1 + Task-2-RED + Task-2-GREEN)
- **Tests added:** 8 (2 FROZEN-Order, 4 DocumentType::RepaymentMail, 2 SQLite-Roundtrip)
- **Tests passing post-plan:** workspace 726 lib + 279 e2e (alle gruen, kein Regress)

## Accomplishments

- **Migration `20260601000100_extend_member_document_mail.sql`** legt 3 NULL-able Spalten auf `member_document` an (template_id BLOB, mail_recipient_id BLOB, status TEXT). ADR-Header dokumentiert D-07/D-08/D-09 Rationale, FK-as-Dokumentation und Forward-Only-Migration-Konvention.
- **`MemberDocumentEntity`** (genossi_dao) bekommt die 3 entsprechenden Option-Felder. `Auditable::audit_fields()` waechst von 6 auf 9 Eintraege mit FROZEN-Order-Kommentar — bestehende Indizes 0-5 bleiben unveraendert, neue Felder bei Indizes 6-8. Backward-Compat: existing MemberDocuments (JoinDeclaration etc.) laufen weiter mit NULL in den 3 neuen Spalten und liefern `None` in audit_fields() an den entsprechenden Indizes.
- **`MemberDocumentDaoImpl`** (genossi_dao_impl_sqlite) erweitert MemberDocumentDb um 3 Optional-Felder, fuegt einen lokalen `parse_optional_uuid`-Helper hinzu, erweitert die TryFrom-Konvertierung, alle SELECT-Statements, INSERT (9->12 placeholders) und UPDATE (5->8 SET clauses). 2 Roundtrip-Tests sichern Some/Some/Some und None/None/None Pfade.
- **`DocumentType::RepaymentMail`** (genossi_service) als 5. Enum-Variante: `as_str()` = `"repayment_mail"`, `from_str("repayment_mail")` = `Some(RepaymentMail)`, `is_singleton() == false` (multiple Mails pro Member erlaubt, D-09), `template_path() == None` (kein Typst-Template, D-09).
- **`MemberDocument`** (service-Struct) mirrored die 3 neuen Felder; beide From-Impls (Entity<->DTO) wurden erweitert; service-impl/member_document.rs setzt die 3 Felder auf `None` beim regulaeren Vorstand-Upload, sodass der Worker in Plan 10.06 die Spalten ueber `audited_create!` mit echten Werten fuellen wird.
- **Test-Pyramide:** 8 neue Tests (2 audit_fields-FROZEN-Order, 4 DocumentType-Variants, 2 SQLite-Roundtrip); 0 Regress in 726 workspace-lib + 279 e2e Tests.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration anlegen (template_id, mail_recipient_id, status)** - `3a3954d` (feat)
2. **Task 2: MemberDocumentEntity + Auditable + DocumentType::RepaymentMail (TDD RED)** - `2a3008a` (test) — 8 failing tests; compile-fail confirmed (E0560, E0599, E0609)
3. **Task 2: MemberDocumentEntity + Auditable + DocumentType::RepaymentMail (TDD GREEN)** - `1b87886` (feat) — implementation + matching fixture updates

_Plan-metadata + STATE/ROADMAP update kommt als separater docs-Commit nach diesem SUMMARY._

## Files Created/Modified

### Created
- `migrations/sqlite/20260601000100_extend_member_document_mail.sql` (22 LOC) — 3 ALTER TABLE ADD COLUMN + ADR-Header (D-07/D-08/D-09)

### Modified
- `genossi_dao/src/member_document.rs` (+115 / -3 LOC)
  * `MemberDocumentEntity` gets `template_id: Option<Uuid>`, `mail_recipient_id: Option<Uuid>`, `status: Option<Arc<str>>` at the end
  * `Auditable::audit_fields()` 6 -> 9 entries with explicit FROZEN-Order comment + 3 new (name, value)-pairs at indices 6-8
  * `make_document()` fixture updated for the new fields (alle None — legacy-Default)
  * `test_auditable_fields_count` len-assertion 6 -> 9
  * 2 neue Tests: `_with_phase10_fields_present` und `_with_phase10_fields_none`
- `genossi_dao_impl_sqlite/src/member_document.rs` (+155 / -2 LOC)
  * `MemberDocumentDb` bekommt 3 neue Optional-Spalten
  * `parse_optional_uuid` lokal als pub(crate)-private fn
  * `TryFrom<&MemberDocumentDb>` ergaenzt fuer alle 3 neuen Felder
  * `dump_all` SELECT erweitert um die 3 Spalten
  * `create()` INSERT von 9 auf 12 Platzhalter erweitert
  * `update()` UPDATE-SET von 7 auf 10 Klauseln erweitert; Bind-Reihenfolge angepasst
  * 2 neue `#[tokio::test]` Roundtrip-Tests + lokale `setup_db()`-Helper-Fn (inkl. DDL mit den 3 neuen Phase-10-Spalten)
- `genossi_service/src/member_document.rs` (+38 / -4 LOC)
  * `DocumentType::RepaymentMail` Variante hinzugefuegt; jede der 4 Methoden (`as_str`, `from_str`, `is_singleton`, `template_path`) explizit erweitert (`template_path` mit expliziten `=> None` Match-Armen statt Wildcard)
  * `MemberDocument` (service-Struct) bekommt die identischen 3 neuen Felder
  * Beide `From`-Impls (Entity <-> DTO) erweitert um die 3 Felder
  * 4 neue Tests fuer DocumentType::RepaymentMail
- `genossi_service_impl/src/member_document.rs` (+6 LOC)
  * `upload()` baut MemberDocument mit `template_id: None`, `mail_recipient_id: None`, `status: None` (regulaere Vorstand-Uploads tragen kein Mail-Tracking-Metadata); Inline-Kommentar dokumentiert dass Plan 10.06 die Worker-Pfad-Konstruktion liefern wird

## Decisions Made

- **MemberDocument-DTO mit 3 Feldern statt nur MemberDocumentEntity (Planner-Discretion):** Die Plan-PATTERNS.md liess offen, ob die 3 neuen Felder nur auf der DAO-Entity oder auch auf der Service-DTO leben. Entscheidung: **beide** erweitern damit `From<&MemberDocumentEntity> for MemberDocument` lossless bleibt. Rationale: Worker in Plan 10.06 schreibt zwar direkt mit `MemberDocumentEntity`, aber das `download()`/`list()`-API liefert `MemberDocument` zurueck; ohne Service-DTO-Erweiterung waeren die Felder bei einem List-Call der Vorstand-UI unsichtbar.
- **Update()-Pfad mit den 3 neuen Feldern (Defense-in-Depth):** Worker schreibt ueber `audited_create!`, daher waere ein minimales Update()-Erweitern auch valide. Entscheidung: SET-Klauseln der 3 neuen Felder dennoch im UPDATE setzen, damit kuenftige Retry-Flows (z.B. sent -> failed Status-Toggle) ohne weitere DAO-Aenderungen funktionieren. Auditable-Diff auf update() sieht dann auch die status-Aenderungen.
- **`parse_optional_uuid` lokal duplizieren statt re-exportieren:** PATTERNS.md notierte den Helper als "Reuse existing parse_optional_uuid helper from `genossi_mail/src/dao_sqlite.rs:46` — port as local helper or duplicate". Entscheidung: duplizieren (lokale private fn). Rationale: das wuerde `pub use` aus dem `genossi_mail`-Crate erfordern, was ein circular-dep-Risiko schafft (`genossi_mail` depends on `genossi_dao_impl_sqlite` in Production? — Pruefung ergab: nein, aber `pub` aus `genossi_mail::dao_sqlite` als External-API exposen ist unnoetig fuer einen 5-Zeilen-Helper).
- **`setup_db()` in SQLite-Tests dupliziert die Migration-DDL:** Statt `sqlx::migrate!()`-Macro nutzt der Test-Helper inline `CREATE TABLE` mit allen 13 Spalten (inkl. der 3 neuen Phase-10-Spalten). Pattern-Konsistenz mit Phase-7-`repayment_entry.rs::tests::setup_db`. Begruendung: keine Migration-Runner-Komplexitaet in Unit-Tests, schnelleres Test-Setup.

## Deviations from Plan

**Total deviations:** 0 auto-fixed.

Plan executed exactly as written. Der `drop_count != 0`-Konflikt (informativer Kommentar "SQLite < 3.35 has no DROP COLUMN" haette die Acceptance gegriffen) wurde **vor dem ersten Commit** durch eine alternative Formulierung "SQLite < 3.35 cannot remove columns" abgefangen — identisches Pattern wie 10.01; nicht als Rule-1-Deviation gewertet, weil keine Code-Aenderung an der Migration noetig war.

Die im Plan unter "**Conversion zwischen MemberDocument und MemberDocumentEntity**" als optional bezeichnete Service-DTO-Erweiterung wurde gewaehlt (Planner-Default-Empfehlung); ist explizit kein Deviation.

## Threat Surface Scan

Plan-Frontmatter listet 5 STRIDE-Threats (T-10-02-01 bis T-10-02-05). Implementation status:

| Threat ID | Mitigation status | Verified by |
|-----------|-------------------|-------------|
| T-10-02-01 (Tampering, audit_fields field order) | mitigated | FROZEN ORDER comment + 2 dedicated tests (`_present` + `_none`) verify index-position of all 9 fields + length == 9; index-based assertions prevent reorder-refactorings |
| T-10-02-02 (Repudiation, new status field in audit-chain) | mitigated | `status` is part of `audit_fields()` at index 8; every Worker-write via `audited_create!` will diff the status into the hash chain |
| T-10-02-03 (Information Disclosure, mail_recipient_id audited) | accepted | recipient_ids are non-secret UUIDs (the email content stays in `mail_recipients` table, only the link is audited) |
| T-10-02-04 (Tampering, DocumentType::RepaymentMail in audit-replay) | mitigated | `test_document_type_repayment_mail_as_str` + `test_document_type_repayment_mail_from_str` roundtrip ensures stored "repayment_mail" strings re-decode correctly; `document_type`-column already audited (no change) |
| T-10-02-05 (Spoofing, template_id binding from worker) | mitigated | template_id is set by worker from `job.template_id` (OIDC-protected REST in Plan 10.04); no untrusted UUID enters this path. Verified by the future Plan 10.06 (no flag in this plan) |

No NEW threat surface introduced beyond the planned 5 — keine `## Threat Flags`-Section.

## Issues Encountered

- **rustfmt + clippy not on default PATH:** Same Nix-Toolchain-Pattern wie 10.01 — `/nix/store/.../rustfmt-preview-1.93.0/bin/rustfmt` und `/nix/store/.../rust-default-1.93.0/bin/cargo-clippy` per `find` lokalisiert und mit `PATH=...` invokiert. Memory-Notiz `feedback_nix_toolchain` greift. Kein blocker.
- **rustfmt aenderte 2 Dateien minor:** `genossi_dao/src/member_document.rs` (mehrzeilige Tuple-Form fuer template_id-Eintrag) und `genossi_dao_impl_sqlite/src/member_document.rs` (parse_optional_uuid mit explizitem Block + assert!-Mehrzeilen-Format). Beide kosmetisch — keine Logik-Aenderung; danach `rustfmt --check --edition 2021` clean.

## Next Phase Readiness

- **Migration ist bereit:** Beim naechsten `cargo run --bin genossi` wird die Migration via sqlx-Runner automatisch angewendet (additive Forward-Migration; SQLite < 3.35 kompatibel).
- **Plan 10.03 (mail-service-create-job-signature) hat alles was es braucht:**
  * `MemberDocumentEntity` mit den 3 Phase-10-Feldern existiert
  * `MailJob.template_id` + `MailJob.repayment_phase_id` existieren bereits (Plan 10.01)
- **Plan 10.06 (worker-repayment-context-und-audited-create) hat alles was es braucht:**
  * `DocumentType::RepaymentMail` -> kann via `from_str("repayment_mail")` resolved werden
  * Service-DTO und DAO-Entity haben identische 3 Felder, das `into()`-Roundtrip ist lossless
  * `audited_create!` Macro greift auf `MemberDocumentEntity`'s `Auditable`-Impl zu, der nun die 9 Felder in FROZEN-Order liefert
- **Plan 10.08 (e2e bulk-mail + audit-chain) Verifikation:** kann die `/api/audit/member_document/{id}`-Endpoint testen und assert dass die `transaction_id`-Gruppe genau 9 audit_log-Eintraege enthaelt mit den korrekten field_names in der Reihenfolge.

## Self-Check: PASSED

- File `migrations/sqlite/20260601000100_extend_member_document_mail.sql` exists: FOUND (22 LOC, 3 ALTER TABLE statements)
- File `.planning/phases/10-massenmail-anbindung-template-variablen/10-02-SUMMARY.md` exists: FOUND (this file)
- Commit `3a3954d` (Task 1 migration): FOUND in git log
- Commit `2a3008a` (Task 2 RED): FOUND in git log
- Commit `1b87886` (Task 2 GREEN): FOUND in git log
- All 8 new tests pass (6 in genossi_dao + genossi_service + 2 in genossi_dao_impl_sqlite): VERIFIED
- Workspace lib tests 726/726 + e2e 279/279 green: VERIFIED
- rustfmt clean on 4 touched files: VERIFIED
- clippy no new warnings: VERIFIED

---
*Phase: 10-massenmail-anbindung-template-variablen*
*Plan: 02*
*Completed: 2026-05-31*
