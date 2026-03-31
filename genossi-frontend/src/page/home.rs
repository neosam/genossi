use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 flex items-center justify-center",
                div { class: "text-center",
                    h1 { class: "text-6xl font-bold mb-8",
                        {i18n.t(Key::AppTitle)}
                    }
                    p { class: "text-xl text-gray-600 mb-12",
                        "Genossenschafts-Verwaltung"
                    }
                    div { class: "flex gap-4 justify-center",
                        button {
                            class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition",
                            onclick: move |_| { nav.push(Route::Members {}); },
                            {i18n.t(Key::Members)}
                        }
                    }
                }
            }
        }
    }
}
