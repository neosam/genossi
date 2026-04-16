use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

pub const ALLOWED_PAGE_SIZES: [i64; 5] = [25, 50, 100, 200, 500];

#[component]
pub fn PageSizeSelect(current_size: i64, on_size_change: EventHandler<i64>) -> Element {
    let i18n = use_i18n();
    rsx! {
        div { class: "flex items-center gap-2",
            label { class: "text-sm text-gray-700", {i18n.t(Key::PageSize)} }
            select {
                class: "border rounded px-2 py-1 text-sm",
                value: "{current_size}",
                onchange: move |e| {
                    if let Ok(n) = e.value().parse::<i64>() {
                        on_size_change.call(n);
                    }
                },
                for size in ALLOWED_PAGE_SIZES.iter() {
                    option { value: "{size}", "{size}" }
                }
            }
        }
    }
}
