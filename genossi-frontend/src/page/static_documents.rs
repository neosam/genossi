use dioxus::prelude::*;

use crate::api::{self, StaticDocumentTO};
use crate::component::TopBar;
use crate::service::config::CONFIG;

#[component]
pub fn StaticDocumentsPage() -> Element {
    let mut documents = use_signal(Vec::<StaticDocumentTO>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut new_name = use_signal(String::new);
    let mut uploading = use_signal(|| false);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_static_documents(&config).await {
                Ok(data) => {
                    documents.set(data);
                    error.set(None);
                }
                Err(e) => error.set(Some(format!("Fehler beim Laden: {}", e))),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        TopBar {}
        div { class: "container mx-auto p-4",
            h1 { class: "text-2xl font-bold mb-4", "Dokumente" }
            p { class: "text-gray-600 mb-4",
                "Statische Dokumente (z.B. Satzung, Flyer), die an Bulk-Mails angehängt werden können."
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    "{err}"
                }
            }

            // Upload form
            div { class: "bg-white rounded-lg shadow p-4 mb-6",
                h2 { class: "text-lg font-semibold mb-3", "Neues Dokument hochladen" }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-3 items-end",
                    div {
                        label { class: "block text-sm font-medium text-gray-700", "Name" }
                        input {
                            class: "mt-1 block w-full rounded border-gray-300 shadow-sm",
                            r#type: "text",
                            placeholder: "z.B. Satzung 2026",
                            value: "{new_name.read()}",
                            oninput: move |e| new_name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700", "Datei (PDF, PNG, JPEG)" }
                        input {
                            id: "static-document-file-input",
                            class: "mt-1 block w-full text-sm",
                            r#type: "file",
                            accept: "application/pdf,image/png,image/jpeg",
                        }
                    }
                    div {
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50",
                            disabled: *uploading.read(),
                            onclick: move |_| {
                                spawn(async move {
                                    uploading.set(true);
                                    error.set(None);
                                    let window = web_sys::window().unwrap();
                                    let document = window.document().unwrap();
                                    let input = document
                                        .get_element_by_id("static-document-file-input")
                                        .and_then(|el| {
                                            use wasm_bindgen::JsCast;
                                            el.dyn_into::<web_sys::HtmlInputElement>().ok()
                                        });
                                    let file = input
                                        .and_then(|inp| inp.files())
                                        .and_then(|files| files.get(0));

                                    match file {
                                        Some(f) => {
                                            let config = CONFIG.read().clone();
                                            let name = new_name.read().clone();
                                            let effective = if name.trim().is_empty() {
                                                f.name()
                                            } else {
                                                name
                                            };
                                            match api::upload_static_document(&config, &effective, f).await {
                                                Ok(_) => {
                                                    new_name.set(String::new());
                                                    reload();
                                                }
                                                Err(e) => error.set(Some(format!("Upload fehlgeschlagen: {}", e))),
                                            }
                                        }
                                        None => error.set(Some("Keine Datei ausgewählt".to_string())),
                                    }
                                    uploading.set(false);
                                });
                            },
                            if *uploading.read() { "Lädt…" } else { "Hochladen" }
                        }
                    }
                }
            }

            // Document list
            if *loading.read() {
                div { class: "text-gray-500", "Lade Dokumente..." }
            } else if documents.read().is_empty() {
                div { class: "bg-white rounded-lg shadow p-6 text-gray-500 text-center",
                    "Noch keine Dokumente hochgeladen."
                }
            } else {
                div { class: "bg-white rounded-lg shadow overflow-x-auto",
                    table { class: "min-w-full divide-y divide-gray-200",
                        thead { class: "bg-gray-50",
                            tr {
                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Name" }
                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Datei" }
                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Typ" }
                                th { class: "px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase", "Größe" }
                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", "" }
                            }
                        }
                        tbody { class: "bg-white divide-y divide-gray-200",
                            for doc in documents.read().iter() {
                                {
                                    let d = doc.clone();
                                    let d_id = d.id.clone();
                                    let d_id_delete = d.id.clone();
                                    let cfg = CONFIG.read().clone();
                                    let download_url = api::static_document_download_url(&cfg, &d_id);
                                    rsx! {
                                        tr {
                                            td { class: "px-4 py-3 text-sm text-gray-900", "{d.name}" }
                                            td { class: "px-4 py-3 text-sm text-gray-600",
                                                a {
                                                    class: "text-blue-600 hover:underline",
                                                    href: "{download_url}",
                                                    target: "_blank",
                                                    "{d.filename}"
                                                }
                                            }
                                            td { class: "px-4 py-3 text-sm text-gray-600", "{d.content_type}" }
                                            td { class: "px-4 py-3 text-sm text-gray-600 text-right",
                                                "{format_size(d.size_bytes)}"
                                            }
                                            td { class: "px-4 py-3 text-sm",
                                                button {
                                                    class: "text-red-600 hover:underline",
                                                    onclick: move |_| {
                                                        let id_str = d_id_delete.clone();
                                                        spawn(async move {
                                                            let config = CONFIG.read().clone();
                                                            match api::delete_static_document(&config, &id_str).await {
                                                                Ok(_) => reload(),
                                                                Err(e) => error.set(Some(format!("Löschen fehlgeschlagen: {}", e))),
                                                            }
                                                        });
                                                    },
                                                    "Löschen"
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

fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
