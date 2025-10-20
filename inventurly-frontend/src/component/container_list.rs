use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use dioxus::prelude::*;

#[component]
pub fn ContainerList() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    let handle_delete = move |id: uuid::Uuid| {
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::delete_container(&config, id).await {
                Ok(_) => {
                    // Reload the container list
                    CONTAINERS.write().loading = true;
                    match api::get_containers(&config).await {
                        Ok(containers) => {
                            CONTAINERS.write().items = containers;
                            CONTAINERS.write().error = None;
                        }
                        Err(e) => {
                            CONTAINERS.write().error =
                                Some(format!("Failed to reload containers: {}", e));
                        }
                    }
                    CONTAINERS.write().loading = false;
                }
                Err(e) => {
                    CONTAINERS.write().error = Some(format!("Failed to delete container: {}", e));
                }
            }
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b flex justify-between items-center",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::Containers)}
                }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                    onclick: move |_| { nav.push(Route::ContainerDetails { id: "new".to_string() }); },
                    {i18n.t(Key::Create)}
                }
            }

            {
                let containers = CONTAINERS.read();
                if containers.loading {
                    rsx! {
                        div { class: "p-6 text-center",
                            {i18n.t(Key::Loading)}
                        }
                    }
                } else if let Some(error) = &containers.error {
                    rsx! {
                        div { class: "p-6 text-center text-red-600",
                            {error.clone()}
                        }
                    }
                } else if containers.items.is_empty() {
                    rsx! {
                        div { class: "p-6 text-center text-gray-500",
                            {i18n.t(Key::NoDataFound)}
                        }
                    }
                } else {
                    let filtered_containers: Vec<_> = containers.items.iter()
                        .filter(|c| c.deleted.is_none())
                        .cloned()
                        .collect();
                    
                    rsx! {
                        div { class: "overflow-x-auto",
                            table { class: "w-full",
                                thead {
                                    tr { class: "border-b bg-gray-50",
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            {i18n.t(Key::ContainerName)}
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            {i18n.t(Key::ContainerWeightGrams)}
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            {i18n.t(Key::ContainerDescription)}
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            {i18n.t(Key::ContainerCreated)}
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            {i18n.t(Key::Actions)}
                                        }
                                    }
                                }
                                tbody {
                                    for container in filtered_containers {
                                        tr {
                                            key: "{container.id:?}",
                                            class: "border-b hover:bg-gray-50",
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900",
                                                {container.name}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                                                {format!("{}g", container.weight_grams)}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-500",
                                                if !container.description.is_empty() {
                                                    span { {container.description} }
                                                } else {
                                                    span { class: "text-gray-400 italic", "-" }
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                                                if let Some(created) = container.created {
                                                    {i18n.format_datetime(created)}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium",
                                                div { class: "flex space-x-2",
                                                    button {
                                                        class: "text-blue-600 hover:text-blue-900",
                                                        onclick: move |_| {
                                                            if let Some(id) = container.id {
                                                                nav.push(Route::ContainerDetails { id: id.to_string() });
                                                            }
                                                        },
                                                        {i18n.t(Key::Edit)}
                                                    }
                                                    button {
                                                        class: "text-red-600 hover:text-red-900",
                                                        onclick: move |_| {
                                                            if let Some(id) = container.id {
                                                                handle_delete(id);
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