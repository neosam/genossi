use crate::i18n::{use_i18n, Key};
use crate::service::tara::{clear_tara, get_tara_grams, set_tara};
use dioxus::prelude::*;

#[component]
pub fn TaraModal(on_close: EventHandler<()>) -> Element {
    let i18n = use_i18n();

    let current_tara = get_tara_grams();
    let mut weight_input = use_signal(|| {
        if current_tara > 0 {
            (current_tara as f64 / 1000.0).to_string()
        } else {
            String::new()
        }
    });
    let mut use_kg = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    let handle_save = move |_| {
        let input_str = weight_input.read().clone();
        if input_str.trim().is_empty() {
            // Empty input means clear tara
            clear_tara();
            on_close.call(());
            return;
        }

        match input_str.parse::<f64>() {
            Ok(value) => {
                if value < 0.0 {
                    error.set(Some("Weight cannot be negative".to_string()));
                    return;
                }

                let grams = if *use_kg.read() {
                    (value * 1000.0) as i64
                } else {
                    value as i64
                };

                set_tara(grams);
                on_close.call(());
            }
            Err(_) => {
                error.set(Some("Invalid number".to_string()));
            }
        }
    };

    let handle_clear = move |_| {
        clear_tara();
        on_close.call(());
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            onclick: move |_| on_close.call(()),

            div {
                class: "bg-white rounded-lg shadow-xl p-6 w-full max-w-md",
                onclick: move |e| e.stop_propagation(),

                h2 { class: "text-2xl font-bold mb-4", {i18n.t(Key::CustomTara)} }

                p { class: "text-gray-600 mb-4 text-sm",
                    {i18n.t(Key::TaraDescription)}
                }

                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-4 text-sm",
                        "{err}"
                    }
                }

                div { class: "space-y-4",
                    // Unit selector
                    div { class: "flex gap-2 mb-2",
                        button {
                            class: if *use_kg.read() {
                                "px-3 py-1 bg-blue-600 text-white rounded"
                            } else {
                                "px-3 py-1 bg-gray-200 text-gray-700 rounded hover:bg-gray-300"
                            },
                            onclick: move |_| use_kg.set(true),
                            "kg"
                        }
                        button {
                            class: if !*use_kg.read() {
                                "px-3 py-1 bg-blue-600 text-white rounded"
                            } else {
                                "px-3 py-1 bg-gray-200 text-gray-700 rounded hover:bg-gray-300"
                            },
                            onclick: move |_| use_kg.set(false),
                            "g"
                        }
                    }

                    // Weight input
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::TaraWeight)}
                        }
                        input {
                            r#type: "number",
                            step: if *use_kg.read() { "0.1" } else { "1" },
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 text-lg",
                            placeholder: if *use_kg.read() { "e.g., 70.5" } else { "e.g., 70500" },
                            value: "{weight_input.read()}",
                            oninput: move |e| {
                                weight_input.set(e.value());
                                error.set(None);
                            },
                            autofocus: true,
                        }
                        p { class: "text-xs text-gray-500 mt-1",
                            {i18n.t(Key::TaraHint)}
                        }
                    }

                    // Current tara display
                    if current_tara > 0 {
                        div { class: "bg-gray-100 p-3 rounded text-sm",
                            span { class: "font-medium", {i18n.t(Key::CurrentTara)} }
                            span { class: "ml-2",
                                "{current_tara / 1000} kg ({current_tara} g)"
                            }
                        }
                    }
                }

                // Buttons
                div { class: "flex justify-between mt-6",
                    button {
                        class: "px-4 py-2 text-red-600 hover:text-red-800",
                        onclick: handle_clear,
                        {i18n.t(Key::ClearTara)}
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 text-gray-600 hover:text-gray-800",
                            onclick: move |_| on_close.call(()),
                            {i18n.t(Key::Cancel)}
                        }
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                            onclick: handle_save,
                            {i18n.t(Key::Save)}
                        }
                    }
                }
            }
        }
    }
}
