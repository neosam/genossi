use dioxus::prelude::*;

use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::member_utils;
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;

fn today_string() -> String {
    let today = member_utils::today();
    format!(
        "{:04}-{:02}-{:02}",
        today.year(),
        today.month() as u8,
        today.day()
    )
}

#[component]
pub fn BackupPage() -> Element {
    let i18n = use_i18n();
    let config = CONFIG.read().clone();
    let mut date = use_signal(|| today_string());

    let members_url = format!(
        "{}/api/backup/members?date={}",
        config.backend,
        date.read()
    );
    let actions_url = format!("{}/api/backup/actions", config.backend);
    let documents_url = format!("{}/api/backup/documents", config.backend);

    rsx! {
        RequirePrivilege {
            privilege: "export_backup",
            fallback: rsx! { AccessDeniedPage { required_privilege: "export_backup".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::Backup)}
                    }

                    div { class: "grid gap-6 md:grid-cols-1 lg:grid-cols-3",
                        // Member list CSV card
                        div { class: "bg-white border border-gray-200 rounded-lg p-6 shadow-sm",
                            h2 { class: "text-xl font-semibold mb-2",
                                {i18n.t(Key::BackupMemberList)}
                            }
                            p { class: "text-gray-600 mb-4",
                                {i18n.t(Key::BackupMemberListDescription)}
                            }
                            div { class: "mb-4",
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::BackupCutoffDate)}
                                }
                                input {
                                    r#type: "date",
                                    class: "border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500",
                                    value: "{date}",
                                    onchange: move |e| {
                                        date.set(e.value());
                                    },
                                }
                            }
                            a {
                                href: "{members_url}",
                                class: "inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                {i18n.t(Key::Download)}
                            }
                        }

                        // Actions CSV card
                        div { class: "bg-white border border-gray-200 rounded-lg p-6 shadow-sm",
                            h2 { class: "text-xl font-semibold mb-2",
                                {i18n.t(Key::BackupActions)}
                            }
                            p { class: "text-gray-600 mb-4",
                                {i18n.t(Key::BackupActionsDescription)}
                            }
                            a {
                                href: "{actions_url}",
                                class: "inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                {i18n.t(Key::Download)}
                            }
                        }

                        // Documents ZIP card
                        div { class: "bg-white border border-gray-200 rounded-lg p-6 shadow-sm",
                            h2 { class: "text-xl font-semibold mb-2",
                                {i18n.t(Key::BackupDocuments)}
                            }
                            p { class: "text-gray-600 mb-4",
                                {i18n.t(Key::BackupDocumentsDescription)}
                            }
                            div { class: "bg-yellow-50 border border-yellow-200 text-yellow-800 text-sm rounded px-3 py-2 mb-4",
                                {i18n.t(Key::BackupDocumentsWarning)}
                            }
                            a {
                                href: "{documents_url}",
                                class: "inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                {i18n.t(Key::Download)}
                            }
                        }
                    }
                }
            }
        }
    }
}
