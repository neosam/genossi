---
quick_id: 260603-cz6
date: 2026-06-03
description: Bulk-Mail: RepaymentLetter automatisch als per-Empfänger-Attachment
status: complete
---

# Quick Task 260603-cz6 — SUMMARY

## What changed

Bulk-Mail-Versand kann jetzt pro Empfänger automatisch dessen persönlichen RepaymentLetter (PDF-Anschreiben) anhängen. Neu im `SendBulkMailRequest`:

```json
{
  "to_addresses": [...],
  "subject": "...",
  "body": "...",
  "repayment_phase_id": "<phase-uuid>",
  "attach_repayment_letter": true
}
```

Worker resolved für jeden Empfänger das passende `MemberDocument` (DocumentType=RepaymentLetter, gefiltert über die Description `"Anschreiben Auszahlung GJ {fiscal_year}"`) und hängt das File über das bestehende `document_storage` an die Mail. Analog D-03-Context-Merge ist die Auflösung Worker-internal in-memory, kein DB-Persist der per-recipient Attachments.

## Design Decisions (USER-DECIDED, 2026-06-03)

| Frage | Entscheidung |
|-------|--------------|
| Trigger | Opt-in via neues Flag `attach_repayment_letter: bool` (default false) |
| 0 Letters für Empfänger | Recipient als `failed` markieren mit `error="no_repayment_letter"`, Mail wird NICHT versendet |
| >1 Letters | Neuesten (`created DESC`) nehmen, `tracing::warn!` mit Count + member_id + fiscal_year |
| Auflösungs-Ort | Worker (analog D-03), nicht REST-Layer |

## Architecture

### Linkage RepaymentLetter ↔ Phase

`MemberDocumentEntity` hat **kein** `phase_id`-Feld. Die Verbindung läuft über die Description `"Anschreiben Auszahlung GJ {fiscal_year}"` — etabliertes Phase-13-Pattern (D-LETT-04). Konsequenz: Zwei `RepaymentPhase`s im selben `fiscal_year` würden bei der Letter-Auflösung kollidieren. Diese Limitation existiert bereits in `find_existing_letter_for_phase` (`genossi_service_impl/src/repayment_letter.rs:192`) und wird 1:1 übernommen.

### Validation: 2 Layer

1. **REST-Layer** (`send_bulk_mail` Handler): Bei `attach_repayment_letter=true && repayment_phase_id=None` → **400 BadRequest** mit Klartext.
2. **Service-Layer** (`MailServiceImpl::create_job`): Gleiche Bedingung → `MailServiceError::DataAccess`. Defense-in-Depth für direkte Service-Aufrufe ohne REST.

### Worker-Failure-Modi (alle → `mark_recipient_failed`)

- `attach_repayment_letter` true, aber `job.repayment_phase_id` ist `None` → `"attach_repayment_letter set but mail_job has no repayment_phase_id"` (sollte durch Validation nicht passieren, defensive)
- `BulkRecipient.member_id` ist `None` → `"attach_repayment_letter requires recipient.member_id"`
- Transaction-Open fehlschlägt → `"tx open failed for repayment_letter lookup"`
- Phase nicht gefunden → `"repayment_phase {uuid} not found"`
- MemberDocument-Query fehlschlägt → `"member_document lookup failed"`
- 0 Letters für Empfänger → `"no_repayment_letter"`

## Files touched

| File | Change |
|------|--------|
| `migrations/sqlite/20260603100000_mail_job_attach_repayment_letter.sql` | **NEW** — `ALTER TABLE mail_jobs ADD COLUMN attach_repayment_letter INTEGER NOT NULL DEFAULT 0` |
| `genossi_mail/src/dao.rs` | `MailJob` struct +1 Feld |
| `genossi_mail/src/dao_sqlite.rs` | `MailJobDb` +1 Feld, INSERT/SELECT/SELECT-all aktualisiert; Test-Schema-DDL erweitert; 2 neue Roundtrip-Tests |
| `genossi_mail/src/service.rs` | `MailService::create_job` Trait + Impl Signatur +1 Parameter; +1 Service-Layer-Validation-Check; alle Tests + Mocks aktualisiert |
| `genossi_mail/src/rest.rs` | `SendBulkMailRequest` +1 Feld; `send_bulk_mail` Handler +1 Validation + Übergabe; `send_mail` Single-Send passes `false`; +2 Serde-Tests |
| `genossi_mail/src/worker.rs` | +`find_repayment_letter_for_recipient` Helper; +Worker-Auflösungs-Block nach attachments-Aggregation; +6 Unit-Tests; +1 Import `DocumentType` |
| `genossi_mail/src/inbox.rs` | `MailJob` Literal +1 Feld (inbox-reply: `false`) |
| `genossi_service_impl/src/application.rs` | `create_job`-Call +1 Argument (confirmation mail: `false`) |
| `genossi_bin/tests/e2e_tests.rs` | 11 `SendBulkMailRequest`-Literale +1 Feld (Rule-3-Auto-Fix via `replace_all`) |

## Tests

- `cargo test -p genossi_mail`: **145 passed, 0 failed** (vorher 133 → 12 neue: 6 helper, 2 dao-roundtrip, 2 rest-serde, +2 verschoben/redundant)
- `cargo test --workspace`: **alle Suites grün, 0 failed**
- `cargo test --test e2e_tests`: **294 passed, 0 failed**
- `cargo clippy --workspace --all-targets`: **0 warnings**

### Neue Tests im Detail

- `worker::tests::find_repayment_letter_tests::*` (6): empty, no-RepaymentLetter-type, fiscal_year_mismatch, exact_match, skips_soft_deleted, returns_newest_when_multiple
- `dao_sqlite::tests::test_job_roundtrip_attach_repayment_letter_default_false`
- `dao_sqlite::tests::test_job_roundtrip_attach_repayment_letter_true`
- `rest::tests::test_send_bulk_mail_request_serde_attach_repayment_letter_explicit_true`
- `rest::tests::test_send_bulk_mail_request_serde_without_phase10_fields_backward_compat` erweitert um `attach_repayment_letter`-Assertion

## Decisions during implementation

### `attach_repayment_letter` in `MailJob` persistiert

Trotz des Worker-Auflösungs-Pfads (in-memory, kein DB-Eintrag der Letter-Attachments) wird das **Flag** selbst im `MailJob` persistiert. Gründe:

- **Retry-Safety**: Worker-Restart liest den Job neu → Konfiguration überlebt
- **Audit**: "Dieser Job wollte Letters automatisch anhängen" ist nachvollziehbar
- **Tests**: `failed_count` und `error="no_repayment_letter"` haben Bezug zum Job-Konfig-Stand

DB-Migration ist trivial (`INTEGER NOT NULL DEFAULT 0`, Phase-10-Pattern für SQLite-Booleans), Backward-Compat ohne Code-Änderung.

### Worker-Auflösung als async block statt closure

Initial mit `(|| async { ... })().await`-Pattern — Clippy moniert `manual_closure`. Refactor zu `async { ... }.await` mit explizitem Return-Type-Annotation auf der `Result`. Funktional identisch, idiomatic-Rust.

### Service-Layer-Validation zusätzlich zu REST-Validation

Defense-in-Depth: REST 400 BadRequest beim Klick-Pfad, Service-Error für Direkt-Aufrufe (z.B. künftiger Cron-Job). Beide Pfade liefern die gleiche Message; keine User-Confusion.

### Single-Send `send_mail` ist explizit `false`

Single-Send hat per Definition keinen Bulk-Kontext und keinen `repayment_phase_id`. Der `false`-Pass ist deterministisch und dokumentiert im REST-Handler-Kommentar.

## Out of Scope (deferred)

- **Frontend-UI** für Checkbox „RepaymentLetter automatisch anhängen" in `mail_page.rs` — separates Quick. Backend per Swagger-UI testbar.
- **Mehrere Letters anhängen** (statt newest) — wäre `Vec<MailRecipientAttachment>`-Push statt einer; aktuell User-entschieden "newest only".
- **Echte `phase_id`-Foreign-Key in MemberDocument** — würde Migration + Letter-Generierungs-Code umstellen, zu groß; Description-Fingerprint bleibt.
- **Bulk-PDF-Bundle als ein Attachment** (statt N Einzel-PDFs) — wäre nochmal eigene Aggregation, aktueller Worker-Pfad ist 1 PDF pro Empfänger.
- **Auto-Mark als 'angeschrieben'** für RepaymentEntries nach Versand — separate Geschäftslogik, Vorstand klickt das aktuell manuell im UI.

## How to test manually

1. Aktive `RepaymentPhase` mit `fiscal_year=2026` haben
2. Mindestens einen `MemberDocument` mit `document_type="repayment_letter"` und `description="Anschreiben Auszahlung GJ 2026"` für einen Member generieren (über bestehenden Endpoint `POST /api/repayment-phase/{id}/letters/generate`)
3. Über Swagger-UI an `/api/mail/send-bulk` POSTen:
   ```json
   {
     "to_addresses": [{ "address": "test@example.com", "member_id": "<member-uuid>" }],
     "subject": "Auszahlung",
     "body": "Hallo {{ first_name }}, anbei das Anschreiben.",
     "repayment_phase_id": "<phase-uuid>",
     "attach_repayment_letter": true
   }
   ```
4. Erwartung: 202 Accepted, Worker hängt das PDF an, Member bekommt Mail mit Attachment.
5. Negativ-Test: `attach_repayment_letter=true` ohne `repayment_phase_id` → 400 BadRequest.
