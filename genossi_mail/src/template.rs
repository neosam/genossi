use async_trait::async_trait;
use genossi_dao::member::MemberEntity;
use minijinja::{context, Value};
use mockall::automock;
use uuid::Uuid;

use crate::service::MailServiceError;

#[automock]
#[async_trait]
pub trait MemberResolver: Send + Sync + 'static {
    async fn find_member_by_id(&self, id: Uuid) -> Result<Option<MemberEntity>, MailServiceError>;
}

pub fn member_to_template_context(entity: &MemberEntity) -> Value {
    let salutation_str = entity.salutation.as_ref().map(|s| s.as_str().to_string());
    let join_date_str = entity.join_date.to_string();
    let exit_date_str = entity.exit_date.map(|d| d.to_string());
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
        let result =
            render_template("Nr: {{ member_number }}, Anteile: {{ current_shares }}", &ctx)
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
        let members = vec![make_member("Max", "Mustermann"), make_member("Erika", "Muster")];
        let result = validate_template("Hallo {{ first_name }}", "Lieber {{ last_name }}", &members);
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
        let member = make_member("Max", "Mustermann");
        let ctx = member_to_template_context(&member);
        let result = render_template("Beitritt: {{ join_date }}", &ctx).unwrap();
        assert_eq!(result, "Beitritt: 2025-01-15");
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
    fn test_informal_template_no_salutation_with_title() {
        let mut member = make_member("Alex", "Weber");
        member.salutation = None;
        member.title = Some(Arc::from("Dr."));
        let ctx = member_to_template_context(&member);
        let result = render_template(TEMPLATE_INFORMAL, &ctx).unwrap();
        assert_eq!(result, "Hallo Dr. Alex,");
    }
}
