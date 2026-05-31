---
phase: 10-massenmail-anbindung-template-variablen
verified: 2026-05-31T18:45:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
---

# Phase 10: Massenmail-Anbindung + Template-Variablen — Verifikationsbericht

**Phase-Ziel:** Vorstand kann mehrere RepaymentEntries gleichzeitig anschreiben; Mail-Templates haben Zugriff auf Auszahlungs-Wert (payout_amount, share_count, fiscal_year). Wiederverwendet POST /api/mail/send-bulk, kein neuer Endpoint.
**Verifiziert:** 2026-05-31T18:45:00Z
**Status:** PASSED
**Re-Verifikation:** Nein — initiale Verifikation

## Ziel-Erreichung

### Observable Truths

| # | Truth | Status | Evidenz |
|---|-------|--------|---------|
| 1 | SC#1: POST /api/mail/send-bulk akzeptiert optional template_id + repayment_phase_id im Body | VERIFIED | `genossi_mail/src/rest.rs:128,134` — `pub template_id: Option<String>` und `pub repayment_phase_id: Option<String>` in `SendBulkMailRequest`; UUID-Parsing mit 400-BadRequest-Handling (Z. 418-435); OpenAPI-Schema-Registration vorhanden (Z. 258) |
| 2 | SC#2: minijinja-Template-Engine löst payout_amount, share_count, fiscal_year korrekt auf | VERIFIED | `merge_repayment_context` in `genossi_mail/src/template.rs:139`; Worker-Aggregation mit D-06-Filter (Open/Contacted, deleted IS NULL) in `worker.rs:332-358`; German-Locale-Formatierung `format!("{},{:02}", cents / 100, cents % 100)` Z. 353; 4 Unit-Tests in `template.rs` grün |
| 3 | SC#3: Pro versendeter Mail entsteht 1 MemberDocument (document_type=repayment_mail) mit template_id, mail_recipient_id, status=sent/failed | VERIFIED | `build_member_document_entity` in `worker.rs:34-73` erzeugt Entity mit `document_type=Arc::from("repayment_mail")`, `template_id=job.template_id`, `mail_recipient_id=Some(recipient_id)`, Status-Feld; `try_create_member_document_audited` via inline Audit-Pattern (`worker_audit::build_create_entries`); E2E-Test E1 bestätigt 3 MemberDocuments für 3 Recipients |
| 4 | SC#4: SMTP-Fehler bei einzelnem Empfänger führt zu status=failed; übrige Empfänger werden verarbeitet; E2E-Test vorhanden | VERIFIED | Fail-tolerance bewahrt (`REPAYMENT_MAIL_PROCESS`-try_create bricht bei Fehler NICHT den Worker-Run ab); E2E-Test E2 bestätigt All-or-Nothing-Freiheit (3 MemberDocuments inkl. 1 AddressError + 2 ConnectionRefused); `[FAILED:]`-Suffix mit 200-Zeichen-Truncation in Z. 45-47 |

**Score: 4/4 Truths verifiziert**

## Erforderliche Artefakte

| Artefakt | Erwartet | Status | Details |
|----------|----------|--------|---------|
| `genossi_mail/src/rest.rs` | SendBulkMailRequest + send_bulk_mail Handler | VERIFIED | Felder `template_id`, `repayment_phase_id` vorhanden; UUID-Parsing; validate_template_with_repayment-Branching; OpenAPI-Schema |
| `genossi_mail/src/template.rs` | merge_repayment_context + validate_template_with_repayment + 4 Unit-Tests | VERIFIED | Beide Funktionen vorhanden (Z. 139, Z. 181); 4 + 1 Tests grün |
| `genossi_mail/src/worker_audit.rs` | compute_entry_hash + build_audit_entries + build_create_entries (KEINE worker_audited_create-Wrapper-Fn) | VERIFIED | Alle 3 pure Helpers vorhanden (Z. 31, Z. 77, Z. 140); kein High-Level-Wrapper; sha2-Dep in Cargo.toml; kein circular dep zu genossi_service_impl |
| `genossi_mail/src/worker.rs` | REPAYMENT_MAIL_PROCESS + WORKER_USER_ID + build_member_document_entity + try_create_member_document_audited + merge_repayment_context-Call + 2 Unit-Tests | VERIFIED | Alle Konstanten und Funktionen vorhanden; D-06-Filter; D-05 Edge-Case; inline Audit-Pattern (get_latest_hash + build_create_entries + create_entries) |
| `genossi_bin/tests/e2e_tests.rs` | 5 E2E-Tests + 5 Test-Infrastruktur-Helpers | VERIFIED | Alle 5 Tests bei Z. 12811-13242; alle 5 Helpers bei Z. 12614-12768 |
| `migrations/sqlite/20260601000000_extend_mail_job_template_phase.sql` | mail_jobs-Schema um template_id + repayment_phase_id erweitert | VERIFIED | Beide ALTER TABLE-Statements vorhanden |
| `migrations/sqlite/20260601000100_extend_member_document_mail.sql` | member_document-Schema um template_id + mail_recipient_id + status erweitert | VERIFIED | Alle 3 ALTER TABLE-Statements vorhanden |

## Key Link Verifikation

| Von | Nach | Via | Status | Details |
|-----|------|-----|--------|---------|
| `SendBulkMailRequest.repayment_phase_id` | `MailService::create_job` arg | `uuid::Uuid::parse_str -> Option<Uuid>` | WIRED | `rest.rs:430-436` — parse_str + Übergabe an create_job Z. 446-448 |
| `SendBulkMailRequest.template_id` | `MailService::create_job` arg | `uuid::Uuid::parse_str -> Option<Uuid>` | WIRED | `rest.rs:418-428` — parse_str + Übergabe an create_job Z. 444-445 |
| Worker render-loop | `merge_repayment_context` (Plan 10.05) | `wenn job.repayment_phase_id Some` | WIRED | `worker.rs:355-359` — `ctx = merge_repayment_context(ctx, &payout_amount, share_count, phase.fiscal_year)` |
| Worker post-send | MemberDocument im member_document table | INLINED MemberDocumentDao::create + worker_audit::build_create_entries + AuditLogDao::create_entries | WIRED | `worker.rs:547-640` — `try_create_member_document_audited`; inline Audit-Calls Z. 587-634 |
| Audit-Hashchain | MemberDocument audit entries | `build_create_entries` (worker_audit.rs) | WIRED | `worker.rs:614`: `crate::worker_audit::build_create_entries(...)` nach `get_latest_hash` |
| REST validate_template branching | `validate_template_with_repayment` | `repayment_phase_id.is_some()` | WIRED | `rest.rs:357-370` — Rule-2-Fix aus Plan 10.08: repayment-linked Requests nutzen `validate_template_with_repayment` |

## Data-Flow Trace (Level 4)

| Artefakt | Datenvariable | Quelle | Liefert Realdaten | Status |
|----------|--------------|--------|-------------------|--------|
| `worker.rs:merge_repayment_context-Block` | `payout_amount`, `share_count`, `fiscal_year` | `repayment_phase_dao.find_by_id` + `repayment_entry_dao.find_by_phase_id` (SQLite) | Ja — echte DB-Queries mit D-06-Filter | FLOWING |
| `try_create_member_document_audited` | `MemberDocumentEntity` | `build_member_document_entity` aus Job+Recipient+SendResult | Ja — send_result aus echtem SMTP-Versuch | FLOWING |
| Audit-Hashchain | `prev_hash` | `audit_log_dao.get_latest_hash(tx)` | Ja — liest letzte Hash aus DB | FLOWING |

## Behavioral Spot-Checks (Step 7b)

| Verhalten | Kommando | Ergebnis | Status |
|-----------|----------|----------|--------|
| 5 E2E-Bulk-Repayment-Tests | `cargo test --package genossi_bin --test e2e_tests test_bulk_repayment_mail -- --test-threads=1` | 5 passed; 0 failed (21.09s) | PASS |
| genossi_mail Unit-Tests | `cargo test -p genossi_mail --lib` | 128 passed; 0 failed | PASS |
| Workspace-Build | `cargo build --workspace` | Finished (nur pre-existing Warnings, keine Errors) | PASS |
| Workspace-Tests | `cargo test --workspace` | 284+128+... passed; 0 failed | PASS |

## Anforderungsabdeckung

| Anforderung | Quell-Plan | Beschreibung | Status | Evidenz |
|-------------|------------|--------------|--------|---------|
| MAIL-01 | 10.04, 10.08 | Vorstand wählt mehrere Einträge und löst Massenmail aus (gleiches Pattern wie bestehende Massenmail) | SATISFIED | POST /api/mail/send-bulk erweitert (kein neuer Endpoint); E2E-Test E1 bestätigt Bulk-Send mit repayment_phase_id |
| MAIL-02 | 10.05, 10.06 | Mail-Template kann `{{ payout_amount }}` referenzieren (share_count × phase.share_value) | SATISFIED | `merge_repayment_context` + Worker-Aggregation; German-Locale `X,YZ`-Format; 4 Unit-Tests; E2E-Test E1 verwendet guarded Template |
| MAIL-03 | 10.06, 10.08 | Mail-Template kann `{{ share_count }}` und `{{ fiscal_year }}` referenzieren | SATISFIED | Alle 3 Variablen in `merge_repayment_context` eingebaut (template.rs:139-164); Unit-Tests test_merge_repayment_context_renders_all_three_vars |
| MAIL-04 | 10.06, 10.08 | Mail-Versand erzeugt pro Empfänger ein MemberDocument mit Template-Referenz | SATISFIED | `try_create_member_document_audited` in worker.rs; E2E-Test E1 bestätigt N MemberDocuments für N Recipients; E3 bestätigt Audit-Chain-Integrität (`/api/audit/verify` valid=true) |

## Anti-Pattern-Scan

| Datei | Zeile | Muster | Schwere | Impact |
|-------|-------|--------|---------|--------|
| `genossi_mail/src/rest.rs` | — | Kein TODO-Placeholder aus Plan 10.03 mehr vorhanden | Info | `grep -c "TODO Plan 10.04"` = 0 — sauber |
| Überall | — | `sha2` dep in `genossi_mail/Cargo.toml` vorhanden, kein `genossi_service_impl`-Circular-Dep | Info | Plan-PATTERNS.md-Warnung (CRITICAL FINDING) korrekt umgangen |
| `genossi_bin/src/lib.rs` | 966 | `unused import: genossi_dao::auditable::Auditable` | Warning (pre-existing) | Clippy-Warning; kein Blocker; pre-existing laut SUMMARY |

Keine Blocker-Anti-Pattern gefunden.

## Abgedeckte Sicherheitsaspekte (STRIDE)

- **T-10-06-01 (PII-Leak):** `[FAILED:]`-Description-Truncation auf 200 Zeichen; E2E-Test E4 mit unique PII-Marker `"private-pii@member-data.test"` bestätigt kein Leak in `MemberDocument.description`
- **T-10-06-02 (Hash-Chain-Tampering):** `worker_audit::compute_entry_hash` byte-identisch zu `genossi_service_impl`; Test `test_compute_entry_hash_matches_service_impl_for_known_input` + E2E-Test E3 (`/api/audit/verify` = valid=true) verifiziert Integrität
- **T-10-06-03 (Repudiation):** `WORKER_USER_ID="SYSTEM"` + `REPAYMENT_MAIL_PROCESS="repayment-mail-worker"`; E3 bestätigt `process="repayment-mail-worker"` in Audit-Entries

## Menschliche Verifikation erforderlich

Keine. Alle kritischen Verhaltensweisen wurden programmatisch verifiziert.

**Hinweis zum "sent"-Pfad:** Der echte SMTP-Erfolgspfad (`status='sent'`) ist in den E2E-Tests NICHT abgedeckt (SMTP-Stub via 127.0.0.1:1 = Connection-Refused; deterministische Fehler-Pfade). Dies ist als `T-10-08-01 (accept)` dokumentiert. SC#4 "kein All-or-Nothing" wird via 2 verschiedene Fehler-Subtypen (AddressError vs. ConnectionRefused) verifiziert. Der `sent`-Pfad ist für Phase 12 UAT vorgesehen. Kein Blocker für Phase-10-Ziel.

## Lücken-Zusammenfassung

Keine Lücken. Alle 4 Success Criteria sind durch existierenden, kompilierenden, testbaren Code erfüllt. 

---

_Verifiziert: 2026-05-31T18:45:00Z_
_Verifier: Claude (gsd-verifier)_
