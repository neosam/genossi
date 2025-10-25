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
                    div { class: "flex items-center mb-4",
                        div { class: "flex-shrink-0",
                            svg { 
                                class: "h-8 w-8 text-red-400",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                                }
                            }
                        }
                        div { class: "ml-3",
                            h3 { class: "text-lg font-medium text-red-800",
                                {i18n.t(Key::AccessDenied)}
                            }
                        }
                    }
                    div { class: "text-red-700",
                        p { class: "mb-2",
                            {i18n.t(Key::InsufficientPrivileges)}
                        }
                        p { class: "text-sm text-red-600",
                            "Required privilege: {props.required_privilege}"
                        }
                    }
                    div { class: "mt-6",
                        Link { 
                            to: crate::router::Route::Home {},
                            class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-red-700 bg-red-100 hover:bg-red-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                            {i18n.t(Key::BackToHome)}
                        }
                    }
                }
            }
        }
    }
}