use dioxus::prelude::*;

use crate::api::InboundMailAttachmentTO;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::util::format::format_size;

/// Single attachment row inside `InboxAttachmentList`.
///
/// Renders one of three layouts depending on MIME type:
/// - oversized → amber hint label, no actions
/// - image/* → inline thumbnail wrapped in preview anchor + primary download
/// - application/pdf → primary download + secondary inline preview link
/// - else → primary download only
///
/// All download/preview actions are anchor elements only — no Dioxus event
/// handlers attached, which sidesteps the documented form-reload pitfall.
/// Every `target="_blank"` anchor carries `rel="noopener"` (T-08 mitigation).
#[component]
pub fn InboxAttachmentListItem(mail_id: String, attachment: InboundMailAttachmentTO) -> Element {
    let i18n = use_i18n();
    let cfg = CONFIG.read().clone();

    let download_url = format!(
        "{}/api/inbox/{}/attachments/{}",
        cfg.backend, mail_id, attachment.id
    );
    let inline_url = format!("{}?disposition=inline", download_url);

    let size_str = format_size(attachment.size_bytes.max(0) as u64);

    // Branch 1 — oversized: no download/preview, only amber hint.
    if attachment.oversized {
        let oversized_label = format!("{} ({})", i18n.t(Key::InboxAttachmentsOversized), size_str);
        return rsx! {
            li { class: "p-3 border rounded bg-white flex items-center gap-3",
                span { aria_hidden: "true", "📎" }
                div { class: "flex flex-col flex-1 min-w-0",
                    span { class: "text-sm truncate", title: "{attachment.file_name}",
                        "{attachment.file_name}"
                    }
                    span { class: "text-xs text-amber-700",
                        "{oversized_label}"
                    }
                }
            }
        };
    }

    // Branch 2/3/4 — normal row with primary download + optional preview.
    let is_image = attachment.mime_type.starts_with("image/");
    let is_pdf = attachment.mime_type == "application/pdf";
    let alt_text = format!(
        "{} {}",
        i18n.t(Key::InboxAttachmentsImageAltPrefix),
        attachment.file_name
    );
    let short_mime_label = short_mime(&attachment.mime_type);
    let meta_line = format!("{} · {}", size_str, short_mime_label);

    rsx! {
        li { class: "p-3 border rounded bg-white flex items-center gap-3",
            // Leading visual: thumbnail (image) or glyph (everything else)
            if is_image {
                a {
                    href: "{inline_url}",
                    target: "_blank",
                    rel: "noopener",
                    img {
                        src: "{inline_url}",
                        alt: "{alt_text}",
                        class: "max-h-24 max-w-32 object-contain rounded border",
                        loading: "lazy",
                    }
                }
            } else {
                span { aria_hidden: "true", "{glyph_for_mime(&attachment.mime_type)}" }
            }

            // Metadata column
            div { class: "flex flex-col flex-1 min-w-0",
                span { class: "text-sm truncate", title: "{attachment.file_name}",
                    "{attachment.file_name}"
                }
                span { class: "text-xs text-gray-500",
                    "{meta_line}"
                }
            }

            // Action column
            div { class: "flex gap-2 ml-auto",
                a {
                    href: "{download_url}",
                    download: "{attachment.file_name}",
                    class: "px-3 py-1.5 bg-blue-500 hover:bg-blue-600 text-white text-sm rounded",
                    "{i18n.t(Key::InboxAttachmentsDownload)}"
                }
                if is_pdf {
                    a {
                        href: "{inline_url}",
                        target: "_blank",
                        rel: "noopener",
                        class: "px-3 py-1.5 text-blue-600 hover:underline text-sm",
                        "{i18n.t(Key::InboxAttachmentsPreview)}"
                    }
                }
            }
        }
    }
}

/// Map a MIME type string to a Unicode glyph used as a leading icon.
/// (Used only when no image thumbnail is rendered.)
fn glyph_for_mime(mime: &str) -> &'static str {
    if mime == "application/pdf" {
        "📄"
    } else if mime.starts_with("image/") {
        "🖼️"
    } else if mime == "application/zip" || mime == "application/x-tar" || mime == "application/gzip"
    {
        "🗜️"
    } else if mime == "application/msword"
        || mime.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml")
    {
        "📝"
    } else if mime == "application/vnd.ms-excel"
        || mime.starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml")
    {
        "📊"
    } else if mime.starts_with("text/") {
        "📃"
    } else {
        "📎"
    }
}

/// Short, human-readable MIME label shown in the metadata line next to the size.
fn short_mime(mime: &str) -> &'static str {
    if mime == "application/pdf" {
        "PDF"
    } else if mime.starts_with("image/") {
        "Bild"
    } else if mime == "application/msword"
        || mime.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml")
    {
        "Word"
    } else if mime == "application/vnd.ms-excel"
        || mime.starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml")
    {
        "Excel"
    } else {
        "Datei"
    }
}
