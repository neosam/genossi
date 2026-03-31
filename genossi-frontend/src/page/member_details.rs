use dioxus::prelude::*;
use rest_types::MemberTO;
use uuid::Uuid;

use crate::api;
use crate::component::TopBar;
use crate::i18n::use_i18n;
use crate::i18n::Key;
use crate::router::Route;
use crate::service::config::CONFIG;

#[component]
pub fn MemberDetails(id: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let is_new = id == "new";

    let mut member = use_signal(|| {
        let today = js_sys::Date::new_0();
        let year = today.get_full_year() as i32;
        let month: time::Month = (today.get_month() as u8 + 1).try_into().unwrap_or(time::Month::January);
        let day = today.get_date() as u8;
        let join_date = time::Date::from_calendar_date(year, month, day)
            .unwrap_or_else(|_| time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap());

        MemberTO {
            id: None,
            member_number: 0,
            first_name: String::new(),
            last_name: String::new(),
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            exit_date: None,
            bank_account: None,
            created: None,
            deleted: None,
            version: None,
        }
    });
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load existing member
    use_effect(move || {
        if !is_new {
            if let Ok(uuid) = id.parse::<Uuid>() {
                spawn(async move {
                    loading.set(true);
                    let config = CONFIG.read().clone();
                    match api::get_member(&config, uuid).await {
                        Ok(data) => {
                            *member.write() = data;
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load member: {}", e)));
                        }
                    }
                    loading.set(false);
                });
            }
        }
    });

    let save = move |_| {
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            let data = member.read().clone();
            let result = if data.id.is_some() {
                api::update_member(&config, data).await
            } else {
                api::create_member(&config, data).await
            };
            match result {
                Ok(_) => {
                    nav.push(Route::Members {});
                }
                Err(e) => {
                    error.set(Some(format!("Failed to save: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let delete = move |_| {
        spawn(async move {
            if let Some(id) = member.read().id {
                loading.set(true);
                let config = CONFIG.read().clone();
                match api::delete_member(&config, id).await {
                    Ok(_) => {
                        nav.push(Route::Members {});
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to delete: {}", e)));
                    }
                }
                loading.set(false);
            }
        });
    };

    rsx! {
        TopBar {}
        div { class: "container mx-auto px-4 py-8",
            div { class: "max-w-2xl mx-auto",
                div { class: "flex justify-between items-center mb-6",
                    h1 { class: "text-3xl font-bold",
                        if is_new { {i18n.t(Key::CreateMember)} } else { {i18n.t(Key::EditMember)} }
                    }
                    button {
                        class: "px-4 py-2 text-gray-600 hover:text-gray-800",
                        onclick: move |_| { nav.push(Route::Members {}); },
                        {i18n.t(Key::Back)}
                    }
                }

                if let Some(err) = error.read().as_ref() {
                    div { class: "mb-4 p-4 bg-red-100 text-red-700 rounded",
                        "{err}"
                    }
                }

                div { class: "bg-white rounded-lg shadow p-6 space-y-4",
                    // Member Number
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::MemberNumber)}
                        }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                            r#type: "number",
                            value: "{member.read().member_number}",
                            oninput: move |e| {
                                member.write().member_number = e.value().parse().unwrap_or(0);
                            },
                        }
                    }

                    // Name row
                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::FirstName)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                value: "{member.read().first_name}",
                                oninput: move |e| { member.write().first_name = e.value().clone(); },
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::LastName)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                value: "{member.read().last_name}",
                                oninput: move |e| { member.write().last_name = e.value().clone(); },
                            }
                        }
                    }

                    // Email & Company
                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::Email)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "email",
                                value: "{member.read().email.clone().unwrap_or_default()}",
                                oninput: move |e| {
                                    let val = e.value();
                                    member.write().email = if val.is_empty() { None } else { Some(val.clone()) };
                                },
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::Company)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                value: "{member.read().company.clone().unwrap_or_default()}",
                                oninput: move |e| {
                                    let val = e.value();
                                    member.write().company = if val.is_empty() { None } else { Some(val.clone()) };
                                },
                            }
                        }
                    }

                    // Address
                    div { class: "grid grid-cols-4 gap-4",
                        div { class: "col-span-2",
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::Street)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                value: "{member.read().street.clone().unwrap_or_default()}",
                                oninput: move |e| {
                                    let val = e.value();
                                    member.write().street = if val.is_empty() { None } else { Some(val.clone()) };
                                },
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::HouseNumber)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                value: "{member.read().house_number.clone().unwrap_or_default()}",
                                oninput: move |e| {
                                    let val = e.value();
                                    member.write().house_number = if val.is_empty() { None } else { Some(val.clone()) };
                                },
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::PostalCode)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                value: "{member.read().postal_code.clone().unwrap_or_default()}",
                                oninput: move |e| {
                                    let val = e.value();
                                    member.write().postal_code = if val.is_empty() { None } else { Some(val.clone()) };
                                },
                            }
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::City)}
                        }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            value: "{member.read().city.clone().unwrap_or_default()}",
                            oninput: move |e| {
                                let val = e.value();
                                member.write().city = if val.is_empty() { None } else { Some(val.clone()) };
                            },
                        }
                    }

                    // Cooperative details
                    div { class: "grid grid-cols-3 gap-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::SharesAtJoining)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "number",
                                value: "{member.read().shares_at_joining}",
                                oninput: move |e| {
                                    member.write().shares_at_joining = e.value().parse().unwrap_or(0);
                                },
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::CurrentShares)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "number",
                                value: "{member.read().current_shares}",
                                oninput: move |e| {
                                    member.write().current_shares = e.value().parse().unwrap_or(0);
                                },
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::CurrentBalance)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                r#type: "number",
                                value: "{member.read().current_balance}",
                                oninput: move |e| {
                                    member.write().current_balance = e.value().parse().unwrap_or(0);
                                },
                            }
                        }
                    }

                    // Bank account
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::BankAccount)}
                        }
                        input {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            placeholder: "DE89 3704 0044 0532 0130 00",
                            value: "{member.read().bank_account.clone().unwrap_or_default()}",
                            oninput: move |e| {
                                let val = e.value();
                                member.write().bank_account = if val.is_empty() { None } else { Some(val.clone()) };
                            },
                        }
                    }

                    // Comment
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::Comment)}
                        }
                        textarea {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                            rows: "3",
                            value: "{member.read().comment.clone().unwrap_or_default()}",
                            oninput: move |e| {
                                let val = e.value();
                                member.write().comment = if val.is_empty() { None } else { Some(val.clone()) };
                            },
                        }
                    }

                    // Action buttons
                    div { class: "flex justify-between pt-4",
                        if !is_new {
                            button {
                                class: "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700",
                                onclick: delete,
                                {i18n.t(Key::Delete)}
                            }
                        }
                        div { class: "flex gap-2 ml-auto",
                            button {
                                class: "px-4 py-2 bg-gray-200 text-gray-700 rounded hover:bg-gray-300",
                                onclick: move |_| { nav.push(Route::Members {}); },
                                {i18n.t(Key::Cancel)}
                            }
                            button {
                                class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                disabled: *loading.read(),
                                onclick: save,
                                {i18n.t(Key::Save)}
                            }
                        }
                    }
                }
            }
        }
    }
}
