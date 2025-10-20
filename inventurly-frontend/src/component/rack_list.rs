use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::rack::RACKS;
use dioxus::prelude::*;

#[component]
pub fn RackList() -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let racks = RACKS.read();

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b flex justify-between items-center",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::Racks)}
                }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                    onclick: move |_| { nav.push(Route::RackDetails { id: "new".to_string() }); },
                    {i18n.t(Key::Create)}
                }
            }

            if racks.items.is_empty() {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::RackName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::RackDescription)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::RackCreated)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for rack in racks.items.iter() {
                                {
                                    let rack_id = rack.id;
                                    rsx! {
                                        tr {
                                            class: "border-b hover:bg-gray-50 cursor-pointer",
                                            onclick: move |_| {
                                                if let Some(id) = rack_id {
                                                    nav.push(Route::RackDetails { id: id.to_string() });
                                                }
                                            },
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                {rack.name.clone()}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                {rack.description.clone()}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                if let Some(created) = rack.created {
                                                    {i18n.format_datetime(created)}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                button {
                                                    class: "text-blue-600 hover:text-blue-800",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        if let Some(id) = rack_id {
                                                            nav.push(Route::RackDetails { id: id.to_string() });
                                                        }
                                                    },
                                                    {i18n.t(Key::Edit)}
                                                }
                                                " | "
                                                button {
                                                    class: "text-red-600 hover:text-red-800",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        if let Some(id) = rack_id {
                                                            let window = web_sys::window().unwrap();
                                                            let confirmed = window.confirm_with_message("Are you sure you want to delete this rack?").unwrap_or(false);
                                                            if confirmed {
                                                                spawn({
                                                                    let config = CONFIG.read().clone();
                                                                    async move {
                                                                        match api::delete_rack(&config, id).await {
                                                                            Ok(_) => {
                                                                                // Reload the rack list
                                                                                RACKS.write().loading = true;
                                                                                match api::get_racks(&config).await {
                                                                                    Ok(racks) => {
                                                                                        RACKS.write().items = racks;
                                                                                        RACKS.write().error = None;
                                                                                    }
                                                                                    Err(e) => {
                                                                                        RACKS.write().error = Some(format!("Failed to reload racks: {}", e));
                                                                                    }
                                                                                }
                                                                                RACKS.write().loading = false;
                                                                            }
                                                                            Err(e) => {
                                                                                RACKS.write().error = Some(format!("Failed to delete rack: {}", e));
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                        }
                                                    },
                                                    {i18n.t(Key::Delete)}
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
    }
}
