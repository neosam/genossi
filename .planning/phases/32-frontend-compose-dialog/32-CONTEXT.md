# Phase 32: Frontend Compose-Dialog - Context

**Gathered:** 2026-08-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Der Vorstand kann auf der Application-Detailansicht eine Erinnerung komponieren, in
**Live-Vorschau mit aufgelösten Platzhaltern** prüfen, **bewusst absenden** und die
**Kommunikations-Historie** einsehen — inklusive prominenter „zuletzt gesendet am …"-Anzeige
und sauberem No-Email-Handling. Das Backend (Phase 31) ist im Wesentlichen fertig; diese Phase
ist **überwiegend Frontend** mit **einer kleinen, bewusst gewählten Backend-Erweiterung** (siehe
D-06): das bereits gespeicherte gerenderte Mail-Body-Feld wird im Application-Communications-Pfad
sichtbar gemacht.

Konkret:
- Dedizierte `api.rs`-Funktionen (Send / Communications / Preview) — **nicht** member-umgeleitet
- Eigene Compose-**Route/Seite** (kein verschachteltes Modal — siehe D-01)
- Wiederverwendung der `component/mail_compose/*`-Bausteine + unveränderte, prop-getriebene
  `communication_timeline.rs`
- „E-Mail senden"-Button + „zuletzt gesendet"-Anzeige auf `application_detail.rs`
- Klickbare Timeline-Einträge → echter gesendeter Body (D-06)

**Nicht in dieser Phase:** Massenversand / Bulk-Erinnerung, Freitext-Empfänger, Open-/Click-Tracking
(Guardrails aus Phase 31 gelten weiter). Kein Neu-Rendern des Bodys für die Historie (der echte
gespeicherte Body wird gezeigt, nicht rekonstruiert). Keine neuen auditierten Felder.

</domain>

<decisions>
## Implementation Decisions

### Darstellung des Compose-Dialogs (APUI-01)
- **D-01 (GA1=c):** Der „Compose-Dialog" wird als **eigene Route/Vollseite** realisiert (Vorbild:
  `page/mail_page.rs`), **nicht** als verschachteltes Modal. Begründung: `component/application_detail.rs`
  ist **bereits selbst ein `Modal`** (mit `on_close` + verschachtelten Confirm/Reject-Bestätigungen);
  ein Modal-über-Modal ist ein Anti-Pattern (Fokus-Falle, z-index/Backdrop-Stacking). Die eigene Seite
  ist zugleich Component-First-konsistent (spiegelt `MailPage`). **Wichtig für Downstream:** Das
  Roadmap-/SC-Wort „Dialog" ist hier bewusst als **dedizierte Compose-Seite** umgesetzt — kein Widerspruch,
  sondern die HOW-Klärung dieser Phase.
- **D-02:** Der „E-Mail senden"-Button auf `application_detail.rs` navigiert per Route auf die Compose-Seite
  (analog `member_details.rs` → `nav.push(Route::MailPage)`), scoped auf die konkrete `application_id`.
  Bei fehlender `application.email` ist der Button **deaktiviert + annotiert** (Muster `is_email_empty` aus
  `member_details.rs`, `Key::NoEmailAddressHint`), nie ein stiller Fehlversuch. Post-Send: **zurück zur
  Application-Detailansicht** + Erfolgs-Toast, Timeline & „zuletzt gesendet" frisch geladen.

### Template-Default & -Auswahl (APUI-01)
- **D-03 (GA2):** Die Compose-Seite öffnet **mit der geseedeten „Zahlungserinnerung"-Vorlage vorbefüllt**
  (Subject + Body vorausgewählt). Der `TemplateSelector` ist auf **Antragsteller-Vorlagen gefiltert**
  (kein Member-Pool → vermeidet die strict-render-Bombe durch Member-only-Platzhalter). Wechsel auf andere
  Antragsteller-Vorlagen bleibt möglich.

### Live-Preview (APMAIL-04) — Claude's Discretion zum genauen Layout
- **D-04 (GA3):** Die Live-Vorschau ruft den **Backend-Preview-Endpoint** (Phase 31, D-06 — garantiert
  identisch zum Worker-Output), **debounced** beim Tippen. Zeigt **aufgelöste Platzhalter**. Layout spiegelt
  `MailPage` (Editor oben, aufgelöste Preview darunter). Der User war hier explizit indifferent → exaktes
  Layout/Debounce-Timing ist Claude's Discretion, solange die Vorschau den real aufgelösten Inhalt zeigt.

### Confirm-before-send & Post-Send (APMAIL-04, APUI-02)
- **D-05 (GA4):** Der bewusste Bestätigungsschritt ist **„Vorschau → Senden" auf derselben Seite** — kein
  zusätzlicher separater Confirm-Dialog. Die sichtbare, aufgelöste Vorschau **ist** die Bestätigung; der
  bewusste Klick auf „Senden" erfüllt confirm-before-send. Der Senden-Button ist **während des laufenden
  Requests deaktiviert** (kein Doppelversand). Die Dioxus-`form onsubmit`-Reload-Falle wird via
  `div`+`onclick`+`r#type:"button"` vermieden (Vorbild: `repayment_phases.rs`). Nach Erfolg: Navigation
  zurück zur Detailansicht + Toast (siehe D-02).

### Kommunikations-Historie, „zuletzt gesendet" & Body-Ansicht (APUI-03, APHIST)
- **D-06 (GA5 + Body-View — bewusste kleine Backend-Erweiterung):**
  - **Platzierung „beides":** Die Timeline ist ein **eigener Abschnitt in `application_detail.rs`**
    (wie unten in `member_details.rs`), umgesetzt mit der **unveränderten** prop-getriebenen
    `communication_timeline.rs`. Die „zuletzt gesendet am …"-Anzeige erscheint an **beiden** Stellen:
    neben dem Button in der Detailansicht **und** prominent auf der Compose-Seite (damit der
    Anti-Doppelversand-Guard direkt vorm Senden sichtbar ist).
  - **„zuletzt gesendet" zeigt Betreff + Status + Datum** (nicht nur das Datum) — die volle Info liegt
    via `communications` vor.
  - **Klickbarer Timeline-Eintrag → echter gesendeter Body.** Verifiziert am Code: der per-Empfänger
    **gerenderte Body wird bereits persistiert** (`mail_recipients.rendered_body` / `rendered_html_body` /
    `rendered_subject`, seit Phase 23 / Quick 260614, inkl. Backfill). Der `CommunicationEntry` trägt
    `mail_job_id` + `recipient_id` → der Body ist eindeutig auffindbar. **Entscheidung: Option 1** —
    `CommunicationEntryTO` (bzw. der Application-Communications-Pfad) wird um `rendered_body` /
    `rendered_html_body` erweitert (kleine Backend-Query-/TO-Anpassung, **keine** Schema-Migration nötig,
    Daten liegen vor). Das Frontend zeigt beim Klick ein **Detail-Panel/Modal im Application-Kontext** mit
    dem **echten gespeicherten Body** — **nicht** neu-gerendert (ein Live-Re-Render wäre potenziell ≠ dem
    real Gesendeten; bewusst verworfen).
  - **Korrektur zur D-09-Notiz aus Phase 31:** Jene besagte „kein Body-Snapshot (APHIST-FUT-01, verschoben)".
    Das bezog sich nur auf das **Nicht-Ausspielen** in der Application-Timeline — **nicht** aufs Speichern.
    Der Body ist gespeichert; diese Phase macht ihn im Application-Kontext sichtbar.

### Claude's Discretion
- Exaktes Preview-Layout, Debounce-Timing, i18n-Keys, Route-Namen/-Pfad der Compose-Seite.
- Exakte Shape der `api.rs`-Funktionen (Send/Communications/Preview) — solange **dediziert** (nicht
  member-umgeleitet) und die Phase-31-Endpoint-Shapes gespiegelt werden.
- Ob `rendered_body`/`rendered_html_body` direkt an `CommunicationEntryTO` hängen oder über einen kleinen
  Detail-Endpoint (`GET …/communications/{recipient_id}`) geliefert werden — solange der echte gespeicherte
  Body ohne Re-Render angezeigt wird und das Admin-Gate greift.
- Darstellung des Body-Detail-Panels (Modal vs. Inline-Expand).

### Reviewed Todos
Siehe `<deferred>` — 5 Keyword-Treffer geprüft, keiner im Phasen-Scope, keiner eingefaltet.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/ROADMAP.md` §"Phase 32: Frontend Compose-Dialog" — Goal + 5 Success Criteria (APMAIL-03/04,
  APUI-01/02/03)
- `.planning/REQUIREMENTS.md` — APMAIL-03/04, APUI-01/02/03
- `.planning/phases/31-service-rest-versand-versand-guardrails/31-CONTEXT.md` — Backend-Entscheide (D-01..D-13):
  `send_mail`, Preview-Endpoint (D-06), `last_sent_at`-Aggregation (D-07/08), `get_application_communications`
  (D-09), Admin-Gate (D-10)
- `.planning/phases/30-application-template-kontext-antragsteller-vorlagen/30-CONTEXT.md` —
  Antragsteller-Vorlagentyp, `application_to_template_context`, Seed „Zahlungserinnerung",
  `validate_application_template`

### Frontend — Wiederverwendung (Component-First)
- `genossi-frontend/src/page/mail_page.rs` — **Vorbild** für die Compose-Seite (D-01): Assemblage aus
  `MailSubjectInput` + `TemplateSelector` + `WysiwygEditor` (`plain_to_html`) + `TemplatePreview`; Send-Flow;
  Signal-State
- `genossi-frontend/src/page/member_details.rs` — **Vorbild** für Button-Muster (D-02): `is_email_empty`
  (Z. 38-42), disabled+annotierter „✉ Mail senden"-Button (Z. 402-431), `nav.push(Route::MailPage)`;
  Communication-Timeline-Abschnitt (Z. 1422-1428, `CommunicationTimeline { entries: … }`);
  `get_member_communications`-Aufruf (Z. 237)
- `genossi-frontend/src/component/mail_compose/mod.rs` — Exporte: `MailSubjectInput`, `WysiwygEditor` +
  `plain_to_html`, `TemplateSelector`, `TemplatePreview`, `MailPreviewFrame`, `TemplateVarButtons`,
  `MailAttachmentPicker`
- `genossi-frontend/src/component/communication_timeline.rs` — **unverändert** wiederverwenden, prop-getrieben
  (`CommunicationTimeline(entries: Vec<CommunicationEntryTO>)`); wird um Klick→Body-Detail ergänzt (Prüfen:
  ob Klick-Handling prop-basiert ergänzbar ist, ohne die Member-Nutzung zu brechen — sonst dünner Wrapper)
- `genossi-frontend/src/component/application_detail.rs` — hier kommen Button + „zuletzt gesendet" + Timeline-
  Abschnitt rein; ist **selbst ein `Modal`** (D-01-Begründung); bestehende Confirm/Reject-Dialoge (Z. 164-227)
  als Muster
- `genossi-frontend/src/api.rs` — bestehende Application-Funktionen (`get_application` Z. 810, `confirm_application`
  Z. 817, `reject_application` Z. 874), `preview_mail` (Z. 1153), `get_member_communications` (Z. 1723),
  `get_mail_job_detail` (Z. 1184) als Muster; **neue dedizierte** Funktionen hier ergänzen
- `genossi-frontend/src/router.rs` — Route-Registrierung (`MailPage` Z. 16/63 als Muster für neue Compose-Route)

### Backend — Konsum & kleine Erweiterung (D-06)
- `genossi_rest/src/application.rs:599` — `get_application_communications`-Handler (Phase 31); Route Z. 653,
  OpenAPI Z. 668 — hier ggf. `rendered_body`/`rendered_html_body` mitgeben oder Detail-Endpoint ergänzen
- `genossi_mail/src/dao.rs:59-84` — `CommunicationEntry` (`mail_job_id`, `recipient_id`, `rendered_subject`,
  `rendered_body`, `rendered_html_body`, `rendered_reconstructed`, `outbound_status`)
- `genossi_mail/src/communication_rest.rs` — `CommunicationEntryTO` + `CommunicationRestState` (Vorbild;
  TO-Erweiterung für D-06)
- `genossi_mail/src/rest.rs:127-356` — `MailRecipientTO`/`MailJobDetailTO` zeigen, dass `rendered_subject/body/
  html_body` bereits als TO exponiert werden (Muster für D-06-Mapping)
- Phase-31-Endpoints: `POST /api/applications/{id}/mail` (Send), `GET /api/applications/{id}/communications`,
  Application-Preview-Endpoint (alle admin-only, `MANAGE_MEMBERS_PRIVILEGE`) — von den neuen `api.rs`-Funktionen
  angesprochen

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`mail_compose/*`-Bausteine** (`MailSubjectInput`, `WysiwygEditor`+`plain_to_html`, `TemplateSelector`,
  `TemplatePreview`, `MailPreviewFrame`): decken die komplette Compose-UI ab — `MailPage` demonstriert die
  Assemblage 1:1 (D-01).
- **`communication_timeline.rs`**: prop-getrieben, in `member_details` unverändert genutzt → in
  `application_detail` identisch einsetzbar (D-06); Klick→Body nur additiv ergänzen.
- **`is_email_empty` + disabled-Button-Muster** (`member_details.rs:38-431`): 1:1 für den „E-Mail senden"-
  Button (D-02) + `Key::NoEmailAddressHint`.
- **Persistierter gerenderter Body** (`mail_recipients.rendered_body`/`rendered_html_body`/`rendered_subject`,
  seit Phase 23): der echte gesendete Inhalt liegt vor — D-06 macht ihn nur sichtbar, ohne Schema-Migration.
- **`MailJobDetailTO`-Mapping** (`rest.rs:352-356`): zeigt, wie `rendered_*` in ein TO gemappt wird — direkt
  übertragbar auf die `CommunicationEntryTO`-Erweiterung.

### Established Patterns
- **Voll-Seite statt Modal-in-Modal** (`MailPage`, `member_details` → `nav.push`): das ist der etablierte
  „großer Compose"-Weg im Frontend (D-01).
- **`div`+`onclick`+`r#type:"button"`** statt `form onsubmit` (Memory/Lesson, Vorbild `repayment_phases.rs`) —
  verhindert den WASM-Page-Reload (D-05).
- **Admin-Gate `MANAGE_MEMBERS_PRIVILEGE`** auf allen Application-Mail-Endpoints (Phase 31, D-10) — Frontend
  ruft nur admin-sichtbare Endpoints.
- **Dedizierte `api.rs`-Funktionen** je Ressource (keine Umleitung von Member-Funktionen) — Requirement +
  bestehende Konvention in `api.rs`.

### Integration Points
- `application_detail.rs` ← Button (D-02) + „zuletzt gesendet" (D-06, beide Stellen) + Timeline-Abschnitt (D-06).
- Neue Compose-Route in `router.rs` (Muster: `MailPage {}`); Seite konsumiert die neuen `api.rs`-Send/Preview-
  Funktionen + Antragsteller-gefilterten `TemplateSelector` (D-03).
- `get_application_communications` (Backend) ← `rendered_body`/`rendered_html_body` ergänzen **oder** kleiner
  Detail-Endpoint (D-06, Claude's Discretion) → neue `api.rs`-Funktion → Body-Detail-Panel.
- Preview-Aufruf ← Phase-31-Application-Preview-Endpoint (D-04), debounced.

</code_context>

<specifics>
## Specific Ideas

- Der User legt Wert darauf, dass die **echte gesendete Mail** einsehbar ist (nicht ein neu-gerenderter,
  potenziell abweichender Body) — deshalb D-06 Option 1 statt Live-Re-Render.
- „Bewusst bestätigen" heißt hier: die **aufgelöste Vorschau + der explizite Senden-Klick** sind der
  Confirm — kein zusätzlicher Modal-Zwischenschritt (D-05).
- „zuletzt gesendet" soll **Betreff + Status + Datum** zeigen und der Anti-Doppelversand-Guard soll
  direkt vor dem Senden auf der Compose-Seite sichtbar sein.

</specifics>

<deferred>
## Deferred Ideas

- **Massen-/Bulk-Erinnerung** an alle `Offen`-Antragsteller (APMAIL-FUT-01) — bewusst nach v1.6.
- **Timeline-Klick-UX-Vertiefung** (z. B. Deep-Link, Diff, Reply-aus-Historie) — über D-06 hinaus nicht in Scope.

### Reviewed Todos (not folded)
- „Originalen Mitgliedsantrag als Datei-Attachment an Application hinterlegen" — Attachment-Feature, nicht
  Compose-Dialog; eigener Vorgang.
- „HTML-Mail-Support statt nur Textmails" — bereits umgesetzt (Phase 23/24); Altlast-Todo.
- „Datums-Template-Variablen im deutschen Format (DD.MM.YYYY)" — Template-Rendering, nicht Frontend-Compose.
- „Pre-Flight-Check: RepaymentLetter vor Send prüfen" — RepaymentLetter-Bulk-Domäne, nicht Application-Mail.
- „Bulk-Action: alle no_repayment_letter-Briefe generieren + retry" — RepaymentLetter-Bulk, nicht Phase 32.

</deferred>

---

*Phase: 32-frontend-compose-dialog*
*Context gathered: 2026-08-20*
