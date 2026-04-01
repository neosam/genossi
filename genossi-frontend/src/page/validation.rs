use dioxus::prelude::*;
use rest_types::{ActionTypeTO, ValidationResultTO};

use crate::api;
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;

#[component]
pub fn Validation() -> Element {
    let i18n = use_i18n();
    let mut result = use_signal(|| None::<ValidationResultTO>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_validation(&config).await {
                Ok(data) => {
                    result.set(Some(data));
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    });

    let action_type_label = |at: &ActionTypeTO| -> Key {
        match at {
            ActionTypeTO::UebertragungEmpfang => Key::ActionUebertragungEmpfang,
            ActionTypeTO::UebertragungAbgabe => Key::ActionUebertragungAbgabe,
            _ => Key::ActionType,
        }
    };

    rsx! {
        RequirePrivilege {
            privilege: "view_members",
            fallback: rsx! { AccessDeniedPage { required_privilege: "view_members".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::Validation)}
                    }

                    if *loading.read() {
                        p { class: "text-gray-600", {i18n.t(Key::Loading)} }
                    } else if let Some(err) = error.read().as_ref() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                            "{err}"
                        }
                    } else if let Some(data) = result.read().as_ref() {
                        // Rule 1: Member number gaps
                        div { class: "bg-white rounded-lg shadow p-6 mb-6",
                            div { class: "flex items-center mb-4",
                                if data.member_number_gaps.is_empty() {
                                    span { class: "text-green-600 text-xl mr-3", "✓" }
                                    h2 { class: "text-xl font-semibold text-green-700",
                                        {i18n.t(Key::MemberNumberGaps)}
                                    }
                                } else {
                                    span { class: "text-yellow-600 text-xl mr-3", "!" }
                                    h2 { class: "text-xl font-semibold text-yellow-700",
                                        {i18n.t(Key::MemberNumberGaps)}
                                    }
                                }
                            }
                            if data.member_number_gaps.is_empty() {
                                p { class: "text-green-600",
                                    {i18n.t(Key::ValidationNoIssues)}
                                }
                            } else {
                                p { class: "text-gray-600 mb-2",
                                    {i18n.t(Key::MissingNumbers)}
                                    ": "
                                }
                                div { class: "flex flex-wrap gap-2",
                                    for gap in data.member_number_gaps.iter() {
                                        span { class: "bg-yellow-100 text-yellow-800 px-3 py-1 rounded-full text-sm font-medium",
                                            "{gap}"
                                        }
                                    }
                                }
                            }
                        }

                        // Rule 2: Unmatched transfers
                        div { class: "bg-white rounded-lg shadow p-6 mb-6",
                            div { class: "flex items-center mb-4",
                                if data.unmatched_transfers.is_empty() {
                                    span { class: "text-green-600 text-xl mr-3", "✓" }
                                    h2 { class: "text-xl font-semibold text-green-700",
                                        {i18n.t(Key::UnmatchedTransfers)}
                                    }
                                } else {
                                    span { class: "text-red-600 text-xl mr-3", "!" }
                                    h2 { class: "text-xl font-semibold text-red-700",
                                        {i18n.t(Key::UnmatchedTransfers)}
                                    }
                                }
                            }
                            if data.unmatched_transfers.is_empty() {
                                p { class: "text-green-600",
                                    {i18n.t(Key::ValidationNoIssues)}
                                }
                            } else {
                                table { class: "w-full",
                                    thead {
                                        tr { class: "border-b text-left",
                                            th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                            th { class: "py-2 px-3", {i18n.t(Key::ActionType)} }
                                            th { class: "py-2 px-3", {i18n.t(Key::TransferMemberNumber)} }
                                            th { class: "py-2 px-3", {i18n.t(Key::SharesChange)} }
                                            th { class: "py-2 px-3", {i18n.t(Key::Date)} }
                                        }
                                    }
                                    tbody {
                                        for transfer in data.unmatched_transfers.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{transfer.member_number}" }
                                                td { class: "py-2 px-3", {i18n.t(action_type_label(&transfer.action_type))} }
                                                td { class: "py-2 px-3", "{transfer.transfer_member_number}" }
                                                td { class: "py-2 px-3", "{transfer.shares_change}" }
                                                td { class: "py-2 px-3", {i18n.format_date(&transfer.date)} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
