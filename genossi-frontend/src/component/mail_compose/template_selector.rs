use dioxus::prelude::*;

use crate::api::{self, MailTemplateTO};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;

#[component]
pub fn TemplateSelector(on_select: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let mut templates = use_signal(Vec::<MailTemplateTO>::new);

    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(data) = api::list_mail_templates(&config).await {
                templates.set(data);
            }
        });
    });

    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", "Vorlage" }
            select {
                class: "w-full border rounded px-3 py-2 text-sm",
                onchange: move |e| {
                    let val = e.value();
                    if !val.is_empty() {
                        if let Some(tpl) = templates.read().iter().find(|t| t.id == val) {
                            on_select.call(tpl.body.clone());
                        }
                    }
                },
                option { value: "", {i18n.t(Key::MailTemplateSelect)} }
                for tpl in templates.read().iter() {
                    option {
                        value: "{tpl.id}",
                        "{tpl.name}"
                    }
                }
            }
            div { class: "mt-1",
                Link {
                    to: Route::MailTemplatesPage {},
                    class: "text-sm text-blue-600 hover:underline",
                    {i18n.t(Key::MailTemplateManage)}
                }
            }
        }
    }
}
