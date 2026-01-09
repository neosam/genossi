use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::container_rack::add_container_to_rack_action;
use dioxus::prelude::*;
use rest_types::ContainerTO;
use uuid::Uuid;

#[component]
pub fn ContainerRackForm(
    container_id: Option<Uuid>,
    rack_id: Option<Uuid>,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();

    let mut selected_container = use_signal(|| container_id);
    let selected_rack = use_signal(|| rack_id);
    let containers = use_signal(|| Vec::<ContainerTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let saving = use_signal(|| false);

    // Load containers on mount
    use_effect(move || {
        spawn({
            let mut containers = containers.clone();
            let mut loading = loading.clone();
            let mut error = error.clone();

            async move {
                loading.set(true);
                let config = CONFIG.read().clone();

                match api::get_containers(&config).await {
                    Ok(container_list) => {
                        containers.set(container_list);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load containers: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    });

    let handle_save = move |_| {
        if let (Some(cont_id), Some(rack_id)) = (selected_container(), selected_rack()) {
            spawn({
                let mut saving = saving.clone();
                let mut error = error.clone();
                let on_saved = on_saved.clone();

                async move {
                    saving.set(true);
                    error.set(None);

                    match add_container_to_rack_action(cont_id, rack_id).await {
                        Ok(()) => {
                            on_saved.call(());
                        }
                        Err(e) => {
                            error.set(Some(e));
                        }
                    }

                    saving.set(false);
                }
            });
        }
    };

    let is_valid = selected_container().is_some() && selected_rack().is_some();

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-bold",
                {i18n.t(Key::AddContainerToRack)}
            }

            if loading() {
                div { class: "text-center py-4",
                    {i18n.t(Key::Loading)}
                }
            } else {
                div { class: "space-y-4",

                    if let Some(error_msg) = error() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                            {error_msg}
                        }
                    }

                    // Container selection
                    div {
                        label { class: "block text-sm font-medium mb-1",
                            {i18n.t(Key::SelectContainerToAdd)}
                        }
                        select {
                            class: "w-full px-3 py-2 border rounded-md",
                            disabled: container_id.is_some() || saving(),
                            value: selected_container().map(|id| id.to_string()).unwrap_or_default(),
                            onchange: move |event| {
                                if let Ok(uuid) = Uuid::parse_str(&event.value()) {
                                    selected_container.set(Some(uuid));
                                } else {
                                    selected_container.set(None);
                                }
                            },

                            option { value: "", "-- {i18n.t(Key::SelectContainerToAdd)} --" }

                            for container in containers().iter() {
                                option {
                                    value: container.id.unwrap().to_string(),
                                    selected: Some(container.id.unwrap()) == selected_container(),
                                    "{container.name} ({container.weight_grams}g)"
                                }
                            }
                        }
                    }

                    // Action buttons
                    div { class: "flex space-x-2 pt-4",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: !is_valid || saving(),
                            onclick: handle_save,

                            if saving() {
                                {i18n.t(Key::Loading)}
                            } else {
                                {i18n.t(Key::Save)}
                            }
                        }

                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600",
                            disabled: saving(),
                            onclick: move |_| on_cancel.call(()),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }
    }
}
