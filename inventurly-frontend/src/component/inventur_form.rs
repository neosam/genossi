use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::inventur::INVENTURS;
use dioxus::prelude::*;
use rest_types::{InventurMeasurementTO, InventurTO};
use std::collections::HashSet;
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

    // Measurement progress tracking
    let measurements = use_signal(|| Vec::<InventurMeasurementTO>::new());
    let total_measured = use_signal(|| 0usize);
    let total_products = use_signal(|| 0usize);

    // Load existing inventur data if editing
    use_effect(move || {
        if let Some(id) = inventur_id {
            spawn({
                let mut inventur = inventur.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();
                let mut start_date_str = start_date_str.clone();
                let mut end_date_str = end_date_str.clone();
                let mut measurements = measurements.clone();
                let mut total_measured = total_measured.clone();
                let mut total_products = total_products.clone();

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

                    // Load measurements to calculate progress
                    match api::get_measurements_by_inventur(&config, id).await {
                        Ok(measurement_data) => {
                            // Calculate unique measured products
                            let measured_products: HashSet<Uuid> = measurement_data
                                .iter()
                                .filter(|m| m.deleted.is_none())
                                .map(|m| m.product_id)
                                .collect();

                            total_measured.set(measured_products.len());
                            measurements.set(measurement_data);
                        }
                        Err(_) => {
                            // Silently fail for measurements - not critical for form editing
                        }
                    }

                    // Load all product-rack relationships to calculate total products
                    match api::get_all_product_rack_relationships(&config).await {
                        Ok(product_racks) => {
                            // Calculate unique products in racks
                            let unique_products: HashSet<Uuid> = product_racks
                                .iter()
                                .filter(|pr| pr.deleted.is_none())
                                .map(|pr| pr.product_id)
                                .collect();

                            total_products.set(unique_products.len());
                        }
                        Err(_) => {
                            // Silently fail - not critical for form editing
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

    // Determine if fields should be locked (when inventur is active or completed)
    let is_locked = inventur.read().status == "active" || inventur.read().status == "completed";

    // Calculate progress percentage
    let percentage = if total_products() > 0 {
        (total_measured() as f64 / total_products() as f64 * 100.0).round() as u32
    } else {
        0
    };

    // Calculate progress badge class and text
    let measured = total_measured();
    let total = total_products();
    let (progress_class, progress_text) = if measured == 0 {
        ("px-2 py-1 rounded-full bg-gray-200 text-gray-700", i18n.t(Key::NotStarted))
    } else if measured == total {
        ("px-2 py-1 rounded-full bg-green-500 text-white", i18n.t(Key::Complete))
    } else {
        ("px-2 py-1 rounded-full bg-green-200 text-green-800", i18n.t(Key::InProgress))
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

            // Progress display (only when editing and have total products loaded)
            if inventur_id.is_some() && total > 0 {
                div { class: "bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6",
                    div { class: "flex items-center justify-between mb-2",
                        h3 { class: "text-lg font-semibold text-blue-900",
                            {i18n.t(Key::MeasurementProgress)}
                        }
                        div { class: "text-sm",
                            span {
                                class: "{progress_class}",
                                {progress_text.clone()}
                            }
                        }
                    }
                    p { class: "text-blue-800",
                        {format!("{} / {} {} ({}%)", measured, total, i18n.t(Key::ProductsMeasured), percentage)}
                    }
                    if let Some(id) = inventur_id {
                        a {
                            href: "#",
                            class: "text-sm text-blue-600 hover:text-blue-800 mt-2 inline-block",
                            onclick: move |_| {
                                nav.push(Route::InventurRackSelection { id: id.to_string() });
                            },
                            "→ {i18n.t(Key::ViewRackProgress)}"
                        }
                    }
                }
            }

            // Info banner when fields are locked
            if is_locked {
                div { class: "bg-yellow-50 border border-yellow-200 text-yellow-800 px-4 py-3 rounded mb-4",
                    p { class: "text-sm",
                        "Fields are locked because the inventur is {inventur.read().status}. Change status to 'draft' or 'cancelled' to edit."
                    }
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
                        class: if is_locked {
                            "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm bg-gray-100 cursor-not-allowed"
                        } else {
                            "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                        },
                        value: "{inventur.read().name}",
                        oninput: move |e| {
                            inventur.write().name = e.value();
                        },
                        disabled: is_locked,
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
                        class: if is_locked {
                            "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm bg-gray-100 cursor-not-allowed"
                        } else {
                            "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                        },
                        rows: "3",
                        value: "{inventur.read().description}",
                        oninput: move |e| {
                            inventur.write().description = e.value();
                        },
                        disabled: is_locked,
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
                            class: if is_locked {
                                "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm bg-gray-100 cursor-not-allowed"
                            } else {
                                "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                            },
                            value: "{start_date_str.read()}",
                            oninput: move |e| {
                                let value = e.value();
                                start_date_str.set(value.clone());
                                inventur.write().start_date = parse_datetime_from_input(&value);
                            },
                            disabled: is_locked,
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
                            class: if is_locked {
                                "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm bg-gray-100 cursor-not-allowed"
                            } else {
                                "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                            },
                            value: "{end_date_str.read()}",
                            oninput: move |e| {
                                let value = e.value();
                                end_date_str.set(value.clone());
                                inventur.write().end_date = parse_datetime_from_input(&value);
                            },
                            disabled: is_locked,
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
