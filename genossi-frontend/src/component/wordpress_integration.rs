use dioxus::prelude::*;

use crate::api::{self, ConfigEntryTO};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

fn is_field_set(entries: &[ConfigEntryTO], key: &str) -> bool {
    entries.iter().any(|e| e.key == key)
}

const REQUIRED_FIELDS: &[(&str, fn() -> Key)] = &[
    ("public_api_key", || Key::GenerateApiKey),
    ("share_value_cents", || Key::ShareValueCents),
    ("bank_iban", || Key::BankIban),
    ("bank_name", || Key::BankNameConfig),
    ("genossenschaft_name", || Key::GenossenschaftName),
];

#[component]
pub fn WordPressIntegrationSection(
    entries: Signal<Vec<ConfigEntryTO>>,
    share_value_cents: Signal<String>,
    bank_iban: Signal<String>,
    bank_name: Signal<String>,
    bank_bic: Signal<String>,
    genossenschaft_name: Signal<String>,
    error: Signal<Option<String>>,
    success_msg: Signal<Option<String>>,
    on_reload: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut generating = use_signal(|| false);
    let mut generated_key = use_signal(|| None::<String>);
    let mut copied = use_signal(|| false);
    let mut saving = use_signal(|| false);

    let api_key_configured = is_field_set(&entries.read(), "public_api_key");

    let missing: Vec<String> = REQUIRED_FIELDS
        .iter()
        .filter(|(key, _)| !is_field_set(&entries.read(), key))
        .map(|(_, key_fn)| i18n.t(key_fn()).to_string())
        .collect();
    let all_configured = missing.is_empty();

    let backend_url = CONFIG.read().backend.to_string();

    rsx! {
        div {
            p { class: "text-sm text-gray-500 mb-4", {i18n.t(Key::WordPressIntegrationDesc)} }

            // Status indicator
            div { class: if all_configured { "mb-4 p-3 rounded-lg bg-green-50 border border-green-200" } else { "mb-4 p-3 rounded-lg bg-yellow-50 border border-yellow-200" },
                if all_configured {
                    p { class: "text-green-700 font-medium text-sm",
                        "✓ " {i18n.t(Key::ConfigComplete)}
                    }
                } else {
                    div {
                        p { class: "text-yellow-700 font-medium text-sm",
                            {i18n.t(Key::ConfigIncomplete)}
                        }
                        p { class: "text-yellow-600 text-sm mt-1",
                            {i18n.t(Key::MissingFields)} ": " {missing.join(", ")}
                        }
                    }
                }
            }

            div { class: "space-y-6",
                // API Key Section
                div {
                    h3 { class: "text-lg font-medium mb-2", "API-Key" }
                    div { class: "flex items-center space-x-3",
                        button {
                            class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                            disabled: *generating.read(),
                            onclick: {
                                let i18n = i18n.clone();
                                move |_| {
                                    let i18n = i18n.clone();
                                    spawn(async move {
                                        generating.set(true);
                                        copied.set(false);
                                        let config = CONFIG.read().clone();
                                        match api::generate_api_key(&config).await {
                                            Ok(key) => {
                                                generated_key.set(Some(key));
                                                success_msg.set(Some(i18n.t(Key::ApiKeyGenerated).to_string()));
                                                on_reload.call(());
                                            }
                                            Err(e) => {
                                                error.set(Some(format!("{}", e)));
                                            }
                                        }
                                        generating.set(false);
                                    });
                                }
                            },
                            if *generating.read() {
                                {i18n.t(Key::Generating)}
                            } else if api_key_configured {
                                {i18n.t(Key::RegenerateApiKey)}
                            } else {
                                {i18n.t(Key::GenerateApiKey)}
                            }
                        }
                        if api_key_configured && generated_key.read().is_none() {
                            span { class: "text-green-600 text-sm font-medium",
                                "✓ " {i18n.t(Key::ApiKeyConfigured)}
                            }
                        }
                    }

                    // Show generated key
                    if let Some(key) = generated_key.read().as_ref() {
                        div { class: "mt-3 p-3 bg-blue-50 border border-blue-200 rounded-lg",
                            p { class: "text-sm font-medium text-blue-800 mb-2",
                                {i18n.t(Key::ApiKeyCopyHint)}
                            }
                            div { class: "flex items-center space-x-2",
                                input {
                                    class: "flex-1 font-mono text-sm bg-white border rounded px-3 py-2",
                                    r#type: "text",
                                    readonly: true,
                                    value: "{key}",
                                }
                                button {
                                    class: "bg-blue-500 hover:bg-blue-600 text-white px-3 py-2 rounded text-sm",
                                    onclick: {
                                        let key = key.clone();
                                        move |_| {
                                            let key = key.clone();
                                            spawn(async move {
                                                if let Some(window) = web_sys::window() {
                                                    if let Ok(clipboard) = js_sys::Reflect::get(
                                                        &window.navigator(),
                                                        &"clipboard".into(),
                                                    ) {
                                                        let promise = js_sys::Reflect::apply(
                                                            &js_sys::Reflect::get(&clipboard, &"writeText".into())
                                                                .unwrap()
                                                                .into(),
                                                            &clipboard,
                                                            &js_sys::Array::of1(&key.into()),
                                                        );
                                                        if let Ok(promise) = promise {
                                                            let _ = wasm_bindgen_futures::JsFuture::from(
                                                                js_sys::Promise::from(promise),
                                                            )
                                                            .await;
                                                            copied.set(true);
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    if *copied.read() {
                                        {i18n.t(Key::Copied)}
                                    } else {
                                        {i18n.t(Key::CopyToClipboard)}
                                    }
                                }
                            }
                        }
                    }
                }

                // Cooperative Settings
                div {
                    h3 { class: "text-lg font-medium mb-2", {i18n.t(Key::CooperativeSettings)} }
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::GenossenschaftName)}
                            }
                            input {
                                class: "w-full border rounded px-3 py-2",
                                r#type: "text",
                                placeholder: "Muster eG",
                                value: "{genossenschaft_name}",
                                oninput: move |e| genossenschaft_name.set(e.value()),
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::ShareValueCents)}
                            }
                            input {
                                class: "w-full border rounded px-3 py-2",
                                r#type: "number",
                                placeholder: "5000",
                                value: "{share_value_cents}",
                                oninput: move |e| share_value_cents.set(e.value()),
                            }
                        }
                    }
                }

                // Bank Details
                div {
                    h3 { class: "text-lg font-medium mb-2", {i18n.t(Key::BankDetails)} }
                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::BankIban)}
                            }
                            input {
                                class: "w-full border rounded px-3 py-2",
                                r#type: "text",
                                placeholder: "DE89 3704 0044 ...",
                                value: "{bank_iban}",
                                oninput: move |e| bank_iban.set(e.value()),
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::BankNameConfig)}
                            }
                            input {
                                class: "w-full border rounded px-3 py-2",
                                r#type: "text",
                                placeholder: "GLS Bank",
                                value: "{bank_name}",
                                oninput: move |e| bank_name.set(e.value()),
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::BankBic)}
                            }
                            input {
                                class: "w-full border rounded px-3 py-2",
                                r#type: "text",
                                placeholder: "GENODEM1GLS",
                                value: "{bank_bic}",
                                oninput: move |e| bank_bic.set(e.value()),
                            }
                        }
                    }
                }

                // Save button
                div { class: "flex items-center space-x-4",
                    button {
                        class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                        disabled: *saving.read(),
                        onclick: {
                            let i18n = i18n.clone();
                            move |_| {
                                let i18n = i18n.clone();
                                let share_val = share_value_cents.read().clone();
                                let iban = bank_iban.read().clone();
                                let bname = bank_name.read().clone();
                                let bic = bank_bic.read().clone();
                                let geno = genossenschaft_name.read().clone();
                                spawn(async move {
                                    saving.set(true);
                                    error.set(None);
                                    success_msg.set(None);
                                    let config = CONFIG.read().clone();
                                    let mut all_ok = true;

                                    let mut entries_to_save: Vec<(&str, String, &str)> = vec![
                                        ("share_value_cents", share_val, "int"),
                                        ("bank_iban", iban, "string"),
                                        ("bank_name", bname, "string"),
                                        ("genossenschaft_name", geno, "string"),
                                    ];
                                    if !bic.is_empty() {
                                        entries_to_save.push(("bank_bic", bic, "string"));
                                    }

                                    for (key, value, vtype) in &entries_to_save {
                                        if let Err(e) = api::set_config_entry(&config, key, value, vtype).await {
                                            error.set(Some(format!("{}", e)));
                                            all_ok = false;
                                            break;
                                        }
                                    }

                                    if all_ok {
                                        success_msg.set(Some(i18n.t(Key::Save).to_string()));
                                        on_reload.call(());
                                    }
                                    saving.set(false);
                                });
                            }
                        },
                        if *saving.read() {
                            {i18n.t(Key::Generating)}
                        } else {
                            {i18n.t(Key::Save)}
                        }
                    }
                }

                // Setup Instructions
                div { class: "mt-2 p-4 bg-gray-50 border border-gray-200 rounded-lg",
                    h3 { class: "text-lg font-medium mb-3", {i18n.t(Key::SetupInstructions)} }
                    div { class: "space-y-3 text-sm text-gray-700",
                        p { {i18n.t(Key::WpStep1)} }
                        div {
                            p { class: "font-medium", {i18n.t(Key::WpStep2)} }
                            div { class: "ml-4 mt-1 space-y-1",
                                div { class: "flex items-center space-x-2",
                                    span { class: "text-gray-500", {i18n.t(Key::ApiUrl)} ":" }
                                    code { class: "bg-white border rounded px-2 py-1 font-mono text-xs",
                                        "{backend_url}"
                                    }
                                }
                                div {
                                    span { class: "text-gray-500", "API-Key: " }
                                    span { class: "text-gray-500 italic", "(siehe oben)" }
                                }
                            }
                        }
                        div {
                            p { class: "font-medium", {i18n.t(Key::WpStep3)} }
                            code { class: "ml-4 mt-1 block bg-white border rounded px-2 py-1 font-mono text-xs",
                                "[genossi_beitritt]"
                            }
                        }
                    }
                }
            }
        }
    }
}
