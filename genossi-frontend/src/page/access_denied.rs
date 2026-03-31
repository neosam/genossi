use dioxus::prelude::*;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};

#[derive(PartialEq, Clone, Props)]
pub struct AccessDeniedProps {
    required_privilege: String,
}

#[component]
pub fn AccessDeniedPage(props: AccessDeniedProps) -> Element {
    let i18n = use_i18n();

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8 flex items-center justify-center",
                div { class: "bg-red-50 border border-red-200 rounded-lg p-8 max-w-md",
                    h3 { class: "text-lg font-medium text-red-800 mb-4",
                        {i18n.t(Key::AccessDenied)}
                    }
                    p { class: "text-red-700 mb-2",
                        "Required privilege: {props.required_privilege}"
                    }
                    div { class: "mt-6",
                        Link {
                            to: crate::router::Route::Home {},
                            class: "inline-flex items-center px-4 py-2 text-sm font-medium rounded-md text-red-700 bg-red-100 hover:bg-red-200",
                            {i18n.t(Key::Back)}
                        }
                    }
                }
            }
        }
    }
}
