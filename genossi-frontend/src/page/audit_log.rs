use std::collections::HashMap;

use dioxus::prelude::*;
use rest_types::VerifyResponseTO;

use crate::api;
use crate::auth::RequirePrivilege;
use crate::component::{ErrorAlert, PageSizeSelect, PaginationControls, TimestampSection, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;

const DEFAULT_PAGE_SIZE: i64 = 50;

#[component]
pub fn AuditLogPage() -> Element {
    let i18n = use_i18n();
    let mut entries = use_signal(|| Vec::<rest_types::AuditLogEntryTO>::new());
    let mut total = use_signal(|| 0_i64);
    let mut current_page = use_signal(|| 0_i64);
    let mut page_size = use_signal(|| DEFAULT_PAGE_SIZE);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);
    let mut verify_result = use_signal(|| None::<VerifyResponseTO>);
    let mut verify_loading = use_signal(|| false);

    // Filter state
    let mut filter_entity_type = use_signal(|| String::new());
    let mut filter_user = use_signal(|| String::new());
    let mut filter_action = use_signal(|| String::new());
    let mut filter_from = use_signal(|| String::new());
    let mut filter_to = use_signal(|| String::new());

    let load_entries = move || {
        let entity_type = filter_entity_type.read().clone();
        let user = filter_user.read().clone();
        let action = filter_action.read().clone();
        let from = filter_from.read().clone();
        let to = filter_to.read().clone();
        let page = *current_page.read();
        let size = *page_size.read();

        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            let mut params = HashMap::new();
            if !entity_type.is_empty() {
                params.insert("entity_type".to_string(), entity_type);
            }
            if !user.is_empty() {
                params.insert("user_id".to_string(), user);
            }
            if !action.is_empty() {
                params.insert("action".to_string(), action);
            }
            if !from.is_empty() {
                params.insert("from".to_string(), from);
            }
            if !to.is_empty() {
                params.insert("to".to_string(), to);
            }
            match api::get_audit_log(&config, &params, page, size).await {
                Ok(envelope) => {
                    entries.set(envelope.entries);
                    total.set(envelope.total);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
            loading.set(false);
        });
    };

    use_effect({
        let load = load_entries.clone();
        move || {
            load();
        }
    });

    let on_verify = move |_| {
        spawn(async move {
            verify_loading.set(true);
            let config = CONFIG.read().clone();
            match api::verify_audit_chain(&config).await {
                Ok(result) => {
                    verify_result.set(Some(result));
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
            verify_loading.set(false);
        });
    };

    let on_filter = {
        let load = load_entries.clone();
        move |_| {
            // Filter change must reset to page 0 so users don't land on an
            // empty page beyond the new filtered total.
            current_page.set(0);
            load();
        }
    };

    let on_page_change = {
        let load = load_entries.clone();
        move |new_page: i64| {
            current_page.set(new_page);
            load();
        }
    };

    let on_size_change = {
        let load = load_entries.clone();
        move |new_size: i64| {
            page_size.set(new_size);
            current_page.set(0);
            load();
        }
    };

    let action_label = |action: &str| -> String {
        match action {
            "create" => i18n.t(Key::AuditActionCreate).to_string(),
            "update" => i18n.t(Key::AuditActionUpdate).to_string(),
            "delete" => i18n.t(Key::AuditActionDelete).to_string(),
            "snapshot" => i18n.t(Key::AuditActionSnapshot).to_string(),
            other => other.to_string(),
        }
    };

    let total_value = *total.read();
    let size_value = *page_size.read();
    let page_value = *current_page.read();
    let total_pages = if total_value <= 0 {
        1
    } else {
        (total_value + size_value - 1) / size_value
    };

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::AuditLog)}
                    }

                    // Verify button and result
                    div { class: "mb-6 flex items-center gap-4",
                        button {
                            class: "bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 disabled:opacity-50",
                            disabled: *verify_loading.read(),
                            onclick: on_verify,
                            {i18n.t(Key::AuditVerifyChain)}
                        }
                        if let Some(result) = verify_result.read().as_ref() {
                            if result.valid {
                                div { class: "bg-green-100 border border-green-400 text-green-700 px-4 py-2 rounded",
                                    "✓ {i18n.t(Key::AuditVerifySuccess)} ({i18n.t(Key::AuditTotalEntries)}: {result.total_entries})"
                                }
                            } else {
                                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-2 rounded",
                                    "✗ {i18n.t(Key::AuditVerifyFailure)} ({i18n.t(Key::AuditBrokenLinks)}: {result.broken_links.len()})"
                                }
                            }
                        }
                    }

                    // Qualified Timestamps
                    TimestampSection {}

                    // Filters
                    div { class: "bg-white rounded-lg shadow p-4 mb-6",
                        div { class: "grid grid-cols-2 md:grid-cols-5 gap-4",
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::AuditFilterEntityType)}
                                }
                                select {
                                    class: "w-full border rounded px-3 py-2",
                                    onchange: move |e| filter_entity_type.set(e.value()),
                                    option { value: "", "---" }
                                    option { value: "member", "Member" }
                                    option { value: "member_action", "MemberAction" }
                                    option { value: "member_document", "MemberDocument" }
                                    option { value: "application", "Application" }
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::AuditFilterUser)}
                                }
                                input {
                                    class: "w-full border rounded px-3 py-2",
                                    r#type: "text",
                                    oninput: move |e| filter_user.set(e.value()),
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::AuditFilterAction)}
                                }
                                select {
                                    class: "w-full border rounded px-3 py-2",
                                    onchange: move |e| filter_action.set(e.value()),
                                    option { value: "", "---" }
                                    option { value: "create", {i18n.t(Key::AuditActionCreate)} }
                                    option { value: "update", {i18n.t(Key::AuditActionUpdate)} }
                                    option { value: "delete", {i18n.t(Key::AuditActionDelete)} }
                                    option { value: "snapshot", {i18n.t(Key::AuditActionSnapshot)} }
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::AuditFilterFrom)}
                                }
                                input {
                                    class: "w-full border rounded px-3 py-2",
                                    r#type: "date",
                                    oninput: move |e| filter_from.set(e.value()),
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::AuditFilterTo)}
                                }
                                input {
                                    class: "w-full border rounded px-3 py-2",
                                    r#type: "date",
                                    oninput: move |e| filter_to.set(e.value()),
                                }
                            }
                        }
                        div { class: "mt-4",
                            button {
                                class: "bg-gray-600 text-white px-4 py-2 rounded hover:bg-gray-700",
                                onclick: on_filter,
                                {i18n.t(Key::Search)}
                            }
                        }
                    }

                    // Pagination toolbar (top): page-size + total + page-of-total
                    div { class: "bg-white rounded-lg shadow p-3 mb-4 flex flex-wrap items-center justify-between gap-4",
                        div { class: "flex items-center gap-4",
                            PageSizeSelect {
                                current_size: size_value,
                                on_size_change: on_size_change.clone(),
                            }
                            div { class: "text-sm text-gray-600",
                                "{i18n.t(Key::PageOfTotal)} {page_value + 1} / {total_pages} · {total_value} {i18n.t(Key::TotalEntries)}"
                            }
                        }
                        PaginationControls {
                            current_page: page_value,
                            total_pages: total_pages,
                            on_page_change: on_page_change.clone(),
                        }
                    }

                    // Content
                    if let Some(ref err) = *error.read() {
                        ErrorAlert {
                            error: err.clone(),
                            on_dismiss: move |_| error.set(None),
                        }
                    }
                    if entries.read().is_empty() {
                        if *loading.read() {
                            p { class: "text-gray-600", {i18n.t(Key::Loading)} }
                        } else {
                            p { class: "text-gray-500", {i18n.t(Key::AuditNoEntries)} }
                        }
                    } else {
                        // Keep the table + bottom pagination mounted across page
                        // transitions so the browser doesn't lose scroll position
                        // when the clicked button disappears. Dim the table while
                        // a new page is being fetched.
                        div {
                            class: if *loading.read() {
                                "bg-white rounded-lg shadow overflow-x-auto opacity-60 transition-opacity"
                            } else {
                                "bg-white rounded-lg shadow overflow-x-auto transition-opacity"
                            },
                            table { class: "w-full text-sm",
                                thead { tr { class: "border-b text-left bg-gray-50",
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditTimestamp)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditUser)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditAction)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditEntityType)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditEntityId)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditFieldName)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditOldValue)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::AuditNewValue)} }
                                }}
                                tbody {
                                    for entry in entries.read().iter() {
                                        {
                                            // Zebra stripe derived from transaction_id so a transaction
                                            // split across page boundaries keeps a single colour.
                                            let bg = if entry.transaction_id.as_bytes()[0] & 1 == 0 {
                                                ""
                                            } else {
                                                "bg-gray-50"
                                            };
                                            rsx! {
                                                tr { class: "border-b hover:bg-blue-50 {bg}",
                                                    td { class: "py-2 px-3 whitespace-nowrap text-xs",
                                                        {i18n.format_datetime_long(&entry.timestamp)}
                                                    }
                                                    td { class: "py-2 px-3", "{entry.user_id}" }
                                                    td { class: "py-2 px-3",
                                                        span {
                                                            class: match entry.action.as_str() {
                                                                "create" => "bg-green-100 text-green-800 px-2 py-0.5 rounded text-xs",
                                                                "update" => "bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded text-xs",
                                                                "delete" => "bg-red-100 text-red-800 px-2 py-0.5 rounded text-xs",
                                                                "snapshot" => "bg-blue-100 text-blue-800 px-2 py-0.5 rounded text-xs",
                                                                _ => "px-2 py-0.5 rounded text-xs",
                                                            },
                                                            {action_label(&entry.action)}
                                                        }
                                                    }
                                                    td { class: "py-2 px-3", "{entry.entity_type}" }
                                                    td { class: "py-2 px-3 text-xs font-mono",
                                                        {entry.entity_id.to_string().chars().take(8).collect::<String>()}
                                                        "..."
                                                    }
                                                    td { class: "py-2 px-3 font-medium", "{entry.field_name}" }
                                                    td { class: "py-2 px-3 text-gray-500 max-w-xs truncate",
                                                        {entry.old_value.as_deref().unwrap_or("-").to_string()}
                                                    }
                                                    td { class: "py-2 px-3 max-w-xs truncate",
                                                        {entry.new_value.as_deref().unwrap_or("-").to_string()}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Pagination toolbar (bottom): same controls duplicated
                        // for convenience after a long table
                        div { class: "mt-4 flex justify-end",
                            PaginationControls {
                                current_page: page_value,
                                total_pages: total_pages,
                                on_page_change: on_page_change.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}
