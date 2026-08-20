# Phase 32: Frontend Compose-Dialog — Pattern Map

**Mapped:** 2026-08-21
**Files analyzed:** 11 (5 neu/erweitert Frontend, 5 erweitert Backend, 1 Route)
**Analogs found:** 11 / 11 (alle am Code verifiziert)

> Downstream-Hinweis: RESEARCH.md korrigiert mehrere CONTEXT.md-Zeilennummern und benennt 3 Landminen
> (doppelte `rest-types`-Crate, TemplateSelector-Filter, Timeline-Klick nicht prop-basiert). Diese Map
> nutzt die **verifizierten** Fundstellen aus dieser Session.

## File Classification

| Neu/Geändert | Rolle | Data Flow | Nächster Analog | Match |
|--------------|-------|-----------|-----------------|-------|
| `genossi-frontend/src/page/application_compose.rs` (NEU) | page | request-response (fetch+submit) | `genossi-frontend/src/page/mail_page.rs` | exact |
| `genossi-frontend/src/component/application_detail.rs` (EDIT: Button + „zuletzt gesendet" + Timeline) | component | request-response | `genossi-frontend/src/page/member_details.rs` (Button-Block Z.395-433) | role+flow |
| `genossi-frontend/src/api.rs` (EDIT: 3 dedizierte Fn + lokale Structs) | api-client | request-response | `preview_mail`/`PreviewRequest`/`PreviewResponse` (Z.1115-1175) | exact |
| `genossi-frontend/src/router.rs` (EDIT: neue Route) | route | — | `MailPage {}` `#[route("/mail")]` (Z.62-63) | exact |
| `genossi-frontend/src/component/communication_timeline.rs` (EDIT additiv: `on_entry_click`) | component | event-driven | eigener Prop-Zusatz (Variante a) | self / wrapper-fallback |
| `genossi-frontend/rest-types/src/lib.rs` (EDIT: `CommunicationEntryTO` + rendered_*) | model/TO | transform | Backend `communication_rest.rs::CommunicationEntryTO` (Z.28-47) | mirror |
| `genossi_mail/src/communication_rest.rs` (EDIT: TO-Feld + From) | model/TO | transform | `rest.rs::MailRecipientTO` From-Mapping (Z.343-359) | blueprint |
| `genossi_mail/src/dao.rs` (EDIT: `CommunicationEntry` + rendered_*) | model | — | Struct Z.277-294 (self, additiv) | self |
| `genossi_mail/src/dao_sqlite.rs` (EDIT: `CommunicationEntryDb` + SQL + TryFrom) | dao | CRUD/read | `get_application_communications` Z.1130-1169 + `CommunicationEntryDb` Z.1012-1057 | self |
| `genossi_rest/src/application.rs` (KONSUM: Send/Preview/Comms, keine Query-Änderung nötig) | controller | request-response | bestehende Handler Z.511/553/599 | reference |
| i18n `de.rs`/`en.rs` + `i18n/mod.rs` (EDIT: neue Keys) | config | — | bestehende Key-Einträge | exact |

## Pattern Assignments

### `application_compose.rs` (page, request-response) — NEU

**Analog:** `genossi-frontend/src/page/mail_page.rs`

**Imports + Skelett** (`mail_page.rs:1-31`): `use dioxus::prelude::*;`, `use rest_types::…`,
`use crate::component::mail_compose::{plain_to_html, MailSubjectInput, TemplatePreview, TemplateSelector, TemplateVarButtons, WysiwygEditor};`,
`use crate::component::{show_toast, ErrorAlert, ToastContainer, TopBar};`, `use crate::auth::RequirePrivilege;`,
`use crate::page::AccessDeniedPage;`, `use crate::service::config::CONFIG;`.

**Signal-State** (`mail_page.rs:58-77`): `subject`, `body`, `body_html`, `sending`, `selected_template_id`
1:1 übernehmen. Zusätzlich: `application` + `communications` per `use_resource`/`spawn` beim Mount laden
(Analog: `get_member_communications`-Aufruf in `member_details.rs:237`).

**Layout-Gerüst** (UI-SPEC Z.40-41): `RequirePrivilege(PRIVILEGE_ADMIN)` → `div.flex.flex-col.min-h-screen`
→ `TopBar` → `div.flex-1.container.mx-auto.px-4.py-8` → `h1.text-3xl.font-bold.mb-6` → Banner →
Compose-Card `div.bg-white.rounded-lg.shadow.p-6.mb-6`. Fallback `AccessDeniedPage` wie `mail_page.rs:198`.

**Send-Button ohne form (D-05):** reines `button { onclick, r#type:"button", disabled: *sending.read() || subject.read().is_empty() }`
(Muster `mail_page.rs`-Send-Button; Reload-Falle via `div`/`button`+`onclick` — Vorbild `repayment_phases.rs`).

**Debounce-Preview (D-04):** `gloo_timers::future::TimeoutFuture::new(<ms>).await` in `spawn`
(bereits in `mail_page.rs:282`); letzten aufgelösten Preview stehen lassen (kein Flackern).

---

### `application_detail.rs` (component, EDIT) — Button + „zuletzt gesendet" + Timeline

**Analog:** `genossi-frontend/src/page/member_details.rs:395-433`

**Button-Muster (D-02, APMAIL-03)** — 1:1 übertragbar:
```rust
// member_details.rs:400-431
let email_empty = is_email_empty(member.read().email.as_deref());
let disabled = email_empty || member_id_for_mail.is_none();
let title = if email_empty { i18n.t(Key::NoEmailAddressHint).to_string() } else { String::new() };
button {
    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed",
    disabled, title,
    onclick: move |_| { /* nav.push(Route::MailPage {}) */ },
    "✉ {i18n.t(Key::MailSendButton)}"
}
if email_empty { span { class: "text-sm text-gray-500 italic", {i18n.t(Key::NoEmailAddressHint)} } }
```
Für Application: `onclick` → `nav.push(Route::ApplicationCompose { id: app_id.to_string() })`,
`is_email_empty(application.email.as_deref())`.

**Modal-in-Modal-Verbot (D-01):** `application_detail.rs` ist selbst ein `Modal` (`rsx!{ Modal { … } }`),
mit verschachtelten Confirm/Reject-`Modal`s → Compose MUSS eigene Route sein, kein weiteres Modal.

**„zuletzt gesendet" (D-06):** aus `communications[0]` (Outbound, `ORDER BY date DESC`):
`{subject} — {outbound_status} am {date}`; Empty → `Key::NeverSent`.

**Timeline-Abschnitt:** `CommunicationTimeline { entries: communications.read().clone() }`
(Muster `member_details.rs:1426`).

---

### `api.rs` (api-client, EDIT) — 3 dedizierte Funktionen + lokale Structs

**Analog:** `preview_mail` + `PreviewRequest`/`PreviewResponse` (`api.rs:1115-1175`)

**Warum lokale Structs (Landmine 1):** Frontend nutzt `genossi-frontend/rest-types/` (Crate `rest-types`),
NICHT `genossi_rest_types`. Send/Preview-Request-Typen sind dort nicht verfügbar → lokale `#[derive(Serialize/Deserialize)]`-Structs
direkt in `api.rs` (exakt wie `PreviewRequest`/`PreviewResponse`).

**Funktions-Muster** (spiegelt Phase-31-Endpoints, siehe RESEARCH Code Examples):
- `send_application_mail(config, id, subject, body, body_html, template_id)` → `POST /api/applications/{id}/mail`
- `preview_application_mail(config, id, subject, body, body_html)` → `POST /api/applications/{id}/mail/preview`, Response `{subject, body, body_html}`
- `get_application_communications(config, id) -> Vec<rest_types::CommunicationEntryTO>` → `GET /api/applications/{id}/communications`

Konvention: `reqwest::Client::new().post(url).json(&req).send().await?` → `check_response(resp).await?` → `resp.json()`
(exakt `preview_mail` Z.1172-1174). **Keine** Umleitung der member-scoped `preview_mail`.

**D-03 Filter:** Frontend-`MailTemplateTO` (in `api.rs`) um `#[serde(default)] pub template_type: String` erweitern
(Backend liefert es via `rest_templates.rs:32`); Filter client-seitig (List-Endpoint hat keinen Type-Query-Param).

---

### `router.rs` (route, EDIT)

**Analog:** `MailPage {}` `#[route("/mail")]` (Z.62-63)
```rust
#[route("/applications/:id/compose")]
ApplicationCompose { id: String },
```
Reihenfolge beachten (spezifische vor generischen Routen — siehe `/mail/jobs` vor `/mail/jobs/:id` Z.66-69).

---

### `communication_timeline.rs` (component, EDIT additiv) — Klick→Body

**Analog / Landmine 3:** Heute rendert jede Zeile einen harten `Link { to: Route::MailJobDetail{…} }`
auf den Betreff (`communication_timeline.rs:69-98`) — **kein** Klick-Prop.

**Empfohlen (Variante a, Component-First):** optionalen Prop
`#[props(default)] on_entry_click: Option<EventHandler<CommunicationEntryTO>>` ergänzen; wenn gesetzt →
`div`+`onclick` statt `Link` (Member-Nutzung ohne Handler bleibt = alter `Link`, APUI-03 „unverändert" gewahrt).

**Fallback (Variante b):** dünner Wrapper `ApplicationCommunicationTimeline`, der die unveränderte
`CommunicationTimeline` nutzt und die Klick→Body-Panel-Logik hält (CONTEXT nennt diesen Fallback explizit).

**Body-Detail-Panel:** `Modal` (`component/modal.rs`) oder Inline-Expand (Discretion); zeigt gespeicherten
`rendered_html_body`/`rendered_body` in `max-h-96 overflow-auto whitespace-pre-wrap` (Muster `MailJobDetail`).
**Nie** neu-rendern.

---

### D-06 Backend-Kette (additiv, KEINE Schema-Migration)

Reihenfolge der Edits (jede Stufe ist Voraussetzung der nächsten):

1. **`genossi_mail/src/dao.rs`** (`CommunicationEntry`, Z.277-294): Felder
   `rendered_body: Option<Arc<str>>`, `rendered_html_body: Option<Arc<str>>` ergänzen (bei den Outbound-Feldern).

2. **`genossi_mail/src/dao_sqlite.rs`** (`CommunicationEntryDb`, Z.1012-1026): `rendered_body: Option<String>`,
   `rendered_html_body: Option<String>` ergänzen. **TryFrom** (Z.1041-1055) mappt die zwei Felder
   (`db.rendered_body.as_deref().map(Arc::from)`).

3. **SQL in `get_application_communications`** (Z.1138-1160): NULL-Platzhalter-Prinzip beibehalten, aber
   `r.rendered_body`, `r.rendered_html_body` selektieren (Spalten existieren in `mail_recipients`).
   `get_member_communications` bewusst nur optional gleichziehen (Scope: Application-Pfad genügt).

4. **`genossi_mail/src/communication_rest.rs`** (`CommunicationEntryTO` Z.28-47 + `From` Z.54-81):
   Felder als `#[serde(skip_serializing_if="Option::is_none")] rendered_body/rendered_html_body: Option<String>`.
   **Mapping-Blueprint:** `rest.rs::MailRecipientTO`-From (Z.352-357):
   `rendered_body: r.rendered_body.as_deref().map(String::from)`.

5. **`genossi-frontend/rest-types/src/lib.rs`** (`CommunicationEntryTO` Z.901-920): **dieselben** additiven
   Felder (Landmine 1 — separate handgepflegte Crate). `direction` ist hier `CommunicationDirection`-Enum,
   Rest wire-kompatibel.

**Handler `application.rs:599` bleibt unverändert** — er reicht die DAO-Entries via `From` durch.

## Shared Patterns

### Admin-Gate
**Quelle:** `auth.rs` — `RequirePrivilege { privilege: PRIVILEGE_ADMIN, fallback: AccessDeniedPage }` (`mail_page.rs:198`)
**Apply to:** Compose-Page. Backend zusätzlich gated (Ph.31 D-10, `MANAGE_MEMBERS_PRIVILEGE`, `application.rs:611`).

### Fehler / Erfolg
**Quelle:** `component/error_alert.rs` (`ErrorAlert { error, on_dismiss }`, in `mail_page.rs` genutzt);
`component/toast.rs` (`show_toast` / Success-Banner `bg-green-100 border-green-400 text-green-700`).
**Apply to:** Compose-Page (Send-Fehler → ErrorAlert; Erfolg → Toast + `nav.push(Route::ApplicationsPage {})`).

### No-Email-Guard
**Quelle:** `is_email_empty(email: Option<&str>) -> bool` (`member_details.rs:41`, inkl. Unit-Tests).
**Apply to:** Trigger-Button in `application_detail.rs`. Ggf. in geteilte Utility heben (Component-First).

### TO-Mapping (rendered_*)
**Quelle:** `rest.rs::MailRecipientTO` From (Z.343-359).
**Apply to:** beide `CommunicationEntryTO`-From-Impls (D-06 Stufe 4/5).

### form-onsubmit-Reload-Vermeidung
**Quelle:** Memory-Lesson, Vorbild `repayment_phases.rs` — `div`/`button`+`onclick`+`r#type:"button"`, kein `form onsubmit`.
**Apply to:** Send-Button der Compose-Page (D-05).

### i18n (beide Locales)
**Quelle:** `i18n/mod.rs` Key-Enum + `de.rs`/`en.rs`.
**Apply to:** neue Keys `LastSentSummary`, `NeverSent`, `SentMailBody` in **beiden** `De`+`En` (nur diese existieren).
Bestehende Keys wiederverwenden: `MailSendButton`, `NoEmailAddressHint`, `MailSend`/`MailSending`,
`CommunicationNone`, `MailJobCreated`.

## No Analog Found

Keine Datei ohne Analog. Zwei Punkte mit Neubau-Anteil (aber klarer Vorlage):

| Element | Rolle | Grund |
|---------|-------|-------|
| `on_entry_click`-Prop / Body-Detail-Panel | component/event-driven | Klick-Handling heute nicht prop-basiert (Landmine 3) — additiver Prop oder dünner Wrapper, Vorbild `MailJobDetail`-Body-Anzeige |
| Antragsteller-Filter im `TemplateSelector` | filter logic | Backend-List-Endpoint hat keinen Type-Query-Param → client-seitige Filterfunktion (testbar ohne WASM) |

## Metadata

**Analog search scope:** `genossi-frontend/src/{page,component,api.rs,router.rs}`, `genossi-frontend/rest-types/src/lib.rs`,
`genossi_mail/src/{dao.rs,dao_sqlite.rs,communication_rest.rs,rest.rs}`, `genossi_rest/src/application.rs`
**Files scanned:** 11 (alle Fundstellen dieser Session verifiziert)
**Pattern extraction date:** 2026-08-21
