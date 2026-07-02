# Phase 25 Discussion Log: Application File Upload + Audited Carryover

**Date:** 2026-07-02
**Mode:** discuss (chat-form, all questions in one turn per Feedback-Memory `feedback_discuss_in_chat_form.md`)
**Participants:** neosam (Simon Goller) + Claude

---

## Domain Boundary Presented

Admin lädt Original-Antrag (Datei) an eine `Application`, beim `confirm()` wird die Datei atomar als auditiertes `MemberDocument` ans Mitglied kopiert. Parallel zur Mail-Strecke (22→23→24) laufend.

## Prior Decisions Carried Forward (nicht erneut diskutiert)

- Layered DAO/Service/REST + generische `Deps`
- Soft-Delete + `version`-UUID projektweites Entity-Muster
- `audited_*!`-Macros für auditierte Entitäten (MemberDocument gehört dazu)
- CR-02 Permission-Check-Ordering (projektweiter BLOCKER, neuer Code nicht regressiv)
- Component-First Frontend (v1.4-Konvention)
- Enum statt Boolean (Feedback-Memory `feedback_enum_not_boolean.md`)
- `jj` statt `git` (Feedback-Memory `feedback_use_jj_not_git.md`)
- Deutsche UI-Sprache

## Codebase Scout (relevant für Gray Areas)

- `genossi_service/src/document_storage.rs:24-28` — Storage-Trait `save/load/delete`, **kein `copy`/`rename`** → Move = load+save+delete
- `genossi_rest/src/member_document.rs:115-224` — Referenz-Upload-Handler
- `genossi_service_impl/src/application.rs:280-419` — bestehender `confirm()`-Ablauf, enthält **CR-02-Anti-Pattern** (Ordering `current_user_id()` vor `check_permission()`)
- `genossi_dao/src/auditable.rs` — nur für auditierte Entitäten

## Gray Areas Presented (5)

1. **Slot-Modell** — Single-Slot vs. Multi-Slot pro Application?
2. **Re-Upload-Verhalten** — überschreiben oder Soft-Delete+neu?
3. **Application-File nach `confirm()`** — kopieren (Roadmap-Wortlaut) oder verschieben?
4. **`application_documents`-Schema-Umfang** — mit oder ohne `document_type`/`description`?
5. **MIME-Allowlist & Body-Limit** — MemberDocument-Werte teilen oder eigen?

## User Answers

1. **Ja, genau eine Datei** → Single-Slot.
2. **Ja, überschreiben** → Replace-in-Place (DB-Zeile bleibt, alte Datei physisch löschen).
3. **Verschieben statt kopieren** — Application braucht die Datei nach `confirm` nicht mehr, das MemberDocument ist der Antrag in seiner endgültigen Form. **⚠ Weicht vom aktuellen APDOC-03-Wortlaut ab** — REQUIREMENTS.md-Text muss synchronisiert werden.
4. **Nur Antrag, kein `document_type`, keine `description`** — Single-Slot + fester impliziter Zweck. Alle weiteren Dokumente werden erst als MemberDocument relevant.
5. **Standards, ein Set reicht** — `allowed_extensions()`, `lookup_allowed_mime()`, `MEMBER_DOCUMENT_BODY_LIMIT` teilen.

## Deferred Ideas

- Housekeeping-Job für verwaiste Application-Files (Best-Effort-Delete-Ausfall) → v1.5+
- Multi-File pro Application (Vorstands-Notizen etc.) → nicht in Scope
- Application-Detail-„Historie" nach confirm → nice-to-have
- CR-02 projektweit als `gen_auth_admin!`-Helper → separate Refactor-Phase (Carry-Forward-Techdebt v1.2-MILESTONE-AUDIT)
- Drag-and-Drop-Upload UI → MVP File-Dialog reicht

## Claude's Discretion (nicht dem User vorgelegt, Best-Practice-Entscheidungen)

- **DB-Constraint** unique partial index `WHERE deleted IS NULL` für Single-Slot-Invariante (statt nur Service-Guard)
- **Storage-Rollback-Reihenfolge** save-new → update-DB → delete-old (best-effort für Delete)
- **Endpoint-Namen** `/api/application/{id}/document` (Einzahl, passt zu Single-Slot)
- **Frontend-Component-Prüfung** — bestehende Datei-Slot-Component wiederverwenden, sonst neues `ApplicationDocumentSlot`
- **Description-Text** beim Carryover: `"Original-Antrag (übernommen bei Bestätigung am {date})"` mit deutschem `DD.MM.YYYY`

## Offene Punkte

- **APDOC-03-Wortlaut-Update in REQUIREMENTS.md** — Vorschlag: im Rahmen Phase-25-Execution als Doku-Commit mit-erledigen (nicht als separate Quick versauern lassen)
</content>
</invoke>