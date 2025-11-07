use crate::api;
use crate::component::InventurStatusBadge;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::inventur::INVENTURS;
use dioxus::prelude::*;

#[component]
pub fn InventurList() -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let inventurs = INVENTURS.read();

    // Filter out deleted inventurs
    let active_inventurs: Vec<_> = inventurs
        .items
        .iter()
        .filter(|inv| inv.deleted.is_none())
        .collect();

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b flex justify-between items-center",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::Inventurs)}
                }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                    onclick: move |_| { nav.push(Route::InventurDetails { id: "new".to_string() }); },
                    {i18n.t(Key::CreateInventur)}
                }
            }

            if inventurs.loading {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::Loading)}
                }
            } else if let Some(ref error) = inventurs.error {
                div { class: "p-6 text-center text-red-500",
                    {error.clone()}
                }
            } else if active_inventurs.is_empty() {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::NoInventursFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::InventurName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::InventurDescription)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::InventurStartDate)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::InventurEndDate)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::InventurStatus)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for inventur in active_inventurs.iter() {
                                {
                                    let inventur_id = inventur.id;
                                    let status = inventur.status.clone();
                                    let name = inventur.name.clone();
                                    let description = inventur.description.clone();
                                    let start_date = inventur.start_date;
                                    let end_date = inventur.end_date;
                                    rsx! {
                                        tr {
                                            class: "border-b hover:bg-gray-50 cursor-pointer",
                                            onclick: move |_| {
                                                if let Some(id) = inventur_id {
                                                    nav.push(Route::InventurDetails { id: id.to_string() });
                                                }
                                            },
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium",
                                                {name}
                                            }
                                            td { class: "px-6 py-4 text-sm",
                                                {description}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                if let Some(date) = start_date {
                                                    {i18n.format_datetime(date)}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                if let Some(date) = end_date {
                                                    {i18n.format_datetime(date)}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                InventurStatusBadge { status: status.clone() }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm space-x-2",
                                                button {
                                                    class: "text-blue-600 hover:text-blue-800",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        if let Some(id) = inventur_id {
                                                            nav.push(Route::InventurDetails { id: id.to_string() });
                                                        }
                                                    },
                                                    {i18n.t(Key::Edit)}
                                                }
                                                " | "
                                                button {
                                                    class: "text-green-600 hover:text-green-800",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        if let Some(id) = inventur_id {
                                                            nav.push(Route::InventurMeasurements { id: id.to_string() });
                                                        }
                                                    },
                                                    {i18n.t(Key::ViewMeasurements)}
                                                }
                                                " | "
                                                {
                                                    let i18n_clone = i18n.clone();
                                                    rsx! {
                                                        button {
                                                            class: "text-red-600 hover:text-red-800",
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                if let Some(id) = inventur_id {
                                                                    let window = web_sys::window().unwrap();
                                                                    let confirmed = window
                                                                        .confirm_with_message(&i18n_clone.t(Key::ConfirmDelete).to_string())
                                                                        .unwrap_or(false);
                                                            if confirmed {
                                                                spawn({
                                                                    let config = CONFIG.read().clone();
                                                                    async move {
                                                                        match api::delete_inventur(&config, id).await {
                                                                            Ok(_) => {
                                                                                // Reload the inventur list
                                                                                INVENTURS.write().loading = true;
                                                                                match api::get_inventurs(&config).await {
                                                                                    Ok(inventurs) => {
                                                                                        INVENTURS.write().items = inventurs;
                                                                                        INVENTURS.write().error = None;
                                                                                    }
                                                                                    Err(e) => {
                                                                                        INVENTURS.write().error = Some(format!("Failed to reload inventurs: {}", e));
                                                                                    }
                                                                                }
                                                                                INVENTURS.write().loading = false;
                                                                            }
                                                                            Err(e) => {
                                                                                INVENTURS.write().error = Some(format!("Failed to delete inventur: {}", e));
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
    }
}
