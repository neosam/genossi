// Phase 25 (APDOC-05): Single-slot Application document component.
//
// Two visual states:
//   * Empty  — "Antrag hochladen" upload trigger.
//   * Filled — filename + size + upload date DD.MM.YYYY + Download / Replace / Delete icons.
//
// All interactive buttons use `r#type: "button"` + `onclick` (never form-submit)
// to avoid the Dioxus form-submit reload bug (Phase 17 hotfix e245013,
// feedback_dioxus_button_type memory). Buttons live outside any <form> anyway
// (they are children of the Application-Detail Modal), but the explicit
// `r#type: "button"` is a belt-and-braces guarantee that the acceptance grep
// gate can enforce going forward.

use dioxus::prelude::*;
use rest_types::ApplicationDocumentTO;
use uuid::Uuid;

use crate::api::{
    application_document_download_url, delete_application_document, get_application_document,
    upload_application_document, AppError,
};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

fn format_size(bytes: i64) -> String {
    // Small helper — <1 MiB shows KB (rounded), otherwise MB (one decimal).
    // Matches the "human readable" pattern in the file-listing page (member_details).
    if bytes < 1024 * 1024 {
        let kb = (bytes as f64 / 1024.0).max(1.0);
        format!("{:.0} KB", kb)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.1} MB", mb)
    }
}

fn format_created(doc: &ApplicationDocumentTO) -> String {
    // DD.MM.YYYY per plan spec. Fall back to empty string if the datetime is
    // missing or the format fails — the row still renders (filename + size).
    if let Some(dt) = doc.created.as_ref() {
        let d = dt.date();
        format!("{:02}.{:02}.{:04}", d.day(), d.month() as u8, d.year())
    } else {
        String::new()
    }
}

fn file_input_id_for(application_id: Uuid) -> String {
    // Unique per Application so nested slots (rare) never collide. The
    // Application-Detail modal creates a fresh slot for each app_id anyway.
    format!("application-document-file-input-{}", application_id)
}

async fn pick_file_from_input(input_id: &str) -> Option<web_sys::File> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window()?;
    let document = window.document()?;
    let element = document.get_element_by_id(input_id)?;
    let input: web_sys::HtmlInputElement = element.dyn_into().ok()?;
    let files = input.files()?;
    let file = files.get(0);
    // Clear the input so re-picking the same file still fires onchange.
    input.set_value("");
    file
}

#[component]
pub fn ApplicationDocumentSlot(application_id: Uuid, on_changed: EventHandler<()>) -> Element {
    let i18n = use_i18n();
    let mut document = use_signal(|| None::<ApplicationDocumentTO>);
    let mut loading = use_signal(|| true);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<AppError>);

    // Load metadata on mount / when application_id changes.
    use_effect(move || {
        let app_id = application_id;
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            match get_application_document(&config, app_id).await {
                Ok(opt) => document.set(opt),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let input_id = file_input_id_for(application_id);
    let download_url = {
        let config = CONFIG.read().clone();
        application_document_download_url(&config, application_id)
    };

    // Two identical file inputs would collide by id — a single hidden input
    // serves both the upload and the replace flows.
    let input_id_for_click = input_id.clone();
    let input_id_for_change = input_id.clone();

    rsx! {
        div { class: "border-t border-b py-4 my-4",
            // Hidden file input reused by both "upload" (empty state) and
            // "replace" (filled state) buttons.
            input {
                id: "{input_id}",
                r#type: "file",
                accept: ".pdf,.png,.jpg,.jpeg,.webp,.txt,.doc,.docx,.odt,.xls,.xlsx,.ods",
                style: "display: none",
                onchange: move |_evt| {
                    let app_id = application_id;
                    let input_id = input_id_for_change.clone();
                    spawn(async move {
                        let Some(file) = pick_file_from_input(&input_id).await else {
                            return;
                        };
                        busy.set(true);
                        error.set(None);
                        let config = CONFIG.read().clone();
                        match upload_application_document(&config, app_id, file).await {
                            Ok(doc) => {
                                document.set(Some(doc));
                                on_changed.call(());
                            }
                            Err(e) => error.set(Some(e)),
                        }
                        busy.set(false);
                    });
                },
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "mb-3 p-2 bg-red-50 border border-red-200 rounded text-red-700 text-sm",
                    "{err}"
                }
            }

            if *loading.read() {
                div { class: "text-sm text-gray-500", {i18n.t(Key::Loading)} }
            } else {
                match document.read().clone() {
                    None => rsx! {
                        // Empty state — single "Antrag hochladen" button.
                        div { class: "flex items-center justify-between",
                            span { class: "text-sm text-gray-500",
                                {i18n.t(Key::ApplicationDocumentEmptyState)}
                            }
                            button {
                                r#type: "button",
                                class: "bg-blue-500 hover:bg-blue-600 text-white text-sm px-3 py-1 rounded disabled:opacity-50",
                                disabled: *busy.read(),
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    use wasm_bindgen::JsCast;
                                    // Trigger the hidden input's click programmatically.
                                    if let Some(w) = web_sys::window() {
                                        if let Some(d) = w.document() {
                                            if let Some(el) = d.get_element_by_id(&input_id_for_click) {
                                                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                                    input.click();
                                                }
                                            }
                                        }
                                    }
                                },
                                {i18n.t(Key::ApplicationDocumentUpload)}
                            }
                        }
                    },
                    Some(doc) => {
                        let file_name = doc.file_name.clone();
                        let size_label = format_size(doc.size);
                        let created_label = format_created(&doc);
                        let input_id_for_replace = input_id.clone();
                        rsx! {
                            div { class: "flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between",
                                div { class: "flex flex-col text-sm",
                                    span { class: "font-medium break-all", "{file_name}" }
                                    span { class: "text-xs text-gray-500",
                                        "{size_label}"
                                        if !created_label.is_empty() {
                                            " · "
                                            "{created_label}"
                                        }
                                    }
                                }
                                div { class: "flex gap-2 flex-wrap",
                                    a {
                                        class: "bg-gray-100 hover:bg-gray-200 text-gray-800 text-xs px-3 py-1 rounded",
                                        href: "{download_url}",
                                        target: "_blank",
                                        {i18n.t(Key::ApplicationDocumentDownload)}
                                    }
                                    button {
                                        r#type: "button",
                                        class: "bg-yellow-100 hover:bg-yellow-200 text-yellow-800 text-xs px-3 py-1 rounded disabled:opacity-50",
                                        disabled: *busy.read(),
                                        onclick: move |evt| {
                                            evt.stop_propagation();
                                            use wasm_bindgen::JsCast;
                                            if let Some(w) = web_sys::window() {
                                                if let Some(d) = w.document() {
                                                    if let Some(el) = d.get_element_by_id(&input_id_for_replace) {
                                                        if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                                            input.click();
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        {i18n.t(Key::ApplicationDocumentReplace)}
                                    }
                                    button {
                                        r#type: "button",
                                        class: "bg-red-100 hover:bg-red-200 text-red-800 text-xs px-3 py-1 rounded disabled:opacity-50",
                                        disabled: *busy.read(),
                                        onclick: move |evt| {
                                            evt.stop_propagation();
                                            let app_id = application_id;
                                            let confirm_msg =
                                                i18n.t(Key::ApplicationDocumentDeleteConfirm).to_string();
                                            let confirmed = web_sys::window()
                                                .and_then(|w| w.confirm_with_message(&confirm_msg).ok())
                                                .unwrap_or(false);
                                            if !confirmed {
                                                return;
                                            }
                                            spawn(async move {
                                                busy.set(true);
                                                error.set(None);
                                                let config = CONFIG.read().clone();
                                                match delete_application_document(&config, app_id).await {
                                                    Ok(_) => {
                                                        document.set(None);
                                                        on_changed.call(());
                                                    }
                                                    Err(e) => error.set(Some(e)),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        {i18n.t(Key::ApplicationDocumentDelete)}
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
