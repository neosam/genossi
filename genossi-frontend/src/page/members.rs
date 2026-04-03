use dioxus::prelude::*;

use crate::api::{self, MailJobTO};
use crate::component::TopBar;
use crate::i18n::use_i18n;
use crate::i18n::Key;
use crate::member_utils::{exited_in_year, is_active, today};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::member::{refresh_members, MEMBERS, SELECTED_MEMBER_IDS};

fn format_date_iso(date: &time::Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

fn parse_date_iso(s: &str) -> Option<time::Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u8 = parts[1].parse().ok()?;
    let day: u8 = parts[2].parse().ok()?;
    let month: time::Month = month.try_into().ok()?;
    time::Date::from_calendar_date(year, month, day).ok()
}

#[component]
pub fn Members() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    let mut reference_date = use_signal(today);
    let mut only_active = use_signal(|| true);
    let mut filter_exited_in_year = use_signal(|| false);
    let mut only_pending_migration = use_signal(|| false);

    // Mail job filter
    let mut mail_jobs: Signal<Vec<MailJobTO>> = use_signal(Vec::new);
    let mut selected_mail_job: Signal<Option<String>> = use_signal(|| None);
    let mut not_reached_members: Signal<Option<Vec<rest_types::MemberTO>>> = use_signal(|| None);
    let mut not_reached_loading = use_signal(|| false);

    // Load mail jobs on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if !config.backend.is_empty() {
                if let Ok(jobs) = api::get_mail_jobs(&config).await {
                    mail_jobs.set(jobs);
                }
            }
        });
    });

    let members_state = MEMBERS.read();
    let filter_query = members_state.filter_query.clone();
    let ref_date = *reference_date.read();
    let show_only_active = *only_active.read();
    let show_exited_in_year = *filter_exited_in_year.read();
    let show_only_pending_migration = *only_pending_migration.read();

    // Use not-reached members if a mail job filter is active, otherwise normal list
    let base_members: Vec<_> = if let Some(ref nr_members) = *not_reached_members.read() {
        nr_members.clone()
    } else {
        members_state.items.clone()
    };

    let filtered_members: Vec<_> = base_members
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
        .filter(|m| {
            if show_only_active {
                is_active(m, &ref_date)
            } else {
                true
            }
        })
        .filter(|m| {
            if show_exited_in_year {
                exited_in_year(m, &ref_date)
            } else {
                true
            }
        })
        .filter(|m| {
            if show_only_pending_migration {
                !m.migrated
            } else {
                true
            }
        })
        .collect();

    let selection = SELECTED_MEMBER_IDS.read();
    let selected_count = selection.count();

    // Determine header checkbox state
    let filtered_ids: Vec<_> = filtered_members.iter().filter_map(|m| m.id).collect();
    let all_filtered_selected = !filtered_ids.is_empty()
        && filtered_ids.iter().all(|id| selection.is_selected(id));

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

            // Reference date + active filter
            div { class: "mb-4 flex items-center gap-4 flex-wrap",
                div { class: "flex items-center gap-2",
                    label { class: "text-sm font-medium text-gray-700",
                        {i18n.t(Key::ReferenceDate)}
                    }
                    input {
                        class: "px-3 py-1.5 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm",
                        r#type: "date",
                        value: "{format_date_iso(&ref_date)}",
                        oninput: move |e| {
                            if let Some(d) = parse_date_iso(&e.value()) {
                                reference_date.set(d);
                            }
                        },
                    }
                }
                label { class: "flex items-center gap-2 text-sm text-gray-700 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                        checked: show_only_active,
                        oninput: move |e| {
                            only_active.set(e.value() == "true");
                        },
                    }
                    {i18n.t(Key::OnlyActiveMembers)}
                }
                label { class: "flex items-center gap-2 text-sm text-gray-700 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                        checked: show_exited_in_year,
                        oninput: move |e| {
                            filter_exited_in_year.set(e.value() == "true");
                        },
                    }
                    {format!("{} {}", i18n.t(Key::ExitedInYear), ref_date.year())}
                }
                label { class: "flex items-center gap-2 text-sm text-gray-700 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                        checked: show_only_pending_migration,
                        oninput: move |e| {
                            only_pending_migration.set(e.value() == "true");
                        },
                    }
                    {i18n.t(Key::OnlyPendingMigration)}
                }
                if !mail_jobs.read().is_empty() {
                    div { class: "flex items-center gap-2",
                        label { class: "text-sm font-medium text-gray-700",
                            {i18n.t(Key::NotReachedByMailJob)}
                        }
                        select {
                            class: "px-3 py-1.5 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm",
                            value: selected_mail_job.read().clone().unwrap_or_else(|| "all".to_string()),
                            onchange: move |e| {
                                let val = e.value();
                                if val == "all" {
                                    selected_mail_job.set(None);
                                    not_reached_members.set(None);
                                } else {
                                    selected_mail_job.set(Some(val.clone()));
                                    not_reached_loading.set(true);
                                    spawn(async move {
                                        let config = CONFIG.read().clone();
                                        match api::get_members_not_reached_by(&config, &val).await {
                                            Ok(members) => {
                                                not_reached_members.set(Some(members));
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to load not-reached members: {e}");
                                                not_reached_members.set(None);
                                            }
                                        }
                                        not_reached_loading.set(false);
                                    });
                                }
                            },
                            option { value: "all", {i18n.t(Key::AllMembers)} }
                            for job in mail_jobs.read().iter() {
                                option {
                                    value: "{job.id}",
                                    {format!("{} ({})", job.subject, job.created.chars().take(10).collect::<String>())}
                                }
                            }
                        }
                    }
                }
            }

            // Selection action bar
            if selected_count > 0 {
                div { class: "mb-4 flex items-center gap-4 bg-blue-50 border border-blue-200 rounded-lg px-4 py-3",
                    span { class: "text-sm font-medium text-blue-800",
                        "{selected_count} {i18n.t(Key::SelectedCount)}"
                    }
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm font-medium",
                        onclick: move |_| {
                            nav.push(Route::MailPage {});
                        },
                        "✉ {i18n.t(Key::SendMailToSelected)}"
                    }
                    button {
                        class: "px-3 py-2 text-sm text-gray-600 hover:text-gray-800",
                        onclick: move |_| {
                            SELECTED_MEMBER_IDS.write().clear();
                        },
                        {i18n.t(Key::Cancel)}
                    }
                }
            }

            if *not_reached_loading.read() {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::Loading)}
                }
            } else if members_state.loading {
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
                                th { class: "px-3 py-3 w-12",
                                    {
                                        let filtered_ids_clone = filtered_ids.clone();
                                        rsx! {
                                            div {
                                                class: "flex items-center justify-center min-w-[44px] min-h-[44px] cursor-pointer",
                                                onclick: move |_| {
                                                    let mut sel = SELECTED_MEMBER_IDS.write();
                                                    if all_filtered_selected {
                                                        for id in &filtered_ids_clone {
                                                            sel.selected_ids.retain(|i| i != id);
                                                        }
                                                    } else {
                                                        for id in &filtered_ids_clone {
                                                            if !sel.is_selected(id) {
                                                                sel.selected_ids.push(*id);
                                                            }
                                                        }
                                                    }
                                                },
                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-5 h-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500 pointer-events-none",
                                                    checked: all_filtered_selected,
                                                }
                                            }
                                        }
                                    }
                                }
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
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ExitDate)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MigrationStatus)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Active)}
                                }
                            }
                        }
                        tbody {
                            for member in filtered_members.iter() {
                                {
                                    let member_id = member.id;
                                    let join_date = i18n.format_date(&member.join_date);
                                    let active = is_active(member, &ref_date);
                                    let is_checked = member_id.map(|id| selection.is_selected(&id)).unwrap_or(false);
                                    rsx! {
                                        tr {
                                            class: if is_checked { "border-b hover:bg-blue-50 cursor-pointer bg-blue-50" } else { "border-b hover:bg-gray-50 cursor-pointer" },
                                            onclick: move |_| {
                                                if let Some(id) = member_id {
                                                    nav.push(Route::MemberDetails { id: id.to_string() });
                                                }
                                            },
                                            td {
                                                class: "px-3 py-2",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    if let Some(id) = member_id {
                                                        SELECTED_MEMBER_IDS.write().toggle(id);
                                                    }
                                                },
                                                div { class: "flex items-center justify-center min-w-[44px] min-h-[44px]",
                                                    input {
                                                        r#type: "checkbox",
                                                        class: "w-5 h-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500 pointer-events-none",
                                                        checked: is_checked,
                                                    }
                                                }
                                            }
                                            td { class: "px-6 py-4 font-medium", "{member.member_number}" }
                                            td { class: "px-6 py-4", {member.last_name.clone()} }
                                            td { class: "px-6 py-4", {member.first_name.clone()} }
                                            td { class: "px-6 py-4", {member.city.clone().unwrap_or_default()} }
                                            td { class: "px-6 py-4", "{member.current_shares}" }
                                            td { class: "px-6 py-4", "{join_date}" }
                                            td { class: "px-6 py-4",
                                                {member.exit_date.as_ref().map(|d| i18n.format_date(d)).unwrap_or_default()}
                                            }
                                            td { class: "px-6 py-4",
                                                if member.migrated {
                                                    span { class: "px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800",
                                                        {i18n.t(Key::Migrated)}
                                                    }
                                                } else {
                                                    span { class: "px-2 py-1 text-xs font-semibold rounded-full bg-orange-100 text-orange-800",
                                                        {i18n.t(Key::Pending)}
                                                    }
                                                }
                                            }
                                            td { class: "px-6 py-4",
                                                if active {
                                                    span { class: "px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800",
                                                        {i18n.t(Key::Active)}
                                                    }
                                                } else {
                                                    span { class: "px-2 py-1 text-xs font-semibold rounded-full bg-red-100 text-red-800",
                                                        {i18n.t(Key::Inactive)}
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
    }
}
