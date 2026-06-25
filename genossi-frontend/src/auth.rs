use dioxus::prelude::*;

use crate::service::auth::AUTH;

/// Zentrale Konstante für das "admin"-Privileg. Vermeidet das über die Pages
/// verstreute Magic-String-Literal `"admin"` (ein Tippfehler wie `"admni"` würde
/// eine Seite sonst unbemerkt falsch öffnen/sperren). Bezeichnet das PRIVILEG —
/// NICHT den gleichnamigen Rollennamen in der Rollenverwaltung (admin-privilege-const).
pub const PRIVILEGE_ADMIN: &str = "admin";

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

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// DRY-Regression-Guard (admin-privilege-const): kein rohes `privilege: "admin"`
    /// Literal mehr in den Pages — alle Privilege-Guards müssen PRIVILEGE_ADMIN
    /// nutzen. Rollennamen ("admin" als Rolle in permissions.rs) sind bewusst
    /// NICHT betroffen (anderes Konzept).
    #[test]
    fn test_no_raw_admin_privilege_literal_in_pages() {
        let pages = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/page");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(&pages).expect("src/page must exist") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            // Fängt sowohl `privilege: "admin"` als auch `required_privilege: "admin"`.
            if src.contains("privilege: \"admin\"") {
                offenders.push(path.display().to_string());
            }
        }
        assert!(
            offenders.is_empty(),
            "Rohes `privilege: \"admin\"` Literal gefunden — \
             bitte crate::auth::PRIVILEGE_ADMIN verwenden: {offenders:?}"
        );
    }
}
