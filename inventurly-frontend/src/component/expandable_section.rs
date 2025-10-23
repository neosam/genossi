use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ExpandableSectionProps {
    pub title: String,
    pub subtitle: Option<String>,
    pub expanded: bool,
    pub on_toggle: EventHandler<bool>,
    pub children: Element,
}

#[component]
pub fn ExpandableSection(props: ExpandableSectionProps) -> Element {
    let i18n = use_i18n();
    
    rsx! {
        div { class: "border border-gray-200 rounded-lg bg-white",
            // Header - always visible
            div { 
                class: "flex items-center justify-between p-4 cursor-pointer hover:bg-gray-50 transition-colors",
                onclick: move |_| props.on_toggle.call(!props.expanded),
                
                div { class: "flex-1",
                    h3 { class: "text-lg font-semibold text-gray-900",
                        {props.title.clone()}
                    }
                    if let Some(subtitle) = &props.subtitle {
                        p { class: "text-sm text-gray-600 mt-1",
                            {subtitle.clone()}
                        }
                    }
                }
                
                // Expand/Collapse button with icon
                div { class: "flex items-center space-x-2",
                    span { class: "text-sm text-gray-500",
                        if props.expanded {
                            {i18n.t(Key::HideDetails)}
                        } else {
                            {i18n.t(Key::ShowDetails)}
                        }
                    }
                    // Chevron icon
                    svg {
                        class: format!("w-5 h-5 text-gray-400 transition-transform duration-200 {}", 
                            if props.expanded { "rotate-180" } else { "" }),
                        fill: "none",
                        stroke: "currentColor",
                        "viewBox": "0 0 24 24",
                        path {
                            "stroke-linecap": "round",
                            "stroke-linejoin": "round",
                            "stroke-width": "2",
                            d: "M19 9l-7 7-7-7"
                        }
                    }
                }
            }
            
            // Content - only visible when expanded
            if props.expanded {
                div { class: "border-t border-gray-100 p-4",
                    {props.children}
                }
            }
        }
    }
}