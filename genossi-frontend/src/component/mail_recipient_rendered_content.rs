//! Quick 260614-9zf — MailRecipientRenderedContent: compact display of the
//! per-recipient rendered subject + body that the worker actually sent.
//!
//! Reused by both recipient tables in `mail_page.rs` (the expandable job list and
//! the `MailJobDetail` page) — Component-First per `genossi-frontend/CLAUDE.md`,
//! no inline RSX duplication. When both fields are None (legacy rows, not-yet-sent
//! or pre-render failures) it renders nothing, so recipient rows stay unobtrusive.
//!
//! Text interpolation `{...}` is HTML-escaped by Dioxus (no dangerous_inner_html);
//! `whitespace-pre-wrap` is CSS-only and preserves line breaks without an HTML path.
use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

#[component]
pub fn MailRecipientRenderedContent(
    rendered_subject: Option<String>,
    rendered_body: Option<String>,
) -> Element {
    // Nothing rendered → no block at all (keeps not-sent rows clean).
    if rendered_subject.is_none() && rendered_body.is_none() {
        return rsx! {};
    }

    let i18n = use_i18n();
    let heading = i18n.t(Key::MailRenderedContent);
    let subject_label = i18n.t(Key::MailSubject);
    let body_label = i18n.t(Key::MailBody);

    rsx! {
        div { class: "mt-1 rounded border border-gray-200 bg-gray-50 p-2 text-xs text-gray-700",
            div { class: "mb-1 font-medium text-gray-500", "{heading}" }
            if let Some(subject) = rendered_subject {
                div { class: "mb-1",
                    span { class: "font-medium", "{subject_label}: " }
                    span { "{subject}" }
                }
            }
            if let Some(body) = rendered_body {
                div {
                    div { class: "font-medium", "{body_label}:" }
                    div { class: "whitespace-pre-wrap", "{body}" }
                }
            }
        }
    }
}
