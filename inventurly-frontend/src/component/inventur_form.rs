use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::inventur::INVENTURS;
use dioxus::prelude::*;
use rest_types::InventurTO;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[component]
pub fn InventurForm(inventur_id: Option<Uuid>) -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let mut inventur = use_signal(|| InventurTO {
        id: inventur_id,
        name: String::new(),
        description: String::new(),
        start_date: None,
        end_date: None,
        status: "draft".to_string(),
        created_by: None,
        created: None,
        deleted: None,
        version: None,
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Store date strings separately for input fields
    let mut start_date_str = use_signal(|| String::new());
    let mut end_date_str = use_signal(|| String::new());

    // Load existing inventur data if editing
    use_effect(move || {
        if let Some(id) = inventur_id {
            spawn({
                let mut inventur = inventur.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();
                let mut start_date_str = start_date_str.clone();
                let mut end_date_str = end_date_str.clone();

                async move {
                    loading.set(true);
                    let config = CONFIG.read().clone();

                    match api::get_inventur(&config, id).await {
                        Ok(inventur_data) => {
                            // Format dates for input fields
                            if let Some(start) = inventur_data.start_date {
                                start_date_str.set(format_datetime_for_input(start));
                            }
                            if let Some(end) = inventur_data.end_date {
                                end_date_str.set(format_datetime_for_input(end));
                            }

                            *inventur.write() = inventur_data;
                            error.set(None);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load inventur: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    let save_inventur = move || {
        spawn({
            let mut inventur = inventur.clone();
            let mut loading = loading.clone();
            let mut error = error.clone();
            let nav = nav.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();
                let inventur_data = inventur.read().clone();

                let result = if inventur_data.id.is_some() {
                    // Update existing inventur
                    api::update_inventur(&config, inventur_data).await
                } else {
                    // Create new inventur
                    api::create_inventur(&config, inventur_data).await
                };

                match result {
                    Ok(saved_inventur) => {
                        // Update the inventur with the returned data (includes ID for new inventurs)
                        *inventur.write() = saved_inventur;

                        // Reload the inventurs list to include the new/updated inventur
                        INVENTURS.write().loading = true;
                        match api::get_inventurs(&config).await {
                            Ok(inventurs) => {
                                INVENTURS.write().items = inventurs;
                                INVENTURS.write().error = None;
                            }
                            Err(e) => {
                                INVENTURS.write().error =
                                    Some(format!("Failed to reload inventurs: {}", e));
                            }
                        }
                        INVENTURS.write().loading = false;

                        // Navigate to inventur list on success
                        nav.push(Route::Inventurs {});
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save inventur: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    };

    let i18n_delete = i18n.clone();
    let delete_inventur = move || {
        if let Some(id) = inventur_id {
            spawn({
                let mut loading = loading.clone();
                let mut error = error.clone();
                let nav = nav.clone();
                let i18n_clone = i18n_delete.clone();

                async move {
                    // Simple confirmation using web_sys
                    let window = web_sys::window().unwrap();
                    let confirmed = window
                        .confirm_with_message(&i18n_clone.t(Key::ConfirmDelete).to_string())
                        .unwrap_or(false);

                    if confirmed {
                        loading.set(true);
                        error.set(None);

                        let config = CONFIG.read().clone();

                        match api::delete_inventur(&config, id).await {
                            Ok(_) => {
                                // Navigate to inventur list on success
                                nav.push(Route::Inventurs {});
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to delete inventur: {}", e)));
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
                if inventur_id.is_some() {
                    {i18n.t(Key::EditInventur)}
                } else {
                    {i18n.t(Key::CreateInventur)}
                }
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
                        {i18n.t(Key::InventurName)}
                    }
                    input {
                        id: "name",
                        r#type: "text",
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                        value: "{inventur.read().name}",
                        oninput: move |e| {
                            inventur.write().name = e.value();
                        },
                        required: true,
                    }
                }

                div { class: "mb-4",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        r#for: "description",
                        {i18n.t(Key::InventurDescription)}
                    }
                    textarea {
                        id: "description",
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                        rows: "3",
                        value: "{inventur.read().description}",
                        oninput: move |e| {
                            inventur.write().description = e.value();
                        },
                        required: true,
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mb-4",
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "start_date",
                            {i18n.t(Key::InventurStartDate)}
                        }
                        input {
                            id: "start_date",
                            r#type: "datetime-local",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{start_date_str.read()}",
                            oninput: move |e| {
                                let value = e.value();
                                start_date_str.set(value.clone());
                                inventur.write().start_date = parse_datetime_from_input(&value);
                            },
                        }
                    }

                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "end_date",
                            {i18n.t(Key::InventurEndDate)}
                        }
                        input {
                            id: "end_date",
                            r#type: "datetime-local",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{end_date_str.read()}",
                            oninput: move |e| {
                                let value = e.value();
                                end_date_str.set(value.clone());
                                inventur.write().end_date = parse_datetime_from_input(&value);
                            },
                        }
                    }
                }

                div { class: "mb-4",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        r#for: "status",
                        {i18n.t(Key::InventurStatus)}
                    }
                    select {
                        id: "status",
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                        value: "{inventur.read().status}",
                        onchange: move |e| {
                            inventur.write().status = e.value();
                        },
                        option { value: "draft", {i18n.t(Key::StatusDraft)} }
                        option { value: "active", {i18n.t(Key::StatusActive)} }
                        option { value: "completed", {i18n.t(Key::StatusCompleted)} }
                        option { value: "cancelled", {i18n.t(Key::StatusCancelled)} }
                    }
                }

                div { class: "flex gap-4",
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50",
                        disabled: *loading.read(),
                        onclick: move |_| save_inventur(),
                        if *loading.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::Save)}
                        }
                    }
                    if inventur_id.is_some() {
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50",
                            disabled: *loading.read(),
                            onclick: move |_| delete_inventur(),
                            {i18n.t(Key::Delete)}
                        }
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                        onclick: move |_| { nav.push(Route::Inventurs {}); },
                        {i18n.t(Key::Cancel)}
                    }
                }
            }
        }
    }
}

// Helper function to format PrimitiveDateTime for datetime-local input
fn format_datetime_for_input(dt: PrimitiveDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute()
    )
}

// Helper function to parse datetime-local input to PrimitiveDateTime
fn parse_datetime_from_input(s: &str) -> Option<PrimitiveDateTime> {
    if s.is_empty() {
        return None;
    }

    // Expected format: "2025-01-07T14:30"
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }

    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() != 3 || time_parts.len() < 2 {
        return None;
    }

    let year = date_parts[0].parse().ok()?;
    let month = date_parts[1].parse::<u8>().ok()?;
    let day = date_parts[2].parse().ok()?;
    let hour = time_parts[0].parse().ok()?;
    let minute = time_parts[1].parse().ok()?;

    let date = time::Date::from_calendar_date(
        year,
        time::Month::try_from(month).ok()?,
        day,
    ).ok()?;

    let time = time::Time::from_hms(hour, minute, 0).ok()?;

    Some(PrimitiveDateTime::new(date, time))
}
