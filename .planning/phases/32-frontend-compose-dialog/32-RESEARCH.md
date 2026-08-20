# Phase 32: Frontend Compose-Dialog — Research

**Researched:** 2026-08-21
**Domain:** Dioxus 0.6 WASM Frontend + kleine Rust/Axum-Backend-Erweiterung (D-06)
**Confidence:** HIGH (alle Canonical Refs am Code verifiziert; keine externen Paketrecherchen nötig)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Compose-Dialog = **eigene Route/Vollseite** (Vorbild `page/mail_page.rs`), **nicht** verschachteltes Modal. Grund: `component/application_detail.rs` ist bereits selbst ein `Modal` (mit `on_close` + verschachtelten Confirm/Reject-Modals) → Modal-über-Modal ist Anti-Pattern (verifiziert, siehe unten). „Dialog" im Roadmap-/SC-Wortlaut ist bewusst als dedizierte Compose-Seite umgesetzt.
- **D-02:** „E-Mail senden"-Button auf `application_detail.rs` navigiert per Route auf die Compose-Seite (analog `member_details.rs` → `nav.push(Route::MailPage {})`), scoped auf `application_id`. Bei fehlender `application.email`: Button **deaktiviert + annotiert** (Muster `is_email_empty`, `Key::NoEmailAddressHint`), nie stiller Fehlversuch. Post-Send: zurück zur Application-Detailansicht + Erfolgs-Toast; Timeline & „zuletzt gesendet" frisch geladen.
- **D-03:** Compose-Seite öffnet mit der geseedeten **„Zahlungserinnerung"-Vorlage vorbefüllt** (Subject + Body). `TemplateSelector` ist auf **Antragsteller-Vorlagen gefiltert** (kein Member-Pool → vermeidet strict-render-Bombe). Wechsel auf andere Antragsteller-Vorlagen bleibt möglich.
- **D-04:** Live-Vorschau ruft den **Backend-Preview-Endpoint** (Phase 31, `POST /api/applications/{id}/mail/preview`), **debounced**, zeigt **aufgelöste Platzhalter**. Layout spiegelt `MailPage` (Editor oben, Preview darunter).
- **D-05:** Bewusster Bestätigungsschritt = **„Vorschau → Senden" auf derselben Seite** — kein zusätzlicher Confirm-Dialog. Senden-Button **während des Requests deaktiviert** (kein Doppelversand). `form onsubmit`-Reload-Falle via `div`+`onclick`+`r#type:"button"` vermeiden (Vorbild `repayment_phases.rs`). Nach Erfolg: Navigation zurück + Toast.
- **D-06:** Timeline als **eigener Abschnitt in `application_detail.rs`** (unveränderte prop-getriebene `communication_timeline.rs`). „zuletzt gesendet am …" an **beiden** Stellen (neben Button in Detailansicht **und** prominent auf Compose-Seite als Anti-Doppelversand-Guard). „zuletzt gesendet" zeigt **Betreff + Status + Datum**. Klickbarer Timeline-Eintrag → **echter gespeicherter Body** (nicht neu-gerendert). **Entscheidung: Option 1** — `CommunicationEntryTO` (bzw. Application-Communications-Pfad) um `rendered_body`/`rendered_html_body` erweitern; **keine Schema-Migration** (Daten liegen in `mail_recipients` vor).

### Claude's Discretion
- Exaktes Preview-Layout, Debounce-Timing, i18n-Keys, Route-Name/-Pfad der Compose-Seite.
- Exakte Shape der `api.rs`-Funktionen (Send/Communications/Preview) — solange **dediziert** (nicht member-umgeleitet) und Phase-31-Endpoint-Shapes gespiegelt.
- Ob `rendered_body`/`rendered_html_body` direkt an `CommunicationEntryTO` hängen ODER über kleinen Detail-Endpoint (`GET …/communications/{recipient_id}`) — solange echter gespeicherter Body ohne Re-Render + Admin-Gate.
- Darstellung des Body-Detail-Panels (Modal vs. Inline-Expand).

### Deferred Ideas (OUT OF SCOPE)
- Massen-/Bulk-Erinnerung an alle `Offen`-Antragsteller (APMAIL-FUT-01).
- Freitext-Empfänger, Open-/Click-Tracking (Phase-31-Guardrails gelten weiter).
- Kein Neu-Rendern des Bodys für die Historie. Keine neuen auditierten Felder.
- Timeline-Klick-UX-Vertiefung (Deep-Link/Diff/Reply-aus-Historie) über D-06 hinaus.
- Original-Mitgliedsantrag als Attachment; HTML-Mail-Support (bereits umgesetzt Ph.23/24); DD.MM.YYYY-Template-Vars; RepaymentLetter-Pre-Flight/Bulk.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| APMAIL-03 | Fehlende E-Mail-Adresse sauber behandeln — Button deaktiviert/annotiert, nie stiller Fehlversuch | `is_email_empty` + disabled/title/Hint-Muster verifiziert in `member_details.rs:400-431`; Backend liefert zusätzlich 422 bei fehlender Adresse (`send_application_mail`, `application.rs:508`) |
| APMAIL-04 | Vorschau mit aufgelösten Platzhaltern + bewusster confirm-before-send | Preview-Endpoint `POST /api/applications/{id}/mail/preview` verifiziert (`application.rs:553`), Response `PreviewApplicationMailResponse{subject,body,body_html}`; D-05: Preview IS die Bestätigung |
| APUI-01 | „E-Mail senden"-Button öffnet Compose-Dialog (Muster `member_details.rs`); bei fehlender Adresse deaktiviert | D-01/D-02: eigene Route statt Modal-in-Modal (verifiziert: `application_detail.rs` ist selbst `Modal`); Button-Muster + `nav.push` verifiziert |
| APUI-02 | Dialog nutzt bestehende `component/mail_compose/`-Bausteine — kein geforktes UI | Exporte verifiziert (`mail_compose/mod.rs`): `MailSubjectInput`, `WysiwygEditor`+`plain_to_html`, `TemplateSelector`, `TemplatePreview`, `MailPreviewFrame`; `MailPage` demonstriert Assemblage |
| APUI-03 | Kommunikations-Historie über unveränderte prop-getriebene `communication_timeline.rs` | Komponente verifiziert (`CommunicationTimeline(entries: Vec<CommunicationEntryTO>)`); in `member_details.rs:1426` bereits so genutzt |
</phase_requirements>

## Summary

Diese Phase ist ein **Verifikations-/De-Risk-Research**: CONTEXT.md trägt bereits Plan-Tiefe mit Datei-/Zeilenreferenzen. Alle Canonical Refs wurden am Code geprüft — **alle Dateien existieren, alle zitierten Symbole/Patterns sind vorhanden** (Zeilennummern sind teils leicht gedriftet, per Symbol bestätigt). Die drei Phase-31-Endpoints (`POST …/mail`, `POST …/mail/preview`, `GET …/communications`) existieren, sind admin-gated und haben stabile Request/Response-Shapes in `genossi_rest_types`.

**Zwei nicht-triviale Landminen**, die CONTEXT.md nicht explizit benennt und die der Planner einplanen MUSS:

1. **Doppelte `rest-types`-Crate.** Das Frontend hat eine **eigene, handgepflegte** `rest-types`-Crate (`genossi-frontend/rest-types/`, Crate-Name `rest-types`), getrennt vom Backend `genossi_rest_types`. `CommunicationEntryTO`, `MailTemplateTO` etc. existieren **zweimal**. D-06 (Feld ergänzen) und die neuen Send/Preview-Typen erfordern Edits in **beiden** Crates bzw. lokale Structs in `api.rs`. Backend-Send/Preview-Request-Typen aus `genossi_rest_types` sind im Frontend **nicht** verfügbar.

2. **`TemplateSelector` filtert nicht und die Frontend-`MailTemplateTO` trägt kein `template_type`.** Für D-03 (Antragsteller-Filter) muss (a) das Frontend-`MailTemplateTO` um `template_type` erweitert werden (Backend liefert es bereits: `template_type: 'member'|'application'`) und (b) `TemplateSelector` einen optionalen Filter-Prop erhalten ODER der Compose-Page-Aufrufer filtert. Der Backend-List-Endpoint hat **keinen** Type-Query-Filter — Filterung passiert client-seitig.

**D-06 ist etwas mehr als „TO-Feld anhängen":** Das gerenderte Body-Feld ist in `mail_recipients` persistiert (verifiziert), aber der **DAO-Pfad selektiert es aktuell nicht**. `CommunicationEntry` (Domain-Struct), `CommunicationEntryDb` (FromRow), die SQL-Query in `get_application_communications` **und** beide `CommunicationEntryTO` müssen erweitert werden. Trotzdem: **keine Schema-Migration** nötig, Daten liegen vor.

**Primary recommendation:** Compose-Page 1:1 nach `MailPage` bauen (RequirePrivilege-Skelett), dedizierte `api.rs`-Funktionen `send_application_mail` / `preview_application_mail` / `get_application_communications` mit lokalen Request/Response-Structs in `api.rs` (spiegeln `genossi_rest_types`), D-06 additiv über **Option 1** (Feld an `CommunicationEntryTO` in beiden Crates + DAO-Query erweitern), Timeline-Klick über **dünnen Wrapper** um die unveränderte `CommunicationTimeline` (Klick-Handling ist heute nicht prop-basiert — siehe Landmine 3).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Compose-UI (Subject/Editor/Selector/Preview) | Frontend (Dioxus Page) | — | Reine UI-Assemblage aus `mail_compose/*`; Vorbild `MailPage` |
| Live-Preview-Rendering (aufgelöste Platzhalter) | API/Backend | Frontend (debounced fetch) | Render-Kernel muss identisch zum Worker sein (Ph.31 D-06); Frontend nur Anzeige |
| Mail-Versand (enqueue) | API/Backend (Service) | Frontend (Trigger) | `application_service().send_mail()` gated + validiert (422 no-email); Frontend nur Auslöser |
| Fehlende-E-Mail-Behandlung | Frontend (disabled Button) | API (422 als Backstop) | UX-Guard clientseitig; Server verweigert zusätzlich |
| Kommunikations-Historie lesen | API/Backend (CommunicationDao) | Frontend (Timeline-Anzeige) | admin-gated via `application_service().get()` |
| Body-Detail (echter gesendeter Body) | API/Backend (Feld ergänzen) | Frontend (Panel/Modal) | Body in `mail_recipients` persistiert; D-06 macht ihn im Application-Pfad sichtbar |
| „zuletzt gesendet" (Betreff+Status+Datum) | Frontend (Ableitung aus communications) | — | Kein eigener Endpoint nötig; erstes Outbound-Entry der communications-Liste |

## Standard Stack

Kein neuer Stack. Ausschließlich bestehende Workspace-Bausteine (verifiziert vorhanden):

| Baustein | Ort | Zweck | Verifiziert |
|----------|-----|-------|-------------|
| `MailSubjectInput`, `WysiwygEditor`+`plain_to_html`, `TemplateSelector`, `TemplatePreview`, `MailPreviewFrame`, `TemplateVarButtons`, `MailAttachmentPicker` | `component/mail_compose/mod.rs` | Compose-UI | ✓ Exporte gelesen |
| `CommunicationTimeline(entries: Vec<CommunicationEntryTO>)` | `component/communication_timeline.rs` | Historie | ✓ prop-getrieben |
| `Modal` | `component/modal.rs` | Body-Detail-Panel (falls Modal-Variante) | ✓ (in `application_detail.rs` importiert) |
| `show_toast` / `show_success_toast` | `component/toast.rs:28/68` | Post-Send-Erfolg | ✓ |
| `ErrorAlert { error, on_dismiss }` | `component/error_alert.rs` | Send-Fehler | ✓ (in `mail_page.rs` genutzt) |
| `RequirePrivilege { privilege, fallback }` + `PRIVILEGE_ADMIN` + `AccessDeniedPage` | `auth.rs` | Admin-Gate der Compose-Seite | ✓ (`mail_page.rs:198`) |
| `is_email_empty(email: Option<&str>) -> bool` | `member_details.rs:41` | No-Email-Guard | ✓ (inkl. Unit-Tests dort) |

**Installation:** keine. Toolchain nur via `nix develop --command <cmd>` (cargo/node/sqlx/dx). Diese Phase braucht überwiegend Read-only-Exploration + Edits.

## Package Legitimacy Audit

Nicht anwendbar — es werden **keine neuen externen Pakete** installiert. Alle verwendeten Crates/Komponenten sind bereits im Workspace.

## Architecture Patterns

### Verifizierte Canonical References

| Ref (CONTEXT) | Verifiziert | Anmerkung |
|---------------|-------------|-----------|
| `page/mail_page.rs` Compose-Blueprint | ✓ | `MailPage()` mit Signal-State (`subject`, `body`, `body_html`, `sending`, `selected_template_id`), Send-Button `disabled: *sending.read() || …`, `RequirePrivilege`-Skelett, `send_bulk_mail`-Flow, `gloo_timers::future::TimeoutFuture` (Debounce-Vorbild, Z.282) |
| `member_details.rs` Button-Muster | ✓ | `is_email_empty` (Z.41), disabled+title+italic-Hint Button (Z.400-431), `nav.push(Route::MailPage {})` (Z.421) |
| `member_details.rs` Timeline-Nutzung | ✓ | `CommunicationTimeline { entries: communications.read().clone() }` (Z.1426), `get_member_communications`-Aufruf (Z.237) |
| `mail_compose/mod.rs` Exporte | ✓ | vollständig wie in CONTEXT gelistet |
| `communication_timeline.rs` prop-getrieben | ✓ | `#[component] CommunicationTimeline(entries: Vec<CommunicationEntryTO>)` — **aber Klick ist heute hart `Link`→`Route::MailJobDetail`/`InboxDetail` (Z.69-98), kein Klick-Prop** (Landmine 3) |
| `application_detail.rs` ist selbst `Modal` | ✓ | `rsx!{ Modal { … } }` (Z.34) + zwei verschachtelte `Modal`-Confirm/Reject (Z.164-227) → D-01-Begründung bestätigt |
| `api.rs` Application/Preview/Comms-Funktionen | ✓ | `get_application` (Z.810), `confirm_application` (Z.817), `reject_application` (Z.874), `preview_mail` (Z.1153, **member-scoped**), `get_member_communications` (Z.1723), `get_mail_job_detail` (Z.1184) |
| `router.rs` Route-Registrierung | ✓ | `MailPage {}` `#[route("/mail")]` (Z.63); **kein `/applications/:id`-Route** existiert — ApplicationDetail ist Modal in `ApplicationsPage` |
| Backend `application.rs` Send/Preview/Comms | ✓ | `send_application_mail` (Z.511), `preview_application_mail` (Z.553), `get_application_communications` (Z.599); Routen Z.646-654; OpenAPI Z.666-668 |
| `genossi_mail/dao.rs` `CommunicationEntry` | ✓ (korrigiert) | Struct liegt bei **Z.277-294** (CONTEXT sagte 59-84 — das ist `MailRecipient`). Trägt `mail_job_id`, `recipient_id`, `outbound_status` — **aber KEIN `rendered_*`** (D-06-Arbeit) |
| `communication_rest.rs` `CommunicationEntryTO` + `From` | ✓ | Z.28-82; `From<&CommunicationEntry>`-Mapping vorhanden |
| `rest.rs` `MailRecipientTO`/`MailJobDetailTO` rendered_*-Mapping | ✓ | Blueprint für D-06-Mapping vorhanden |
| Persistierter Body in `mail_recipients` | ✓ | `MailRecipient.rendered_subject/rendered_body/rendered_html_body` (`dao.rs:74-79`); `mail_recipients`-Tabelle wird bereits von `get_application_communications` gejoint |
| Seed „Zahlungserinnerung" | ✓ | Migration `20260820000001_seed_zahlungserinnerung_template.sql`, feste UUID `00000000-0000-0000-0000-000000000003`, `template_type='application'` |

### System-Datenfluss (Compose-Seite)

```
[application_detail.rs (Modal)]
   ├── "✉ E-Mail senden"-Button (disabled wenn is_email_empty)
   │        └── onclick → nav.push(Route::ApplicationCompose { id: app_id })
   ├── "zuletzt gesendet: {Betreff} — {Status} am {Datum}"  (aus communications[0])
   └── Timeline-Abschnitt (CommunicationTimeline, Klick → Body-Detail-Panel)

[ApplicationCompose-Page]  (RequirePrivilege(PRIVILEGE_ADMIN))
   ├── load: get_application(id) + get_application_communications(id)
   ├── prominente "zuletzt gesendet"-Zeile (Anti-Doppelversand-Guard)
   ├── MailSubjectInput ─┐
   ├── TemplateSelector  │ (Antragsteller-gefiltert, Default = Zahlungserinnerung …0003)
   ├── WysiwygEditor ────┤──(typing, debounced)──► preview_application_mail(id, draft)
   ├── TemplatePreview ◄──┘  zeigt aufgelöste Platzhalter (Backend-Render-Kernel)
   └── "E-Mail senden" (div+onclick, disabled während sending)
            └── send_application_mail(id, {subject, body, body_html, template_id})
                   └── Erfolg → show_success_toast + nav.push(Route::ApplicationsPage {})
```

### Pattern 1: Send-Button ohne `form onsubmit` (D-05)
**Was:** Klick-Auslöser als `div`/`button` mit explizitem `onclick` + `r#type: "button"`, kein `form`+`onsubmit`.
**Warum:** In Dioxus 0.6 WASM reloadet `form onsubmit` trotz `prevent_default` die Seite (Memory-Lesson, Vorbild `repayment_phases.rs`). `MailPage` verwendet bereits ein reines `button { onclick … }` (Z.512-633) — 1:1 übernehmbar.
**Doppelversand-Guard:** `disabled: *sending.read() || subject.read().is_empty()` (wie `mail_page.rs:514`).

### Pattern 2: Debounced Preview (D-04)
**Was:** Bei Editor-Change einen Timer setzen; nach Ruhe (~300-500ms, Discretion) `preview_application_mail` rufen; während Pending letzte aufgelöste Preview stehen lassen (kein Flackern).
**Vorbild:** `gloo_timers::future::TimeoutFuture::new(<ms>).await` in `spawn` (bereits in `mail_page.rs:282` genutzt). Guard gegen Race: generation-Counter oder „nur letzten Request anzeigen".

### Pattern 3: Antragsteller-gefilterter TemplateSelector (D-03)
**Was:** Nur `template_type == "application"`-Vorlagen anbieten; Default = Seed …0003.
**Empfehlung (siehe Landmine 2):** Frontend-`MailTemplateTO` um `#[serde(default)] pub template_type: String` erweitern; `TemplateSelector` optionalen Prop `#[props(default)] filter_type: Option<String>` geben, der `templates.read()` clientseitig filtert. Backward-kompatibel (bestehende `MailPage`-Nutzung unverändert). Vermeidet strict-render-Bombe: Member-Vorlagen mit `{{ member.* }}`-Platzhaltern würden gegen den Application-Context strict fehlschlagen.

### Anti-Patterns to Avoid
- **Modal-in-Modal:** Compose NICHT als weiteres `Modal` in `application_detail.rs` rendern (dieses ist bereits `Modal`). D-01: eigene Route.
- **Member-Funktionen umleiten:** `preview_mail` (member-scoped, `/api/mail/preview` mit `member_id`) NICHT für Application nutzen — dedizierte `preview_application_mail` gegen `/api/applications/{id}/mail/preview`.
- **Body neu-rendern für Historie:** D-06 zeigt den **gespeicherten** `rendered_body`, kein Live-Re-Render (könnte ≠ real Gesendetem sein).
- **`CommunicationTimeline` forken/umstylen:** unverändert lassen (APUI-03). Klick-Erweiterung additiv (Wrapper).

## Don't Hand-Roll

| Problem | Nicht selbst bauen | Stattdessen | Warum |
|---------|--------------------|-------------|-------|
| Compose-UI (Subject/Editor/Selector/Preview) | Inline-RSX auf der Seite | `mail_compose/*`-Komponenten | Component-First-Regel; `MailPage` zeigt Assemblage |
| Timeline-Tabelle | Neue Tabelle | `CommunicationTimeline` unverändert | APUI-03 |
| Platzhalter-Auflösung/Preview | Client-seitiges Rendering | Backend `preview_application_mail` | Muss identisch zum Worker sein (Ph.31 D-06) |
| Admin-Gate | Eigener Check | `RequirePrivilege(PRIVILEGE_ADMIN)` + Backend gated bereits | Konsistenz; Backend ist Source of Truth |
| No-Email-Erkennung | Eigene Prüfung | `is_email_empty` aus `member_details.rs` (ggf. in Component heben) | bereits getestet |
| Toast/Fehler | Eigenes UI | `show_success_toast` / `ErrorAlert` | etabliert |

**Key insight:** Fast alles existiert. Der Neubau beschränkt sich auf: 1 neue Page + 1 Route, 3 dedizierte `api.rs`-Funktionen (+ lokale Request/Response-Structs), Button+Zeile+Timeline-Abschnitt in `application_detail.rs`, ein Body-Detail-Panel/Wrapper, und die additive D-06-Backend-Erweiterung.

## Common Pitfalls

### Pitfall 1: Doppelte `rest-types`-Crate (Landmine 1 — HIGH Impact)
**Was schiefgeht:** Man erweitert nur `genossi_mail/src/communication_rest.rs::CommunicationEntryTO` und wundert sich, dass das Frontend das Feld nicht kennt.
**Root cause:** Frontend nutzt `genossi-frontend/rest-types/` (Crate `rest-types`), eine **handgepflegte Kopie**, NICHT `genossi_rest_types`. `CommunicationEntryTO` liegt dort separat bei `rest-types/src/lib.rs:901-920` (Feld `direction: CommunicationDirection` als Enum statt `String` — wire-kompatibel).
**Vermeidung:** D-06 = Edit in **beiden** `CommunicationEntryTO`-Definitionen (Backend + Frontend-`rest-types`) als additives `#[serde(skip_serializing_if="Option::is_none")] rendered_body/rendered_html_body: Option<String>`. Send/Preview-Request+Response-Typen sind im Frontend nicht verfügbar → **lokale Structs in `api.rs`** (wie bestehendes `PreviewRequest`/`PreviewResponse` Z.1116/1133).
**Frühwarnzeichen:** `cargo check -p genossi-frontend` meldet unbekanntes Feld.

### Pitfall 2: `CommunicationEntry`/DAO selektiert `rendered_*` nicht (D-06 Kern)
**Was schiefgeht:** Annahme „Daten liegen vor, also nur TO anhängen". Die Domain-Struct `CommunicationEntry` (`dao.rs:277-294`) und die `CommunicationEntryDb` FromRow-Struct (`dao_sqlite.rs:1013-1026`) haben KEINE rendered-Felder; die SQL in `get_application_communications` (`dao_sqlite.rs:1138-1160`) selektiert sie nicht.
**Vermeidung (Option 1, empfohlen):** additive Kette:
1. `CommunicationEntry` (dao.rs): Felder `rendered_body`/`rendered_html_body: Option<Arc<str>>`.
2. `CommunicationEntryDb` (dao_sqlite.rs): `rendered_body: Option<String>`, `rendered_html_body: Option<String>`.
3. SQL in `get_application_communications`: `r.rendered_body`, `r.rendered_html_body` statt/zusätzlich zu NULL-Platzhaltern (Spalten existieren in `mail_recipients`). `get_member_communications` optional gleichziehen (Konsistenz) oder bewusst nur Application-Pfad — Scope-Entscheidung.
4. `TryFrom<&CommunicationEntryDb>` (dao_sqlite.rs:1028) mappt die zwei Felder.
5. Beide `CommunicationEntryTO` + `From`-Impl (Backend `communication_rest.rs:66` + Frontend `rest-types`).
**Keine** Schema-Migration — `mail_recipients.rendered_body/rendered_html_body` existieren seit Ph.23/Quick 260614.
**Alternative (Option 1b, Discretion):** kleiner Detail-Endpoint `GET /api/applications/{id}/communications/{recipient_id}` der nur den Body liefert → schlankere Timeline-Payload, aber zusätzlicher Handler/Route/OpenAPI/api.rs-Funktion. **Empfehlung: Option 1** (weniger Round-Trips, Body ist klein, Mapping-Blueprint via `MailJobDetailTO` vorhanden).

### Pitfall 3: `CommunicationTimeline`-Klick ist nicht prop-basiert (Landmine 3)
**Was schiefgeht:** CONTEXT nimmt an, Klick→Body sei „nur additiv" per Prop ergänzbar. Tatsächlich rendert jede Zeile heute einen harten `Link { to: Route::MailJobDetail{…} }` auf den Betreff (`communication_timeline.rs:69-98`) — **kein** `on_row_click`-Prop.
**Vermeidung:** Zwei Optionen:
- (a) `CommunicationTimeline` um optionalen `#[props(default)] on_entry_click: EventHandler<CommunicationEntryTO>` erweitern; wenn gesetzt → `div`+`onclick` statt `Link` (Member-Nutzung ohne Handler bleibt = alter `Link`). Additiv, backward-kompatibel, hält APUI-03 („unverändert" im Verhalten für Member).
- (b) **Dünner Wrapper** `ApplicationCommunicationTimeline` im Application-Kontext (CONTEXT nennt diesen Fallback explizit), der die Klick-→-Body-Panel-Logik hält und intern die unveränderte `CommunicationTimeline` nutzt — falls (a) die Member-Nutzung visuell/verhaltensmäßig verändern würde.
**Empfehlung:** (a) mit `#[props(default)]` (bevorzugt, Component-First, kein Duplikat), Fallback (b) falls Review „unverändert" strikt auslegt.

### Pitfall 4: Frontend `MailTemplateTO` ohne `template_type` (D-03)
**Was schiefgeht:** Client kann nicht auf Antragsteller-Vorlagen filtern → strict-render-Bombe bei Member-Vorlagen.
**Vermeidung:** Frontend-`MailTemplateTO` (`api.rs`) um `#[serde(default)] pub template_type: String` erweitern (Backend `rest_templates.rs:32` liefert es bereits). Filter clientseitig (List-Endpoint hat keinen Type-Query-Param). Siehe Pattern 3.

### Pitfall 5: Post-Send-Navigation verliert Modal-Kontext
**Was schiefgeht:** `application_detail.rs` ist ein Modal in `ApplicationsPage`; Rücksprung nach `/applications` zeigt die Liste ohne offenes Detail-Modal.
**Vermeidung:** Akzeptabel per CONTEXT („zurück zur Application-Detailansicht" = Liste + Toast). Falls Detail-Reopen gewünscht: Query-Param `?open=<app_id>` an `ApplicationsPage` oder Toast genügt. Kein Blocker; Discretion.

### Pitfall 6: „zuletzt gesendet"-Ableitung
**Was:** Kein eigener Endpoint. `get_application_communications` liefert Outbound-Entries `ORDER BY date DESC` (`dao_sqlite.rs:1159`). Der erste Outbound-Eintrag = zuletzt gesendet → `{subject} — {outbound_status} am {date}`. Leerer Fall → `Key::NeverSent`.

## Code Examples

### Dedizierte api.rs-Funktionen (Signaturen, spiegeln genossi_rest_types)
```rust
// Source: genossi_rest/src/application.rs:511/553/599 + genossi_rest_types/src/lib.rs:1172-1201
// LOKALE Structs in api.rs (Frontend-rest-types kennt SendApplicationMailRequest NICHT)

#[derive(serde::Serialize)]
struct SendApplicationMailReq {
    subject: String, body: String,
    #[serde(skip_serializing_if = "Option::is_none")] body_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] template_id: Option<uuid::Uuid>,
}
pub async fn send_application_mail(
    config: &Config, id: Uuid, subject: &str, body: &str,
    body_html: Option<&str>, template_id: Option<Uuid>,
) -> Result<(), AppError> {
    let url = format!("{}/api/applications/{}/mail", config.backend, id);
    let req = SendApplicationMailReq { /* … */ };
    let resp = reqwest::Client::new().post(url).json(&req).send().await?;
    check_response(resp).await?; Ok(())   // 200 body: {"status":"queued"}
}

#[derive(serde::Serialize)] struct PreviewApplicationMailReq { subject: String, body: String, body_html: Option<String> }
#[derive(serde::Deserialize)] pub struct ApplicationPreviewResponse { pub subject: String, pub body: String, pub body_html: Option<String> }
pub async fn preview_application_mail(
    config: &Config, id: Uuid, subject: &str, body: &str, body_html: Option<&str>,
) -> Result<ApplicationPreviewResponse, AppError> {
    let url = format!("{}/api/applications/{}/mail/preview", config.backend, id);
    /* POST json → check_response → json() */
}

pub async fn get_application_communications(
    config: &Config, id: Uuid,
) -> Result<Vec<rest_types::CommunicationEntryTO>, AppError> {
    let url = format!("{}/api/applications/{}/communications", config.backend, id);
    let resp = check_response(reqwest::get(url).await?).await?;
    Ok(resp.json().await?)
}
```

### Button-Muster in application_detail.rs (spiegelt member_details.rs:400-431)
```rust
// Source: genossi-frontend/src/page/member_details.rs:400-431
let email_empty = is_email_empty(application.email.as_deref());
rsx! {
    button {
        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed",
        disabled: email_empty,
        title: if email_empty { i18n.t(Key::NoEmailAddressHint).to_string() } else { String::new() },
        onclick: move |_| { if !email_empty { nav.push(Route::ApplicationCompose { id: app_id.to_string() }); } },
        "✉ {i18n.t(Key::MailSendButton)}"
    }
    if email_empty { span { class: "text-sm text-gray-500 italic", {i18n.t(Key::NoEmailAddressHint)} } }
}
```

### Neue Route (router.rs, Muster MailPage)
```rust
// Source: genossi-frontend/src/router.rs:62-63
#[route("/applications/:id/compose")]
ApplicationCompose { id: String },
```

## Runtime State Inventory

Kein Rename/Refactor/Migration — greenfield-artige Ergänzung. Abschnitt entfällt bzgl. gespeicherter State-Umbenennungen. Relevanter Datenstatus (verifiziert): `mail_recipients.rendered_body/rendered_html_body/rendered_subject` bereits befüllt (inkl. Backfill Quick 260614-b1t für Legacy-Zeilen; `rendered_reconstructed`-Flag markiert rekonstruierte). **Keine neue Migration**, **keine** neuen auditierten Felder (GV-/Application-Mail-Entitäten sind nicht audit-pflichtig).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Debounce ~300-500ms ist angemessen | Pattern 2 | niedrig — Discretion (D-04); frei justierbar |
| A2 | Option 1 (Feld an TO) schlägt Detail-Endpoint | Pitfall 2 | niedrig — beide von CONTEXT erlaubt; Detail-Endpoint bleibt gültige Alternative |
| A3 | Compose-Route-Pfad `/applications/:id/compose` | Code Examples | niedrig — Route-Name ist Discretion |
| A4 | Timeline-Klick via `#[props(default)] on_entry_click` (Variante a) | Pitfall 3 | mittel — falls Review „unverändert" strikt liest, Wrapper (Variante b) nötig |
| A5 | „zuletzt gesendet" aus communications[0] ableitbar (kein Extra-Endpoint) | Pitfall 6 | niedrig — `ORDER BY date DESC` verifiziert |

## Open Questions

1. **`get_member_communications` bei D-06 mit-erweitern?**
   - Bekannt: Nur der Application-Pfad braucht den Body-View laut Scope.
   - Unklar: Ob Member-Timeline denselben Klick→Body künftig will.
   - Empfehlung: `CommunicationEntry`-Struct trägt die Felder ohnehin gemeinsam; SQL in `get_member_communications` optional gleich mit-selektieren (billig), aber Frontend-Body-Panel nur im Application-Kontext ausspielen. Scope-Minimal: nur Application-SQL erweitern.

2. **`template_id` beim Send mitschicken?**
   - Backend `SendApplicationMailRequest` akzeptiert optional `template_id: Option<Uuid>` (Server rendert dann via Template gegen Application-Context).
   - Empfehlung: Ja — `selected_template_id` (aus `TemplateSelector::on_select_id`) mitsenden, damit Server-Render == Preview-Render. Default …0003.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo/rustc | Build/Test Backend+Frontend | ✓ (nur via `nix develop --command`) | Workspace 2021 | — |
| node/npx (Tailwind) | Frontend-CSS-Build | ✓ (Nix-Devshell) | — | — |
| dx (Dioxus CLI) | Frontend serve/build | ✓ (Nix-Devshell) | 0.6 | — |
| sqlx-cli | Migrations (hier keine neue) | ✓ (Nix-Devshell) | — | n/a diese Phase |

**Hinweis:** Toolchain fehlt auf Base-PATH — alle Builds/Tests über `nix develop --command <cmd>` (Memory-Lesson). `cargo fmt` reformatiert workspace-weit ~24 Fremddateien → gezielt `cargo fmt -p <crate>` bzw. nur geänderte Dateien committen.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (Backend Unit + `#[tokio::test]`, e2e via reqwest); Frontend inline `#[cfg(test)]`-Module (reine Logik) |
| Config file | keine — Workspace-Cargo |
| Quick run command | `nix develop --command cargo test -p genossi_mail` (D-06 DAO/TO) |
| Full suite command | `nix develop --command cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| APMAIL-03 | No-Email → 422 / disabled | e2e (Backend) + unit (`is_email_empty`) | `cargo test -p genossi_bin --test e2e_tests test_admin_create_application_send_mail_without_email` | ✅ (`e2e_tests.rs:7653`) |
| APMAIL-04 | Preview liefert aufgelöste Platzhalter | e2e (Backend) | `cargo test -p genossi_bin --test e2e_tests` (Preview-Endpoint) | ⚠️ Preview-e2e ggf. Wave 0 ergänzen |
| APUI-01/02 | Button/Compose/Components | Frontend `#[cfg(test)]` (Logik) + manuell (WASM-UI) | `cargo test -p genossi-frontend` | ✅ Muster vorhanden (`is_email_empty`-Tests, `template_preview.rs`-Tests) |
| APUI-03 | Timeline-Anzeige | Frontend-Logik + manuell | `cargo test -p genossi-frontend` | ✅ |
| D-06 | `rendered_body` im Application-Comms-Pfad | DAO-`#[tokio::test]` + e2e | `cargo test -p genossi_mail get_application_communications` | ⚠️ **Wave 0**: neuen DAO-Test für rendered-Felder ergänzen (Vorbild `e2e_tests.rs:6847`) |

### Sampling Rate
- **Per task commit:** `nix develop --command cargo test -p <geänderte-crate>`
- **Per wave merge:** `nix develop --command cargo test`
- **Phase gate:** volle Suite grün + `cargo clippy` sauber vor `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] DAO-Test: `get_application_communications` gibt `rendered_body`/`rendered_html_body` zurück (erweitert bestehenden Test bei `dao_sqlite.rs`-Testmodul / `e2e_tests.rs:6847`).
- [ ] e2e: `POST /api/applications/{id}/mail/preview` liefert aufgelöste Platzhalter (falls noch nicht vorhanden — prüfen; Send-with/without-email e2e existieren bereits Z.7653/7682).
- [ ] Frontend-Logik-Test: Antragsteller-Filter im TemplateSelector (template_type == "application") — reine Filterfunktion testbar ohne WASM.
- [ ] Frontend-Logik-Test: „zuletzt gesendet"-Ableitung aus `Vec<CommunicationEntryTO>` (erstes Outbound-Entry, Empty→NeverSent).
- Projekt-/User-Regel: **Änderungen brauchen Tests** (CLAUDE.md global). WASM-Rendering ist nicht unit-testbar → Logik in testbare freie Funktionen ziehen (Muster: `is_email_empty`).

## Security Domain

`security_enforcement` nicht als `false` markiert → Abschnitt enthalten.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V4 Access Control | yes | Admin-Gate `MANAGE_MEMBERS_PRIVILEGE` auf allen Application-Mail-Endpoints (Ph.31 D-10, verifiziert: `get_application_communications` ruft `application_service().get()` als Gate zuerst, `application.rs:611`); Frontend `RequirePrivilege(PRIVILEGE_ADMIN)` |
| V5 Input Validation | yes | Server validiert (422 bei fehlender E-Mail); kein Freitext-Empfänger (Ph.31 D-13); Subject/Body serverseitig verarbeitet |
| V6 Cryptography | no | — |
| V2 Auth / V3 Session | no (bestehend) | Session/OIDC-Middleware unverändert |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Info-Disclosure: Body-View eines fremden Antragstellers | Information Disclosure | Body nur über admin-gated Application-Pfad (`get()` prüft Permission→401/404 vor DAO); recipient_id an application_id gebunden |
| Doppelversand / Spam | Elevation/Misuse | Send-Button während Request `disabled` (D-05); „zuletzt gesendet"-Guard prominent auf Compose-Seite |
| HTML-Body-Injection in Detail-Panel | Tampering/XSS | Body ist bereits ammonia-sanitized beim Speichern (Ph.23 D-03); Anzeige als `whitespace-pre-wrap`-Text bzw. sanitized HTML im bounded Scroll-Container (Muster `MailJobDetail`) |
| Stiller Fehlversand ohne E-Mail | Repudiation | 422 serverseitig + disabled Button clientseitig (APMAIL-03) |

## State of the Art

| Old Approach | Current Approach | When | Impact |
|--------------|------------------|------|--------|
| Member-scoped `preview_mail`/`send_bulk_mail` | Dedizierte Application-Endpoints (`/applications/{id}/mail[/preview]`, `/communications`) | Ph.31 | Frontend nutzt neue dedizierte api.rs-Funktionen, keine Member-Umleitung |
| `MailBodyEditor` (textarea) | `WysiwygEditor` (contenteditable) + `plain_to_html` | Ph.24 | Compose-Seite nutzt WysiwygEditor wie `MailPage` |
| Body nur beim Job, nicht per Empfänger | per-Empfänger `rendered_body/html_body/subject` in `mail_recipients` | Ph.23 + Quick 260614 (+Backfill) | D-06 kann echten Body zeigen ohne Re-Render |

**Deprecated/outdated:** `component/mail_compose/body_editor.rs` gelöscht (Ph.24) — nicht referenzieren.

## Sources

### Primary (HIGH confidence)
- Codebase (grep/read, verifiziert diese Session): `genossi-frontend/src/page/{mail_page,member_details}.rs`, `component/mail_compose/mod.rs`, `component/communication_timeline.rs`, `component/application_detail.rs`, `src/router.rs`, `src/api.rs`, `genossi-frontend/rest-types/src/lib.rs`
- Backend: `genossi_rest/src/application.rs`, `genossi_rest_types/src/lib.rs:1168-1201`, `genossi_mail/src/{dao.rs,dao_sqlite.rs,communication_rest.rs,rest_templates.rs,rest.rs}`
- `migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql`; `genossi_bin/tests/e2e_tests.rs` (Comms/Send-Tests)
- `.planning/phases/32-frontend-compose-dialog/{32-CONTEXT.md,32-UI-SPEC.md}`, `.planning/REQUIREMENTS.md`

### Secondary
- CLAUDE.md (Projekt + genossi-frontend), MEMORY.md (form-onsubmit-Reload-Lesson, node-PATH, nix-develop-Toolchain)

## Project Constraints (from CLAUDE.md)

- **Layered DAO/Service/REST einhalten**; neue Backend-Felder folgen bestehenden Trait-/TO-Mapping-Patterns (D-06 additiv).
- **Component-First (Frontend):** keine inline-RSX-Duplikate; wiederverwendbare Komponenten in `genossi-frontend/src/component/`. Compose-Seite komponiert `mail_compose/*`.
- **i18n:** neue Keys in **beiden** Locales `De` + `En` (`i18n/mod.rs` Enum + `de.rs`/`en.rs`); nur `En`/`De` existieren.
- **Tests Pflicht** (User-Global + Projekt): jede Änderung testabgedeckt; WASM-Logik in testbare freie Funktionen ziehen.
- **Toolchain nur via `nix develop --command`**; `cargo fmt` gezielt einsetzen (workspace-fmt reformatiert Fremddateien).
- **GSD-Workflow:** Edits nur innerhalb GSD-Command-Kontext.
- **Keine Audit-Macros** für GV-/Application-Mail-Entitäten nötig (nur Member/MemberAction/MemberDocument/Application-Kern sind audit-pflichtig; hier keine neuen auditierten Felder).

## Metadata

**Confidence breakdown:**
- Standard Stack / Reuse: HIGH — alle Komponenten/Exporte am Code gelesen.
- Architektur/Patterns: HIGH — Send/Preview/Comms-Endpoints + Request/Response-Typen verifiziert.
- D-06-Backend-Umfang: HIGH — Struct/FromRow/SQL/TO-Kette präzise lokalisiert; keine Migration nötig bestätigt.
- Landminen (dual rest-types, TemplateSelector-Filter, Timeline-Klick): HIGH — direkt am Code belegt.

**Research date:** 2026-08-21
**Valid until:** ~2026-09-20 (stabile interne Codebase; bei Merge fremder mail/application-Änderungen erneut prüfen)
