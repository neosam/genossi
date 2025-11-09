use dioxus::prelude::*;

use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::auth::load_auth_info;
use crate::service::config::CONFIG;

#[component]
pub fn InventurTokenLogin(token: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    let mut name = use_signal(|| String::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Clone values that will be moved into closure
    let token_clone = token.clone();
    let i18n_error = i18n.clone();

    let handle_login = move || {
        spawn({
            let mut loading = loading.clone();
            let mut error = error.clone();
            let name_value = name.read().clone();
            let token_value = token_clone.clone();
            let nav = nav.clone();
            let i18n = i18n_error.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();

                match api::login_with_inventur_token(config.backend.clone(), &name_value, &token_value).await {
                    Ok(()) => {
                        // Reload authentication info to get user data
                        load_auth_info().await;

                        // Navigate to home page
                        nav.push(Route::Home {});
                    }
                    Err(e) => {
                        error.set(Some(format!("{}: {}", i18n.t(Key::InventurTokenLoginFailed), e)));
                        loading.set(false);
                    }
                }
            }
        });
    };

    rsx! {
        div { class: "flex items-center justify-center min-h-screen bg-gray-100",
            div { class: "w-full max-w-md p-8 space-y-6 bg-white rounded-lg shadow-md",
                h1 { class: "text-3xl font-bold text-center text-gray-900",
                    {i18n.t(Key::InventurTokenLoginTitle)}
                }

                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                        {err.clone()}
                    }
                }

                div { class: "space-y-4",
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "name",
                            {i18n.t(Key::EnterYourName)}
                        }
                        input {
                            id: "name",
                            r#type: "text",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            placeholder: "{i18n.t(Key::NamePlaceholder)}",
                            value: "{name.read()}",
                            oninput: move |e| name.set(e.value()),
                            disabled: *loading.read(),
                            autofocus: true,
                            onkeydown: {
                                let login = handle_login.clone();
                                move |evt: Event<KeyboardData>| {
                                    if evt.key().to_string() == "Enter" && !name.read().trim().is_empty() && !*loading.read() {
                                        login();
                                    }
                                }
                            },
                        }
                    }

                    button {
                        r#type: "button",
                        class: "w-full px-4 py-2 text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: *loading.read() || name.read().trim().is_empty(),
                        onclick: {
                            let login = handle_login.clone();
                            move |_| login()
                        },
                        if *loading.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::Login)}
                        }
                    }
                }

                // Debug info (remove in production)
                if cfg!(debug_assertions) {
                    div { class: "mt-4 text-xs text-gray-500",
                        p { "Token: {token.chars().take(10).collect::<String>()}..." }
                    }
                }
            }
        }
    }
}
