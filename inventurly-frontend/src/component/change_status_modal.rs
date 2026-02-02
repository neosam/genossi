use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use uuid::Uuid;

/// Returns the valid next statuses for a given current status
fn get_valid_next_statuses(current: &str) -> Vec<(&'static str, Key)> {
    match current {
        "draft" => vec![
            ("active", Key::StatusActive),
            ("cancelled", Key::StatusCancelled),
        ],
        "active" => vec![
            ("post_processing", Key::StatusPostProcessing),
            ("cancelled", Key::StatusCancelled),
        ],
        "post_processing" => vec![
            ("completed", Key::StatusCompleted),
            ("cancelled", Key::StatusCancelled),
        ],
        "completed" => vec![
            ("post_processing", Key::StatusPostProcessing),
            ("cancelled", Key::StatusCancelled),
        ],
        "cancelled" => vec![("draft", Key::StatusDraft)],
        _ => vec![],
    }
}

/// Returns the button class for a given status
fn get_status_button_class(status: &str) -> &'static str {
    match status {
        "draft" => "w-full px-4 py-3 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 font-medium transition-colors",
        "active" => "w-full px-4 py-3 bg-green-200 text-green-800 rounded-lg hover:bg-green-300 font-medium transition-colors",
        "post_processing" => "w-full px-4 py-3 bg-yellow-200 text-yellow-800 rounded-lg hover:bg-yellow-300 font-medium transition-colors",
        "completed" => "w-full px-4 py-3 bg-blue-200 text-blue-800 rounded-lg hover:bg-blue-300 font-medium transition-colors",
        "cancelled" => "w-full px-4 py-3 bg-red-200 text-red-800 rounded-lg hover:bg-red-300 font-medium transition-colors",
        _ => "w-full px-4 py-3 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 font-medium transition-colors",
    }
}

#[component]
pub fn ChangeStatusModal(
    inventur_id: Uuid,
    current_status: String,
    on_close: EventHandler<()>,
    on_status_changed: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    let valid_statuses = get_valid_next_statuses(&current_status);

    let handle_status_change = move |new_status: &'static str| {
        let on_status_changed = on_status_changed.clone();
        spawn({
            let new_status = new_status.to_string();
            let mut loading = loading.clone();
            let mut error = error.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();

                match api::change_inventur_status(&config, inventur_id, new_status.clone()).await {
                    Ok(_) => {
                        on_status_changed.call(new_status);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to change status: {}", e)));
                        loading.set(false);
                    }
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            onclick: move |_| on_close.call(()),

            div {
                class: "bg-white rounded-lg shadow-xl p-6 w-full max-w-md",
                onclick: move |e| e.stop_propagation(),

                h2 { class: "text-2xl font-bold mb-4", {i18n.t(Key::ChangeStatus)} }

                // Current status display
                div { class: "mb-4",
                    p { class: "text-sm text-gray-600 mb-2", {i18n.t(Key::InventurStatus)} ":" }
                    div { class: "flex items-center gap-2",
                        span {
                            class: match current_status.as_str() {
                                "draft" => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-200 text-gray-800",
                                "active" => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-200 text-green-800",
                                "post_processing" => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-200 text-yellow-800",
                                "completed" => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-200 text-blue-800",
                                "cancelled" => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-200 text-red-800",
                                _ => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-200 text-gray-800",
                            },
                            {match current_status.as_str() {
                                "draft" => i18n.t(Key::StatusDraft),
                                "active" => i18n.t(Key::StatusActive),
                                "post_processing" => i18n.t(Key::StatusPostProcessing),
                                "completed" => i18n.t(Key::StatusCompleted),
                                "cancelled" => i18n.t(Key::StatusCancelled),
                                _ => current_status.clone().into(),
                            }}
                        }
                    }
                }

                // Error display
                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-4 text-sm",
                        "{err}"
                    }
                }

                // Valid next statuses
                div { class: "space-y-3 mb-6",
                    p { class: "text-sm text-gray-600 mb-2", {i18n.t(Key::ChangeStatusTo)} ":" }
                    for (status, key) in valid_statuses {
                        button {
                            class: get_status_button_class(status),
                            disabled: *loading.read(),
                            onclick: move |_| handle_status_change(status),
                            {i18n.t(key)}
                        }
                    }
                }

                // Cancel button
                div { class: "flex justify-end",
                    button {
                        class: "px-4 py-2 text-gray-600 hover:text-gray-800",
                        onclick: move |_| on_close.call(()),
                        {i18n.t(Key::Cancel)}
                    }
                }
            }
        }
    }
}
