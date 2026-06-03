---
quick_id: 260603-kon
slug: beim-testen-von-templates-fehlen-dummy-d
type: execute
wave: 1
depends_on: []
files_modified:
  - genossi_mail/src/template.rs
  - genossi_mail/src/rest.rs
  - genossi_service_impl/src/pdf_generation.rs
  - genossi_rest/src/template.rs
autonomous: true
requirements: [QUICK-260603-kon]

must_haves:
  truths:
    - "Mail-Template-Test rendert ohne Fehler, wenn Member KEINE aktive Open/Contacted Repayment-Phase hat, auch bei Templates mit {{ payout_amount }}/{{ share_count }}/{{ share_value }}/{{ fiscal_year }}"
    - "Typst-Template-Test (PDF-Preview) rendert ein Auszahlungs-Anschreiben, wenn Member nicht in aktiver Repayment-Phase ist"
    - "Beim Template-Test sind Dummy-Repayment-Werte als solche erkennbar (Sentinel-Werte: payout_amount=\"99,99\", share_count=99, share_value=\"99,99\", fiscal_year=2099)"
    - "Worker-Pfad (echte Mails an echte Mitglieder via /api/mail/send-bulk) ist UNVERAENDERT — keine Dummy-Daten gelangen in produktive Mails oder ins Audit-Log"
    - "Frontend zeigt im Mail-Tester eine sichtbare Hinweis-Zeile, wenn Dummy-Daten gerendert wurden"
  artifacts:
    - path: "genossi_mail/src/template.rs"
      provides: "Pure-fn dummy_repayment_context() -> (String, i32, String, i32) liefert Sentinel-Werte fuer Test-Pfade"
      contains: "pub fn dummy_repayment_context"
    - path: "genossi_mail/src/rest.rs"
      provides: "preview_mail + send_test_mail_with_template fallback auf dummy_repayment_context, wenn repayment_phase_id gesetzt aber resolve_repayment_context None liefert; PreviewResponse signalisiert via Feld used_dummy_repayment"
      contains: "dummy_repayment_context"
    - path: "genossi_service_impl/src/pdf_generation.rs"
      provides: "Helper fn dummy_repayment_context_for_typst() liefert (RepaymentContext, RepaymentPhaseEntity), nutzbar wenn Typst-Repayment-Letter ohne aktive Phase getestet werden soll"
      contains: "dummy_repayment_context_for_typst"
    - path: "genossi_rest/src/template.rs"
      provides: "Neue Route /api/templates/render-repayment-test/*path/{member_id} rendert Repayment-Letter mit Dummy-Phase + Dummy-Context"
      contains: "render_repayment_letter_test"
  key_links:
    - from: "genossi_mail/src/rest.rs:preview_mail"
      to: "genossi_mail/src/template.rs:dummy_repayment_context"
      via: "fallback im match-Arm match state.resolve_repayment_context(...).await { Some(...) => merge_real, None => merge_dummy }"
      pattern: "dummy_repayment_context"
    - from: "genossi_mail/src/rest.rs:send_test_mail_with_template"
      to: "genossi_mail/src/template.rs:dummy_repayment_context"
      via: "identischer Fallback-Pfad wie preview_mail (D-05-Symmetrie-Variante: nur fuer Test-Endpoints, NICHT im Worker)"
      pattern: "dummy_repayment_context"
    - from: "genossi_rest/src/template.rs:render_repayment_letter_test"
      to: "genossi_service_impl/src/pdf_generation.rs:render_repayment_letter"
      via: "Aufruf mit Dummy-Phase + Dummy-Context aus dummy_repayment_context_for_typst"
      pattern: "render_repayment_letter\\("
    - from: "genossi-frontend/src/component/mail_compose/template_tester.rs"
      to: "PreviewResponse.used_dummy_repayment Flag"
      via: "Wenn true -> sichtbares amber Hinweis-Banner unter dem Preview-Block"
      pattern: "used_dummy_repayment"
---

<objective>
Beim Testen von Mail- und Typst-Templates Dummy-Repayment-Daten einsetzen, wenn das gewaehlte Mitglied keine aktive Open/Contacted Repayment-Phase hat. Vorstand kann dadurch Templates mit Rueckzahlungs-Platzhaltern auch in Ruhe-Phasen zwischen Generalversammlungen testen.

Purpose: Heute scheitern beide Test-Pfade (Mail Preview/Test, Typst PDF-Preview) bei strict-env Jinja bzw. fehlender Phase, sobald das Template Repayment-Variablen referenziert und der Member zufaellig keine offenen Entries hat. Das ist die mit Abstand haeufigste Situation ausserhalb aktiver Auszahlungsphasen.

Output:
- Pure-fn Helpers `dummy_repayment_context` (Mail) und `dummy_repayment_context_for_typst` (PDF)
- Aktualisierte Mail-Test-Endpoints mit Dummy-Fallback + `used_dummy_repayment` Flag in `PreviewResponse`
- Neue REST-Route `POST /api/templates/render-repayment-test/{*path}/{member_id}` fuer Typst-Repayment-Letter-Tests
- Frontend-Hinweis im `TemplateTester` ueber Dummy-Daten
- Mind. 4 neue Unit-Tests (2 Mail, 2 Typst/REST)
- KEINE Aenderung am Worker-Pfad (`worker.rs`, `send-bulk`) — Dummy-Daten bleiben strikt auf Test-Endpoints beschraenkt
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@genossi-frontend/CLAUDE.md
@.planning/STATE.md

<!-- Quelldateien: Backend-Mail-Pfad -->
@genossi_mail/src/rest.rs
@genossi_mail/src/template.rs
@genossi_mail/src/worker.rs

<!-- Quelldateien: Backend-Typst-Pfad -->
@genossi_service_impl/src/pdf_generation.rs
@genossi_rest/src/template.rs

<!-- Quelldateien: Frontend-Tester -->
@genossi-frontend/src/component/mail_compose/template_tester.rs
@genossi-frontend/src/component/mail_compose/template_preview.rs
@genossi-frontend/src/api.rs

<interfaces>
<!-- Key types and contracts the executor needs. Extracted from codebase. -->
<!-- Executor should use these directly — no codebase exploration needed. -->

From genossi_mail/src/template.rs:
```rust
pub fn merge_repayment_context(
    base: Value,
    payout_amount: &str,
    share_count: i32,
    share_value: &str,
    fiscal_year: i32,
) -> Value;

pub fn render_template(template_str: &str, context: &Value) -> Result<String, TemplateError>;

pub fn member_to_template_context(entity: &MemberEntity) -> Value;
```

From genossi_mail/src/rest.rs (handler signatures + state-trait method):
```rust
// MailRestState trait method already present:
fn resolve_repayment_context(
    &self,
    phase_id: uuid::Uuid,
    member_id: uuid::Uuid,
) -> Pin<Box<dyn Future<Output = Option<(String, i32, String, i32)>> + Send + '_>>;
// Tuple: (payout_amount, share_count, share_value, fiscal_year)

pub struct PreviewResponse {
    pub subject: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

pub struct TestMailWithTemplateRequest {
    pub to_address: String,
    pub subject: String,
    pub body: String,
    pub member_id: String,
    pub repayment_phase_id: Option<String>,
}
```

From genossi_service/src/repayment_context.rs:
```rust
pub struct RepaymentContext {
    pub share_count: i32,
    pub payout_amount: String,  // German euro format "X,YZ"
    pub fiscal_year: i32,
}
```

From genossi_dao/src/repayment_phase.rs (RepaymentPhaseEntity — verify field names with `grep -n "pub struct RepaymentPhaseEntity" genossi_dao/src/repayment_phase.rs` then `Read` around the struct; needed: `id`, `fiscal_year`, `share_value: i64` (Cent), plus `created`, `deleted`, `version` for entity invariants).

From genossi_service_impl/src/pdf_generation.rs:
```rust
pub fn render_repayment_letter(
    &self,
    template_path: &str,
    template_base: &Path,
    phase: &RepaymentPhaseEntity,
    member: &MemberEntity,
    ctx: &RepaymentContext,
) -> Result<Vec<u8>, ServiceError>;
```

From genossi_rest/src/template.rs:
```rust
pub fn generate_render_route<RestState: RestStateDef>() -> Router<RestState>;
// Pattern parse_render_path: parses "template/path.typ/{member_id}" -> ("template/path.typ", "{member_id}")
```

From genossi-frontend/src/component/mail_compose/template_tester.rs:
```rust
#[component]
pub fn TemplateTester(subject: ReadOnlySignal<String>, body: ReadOnlySignal<String>) -> Element;
// Component uses MemberSearch + TemplatePreview + test_address input
```

From genossi-frontend/src/component/mail_compose/template_preview.rs:
```rust
#[component]
pub fn TemplatePreview(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    member_id: ReadOnlySignal<Option<Uuid>>,
    #[props(default)] repayment_phase_id: Option<Uuid>,
) -> Element;
```
</interfaces>

<key_existing_behaviors>
- **Worker-Pfad** (`genossi_mail/src/worker.rs:467-498`): Wenn `resolve_repayment_context` `None`/`EntityNotFound` liefert, wird `merge_repayment_context` NICHT aufgerufen. Templates mit Repayment-Vars rendern dann in strict-env mit Fehler -> `mark_recipient_failed`. **Dies muss so bleiben** — der Worker ist KEIN Test-Pfad.
- **Mail Preview-Pfad** (`genossi_mail/src/rest.rs:594-614`): Heute identisches Verhalten wie Worker (kein Merge -> strict-env-Fehler im Response). Dieser Pfad SOLL geaendert werden.
- **Test-with-template-Pfad** (`genossi_mail/src/rest.rs:721-741`): Heute identisches Verhalten. SOLL geaendert werden.
- **Typst-Render-Pfad** (`genossi_rest/src/template.rs:269-327`): Heute KEIN Repayment-Pfad — der Endpoint `render_template` ruft `pdf_generator.render(...)` ohne RepaymentContext auf. Repayment-Letter-Templates koennen heute ueberhaupt NICHT via UI getestet werden. NEU: dedizierte Test-Route hinzufuegen.
- **Existierende Validation-Probe** (`genossi_mail/src/template.rs:230`): `validate_template_with_repayment` nutzt bereits Dummy-Werte `"0,00", 0, "0,00", 2026`. Diese sind ABSICHTLICH neutral fuer Probe-Renders (Pre-Send-Validation, keine Test-Anzeige). Im neuen Test-Pfad sind die Sentinel-Werte HOCH und auffaellig (99/99,99/2099), damit User klar sieht "das ist nicht real". `validate_template_with_repayment` bleibt unveraendert.
</key_existing_behaviors>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Backend Mail-Pfad — dummy_repayment_context Helper + Fallback in preview_mail/send_test_mail_with_template + used_dummy_repayment Flag</name>
  <files>genossi_mail/src/template.rs, genossi_mail/src/rest.rs</files>
  <behavior>
    Pure-fn `dummy_repayment_context()` in `genossi_mail/src/template.rs`:
    - Signatur: `pub fn dummy_repayment_context() -> (&'static str, i32, &'static str, i32)`
    - Rueckgabe: `("99,99", 99, "99,99", 2099)` — Sentinel-Werte hoch und einpraegsam
    - Doc-Comment erklaert: "Test-Endpoints only; NEVER call from worker/send-bulk path"
    - Test 1 (template.rs `mod tests`): `assert_eq!(dummy_repayment_context(), ("99,99", 99, "99,99", 2099))` — Locks die Sentinel-Werte fuer Stabilitaet (Frontend-Banner-Text verlaesst sich darauf)
    - Test 2 (template.rs): Render ein Template `"{{ payout_amount }} EUR fuer {{ share_count }} Anteile a {{ share_value }} EUR ({{ fiscal_year }})"` gegen `merge_repayment_context(member_ctx, dummy.0, dummy.1, dummy.2, dummy.3)` und assert Output `"99,99 EUR fuer 99 Anteile a 99,99 EUR (2099)"` — End-to-End-Render-Beweis

    `PreviewResponse` erweitern in `genossi_mail/src/rest.rs`:
    ```rust
    pub struct PreviewResponse {
        pub subject: String,
        pub body: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub errors: Vec<String>,
        // Quick 260603-kon: signalisiert dem Frontend, dass Dummy-Repayment-Daten
        // gerendert wurden (repayment_phase_id war gesetzt, aber Member hat keine
        // Open/Contacted-Entries). Frontend zeigt darauf einen Hinweis-Banner.
        // Bleibt false/abwesend wenn (a) kein repayment_phase_id geschickt wurde
        // ODER (b) echte Repayment-Daten gefunden wurden.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pub used_dummy_repayment: bool,
    }
    ```
    `std::ops::Not::not` filter: skip_serializing wenn `!field` true ist — also serialisiert nur wenn `used_dummy_repayment == true`. Backward-compat: aelterer Frontend-Code, der das Feld nicht kennt, ignoriert es einfach.

    `preview_mail` (`genossi_mail/src/rest.rs:572-647`) Fallback:
    Im match-Arm `Some(s) if !s.is_empty() =>` aendern:
    ```rust
    let (ctx, used_dummy_repayment) = match state.resolve_repayment_context(phase_id, member_id).await {
        Some((payout, share_count, share_value, fiscal_year)) => {
            (crate::template::merge_repayment_context(base_ctx, &payout, share_count, &share_value, fiscal_year), false)
        }
        None => {
            // Quick 260603-kon: Dummy-Fallback nur fuer Test-Pfade.
            let (payout, share_count, share_value, fiscal_year) = crate::template::dummy_repayment_context();
            (crate::template::merge_repayment_context(base_ctx, payout, share_count, share_value, fiscal_year), true)
        }
    };
    ```
    Im `_ => base_ctx` Branch (kein repayment_phase_id im Request) bleibt `used_dummy_repayment = false`. Response-Builder am Ende setzt `used_dummy_repayment` ins `PreviewResponse`.

    `send_test_mail_with_template` (`genossi_mail/src/rest.rs:705-770`) Fallback:
    Identisches Pattern wie `preview_mail` im match-Arm — gleiche Fallback-Logik. Da der Handler kein JSON-Response-Body mit Feldern liefert (heute `{"success": true}`), wird das Response-Body um ein `used_dummy_repayment: bool` Feld erweitert:
    ```rust
    let response_body = serde_json::json!({
        "success": true,
        "used_dummy_repayment": used_dummy_repayment
    });
    ```

    Test 3 (rest.rs `mod tests` — mit existierendem `MockMailRestState` Pattern): preview_mail mit `repayment_phase_id = Some(...)` und Mock `resolve_repayment_context -> None` -> Response enthaelt `"used_dummy_repayment":true` UND der gerenderte Body enthaelt `"99,99"` (Sentinel-Wert verifiziert Dummy-Pfad). Falls existierendes Mock-Pattern in rest.rs aufwendig ist (z.B. RestState-Trait mit vielen Methoden), nutze `cfg(test) impl` mit Stub-Struct nach Vorbild von `genossi_mail/src/rest.rs:~1009ff` Test-Modul.

    Test 4 (rest.rs): preview_mail mit `repayment_phase_id = None` -> Response enthaelt KEIN `used_dummy_repayment` Feld (skip_serializing_if). Verifiziert: kein silent-injection wenn Frontend gar keine Phase mitschickt.
  </behavior>
  <action>
    1. Read `genossi_mail/src/template.rs` (full file) und identifiziere wo Helper hin soll (nach `merge_repayment_context`, vor `validate_template_with_repayment`).
    2. Read `genossi_mail/src/rest.rs:1000-1100` um existierendes Mock/Test-Pattern fuer `MailRestState` zu verstehen.
    3. Implementiere `dummy_repayment_context()` mit doc-comment der Sentinel-Werte UND der Test-only-Beschraenkung.
    4. Schreibe Tests 1+2 in `template.rs` (RED -> GREEN).
    5. Erweitere `PreviewResponse` um `used_dummy_repayment: bool` mit `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`.
    6. Aendere `preview_mail` und `send_test_mail_with_template` per behavior-Spec.
    7. Schreibe Tests 3+4 in `rest.rs::tests`.
    8. Run: `cargo test -p genossi_mail` — alle bestehenden + 4 neue Tests gruen.
    9. Run: `cargo clippy -p genossi_mail --all-targets` — keine neuen Warnungen.

    **Anti-Pattern explizit vermeiden:**
    - Den Helper NICHT in `genossi_mail/src/worker.rs` aufrufen, auch nicht "fuer Symmetrie".
    - Die existierende `validate_template_with_repayment`-Probe (template.rs:197-249) NICHT aendern — sie nutzt absichtlich neutrale `"0,00"`-Werte und ist ein Validierungs-, kein Anzeige-Pfad.
    - Den Sentinel `99,99` als String-Literal NICHT inline kopieren; immer ueber `dummy_repayment_context()` ziehen, damit eine Aenderung an einer Stelle reicht.
  </action>
  <verify>
    <automated>cargo test -p genossi_mail --quiet 2>&amp;1 | tail -20</automated>
  </verify>
  <done>
    - `dummy_repayment_context()` in template.rs exportiert, returnt `("99,99", 99, "99,99", 2099)`.
    - `PreviewResponse.used_dummy_repayment: bool` mit `skip_serializing_if = "std::ops::Not::not"`.
    - `preview_mail` und `send_test_mail_with_template` fallen auf Dummy zurueck wenn `repayment_phase_id` gesetzt UND `resolve_repayment_context` None liefert.
    - 4 neue Tests gruen, alle bestehenden Mail-Tests bleiben gruen.
    - `genossi_mail/src/worker.rs` ist NICHT modifiziert (verify via `git diff genossi_mail/src/worker.rs` => leer).
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Backend Typst-Pfad — dummy_repayment_context_for_typst + neue Test-Route render_repayment_letter_test</name>
  <files>genossi_service_impl/src/pdf_generation.rs, genossi_rest/src/template.rs, genossi_bin/src/lib.rs</files>
  <behavior>
    Pure-fn `dummy_repayment_context_for_typst()` in `genossi_service_impl/src/pdf_generation.rs`:
    - Signatur: `pub fn dummy_repayment_context_for_typst() -> (genossi_dao::repayment_phase::RepaymentPhaseEntity, genossi_service::repayment_context::RepaymentContext)`
    - Erzeugt:
      - `RepaymentPhaseEntity` mit `id = Uuid::nil()`, `fiscal_year = 2099`, `share_value = 9999` (Cent = "99,99" EUR), `created = current time`, `deleted = None`, `version = Uuid::nil()`, plus alle anderen Pflichtfelder via `Default::default()` oder dokumentierten Default-Werten. Falls die Entity kein `Default` hat, alle Felder explizit setzen (lies dazu die Struktur in `genossi_dao/src/repayment_phase.rs`).
      - `RepaymentContext { share_count: 99, payout_amount: "99,99".to_string(), fiscal_year: 2099 }`
    - Doc-Comment: "Test-Endpoints only; NEVER call from production letter-service / worker-paths"
    - Test 1 (pdf_generation.rs `mod tests`): Lock-Test der Sentinel-Werte: `let (phase, ctx) = dummy_repayment_context_for_typst(); assert_eq!(phase.fiscal_year, 2099); assert_eq!(phase.share_value, 9999); assert_eq!(ctx.share_count, 99); assert_eq!(ctx.payout_amount, "99,99"); assert_eq!(ctx.fiscal_year, 2099);`

    Neue Route in `genossi_rest/src/template.rs`:
    - `pub fn generate_render_repayment_test_route<RestState: RestStateDef>() -> Router<RestState>` analog zu `generate_render_route`, mit Pfad-Pattern `/{*path}`.
    - Handler `render_repayment_letter_test`:
      ```rust
      async fn render_repayment_letter_test<RestState: RestStateDef>(
          rest_state: State<RestState>,
          Extension(context): Extension<Context>,
          Path(path): Path<String>,
      ) -> Response
      ```
    - Logik:
      1. extract_auth_context + `check_permission("manage_members", auth)`.
      2. `parse_render_path(&path)` -> `(template_path, member_id_str)`.
      3. Member laden via `rest_state.member_service().get(member_id, auth, None)`.
      4. `let (dummy_phase, dummy_ctx) = pdf_generation::dummy_repayment_context_for_typst();`
      5. `rest_state.pdf_generator().render_repayment_letter(&template_path, rest_state.template_storage().base_path(), &dummy_phase, &member, &dummy_ctx)`.
      6. Response: PDF mit `Content-Type: application/pdf`, `Content-Disposition: attachment; filename="<name>.pdf"`. Filename-Generierung identisch zu existierendem `render_template`-Handler (`.typ` -> `.pdf`).
    - **Privacy-Note** im doc-comment: Member-Daten flowen in den Letter; Vorstand sollte das Test-PDF nicht weiterverteilen. Konsistent mit existierender Privacy-Disziplin (Mail-Tester-Doc).

    Test 2 (rest_state_mock-basierter Integrationstest, falls vorhanden — sonst Unit-Test der Pfad-Parser-Reuse). Pragmatisches Minimum:
    - Test in `genossi_service_impl/src/pdf_generation.rs::tests`: Lade Default-Template `"defaults/auszahlungs_anschreiben.typ"` via existierendes Test-Pattern (siehe `pdf_generation.rs:2168-2300` als Vorlage), erzeuge dummy `MemberEntity`, dann
      ```rust
      let (phase, ctx) = dummy_repayment_context_for_typst();
      let pdf = generator.render_repayment_letter(
          "auszahlungs_anschreiben.typ",
          &template_base,
          &phase,
          &dummy_member,
          &ctx,
      ).expect("render with dummy ctx must succeed");
      assert!(pdf.starts_with(b"%PDF-"));
      ```
    - Damit ist E2E bewiesen: Dummy-Werte gehen durch typst-compile, PDF kommt raus.

    Wiring in `genossi_bin/src/lib.rs`:
    - In `setup_router` (oder wo `generate_render_route` heute angehaengt wird) Route `/api/templates/render-repayment-test` zusaetzlich registrieren. Suche das existierende Mount-Pattern via `grep -n "generate_render_route" genossi_bin/src/lib.rs` und spiegle es. Keine Default-Templates wechseln, keine Pfade umbenennen.
  </behavior>
  <action>
    1. Read `genossi_dao/src/repayment_phase.rs` rund um `RepaymentPhaseEntity` Definition — alle Pflichtfelder identifizieren.
    2. Read `genossi_service_impl/src/pdf_generation.rs:2168-2330` um existierendes Test-Pattern fuer `render_repayment_letter` (template_base setup, dummy member) zu uebernehmen.
    3. Read `genossi_rest/src/template.rs:269-415` als Vorlage fuer `render_repayment_letter_test`.
    4. Read `genossi_bin/src/lib.rs` rund um `generate_render_route` Mount-Punkt.
    5. Implementiere `dummy_repayment_context_for_typst()` mit doc-comment + Test 1.
    6. Implementiere `render_repayment_letter_test` Handler + `generate_render_repayment_test_route`.
    7. Implementiere Test 2 (E2E PDF-render mit Dummy-Werten).
    8. Mounte neue Route in `genossi_bin/src/lib.rs`.
    9. Erweitere `#[openapi(paths(...))]` in `genossi_rest/src/template.rs:417-431` um den neuen Handler (analog zu `render_template`).
    10. Run: `cargo test -p genossi_service_impl --quiet` + `cargo test -p genossi_rest --quiet`.
    11. Run: `cargo build --bin genossi` — final Compile-Check inkl. Router-Wiring.
    12. Run: `cargo clippy --all-targets -p genossi_service_impl -p genossi_rest -p genossi_bin`.

    **Anti-Pattern explizit vermeiden:**
    - Den Dummy-Helper NICHT in `genossi_service_impl/src/repayment_letter.rs` (`generate_letters_for_phase` o.ae.) aufrufen — der Bundle-Render-Pfad ist Produktiv-Code und MUSS echte Phasen + Contexts nutzen (sonst Audit-Korruption).
    - Den Default-Template-Path nicht aendern; Vorstand testet exakt das gleiche Template wie spaeter im Produktiv-Render.
    - Keine Audit-Macros im Test-Handler aufrufen — Template-Tests sind nicht auditierbar.
    - Wenn `RepaymentPhaseEntity::default()` nicht existiert, alle Felder explizit setzen anstatt das Trait zu impl'en (Trait-Impl waere ungewollte API-Surface).
  </action>
  <verify>
    <automated>cargo test -p genossi_service_impl --quiet 2>&amp;1 | tail -15 &amp;&amp; cargo test -p genossi_rest --quiet 2>&amp;1 | tail -10 &amp;&amp; cargo build --bin genossi 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    - `dummy_repayment_context_for_typst()` liefert (Phase{share_value=9999, fiscal_year=2099}, Ctx{share_count=99, payout_amount="99,99", fiscal_year=2099}).
    - Neue Route `POST /api/templates/render-repayment-test/{*path}/{member_id}` rendert ein Repayment-Letter-PDF.
    - PDF startet mit `%PDF-` Header (verified im Test).
    - `cargo build --bin genossi` durchlaeuft.
    - `genossi_service_impl/src/repayment_letter.rs` ist NICHT modifiziert (verify via `git diff genossi_service_impl/src/repayment_letter.rs` => leer).
    - Worker-Pfad weiterhin unangetastet (`git diff genossi_mail/src/worker.rs` => leer).
  </done>
</task>

<task type="auto">
  <name>Task 3: Frontend — used_dummy_repayment Hinweis-Banner im TemplateTester + API-Anbindung neue Typst-Test-Route</name>
  <files>genossi-frontend/src/component/mail_compose/template_preview.rs, genossi-frontend/src/component/mail_compose/template_tester.rs, genossi-frontend/src/api.rs, genossi-frontend/src/i18n/mod.rs, genossi-frontend/src/i18n/de.rs, genossi-frontend/src/i18n/en.rs</files>
  <behavior>
    Frontend-Schnittstelle erweitern:
    1. `genossi-frontend/src/api.rs`: Struct (oder Type-Alias), die `PreviewResponse` deserialisiert, um Feld `used_dummy_repayment: bool` mit `#[serde(default)]` ergaenzen. Wenn die Response heute via `serde_json::Value`-Inspection geparst wird, das Feld an der Parse-Stelle (`preview_mail` Funktion) lesen.
    2. `TemplatePreview` (`genossi-frontend/src/component/mail_compose/template_preview.rs`):
       - Output-State-Signal um `used_dummy_repayment: Signal<bool>` erweitern (oder Tuple-Signal `(String, String, Vec<String>, bool)` falls bisheriger Stil).
       - Bei `trigger_preview` Erfolg den Wert aus der API-Response setzen.
       - Im RSX nach dem rendered-Body-Block, BEDINGT auf `used_dummy_repayment == true`, ein amber Hinweis-Banner einfuegen:
         ```rsx
         if used_dummy_repayment.read().clone() {
             div { class: "mt-2 px-3 py-2 bg-amber-50 border border-amber-200 rounded text-xs text-amber-800",
                 {i18n.t(Key::MailTemplateTestDummyRepaymentHint)}
             }
         }
         ```
       - Hinweis-Text (DE): "Test-Modus: Mitglied hat keine aktive Rueckzahlung — Repayment-Platzhalter werden mit Dummy-Werten gefuellt (99,99 EUR, 99 Anteile, Jahr 2099)."
       - Hinweis-Text (EN): "Test mode: member has no active repayment — Repayment placeholders rendered with dummy values (99.99 EUR, 99 shares, year 2099)."
    3. i18n-Key `MailTemplateTestDummyRepaymentHint` in `mod.rs` Key-Enum + DE+EN-Translation in beiden Locale-Dateien.
    4. `TemplateTester` muss NICHT geaendert werden — es nutzt `TemplatePreview` als Kind, der Banner taucht automatisch dort auf, wo der Tester den Preview einbettet. Aber: Falls der Tester den Preview NICHT verwendet (sondern eigene Render-Logik), Banner-Insertion auch dort spiegeln (verify via Re-Read).
    5. Component-First-Check: kein inline-RSX-Duplikat — wenn der Banner woanders im Frontend ebenfalls auftaucht (z.B. spaeter im Typst-Test), wandert er in eine eigene Component `DummyRepaymentBanner`. Fuer diesen Quick-Task reicht inline im `TemplatePreview`, ABER mit TODO-Kommentar: "Falls 2. Verwender auftaucht, in component/ extrahieren."

    Optional (nice-to-have, NICHT Pflicht fuer diese Quick-Task): API-Helper `render_template_pdf_repayment_test(path, member_id)` analog zu `render_template_pdf` (`genossi-frontend/src/api.rs:508`) — ruft neue Backend-Route auf, returnt PDF-Blob. Wenn die Typst-Editor-Seite (`templates.rs`) heute schon einen "Repayment-Letter testen"-Knopf hat, anschliessen; falls nicht, Backend-Route ist trotzdem fertig und kann spaeter via curl/Swagger verwendet werden. Frontend-UI-Knopf ist OUT-OF-SCOPE dieser Quick-Task (waere eigener Tester wie der Mail-Tester).

    Tests (Frontend):
    - **Frontend-Test 1** (`api.rs::tests` oder `template_preview.rs::tests`): Deserialisierungs-Test fuer Preview-Response mit `"used_dummy_repayment":true` -> Feld liest sich korrekt aus.
    - **Frontend-Test 2**: Deserialisierungs-Test fuer Preview-Response OHNE das Feld (Backward-Compat) -> Default `false`.
    - Wenn `cargo test -p genossi-frontend` nicht im Standard-Workflow laeuft (WASM-Build), beschraenke auf pure Serde-Tests in `mod tests { ... }` — keine Dioxus-Component-Render-Tests noetig.
  </behavior>
  <action>
    1. Read `genossi-frontend/src/api.rs` rund um `preview_mail` (existierender API-Caller) und identifiziere Response-Parsing.
    2. Read `genossi-frontend/src/component/mail_compose/template_preview.rs` (full file) um Output-State-Pattern zu verstehen.
    3. Read `genossi-frontend/src/component/mail_compose/template_tester.rs` (full file) um zu pruefen, ob Tester den Preview einbettet oder eigene Render-Logik hat.
    4. Read `genossi-frontend/src/i18n/mod.rs` um Key-Enum-Pattern zu sehen.
    5. Erweitere API-Response-Parsing um `used_dummy_repayment: bool` mit `#[serde(default)]`.
    6. Erweitere `TemplatePreview` State + RSX um den Banner.
    7. Fuege i18n-Key `MailTemplateTestDummyRepaymentHint` zu mod.rs hinzu, DE + EN translations (Component-First-CLAUDE.md fordert: BEIDE Locales!).
    8. Schreibe 2 Serde-Tests in `api.rs::tests`.
    9. Run: `cargo build -p genossi-frontend` (oder das Workspace-Build).
    10. Run: `cargo test -p genossi-frontend --quiet` (falls testbar).
    11. Run: `cargo clippy -p genossi-frontend --all-targets` — keine neuen Warnungen.

    **Anti-Pattern explizit vermeiden:**
    - Banner NICHT inline kopieren, falls er an mehr als einer Stelle landet (Component-First).
    - NICHT in der `Locale::En`-Arm-Variante mit deutschem Text antworten und vice versa (siehe `genossi-frontend/CLAUDE.md` Bug-Memo).
    - Hardcoded Sentinel-Werte im UI-Text sind OK (sie sind Teil der UX-Botschaft "diese Werte siehst du, das ist Dummy"); aber wenn Backend die Sentinel je aendert, muessen DE+EN nachgezogen werden — dafuer Lock-Test 1 in Task 1.
    - KEINE neue API-Call-Route fuer den Typst-Test-Endpoint bauen, wenn die Typst-Editor-Seite (`templates.rs`) keinen Testknopf hat — Frontend-UI-Knopf ist nicht Scope. Backend-Route ist via Swagger/curl benutzbar; UI kann in spaeterem Quick nachgezogen werden.
  </action>
  <verify>
    <automated>cargo build -p genossi-frontend 2>&amp;1 | tail -5 &amp;&amp; cargo test -p genossi-frontend --quiet 2>&amp;1 | tail -10</automated>
  </verify>
  <done>
    - `TemplatePreview` zeigt amber Banner wenn API `used_dummy_repayment: true` liefert.
    - i18n-Key `MailTemplateTestDummyRepaymentHint` in beiden Locales gepflegt.
    - 2 Serde-Tests gruen (Feld present + Feld absent fuer Backward-Compat).
    - `cargo build -p genossi-frontend` durchlaeuft ohne neue Warnings.
    - Sentinel-Werte (99,99 / 99 / 2099) im UI-Text sichtbar — User erkennt Dummy-Modus visuell.
  </done>
</task>

</tasks>

<verification>
**Aggregat-Verifikation (manuell, am Ende):**

1. **Mail-Pfad happy path (ohne aktive Phase):**
   - Backend hochziehen: `cargo run --bin genossi`
   - Frontend: `dx serve`
   - Mail-Template-Editor oeffnen, Template mit `{{ payout_amount }}` und `{{ share_count }}` schreiben, Test-Member auswaehlen (irgendein Member ohne aktive RepaymentPhase), `repayment_phase_id` einer **alten/abgeschlossenen** Phase setzen.
   - **Erwartung:** Preview zeigt `99,99` und `99` im Body, amber Banner sichtbar unter dem Preview. Test-Mail-Versand an `test-empfaenger@example.com` funktioniert.

2. **Mail-Pfad happy path (mit aktiver Phase):**
   - Template-Editor mit gleichem Template, Test-Member der `Open` Entries in aktueller Phase hat, `repayment_phase_id` der aktuellen Phase.
   - **Erwartung:** Preview zeigt echte Werte (NICHT 99,99), KEIN amber Banner.

3. **Mail-Pfad regression check (kein repayment_phase_id):**
   - Template ohne Repayment-Variablen, kein `repayment_phase_id`.
   - **Erwartung:** Preview unveraendert wie vor dieser Quick-Task.

4. **Typst-Pfad:**
   - `curl -X POST -H "Cookie: <session>" http://localhost:3000/api/templates/render-repayment-test/defaults/auszahlungs_anschreiben.typ/<member_id> --output test.pdf`
   - **Erwartung:** test.pdf laesst sich oeffnen, Member-Daten sind real, Repayment-Daten sind Dummy (99,99 EUR / 99 Anteile / 2099).

5. **Worker-Pfad-Schutz:**
   - `git diff genossi_mail/src/worker.rs` -> MUSS leer sein.
   - `git diff genossi_service_impl/src/repayment_letter.rs` -> MUSS leer sein.
   - `grep -rn "dummy_repayment" genossi_mail/src/worker.rs genossi_service_impl/src/repayment_letter.rs` -> MUSS 0 Treffer ergeben.

6. **Audit-Pfad-Schutz:**
   - Test-Mail loest keine `mail_jobs`-DB-Insert aus (nur SMTP-Send via `send_test_mail_with_body`).
   - Typst-Test loest keine `member_documents`-Insert aus (`render_repayment_letter` ist pure, der Test-Handler ruft KEIN `audited_create!`).
   - Verify: nach Test-Run mit `sqlite3 genossi.db "SELECT count(*) FROM mail_jobs WHERE created > datetime('now', '-5 minutes')"` -> 0.
</verification>

<success_criteria>
- [x] Backend-Tests: `cargo test -p genossi_mail` gruen (4+ neue Tests), `cargo test -p genossi_service_impl` gruen (1+ neuer Test, optional + E2E-PDF-Test), `cargo test -p genossi_rest` gruen.
- [x] Frontend-Build: `cargo build -p genossi-frontend` ohne neue Warnings.
- [x] `cargo build --bin genossi` durchlaeuft (Router-Wiring funktioniert).
- [x] Worker-Pfad und Repayment-Letter-Service-Pfad sind nachweislich unangetastet (git diff leer auf den beiden Dateien).
- [x] Sentinel-Werte (99,99 / 99 / 99,99 / 2099) sind im Code an EINER Stelle definiert (Helper in `template.rs` + Helper in `pdf_generation.rs`).
- [x] Frontend zeigt amber Hinweis-Banner mit DE+EN-Texten, wenn Dummy-Werte verwendet wurden.
- [x] Neue REST-Route `/api/templates/render-repayment-test/{*path}/{member_id}` rendert ein PDF.
- [x] Manuelle Verifikation 1-6 (siehe `<verification>`) bestanden — insbesondere Worker-Pfad-Schutz und Audit-Pfad-Schutz.
</success_criteria>

<output>
After completion, create `.planning/quick/260603-kon-beim-testen-von-templates-fehlen-dummy-d/260603-kon-SUMMARY.md`
</output>
