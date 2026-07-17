use dioxus::prelude::*;

use crate::{
    api::{self, AppError},
    component::{ErrorAlert, Modal},
    i18n::{use_i18n, Key},
    service::config::CONFIG,
};

#[component]
pub fn RevokeSessionsButton() -> Element {
    let i18n = use_i18n();
    let mut show_confirm = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut error: Signal<Option<AppError>> = use_signal(|| None);

    let confirm_revoke = move |_| {
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            match api::revoke_all_sessions(&config).await {
                Ok(_) => {
                    // Redirect to backend logout to clear browser state
                    if let Some(window) = web_sys::window() {
                        let _ = window
                            .location()
                            .set_href(&format!("{}/logout", config.backend));
                    }
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        li {
            button {
                class: "hover:underline px-3 py-2 md:py-4 text-left",
                onclick: move |_| {
                    show_confirm.set(true);
                    error.set(None);
                },
                {i18n.t(Key::RevokeAllSessions)}
            }
        }

        if *show_confirm.read() {
            Modal {
                div { class: "space-y-4",
                    h2 { class: "text-xl font-bold text-red-600",
                        {i18n.t(Key::RevokeSessionsConfirmTitle)}
                    }
                    p { {i18n.t(Key::RevokeSessionsConfirmText)} }

                    if let Some(err) = error.read().clone() {
                        ErrorAlert {
                            error: err,
                            on_dismiss: move |_| error.set(None),
                        }
                    }

                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50",
                            disabled: *loading.read(),
                            onclick: confirm_revoke,
                            if *loading.read() {
                                "..."
                            } else {
                                {i18n.t(Key::Confirm)}
                            }
                        }
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded hover:bg-gray-400",
                            disabled: *loading.read(),
                            onclick: move |_| show_confirm.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }
    }
}
