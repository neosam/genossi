---
quick_id: 260603-jtf
type: execute
wave: 1
depends_on: []
files_modified:
  - genossi-frontend/src/component/mail_compose/mod.rs
  - genossi-frontend/src/component/mail_compose/template_tester.rs
  - genossi-frontend/src/page/mail_templates.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
autonomous: true

must_haves:
  truths:
    - "User kann auf der Mail-Template-Editor-Seite ein Mitglied auswählen, dessen reale Daten zum Rendern des aktuell editierten Templates verwendet werden"
    - "User sieht eine Live-Preview von Subject + Body, gerendert mit den Member-Variablen des ausgewählten Mitglieds"
    - "User kann eine Test-Empfänger-Adresse eingeben (NICHT die Member-Adresse) und das gerenderte Template als echte SMTP-Test-Mail dorthin senden"
    - "Beim Test-Versand wird das ausgewählte Mitglied NIE als Empfänger verwendet, auch nicht versehentlich — Member liefert nur die Template-Variablen, Empfänger ist immer das separate Test-Adress-Feld"
    - "Die Tester-UI ist als wiederverwendbare Komponente unter `genossi-frontend/src/component/mail_compose/` abgelegt (Component-First); die Seite enthält keine inline duplizierte Preview-/Selector-RSX"
    - "Cargo test grün für die neue Backend-Render-Logik und für den Frontend-Pure-Helper (UUID-Parsing/Adress-Validation)"
  artifacts:
    - path: "genossi-frontend/src/component/mail_compose/template_tester.rs"
      provides: "Wiederverwendbare TemplateTester-Komponente: Member-Selector + TemplatePreview-Reuse + Test-Adress-Input + 'Test-Mail senden'-Button"
      min_lines: 80
    - path: "genossi-frontend/src/page/mail_templates.rs"
      provides: "Editor-Seite mit eingebettetem <TemplateTester subject body /> unterhalb des Body-Textareas"
      contains: "TemplateTester"
  key_links:
    - from: "genossi-frontend/src/component/mail_compose/template_tester.rs"
      to: "genossi-frontend/src/component/mail_compose/template_preview.rs"
      via: "Direktes Re-use von TemplatePreview-Component mit member_ids=vec![selected_id]"
      pattern: "TemplatePreview \\{"
    - from: "genossi-frontend/src/component/mail_compose/template_tester.rs"
      to: "/api/mail/test-with-template"
      via: "api::send_test_mail_with_template aufruf"
      pattern: "send_test_mail_with_template"
    - from: "genossi_mail/src/rest.rs"
      to: "genossi_mail/src/template.rs::render_template + member_to_template_context"
      via: "Neuer Handler send_test_mail_with_template rendert Subject+Body mit Member-Context und ruft dann mail_service.send_test_mail_with_body() auf"
      pattern: "render_template"
---

<objective>
Auf der Mail-Template-Editor-Seite (`genossi-frontend/src/page/mail_templates.rs`) wird eine "Template testen"-Funktion ergänzt. Der User wählt ein Mitglied, sieht eine Live-Preview des aktuell editierten Templates (Subject + Body) gerendert mit den realen Member-Variablen, und kann optional eine echte SMTP-Test-Mail an eine **separate Test-Adresse** (nicht an den Member!) senden — der Member liefert nur die Template-Variablen.

Purpose: Der Vorstand muss vor dem produktiven Versand prüfen können, wie ein Template mit echten Daten aussieht (Anrede, Titel, Beträge, Vars-Fallbacks) — heute wird das auf der separaten Mail-Compose-Seite gemacht (umständlich, weil man dort den Bulk-Send-Flow durchläuft und Risiko besteht versehentlich an echte Member zu senden).

Output:
- Neue wiederverwendbare Komponente `TemplateTester` in `mail_compose/`
- Editor-Seite zeigt den Tester unterhalb des Body-Feldes (Component-First)
- Neuer Backend-Endpoint `POST /api/mail/test-with-template` für gerendertes Test-Versenden (das existierende `/api/mail/test` schickt eine fixe Konstanten-Mail ohne Template — nicht ausreichend für "Template testen")
- Bestehender `POST /api/mail/preview` wird für die Live-Preview wiederverwendet (kein neuer Endpoint nötig)
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@./genossi-frontend/CLAUDE.md
@./.planning/STATE.md
@./genossi-frontend/src/page/mail_templates.rs
@./genossi-frontend/src/page/mail_page.rs
@./genossi-frontend/src/component/mail_compose/mod.rs
@./genossi-frontend/src/component/mail_compose/template_preview.rs
@./genossi-frontend/src/component/mail_compose/template_var_buttons.rs
@./genossi-frontend/src/component/member_search.rs
@./genossi_mail/src/rest.rs
@./genossi_mail/src/service.rs
@./genossi_mail/src/template.rs

<interfaces>
<!-- Schlüssel-Interfaces aus dem bestehenden Code, die der Executor direkt verwendet — kein Codebase-Scavenger-Hunt nötig. -->

From `genossi-frontend/src/component/mail_compose/template_preview.rs`:
```rust
#[component]
pub fn TemplatePreview(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    member_ids: Vec<Uuid>,
    #[props(default)] repayment_phase_id: Option<Uuid>,
) -> Element
```
→ Wenn `member_ids.len() == 1` zeigt es einen Member im Dropdown; intern wird `/api/mail/preview` gerufen. Wir geben `vec![selected_id]` rein und der Tester zeigt automatisch nur diesen Member im Dropdown.

From `genossi-frontend/src/component/member_search.rs`:
```rust
#[component]
pub fn MemberSearch(
    on_select: EventHandler<Option<Uuid>>,
    selected_id: Option<Uuid>,
    exclude_id: Option<Uuid>,
) -> Element
```
→ Selektiert genau EIN Mitglied per Suche. Genau das, was wir für den Tester brauchen.

From `genossi-frontend/src/api.rs` (bereits vorhanden, nicht ändern):
```rust
pub async fn preview_mail(
    config: &Config,
    subject: &str,
    body: &str,
    member_id: &str,
    repayment_phase_id: Option<Uuid>,
) -> Result<PreviewResponse, AppError>;

pub async fn send_test_mail(config: &Config, to_address: &str) -> Result<(), AppError>;
// ↑ NICHT geeignet: schickt fixe Konstanten-Mail "Genossi Test-E-Mail" ohne Template-Rendering.
// Wir brauchen einen neuen Aufruf send_test_mail_with_template(...).
```

From `genossi_mail/src/rest.rs`:
```rust
pub struct TestMailRequest { pub to_address: String }
pub struct PreviewRequest { pub subject: String, pub body: String, pub member_id: String, pub repayment_phase_id: Option<String> }
pub struct PreviewResponse { pub subject: String, pub body: String, pub errors: Vec<String> }

pub fn generate_route<S: MailRestState>() -> Router<S> {
    Router::new()
        .route("/send", post(send_mail::<S>))
        .route("/send-bulk", post(send_bulk_mail::<S>))
        .route("/preview", post(preview_mail::<S>))
        .route("/test", post(send_test_mail::<S>))
        .route("/jobs", get(get_jobs::<S>))
        .route("/jobs/{id}", get(get_job_detail::<S>))
        .route("/jobs/{id}/retry", post(retry_job::<S>))
}
```

From `genossi_mail/src/template.rs`:
```rust
pub fn member_to_template_context(entity: &MemberEntity) -> Value;
pub fn render_template(template_str: &str, context: &Value) -> Result<String, TemplateError>;
```

From `genossi_mail/src/service.rs`:
```rust
#[async_trait]
pub trait MailService: Send + Sync + 'static {
    // …
    async fn send_test_mail(&self, to: &str) -> Result<(), MailServiceError>;
    // ↑ verwendet einen fixen Constant-String — wir brauchen einen Sibling
    //   send_test_mail_with_body(&self, to: &str, subject: &str, body: &str)
    //   oder rendern im REST-Handler und rufen einen neuen MailService-Helper.
}
```

`MailRestState` (siehe `genossi_mail/src/rest.rs:40-60` — `resolve_member(member_id) -> Option<MemberEntity>` + `mail_service() -> &dyn MailService`) ist bereits ausreichend; KEIN State-Trait-Refactor nötig.

From `genossi_mail/src/service.rs:396-426` (Referenz-Implementierung für SMTP-Versand — kopieren mit gerendertem Subject/Body):
```rust
let smtp_config = load_smtp_config(self.config_service.as_ref()).await?;
let transport = build_transport(&smtp_config)?;
let email = Message::builder()
    .from(smtp_config.from.parse()?)
    .to(to.parse()?)
    .subject(<rendered_subject>)
    .body(<rendered_body>)?;
transport.send(email).await?;
```
</interfaces>

**Existing reusable infrastructure (DO use, DO NOT reimplement):**
- `TemplatePreview` component (`mail_compose/template_preview.rs`) — bereits Member-Selector + Render-Trigger; reuse 1:1 mit `member_ids=vec![selected_id]`
- `MemberSearch` component (`component/member_search.rs`) — bereits Single-Member-Selector mit Suche
- `api::preview_mail(...)` — Endpoint `POST /api/mail/preview` existiert und unterstützt member-id + optional phase
- `MEMBERS` global signal + `refresh_members()` (siehe `mail_page.rs:111-114`)
- `member_to_template_context()` + `render_template()` in `genossi_mail/src/template.rs`
- `load_smtp_config` + `build_transport` + lettre-Pattern aus `service.rs:396-426`

**Wichtige Konventionen (Verstöße = Re-Work):**
- **Component-First** (CLAUDE.md + Memory `feedback_component_first.md`): Editor-Seite darf KEINE inline-RSX für Selector / Preview / Test-Versand enthalten — alles muss in `TemplateTester` gekapselt sein. Diese Komponente ist später potenziell auch aus der Compose-Seite reusbar.
- **jj statt git**: Commits per `jj commit -m "..."`. NICHT `git commit`.
- **Audit**: Keine neuen Audit-Pflichten. Member-Lesen wird grundsätzlich nicht auditiert; Test-Mail-Versand erzeugt KEIN MailJob (das war eine bewusste Design-Entscheidung am bestehenden `/api/mail/test` — wir folgen demselben Pattern).
- **Tests sind Pflicht** (global CLAUDE.md): Backend-Unit-Test für den neuen Render-Pfad (analog `test_send_test_mail_missing_config`/`test_send_test_mail_smtp_failure`); Frontend-Pure-Helper-Tests für Adress-Validation (analog `parse_mail_query` Tests in `mail_page.rs:1044+`).
- **Privacy-Constraint (User-Intent explizit):** Test-Empfänger ist NIE der ausgewählte Member. Das Adress-Feld muss explizit befüllt werden (Default: leerer String, NICHT vom Member abgeleitet). Backend-Handler nimmt `to_address` und `member_id` als getrennte Felder im Request-Body — der Member wird ausschließlich für `member_to_template_context()` verwendet.
- **i18n**: Beide Locales (`de.rs`, `en.rs`) müssen für jeden neuen Key gepflegt werden — siehe `genossi-frontend/CLAUDE.md`.
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Backend — POST /api/mail/test-with-template + Unit-Test</name>
  <files>
    genossi_mail/src/rest.rs,
    genossi_mail/src/service.rs
  </files>
  <behavior>
    - **Test 1 (genossi_mail/src/service.rs::tests)** `test_send_test_mail_with_body_missing_config`: ohne SMTP-Config → `Err(MailServiceError::ConfigMissing(_))` (1:1 Pattern wie bestehender `test_send_test_mail_missing_config` Z. 724).
    - **Test 2 (genossi_mail/src/service.rs::tests)** `test_send_test_mail_with_body_smtp_failure`: mit dummy-SMTP-Config (host=127.0.0.1, port=19999) → `Err(MailServiceError::SmtpError(_))` und der Subject/Body-Parameter wurde durch die Funktion verwendet (kein Constant-String — Verifikation z.B. dass `subject="X-CUSTOM-SUBJECT"` keinen Compile-Error wirft und der Funktionsbody den Parameter konsumiert).
    - **Test 3 (genossi_mail/src/rest.rs::tests)** `test_test_with_template_request_serde_roundtrip`: pure-serde Test für die neue Request-Struct (`to_address`, `subject`, `body`, `member_id`, optional `repayment_phase_id`) — Backward-Compat für fehlende phase_id.
    - Kein Mocking von `resolve_member` nötig im Service-Unit-Test, da das im REST-Handler passiert (analog bestehender Trennung).
  </behavior>
  <action>
    **Backend: Neuer Endpoint `POST /api/mail/test-with-template`.**

    1. **Service-Trait-Erweiterung** in `genossi_mail/src/service.rs`:
       Füge im `trait MailService` (ab Z. 52) eine neue Methode hinzu:
       ```rust
       async fn send_test_mail_with_body(
           &self,
           to: &str,
           subject: &str,
           body: &str,
       ) -> Result<(), MailServiceError>;
       ```
       Implementierung im `impl MailService for MailServiceImpl` (ab Z. 396): kopiere `send_test_mail`-Body (Z. 396-426), aber ersetze die hartkodierten `.subject("Genossi Test-E-Mail")` und `.body("Diese E-Mail bestätigt…")` durch die übergebenen Parameter (verwende `subject.to_string()` bzw. `body.to_string()` für lettre's `Message::builder()`). **NICHT** die bestehende `send_test_mail` ändern oder löschen — es bleibt rückwärtskompatibel für die SMTP-Config-Test-Funktion auf der Settings-Seite.

       MockMailService auto-generiert sich via `#[automock]` falls vorhanden — sonst muss der MockMailService-Block in dieser Datei manuell ergänzt werden (grep `MockMailService` zur Verifikation; im aktuellen Code wird mockall vermutlich nicht für MailService verwendet, also kein Mock-Update nötig — falls doch, Methode dort manuell hinzufügen).

    2. **REST-Request-Struct** in `genossi_mail/src/rest.rs` (nach `TestMailRequest` Z. 181):
       ```rust
       #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
       pub struct TestMailWithTemplateRequest {
           #[schema(example = "vorstand@example.com")]
           pub to_address: String,
           #[schema(example = "Hallo {{ first_name }}")]
           pub subject: String,
           #[schema(example = "Liebe/r {{ first_name }} {{ last_name }}…")]
           pub body: String,
           #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
           pub member_id: String,
           #[serde(default, skip_serializing_if = "Option::is_none")]
           pub repayment_phase_id: Option<String>,
       }
       ```

    3. **REST-Handler** in `genossi_mail/src/rest.rs` (zwischen `preview_mail` und `send_test_mail` einfügen, ca. Z. 615):
       ```rust
       #[instrument(skip(state))]
       #[utoipa::path(
           post,
           tag = "Mail",
           path = "/test-with-template",
           request_body = TestMailWithTemplateRequest,
           responses(
               (status = 200, description = "Test mail with rendered template sent"),
               (status = 400, description = "Invalid request or template error"),
               (status = 404, description = "Member not found"),
               (status = 502, description = "SMTP error"),
               (status = 500, description = "Internal server error"),
           ),
       )]
       pub async fn send_test_mail_with_template<S: MailRestState>(
           state: State<S>,
           axum::Json(body): axum::Json<TestMailWithTemplateRequest>,
       ) -> Response { … }
       ```
       Handler-Body: analog zu `preview_mail` (Z. 540-615) für Member-Lookup + Context-Build (inkl. optional repayment-merge — kopiere die Match-Logik 1:1), dann `render_template` für subject und body. **WICHTIG (Privacy-Defense):** der Handler ruft `state.mail_service().send_test_mail_with_body(&body.to_address, &rendered_subject, &rendered_body).await?` mit der **`body.to_address`** aus dem Request, NIEMALS mit der Member-Email-Adresse. Template-Errors aus `render_template` werden zu `MailServiceError::TemplateValidation(...)` → 400.

    4. **Route registrieren** in `generate_route` (Z. 300-309):
       ```rust
       .route("/test-with-template", post(send_test_mail_with_template::<S>))
       ```

    5. **OpenAPI-Doc** in `ApiDoc` (Z. 312-316): füge `send_test_mail_with_template` zu `paths(...)` und `TestMailWithTemplateRequest` zu `components(schemas(...))` hinzu.

    6. **Tests** wie in `<behavior>` spezifiziert. Tests sind Pflicht (global CLAUDE.md).

    **Why kein Reuse des bestehenden `/api/mail/test`:** Der bestehende Endpoint sendet eine fixe Constant-Mail "Genossi Test-E-Mail" als SMTP-Config-Smoke-Test — verändert man dessen Signatur, breakt das die Settings-Seite (`config_page.rs:445`). Sauber: neuer Endpoint mit klarer Semantik "Template-Test".

    **Why kein Reuse von `send-bulk`/`send`:** Diese erzeugen MailJob-Persistenz, AuditLog-Einträge, MemberDocument-Status-Updates etc. Ein Template-Test soll fire-and-forget sein (kein Job, keine History) und NIE versehentlich einen Member als Empfänger haben können (Datenschutz).
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3 && cargo test -p genossi_mail send_test_mail_with_body 2>&1 | tail -30 && cargo test -p genossi_mail test_test_with_template_request_serde_roundtrip 2>&1 | tail -10 && cargo check -p genossi_mail 2>&1 | tail -5</automated>
  </verify>
  <done>
    - 3 neue Tests grün (2 Service-Unit + 1 REST-Serde-Roundtrip)
    - `cargo check -p genossi_mail` passt ohne Warnings für die neuen Items
    - Bestehende `send_test_mail` weiterhin vorhanden und unverändert
    - Neuer Endpoint registriert in `generate_route` UND in `ApiDoc::paths`/`components`
    - Grep-Gate: `grep -c "send_test_mail_with_template\|send_test_mail_with_body" genossi_mail/src/rest.rs genossi_mail/src/service.rs` ≥ 6 (Definition + Test-Calls + Route)
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Frontend — TemplateTester-Komponente + Einbettung in mail_templates.rs + Pure-Helper-Test</name>
  <files>
    genossi-frontend/src/component/mail_compose/template_tester.rs,
    genossi-frontend/src/component/mail_compose/mod.rs,
    genossi-frontend/src/page/mail_templates.rs,
    genossi-frontend/src/api.rs,
    genossi-frontend/src/i18n/mod.rs,
    genossi-frontend/src/i18n/de.rs,
    genossi-frontend/src/i18n/en.rs
  </files>
  <behavior>
    - **Test 1** (in `template_tester.rs::tests`) `test_is_valid_test_address_accepts_normal`: `is_valid_test_address("vorstand@example.com") == true`
    - **Test 2** `test_is_valid_test_address_rejects_empty_and_missing_at`: `""`, `"no-at-sign"`, `"   "` alle `false`
    - **Test 3** `test_is_valid_test_address_trims`: `"  foo@bar.de  "` → `true` (trim wird angewendet)
    - Diese Helper sind pure Rust (kein dioxus/web_sys), laufen also via `cargo test -p genossi-frontend`.
  </behavior>
  <action>
    **Frontend: TemplateTester-Komponente als Component-First-Refactor.**

    1. **Neue API-Client-Funktion** in `genossi-frontend/src/api.rs` (direkt nach `send_test_mail` Z. 988-995 einfügen):
       ```rust
       pub async fn send_test_mail_with_template(
           config: &Config,
           to_address: &str,
           subject: &str,
           body: &str,
           member_id: &str,
       ) -> Result<(), AppError> {
           info!("Sending test mail with rendered template to: {to_address}");
           let url = format!("{}/api/mail/test-with-template", config.backend);
           let req = serde_json::json!({
               "to_address": to_address,
               "subject": subject,
               "body": body,
               "member_id": member_id,
           });
           let response = reqwest::Client::new().post(url).json(&req).send().await?;
           check_response(response).await?;
           Ok(())
       }
       ```
       (repayment_phase_id wird im Quick-Scope nicht vom Editor-Tester durchgereicht — Editor-Templates haben keinen Phase-Kontext; das ist Phase-12-spezifisch im Bulk-Compose-Flow. Falls in Zukunft nötig, additiv erweiterbar.)

    2. **Neue Komponente** `genossi-frontend/src/component/mail_compose/template_tester.rs`:

       Struktur (Props-Interface):
       ```rust
       #[component]
       pub fn TemplateTester(
           subject: ReadOnlySignal<String>,
           body: ReadOnlySignal<String>,
       ) -> Element { … }
       ```

       Internes State:
       - `selected_member_id: Signal<Option<Uuid>>` (default None)
       - `test_address: Signal<String>` (default `""`)
       - `sending: Signal<bool>` (default false)
       - `feedback: Signal<Option<(bool /* is_error */, String /* message */)>>` (default None)

       Mount-Hook: `use_effect(|| spawn(refresh_members().await))` (1:1 wie `mail_page.rs:110-114`) — damit `MEMBERS` Signal befüllt ist.

       Pure-Helper:
       ```rust
       pub(crate) fn is_valid_test_address(addr: &str) -> bool {
           let trimmed = addr.trim();
           !trimmed.is_empty() && trimmed.contains('@')
       }
       ```
       (Bewusst minimal — keine RFC5321-Vollparser-Validation; lettre rejected ungültige Adressen serverseitig mit 502.)

       RSX-Layout (innerhalb `bg-gray-50 rounded-lg p-4 mt-4`-Container, analog `TemplatePreview`-Stil):
       - h3-Überschrift (i18n-Key `MailTemplateTest`)
       - `MemberSearch { on_select: move |id| selected_member_id.set(id), selected_id: *selected_member_id.read(), exclude_id: None }`
       - **WENN ein Member gewählt:** rendere `TemplatePreview { subject, body, member_ids: vec![selected_id] }` (Live-Preview greift sofort und re-rendert bei subject/body-Changes via Signal-Subscription)
       - Test-Adress-Block (immer sichtbar, aber Button disabled bei missing prerequisites):
         ```rsx
         div { class: "mt-3 border-t pt-3",
             label { class: "block text-xs font-medium text-gray-500 mb-1", {i18n.t(Key::MailTemplateTestSendTo)} }
             p { class: "text-xs text-amber-600 mb-2", {i18n.t(Key::MailTemplateTestPrivacyHint)} }  // ← Datenschutz-Hinweis: "Geht NICHT an das ausgewählte Mitglied"
             input { r#type: "email", class: "...", value: "{test_address}", oninput: ..., placeholder: "test@example.com" }
             button {
                 r#type: "button",  // ← Memory `feedback_dioxus_button_type.md`: ohne r#type="button" → Page-Reload-Bug
                 class: "...",
                 disabled: *sending.read() || !is_valid_test_address(&test_address.read()) || selected_member_id.read().is_none(),
                 onclick: move |_| { /* spawn send_test_mail_with_template, set feedback */ },
                 {i18n.t(if *sending.read() { Key::MailSending } else { Key::MailTemplateTestSend })}
             }
         }
         ```
       - Feedback-Block: bei `Some((false, msg))` grüner Success-Toast, bei `Some((true, msg))` roter Error-Toast.

       Tests-Modul (`#[cfg(test)] mod tests`) mit den 3 Pure-Helper-Tests aus `<behavior>`.

    3. **mod.rs aktualisieren** (`genossi-frontend/src/component/mail_compose/mod.rs`):
       ```rust
       pub mod template_tester;
       pub use template_tester::TemplateTester;
       ```

    4. **mail_templates.rs erweitern** (`genossi-frontend/src/page/mail_templates.rs`):
       - Import oben: `use crate::component::mail_compose::{TemplateTester, TemplateVarButtons};`
       - **Im Editor-Block** (innerhalb `if is_editing { … }`, nach dem Body-Textarea (Z. 244-254) und vor den Action-Buttons (Z. 257)) einfügen:
         ```rsx
         TemplateTester {
             subject: edit_subject.into(),  // Signal<String> → ReadOnlySignal<String>
             body: edit_body.into(),
         }
         ```
         (Die `.into()`-Conversion ist Dioxus-Standard für Signal → ReadOnlySignal — verifiziere am bestehenden `TemplatePreview` Call in `mail_page.rs:447-453`, der mit `subject: subject` direkt funktioniert weil das schon ein Signal ist; ggf. einfach `subject: edit_subject, body: edit_body` ausreichend.)
       - **KEIN inline-RSX für Member-Selector/Preview/Test-Button hinzufügen** — das ist die Component-First-Anforderung. Verifikation: `grep -c 'MemberSearch\|TemplatePreview\|test_address' genossi-frontend/src/page/mail_templates.rs` muss exakt 0 sein (nur die `TemplateTester`-Reference, die diese intern verwendet, zählt).

    5. **i18n-Keys** in `genossi-frontend/src/i18n/mod.rs` (innerhalb des `Key`-Enums, in der "Mail templates"-Sektion ab Z. 306):
       ```rust
       MailTemplateTest,             // h3-Überschrift "Template testen" / "Test template"
       MailTemplateTestSendTo,       // Label "Test-Empfänger" / "Test recipient"
       MailTemplateTestSend,         // Button "Test-Mail senden" / "Send test mail"
       MailTemplateTestPrivacyHint,  // Hinweis "Wird an die Test-Adresse gesendet, NICHT an das ausgewählte Mitglied." / "Will be sent to the test address, NOT to the selected member."
       MailTemplateTestSuccess,      // "Test-Mail gesendet." / "Test mail sent."
       MailTemplateTestFailed,       // "Test-Mail fehlgeschlagen: {error}" — verwendet als Format-String im Code
       ```
       In `genossi-frontend/src/i18n/de.rs` und `genossi-frontend/src/i18n/en.rs` jeweils die match-Arme ergänzen (beide Locales, sonst zur Laufzeit `Key not found` — siehe `genossi-frontend/CLAUDE.md` Sektion i18n). Übersetzungen müssen sinnvoll sein, nicht der enum-Variantenname.

    6. **Privacy-Defense Doc-Comment** über dem `onclick` des Send-Buttons in `template_tester.rs`:
       ```rust
       // PRIVACY: to_address kommt AUSSCHLIESSLICH aus dem test_address-Signal,
       // NIE aus member.email. Member liefert nur die Template-Variablen via
       // member_id im Request — Backend (genossi_mail/src/rest.rs::send_test_mail_with_template)
       // rendert mit member-Context und sendet an body.to_address.
       ```

    **Why Component-First für TemplateTester (nicht inline in mail_templates.rs):**
    - Memory `feedback_component_first.md` ist explizit ein Re-Work-Trigger
    - Künftiges Reuse-Szenario: Compose-Seite könnte denselben Tester unterhalb des Templates anzeigen (statt der dort bereits eingebetteten `TemplatePreview` + separater Bulk-Send-Flow)
    - Tests sind so isolierbar (Pure-Helper testbar ohne ganzes Page-Mounting)
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3/genossi-frontend && cargo test --lib is_valid_test_address 2>&1 | tail -15 && cargo check 2>&1 | tail -20 && cd .. && bash -c 'COUNT=$(grep -E "MemberSearch|TemplatePreview|test_address" genossi-frontend/src/page/mail_templates.rs | grep -v "^//" | wc -l); echo "Inline-Selector-Leaks in mail_templates.rs (must be 0): $COUNT"; [ "$COUNT" -eq 0 ] && echo "PASS: Component-First eingehalten" || (echo "FAIL: Inline-RSX statt TemplateTester verwenden"; exit 1)'</automated>
  </verify>
  <done>
    - 3 Pure-Helper-Tests grün
    - `cargo check` für `genossi-frontend` ohne Compile-Errors
    - `mod.rs` exportiert `TemplateTester`
    - `mail_templates.rs` hat `TemplateTester { subject: ..., body: ... }` UND keine inline `MemberSearch`/`TemplatePreview`/`test_address`-Referenzen (Component-First-Gate grün)
    - Alle 6 neuen i18n-Keys in `mod.rs`, `de.rs`, `en.rs` deklariert und übersetzt
    - Privacy-Doc-Comment im onclick-Handler vorhanden
  </done>
</task>

<task type="auto">
  <name>Task 3: Workspace-Integration prüfen + jj-Commit</name>
  <files>
    (keine Datei-Änderungen außer evtl. minimaler Fixups)
  </files>
  <action>
    1. **Workspace-Build prüfen:**
       ```bash
       cargo build --workspace 2>&1 | tail -20
       cargo test --workspace 2>&1 | tail -30
       cargo clippy --workspace --all-targets 2>&1 | tail -20
       cargo fmt
       ```
       Falls clippy oder fmt etwas anmeckert: fixen (kein `--allow`).

    2. **Manueller Sanity-Check (Doc-Hinweis im Commit, keine Pflicht-Test-Aktion):** verifiziere visuell durch Lesen des finalen Codes:
       - `mail_templates.rs` enthält genau eine `TemplateTester`-Invocation im Editor-Panel
       - `TemplateTester` rendert intern `MemberSearch` UND `TemplatePreview` (Component-Reuse, kein Re-implement)
       - Backend-Handler reicht `body.to_address` (NICHT `member.email`) an `send_test_mail_with_body`

    3. **jj-Commit** (Projekt ist jj-Repo — Memory `feedback_use_jj_not_git.md`):
       ```bash
       jj status
       jj commit -m "feat(quick-260603-jtf): Template-Test-Funktion auf Editor-Seite

       - Neuer Backend-Endpoint POST /api/mail/test-with-template:
         rendert Subject+Body mit Member-Context und sendet an separate
         Test-Adresse (NICHT an den Member) via SMTP.
       - Neue MailService-Methode send_test_mail_with_body (sibling zu
         send_test_mail; bestehende Constant-Mail-Funktion unverändert).
       - Neue Frontend-Komponente TemplateTester (mail_compose/template_tester.rs):
         Member-Selector + TemplatePreview-Reuse + Test-Adress-Input +
         Send-Button. Component-First — keine inline-RSX in der Editor-Seite.
       - mail_templates.rs bettet <TemplateTester> unterhalb des Body-Textareas ein.
       - Privacy-Defense: Test-Empfänger ist immer das separate Adress-Feld,
         nie die Member-Email — sowohl im UI-Layout (separates Input + Hinweis-Text)
         als auch im Backend-Handler (to_address kommt aus Request-Body, nicht
         aus dem aufgelösten MemberEntity).
       - Tests: 3 Backend (2 Service-Unit + 1 REST-Serde-Roundtrip) + 3
         Frontend-Pure-Helper-Tests (is_valid_test_address)."
       ```

    4. **STATE.md** wird vom Quick-Workflow-Wrapper aktualisiert (nicht in dieser Task).
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3 && cargo build --workspace 2>&1 | tail -10 && cargo test --workspace --quiet 2>&1 | tail -15 && cargo clippy --workspace --all-targets 2>&1 | grep -E "warning|error" | head -20; echo "---"; jj log -r '@-' --no-graph -T 'commit_id ++ " " ++ description.first_line()' 2>&1 | head -3</automated>
  </verify>
  <done>
    - `cargo build --workspace` grün
    - `cargo test --workspace` grün (insbesondere die 6 neuen Tests aus Task 1+2)
    - `cargo clippy --workspace --all-targets` ohne neue Warnings für berührte Dateien
    - `jj log -r '@-'` zeigt den neuen Commit mit dem oben spezifizierten Message-Format
  </done>
</task>

</tasks>

<verification>
**End-to-End Smoke (manuell, Optional — der Build/Test-Pipeline-Check oben reicht für Acceptance):**

1. `cargo run --bin genossi` → Browser auf `http://localhost:8080/mail-templates`
2. Template anlegen oder öffnen
3. Im Editor unterhalb des Body-Felds: neuer "Template testen"-Block sichtbar
4. Mitglied wählen → Live-Preview rendert sofort mit Member-Variablen
5. Subject/Body editieren → Preview-Button neu klicken → Re-render mit aktuellen Werten
6. Test-Adresse eingeben (eigene Adresse, NICHT die des Members) → "Test-Mail senden" klicken
7. Bei korrekter SMTP-Config: Test-Mail kommt an der Test-Adresse an mit gerendertem Subject/Body
8. Bei fehlender SMTP-Config: rote Fehler-Box "SMTP-Config fehlt"
9. **Privacy-Sanity:** sicherstellen dass die Test-Mail NICHT an die Email-Adresse des ausgewählten Members ging
</verification>

<success_criteria>
- 6 neue Tests grün (3 Backend + 3 Frontend-Helper)
- `cargo build --workspace` + `cargo clippy --workspace` ohne neue Warnings
- `cargo fmt` clean
- TemplateTester-Komponente exportiert, in mail_templates.rs eingebettet
- Component-First-Gate grün: keine `MemberSearch`/`TemplatePreview`/`test_address`-Inline-Refs in `mail_templates.rs`
- Neuer Endpoint `POST /api/mail/test-with-template` in `generate_route` + `ApiDoc`
- jj-Commit existiert (kein git commit)
- Privacy-Defense doppelt verankert: Doc-Comment im Frontend onclick + getrenntes Request-Feld `to_address` im Backend
</success_criteria>

<output>
After completion, create `.planning/quick/260603-jtf-mail-templates-test-funktion-auf-der-tem/260603-jtf-SUMMARY.md` mit:
- Was gebaut wurde (Backend-Endpoint, Frontend-Komponente, i18n-Keys)
- Test-Coverage-Stand (6 neue Tests + Workspace-Test-Zahlen vorher/nachher)
- Privacy-Defense-Verifikation (welche Stellen schützen vor versehentlichem Versand an Member)
- Etwaige Tech-Debt-Items (z.B. falls TemplateTester später auch in Compose-Seite eingesetzt werden soll — Anker für ein Follow-up-Quick)
- jj-Commit-ID
</output>
