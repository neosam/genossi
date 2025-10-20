use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::rack::RACKS;
use dioxus::prelude::*;
use rest_types::RackTO;
use uuid::Uuid;

#[component]
pub fn RackForm(rack_id: Option<Uuid>) -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let mut rack = use_signal(|| RackTO {
        id: rack_id,
        name: String::new(),
        description: String::new(),
        created: None,
        deleted: None,
        version: None,
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Load existing rack data if editing
    use_effect(move || {
        if let Some(id) = rack_id {
            spawn({
                let mut rack = rack.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    let config = CONFIG.read().clone();

                    match api::get_rack(&config, id).await {
                        Ok(rack_data) => {
                            *rack.write() = rack_data;
                            error.set(None);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load rack: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    let save_rack = move || {
        spawn({
            let mut rack = rack.clone();
            let mut loading = loading.clone();
            let mut error = error.clone();
            let nav = nav.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();
                let rack_data = rack.read().clone();

                let result = if rack_data.id.is_some() {
                    // Update existing rack
                    api::update_rack(&config, rack_data).await
                } else {
                    // Create new rack
                    api::create_rack(&config, rack_data).await
                };

                match result {
                    Ok(saved_rack) => {
                        // Update the rack with the returned data (includes ID for new racks)
                        *rack.write() = saved_rack;

                        // Reload the racks list to include the new/updated rack
                        RACKS.write().loading = true;
                        match api::get_racks(&config).await {
                            Ok(racks) => {
                                RACKS.write().items = racks;
                                RACKS.write().error = None;
                            }
                            Err(e) => {
                                RACKS.write().error =
                                    Some(format!("Failed to reload racks: {}", e));
                            }
                        }
                        RACKS.write().loading = false;

                        // Navigate to rack list on success
                        nav.push(Route::Racks {});
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save rack: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    };

    let delete_rack = move || {
        if let Some(id) = rack_id {
            spawn({
                let mut loading = loading.clone();
                let mut error = error.clone();
                let nav = nav.clone();

                async move {
                    // Simple confirmation using web_sys
                    let window = web_sys::window().unwrap();
                    let confirmed = window
                        .confirm_with_message("Are you sure you want to delete this rack?")
                        .unwrap_or(false);

                    if confirmed {
                        loading.set(true);
                        error.set(None);

                        let config = CONFIG.read().clone();

                        match api::delete_rack(&config, id).await {
                            Ok(_) => {
                                // Navigate to rack list on success
                                nav.push(Route::Racks {});
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to delete rack: {}", e)));
                            }
                        }

                        loading.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-2xl font-bold mb-6",
                if rack_id.is_some() {
                    {i18n.t(Key::Edit)}
                } else {
                    {i18n.t(Key::Create)}
                }
                " "
                {i18n.t(Key::Racks)}
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    {err.clone()}
                }
            }

            div {

                div { class: "mb-4",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        r#for: "name",
                        {i18n.t(Key::RackName)}
                    }
                    input {
                        id: "name",
                        r#type: "text",
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                        value: "{rack.read().name}",
                        oninput: move |e| {
                            rack.write().name = e.value();
                        },
                        required: true,
                    }
                }

                div { class: "mb-4",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        r#for: "description",
                        {i18n.t(Key::RackDescription)}
                    }
                    textarea {
                        id: "description",
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                        rows: "3",
                        value: "{rack.read().description}",
                        oninput: move |e| {
                            rack.write().description = e.value();
                        },
                        required: true,
                    }
                }

                div { class: "flex gap-4",
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50",
                        disabled: *loading.read(),
                        onclick: move |_| save_rack(),
                        if *loading.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::Save)}
                        }
                    }
                    if rack_id.is_some() {
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50",
                            disabled: *loading.read(),
                            onclick: move |_| delete_rack(),
                            {i18n.t(Key::Delete)}
                        }
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                        onclick: move |_| { nav.push(Route::Racks {}); },
                        {i18n.t(Key::Cancel)}
                    }
                }
            }
        }
    }
}
