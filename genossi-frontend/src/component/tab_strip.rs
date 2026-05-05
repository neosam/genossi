//! TabStrip (Phase 4 Plan 06) — generic tab-strip component.
//!
//! Extracted from applications_page.rs:75-110 inline tab-pattern (D-13 — Component-First).
//! Used in `assembly_details.rs` (Plan 08) for 3-tab layout (Stamm-Daten / Tokens / Anwesenheit).
//!
//! NOTE: applications_page.rs has NOT been migrated in this plan — keeping blast-radius small
//! per Plan-06 instruction. The inline pattern there remains; future refactor can migrate.
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TabDef {
    pub key: &'static str,
    pub label: String,
}

#[component]
pub fn TabStrip(
    tabs: Vec<TabDef>,
    active_key: String,
    on_change: EventHandler<String>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex border-b border-gray-200 mb-6 print:hidden",
            for tab in tabs.iter() {
                {
                    let is_active = active_key == tab.key;
                    let class = if is_active {
                        "px-4 py-3 text-sm font-medium text-blue-600 cursor-default border-b-2 border-blue-600"
                    } else {
                        "px-4 py-3 text-sm font-medium text-gray-500 hover:text-gray-700 cursor-pointer border-b-2 border-transparent"
                    };
                    let key_for_click = tab.key.to_string();
                    let key_attr = tab.key.to_string();
                    let label = tab.label.clone();
                    rsx! {
                        button {
                            key: "{key_attr}",
                            class: "{class}",
                            role: "tab",
                            onclick: move |_| on_change.call(key_for_click.clone()),
                            "{label}"
                        }
                    }
                }
            }
        }
        div { class: "tab-body", {children} }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_def_clones_and_compares() {
        let a = TabDef {
            key: "basics",
            label: "Stammdaten".to_string(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn tab_def_distinct_keys_not_equal() {
        let a = TabDef {
            key: "basics",
            label: "Stammdaten".to_string(),
        };
        let b = TabDef {
            key: "tokens",
            label: "Stammdaten".to_string(),
        };
        assert_ne!(a, b);
    }
}
