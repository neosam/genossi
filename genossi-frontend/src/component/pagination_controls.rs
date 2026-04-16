use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

/// Build the list of page numbers to render in the navigation strip,
/// inserting `None` placeholders for ellipsis gaps. Always includes
/// page 0, the last page, the current page, and one neighbor on each side.
fn page_strip(current_page: i64, total_pages: i64) -> Vec<Option<i64>> {
    if total_pages <= 0 {
        return vec![Some(0)];
    }
    let last = total_pages - 1;
    let mut shown: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    shown.insert(0);
    shown.insert(last);
    for delta in -1_i64..=1 {
        let p = current_page + delta;
        if p >= 0 && p <= last {
            shown.insert(p);
        }
    }
    let mut out: Vec<Option<i64>> = Vec::new();
    let mut prev: Option<i64> = None;
    for p in shown {
        if let Some(prev_p) = prev {
            if p > prev_p + 1 {
                out.push(None);
            }
        }
        out.push(Some(p));
        prev = Some(p);
    }
    out
}

#[component]
pub fn PaginationControls(
    current_page: i64,
    total_pages: i64,
    on_page_change: EventHandler<i64>,
) -> Element {
    let i18n = use_i18n();
    let total_pages = total_pages.max(1);
    let last = total_pages - 1;
    let at_first = current_page <= 0;
    let at_last = current_page >= last;

    let nav_btn_class = "px-3 py-1 rounded border bg-white text-gray-700 hover:bg-gray-100 \
                        disabled:opacity-40 disabled:cursor-not-allowed";
    let page_btn_class = "px-3 py-1 rounded border bg-white text-gray-700 hover:bg-gray-100";
    let page_btn_active = "px-3 py-1 rounded border bg-blue-600 text-white font-semibold";

    rsx! {
        div { class: "flex items-center gap-1 flex-wrap",
            button {
                class: "{nav_btn_class}",
                disabled: at_first,
                onclick: move |_| on_page_change.call(0),
                {i18n.t(Key::PaginationFirst)}
            }
            button {
                class: "{nav_btn_class}",
                disabled: at_first,
                onclick: move |_| on_page_change.call((current_page - 1).max(0)),
                {i18n.t(Key::PaginationPrev)}
            }
            for entry in page_strip(current_page, total_pages) {
                match entry {
                    Some(p) => {
                        let cls = if p == current_page { page_btn_active } else { page_btn_class };
                        rsx! {
                            button {
                                class: "{cls}",
                                onclick: move |_| on_page_change.call(p),
                                "{p + 1}"
                            }
                        }
                    }
                    None => rsx! {
                        span { class: "px-2 text-gray-400", "…" }
                    },
                }
            }
            button {
                class: "{nav_btn_class}",
                disabled: at_last,
                onclick: move |_| on_page_change.call((current_page + 1).min(last)),
                {i18n.t(Key::PaginationNext)}
            }
            button {
                class: "{nav_btn_class}",
                disabled: at_last,
                onclick: move |_| on_page_change.call(last),
                {i18n.t(Key::PaginationLast)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_strip_short() {
        // 3 pages, current = 1 → all pages shown, no ellipsis.
        let strip = page_strip(1, 3);
        assert_eq!(strip, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn test_page_strip_long_middle() {
        // 20 pages, current = 10 → 0, …, 9, 10, 11, …, 19
        let strip = page_strip(10, 20);
        assert_eq!(
            strip,
            vec![Some(0), None, Some(9), Some(10), Some(11), None, Some(19)]
        );
    }

    #[test]
    fn test_page_strip_long_at_start() {
        // 20 pages, current = 0 → 0, 1, …, 19
        let strip = page_strip(0, 20);
        assert_eq!(strip, vec![Some(0), Some(1), None, Some(19)]);
    }

    #[test]
    fn test_page_strip_long_at_end() {
        // 20 pages, current = 19 → 0, …, 18, 19
        let strip = page_strip(19, 20);
        assert_eq!(strip, vec![Some(0), None, Some(18), Some(19)]);
    }

    #[test]
    fn test_page_strip_single_page() {
        let strip = page_strip(0, 1);
        assert_eq!(strip, vec![Some(0)]);
    }

    #[test]
    fn test_page_strip_zero_pages() {
        let strip = page_strip(0, 0);
        assert_eq!(strip, vec![Some(0)]);
    }
}
