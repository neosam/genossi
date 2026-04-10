use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

const TEMPLATE_FORMAL: &str = r#"Sehr geehrte{% if salutation == "Herr" %}r Herr{% elif salutation == "Frau" %} Frau{% else %}s Mitglied{% endif %}{% if title %} {{ title }}{% endif %} {{ last_name }},



Mit freundlichen Grüßen"#;

const TEMPLATE_INFORMAL: &str = r#"{% if salutation == "Herr" %}Lieber{% elif salutation == "Frau" %}Liebe{% else %}Hallo{% endif %}{% if title %} {{ title }}{% endif %} {{ first_name }},



Viele Grüße"#;

#[component]
pub fn TemplateSelector(on_select: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", "Vorlage" }
            select {
                class: "w-full border rounded px-3 py-2 text-sm",
                onchange: move |e| {
                    let val = e.value();
                    match val.as_str() {
                        "formal" => on_select.call(TEMPLATE_FORMAL.to_string()),
                        "informal" => on_select.call(TEMPLATE_INFORMAL.to_string()),
                        _ => {}
                    }
                },
                option { value: "", {i18n.t(Key::MailTemplateSelect)} }
                option { value: "formal", {i18n.t(Key::MailTemplateFormal)} }
                option { value: "informal", {i18n.t(Key::MailTemplateInformal)} }
            }
        }
    }
}
