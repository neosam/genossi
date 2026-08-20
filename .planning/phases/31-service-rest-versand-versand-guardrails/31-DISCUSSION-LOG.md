# Phase 31: Service + REST Versand (Versand + Guardrails) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-20
**Phase:** 31-service-rest-versand-versand-guardrails
**Areas discussed:** Fehler-Semantik (APMAIL-02), send_mail-Signatur, "Zuletzt gesendet"-Quelle, Admin-Gate & no-email

---

## Fehler-Semantik (APMAIL-02)

| Option | Description | Selected |
|--------|-------------|----------|
| A — Synchroner Pre-Flight + enqueue | Config/Render/SMTP-Config-Präsenz prüfen, dann Job anlegen; SMTP-Delivery async im Worker | ✓ |
| B — Voll-synchroner Send | Direktes `.send()` wie `send_test_mail_with_body`, echtes Delivery-Result sofort; Historie separat | |
| C — Enqueue + Job-Status pollen | | |

**User's choice:** A — aber OHNE expliziten SMTP-Config-Pre-Flight.
**Notes:** „reicht wenn er beim senden scheitert. Bei der Konfiguration wird der SMTP server in der
Regel ohnehin getestet." → kein synchroner SMTP-Test; Delivery-Fehler landen im
Recipient-`outbound_status` (in der Timeline sichtbar). Kern-Kontrast zu `send_confirmation_mail`:
echtes `Result` statt stillem `()`. (D-01, D-02)

---

## send_mail-Signatur

| Option | Description | Selected |
|--------|-------------|----------|
| A — Vorgerenderter/roher Content | Client schickt subject+body(+html), wie die anderen Sends | ✓ |
| B — template_id + Server-Render | Server lädt Template und rendert | |
| C — Hybrid | | |

**User's choice:** A — „wie bei den anderen Mails auch".
**Notes:** Verifiziert am Code: die anderen Sends speichern den rohen Body mit Platzhaltern; der
**Worker** rendert per-recipient via `resolve_rendered_content`. → send_mail nimmt rohen Content;
der Application-Zweig kommt in den Worker-Renderer (D-03/D-04). Keine strict-Render-Prüfung beim
Versand, da `validate_application_template` bereits bei Create/Update gatet (D-05).

### Preview-Endpoint (2c)
**User's choice:** „Was auch immer du besser findest. Ist mir eigentlich egal, in welcher Phase
das gebaut wird." → Backend-Preview-Render-Endpoint in **Phase 31**, teilt sich den Renderer-Seam
mit dem Send; Phase 32 verdrahtet nur UI (D-06).

---

## "Zuletzt gesendet"-Quelle (APHIST-02)

| Option | Description | Selected |
|--------|-------------|----------|
| 3a-A — `created` (Enqueue) | Guard greift sofort beim Absenden | ✓ |
| 3a-B — `sent_at` (Worker-Delivery) | | |
| 3b-A — Volle Einträge, Client leitet ab | | |
| 3b-B — Dediziertes serverseitiges Feld | Keine Client-Aggregation | ✓ |

**User's choice:** 3a-A (created, empfohlen) · 3b-B · 3c: Betreff mit rein.
**Notes:** „clientseitig sollte nichts aggregiert werden" → serverseitiges `last_sent_at` (D-08).
„Betreff sollte mit rein. Wie bei einem Mitglied." → bestehendes `CommunicationEntryTO` (trägt
`subject`) 1:1 wiederverwenden (D-09). Kein Body-Snapshot (APHIST-FUT-01).

---

## Admin-Gate & no-email

| Option | Description | Selected |
|--------|-------------|----------|
| 4a-A — `MANAGE_MEMBERS_PRIVILEGE` | Wie confirm/reject, Konsistenz | ✓ |
| 4a-B — Dedizierter Admin-Check | | |
| 4b-A — Backend-Fehler + Frontend-Disable | Defense-in-Depth bei fehlender Adresse | ✓ |
| 4b-B — Nur Frontend-Guard | | |

**User's choice:** Alles wie empfohlen (4a-A, 4b-A, 4c: Permission→NotFound→409).
**Notes:** „Ansonsten alles wie empfohlen. Mir fällt nichts neues ein." (D-10, D-11, D-12)

## Claude's Discretion

- Exakte Pfad-/Request-/Response-Shapes (Preview-Endpoint, send-Body), Member-Muster spiegeln.
- `last_sent_at` auf `get_application`-Response vs. eigener Endpoint (solange serverseitig).
- OpenAPI-Doku-Detail, i18n-Keys, ServiceError-Variante für no-email.
- In welcher Phase der Preview-Endpoint landet (→ Phase 31 gewählt).

## Deferred Ideas

- APMAIL-04 UI-Verdrahtung + APMAIL-03 Button-Disable → Phase 32.
- APHIST-FUT-01 (Betreff-/Body-Snapshot pro Eintrag), APMAIL-FUT-01 (Bulk-Send) → nach v1.6.
