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

                        // === ERROR LEVEL (Red) ===

                        // Shares mismatches
                        {rule_section_header(data.shares_mismatches.is_empty(), &i18n, Key::SharesMismatches, "red")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.shares_mismatches.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::Expected)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::Actual)} }
                                    }}
                                    tbody {
                                        for item in data.shares_mismatches.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number}" }
                                                td { class: "py-2 px-3", "{item.expected}" }
                                                td { class: "py-2 px-3", "{item.actual}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Missing entry actions
                        {rule_section_header(data.missing_entry_actions.is_empty(), &i18n, Key::MissingEntryActions, "red")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.missing_entry_actions.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::EntryActionCount)} }
                                    }}
                                    tbody {
                                        for item in data.missing_entry_actions.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number}" }
                                                td { class: "py-2 px-3", "{item.actual_count}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Duplicate member numbers
                        {rule_section_header(data.duplicate_member_numbers.is_empty(), &i18n, Key::DuplicateMemberNumbers, "red")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.duplicate_member_numbers.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                    }}
                                    tbody {
                                        for item in data.duplicate_member_numbers.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number} ({item.member_ids.len()}x)" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Unmatched transfers
                        {rule_section_header(data.unmatched_transfers.is_empty(), &i18n, Key::UnmatchedTransfers, "red")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.unmatched_transfers.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::ActionType)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::TransferMemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::SharesChange)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::Date)} }
                                    }}
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

                        // === WARNING LEVEL (Yellow) ===

                        // Member number gaps
                        {rule_section_header(data.member_number_gaps.is_empty(), &i18n, Key::MemberNumberGaps, "yellow")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.member_number_gaps.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                p { class: "text-gray-600 mb-2", {i18n.t(Key::MissingNumbers)} ": " }
                                div { class: "flex flex-wrap gap-2",
                                    for gap in data.member_number_gaps.iter() {
                                        span { class: "bg-yellow-100 text-yellow-800 px-3 py-1 rounded-full text-sm font-medium",
                                            "{gap}"
                                        }
                                    }
                                }
                            }
                        }

                        // Exit date mismatches
                        {rule_section_header(data.exit_date_mismatches.is_empty(), &i18n, Key::ExitDateMismatches, "yellow")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.exit_date_mismatches.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::HasExitDate)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::HasAustrittAction)} }
                                    }}
                                    tbody {
                                        for item in data.exit_date_mismatches.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number}" }
                                                td { class: "py-2 px-3", {i18n.t(if item.has_exit_date { Key::Yes } else { Key::No })} }
                                                td { class: "py-2 px-3", {i18n.t(if item.has_austritt_action { Key::Yes } else { Key::No })} }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Active members no shares
                        {rule_section_header(data.active_members_no_shares.is_empty(), &i18n, Key::ActiveMembersNoShares, "yellow")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.active_members_no_shares.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                    }}
                                    tbody {
                                        for item in data.active_members_no_shares.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Migrated flag mismatches
                        {rule_section_header(data.migrated_flag_mismatches.is_empty(), &i18n, Key::MigratedFlagMismatches, "yellow")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.migrated_flag_mismatches.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::FlagValue)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::ComputedStatus)} }
                                    }}
                                    tbody {
                                        for item in data.migrated_flag_mismatches.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number}" }
                                                td { class: "py-2 px-3", "{item.flag_value}" }
                                                td { class: "py-2 px-3", "{item.computed_status}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // === INFO LEVEL (Blue) ===

                        // Exited members with shares
                        {rule_section_header(data.exited_members_with_shares.is_empty(), &i18n, Key::ExitedMembersWithShares, "blue")}
                        div { class: "bg-white rounded-lg shadow p-6 mb-4",
                            if data.exited_members_with_shares.is_empty() {
                                p { class: "text-green-600", {i18n.t(Key::ValidationNoIssues)} }
                            } else {
                                table { class: "w-full",
                                    thead { tr { class: "border-b text-left",
                                        th { class: "py-2 px-3", {i18n.t(Key::MemberNumber)} }
                                        th { class: "py-2 px-3", {i18n.t(Key::CurrentShares)} }
                                    }}
                                    tbody {
                                        for item in data.exited_members_with_shares.iter() {
                                            tr { class: "border-b hover:bg-gray-50",
                                                td { class: "py-2 px-3", "{item.member_number}" }
                                                td { class: "py-2 px-3", "{item.current_shares}" }
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

fn rule_section_header(is_ok: bool, i18n: &crate::i18n::I18n, title_key: Key, severity: &str) -> Element {
    let (icon, icon_color, title_color) = if is_ok {
        ("✓", "text-green-600", "text-green-700")
    } else {
        match severity {
            "red" => ("!", "text-red-600", "text-red-700"),
            "yellow" => ("!", "text-yellow-600", "text-yellow-700"),
            "blue" => ("i", "text-blue-600", "text-blue-700"),
            _ => ("!", "text-gray-600", "text-gray-700"),
        }
    };

    rsx! {
        div { class: "flex items-center mb-2 mt-4",
            span { class: "text-xl mr-3 {icon_color}", "{icon}" }
            h2 { class: "text-xl font-semibold {title_color}",
                {i18n.t(title_key)}
            }
        }
    }
}
