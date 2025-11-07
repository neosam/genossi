use dioxus::prelude::*;
use crate::i18n::{use_i18n, Key};

#[component]
pub fn InventurStatusBadge(status: String) -> Element {
    let i18n = use_i18n();

    let (status_text, status_class) = match status.as_str() {
        "draft" => (i18n.t(Key::StatusDraft), "bg-gray-200 text-gray-800"),
        "active" => (i18n.t(Key::StatusActive), "bg-green-200 text-green-800"),
        "completed" => (i18n.t(Key::StatusCompleted), "bg-blue-200 text-blue-800"),
        "cancelled" => (i18n.t(Key::StatusCancelled), "bg-red-200 text-red-800"),
        _ => (status.clone().into(), "bg-gray-200 text-gray-800"),
    };

    rsx! {
        span {
            class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {status_class}",
            "{status_text}"
        }
    }
}
