use async_trait::async_trait;
use genossi_dao::member::MemberEntity;
use minijinja::{context, Value};
use mockall::automock;
use time::macros::format_description;
use uuid::Uuid;

use crate::service::MailServiceError;

#[automock]
#[async_trait]
pub trait MemberResolver: Send + Sync + 'static {
    async fn find_member_by_id(&self, id: Uuid) -> Result<Option<MemberEntity>, MailServiceError>;
}

pub fn member_to_template_context(entity: &MemberEntity) -> Value {
    let salutation_str = entity.salutation.as_ref().map(|s| s.as_str().to_string());
    // FMT-01 (Phase 23, D-11): dates render as DD.MM.YYYY in both text AND html
    // bodies — shared context feeds both envs, so wiring format_de here is the
    // single source of truth.
    let join_date_str = format_de(entity.join_date);
    let exit_date_str = entity.exit_date.map(format_de);
    // Quick 260603-b43: masked_bank_account = bank_account maskiert (DSGVO-konforme
    // Anzeige in E-Mail-Templates). Bei None bleibt das Feld None — Templates können
    // mit `{% if masked_bank_account %}` darauf reagieren.
    let masked_bank_account = entity
        .bank_account
        .as_deref()
        .map(genossi_service::iban::mask_iban);
    context! {
        member_number => entity.member_number,
        first_name => entity.first_name.as_ref(),
        last_name => entity.last_name.as_ref(),
        email => entity.email.as_deref(),
        company => entity.company.as_deref(),
        comment => entity.comment.as_deref(),
        street => entity.street.as_deref(),
        house_number => entity.house_number.as_deref(),
        postal_code => entity.postal_code.as_deref(),
        city => entity.city.as_deref(),
        join_date => join_date_str,
        shares_at_joining => entity.shares_at_joining,
        current_shares => entity.current_shares,
        current_balance => entity.current_balance,
        exit_date => exit_date_str,
        bank_account => entity.bank_account.as_deref(),
        masked_bank_account => masked_bank_account,
        migrated => entity.migrated,
        salutation => salutation_str,
        title => entity.title.as_deref(),
    }
}

#[derive(Debug)]
pub struct TemplateError {
    pub message: String,
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn strict_env() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env
}

pub fn render_template(template_str: &str, context: &Value) -> Result<String, TemplateError> {
    let env = strict_env();
    let tmpl = env
        .template_from_str(template_str)
        .map_err(|e| TemplateError {
            message: format!("Template syntax error: {}", e),
        })?;
    tmpl.render(context).map_err(|e| TemplateError {
        message: format!("Template render error: {}", e),
    })
}

/// Phase 23 D-04 (HTML-04): separate autoescaping minijinja env for the HTML
/// body. Same strictness as [`strict_env`] plus a global HTML-autoescape
/// callback — a member value like `<script>&Co` renders as
/// `&lt;script&gt;&amp;Co` while the author's markup (`<p>Hallo</p>`) stays
/// intact. Kept intentionally separate from `strict_env` so text bodies +
/// subjects continue to render raw.
pub fn html_env() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_name| minijinja::AutoEscape::Html);
    env
}

/// Render an HTML body template through the autoescaping env ([`html_env`]).
///
/// Mirrors [`render_template`] but uses the HTML env; error messages prefix
/// "HTML template …" so downstream error handling can distinguish the two
/// paths.
pub fn render_html_template(template_str: &str, context: &Value) -> Result<String, TemplateError> {
    let env = html_env();
    let tmpl = env
        .template_from_str(template_str)
        .map_err(|e| TemplateError {
            message: format!("HTML template syntax error: {}", e),
        })?;
    tmpl.render(context).map_err(|e| TemplateError {
        message: format!("HTML template render error: {}", e),
    })
}

/// FMT-01 (Phase 23, D-11): render a [`time::Date`] as `DD.MM.YYYY`. Applied
/// in [`member_to_template_context`] to `join_date` and `exit_date` so both
/// text and HTML bodies inherit the German format via the shared context.
///
/// Falls back to `date.to_string()` on the impossible formatting-error branch
/// (a well-formed `Date` + fixed pattern cannot fail in practice, but the
/// guard preserves the pre-existing render behavior on the pathological path).
fn format_de(date: time::Date) -> String {
    const FMT: &[time::format_description::BorrowedFormatItem<'static>] =
        format_description!("[day].[month].[year]");
    date.format(FMT).unwrap_or_else(|_| date.to_string())
}

pub fn validate_template(
    subject: &str,
    body: &str,
    members: &[MemberEntity],
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let env = strict_env();

    // Check syntax first
    if let Err(e) = env.template_from_str(subject) {
        errors.push(format!("Subject syntax error: {}", e));
    }
    if let Err(e) = env.template_from_str(body) {
        errors.push(format!("Body syntax error: {}", e));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Probe-render against all members
    let subject_tmpl = env.template_from_str(subject).unwrap();
    let body_tmpl = env.template_from_str(body).unwrap();

    for member in members {
        let ctx = member_to_template_context(member);
        if let Err(e) = subject_tmpl.render(&ctx) {
            errors.push(format!(
                "Subject render error for member #{}: {}",
                member.member_number, e
            ));
        }
        if let Err(e) = body_tmpl.render(&ctx) {
            errors.push(format!(
                "Body render error for member #{}: {}",
                member.member_number, e
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Phase 10 D-04 (MAIL-02): merge per-recipient repayment-context into a base context.
///
/// Adds four variables to a minijinja context that the Worker pre-computed from
/// RepaymentEntry aggregation + RepaymentPhase lookup:
/// - `payout_amount`: German-localized euro string, format "X,YZ" (e.g. "60,00").
/// - `share_count`: total share count being paid out (i32).
/// - `share_value`: phase-wide Anteilswert pro Anteil, German-localized euro
///   string, format "X,YZ" (e.g. "20,00"). Quick 260602-r2i.
/// - `fiscal_year`: phase.fiscal_year (i32, e.g. 2026).
///
/// The base context (typically produced by `member_to_template_context`) is
/// preserved verbatim. The four new fields are appended; if base happens to
/// contain a clashing name, the new value wins (Phase 10 accepts that — the
/// worker only calls this for repayment-flagged jobs).
///
/// D-13 strict opt-in: if the worker does NOT call this helper (D-05 edge-case
/// where a member has 0 Open/Contacted entries), templates that reference
/// `payout_amount`/`share_count`/`share_value`/`fiscal_year` without
/// `{% if %}`-guards will fail render under strict-env → triggers
/// `mark_recipient_failed`.
///
/// Implementation note: `context! { ..base, ... }`-spread is not supported by
/// minijinja 2.19; we round-trip `base` through `serde_json` into a `BTreeMap`,
/// insert the four new fields, and convert back via `Value::from_serialize`.
pub fn merge_repayment_context(
    base: Value,
    payout_amount: &str,
    share_count: i32,
    share_value: &str,
    fiscal_year: i32,
) -> Value {
    use std::collections::BTreeMap;

    // Step 1: convert base Value into a BTreeMap<String, serde_json::Value>.
    let base_json: serde_json::Value =
        serde_json::to_value(&base).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let mut map: BTreeMap<String, serde_json::Value> = match base_json {
        serde_json::Value::Object(obj) => obj.into_iter().collect(),
        _ => BTreeMap::new(),
    };

    // Step 2: insert the 4 new fields (overwrites base if name clashes).
    map.insert(
        "payout_amount".to_string(),
        serde_json::Value::String(payout_amount.to_string()),
    );
    map.insert(
        "share_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(share_count)),
    );
    map.insert(
        "share_value".to_string(),
        serde_json::Value::String(share_value.to_string()),
    );
    map.insert(
        "fiscal_year".to_string(),
        serde_json::Value::Number(serde_json::Number::from(fiscal_year)),
    );

    // Step 3: convert the merged map back into a minijinja Value.
    Value::from_serialize(&map)
}

/// Quick 260603-kon: Sentinel Dummy-Repayment-Werte fuer Template-Test-Pfade.
///
/// Liefert `(payout_amount, share_count, share_value, fiscal_year)` mit
/// auffallend hohen Sentinel-Werten (`"99,99"`, `99`, `"99,99"`, `2099`),
/// damit Vorstandsmitglieder im Mail-Tester sofort visuell erkennen, dass
/// es sich um Dummy-Daten handelt — distinkt vom neutralen `"0,00"`-Pfad
/// in `validate_template_with_repayment` (das ist eine reine
/// Pre-Send-Probe ohne UI-Display).
///
/// **WARNUNG — Test-Endpoints only:** Diese Funktion darf AUSSCHLIESSLICH
/// von `/api/mail/preview` und `/api/mail/test-with-template` aufgerufen
/// werden, wenn `repayment_phase_id` gesetzt ist aber der Member keine
/// Open/Contacted-Entries hat. NIEMALS aus `worker.rs` (Bulk-Send) oder
/// aus `repayment_letter.rs` (Produktiv-Brief-Render) — dort wuerden
/// Sentinel-Werte ins Audit-Log und in echte E-Mails an Mitglieder
/// lecken, was DSGVO-relevant und verbandskonformitaets-relevant ist.
///
/// Frontend (Phase-12-`TemplatePreview`) liest das `used_dummy_repayment`-
/// Flag aus `PreviewResponse` und zeigt einen amber Hinweis-Banner mit
/// den hier definierten Sentinel-Werten an. Aenderungen an den
/// Sentinel-Werten muessen die DE+EN-Banner-Texte synchron nachziehen
/// (Lock-Test `test_dummy_repayment_context_sentinel_values_locked`).
pub fn dummy_repayment_context() -> (&'static str, i32, &'static str, i32) {
    ("99,99", 99, "99,99", 2099)
}

/// Quick 260603-n3m: Detektiert, ob ein Template (subject ODER body) eine
/// der vier Repayment-Variablen referenziert (`payout_amount`,
/// `share_count`, `share_value`, `fiscal_year`).
///
/// Wird von den Test-Endpoints (`preview_mail` /
/// `send_test_mail_with_template`) in `rest.rs` aufgerufen, um zu
/// entscheiden, ob der Dummy-Repayment-Context auch dann gemergt werden
/// muss, wenn der Caller (Template-Editor) gar kein `repayment_phase_id`
/// geschickt hat.
///
/// Implementierungs-Entscheidung: simple Substring-Suche statt
/// AST-Parsing. Jinja-Ausdruecke wie `{{ payout_amount }}`,
/// `{% if payout_amount is defined %}` und sogar Kommentare
/// `{# payout_amount #}` enthalten den Variablen-Namen als Substring.
/// False-Positives bei Literalen, die zufaellig "payout_amount" als
/// Plain-Text enthalten (unwahrscheinlich), sind harmlos: der Dummy-Merge
/// ist additiv und ueberschreibt nichts.
///
/// Trade-off versus "immer mergen": waeren wir immer-merge, wuerden auch
/// Non-Repayment-Templates `used_dummy_repayment: true` setzen und der
/// amber Banner wuerde luegen ("Vorsicht, Dummy-Daten" — aber das Template
/// nutzt sie gar nicht). Detection haelt den Banner vertrauenswuerdig.
pub fn template_uses_repayment_vars(subject: &str, body: &str) -> bool {
    const REPAYMENT_VARS: [&str; 4] =
        ["payout_amount", "share_count", "share_value", "fiscal_year"];
    REPAYMENT_VARS
        .iter()
        .any(|v| subject.contains(v) || body.contains(v))
}

/// Phase 10 D-14 (additive, Planner-Discretion): probe-render templates against
/// both Member-context AND a dummy Repayment-context. Catches references like
/// `{{ payout_amount }}` without `{% if %}`-guards before the worker actually
/// sends mails — fail-fast in REST validation.
///
/// Used by the REST send-bulk endpoint when `SendBulkMailRequest.repayment_phase_id`
/// is `Some` (call-site wired in a later plan; the helper is available now so
/// tests can be added next to the rest of the template helpers).
pub fn validate_template_with_repayment(
    subject: &str,
    body: &str,
    members: &[MemberEntity],
) -> Result<(), Vec<String>> {
    // UAT-Defekt #5 Fix (Phase-10 Bug): Pure-member-Probe entfernt — sie würde
    // bei jedem Body mit `{{ payout_amount }}` o.ä. fehlschlagen, BEVOR die
    // merged-context-Probe (die diese Variablen kennt) jemals durchläuft.
    // Der merged-context enthält das komplette pure-member-Context plus die
    // drei Repayment-Vars; alle pure-member-Fehler werden hier ebenfalls
    // abgefangen, also kein Verlust an Validierungstiefe.

    let mut errors = Vec::new();
    let env = strict_env();
    let subject_tmpl = match env.template_from_str(subject) {
        Ok(t) => t,
        Err(e) => {
            errors.push(format!("Subject syntax error: {}", e));
            return Err(errors);
        }
    };
    let body_tmpl = match env.template_from_str(body) {
        Ok(t) => t,
        Err(e) => {
            errors.push(format!("Body syntax error: {}", e));
            return Err(errors);
        }
    };
    for member in members {
        let base = member_to_template_context(member);
        // Quick 260602-r2i: share_value als 4. Variable (Dummy "0,00") fuer
        // Validation-Probe — Templates die {{ share_value }} ungeguarded
        // referenzieren werden so abgefangen.
        let merged = merge_repayment_context(base, "0,00", 0, "0,00", 2026);
        if let Err(e) = subject_tmpl.render(&merged) {
            errors.push(format!(
                "Subject render error with repayment context for member #{}: {}",
                member.member_number, e
            ));
        }
        if let Err(e) = body_tmpl.render(&merged) {
            errors.push(format!(
                "Body render error with repayment context for member #{}: {}",
                member.member_number, e
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn render_footer(template_str: &str, sender_name: &str) -> Result<String, TemplateError> {
    let ctx = context! {
        sender_name => sender_name,
    };
    let env = strict_env();
    let tmpl = env
        .template_from_str(template_str)
        .map_err(|e| TemplateError {
            message: format!("Footer template syntax error: {}", e),
        })?;
    tmpl.render(&ctx).map_err(|e| TemplateError {
        message: format!("Footer template render error: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_dao::member::Salutation;
    use std::sync::Arc;

    fn make_member(first_name: &str, last_name: &str) -> MemberEntity {
        let date = time::Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberEntity {
            id: Uuid::new_v4(),
            member_number: 42,
            first_name: Arc::from(first_name),
            last_name: Arc::from(last_name),
            salutation: Some(Salutation::Herr),
            title: Some(Arc::from("Dr.")),
            email: Some(Arc::from("max@example.com")),
            company: None,
            comment: None,
            street: Some(Arc::from("Musterstraße")),
            house_number: Some(Arc::from("12")),
            postal_code: Some(Arc::from("12345")),
            city: Some(Arc::from("Berlin")),
            join_date: date,
            shares_at_joining: 1,
            current_shares: 3,
            current_balance: 15000,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: Some(Arc::from("DE89370400440532013000")),
            status: genossi_dao::member::MemberStatus::Normal,
            account_holder: None,
            postal_status: genossi_dao::member::PostalStatus::Erreichbar,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_simple_variable_substitution() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("Hallo {{ first_name }} {{ last_name }}", &ctx).unwrap();
        assert_eq!(result, "Hallo Max Mustermann");
    }

    #[test]
    fn test_conditional_logic() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let template = r#"{% if salutation == "Herr" %}Sehr geehrter Herr{% elif salutation == "Frau" %}Sehr geehrte Frau{% endif %} {{ last_name }}"#;
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Sehr geehrter Herr Mustermann");
    }

    #[test]
    fn test_null_field_conditional() {
        let member = make_member("Max", "Mustermann");
        // company is None
        let ctx = member_to_template_context(&member);
        let template = "{% if company %}Firma: {{ company }}{% endif %}Ende";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Ende");
    }

    #[test]
    fn test_present_optional_field() {
        let mut member = make_member("Max", "Mustermann");
        member.company = Some(Arc::from("ACME GmbH"));
        let ctx = member_to_template_context(&member);
        let template = "{% if company %}Firma: {{ company }}{% endif %}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Firma: ACME GmbH");
    }

    #[test]
    fn test_numeric_fields() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template(
            "Nr: {{ member_number }}, Anteile: {{ current_shares }}",
            &ctx,
        )
        .unwrap();
        assert_eq!(result, "Nr: 42, Anteile: 3");
    }

    #[test]
    fn test_title_field() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let template = "{% if title %}{{ title }} {% endif %}{{ first_name }} {{ last_name }}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Dr. Max Mustermann");
    }

    #[test]
    fn test_syntax_error() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("Hallo {{ first_name", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("syntax error"));
    }

    #[test]
    fn test_unknown_variable() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("{{ nonexistent_field }}", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_template_valid() {
        let members = vec![
            make_member("Max", "Mustermann"),
            make_member("Erika", "Muster"),
        ];
        let result =
            validate_template("Hallo {{ first_name }}", "Lieber {{ last_name }}", &members);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_template_syntax_error() {
        let members = vec![make_member("Max", "Mustermann")];
        let result = validate_template("Ok", "{{ unclosed", &members);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("syntax error")));
    }

    #[test]
    fn test_validate_template_unknown_variable() {
        let members = vec![make_member("Max", "Mustermann")];
        let result = validate_template("Ok", "{{ nonexistent }}", &members);
        assert!(result.is_err());
    }

    #[test]
    fn test_plain_text_passthrough() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("Keine Variablen hier.", &ctx).unwrap();
        assert_eq!(result, "Keine Variablen hier.");
    }

    #[test]
    fn test_date_fields() {
        // FMT-01 (Phase 23, D-11): join_date renders as DD.MM.YYYY (was 2025-01-15).
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("Beitritt: {{ join_date }}", &ctx).unwrap();
        assert_eq!(result, "Beitritt: 15.01.2025");
    }

    #[test]
    fn test_exit_date_null() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let template = "{% if exit_date %}Austritt: {{ exit_date }}{% else %}Aktiv{% endif %}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Aktiv");
    }

    const TEMPLATE_FORMAL: &str = r#"Sehr geehrte{% if salutation == "Herr" %}r Herr{% elif salutation == "Frau" %} Frau{% else %}s Mitglied{% endif %}{% if title %} {{ title }}{% endif %} {{ last_name }},"#;

    const TEMPLATE_INFORMAL: &str = r#"{% if salutation == "Herr" %}Lieber{% elif salutation == "Frau" %}Liebe{% else %}Hallo{% endif %}{% if title %} {{ title }}{% endif %} {{ first_name }},"#;

    #[test]
    fn test_formal_template_herr_with_title() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_FORMAL, &ctx).unwrap();
        assert_eq!(result, "Sehr geehrter Herr Dr. Mustermann,");
    }

    #[test]
    fn test_formal_template_frau_without_title() {
        let mut member = make_member("Erika", "Muster");
        member.salutation = Some(Salutation::Frau);
        member.title = None;
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_FORMAL, &ctx).unwrap();
        assert_eq!(result, "Sehr geehrte Frau Muster,");
    }

    #[test]
    fn test_formal_template_no_salutation() {
        let mut member = make_member("Simon", "Goller");
        member.salutation = None;
        member.title = None;
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_FORMAL, &ctx).unwrap();
        assert_eq!(result, "Sehr geehrtes Mitglied Goller,");
    }

    #[test]
    fn test_formal_template_frau_with_title() {
        let mut member = make_member("Anna", "Schmidt");
        member.salutation = Some(Salutation::Frau);
        member.title = Some(Arc::from("Prof."));
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_FORMAL, &ctx).unwrap();
        assert_eq!(result, "Sehr geehrte Frau Prof. Schmidt,");
    }

    #[test]
    fn test_formal_template_no_salutation_with_title() {
        let mut member = make_member("Alex", "Weber");
        member.salutation = None;
        member.title = Some(Arc::from("Dr."));
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_FORMAL, &ctx).unwrap();
        assert_eq!(result, "Sehr geehrtes Mitglied Dr. Weber,");
    }

    #[test]
    fn test_informal_template_herr_with_title() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_INFORMAL, &ctx).unwrap();
        assert_eq!(result, "Lieber Dr. Max,");
    }

    #[test]
    fn test_informal_template_frau_without_title() {
        let mut member = make_member("Erika", "Muster");
        member.salutation = Some(Salutation::Frau);
        member.title = None;
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_INFORMAL, &ctx).unwrap();
        assert_eq!(result, "Liebe Erika,");
    }

    #[test]
    fn test_informal_template_no_salutation() {
        let mut member = make_member("Simon", "Goller");
        member.salutation = None;
        member.title = None;
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_INFORMAL, &ctx).unwrap();
        assert_eq!(result, "Hallo Simon,");
    }

    #[test]
    fn test_informal_template_frau_with_title() {
        let mut member = make_member("Anna", "Schmidt");
        member.salutation = Some(Salutation::Frau);
        member.title = Some(Arc::from("Prof."));
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_INFORMAL, &ctx).unwrap();
        assert_eq!(result, "Liebe Prof. Anna,");
    }

    #[test]
    fn test_render_footer_with_sender_name() {
        let result =
            render_footer("Mit freundlichen Grüßen\n{{ sender_name }}", "Anna Schmidt").unwrap();
        assert_eq!(result, "Mit freundlichen Grüßen\nAnna Schmidt");
    }

    #[test]
    fn test_render_footer_empty_sender_name() {
        let result = render_footer("Mit freundlichen Grüßen\n{{ sender_name }}", "").unwrap();
        assert_eq!(result, "Mit freundlichen Grüßen\n");
    }

    #[test]
    fn test_render_footer_no_variables() {
        let result = render_footer("Mein Verein e.G.", "Anna Schmidt").unwrap();
        assert_eq!(result, "Mein Verein e.G.");
    }

    #[test]
    fn test_render_footer_invalid_template() {
        let result = render_footer("{{ unclosed", "Anna");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("syntax error"));
    }

    #[test]
    fn test_render_footer_multiline() {
        let template = "Mit freundlichen Grüßen\n{{ sender_name }}\nMein Verein e.G.";
        let result = render_footer(template, "Anna Schmidt").unwrap();
        assert_eq!(
            result,
            "Mit freundlichen Grüßen\nAnna Schmidt\nMein Verein e.G."
        );
    }

    #[test]
    fn test_informal_template_no_salutation_with_title() {
        let mut member = make_member("Alex", "Weber");
        member.salutation = None;
        member.title = Some(Arc::from("Dr."));
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_INFORMAL, &ctx).unwrap();
        assert_eq!(result, "Hallo Dr. Alex,");
    }

    // ============================================================
    // Phase 10 D-04 / D-05 / D-13: merge_repayment_context tests
    // ============================================================

    #[test]
    fn test_merge_repayment_context_renders_all_four_vars() {
        // Quick 260602-r2i: aus "all_three_vars" wurde "all_four_vars" —
        // share_value ist die 4. Variable.
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        let merged = merge_repayment_context(base, "60,00", 3, "20,00", 2026);
        let template = "Auszahlung: {{ payout_amount }} EUR, Anteile: {{ share_count }}, Pro Anteil: {{ share_value }} EUR, Geschaeftsjahr: {{ fiscal_year }}";
        let result = render_template(template, &merged).unwrap();
        assert_eq!(
            result,
            "Auszahlung: 60,00 EUR, Anteile: 3, Pro Anteil: 20,00 EUR, Geschaeftsjahr: 2026"
        );
    }

    #[test]
    fn test_repayment_variable_missing_with_if_guard_renders_empty() {
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        // NOTE: DO NOT call merge_repayment_context here — simulating D-05 edge-case
        // where member has 0 Open/Contacted entries in the phase.
        //
        // D-13 strict opt-in pattern: under minijinja `UndefinedBehavior::Strict`,
        // `{% if payout_amount %}` on a NOT-present variable still errors (strict
        // does not treat undefined as falsy in boolean context). The idiomatic
        // guard is `{% if payout_amount is defined %}` — template authors MUST
        // use that form to opt into the soft-render path.
        let template =
            "{% if payout_amount is defined %}Auszahlung: {{ payout_amount }} EUR{% endif %}Ende";
        let result = render_template(template, &base).unwrap();
        assert_eq!(result, "Ende");
    }

    #[test]
    fn test_repayment_variable_missing_without_guard_fails_strict() {
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        // No merge_repayment_context call — strict-env should fail when template
        // references payout_amount (D-05/D-15 -> Worker mark_recipient_failed).
        let template = "Auszahlung: {{ payout_amount }} EUR";
        let result = render_template(template, &base);
        assert!(
            result.is_err(),
            "Strict-env must error on undefined payout_amount (D-05/D-15 -> Worker mark_recipient_failed)"
        );
        let err_msg = result.unwrap_err().message;
        assert!(
            err_msg.contains("payout_amount") || err_msg.to_lowercase().contains("undefined"),
            "Error message must reference the missing variable, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_merge_preserves_base_context_fields() {
        let member = make_member("Anna", "Schmidt");
        let base = member_to_template_context(&member);
        let merged = merge_repayment_context(base, "60,00", 3, "20,00", 2026);
        // Both base fields (first_name/last_name) AND new fields are accessible
        let template =
            "Hallo {{ first_name }} {{ last_name }}, Auszahlung: {{ payout_amount }} EUR";
        let result = render_template(template, &merged).unwrap();
        assert!(
            result.contains("Anna Schmidt"),
            "Base fields must be preserved: got {}",
            result
        );
        assert!(
            result.contains("60,00 EUR"),
            "Repayment vars must be added: got {}",
            result
        );
    }

    // ============================================================
    // Phase 10 D-14: validate_template_with_repayment tests
    // ============================================================

    #[test]
    fn test_validate_template_with_repayment_accepts_unguarded_payout_amount() {
        let member = make_member("Max", "Mustermann");
        // UAT-Defekt #5: Phase-10's ursprüngliche "fail-fast"-Logik forderte
        // ein `{% if payout_amount is defined %}`-Guard, sonst Validation-Error.
        // Das passt aber nicht zum tatsächlichen Phase-12-UX (Vorstand schreibt
        // `{{ payout_amount }}` natürlich, weil im Repayment-Bulk-Flow ALLE
        // Empfänger es bekommen). validate_template_with_repayment akzeptiert
        // unguarded Phase-10-Vars im merged-context, der die Vars mit
        // Dummy-Werten liefert.
        let result = validate_template_with_repayment(
            "Subject",
            "Auszahlung: {{ payout_amount }} EUR",
            std::slice::from_ref(&member),
        );
        assert!(
            result.is_ok(),
            "Template mit unguarded payout_amount MUSS durchgehen — merged-context liefert die Variable, got: {:?}",
            result
        );

        // Negative path: a syntax error still fails.
        let result2 = validate_template_with_repayment(
            "Subject",
            "{{ payout_amount } EUR", // syntax error: missing closing brace
            std::slice::from_ref(&member),
        );
        assert!(result2.is_err(), "Syntax errors must still surface as Err");

        // Negative path: typos in member vars still caught.
        let result3 = validate_template_with_repayment(
            "Subject",
            "Hallo {{ vorname }}, Auszahlung {{ payout_amount }}", // typo: vorname (correct: first_name)
            std::slice::from_ref(&member),
        );
        assert!(
            result3.is_err(),
            "Member-Var-Typos müssen weiterhin abgefangen werden, got: {:?}",
            result3
        );
    }

    #[test]
    fn test_validate_template_with_repayment_passes_for_guarded_template() {
        let member = make_member("Max", "Mustermann");
        // Guarded template renders fine for member-only AND merged-repayment contexts.
        let result = validate_template_with_repayment(
            "Subject",
            "{% if payout_amount is defined %}Auszahlung: {{ payout_amount }} EUR{% endif %} Hallo {{ first_name }}",
            std::slice::from_ref(&member),
        );
        assert!(
            result.is_ok(),
            "Guarded template must validate: {:?}",
            result
        );
    }

    // ============================================================
    // Quick 260602-r2i: share_value im MiniJinja-Render-Pfad
    // ============================================================

    #[test]
    fn test_merge_repayment_context_includes_share_value() {
        // Quick 260602-r2i: share_value als 4. Variable im Render-Context.
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        let merged = merge_repayment_context(base, "60,00", 3, "20,00", 2026);
        let template = "Auszahlung: {{ payout_amount }}, Anteile: {{ share_count }}, Pro Anteil: {{ share_value }}, GJ: {{ fiscal_year }}";
        let result = render_template(template, &merged).unwrap();
        assert_eq!(
            result,
            "Auszahlung: 60,00, Anteile: 3, Pro Anteil: 20,00, GJ: 2026"
        );
    }

    #[test]
    fn test_merge_preserves_base_context_fields_with_share_value() {
        // share_value ist additiv — Member-Felder bleiben erhalten.
        let member = make_member("Anna", "Schmidt");
        let base = member_to_template_context(&member);
        let merged = merge_repayment_context(base, "60,00", 3, "20,00", 2026);
        let template = "Hallo {{ first_name }} {{ last_name }}, pro Anteil: {{ share_value }} EUR";
        let result = render_template(template, &merged).unwrap();
        assert!(
            result.contains("Anna Schmidt"),
            "Base fields must be preserved: got {}",
            result
        );
        assert!(
            result.contains("20,00 EUR"),
            "share_value must be injected: got {}",
            result
        );
    }

    #[test]
    fn test_share_value_missing_with_if_guard_renders_empty() {
        // D-13-Strict-Opt-in-Pattern, gespiegelt fuer share_value:
        // Wenn merge_repayment_context NICHT aufgerufen wird, blendet
        // `{% if share_value is defined %}` die Variable sauber aus.
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        let template =
            "{% if share_value is defined %}Pro Anteil: {{ share_value }}{% endif %}Ende";
        let result = render_template(template, &base).unwrap();
        assert_eq!(result, "Ende");
    }

    // ============================================================
    // Quick 260603-b43: masked_bank_account Template-Variable
    // ============================================================

    #[test]
    fn test_masked_bank_account_renders_for_member_with_iban() {
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("IBAN: {{ masked_bank_account }}", &ctx).unwrap();
        // make_member() liefert bank_account = "DE89370400440532013000" (22 chars)
        // → masked: "DE•• •••• •••• •••• ••30 00"
        assert_eq!(
            result,
            "IBAN: DE\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} \
             \u{2022}\u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} \
             \u{2022}\u{2022}30 00"
        );
    }

    #[test]
    fn test_masked_bank_account_none_when_no_iban() {
        let mut member = make_member("Max", "Mustermann");
        member.bank_account = None;
        let ctx = member_to_template_context(&member);
        // Conditional guard greift wie bei jedem anderen Optional-Feld.
        let result = render_template(
            "{% if masked_bank_account %}IBAN: {{ masked_bank_account }}{% else %}-{% endif %}",
            &ctx,
        )
        .unwrap();
        assert_eq!(result, "-");
    }

    #[test]
    fn test_masked_bank_account_preserves_country_code_and_suffix() {
        let mut member = make_member("Anna", "Schmidt");
        member.bank_account = Some(Arc::from("AT611904300234573201"));
        let ctx = member_to_template_context(&member);
        let result = render_template("{{ masked_bank_account }}", &ctx).unwrap();
        assert!(
            result.starts_with("AT"),
            "expected AT prefix, got: {result}"
        );
        assert!(
            result.ends_with("3201"),
            "expected 3201 suffix, got: {result}"
        );
    }

    #[test]
    fn test_bank_account_still_available_unmasked() {
        // Regression: das bestehende `bank_account` Feld bleibt unverändert
        // verfügbar, masked_bank_account ist additiv.
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("{{ bank_account }}", &ctx).unwrap();
        assert_eq!(result, "DE89370400440532013000");
    }

    #[test]
    fn test_share_value_missing_without_guard_fails_strict() {
        // Strict-Env errort fail-fast auf undefined share_value
        // (T-r2i-03 Mitigation: D-14 Validation faengt das im REST-Layer ab).
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        let template = "Pro Anteil: {{ share_value }} EUR";
        let result = render_template(template, &base);
        assert!(
            result.is_err(),
            "Strict-env must error on undefined share_value"
        );
        let err_msg = result.unwrap_err().message;
        assert!(
            err_msg.contains("share_value") || err_msg.to_lowercase().contains("undefined"),
            "Error message must reference the missing variable, got: {}",
            err_msg
        );
    }

    // ============================================================
    // Quick 260603-kon: dummy_repayment_context Sentinel-Werte
    // ============================================================

    /// Quick 260603-kon Lock-Test: Sentinel-Werte sind eingefroren — wenn
    /// jemand die Werte aendert, BRECHEN dieses Test UND die DE+EN
    /// Banner-Texte im Frontend (`MailTemplateTestDummyRepaymentHint`).
    /// Beide muessen synchron nachgezogen werden.
    #[test]
    fn test_dummy_repayment_context_sentinel_values_locked() {
        assert_eq!(dummy_repayment_context(), ("99,99", 99, "99,99", 2099));
    }

    /// Quick 260603-kon: End-to-End-Render-Beweis — die Sentinel-Werte
    /// landen tatsaechlich im gerenderten Output via
    /// `merge_repayment_context`. Frontend kann sich darauf verlassen,
    /// dass `"99,99"` und `"99"` im Preview-Body sichtbar sind.
    #[test]
    fn test_dummy_repayment_context_renders_end_to_end() {
        let member = make_member("Max", "Mustermann");
        let base = member_to_template_context(&member);
        let dummy = dummy_repayment_context();
        let merged = merge_repayment_context(base, dummy.0, dummy.1, dummy.2, dummy.3);
        let template = "{{ payout_amount }} EUR fuer {{ share_count }} Anteile a {{ share_value }} EUR ({{ fiscal_year }})";
        let result = render_template(template, &merged).unwrap();
        assert_eq!(result, "99,99 EUR fuer 99 Anteile a 99,99 EUR (2099)");
    }

    // ============================================================
    // Quick 260603-n3m: template_uses_repayment_vars detection
    // ============================================================

    #[test]
    fn test_template_uses_repayment_vars_pure_member_template() {
        // Nur Member-Vars -> false (Editor-Template ohne Repayment-Bezug)
        assert!(!template_uses_repayment_vars(
            "Hallo {{ first_name }}",
            "Lieber {{ last_name }}, willkommen!"
        ));
    }

    #[test]
    fn test_template_uses_repayment_vars_detects_payout_amount_in_body() {
        assert!(template_uses_repayment_vars(
            "Subject",
            "Auszahlung: {{ payout_amount }} EUR"
        ));
    }

    #[test]
    fn test_template_uses_repayment_vars_detects_share_count() {
        assert!(template_uses_repayment_vars(
            "Subject",
            "{{ share_count }} Anteile"
        ));
    }

    #[test]
    fn test_template_uses_repayment_vars_detects_share_value() {
        assert!(template_uses_repayment_vars(
            "Subject",
            "{{ share_value }} EUR pro Anteil"
        ));
    }

    #[test]
    fn test_template_uses_repayment_vars_detects_fiscal_year() {
        assert!(template_uses_repayment_vars(
            "Subject",
            "Geschaeftsjahr {{ fiscal_year }}"
        ));
    }

    #[test]
    fn test_template_uses_repayment_vars_detects_in_subject() {
        // Subject-only hit muss auch zaehlen (Vorstand setzt z.B.
        // `Auszahlung {{ payout_amount }} EUR` in den Subject-Slot)
        assert!(template_uses_repayment_vars(
            "Auszahlung {{ payout_amount }} EUR",
            "Plain body"
        ));
    }

    #[test]
    fn test_template_uses_repayment_vars_detects_guarded_reference() {
        // {% if ... is defined %}-Guards enthalten den Variablen-Namen
        // weiterhin als Substring -> werden detektiert. Das ist korrekt:
        // ohne Dummy-Merge wuerde der Render trotzdem auf undefined laufen,
        // wenn der else-Zweig ebenfalls die Variable nutzt — sicherer ist,
        // den Dummy-Context auch bei guarded References zu liefern.
        assert!(template_uses_repayment_vars(
            "Subject",
            "{% if payout_amount is defined %}Auszahlung: {{ payout_amount }}{% endif %}"
        ));
    }

    // ============================================================
    // Phase 23 D-04 (HTML-04): html_env autoescape + strict_env
    // regression pin + FMT-01 (D-11) date-format wiring
    // ============================================================

    #[test]
    fn html_env_autoescapes_member_value() {
        // Member value containing HTML reserved chars must be encoded when
        // rendered through the autoescape env — this is the core HTML-04
        // guarantee.
        let mut member = make_member("Max", "Mustermann");
        member.first_name = Arc::from("<script>&Co");
        let ctx = member_to_template_context(&member);
        let result = render_html_template("{{ first_name }}", &ctx).unwrap();
        assert_eq!(result, "&lt;script&gt;&amp;Co");
    }

    #[test]
    fn html_env_preserves_author_markup() {
        // Author-supplied <p>-markup around a benign member value must survive
        // — autoescape only touches the *interpolated value*, not the literal
        // template markup.
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_html_template("<p>Hallo {{ first_name }}</p>", &ctx).unwrap();
        assert_eq!(result, "<p>Hallo Max</p>");
    }

    #[test]
    fn strict_env_does_not_escape_member_value() {
        // Regression pin (Pitfall 3): if a future change accidentally enables
        // autoescape on strict_env, text mails would ship with &amp;/&lt; —
        // this test breaks first.
        let mut member = make_member("Max", "Mustermann");
        member.first_name = Arc::from("<script>&Co");
        let ctx = member_to_template_context(&member);
        let result = render_template("{{ first_name }}", &ctx).unwrap();
        assert_eq!(result, "<script>&Co");
    }

    #[test]
    fn test_date_fields_renders_german_format() {
        // FMT-01 (D-11): join_date + exit_date rendered as DD.MM.YYYY via
        // format_de, wired into the shared context builder so text and HTML
        // bodies stay in sync.
        let mut member = make_member("Max", "Mustermann");
        member.join_date = time::Date::from_calendar_date(2026, time::Month::July, 2).unwrap();
        member.exit_date =
            Some(time::Date::from_calendar_date(2025, time::Month::December, 31).unwrap());
        let ctx = member_to_template_context(&member);
        let result = render_template("{{ join_date }} / {{ exit_date }}", &ctx).unwrap();
        assert_eq!(result, "02.07.2026 / 31.12.2025");
    }
}
