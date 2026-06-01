use dioxus::prelude::*;

use crate::api::{self, MailTemplateTO};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;

/// Mail-Template-Selector.
///
/// Phase 12 D-18 / Issue #2 BLOCKER-Fix: `on_select_id` is an additional
/// EventHandler that delivers the selected template's ID (or `None` on reset)
/// alongside the existing `on_select(body)` callback. Backward-compatible:
/// callers that don't need the ID can pass a no-op closure (or use the
/// `#[props(default)]` default).
#[component]
pub fn TemplateSelector(
    on_select: EventHandler<String>,
    #[props(default)] on_select_id: EventHandler<Option<String>>,
) -> Element {
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
                    if val.is_empty() {
                        // "Vorlage waehlen..."-Option (empty value): Reset selection
                        on_select_id.call(None);
                    } else if let Some(tpl) = templates.read().iter().find(|t| t.id == val) {
                        on_select.call(tpl.body.clone());
                        // Phase 12 D-18 / Issue #2 BLOCKER-Fix: deliver template ID
                        // to caller so send_bulk_mail can populate `template_id`.
                        on_select_id.call(Some(tpl.id.clone()));
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
