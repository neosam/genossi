use dioxus::prelude::*;
use crate::api;
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;

#[derive(Clone, Debug)]
struct UserRow {
    username: String,
    sender_name: String,
    is_admin: bool,
}

#[component]
pub fn Permissions() -> Element {
    let i18n = use_i18n();
    let mut users: Signal<Vec<UserRow>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();

            match api::get_all_users(&config).await {
                Ok(all_users) => {
                    let mut rows = Vec::new();
                    for user in &all_users {
                        let roles = api::get_user_roles(&config, &user.name).await.unwrap_or_default();
                        let is_admin = roles.iter().any(|r| r.name == "admin");

                        let sender_name = api::get_user_preference_admin(&config, &user.name, "sender_name")
                            .await
                            .ok()
                            .flatten()
                            .map(|p| p.value)
                            .unwrap_or_default();

                        rows.push(UserRow {
                            username: user.name.clone(),
                            sender_name,
                            is_admin,
                        });
                    }
                    users.set(rows);
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::Permissions)}
                    }

                    if let Some(err) = error.read().as_ref() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                            "{err}"
                        }
                    }

                    if *loading.read() {
                        div { class: "text-gray-600", "Laden..." }
                    } else {
                        div { class: "bg-white rounded-lg shadow overflow-hidden",
                            table { class: "min-w-full divide-y divide-gray-200",
                                thead { class: "bg-gray-50",
                                    tr {
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            "Username"
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            "Anzeigename"
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            "Admin"
                                        }
                                    }
                                }
                                tbody { class: "bg-white divide-y divide-gray-200",
                                    for (idx, _user) in users.read().iter().enumerate() {
                                        UserRowComponent {
                                            key: "{_user.username}",
                                            idx: idx,
                                            users: users,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn UserRowComponent(idx: usize, mut users: Signal<Vec<UserRow>>) -> Element {
    let user = &users.read()[idx];
    let username = user.username.clone();
    let sender_name = user.sender_name.clone();
    let is_admin = user.is_admin;

    let mut saving_name = use_signal(|| false);
    let mut toggling_admin = use_signal(|| false);

    rsx! {
        tr {
            td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900",
                "{username}"
            }
            td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                div { class: "flex gap-2",
                    input {
                        class: "border rounded px-2 py-1 text-sm flex-1",
                        r#type: "text",
                        value: "{sender_name}",
                        oninput: move |e| {
                            let mut current = users.read().clone();
                            if let Some(row) = current.get_mut(idx) {
                                row.sender_name = e.value();
                            }
                            users.set(current);
                        },
                    }
                    button {
                        class: "bg-blue-500 hover:bg-blue-600 text-white px-3 py-1 rounded text-sm disabled:opacity-50",
                        disabled: *saving_name.read(),
                        onclick: {
                            let username = username.clone();
                            move |_| {
                                let name = users.read()[idx].sender_name.clone();
                                let username = username.clone();
                                spawn(async move {
                                    saving_name.set(true);
                                    let config = CONFIG.read().clone();
                                    if let Err(e) = api::set_user_preference_admin(&config, &username, "sender_name", &name).await {
                                        tracing::error!("Failed to save sender_name: {}", e);
                                    }
                                    saving_name.set(false);
                                });
                            }
                        },
                        if *saving_name.read() { "..." } else { "Speichern" }
                    }
                }
            }
            td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                input {
                    class: "h-4 w-4",
                    r#type: "checkbox",
                    checked: is_admin,
                    disabled: *toggling_admin.read(),
                    onchange: {
                        let username = username.clone();
                        move |e: Event<FormData>| {
                            let checked = e.checked();
                            let username = username.clone();
                            let mut users = users.clone();
                            spawn(async move {
                                toggling_admin.set(true);
                                let config = CONFIG.read().clone();
                                let result = if checked {
                                    api::assign_user_role(&config, &username, "admin").await
                                } else {
                                    api::remove_user_role(&config, &username, "admin").await
                                };
                                match result {
                                    Ok(_) => {
                                        let mut current = users.read().clone();
                                        if let Some(row) = current.iter_mut().find(|r| r.username == username) {
                                            row.is_admin = checked;
                                        }
                                        users.set(current);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to toggle admin: {}", e);
                                    }
                                }
                                toggling_admin.set(false);
                            });
                        }
                    },
                }
            }
        }
    }
}
