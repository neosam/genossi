use dioxus::prelude::*;
use rest_types::{ActionTypeTO, DocumentTypeTO, MemberActionTO, MemberDocumentTO, MemberTO, MigrationStatusTO};
use uuid::Uuid;

use crate::api::{self, FileTreeEntry};
use crate::component::{MemberSearch, Modal, TopBar};
use crate::i18n::use_i18n;
use crate::i18n::Key;
use crate::router::Route;
use crate::service::auth::AUTH;
use crate::service::config::CONFIG;

fn action_type_label(i18n: &crate::i18n::I18n, at: &ActionTypeTO) -> String {
    match at {
        ActionTypeTO::Eintritt => i18n.t(Key::ActionEintritt).to_string(),
        ActionTypeTO::Austritt => i18n.t(Key::ActionAustritt).to_string(),
        ActionTypeTO::Todesfall => i18n.t(Key::ActionTodesfall).to_string(),
        ActionTypeTO::Aufstockung => i18n.t(Key::ActionAufstockung).to_string(),
        ActionTypeTO::Verkauf => i18n.t(Key::ActionVerkauf).to_string(),
        ActionTypeTO::UebertragungEmpfang => i18n.t(Key::ActionUebertragungEmpfang).to_string(),
        ActionTypeTO::UebertragungAbgabe => i18n.t(Key::ActionUebertragungAbgabe).to_string(),
    }
}

fn format_date_input(d: &time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

fn parse_date_input(s: &str) -> Option<time::Date> {
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
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            created: None,
            deleted: None,
            version: None,
        }
    });
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut show_delete_modal = use_signal(|| false);

    // Actions state
    let mut actions = use_signal(|| Vec::<MemberActionTO>::new());
    let mut migration_status = use_signal(|| None::<MigrationStatusTO>);

    // Documents state
    let mut documents = use_signal(|| Vec::<MemberDocumentTO>::new());
    let mut show_upload_form = use_signal(|| false);
    let mut doc_type = use_signal(|| DocumentTypeTO::JoinDeclaration);
    let mut doc_description = use_signal(|| String::new());
    let mut show_action_form = use_signal(|| false);
    let mut editing_action = use_signal(|| None::<MemberActionTO>);

    // Action form state
    let mut action_type = use_signal(|| ActionTypeTO::Aufstockung);
    let mut action_date = use_signal(|| {
        let today = js_sys::Date::new_0();
        let year = today.get_full_year() as i32;
        let month: time::Month = (today.get_month() as u8 + 1).try_into().unwrap_or(time::Month::January);
        let day = today.get_date() as u8;
        time::Date::from_calendar_date(year, month, day)
            .unwrap_or_else(|_| time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
    });
    let mut action_shares_change = use_signal(|| 1_i32);
    let mut action_transfer_member_id = use_signal(|| String::new());
    let mut action_effective_date = use_signal(|| String::new());
    let mut action_comment = use_signal(|| String::new());

    // Generate document state
    let mut show_generate_doc = use_signal(|| false);
    let mut template_list = use_signal(|| Vec::<String>::new());

    // Load existing member + actions
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
                    // Load actions
                    match api::get_member_actions(&config, uuid).await {
                        Ok(data) => {
                            *actions.write() = data;
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load actions: {}", e)));
                        }
                    }
                    // Load documents
                    match api::get_member_documents(&config, uuid).await {
                        Ok(data) => {
                            *documents.write() = data;
                        }
                        Err(_) => {}
                    }
                    // Load migration status
                    match api::get_migration_status(&config, uuid).await {
                        Ok(data) => {
                            migration_status.set(Some(data));
                        }
                        Err(_) => {}
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
        show_delete_modal.set(true);
    };

    let confirm_delete = move |_| {
        spawn(async move {
            if let Some(id) = member.read().id {
                show_delete_modal.set(false);
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

    let save_action = move |_| {
        spawn(async move {
            let member_id = match member.read().id {
                Some(id) => id,
                None => return,
            };
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();

            let at = action_type.read().clone();
            let shares = if at.is_status_action() {
                0
            } else {
                *action_shares_change.read()
            };
            let transfer_id = if at.is_transfer() {
                action_transfer_member_id.read().parse::<Uuid>().ok()
            } else {
                None
            };
            let eff_date = if at == ActionTypeTO::Austritt {
                parse_date_input(&action_effective_date.read())
            } else {
                None
            };
            let comment_val = action_comment.read().clone();

            let action_to = MemberActionTO {
                id: editing_action.read().as_ref().and_then(|a| a.id),
                member_id,
                action_type: at,
                date: *action_date.read(),
                shares_change: shares,
                transfer_member_id: transfer_id,
                effective_date: eff_date,
                comment: if comment_val.is_empty() { None } else { Some(comment_val) },
                created: None,
                deleted: None,
                version: editing_action.read().as_ref().and_then(|a| a.version),
            };

            let result = if let Some(action_id) = action_to.id {
                api::update_member_action(&config, member_id, action_id, action_to).await
            } else {
                api::create_member_action(&config, member_id, action_to).await
            };

            match result {
                Ok(_) => {
                    // Refresh member data (join_date/exit_date may have changed)
                    if let Ok(data) = api::get_member(&config, member_id).await {
                        *member.write() = data;
                    }
                    // Refresh actions and migration status
                    if let Ok(data) = api::get_member_actions(&config, member_id).await {
                        *actions.write() = data;
                    }
                    if let Ok(data) = api::get_migration_status(&config, member_id).await {
                        migration_status.set(Some(data));
                    }
                    show_action_form.set(false);
                    editing_action.set(None);
                    action_comment.set(String::new());
                    action_transfer_member_id.set(String::new());
                    action_effective_date.set(String::new());
                }
                Err(e) => {
                    error.set(Some(format!("Failed to save action: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let current_action_type = action_type.read().clone();
    let is_status = current_action_type.is_status_action();
    let is_transfer = current_action_type.is_transfer();
    let is_austritt = current_action_type == ActionTypeTO::Austritt;

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

                // Migration Status Badge
                if !is_new {
                    if let Some(status) = migration_status.read().as_ref() {
                        div { class: "mb-4",
                            if status.status == "migrated" {
                                span { class: "inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-green-100 text-green-800",
                                    {i18n.t(Key::Migrated)}
                                }
                            } else {
                                div { class: "inline-flex flex-col gap-1 px-3 py-2 rounded-lg bg-orange-100 text-orange-800 text-sm",
                                    span { class: "font-medium", {i18n.t(Key::Pending)} }
                                    span {
                                        "{i18n.t(Key::ExpectedShares)}: {status.expected_shares} / {i18n.t(Key::ActualShares)}: {status.actual_shares}"
                                    }
                                    div { class: "flex items-center gap-2",
                                        span {
                                            "{i18n.t(Key::ExpectedActionCount)}: {status.expected_action_count} / {i18n.t(Key::ActualActionCount)}: {status.actual_action_count}"
                                        }
                                        if status.expected_action_count != status.actual_action_count {
                                            {
                                                let member_id_for_confirm = member.read().id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 text-xs font-medium rounded bg-orange-600 text-white hover:bg-orange-700",
                                                        onclick: move |_| {
                                                            if let Some(mid) = member_id_for_confirm {
                                                                spawn(async move {
                                                                    let config = CONFIG.read().clone();
                                                                    if api::confirm_migration(&config, mid).await.is_ok() {
                                                                        if let Ok(data) = api::get_migration_status(&config, mid).await {
                                                                            migration_status.set(Some(data));
                                                                        }
                                                                        if let Ok(data) = api::get_member(&config, mid).await {
                                                                            member.set(data);
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        {i18n.t(Key::ConfirmMigration)}
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

                    // Join date & Exit date
                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::JoinDate)}
                            }
                            if is_new {
                                input {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                    r#type: "date",
                                    value: "{format_date_input(&member.read().join_date)}",
                                    oninput: move |e| {
                                        if let Some(d) = parse_date_input(&e.value()) {
                                            member.write().join_date = d;
                                        }
                                    },
                                }
                            } else {
                                input {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md bg-gray-100 text-gray-600",
                                    r#type: "date",
                                    value: "{format_date_input(&member.read().join_date)}",
                                    readonly: true,
                                }
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::ExitDate)}
                            }
                            input {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md bg-gray-100 text-gray-600",
                                r#type: "date",
                                value: "{member.read().exit_date.as_ref().map(|d| format_date_input(d)).unwrap_or_default()}",
                                readonly: true,
                            }
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

                // === Delete Confirmation Modal ===
                if *show_delete_modal.read() {
                    Modal {
                        div { class: "space-y-4",
                            h2 { class: "text-xl font-bold text-red-600",
                                {i18n.t(Key::DeleteMemberConfirmTitle)}
                            }
                            p {
                                {i18n.t(Key::ConfirmDelete)}
                            }
                            p { class: "font-semibold",
                                {format!("{} {}", member.read().first_name, member.read().last_name)}
                            }
                            div { class: "flex justify-end gap-2 pt-4",
                                button {
                                    class: "px-4 py-2 bg-gray-200 text-gray-700 rounded hover:bg-gray-300",
                                    onclick: move |_| { show_delete_modal.set(false); },
                                    {i18n.t(Key::Cancel)}
                                }
                                button {
                                    class: "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700",
                                    onclick: confirm_delete,
                                    {i18n.t(Key::Confirm)}
                                }
                            }
                        }
                    }
                }

                // === Actions Section (only for existing members) ===
                if !is_new {
                    div { class: "mt-8",
                        div { class: "flex justify-between items-center mb-4",
                            h2 { class: "text-2xl font-bold", {i18n.t(Key::Actions)} }
                            button {
                                class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm",
                                onclick: move |_| {
                                    show_action_form.set(true);
                                    editing_action.set(None);
                                    action_type.set(ActionTypeTO::Aufstockung);
                                    action_shares_change.set(1);
                                    action_comment.set(String::new());
                                    action_transfer_member_id.set(String::new());
                                    action_effective_date.set(String::new());
                                },
                                {i18n.t(Key::NewAction)}
                            }
                        }

                        // Action Form (inline, collapsible)
                        if *show_action_form.read() {
                            div { class: "bg-white rounded-lg shadow p-6 mb-4 space-y-4 border-l-4 border-blue-500",
                                h3 { class: "text-lg font-semibold mb-2",
                                    if editing_action.read().is_some() { {i18n.t(Key::EditAction)} } else { {i18n.t(Key::NewAction)} }
                                }

                                // Action Type
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::ActionType)}
                                    }
                                    select {
                                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                        value: "{action_type.read().as_str()}",
                                        oninput: move |e| {
                                            if let Some(at) = ActionTypeTO::from_str(&e.value()) {
                                                action_type.set(at);
                                            }
                                        },
                                        for at in ActionTypeTO::all() {
                                            option {
                                                value: "{at.as_str()}",
                                                selected: *action_type.read() == *at,
                                                {action_type_label(&i18n, at)}
                                            }
                                        }
                                    }
                                }

                                // Date
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::Date)}
                                    }
                                    input {
                                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                        r#type: "date",
                                        value: "{format_date_input(&action_date.read())}",
                                        oninput: move |e| {
                                            if let Some(d) = parse_date_input(&e.value()) {
                                                action_date.set(d);
                                            }
                                        },
                                    }
                                }

                                // Shares Change (hidden for status actions)
                                if !is_status {
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SharesChange)}
                                        }
                                        input {
                                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                            r#type: "number",
                                            value: "{action_shares_change}",
                                            oninput: move |e| {
                                                action_shares_change.set(e.value().parse().unwrap_or(0));
                                            },
                                        }
                                    }
                                }

                                // Transfer Member ID (only for transfers)
                                if is_transfer {
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::TransferMember)}
                                        }
                                        MemberSearch {
                                            on_select: move |id: Option<Uuid>| {
                                                action_transfer_member_id.set(
                                                    id.map(|u| u.to_string()).unwrap_or_default()
                                                );
                                            },
                                            selected_id: action_transfer_member_id.read().parse::<Uuid>().ok(),
                                            exclude_id: member.read().id,
                                        }
                                    }
                                }

                                // Effective Date (only for Austritt)
                                if is_austritt {
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::EffectiveDate)}
                                        }
                                        input {
                                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                            r#type: "date",
                                            value: "{action_effective_date}",
                                            oninput: move |e| {
                                                action_effective_date.set(e.value().clone());
                                            },
                                        }
                                    }
                                }

                                // Comment
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::Comment)}
                                    }
                                    input {
                                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                        r#type: "text",
                                        value: "{action_comment}",
                                        oninput: move |e| {
                                            action_comment.set(e.value().clone());
                                        },
                                    }
                                }

                                // Form buttons
                                div { class: "flex gap-2 justify-end pt-2",
                                    button {
                                        class: "px-4 py-2 bg-gray-200 text-gray-700 rounded hover:bg-gray-300",
                                        onclick: move |_| {
                                            show_action_form.set(false);
                                            editing_action.set(None);
                                        },
                                        {i18n.t(Key::Cancel)}
                                    }
                                    button {
                                        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                        disabled: *loading.read(),
                                        onclick: save_action,
                                        {i18n.t(Key::Save)}
                                    }
                                }
                            }
                        }

                        // Actions List
                        if actions.read().is_empty() {
                            div { class: "bg-white rounded-lg shadow p-6 text-gray-500 text-center",
                                {i18n.t(Key::NoActions)}
                            }
                        } else {
                            div { class: "bg-white rounded-lg shadow overflow-hidden",
                                table { class: "min-w-full divide-y divide-gray-200",
                                    thead { class: "bg-gray-50",
                                        tr {
                                            th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::ActionType)} }
                                            th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::Date)} }
                                            th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::SharesChange)} }
                                            th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::Comment)} }
                                            th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", "" }
                                        }
                                    }
                                    tbody { class: "bg-white divide-y divide-gray-200",
                                        for action in actions.read().iter() {
                                            {
                                                let action_clone = action.clone();
                                                let action_for_edit = action.clone();
                                                let action_id_for_delete = action.id;
                                                rsx! {
                                                    tr {
                                                        class: "hover:bg-gray-50 cursor-pointer",
                                                        onclick: move |_| {
                                                            let a = action_for_edit.clone();
                                                            action_type.set(a.action_type.clone());
                                                            action_date.set(a.date);
                                                            action_shares_change.set(a.shares_change);
                                                            action_transfer_member_id.set(
                                                                a.transfer_member_id.map(|u| u.to_string()).unwrap_or_default()
                                                            );
                                                            action_effective_date.set(
                                                                a.effective_date.map(|d| format_date_input(&d)).unwrap_or_default()
                                                            );
                                                            action_comment.set(a.comment.clone().unwrap_or_default());
                                                            editing_action.set(Some(a));
                                                            show_action_form.set(true);
                                                        },
                                                        td { class: "px-4 py-3 text-sm", {action_type_label(&i18n, &action_clone.action_type)} }
                                                        td { class: "px-4 py-3 text-sm", {i18n.format_date(&action_clone.date)} }
                                                        td { class: "px-4 py-3 text-sm",
                                                            if action_clone.shares_change > 0 {
                                                                span { class: "text-green-600 font-medium", "+{action_clone.shares_change}" }
                                                            } else if action_clone.shares_change < 0 {
                                                                span { class: "text-red-600 font-medium", "{action_clone.shares_change}" }
                                                            } else {
                                                                span { class: "text-gray-400", "0" }
                                                            }
                                                        }
                                                        td { class: "px-4 py-3 text-sm text-gray-500",
                                                            {action_clone.comment.clone().unwrap_or_default()}
                                                        }
                                                        td { class: "px-4 py-3 text-sm",
                                                            button {
                                                                class: "text-red-600 hover:text-red-800 text-xs",
                                                                onclick: move |evt| {
                                                                    evt.stop_propagation();
                                                                    if let Some(aid) = action_id_for_delete {
                                                                        spawn(async move {
                                                                            let member_id = match member.read().id {
                                                                                Some(id) => id,
                                                                                None => return,
                                                                            };
                                                                            let config = CONFIG.read().clone();
                                                                            match api::delete_member_action(&config, member_id, aid).await {
                                                                                Ok(_) => {
                                                                                    if let Ok(data) = api::get_member(&config, member_id).await {
                                                                                        *member.write() = data;
                                                                                    }
                                                                                    if let Ok(data) = api::get_member_actions(&config, member_id).await {
                                                                                        *actions.write() = data;
                                                                                    }
                                                                                    if let Ok(data) = api::get_migration_status(&config, member_id).await {
                                                                                        migration_status.set(Some(data));
                                                                                    }
                                                                                }
                                                                                Err(e) => {
                                                                                    error.set(Some(format!("Failed to delete action: {}", e)));
                                                                                }
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                                {i18n.t(Key::Delete)}
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

                        // === Documents Section ===
                        div { class: "mt-8",
                            div { class: "flex justify-between items-center mb-4",
                                h2 { class: "text-2xl font-bold", {i18n.t(Key::Documents)} }
                                div { class: "flex gap-2",
                                    // Generate buttons for types that have template mappings and no existing document
                                    {
                                        let has_join_confirmation = documents.read().iter().any(|d| d.document_type == "join_confirmation");
                                        let gen_label = format!("{}: {}", i18n.t(Key::DocJoinConfirmation), i18n.t(Key::GenerateAndStore));
                                        let error_label = i18n.t(Key::DocJoinConfirmation).to_string();
                                        rsx! {
                                            if !has_join_confirmation {
                                                button {
                                                    class: "px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 text-sm",
                                                    onclick: move |_| {
                                                        let err_label = error_label.clone();
                                                        if let Some(member_id) = member.read().id {
                                                            spawn(async move {
                                                                let config = CONFIG.read().clone();
                                                                match api::generate_member_document(&config, member_id, "join_confirmation").await {
                                                                    Ok(_) => {
                                                                        if let Ok(data) = api::get_member_documents(&config, member_id).await {
                                                                            *documents.write() = data;
                                                                        }
                                                                    }
                                                                    Err(e) => error.set(Some(format!("{}: {}", err_label, e))),
                                                                }
                                                            });
                                                        }
                                                    },
                                                    "{gen_label}"
                                                }
                                            }
                                        }
                                    }
                                    button {
                                        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm",
                                        onclick: move |_| {
                                            show_upload_form.set(true);
                                            doc_type.set(DocumentTypeTO::JoinDeclaration);
                                            doc_description.set(String::new());
                                        },
                                        {i18n.t(Key::UploadDocument)}
                                    }
                                }
                            }

                            // Upload Form
                            if *show_upload_form.read() {
                                div { class: "bg-white rounded-lg shadow p-6 mb-4 space-y-4 border-l-4 border-green-500",
                                    h3 { class: "text-lg font-semibold mb-2", {i18n.t(Key::UploadDocument)} }

                                    // Document Type
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::DocumentType)}
                                        }
                                        select {
                                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                            value: "{doc_type.read().as_str()}",
                                            oninput: move |e| {
                                                if let Some(dt) = DocumentTypeTO::from_str(&e.value()) {
                                                    doc_type.set(dt);
                                                }
                                            },
                                            for dt in DocumentTypeTO::all() {
                                                {
                                                    let label = match dt {
                                                        DocumentTypeTO::JoinDeclaration => i18n.t(Key::DocJoinDeclaration),
                                                        DocumentTypeTO::JoinConfirmation => i18n.t(Key::DocJoinConfirmation),
                                                        DocumentTypeTO::ShareIncrease => i18n.t(Key::DocShareIncrease),
                                                        DocumentTypeTO::Other => i18n.t(Key::DocOther),
                                                    };
                                                    rsx! {
                                                        option {
                                                            value: "{dt.as_str()}",
                                                            selected: *doc_type.read() == *dt,
                                                            {label}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Description (shown for Other)
                                    if *doc_type.read() == DocumentTypeTO::Other {
                                        div {
                                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                                {i18n.t(Key::Description)}
                                            }
                                            input {
                                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                                r#type: "text",
                                                value: "{doc_description}",
                                                oninput: move |e| {
                                                    doc_description.set(e.value().clone());
                                                },
                                            }
                                        }
                                    }

                                    // File Input
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            "Datei"
                                        }
                                        input {
                                            class: "w-full px-3 py-2 border border-gray-300 rounded-md",
                                            r#type: "file",
                                            id: "document-file-input",
                                        }
                                    }

                                    // Form buttons
                                    div { class: "flex gap-2 justify-end pt-2",
                                        button {
                                            class: "px-4 py-2 bg-gray-200 text-gray-700 rounded hover:bg-gray-300",
                                            onclick: move |_| {
                                                show_upload_form.set(false);
                                            },
                                            {i18n.t(Key::Cancel)}
                                        }
                                        button {
                                            class: "px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700",
                                            disabled: *loading.read(),
                                            onclick: move |_| {
                                                spawn(async move {
                                                    let member_id = match member.read().id {
                                                        Some(id) => id,
                                                        None => return,
                                                    };
                                                    loading.set(true);
                                                    error.set(None);
                                                    let config = CONFIG.read().clone();
                                                    let dt = doc_type.read().as_str().to_string();
                                                    let desc = doc_description.read().clone();
                                                    let desc_opt = if desc.is_empty() { None } else { Some(desc.as_str()) };

                                                    // Get file from input
                                                    let window = web_sys::window().unwrap();
                                                    let document = window.document().unwrap();
                                                    let input = document
                                                        .get_element_by_id("document-file-input")
                                                        .and_then(|el| {
                                                            use wasm_bindgen::JsCast;
                                                            el.dyn_into::<web_sys::HtmlInputElement>().ok()
                                                        });
                                                    let file = input
                                                        .and_then(|inp| inp.files())
                                                        .and_then(|files| files.get(0));

                                                    match file {
                                                        Some(f) => {
                                                            match api::upload_member_document(&config, member_id, &dt, desc_opt, f).await {
                                                                Ok(_) => {
                                                                    if let Ok(data) = api::get_member_documents(&config, member_id).await {
                                                                        *documents.write() = data;
                                                                    }
                                                                    show_upload_form.set(false);
                                                                    doc_description.set(String::new());
                                                                }
                                                                Err(e) => {
                                                                    error.set(Some(format!("Upload failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        None => {
                                                            error.set(Some("No file selected".to_string()));
                                                        }
                                                    }
                                                    loading.set(false);
                                                });
                                            },
                                            {i18n.t(Key::Upload)}
                                        }
                                    }
                                }
                            }

                            // Documents List
                            if documents.read().is_empty() {
                                div { class: "bg-white rounded-lg shadow p-6 text-gray-500 text-center",
                                    {i18n.t(Key::NoDocuments)}
                                }
                            } else {
                                div { class: "bg-white rounded-lg shadow overflow-x-auto",
                                    table { class: "min-w-full divide-y divide-gray-200",
                                        thead { class: "bg-gray-50",
                                            tr {
                                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::DocumentType)} }
                                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::FileName)} }
                                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", {i18n.t(Key::Description)} }
                                                th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase", "" }
                                            }
                                        }
                                        tbody { class: "bg-white divide-y divide-gray-200",
                                            for doc in documents.read().iter() {
                                                {
                                                    let doc_clone = doc.clone();
                                                    let doc_id_for_delete = doc.id;
                                                    let doc_type_label = match DocumentTypeTO::from_str(&doc_clone.document_type) {
                                                        Some(DocumentTypeTO::JoinDeclaration) => i18n.t(Key::DocJoinDeclaration),
                                                        Some(DocumentTypeTO::JoinConfirmation) => i18n.t(Key::DocJoinConfirmation),
                                                        Some(DocumentTypeTO::ShareIncrease) => i18n.t(Key::DocShareIncrease),
                                                        Some(DocumentTypeTO::Other) => i18n.t(Key::DocOther),
                                                        None => doc_clone.document_type.clone().into(),
                                                    };
                                                    let download_url = {
                                                        let config = CONFIG.read().clone();
                                                        let member_id = member.read().id.unwrap_or_default();
                                                        let doc_id = doc_clone.id.unwrap_or_default();
                                                        api::document_download_url(&config, member_id, doc_id)
                                                    };
                                                    rsx! {
                                                        tr { class: "hover:bg-gray-50",
                                                            td { class: "px-4 py-3 text-sm", {doc_type_label} }
                                                            td { class: "px-4 py-3 text-sm break-all", {doc_clone.file_name.clone()} }
                                                            td { class: "px-4 py-3 text-sm text-gray-500",
                                                                {doc_clone.description.clone().unwrap_or_default()}
                                                            }
                                                            td { class: "px-4 py-3 text-sm flex gap-2 whitespace-nowrap",
                                                                a {
                                                                    class: "text-blue-600 hover:text-blue-800 text-xs",
                                                                    href: "{download_url}",
                                                                    target: "_blank",
                                                                    {i18n.t(Key::Download)}
                                                                }
                                                                button {
                                                                    class: "text-red-600 hover:text-red-800 text-xs",
                                                                    onclick: move |evt| {
                                                                        evt.stop_propagation();
                                                                        if let Some(did) = doc_id_for_delete {
                                                                            spawn(async move {
                                                                                let member_id = match member.read().id {
                                                                                    Some(id) => id,
                                                                                    None => return,
                                                                                };
                                                                                let config = CONFIG.read().clone();
                                                                                match api::delete_member_document(&config, member_id, did).await {
                                                                                    Ok(_) => {
                                                                                        if let Ok(data) = api::get_member_documents(&config, member_id).await {
                                                                                            *documents.write() = data;
                                                                                        }
                                                                                    }
                                                                                    Err(e) => {
                                                                                        error.set(Some(format!("Failed to delete document: {}", e)));
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    {i18n.t(Key::Delete)}
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

                        // === Generate Document Section ===
                        {
                            let is_board = AUTH.read().auth_info.as_ref()
                                .map(|a| a.has_privilege("manage_members") || a.has_privilege("admin"))
                                .unwrap_or(false);
                            rsx! {
                                if is_board && !is_new {
                                    div { class: "mt-8",
                                        div { class: "flex justify-between items-center mb-4",
                                            h2 { class: "text-2xl font-bold", {i18n.t(Key::GenerateDocument)} }
                                            button {
                                                class: "px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 text-sm",
                                                onclick: move |_| {
                                                    spawn(async move {
                                                        let config = CONFIG.read().clone();
                                                        if let Ok(entries) = api::get_templates(&config).await {
                                                            let paths = collect_template_paths(&entries);
                                                            template_list.set(paths);
                                                        }
                                                        show_generate_doc.set(true);
                                                    });
                                                },
                                                {i18n.t(Key::GenerateDocument)}
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

        // Generate Document modal
        if *show_generate_doc.read() {
            Modal {
                div { class: "space-y-4",
                    h2 { class: "text-xl font-bold", {i18n.t(Key::SelectTemplate)} }
                    if template_list.read().is_empty() {
                        p { class: "text-gray-500", {i18n.t(Key::NoTemplates)} }
                    } else {
                        div { class: "max-h-96 overflow-y-auto space-y-1",
                            for tpl_path in template_list.read().iter().cloned() {
                                {
                                    let path_for_click = tpl_path.clone();
                                    rsx! {
                                        button {
                                            class: "w-full text-left px-3 py-2 hover:bg-blue-50 rounded border text-sm font-mono",
                                            onclick: move |_| {
                                                if let Some(member_id) = member.read().id {
                                                    let path = path_for_click.clone();
                                                    spawn(async move {
                                                        let config = CONFIG.read().clone();
                                                        match api::render_template_pdf(&config, &path, member_id).await {
                                                            Ok(blob_url) => {
                                                                let window = web_sys::window().unwrap();
                                                                let _ = window.open_with_url_and_target(&blob_url, "_blank");
                                                            }
                                                            Err(e) => error.set(Some(format!("Render failed: {}", e))),
                                                        }
                                                    });
                                                }
                                                show_generate_doc.set(false);
                                            },
                                            "{tpl_path}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex justify-end pt-2",
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded hover:bg-gray-400",
                            onclick: move |_| show_generate_doc.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }
    }
}

fn collect_template_paths(entries: &[FileTreeEntry]) -> Vec<String> {
    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            FileTreeEntry::File { path, .. } => {
                if !path.starts_with('_') {
                    paths.push(path.clone());
                }
            }
            FileTreeEntry::Directory { children, .. } => {
                paths.extend(collect_template_paths(children));
            }
        }
    }
    paths
}
