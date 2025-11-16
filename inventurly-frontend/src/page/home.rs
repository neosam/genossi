use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::auth::AUTH;
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    // Auto-redirect if user has inventur_id claim
    use_effect(move || {
        let auth = AUTH.read();
        if let Some(ref auth_info) = auth.auth_info {
            if let Some(inventur_id) = auth_info.get_inventur_id() {
                nav.push(Route::InventurRackSelection { id: inventur_id });
            }
        }
    });

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 flex items-center justify-center",
                div { class: "text-center",
                    h1 { class: "text-6xl font-bold mb-8",
                        {i18n.t(Key::AppTitle)}
                    }
                    p { class: "text-xl text-gray-600 mb-12",
                        "Inventory Management System"
                    }
                    div { class: "flex gap-4 justify-center",
                        button {
                            class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition",
                            onclick: move |_| { nav.push(Route::Products {}); },
                            {i18n.t(Key::Products)}
                        }
                    }
                }
            }
        }
    }
}
