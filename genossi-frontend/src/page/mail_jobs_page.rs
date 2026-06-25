use dioxus::prelude::*;

use crate::auth::RequirePrivilege;
use crate::component::{MailJobsList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;

/// Quick 260614-ckn: Dedizierte Seite für die Mail-Job-Liste (/mail/jobs).
/// Die Seite komponiert nur Layout + `MailJobsList` (Pages enthalten kein rohes
/// Listen-RSX, Component-First). Admin-gated analog zur Versand-Seite (MailPage).
#[component]
pub fn MailJobsPage() -> Element {
    let i18n = use_i18n();
    rsx! {
        RequirePrivilege {
            privilege: crate::auth::PRIVILEGE_ADMIN,
            fallback: rsx! { AccessDeniedPage { required_privilege: crate::auth::PRIVILEGE_ADMIN.to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::MailHistory)}
                    }
                    MailJobsList {}
                }
            }
        }
    }
}
