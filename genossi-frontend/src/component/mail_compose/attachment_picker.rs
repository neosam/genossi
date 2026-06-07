//! Quick 260607-s0s: shared MailAttachmentPicker — extracted 1:1 from the
//! inline Compose-Picker (mail_page.rs:478-559) so the Inbox Reply form can
//! show EXACTLY the same UI.
//!
//! The Compose-page wraps the member-doc block in
//! `if selected_member_ids.len() == 1 { … }` because it supports bulk-send
//! to multiple recipients. The Reply-flow only ever has a single recipient,
//! so the equivalent condition collapses to `member_id.is_some()`.
//!
//! Component-First (project rule): both call sites render this one
//! component instead of duplicating the RSX.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::StaticDocumentTO;
use rest_types::MemberDocumentTO;

#[component]
pub fn MailAttachmentPicker(
    /// The recipient member id. Some(id) renders the member-doc checkbox
    /// block; None hides it (analog to Compose's "no single recipient" case).
    member_id: Option<Uuid>,
    available_documents: Signal<Vec<MemberDocumentTO>>,
    available_static_documents: Signal<Vec<StaticDocumentTO>>,
    mut selected_member_doc_ids: Signal<Vec<Uuid>>,
    mut selected_static_doc_ids: Signal<Vec<String>>,
) -> Element {
    rsx! {
        // Member-document picker — visible only when we have exactly one
        // identified recipient (= we can scope the docs to that member).
        if member_id.is_some() {
            div {
                label { class: "block text-sm font-medium text-gray-700 mb-1",
                    "Anhänge"
                }
                if available_documents.read().is_empty() {
                    p { class: "text-sm text-gray-400 italic",
                        "Keine Dokumente vorhanden"
                    }
                } else {
                    div { class: "border rounded-md p-2 max-h-40 overflow-y-auto space-y-1",
                        for doc in available_documents.read().iter() {
                            {
                                let doc_id = doc.id;
                                let doc_type = doc.document_type.clone();
                                let file_name = doc.file_name.clone();
                                let is_checked = doc_id.map(|id| selected_member_doc_ids.read().contains(&id)).unwrap_or(false);
                                rsx! {
                                    label { class: "flex items-center gap-2 px-2 py-1 hover:bg-gray-50 rounded cursor-pointer text-sm",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_checked,
                                            onchange: move |_| {
                                                if let Some(id) = doc_id {
                                                    let mut ids = selected_member_doc_ids.write();
                                                    if ids.contains(&id) {
                                                        ids.retain(|i| *i != id);
                                                    } else {
                                                        ids.push(id);
                                                    }
                                                }
                                            },
                                        }
                                        span { class: "text-gray-600", "{doc_type}" }
                                        span { class: "text-gray-800 font-medium", "{file_name}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Static document multiselect (applies to every recipient in this send).
        if !available_static_documents.read().is_empty() {
            div { class: "mt-4 p-3 border border-gray-200 rounded bg-gray-50",
                div { class: "text-sm font-medium text-gray-700 mb-2", "Statische Dokumente anhängen (alle Empfänger erhalten diese)" }
                div { class: "flex flex-col space-y-1 max-h-40 overflow-y-auto",
                    for sd in available_static_documents.read().iter() {
                        {
                            let sd_id = sd.id.clone();
                            let sd_name = sd.name.clone();
                            let sd_filename = sd.filename.clone();
                            let is_checked = selected_static_doc_ids.read().contains(&sd_id);
                            let sd_id_for_change = sd_id.clone();
                            rsx! {
                                label { class: "flex items-center space-x-2 text-sm",
                                    input {
                                        r#type: "checkbox",
                                        checked: is_checked,
                                        onchange: move |evt| {
                                            let id = sd_id_for_change.clone();
                                            let mut ids = selected_static_doc_ids.write();
                                            if evt.checked() {
                                                if !ids.contains(&id) {
                                                    ids.push(id);
                                                }
                                            } else {
                                                ids.retain(|x| x != &id);
                                            }
                                        },
                                    }
                                    span { "{sd_name} ({sd_filename})" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
