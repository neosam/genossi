use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use dioxus::prelude::*;
use rest_types::ContainerTO;
use uuid::Uuid;

#[component]
pub fn ContainerForm(container_id: Option<Uuid>) -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let mut container = use_signal(|| ContainerTO {
        id: container_id,
        name: String::new(),
        weight_grams: 0,
        description: String::new(),
        created: None,
        deleted: None,
        version: None,
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Load existing container data if editing
    use_effect(move || {
        if let Some(id) = container_id {
            spawn({
                let mut container = container.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    let config = CONFIG.read().clone();

                    match api::get_container(&config, id).await {
                        Ok(container_data) => {
                            *container.write() = container_data;
                            error.set(None);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load container: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    let handle_submit = move |_| {
        spawn({
            let mut loading = loading.clone();
            let mut error = error.clone();
            let container = container.read().clone();
            let nav = nav.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();

                let result = if container.id.is_some() {
                    api::update_container(&config, container).await
                } else {
                    api::create_container(&config, container).await
                };

                match result {
                    Ok(_) => {
                        // Reload the containers list
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

                        // Navigate back to containers list
                        nav.push(Route::Containers {});
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save container: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    };

    let handle_cancel = move |_| {
        nav.push(Route::Containers {});
    };

    rsx! {
        div { class: "max-w-2xl mx-auto",
            if let Some(error_msg) = error.read().as_ref() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    {error_msg.clone()}
                }
            }

            div { class: "bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4",
                div { class: "mb-4",
                    label { class: "block text-gray-700 text-sm font-bold mb-2",
                        r#for: "name",
                        {i18n.t(Key::ContainerName)}
                    }
                    input {
                        class: "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline",
                        id: "name",
                        r#type: "text",
                        value: "{container.read().name}",
                        oninput: move |evt| {
                            let mut c = container.write();
                            c.name = evt.value();
                        },
                        placeholder: "{i18n.t(Key::ContainerName)}",
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-gray-700 text-sm font-bold mb-2",
                        r#for: "weight_grams",
                        {i18n.t(Key::ContainerWeightGrams)}
                    }
                    input {
                        class: "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline",
                        id: "weight_grams",
                        r#type: "number",
                        value: "{container.read().weight_grams}",
                        oninput: move |evt| {
                            if let Ok(value) = evt.value().parse::<i64>() {
                                let mut c = container.write();
                                c.weight_grams = value;
                            }
                        },
                        placeholder: "{i18n.t(Key::ContainerWeightGrams)}",
                        min: "0",
                    }
                }

                div { class: "mb-6",
                    label { class: "block text-gray-700 text-sm font-bold mb-2",
                        r#for: "description",
                        {i18n.t(Key::ContainerDescription)}
                    }
                    textarea {
                        class: "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline",
                        id: "description",
                        value: "{container.read().description}",
                        oninput: move |evt| {
                            let mut c = container.write();
                            c.description = evt.value();
                        },
                        placeholder: "{i18n.t(Key::ContainerDescription)}",
                        rows: "3",
                    }
                }

                div { class: "flex items-center justify-between",
                    button {
                        class: "bg-gray-500 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline",
                        r#type: "button",
                        onclick: handle_cancel,
                        disabled: *loading.read(),
                        {i18n.t(Key::Cancel)}
                    }

                    button {
                        class: "bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline disabled:opacity-50",
                        r#type: "button",
                        onclick: handle_submit,
                        disabled: *loading.read() || container.read().name.trim().is_empty() || container.read().weight_grams <= 0,
                        if *loading.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::Save)}
                        }
                    }
                }
            }
        }
    }
}
