use dioxus::prelude::*;

use crate::component::TopBar;
use crate::i18n::use_i18n;
use crate::i18n::Key;
use crate::router::Route;
use crate::service::member::{refresh_members, MEMBERS};

#[component]
pub fn Members() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    let members_state = MEMBERS.read();
    let filter_query = members_state.filter_query.clone();

    let filtered_members: Vec<_> = members_state
        .items
        .iter()
        .filter(|m| m.deleted.is_none())
        .filter(|m| {
            if filter_query.is_empty() {
                return true;
            }
            let q = filter_query.to_lowercase();
            m.first_name.to_lowercase().contains(&q)
                || m.last_name.to_lowercase().contains(&q)
                || m.member_number.to_string().contains(&q)
                || m.city.as_deref().unwrap_or("").to_lowercase().contains(&q)
                || m.email.as_deref().unwrap_or("").to_lowercase().contains(&q)
        })
        .collect();

    rsx! {
        TopBar {}
        div { class: "container mx-auto px-4 py-8",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-3xl font-bold",
                    {i18n.t(Key::Members)}
                    span { class: "ml-2 text-gray-500 font-normal text-base",
                        "({filtered_members.len()})"
                    }
                }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                    onclick: move |_| { nav.push(Route::MemberDetails { id: "new".to_string() }); },
                    {i18n.t(Key::Create)}
                }
            }

            // Search
            div { class: "mb-4",
                input {
                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    placeholder: "{i18n.t(Key::Search)}",
                    value: "{members_state.filter_query}",
                    oninput: move |e| {
                        MEMBERS.write().filter_query = e.value().clone();
                    },
                }
            }

            if members_state.loading {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::Loading)}
                }
            } else if let Some(error) = &members_state.error {
                div { class: "text-center py-8 text-red-500",
                    "{error}"
                }
            } else if filtered_members.is_empty() {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "bg-white rounded-lg shadow overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MemberNumber)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::LastName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::FirstName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::City)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::CurrentShares)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::JoinDate)}
                                }
                            }
                        }
                        tbody {
                            for member in filtered_members.iter() {
                                {
                                    let member_id = member.id;
                                    let join_date = i18n.format_date(&member.join_date);
                                    rsx! {
                                        tr {
                                            class: "border-b hover:bg-gray-50 cursor-pointer",
                                            onclick: move |_| {
                                                if let Some(id) = member_id {
                                                    nav.push(Route::MemberDetails { id: id.to_string() });
                                                }
                                            },
                                            td { class: "px-6 py-4 font-medium", "{member.member_number}" }
                                            td { class: "px-6 py-4", {member.last_name.clone()} }
                                            td { class: "px-6 py-4", {member.first_name.clone()} }
                                            td { class: "px-6 py-4", {member.city.clone().unwrap_or_default()} }
                                            td { class: "px-6 py-4", "{member.current_shares}" }
                                            td { class: "px-6 py-4", "{join_date}" }
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
