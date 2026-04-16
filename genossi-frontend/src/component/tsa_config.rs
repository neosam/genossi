use dioxus::prelude::*;

use crate::api::{self, ConfigEntryTO};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

fn get_value(entries: &[ConfigEntryTO], key: &str) -> String {
    entries
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.clone())
        .unwrap_or_default()
}

#[component]
pub fn TsaConfigSection(
    entries: Signal<Vec<ConfigEntryTO>>,
    error: Signal<Option<String>>,
    success_msg: Signal<Option<String>>,
    on_reload: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut saving = use_signal(|| false);

    let mut tsa_enabled = use_signal(|| get_value(&entries.read(), "tsa_enabled") == "true");
    let mut tsa_url = use_signal(|| get_value(&entries.read(), "tsa_url"));
    let mut tsa_user = use_signal(|| get_value(&entries.read(), "tsa_user"));
    let mut tsa_pass = use_signal(|| get_value(&entries.read(), "tsa_pass"));
    let mut tsa_interval = use_signal(|| {
        let v = get_value(&entries.read(), "tsa_interval_hours");
        if v.is_empty() {
            "168".to_string()
        } else {
            v
        }
    });

    let on_save = {
        let mut error = error.clone();
        let mut success_msg = success_msg.clone();
        let on_reload = on_reload.clone();
        let i18n = i18n.clone();
        move |_| {
            let enabled = *tsa_enabled.read();
            let url = tsa_url.read().clone();
            let user = tsa_user.read().clone();
            let pass = tsa_pass.read().clone();
            let interval = tsa_interval.read().clone();
            let on_reload = on_reload.clone();
            let i18n = i18n.clone();
            spawn(async move {
                saving.set(true);
                let config = CONFIG.read().clone();
                let mut had_error = false;

                let settings = vec![
                    (
                        "tsa_enabled",
                        if enabled { "true" } else { "false" },
                        "bool",
                    ),
                    ("tsa_url", &url, "string"),
                    ("tsa_user", &user, "string"),
                    ("tsa_pass", &pass, "secret"),
                    ("tsa_interval_hours", &interval, "int"),
                ];

                for (key, value, value_type) in settings {
                    if let Err(e) = api::set_config_entry(&config, key, value, value_type).await {
                        error.set(Some(format!("Error saving {}: {}", key, e)));
                        had_error = true;
                        break;
                    }
                }

                if !had_error {
                    success_msg.set(Some(i18n.t(Key::Save).to_string()));
                    on_reload.call(());
                }
                saving.set(false);
            });
        }
    };

    rsx! {
        div {
            p { class: "text-sm text-gray-500 mb-4",
                "RFC 3161 Qualified Timestamping (eIDAS)"
            }

            div { class: "space-y-4",
                // Enabled toggle
                div { class: "flex items-center gap-3",
                    input {
                        r#type: "checkbox",
                        class: "h-4 w-4",
                        checked: *tsa_enabled.read(),
                        onchange: move |e: Event<FormData>| tsa_enabled.set(e.value() == "true"),
                    }
                    label { class: "text-sm font-medium", {i18n.t(Key::TimestampTsaEnabled)} }
                }

                // TSA URL
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::TimestampTsaUrl)}
                    }
                    input {
                        r#type: "text",
                        class: "w-full border rounded px-3 py-2",
                        placeholder: "https://freetsa.org/tsr",
                        value: "{tsa_url}",
                        oninput: move |e| tsa_url.set(e.value()),
                    }
                }

                // Username
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::TimestampTsaUser)}
                    }
                    input {
                        r#type: "text",
                        class: "w-full border rounded px-3 py-2",
                        value: "{tsa_user}",
                        oninput: move |e| tsa_user.set(e.value()),
                    }
                }

                // Password
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::TimestampTsaPass)}
                    }
                    input {
                        r#type: "password",
                        class: "w-full border rounded px-3 py-2",
                        value: "{tsa_pass}",
                        oninput: move |e| tsa_pass.set(e.value()),
                    }
                }

                // Interval
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::TimestampTsaInterval)}
                    }
                    input {
                        r#type: "number",
                        class: "w-full border rounded px-3 py-2",
                        value: "{tsa_interval}",
                        min: "1",
                        oninput: move |e| tsa_interval.set(e.value()),
                    }
                    p { class: "text-xs text-gray-500 mt-1",
                        "168 = 7 Tage (5 Gratis/Monat bei DGN)"
                    }
                }

                // Save button
                button {
                    class: "bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 disabled:opacity-50",
                    disabled: *saving.read(),
                    onclick: on_save,
                    {i18n.t(Key::Save)}
                }
            }
        }
    }
}
