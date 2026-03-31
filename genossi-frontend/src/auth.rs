use dioxus::prelude::*;

use crate::service::auth::AUTH;

#[derive(PartialEq, Clone, Props)]
pub struct AuthProps {
    authenticated: Element,
    unauthenticated: Element,
}

#[component]
pub fn Auth(props: AuthProps) -> Element {
    let auth = AUTH.read().clone();

    match (auth.auth_info, auth.loading_done) {
        (Some(_auth_info), true) => props.authenticated,
        (None, true) => props.unauthenticated,
        (_, false) => {
            rsx! {
                div { "Fetching auth information..." }
            }
        }
    }
}

#[derive(PartialEq, Clone, Props)]
pub struct RequirePrivilegeProps {
    privilege: &'static str,
    children: Element,
    #[props(default)]
    fallback: Option<Element>,
}

#[component]
pub fn RequirePrivilege(props: RequirePrivilegeProps) -> Element {
    let auth = AUTH.read().clone();

    match auth.auth_info {
        Some(auth_info) if auth_info.has_privilege(props.privilege) => props.children,
        _ => props.fallback.unwrap_or_else(|| {
            rsx! {
                div { class: "text-red-600 p-4",
                    "Access denied. Required privilege: {props.privilege}"
                }
            }
        }),
    }
}
