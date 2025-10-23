use crate::component::{DuplicateMatch, ExpandableSection};
use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;
use rest_types::DuplicateDetectionResultTO;
use std::collections::HashMap;

#[derive(Props, Clone, PartialEq)]
pub struct DuplicateResultsListProps {
    pub results: Vec<DuplicateDetectionResultTO>,
}

#[component]
pub fn DuplicateResultsList(props: DuplicateResultsListProps) -> Element {
    let i18n = use_i18n();
    
    // State to track which sections are expanded
    let mut expanded_sections = use_signal(|| HashMap::<String, bool>::new());
    let mut all_expanded = use_signal(|| false);

    if props.results.is_empty() {
        return rsx! {
            div { class: "text-center py-8",
                div { class: "text-gray-500 text-lg",
                    {i18n.t(Key::NoDuplicatesFound)}
                }
            }
        };
    }

    // Clone results for use in closures
    let results_for_expand = props.results.clone();
    let results_for_collapse = props.results.clone();
    
    // Helper functions for expanding/collapsing
    let handle_expand_all = move |_| {
        all_expanded.set(true);
        let mut new_map = HashMap::new();
        for result in results_for_expand.iter() {
            if !result.matches.is_empty() {
                let key = result.checked_product.ean.clone();
                new_map.insert(key, true);
            }
        }
        expanded_sections.set(new_map);
    };
    
    let handle_collapse_all = move |_| {
        all_expanded.set(false);
        let mut new_map = HashMap::new();
        for result in results_for_collapse.iter() {
            if !result.matches.is_empty() {
                let key = result.checked_product.ean.clone();
                new_map.insert(key, false);
            }
        }
        expanded_sections.set(new_map);
    };
    
    rsx! {
        div { class: "space-y-4",
            // Expand/Collapse All controls
            if !props.results.is_empty() && props.results.iter().any(|r| !r.matches.is_empty()) {
                div { class: "flex justify-end space-x-2 mb-4",
                    button {
                        class: "px-3 py-1 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 transition-colors",
                        onclick: handle_expand_all,
                        {i18n.t(Key::ExpandAll)}
                    }
                    button {
                        class: "px-3 py-1 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 transition-colors",
                        onclick: handle_collapse_all,
                        {i18n.t(Key::CollapseAll)}
                    }
                }
            }
            
            // Results with expandable sections
            for result in props.results.iter() {
                if !result.matches.is_empty() {
                    ExpandableSection {
                        title: format!("{}: {}", i18n.t(Key::OriginalProduct), result.checked_product.name),
                        subtitle: Some(format!(
                            "EAN: {} | {} | {} {}", 
                            result.checked_product.ean,
                            i18n.format_price(result.checked_product.price.to_cents()),
                            result.matches.len(),
                            i18n.t(Key::PotentialDuplicatesFound)
                        )),
                        expanded: *expanded_sections.read().get(&result.checked_product.ean).unwrap_or(&false),
                        on_toggle: {
                            let key = result.checked_product.ean.clone();
                            move |_expanded: bool| {
                                let mut current = expanded_sections.read().clone();
                                let is_expanded = current.get(&key).unwrap_or(&false);
                                current.insert(key.clone(), !is_expanded);
                                expanded_sections.set(current);
                            }
                        },
                        children: rsx! {
                            div { class: "space-y-3",
                                for duplicate_match in result.matches.iter() {
                                    DuplicateMatch {
                                        duplicate_match: duplicate_match.clone(),
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