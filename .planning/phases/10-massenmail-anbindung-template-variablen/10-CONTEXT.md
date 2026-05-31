# Phase 10: Massenmail-Anbindung + Template-Variablen - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 10 erweitert die bestehende Mail-Pipeline um eine optionale Bindung an eine `RepaymentPhase`, sodass Templates die Variablen `{{ payout_amount }}`, `{{ share_count }}` und `{{ fiscal_year }}` referenzieren können. Vorstand wählt Empfänger im Frontend (Multi-Select analog zu Mitgliederliste-Massenmail in `genossi-frontend`) und nutzt den existierenden `POST /api/mail/send-bulk`-Endpoint — kein eigener Repayment-Mail-Endpoint. Zusätzlich: Pro versendeter Mail entsteht ein auditierter `MemberDocument`-Eintrag mit Template-Referenz, Recipient-Referenz und Versand-Status.

**In scope:**
- Migration `mail_job.repayment_phase_id BLOB NULL` (FK, kein Cascade) + SQLx-Felder + DAO-Lese-/Schreibpfad
- Erweiterung von `SendBulkMailRequest` um optionales `repayment_phase_id` (Job-weit, nicht per Recipient) und `MailService::create_job`-Signatur
- Migration `member_document.template_id BLOB NULL`, `member_document.mail_recipient_id BLOB NULL`, `member_document.status TEXT NULL` + `Auditable::audit_fields()`-Erweiterung
- Worker-Erweiterung (`genossi_mail/src/worker.rs`): wenn `job.repayment_phase_id IS NOT NULL`, lädt der Worker pro Recipient die offenen+contacted Entries (`deleted IS NULL AND status IN ('Open', 'Contacted')`) für `(member_id, phase_id)`, aggregiert `share_count = SUM(share_count_to_pay_out)` und `payout_amount = share_count × phase.share_value` (in Cent → Euro-String) und merged sie in den minijinja-Context zusätzlich zu `member_to_template_context`
- Worker erzeugt nach Versand (oder Failure) pro Recipient EINEN `MemberDocument`-Eintrag via `audited_create!` mit `document_type = "RepaymentMail"` (neue Variante in `DocumentType`-Enum), `template_id = job.template_id` (oder NULL falls Ad-hoc-Subject/-Body), `mail_recipient_id = recipient.id`, `status = "sent"|"failed"`, `description = job.subject` (sinnvoller Audit-Anchor)
- Frontend-Anker (Phase 12): Repayment-Detail-Page liefert die Multi-Select-Empfängerliste an Existing Mass-Mail-Page, mit `RepaymentPhase` vorausgewählt; Mass-Mail-Page bietet optional einen `RepaymentPhase`-Selector
- Existing-`MailService`-Erweiterung um optionales `template_id`-Tracking, sodass der Worker beim Erzeugen des `MemberDocument` die Template-Referenz mitliefern kann (heutiges `send-bulk` schickt body/subject inline, kein `template_id`); Migration: `mail_job.template_id BLOB NULL`
- Validation: Templates mit `{{ payout_amount }}`/`{{ share_count }}`/`{{ fiscal_year }}` müssen `{% if %}`-Guards verwenden (Strict-Opt-in). Bestehende `validate_template`-Probe in `template.rs` wird optional erweitert um Dummy-Werte für die 3 Variablen, wenn `repayment_phase_id` im Request steht
- Unit-Test für minijinja-Render mit allen 3 Variablen (Aggregation, Euro-Format, fiscal_year)
- E2E-Test mit MockSmtp: Bulk-Send für N Member, davon einer mit absichtlich kaputter Email-Adresse → N-1 MemberDocuments mit `status='sent'`, 1 mit `status='failed'`; Audit-Hashchain bleibt valide

**Out of scope (gehört in Phase 12 oder explizit nicht gewollt):**
- Eigener Endpoint `POST /api/repayment-phase/{id}/send-mail` — bewusst NICHT; Frontend nutzt existing `POST /api/mail/send-bulk`. Die ROADMAP-Formulierung des dedizierten Endpoints wird durch Body-Erweiterung am bestehenden Endpoint ersetzt (User-Decision: "kein Rad neu erfinden")
- Auto-Status-Toggle der Entries auf `Contacted` nach Versand — Frontend triggert den existing Batch-Endpoint aus Phase 8 separat (UI-Flow), oder Vorstand setzt manuell
- Backdating der Mail / Custom `MemberDocument.created` — Worker setzt `now()`
- HTML-Mail-Templates / Mail-Body-Anhänge mit gerendeter PDF — Phase 12 bzw. nicht im v1.1-Scope
- Speichern des gerenderten Mail-Bodies als File (relative_path = ""-Anchor) — Audit-Trail reicht über `description` + Mail-Job-Referenz
- Aggregation über PaidOut-Entries — Planner-Default ist `Open`/`Contacted`; PaidOut wird gefiltert, weil "schon ausgezahlt"
- Mail an Nicht-Member-Adressen mit Repayment-Variablen — Validation lehnt das ab (existing `send-bulk` verlangt schon `member_id` für Template-Rendering)

</domain>

<decisions>
## Implementation Decisions

### Mail-Pipeline-Integration

- **D-01:** **KEIN eigener Repayment-Mail-Endpoint.** Frontend nutzt das existing `POST /api/mail/send-bulk` (`genossi_mail/src/rest.rs:305`) genauso wie die Mitgliederliste-Massenmail. ROADMAP-Formulierung `POST /api/repayment-phase/{id}/send-mail` wird durch Body-Erweiterung am bestehenden Endpoint ersetzt. Begründung: User-Decision ("Wir haben Endpunkte für alles. Warum jetzt das Rad nochmal erfinden?") + Konsistenz mit Phase 12 UI-06 ("analog Mitgliederliste-Pattern").
- **D-02:** Frontend-Flow: Auf der `RepaymentPhase`-Detail-Page (Phase 12 UI-02/UI-03) wählt Vorstand Entries multi-select, klickt "Mail senden" → wird auf die existierende Mass-Mail-Page weitergeleitet. Die Mass-Mail-Page bietet optional einen `RepaymentPhase`-Selector, der vorausgewählt ist, wenn vom Repayment-Kontext kommend. UI-Detail-Ausgestaltung gehört zu Phase 12; Phase 10 liefert nur den Backend-Vertrag.

### Template-Context-Strategy

- **D-03:** **Job-weit `repayment_phase_id`**, nicht per Recipient. `SendBulkMailRequest` bekommt optional `repayment_phase_id: Option<String>` im Body; `MailService::create_job` bekommt einen zusätzlichen `repayment_phase_id: Option<Uuid>`-Parameter; `mail_job`-Migration: `repayment_phase_id BLOB NULL` (FK auf `repayment_phase.id`, ON DELETE SET NULL). Begründung: User-Decision; vermeidet Per-Recipient-Datenduplikat; bei Multi-Entry-Member wird aggregiert (siehe D-04).
- **D-04:** **Worker-Aggregation pro Member.** Wenn `job.repayment_phase_id IS NOT NULL`, lädt der Worker pro Recipient (mit `member_id`) alle nicht-soft-deleted Entries dieser `(member_id, phase_id)`-Kombi mit `status IN ('Open', 'Contacted')` und summiert: `share_count = SUM(entry.share_count_to_pay_out)`, `payout_amount = share_count × phase.share_value` (Cent → Euro-String mit 2 Nachkommastellen, deutsche Lokalisierung `"X,YZ"` ist Planner-Default — siehe Discretion). `fiscal_year = phase.fiscal_year`. Diese 3 Variablen werden zusätzlich zu `member_to_template_context` in den minijinja-Context gemerged.
- **D-05:** **Edge-Case: Member hat 0 Entries in Phase (oder alle PaidOut/deleted).** Worker rendert den Context OHNE `payout_amount`/`share_count`/`fiscal_year` (Variablen bleiben undefined). Wenn das Template sie ohne `{% if %}` referenziert, schlägt minijinja-strict → recipient wird via existing `mark_recipient_failed` als `failed` markiert mit klarer Fehlermessage. Ist explizit gewollt: Vorstand bekommt im Job-Detail die Fehler-Liste und sieht "Mitglied X hatte keine offenen Auszahlungs-Einträge".
- **D-06:** **Aggregations-Filter ist Planner-Discretion.** Default-Empfehlung: `deleted IS NULL AND status IN ('Open', 'Contacted')` — `PaidOut` ist semantisch "bereits ausgezahlt, keine Erinnerung mehr nötig". Planner darf den Filter via `RepaymentEntryDao::find_by_member_and_phase`-Methode oder einen In-Memory-Filter auf `find_by_phase_id`-Ergebnis umsetzen.

### MemberDocument-Persistenz (MAIL-04)

- **D-07:** **Migration mit drei neuen Spalten** auf `member_document`:
  - `template_id BLOB NULL` (FK auf `mail_template.id`, ON DELETE SET NULL — Templates können nachträglich gelöscht werden ohne Audit-Trail zu brechen)
  - `mail_recipient_id BLOB NULL` (FK auf `mail_recipient.id`, ON DELETE SET NULL)
  - `status TEXT NULL` (Werte: `"sent"`, `"failed"`; NULL für bestehende Documents = "kein Mail-Document")
- **D-08:** **`Auditable::audit_fields()` erweitert.** Die drei neuen Felder werden in `audit_fields()` ergänzt (`template_id`, `mail_recipient_id`, `status`). Bestehende `MemberDocument`-Audits sind backward-kompatibel (vorhandene Audit-Einträge referenzieren nur die alten Felder; neue Audits enthalten zusätzlich die neuen mit NULL-Werten für existing-Documents). Existing-Documents migrieren nicht (alle Felder NULL).
- **D-09:** **Neuer `DocumentType::RepaymentMail`-Variante** in `genossi_service/src/member_document.rs:55-71` mit `as_str = "repayment_mail"` und `from_str`-Reverse. **Nicht-Singleton** (mehrere Mails pro Mitglied möglich). Kein Typst-Template (kein `template_filename`).
- **D-10:** **Worker erzeugt `MemberDocument` nach Versand** (Final-State-Pattern), nicht beim Job-Anlegen. Begründung: ein `audited_create!` pro Recipient statt einer create+update-Sequenz. Wenn `send_mail_for_recipient` `Ok(())` zurückgibt → `MemberDocument` mit `status = "sent"`; bei `Err(...)` → `MemberDocument` mit `status = "failed"` UND `description` enthält die Fehler-Message (Truncated auf max 255 Chars). Wenn der Recipient kein `member_id` hat (Ad-hoc-Adresse) → KEIN `MemberDocument` erzeugt (existing `send-bulk` validiert das ohnehin für Template-Rendering, aber Defense-in-Depth).
- **D-11:** **Worker-`audited_create!`-Process-String:** `const REPAYMENT_MAIL_PROCESS: &str = "repayment-mail-worker"`. Der Worker bekommt zusätzliche Dependencies: `MemberDocumentDao`, `AuditLogDao`, `MailTemplateDao` (für template_id-Resolution), `UuidService`, `TransactionDao`. Die Wiring liegt in `start_mail_worker(...)`-Signatur (`genossi_mail/src/worker.rs:94`) und entsprechend `genossi_bin/src/lib.rs::RestStateImpl::new()`.

### MailJob-Schema-Erweiterung (Konsequenz aus D-10)

- **D-12:** **Migration `mail_job.template_id BLOB NULL`** (FK auf `mail_template.id`, ON DELETE SET NULL). Begründung: damit der Worker im `MemberDocument`-Create die Template-Referenz mitliefern kann, muss der Job die Template-ID kennen. Heute schickt `send-bulk` body/subject inline ohne Template-Bezug. `SendBulkMailRequest` bekommt optional `template_id: Option<String>` (zusätzlich zu subject/body — Frontend liefert beide: die gerenderten body/subject UND die Referenz auf das Quelltemplate). Bei `template_id = NULL`: `MemberDocument.template_id` ist NULL (Ad-hoc-Mail ohne Template).

### MailTemplate-Type + Validation

- **D-13:** **Strict opt-in über `{% if %}`-Guards.** `MailTemplate` bleibt typenlos. Templates mit Auszahlungs-Variablen schützen sie wie folgt: `{% if payout_amount %}Auszahlung: {{ payout_amount }}{% endif %}`. Begründung: User-Decision; vermeidet Migration auf `mail_template` (kein `template_type`-Feld); maximale Flexibilität; konsistent mit existing-`{% if title %}`/`{% if exit_date %}`-Pattern im genossi_mail-Test-Set.
- **D-14:** **Validation-Strategie:** Existing `validate_template` (`genossi_mail/src/template.rs:71`) bleibt Member-zentriert. Planner darf optional erweitern: wenn `repayment_phase_id` im Request, probe-rendert die Validation zusätzlich mit Dummy-Repayment-Context (`payout_amount = "0,00"`, `share_count = 0`, `fiscal_year = 2026`). Genaue Signatur-Änderung ist Planner-Discretion (entweder zweite `validate_template_with_phase`-Funktion oder optionaler Parameter).
- **D-15:** **Worker-Fail-Behavior bei Template-Render-Fehler ist unverändert** (`mark_recipient_failed` mit Fehler-Message). Der Worker geht weiter zum nächsten Recipient — kein All-or-Nothing (SC#4). Dieses Verhalten gilt explizit auch für die neuen Repayment-Variablen.

### Claude's Discretion

- **`payout_amount`-Format:** Empfehlung — Worker bildet einen String wie `"60,00"` (deutsche Lokalisierung, 2 Nachkommastellen, Komma als Dezimaltrennzeichen, KEIN Tausenderpunkt für v1.1, Euro-Symbol wird vom Template selbst gerendert wie `"{{ payout_amount }} €"`). Cent-Konvertierung: `share_count (i32) × share_value (i32 Cent) → i64 Cent`. Planner kann alternativ einen minijinja-Filter `format_euro` einführen, falls das wartbarer ist.
- **Worker-Dependencies vs. neuer Service:** Empfehlung — Worker bekommt 4 neue DAO-Deps direkt, kein neuer Service-Wrapper. Begründung: Worker ist sowieso State-haltend (config, smtp, document_storage); zusätzlicher Service-Layer wäre Overhead. Planner darf alternativ einen `RepaymentMailContextResolver`-Service einführen, falls Unit-Tests komplexer werden.
- **Aggregations-Order/Stable-Sort:** Wenn ein Member mehrere Entries hat, ist die Reihenfolge der Summation irrelevant (Addition kommutativ). Aber falls der Planner einen `share_count_breakdown`-Variable im Template-Context anlegen will (z.B. `[3, 2]` statt aggregiertem `5`), bleibt das Phase-Discretion — nicht in REQ.
- **`MailJob.template_id`-Update beim Send-Test/Send-Single:** Test-Mail (`POST /test`) und Single-Send (`POST /send`) müssen `template_id = NULL` setzen — sie kommen ohne Template aus.
- **Edge-Case-Test:** Member hat zwei Entries (eine Open, eine PaidOut). Worker rendert mit `share_count = 1` (nur Open) ODER `share_count = 2` (alle)? Planner-Default ist D-06 (`Open`/`Contacted` only).
- **Reihenfolge der `audited_create!`-Calls im Worker:** Recipient-Update (sent/failed-State + version-bump) und `MemberDocument`-create sollten idealerweise in einer Tx zusammen. Planner darf Reihenfolge bestimmen (Empfehlung: erst recipient-update, dann MemberDocument-create — chronologisch lesbar).
- **Migration-Reihenfolge & Backward-Compat-Tests:** Planner schreibt Migrations atomar (zwei: eine für `mail_job` + `mail_recipient`-Anker, eine für `member_document`). Backward-Compat-Test: Bestehende MemberDocuments dürfen weiterhin gelesen werden (alle Felder NULL).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Anforderungen
- `.planning/ROADMAP.md` §"Phase 10: Massenmail-Anbindung + Template-Variablen" — Goal + 4 Success Criteria (Bulk-Endpoint, Template-Engine, MemberDocument pro Mail, Failure-Handling)
- `.planning/REQUIREMENTS.md` §"Massenmail" MAIL-01..04 — Anforderungs-IDs vollständig in Phase 10
- `.planning/PROJECT.md` §"Current Milestone: v1.1 Anteile-Rückzahlungsphase" — Massenmail-Anbindung mit Auszahlungs-Wert als Template-Variable

### Vorgänger-Phasen (direkte Bauteil-Lieferanten)
- `.planning/phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — RepaymentPhase mit `fiscal_year` (i32) + `share_value` (i32 Cent); Lifecycle `Preparation → Open → Closed`
- `.planning/phases/08-repaymententry-auto-bef-llung/08-CONTEXT.md` — RepaymentEntry mit `share_count_to_pay_out`, `status` ∈ {Open, Contacted, PaidOut}; ENTR-03 erlaubt mehrere Entries pro (Member, Phase); soft-delete via `deleted`
- `.planning/phases/09-auszahlungs-buchung-atomisch-auditiert/09-CONTEXT.md` — Audit-Pragma D-01 (gemeinsamer process-String), Re-Read-Pattern (D-09 Schritt 8/10) — anwendbar wenn Worker eine Same-Tx-Cascade braucht

### Code-Anker: Mail-Pipeline (Reuse + Erweiterung)
- `genossi_mail/src/rest.rs:305-399` — `send_bulk_mail`-Handler; Phase 10 erweitert `SendBulkMailRequest` um `template_id`, `repayment_phase_id` + ggf. erweiterte Template-Validation
- `genossi_mail/src/service.rs:52-82` — `MailService`-Trait; Phase 10 erweitert `create_job`-Signatur um `template_id: Option<Uuid>` und `repayment_phase_id: Option<Uuid>`
- `genossi_mail/src/service.rs:234-321` — `MailServiceImpl::create_job`; Phase 10 schreibt die zwei neuen Felder in `MailJob`
- `genossi_mail/src/dao.rs` — `MailJob`-Struct und `MailJobDao`-Trait; Phase 10 erweitert beide um die zwei neuen Felder
- `genossi_mail/src/worker.rs:94-300+` — `start_mail_worker`; Phase 10 erweitert die Signatur um 4 neue DAOs (MemberDocumentDao, AuditLogDao, MailTemplateDao, UuidService, TransactionDao) und merged Repayment-Context vor `render_template`
- `genossi_mail/src/template.rs:15-40` — `member_to_template_context`; Phase 10 NICHT verändern, sondern den Repayment-Context separat mergen
- `genossi_mail/src/template.rs:53-69` — `strict_env` + `render_template`; bleibt strict (gewollt für D-15)
- `genossi_mail/src/template.rs:71-116` — `validate_template`; Phase 10 erweitert optional um Dummy-Repayment-Context

### Code-Anker: MemberDocument (Migration + DAO)
- `genossi_dao/src/member_document.rs:9-44` — `MemberDocumentEntity` + `Auditable::audit_fields`; Phase 10 ergänzt 3 Felder + erweitert `audit_fields()`
- `genossi_dao/src/member_document.rs:47-121` — `MemberDocumentDao`-Trait; Phase 10 berührt das Trait nicht (nur Struct erweitert), Default-Methoden bleiben unverändert
- `genossi_service/src/member_document.rs:55-71` — `DocumentType`-Enum + `as_str`/`from_str`; Phase 10 ergänzt `RepaymentMail`-Variante
- `genossi_service/src/member_document.rs:78-81` — `is_singleton`; Phase 10 setzt `RepaymentMail`-Wert auf `false` (Multi-Mail erlaubt)
- `genossi_service_impl/src/member_document.rs:120-142` — Existing `audited_create!`-Pattern für MemberDocument; Phase 10 reuse, aber im Worker statt im Service
- `genossi_dao_impl_sqlite/src/member_document_dao_impl_sqlite.rs` — SQLite-Impl; Phase 10 erweitert `INSERT`/`SELECT` mit den 3 neuen Spalten

### Code-Anker: RepaymentEntry/Phase-Resolution (Worker-Dep)
- `genossi_dao/src/repayment_entry.rs` (Phase 8) — `RepaymentEntryEntity`, `RepaymentEntryDao` mit `find_by_phase_id`; Phase 10 ergänzt evtl. `find_by_member_and_phase` oder filtert in-memory
- `genossi_dao/src/repayment_phase.rs` (Phase 7) — `RepaymentPhaseEntity` mit `fiscal_year`, `share_value`; Worker liest beim Aggregieren

### Code-Anker: MailTemplate (Resolution für Worker)
- `genossi_mail/src/mail_template_service.rs:32-52` — `MailTemplateService`-Trait; Worker braucht `MailTemplateDao` (direkt, nicht über Service — Worker hat keinen Auth-Context)
- `genossi_mail/src/dao.rs` `MailTemplate`-Struct + `MailTemplateDao`-Trait

### Code-Anker: Audit-Macros
- `genossi_service_impl/src/audit_macros.rs:5-36` — `audited_create!` (6 Args); Worker ruft es 1x pro Recipient nach Final-State
- `genossi_service_impl/src/audit_log.rs:55-113` — `build_audit_entries`; eine neue `transaction_id` pro `audited_create!`-Aufruf (Worker akzeptiert das — verschiedene Mail-Recipients sind separate Geschäftsvorfälle)

### Code-Anker: REST + Wiring
- `genossi_rest/src/lib.rs:265` — `/api/members/{member_id}/communications`-Route + Mail-API-Wiring; Phase 10 ergänzt keine neuen Routes
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — DI-Wiring; Phase 10 verbindet 4 neue DAOs an `start_mail_worker(...)` und übergibt evtl. `MailService::new(...)` mit erweiterten Args
- `genossi_mail/src/lib.rs` — Modul-Export (`rest_state`/`rest`); Phase 10 ergänzt evtl. Resolver-Trait für RepaymentEntry+Phase (analog zu `MemberResolver`)

### Testing-Anker
- `genossi_bin/tests/e2e_tests.rs` — E2E-Pattern; Phase 10 ergänzt: Bulk-Send mit `repayment_phase_id`, Mock-SMTP, eine kaputte Adresse → MemberDocument-Status-Mix; Audit-Chain valide
- `genossi_mail/src/template.rs:171-432` — bestehendes Test-Set für `render_template`/`validate_template`; Phase 10 ergänzt Unit-Tests für die 3 neuen Variablen + `{% if %}`-Pattern
- `genossi_mail/src/worker.rs` — Worker-Tests (existing); Phase 10 ergänzt MemberDocument-Create-Pfad mit Mock-DAOs

### Architektur-Constraints
- `.planning/codebase/ARCHITECTURE.md` — Anti-Patterns; insbesondere "Service Creating Its Own Transaction" — der Worker hält bereits Tx-Locks und sollte das Pattern nicht brechen
- `CLAUDE.md` §"Audit Log System" — 4-Schritt-Checklist; Phase 10 fügt RepaymentMail-MemberDocument zum auditierten Kreis hinzu, aber nutzt existing Auditable-Impl (nur erweitert)
- `CLAUDE.md` §"Entity Structure" — BLOB-UUIDs, ISO8601, optimistic locking; Phase 10 hält das Pattern ein
- `.planning/PROJECT.md` §"Constraints" — Audit-Pflicht, Component-First-Frontend

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`send-bulk`-Endpoint** (`genossi_mail/src/rest.rs:305`): Multi-Recipient-Pattern + Template-Validation gegen alle Member + Attachment-Resolution; Phase 10 erweitert Body und Validation
- **Worker-Loop** (`genossi_mail/src/worker.rs:113-300+`): pollt `next_pending` Recipient, lädt Job + Attachments, rendert Template per-Recipient, sendet via lettre, markiert sent/failed via `mark_recipient_failed` (existing fail-tolerant Pattern, deckt SC#4 nativ ab). Phase 10 ergänzt vor dem Render einen Repayment-Context-Merge und nach dem Send einen MemberDocument-Create
- **`member_to_template_context`** (`genossi_mail/src/template.rs:15`): liefert die 20 Member-Felder als minijinja-Value; Phase 10 ruft das UNVERÄNDERT auf und merged danach die 3 Repayment-Variablen via `context! { ..., payout_amount => ..., }` oder via `Value::from_serialize`
- **`audited_create!`-Macro** (`genossi_service_impl/src/audit_macros.rs:5`): Pattern für audited DAO-Schreibvorgänge; Worker wird damit MemberDocument anlegen — gleiches Pattern wie `member_document.rs:134`
- **`Auditable`-Impl auf MemberDocument** (`genossi_dao/src/member_document.rs:23-44`): existiert bereits, Phase 10 erweitert nur `audit_fields()` um die drei neuen Felder

### Established Patterns

- **Async Worker mit `next_pending`-Poll** und `mark_recipient_failed`-Pattern (existing): pro Recipient ein Render+Send-Versuch, Fehler → status='failed', error-Message gesetzt; Worker geht zum nächsten — kein All-or-Nothing
- **Strict-`{% if %}`-Pattern für optionale Variablen** (existing in `template.rs:test_formal_template_no_salutation`): Templates schützen optional verfügbare Variablen mit `{% if ... %}` — Phase 10 fordert dasselbe Pattern für die 3 neuen Variablen
- **`audited_*!`-Cross-Module-Reuse**: Worker im `genossi_mail`-Crate ruft `audited_create!` aus `genossi_service_impl` — kein Anti-Pattern, weil das Macro stand-alone aufgelöst werden kann (per Re-Export oder Direct-Import). Planner verifiziert das (evtl. Macro-Re-Export aus `genossi_service_impl::audit_macros` nötig)
- **Migration-Style**: SQLite-Migration in `migrations/sqlite/` (z.B. `20260506000000_add_code_to_helper_token.sql`-Pattern); Forward-only, ALTER TABLE ADD COLUMN für die neuen NULL-able Felder, keine Down-Migration
- **DocumentType-Singleton-Toggle** (`member_document.rs:78`): Phase 10 setzt `RepaymentMail` auf `is_singleton() = false`

### Integration Points

- **`SendBulkMailRequest`** (`genossi_mail/src/rest.rs:113-123`): Body bekommt zwei neue optionale Felder `template_id: Option<String>` und `repayment_phase_id: Option<String>`; bestehende Aufrufe bleiben kompatibel
- **`MailService::create_job`** (`genossi_mail/src/service.rs:56-63`): Signatur bekommt zwei neue `Option<Uuid>`-Parameter; das ist Breaking-Change für genossi-mail-interne Aufrufer (Phase 10 muss alle Call-Sites updaten — die existieren in `rest.rs:269`, `rest.rs:380`)
- **`MailJob`-Entity** (`genossi_mail/src/dao.rs`): zwei neue Felder `template_id: Option<Uuid>`, `repayment_phase_id: Option<Uuid>`; `MailJobDao::create`/`update`/`find_by_id` werden angepasst, SQLite-Impl folgt
- **`start_mail_worker`-Signatur** (`genossi_mail/src/worker.rs:94`): bekommt 5 neue Dependencies (MemberDocumentDao, MailTemplateDao, RepaymentEntryDao/Resolver, RepaymentPhaseDao/Resolver, UuidService, TransactionDao + AuditLogDao). Planner kann das durch einen `RepaymentMailContext`-Resolver-Trait bündeln (analog zu `MemberResolver`), um die Signatur überschaubar zu halten
- **`RestStateImpl::new()`** (`genossi_bin/src/lib.rs`): DI-Wiring aktualisiert; alle neuen DAOs sind schon im RestState vorhanden, nur Übergabe an Worker
- **Frontend-Mass-Mail-Page** (Phase 12): erhält optionalen `repayment_phase_id`-Selector + Pre-Selection-Logik; Phase 10 liefert nur den Backend-Vertrag (kein Frontend-Code im Scope)

</code_context>

<specifics>
## Specific Ideas

- **User-Zitat zur Architektur-Entscheidung:** "Ich wollte, dass das rein im Frontend gelöst wird. Also dass man die selbe Funktion nutzt wie bei der Mitgliederliste. Dort kann man beliebige Mitglieder markieren und dann 'Mail senden' klicken. Daraufhin wird man auf die Mail Senden Seite weitergeleitet. [...] Wir haben Endpunkte für alles. Warum jetzt das Rad nochmal erfinden?" — Diese Vorgabe ist die Quelle für D-01, D-02 und das Out-of-Scope-Item "eigener Endpoint".
- **User-Zitat zum optional Selector:** "Wie wäre es, wenn man im Frontend optional die PayoutPeriod auswählen kann? Und wenn man von der Seite kommt, wird die ID mitgeliefert und vorausgewählt?" — Quelle für D-03 (job-weit) und D-02 (Pre-Selection).
- **`payout_amount`-Format-Vorschlag (Planner-Discretion):** `"60,00"` (deutsche Lokalisierung, Komma als Dezimaltrennzeichen, kein Tausenderpunkt, kein Euro-Symbol). Template-Autor schreibt `{{ payout_amount }} €` oder Euro-Symbol nach Wahl. Konsistent mit anderen Genossi-Anzeigen in `genossi-frontend` (wenn dort schon ein Format etabliert ist — Planner verifiziert).
- **`status`-String-Enum-Convention:** Kleinbuchstaben `"sent"`, `"failed"` (analog zu `mail_recipient.status` im existing schema, das die Werte `pending`/`sent`/`failed` verwendet). NICHT PascalCase wie `Status::Sent`-Enum — bleibt String für SQL-Filterbarkeit.
- **`description`-Befüllung** beim MemberDocument-Create: `description = job.subject` für `status='sent'`; bei `status='failed'` zusätzlich Suffix mit Truncated-Fehler-Message (z.B. `"{subject} [FAILED: {error_truncated_to_200}]"`). Audit-Reader sieht den Subject + Fehler-Kontext.
- **`relative_path`-Konvention:** Für RepaymentMail-MemberDocuments gibt es kein File auf dem Filesystem (anders als JoinDeclaration-PDFs). Planner-Empfehlung: `relative_path = ""` oder `"mail/{recipient_id}"` als symbolischer Anker (kein File-Lookup gewollt). DocumentStorage hat aktuell keine "no-file"-Variante — Planner darf den `download_file`-Pfad für `RepaymentMail` als 404 zurückgeben.

</specifics>

<deferred>
## Deferred Ideas

- **Speichern des gerendeten Mail-Bodies als `.txt`/`.eml` im DocumentStorage** — könnte für Verbands-Compliance ("Was haben wir konkret geschickt?") sinnvoll sein. Aktuell reichen Audit-Trail + Mail-Job-Detail. Migration-Aufwand: 1 zusätzliches File pro Mail im document_storage-Pfad.
- **Auto-Status-Toggle auf RepaymentEntry nach Versand** (Open → Contacted): existing Phase-8-Batch-Endpoint kann nach erfolgreicher Mail-Job-Beendigung getriggert werden; Frontend-Flow oder Worker-Cascade. Nicht in MAIL-Reqs verlangt.
- **`format_euro`-minijinja-Filter**: stattdessen pre-formatierter String wie D-04 + Specifics. Filter wäre wartbarer wenn mehrere Mail-Templates den gleichen Format-Style brauchen.
- **Dedicated `template_type`-Feld auf `MailTemplate`**: würde Auszahlungs-Templates von normalen Member-Templates abgrenzen und falsche Verwendung im UI verhindern. Nicht in MAIL-Reqs; Strict-Opt-in-Pattern (D-13) ist ausreichend.
- **Per-Recipient `repayment_entry_id` statt Job-weit `repayment_phase_id`**: würde Multi-Entry-Member nicht aggregieren, sondern 1 Mail pro Entry schicken. User hat explizit das aggregierte Job-Modell gewählt — späteres Switching möglich, Schema-Migration nötig.
- **MailJob-weit Resolver-Cache**: Worker lädt `RepaymentPhase` pro Recipient erneut. Bei großen Jobs (>100 Recipients) wäre ein Phase-Cache je Job-Iteration sinnvoll. Performance-Optimierung, kein REQ.
- **HTML-Templates statt Plain-Text**: weiterhin out-of-scope; existing Mail-Pipeline ist Plain-Text.
- **Retry-Strategie für transient SMTP-Fehler**: aktuell muss Vorstand manuell via `POST /jobs/{id}/retry` retry'en. Auto-Retry mit exponential backoff wäre v2.0-Feature.

### Reviewed Todos (not folded)

Keine — `gsd-sdk query todo.match-phase 10` nicht ausgeführt (Tool nicht in Standardablauf bei dieser Sitzung; falls Todos existieren, sind sie nicht auf Phase 10 referenziert).

</deferred>

---

*Phase: 10-massenmail-anbindung-template-variablen*
*Context gathered: 2026-05-31*
