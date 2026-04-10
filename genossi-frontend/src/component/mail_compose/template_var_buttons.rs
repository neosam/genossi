use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

const PRIMARY_VARS: &[(&str, &str)] = &[
    ("first_name", "Vorname"),
    ("last_name", "Nachname"),
    ("salutation", "Anrede"),
    ("title", "Titel"),
    ("member_number", "Nr."),
    ("company", "Firma"),
];

const SECONDARY_VARS: &[(&str, &str)] = &[
    ("street", "Straße"),
    ("house_number", "Hausnr."),
    ("postal_code", "PLZ"),
    ("city", "Stadt"),
    ("join_date", "Beitrittsdatum"),
    ("shares_at_joining", "Anteile (Beitritt)"),
    ("current_shares", "Anteile (aktuell)"),
    ("current_balance", "Guthaben"),
    ("exit_date", "Austrittsdatum"),
    ("bank_account", "Bankverbindung"),
    ("email", "E-Mail"),
];

#[component]
pub fn TemplateVarButtons(on_insert: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let mut show_more = use_signal(|| false);

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-3",
            label { class: "block text-xs font-medium text-gray-500 mb-2",
                {i18n.t(Key::MailTemplateVariables)}
            }
            div { class: "flex flex-wrap gap-1",
                for (var_name, label) in PRIMARY_VARS.iter() {
                    {
                        let vn = var_name.to_string();
                        let lbl = label.to_string();
                        rsx! {
                            button {
                                class: "bg-blue-100 hover:bg-blue-200 text-blue-800 px-2 py-1 rounded text-xs font-mono",
                                r#type: "button",
                                title: "{var_name}",
                                onclick: move |_| {
                                    on_insert.call(format!("{{{{ {} }}}}", vn));
                                },
                                "{lbl}"
                            }
                        }
                    }
                }
                if *show_more.read() {
                    for (var_name, label) in SECONDARY_VARS.iter() {
                        {
                            let vn = var_name.to_string();
                            let lbl = label.to_string();
                            rsx! {
                                button {
                                    class: "bg-gray-100 hover:bg-gray-200 text-gray-700 px-2 py-1 rounded text-xs font-mono",
                                    r#type: "button",
                                    onclick: move |_| {
                                        on_insert.call(format!("{{{{ {} }}}}", vn));
                                    },
                                    "{lbl}"
                                }
                            }
                        }
                    }
                }
                button {
                    class: "text-gray-500 hover:text-gray-700 px-2 py-1 text-xs underline",
                    r#type: "button",
                    onclick: move |_| {
                        let current = *show_more.read();
                        show_more.set(!current);
                    },
                    if *show_more.read() {
                        "Weniger"
                    } else {
                        {i18n.t(Key::MailTemplateMore)}
                    }
                }
            }
        }
    }
}
