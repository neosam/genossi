# Phase 32: Frontend Compose-Dialog - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-20
**Phase:** 32-frontend-compose-dialog
**Areas discussed:** Dialog-Darstellung, Template-Default, Live-Preview, Confirm/Post-Send, Timeline & „zuletzt gesendet" inkl. Body-Ansicht

---

## GA1 — Darstellung des Compose-Dialogs

| Option | Description | Selected |
|--------|-------------|----------|
| (a) verschachteltes Modal | Modal über dem Detail-Modal | |
| (b) In-Place-Toggle | Detail-Inhalt im selben Modal durch Compose-Ansicht ersetzen | |
| (c) eigene Route/Vollseite | Wie beim Member (`MailPage`) | ✓ |

**User's choice:** (c) — „verschachtelte Modals sind nicht so gut, oder?"
**Notes:** Bestätigt: Modal-in-Modal ist Anti-Pattern; `application_detail.rs` ist selbst schon ein Modal.
Roadmap-Wort „Dialog" wird bewusst als eigene Compose-Seite realisiert.

---

## GA2 — Template-Default & -Auswahl

| Option | Description | Selected |
|--------|-------------|----------|
| Leer öffnen | Kein Template vorbelegt | |
| Vorbefüllt | Geseedete „Zahlungserinnerung" vorausgewählt, Selector auf Antragsteller-Vorlagen gefiltert | ✓ |

**User's choice:** „Gleich mit der Vorlage vorausgefüllt."
**Notes:** —

---

## GA3 — Live-Preview: Mechanik & Layout

| Option | Description | Selected |
|--------|-------------|----------|
| Split-View / Tab / Confirm-only + Debounce-Wahl | Diverse Layout-/Timing-Varianten | (Claude) |

**User's choice:** „Ist mir eigentlich egal." → Claude's Discretion
**Notes:** Festgelegt: Backend-Preview-Endpoint (D-06 Phase 31), debounced, Layout wie `MailPage`.

---

## GA4 — Confirm-Schritt & Post-Send-Verhalten

| Option | Description | Selected |
|--------|-------------|----------|
| Separater Confirm-Dialog | Extra-Modal wie confirm/reject | |
| Vorschau → Senden | Aufgelöste Vorschau + expliziter Senden-Klick = Bestätigung | ✓ |

**User's choice:** „Vorschau -> Senden - das ist ja wie ein Confirm"
**Notes:** Post-Send: zurück zur Application-Detailansicht + Toast, Timeline & „zuletzt gesendet" refreshen.
Senden-Button disabled während Request; `form onsubmit`-Falle via `div`+`onclick`+`r#type:"button"`.

---

## GA5 — Timeline & „zuletzt gesendet" (+ Body-Ansicht)

| Option | Description | Selected |
|--------|-------------|----------|
| Nur Detailansicht | Timeline/last-sent nur in application_detail | |
| Nur Compose-Seite | Nur auf der Compose-Seite | |
| beides | Timeline-Abschnitt in Detail + last-sent an beiden Stellen | ✓ |

**User's choice:** „beides"; zusätzlich: „Ich will schon Betreff und Status sehen. Und ich will auch
draufklicken können und die Mail sehen können, die verschickt wurde."

**Body-Ansicht — Zusatz-Entscheid:**
| Option | Description | Selected |
|--------|-------------|----------|
| (1) TO-Erweiterung | `rendered_body`/`rendered_html_body` an Communications-Pfad hängen, echten gespeicherten Body zeigen | ✓ |
| (2) Deep-Link Job-Detail | Verlinkung auf Admin-`get_mail_job_detail`-View | |
| (3) Live-Re-Render | Body neu rendern (potenziell ≠ Gesendetem) | |

**User's choice:** „Wird denn der Body gespeichert? Das ist wichtig." → nach Code-Verifikation:
**Ja** (`mail_recipients.rendered_body`/`rendered_html_body` seit Phase 23). → „Option 1".
**Notes:** Korrigiert die D-09-Notiz aus Phase 31 („kein Snapshot" bezog sich nur aufs Nicht-Ausspielen,
nicht aufs Speichern). Option 1 braucht **keine** Schema-Migration, nur TO-/Query-Anpassung.

---

## Claude's Discretion

- Exaktes Preview-Layout, Debounce-Timing, Route-Name/-Pfad, i18n-Keys.
- Shape der dedizierten `api.rs`-Funktionen (Send/Communications/Preview).
- TO-Erweiterung vs. kleiner Detail-Endpoint für den gerenderten Body.
- Body-Detail-Panel: Modal vs. Inline-Expand.

## Deferred Ideas

- Massen-/Bulk-Erinnerung (APMAIL-FUT-01) — nach v1.6.
- 5 geprüfte Keyword-Todos (Attachment, HTML-Mail, Datumsformat, RepaymentLetter-Preflight, Bulk-Retry) —
  keiner im Phasen-Scope, keiner eingefaltet.
