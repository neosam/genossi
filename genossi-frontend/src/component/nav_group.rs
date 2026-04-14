use dioxus::prelude::*;

use crate::router::Route;

#[derive(Clone, PartialEq)]
pub struct NavItem {
    pub label: String,
    pub route: Route,
}

#[component]
pub fn NavGroup(
    label: String,
    items: Vec<NavItem>,
    is_open: bool,
    on_toggle: EventHandler<()>,
    on_navigate: EventHandler<()>,
) -> Element {
    rsx! {
        li { class: "relative",
            button {
                class: "hover:underline px-3 py-2 md:py-4 flex items-center gap-1 cursor-pointer",
                onclick: move |_| on_toggle.call(()),
                "{label}"
                span { class: "text-xs ml-1",
                    if is_open { "\u{25BE}" } else { "\u{25B8}" }
                }
            }
            if is_open {
                ul {
                    class: "pl-4 md:pl-0 md:absolute md:left-0 md:top-full md:bg-gray-700 md:rounded-b md:shadow-lg md:min-w-48 md:py-1",
                    for item in items.iter() {
                        li {
                            onclick: move |_| on_navigate.call(()),
                            Link {
                                class: "hover:underline md:hover:bg-gray-600 block px-4 py-2 whitespace-nowrap",
                                to: item.route.clone(),
                                "{item.label}"
                            }
                        }
                    }
                }
            }
        }
    }
}
