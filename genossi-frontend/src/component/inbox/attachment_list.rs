use dioxus::prelude::*;

use crate::api::InboundMailAttachmentTO;
use crate::i18n::{use_i18n, Key};

use super::InboxAttachmentListItem;

/// Attachment section rendered inside the inbox detail pane.
///
/// Visibility rules (UI-SPEC §Component Contract):
/// - `attachments.is_empty() && !has_legacy_attachments` → renders nothing
/// - `attachments.is_empty() && has_legacy_attachments` → header + amber legacy hint
/// - non-empty → header + list of `InboxAttachmentListItem` rows
#[component]
pub fn InboxAttachmentList(
    mail_id: String,
    attachments: Vec<InboundMailAttachmentTO>,
    has_legacy_attachments: bool,
) -> Element {
    if attachments.is_empty() && !has_legacy_attachments {
        return rsx! {};
    }

    let i18n = use_i18n();
    let count = attachments.len();
    let header_text = format!("{} ({})", i18n.t(Key::InboxAttachmentsHeader), count);

    rsx! {
        div { class: "border-t pt-2 mt-3 flex flex-col gap-2",
            div { class: "text-sm font-semibold",
                span { aria_hidden: "true", "📎 " }
                "{header_text}"
            }
            if attachments.is_empty() {
                // Legacy backfill — header + amber hint, no rows
                div { class: "text-xs text-amber-700",
                    "{i18n.t(Key::InboxAttachmentsEmptyLegacy)}"
                }
            } else {
                ul { class: "flex flex-col gap-2",
                    for att in attachments.iter().cloned() {
                        InboxAttachmentListItem {
                            key: "{att.id}",
                            mail_id: mail_id.clone(),
                            attachment: att,
                        }
                    }
                }
            }
        }
    }
}
