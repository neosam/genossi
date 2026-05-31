# Phase 10: Massenmail-Anbindung + Template-Variablen - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 10-massenmail-anbindung-template-variablen
**Areas discussed:** Mail-Pipeline-Integration, Template-Context-Strategy, MemberDocument-Persistenz, MailTemplate-Type + Validation

---

## Mail-Pipeline-Integration

| Option | Description | Selected |
|--------|-------------|----------|
| Neuer Endpoint im repayment-Modul | POST /api/repayment-phase/{id}/send-mail (ROADMAP-literal); Endpoint baut RecipientInputs aus Entry-IDs + Member-IDs auf, nutzt intern existing MailService::create_job | |
| Existing send-bulk erweitern | POST /api/mail/send-bulk bekommt optionales repayment_entry_ids-Array; Worker resolved Context per Recipient | |
| Eigener Service + eigener Worker | Komplett separates Modul (genossi_repayment_mail) | |
| **Frontend-only: existing send-bulk reuse** | **Vom User explizit gefordert: Frontend nutzt existing send-bulk wie bei Mitgliederliste-Massenmail; kein neuer Backend-Endpoint** | ✓ |

**User's choice:** Frontend-only (Free-text response): "Ich wollte, dass das rein im Frontend gelöst wird. Also dass man die selbe Funktion nutzt wie bei der Mitgliederliste. Dort kann man beliebige Mitglieder markieren und dann 'Mail senden' klicken. Daraufhin wird man auf die Mail Senden Seite weitergeleitet. [...] Wir haben Endpunkte für alles. Warum jetzt das Rad nochmal erfinden?"

**Notes:** Konsequenz für ROADMAP: Die wörtliche `POST /api/repayment-phase/{id}/send-mail`-Formulierung wird ersetzt durch Body-Erweiterung am bestehenden Endpoint. Phase 10 liefert NUR Backend-Erweiterungen (Body-Felder + Worker-Resolver + MemberDocument-Create). Frontend-Flow gehört zu Phase 12 UI-06.

---

## Template-Context-Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Recipient bekommt repayment_entry_id-FK | Migration: mail_recipient.repayment_entry_id NULL; Frontend schickt pro Recipient die Entry-ID; Worker-Resolver lookupt Entry+Phase | |
| send-bulk-Body bekommt extra_context-Map pro Recipient | Frontend liefert pre-formatierte Werte als JSON-Map pro Recipient | |
| Frontend pre-rendert + per-Recipient body override | Frontend rendert komplett vorab; Worker schickt nur | |
| **Job-weit phase_id + Worker aggregiert pro Member** | **Job-Level repayment_phase_id; Worker findet alle Entries für (member_id, phase_id) und summiert** | ✓ |

**User's choice:** Job-weit phase_id + Worker aggregiert pro Member.

**Notes:** Begründet durch User-Anschluss-Idee: "Wie wäre es, wenn man im Frontend optional die PayoutPeriod auswählen kann? Und wenn man von der Seite kommt, wird die ID mitgeliefert und vorausgewählt?" — daraus folgt Job-Level-Bindung. Multi-Entry-Member werden aggregiert; Default-Filter (Open/Contacted, ohne PaidOut, ohne deleted) ist Planner-Discretion mit Empfehlung in CONTEXT D-06.

---

## MemberDocument-Persistenz

| Option | Description | Selected |
|--------|-------------|----------|
| **Migration: template_id, status, mail_recipient_id** | **3 neue Spalten auf member_document, Auditable-Erweiterung** | ✓ |
| Migration: nur mail_recipient_id (status via JOIN) | Schmal, aber MemberDocument bleibt status-los | |
| Reuse description-Feld + neuer DocumentType | JSON-String in description, keine Migration | |

**User's choice:** Migration mit 3 neuen Spalten.

**Notes:** Spalten sind NULL-able für Backward-Compat (existing MemberDocuments bleiben funktional). Wann der MemberDocument erzeugt wird (beim Endpoint mit `pending` vs. erst vom Worker mit Final-State) wurde nicht separat gefragt — als Planner-Discretion festgelegt mit Empfehlung "Worker nach Versand" (D-10 in CONTEXT.md).

---

## MailTemplate-Type + Validation

| Option | Description | Selected |
|--------|-------------|----------|
| **Strict opt-in mit `{% if %}`** | **Templates schützen Variablen mit `{% if payout_amount %}...{% endif %}`; MailTemplate bleibt typenlos; keine Schema-Änderung** | ✓ |
| minijinja UndefinedBehavior::Lenient für Repayment-Variablen | Default-Empty-Values bei undefined; stille Fehler möglich | |
| Validation im Endpoint: Pflicht-Kombination | Endpoint parsed Template, verlangt repayment_phase_id wenn Variablen referenziert | |

**User's choice:** Strict opt-in mit `{% if %}`.

**Notes:** Konsistent mit bestehendem `{% if title %}`/`{% if exit_date %}`-Pattern in den existing minijinja-Tests. Worker-Fail-Behavior bei undefined Variablen + fehlendem `{% if %}` ist unverändert (per-recipient failed via existing `mark_recipient_failed`). Existing `validate_template`-Erweiterung um optionalen Phase-Context ist Planner-Discretion.

---

## Claude's Discretion

Aus CONTEXT.md (zusammengefasst):
- **`payout_amount`-Format**: empfohlen `"60,00"` (deutsche Lokalisierung, 2 Nachkommastellen, kein Tausenderpunkt, kein Euro-Symbol)
- **Worker-Dependencies vs. neuer Resolver-Service**: empfohlen Worker bekommt DAOs direkt; Resolver-Trait als Bündelung erlaubt
- **Aggregations-Filter** der Worker-Aggregation: empfohlen `deleted IS NULL AND status IN ('Open', 'Contacted')`
- **`audited_create!`-Call-Order im Worker**: empfohlen erst recipient-update (sent/failed), dann MemberDocument-create
- **Migration-Reihenfolge**: zwei atomare Migrationen (mail_job/mail_recipient erste; member_document zweite)
- **`description`-Befüllung** bei failed-Status: subject + truncated error-Message
- **`relative_path`-Konvention** für RepaymentMail-MemberDocuments: empfohlen leerer String oder symbolischer Anker; kein File-Lookup
- **Template-Validation-Erweiterung** mit Dummy-Repayment-Context wenn `repayment_phase_id` im Request

## Deferred Ideas

- Speichern des gerendeten Mail-Bodies als `.txt`/`.eml` im DocumentStorage
- Auto-Status-Toggle der RepaymentEntries auf `Contacted` nach Versand
- `format_euro`-minijinja-Filter als wartbarere Alternative zu pre-formatierten Strings
- Dedicated `template_type`-Feld auf `MailTemplate`
- Per-Recipient `repayment_entry_id` statt Job-weit aggregierte Variante
- MailJob-weit Resolver-Cache zur Performance-Optimierung bei Großjobs
- HTML-Templates statt Plain-Text
- Auto-Retry-Strategie für transient SMTP-Fehler
