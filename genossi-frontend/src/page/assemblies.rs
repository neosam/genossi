//! Assemblies list page (Phase 4 Plan 08) — admin-only.
//!
//! Pattern aus applications_page.rs:48-150 (Liste + Create-Modal); replaces inline-rsx
//! list-rendering with <AssemblyListRow> (Component-First).

use dioxus::prelude::*;

use crate::api::{self, AssemblyTO, CreateAssemblyRequest};
use crate::auth::RequirePrivilege;
use crate::component::{show_toast, AssemblyListRow, Modal, ToastContainer, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::access_denied::AccessDeniedPage;
use crate::service::config::CONFIG;

#[component]
pub fn Assemblies() -> Element {
    let i18n = use_i18n();

    let mut assemblies = use_signal(Vec::<AssemblyTO>::new);
    let mut loading = use_signal(|| true);
    let mut show_create = use_signal(|| false);
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);

    let load = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_assemblies(&config).await {
                Ok(list) => assemblies.set(list),
                Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            TopBar {}
            div { class: "container mx-auto px-4 py-6",
                div { class: "flex justify-between items-start mb-4",
                    h1 { class: "text-2xl font-bold mb-1", "{i18n.t(Key::Assemblies)}" }
                    button {
                        r#type: "button",
                        class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm min-h-[44px]",
                        onclick: move |_| show_create.set(true),
                        "{i18n.t(Key::AssemblyCreate)}"
                    }
                }

                if *loading.read() {
                    p { class: "text-gray-500 text-center py-8", "{i18n.t(Key::Loading)}" }
                } else if assemblies.read().is_empty() {
                    div { class: "text-center py-12",
                        p { class: "text-lg font-medium text-gray-700", "{i18n.t(Key::AssemblyEmpty)}" }
                        p { class: "text-sm text-gray-500 mt-2 mb-6", "{i18n.t(Key::AssemblyEmptyHint)}" }
                        button {
                            r#type: "button",
                            class: "bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded min-h-[44px]",
                            onclick: move |_| show_create.set(true),
                            "{i18n.t(Key::AssemblyCreate)}"
                        }
                    }
                } else {
                    div { class: "flex flex-col",
                        for a in assemblies.read().iter() {
                            AssemblyListRow { key: "{a.id}", assembly: a.clone() }
                        }
                    }
                }

                if *show_create.read() {
                    Modal {
                        CreateAssemblyForm {
                            on_close: move |_| show_create.set(false),
                            on_created: move |_| {
                                show_create.set(false);
                                load();
                            },
                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                        }
                    }
                }
            }
            ToastContainer { messages: toast_messages }
        }
    }
}

#[component]
fn CreateAssemblyForm(
    on_close: EventHandler<()>,
    on_created: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut name = use_signal(String::new);
    let mut date_str = use_signal(String::new);
    let mut location = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    // Pre-resolve the i18n string used inside the closure (i18n is not Copy).
    let name_required_msg = i18n.t(Key::AssemblyName).to_string();

    rsx! {
        form {
            class: "flex flex-col gap-4",
            onsubmit: move |e| {
                e.prevent_default();
                if name.read().trim().is_empty() {
                    on_error.call(name_required_msg.clone());
                    return;
                }
                submitting.set(true);
                let req = CreateAssemblyRequest {
                    name: name.read().trim().to_string(),
                    date: {
                        let s = date_str.read().trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    },
                    location: {
                        let s = location.read().trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    },
                };
                spawn(async move {
                    let config = CONFIG.read().clone();
                    match api::create_assembly(&config, &req).await {
                        Ok(_) => on_created.call(()),
                        Err(e) => on_error.call(e.message),
                    }
                    submitting.set(false);
                });
            },
            h2 { class: "text-xl font-semibold", "{i18n.t(Key::AssemblyCreate)}" }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::AssemblyName)}" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                }
            }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::AssemblyDate)}" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "datetime-local",
                    value: "{date_str}",
                    oninput: move |e| date_str.set(e.value()),
                }
            }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::AssemblyLocation)}" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "text",
                    value: "{location}",
                    oninput: move |e| location.set(e.value()),
                }
            }
            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                    onclick: move |_| on_close.call(()),
                    "{i18n.t(Key::Cancel)}"
                }
                button {
                    r#type: "submit",
                    class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded disabled:opacity-50 min-h-[44px]",
                    disabled: *submitting.read(),
                    "{i18n.t(Key::Save)}"
                }
            }
        }
    }
}
