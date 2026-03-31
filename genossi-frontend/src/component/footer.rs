use dioxus::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "bg-gray-800 text-gray-400 py-4 px-4 text-center text-sm print:hidden",
            p { "Genossi v{VERSION}" }
        }
    }
}
