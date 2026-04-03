use dioxus::prelude::*;

use crate::api::{self, ConfigEntryTO};
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;

fn get_config_value(entries: &[ConfigEntryTO], key: &str) -> String {
    entries
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.clone())
        .unwrap_or_default()
}

fn has_config_key(entries: &[ConfigEntryTO], key: &str) -> bool {
    entries.iter().any(|e| e.key == key)
}

#[component]
pub fn ConfigPage() -> Element {
    let i18n = use_i18n();
    let mut entries = use_signal(|| Vec::<ConfigEntryTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut success_msg = use_signal(|| None::<String>);

    // SMTP form state
    let mut smtp_host = use_signal(|| String::new());
    let mut smtp_port = use_signal(|| "587".to_string());
    let mut smtp_tls = use_signal(|| "starttls".to_string());
    let mut smtp_user = use_signal(|| String::new());
    let mut smtp_pass = use_signal(|| String::new());
    let mut smtp_from = use_signal(|| String::new());
    let mut smtp_from_name = use_signal(|| String::new());
    let mut smtp_pass_set = use_signal(|| false);
    let mut smtp_saving = use_signal(|| false);

    // Test mail state
    let mut test_address = use_signal(|| String::new());
    let mut test_sending = use_signal(|| false);

    // New entry form state
    let mut new_key = use_signal(|| String::new());
    let mut new_value = use_signal(|| String::new());
    let mut new_value_type = use_signal(|| "string".to_string());
    let mut show_add_form = use_signal(|| false);
    let mut saving = use_signal(|| false);

    // Edit state
    let mut editing_key = use_signal(|| None::<String>);
    let mut edit_value = use_signal(|| String::new());
    let mut edit_value_type = use_signal(|| String::new());

    // Advanced config collapsed state
    let mut show_advanced = use_signal(|| false);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_config_entries(&config).await {
                Ok(data) => {
                    // Populate SMTP form from entries
                    smtp_host.set(get_config_value(&data, "smtp_host"));
                    let port_val = get_config_value(&data, "smtp_port");
                    if !port_val.is_empty() {
                        smtp_port.set(port_val);
                    }
                    let tls_val = get_config_value(&data, "smtp_tls");
                    if !tls_val.is_empty() {
                        smtp_tls.set(tls_val);
                    }
                    smtp_user.set(get_config_value(&data, "smtp_user"));
                    smtp_from.set(get_config_value(&data, "smtp_from"));
                    smtp_from_name.set(get_config_value(&data, "smtp_from_name"));
                    smtp_pass_set.set(has_config_key(&data, "smtp_pass"));
                    smtp_pass.set(String::new());

                    entries.set(data);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::Config)}
                    }

                    // Success message
                    if let Some(msg) = success_msg.read().as_ref() {
                        div { class: "bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4",
                            "{msg}"
                        }
                    }

                    // Error message
                    if let Some(err) = error.read().as_ref() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                            "{err}"
                        }
                    }

                    if *loading.read() {
                        p { class: "text-gray-600", {i18n.t(Key::Loading)} }
                    } else {
                        // SMTP Settings Section
                        div { class: "bg-white rounded-lg shadow p-6 mb-6",
                            h2 { class: "text-xl font-semibold mb-4", {i18n.t(Key::SmtpSettings)} }
                            div { class: "space-y-4",
                                // Host + Port row
                                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                                    div { class: "md:col-span-2",
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SmtpHost)}
                                        }
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "text",
                                            placeholder: "mail.example.com",
                                            value: "{smtp_host}",
                                            oninput: move |e| smtp_host.set(e.value()),
                                        }
                                    }
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SmtpPort)}
                                        }
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "number",
                                            placeholder: "587",
                                            value: "{smtp_port}",
                                            oninput: move |e| smtp_port.set(e.value()),
                                        }
                                    }
                                }

                                // Encryption
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                                        {i18n.t(Key::SmtpEncryption)}
                                    }
                                    div { class: "flex space-x-6",
                                        label { class: "inline-flex items-center",
                                            input {
                                                r#type: "radio",
                                                name: "smtp_tls",
                                                class: "mr-2",
                                                value: "none",
                                                checked: *smtp_tls.read() == "none",
                                                onchange: move |_| smtp_tls.set("none".to_string()),
                                            }
                                            {i18n.t(Key::SmtpEncryptionNone)}
                                        }
                                        label { class: "inline-flex items-center",
                                            input {
                                                r#type: "radio",
                                                name: "smtp_tls",
                                                class: "mr-2",
                                                value: "starttls",
                                                checked: *smtp_tls.read() == "starttls",
                                                onchange: move |_| smtp_tls.set("starttls".to_string()),
                                            }
                                            {i18n.t(Key::SmtpEncryptionStarttls)}
                                        }
                                        label { class: "inline-flex items-center",
                                            input {
                                                r#type: "radio",
                                                name: "smtp_tls",
                                                class: "mr-2",
                                                value: "tls",
                                                checked: *smtp_tls.read() == "tls",
                                                onchange: move |_| smtp_tls.set("tls".to_string()),
                                            }
                                            {i18n.t(Key::SmtpEncryptionTls)}
                                        }
                                    }
                                }

                                // Username + Password row
                                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SmtpUser)}
                                        }
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "text",
                                            value: "{smtp_user}",
                                            oninput: move |e| smtp_user.set(e.value()),
                                        }
                                    }
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SmtpPassword)}
                                        }
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "password",
                                            placeholder: if *smtp_pass_set.read() { "********" } else { "" },
                                            value: "{smtp_pass}",
                                            oninput: move |e| smtp_pass.set(e.value()),
                                        }
                                    }
                                }

                                // From name + address
                                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SmtpFromName)}
                                        }
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "text",
                                            placeholder: "Mein Verein e.V.",
                                            value: "{smtp_from_name}",
                                            oninput: move |e| smtp_from_name.set(e.value()),
                                        }
                                    }
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-1",
                                            {i18n.t(Key::SmtpFrom)}
                                        }
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "email",
                                            placeholder: "noreply@example.com",
                                            value: "{smtp_from}",
                                            oninput: move |e| smtp_from.set(e.value()),
                                        }
                                    }
                                }

                                // Save button
                                div { class: "flex items-center space-x-4 pt-2",
                                    button {
                                        class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                        disabled: *smtp_saving.read() || smtp_host.read().is_empty(),
                                        onclick: {
                                            let i18n = i18n.clone();
                                            move |_| {
                                            let host = smtp_host.read().clone();
                                            let port = smtp_port.read().clone();
                                            let tls = smtp_tls.read().clone();
                                            let user = smtp_user.read().clone();
                                            let pass = smtp_pass.read().clone();
                                            let from = smtp_from.read().clone();
                                            let from_name = smtp_from_name.read().clone();
                                            let pass_set = *smtp_pass_set.read();
                                            let i18n = i18n.clone();
                                            spawn(async move {
                                                smtp_saving.set(true);
                                                error.set(None);
                                                success_msg.set(None);
                                                let config = CONFIG.read().clone();
                                                let mut all_ok = true;

                                                let entries_to_save: Vec<(&str, String, &str)> = {
                                                    let mut v = vec![
                                                        ("smtp_host", host, "string"),
                                                        ("smtp_port", port, "int"),
                                                        ("smtp_tls", tls, "string"),
                                                        ("smtp_user", user, "string"),
                                                        ("smtp_from", from, "string"),
                                                        ("smtp_from_name", from_name, "string"),
                                                    ];
                                                    // Only save password if user entered a new one
                                                    if !pass.is_empty() || !pass_set {
                                                        v.push(("smtp_pass", pass, "secret"));
                                                    }
                                                    v
                                                };

                                                for (key, value, vtype) in &entries_to_save {
                                                    if let Err(e) = api::set_config_entry(&config, key, value, vtype).await {
                                                        error.set(Some(format!("{}", e)));
                                                        all_ok = false;
                                                        break;
                                                    }
                                                }

                                                if all_ok {
                                                    success_msg.set(Some(i18n.t(Key::SmtpTestSuccess).to_string()));
                                                    reload();
                                                }
                                                smtp_saving.set(false);
                                            });
                                        }},
                                        if *smtp_saving.read() {
                                            {i18n.t(Key::SmtpSaving)}
                                        } else {
                                            {i18n.t(Key::Save)}
                                        }
                                    }
                                }

                                // Test mail section
                                div { class: "border-t pt-4 mt-4",
                                    h3 { class: "text-sm font-medium text-gray-700 mb-2",
                                        {i18n.t(Key::SmtpTestMail)}
                                    }
                                    div { class: "flex space-x-2",
                                        div { class: "flex-1",
                                            input {
                                                class: "w-full border rounded px-3 py-2",
                                                r#type: "email",
                                                placeholder: "test@example.com",
                                                value: "{test_address}",
                                                oninput: move |e| test_address.set(e.value()),
                                            }
                                        }
                                        button {
                                            class: "bg-gray-500 hover:bg-gray-600 text-white px-4 py-2 rounded disabled:opacity-50 whitespace-nowrap",
                                            disabled: *test_sending.read() || test_address.read().is_empty(),
                                            onclick: {
                                                let i18n = i18n.clone();
                                                move |_| {
                                                let addr = test_address.read().clone();
                                                let i18n = i18n.clone();
                                                spawn(async move {
                                                    test_sending.set(true);
                                                    error.set(None);
                                                    success_msg.set(None);
                                                    let config = CONFIG.read().clone();
                                                    match api::send_test_mail(&config, &addr).await {
                                                        Ok(()) => {
                                                            success_msg.set(Some(i18n.t(Key::SmtpTestSuccess).to_string()));
                                                        }
                                                        Err(e) => {
                                                            error.set(Some(format!("{}: {}", i18n.t(Key::SmtpTestFailed), e)));
                                                        }
                                                    }
                                                    test_sending.set(false);
                                                });
                                            }},
                                            {i18n.t(Key::SmtpTestMail)}
                                        }
                                    }
                                }
                            }
                        }

                        // Advanced Configuration (collapsible)
                        div { class: "bg-white rounded-lg shadow mb-6",
                            button {
                                class: "w-full flex items-center justify-between p-6 text-left",
                                onclick: move |_| {
                                    let current = *show_advanced.read();
                                    show_advanced.set(!current);
                                },
                                h2 { class: "text-xl font-semibold",
                                    {i18n.t(Key::AdvancedConfig)}
                                }
                                span { class: "text-gray-400 text-xl",
                                    if *show_advanced.read() { "\u{25B2}" } else { "\u{25BC}" }
                                }
                            }

                            if *show_advanced.read() {
                                div { class: "px-6 pb-6",
                                    // Add entry button
                                    div { class: "mb-4",
                                        if !*show_add_form.read() {
                                            button {
                                                class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded",
                                                onclick: move |_| show_add_form.set(true),
                                                {i18n.t(Key::ConfigAddEntry)}
                                            }
                                        }
                                    }

                                    // Add entry form
                                    if *show_add_form.read() {
                                        div { class: "bg-gray-50 rounded-lg p-4 mb-4",
                                            h3 { class: "text-lg font-semibold mb-4", {i18n.t(Key::ConfigAddEntry)} }
                                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4 mb-4",
                                                div {
                                                    label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::ConfigKey)} }
                                                    input {
                                                        class: "w-full border rounded px-3 py-2",
                                                        r#type: "text",
                                                        value: "{new_key}",
                                                        oninput: move |e| new_key.set(e.value()),
                                                    }
                                                }
                                                div {
                                                    label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::ConfigValue)} }
                                                    input {
                                                        class: "w-full border rounded px-3 py-2",
                                                        r#type: if *new_value_type.read() == "secret" { "password" } else { "text" },
                                                        value: "{new_value}",
                                                        oninput: move |e| new_value.set(e.value()),
                                                    }
                                                }
                                                div {
                                                    label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::ConfigValueType)} }
                                                    select {
                                                        class: "w-full border rounded px-3 py-2",
                                                        value: "{new_value_type}",
                                                        onchange: move |e| new_value_type.set(e.value()),
                                                        option { value: "string", {i18n.t(Key::ConfigTypeString)} }
                                                        option { value: "int", {i18n.t(Key::ConfigTypeInt)} }
                                                        option { value: "bool", {i18n.t(Key::ConfigTypeBool)} }
                                                        option { value: "secret", {i18n.t(Key::ConfigTypeSecret)} }
                                                    }
                                                }
                                            }
                                            div { class: "flex space-x-2",
                                                button {
                                                    class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                                                    disabled: *saving.read() || new_key.read().is_empty(),
                                                    onclick: move |_| {
                                                        let key = new_key.read().clone();
                                                        let value = new_value.read().clone();
                                                        let vtype = new_value_type.read().clone();
                                                        spawn(async move {
                                                            saving.set(true);
                                                            let config = CONFIG.read().clone();
                                                            match api::set_config_entry(&config, &key, &value, &vtype).await {
                                                                Ok(_) => {
                                                                    new_key.set(String::new());
                                                                    new_value.set(String::new());
                                                                    new_value_type.set("string".to_string());
                                                                    show_add_form.set(false);
                                                                    reload();
                                                                }
                                                                Err(e) => {
                                                                    error.set(Some(format!("{}", e)));
                                                                }
                                                            }
                                                            saving.set(false);
                                                        });
                                                    },
                                                    {i18n.t(Key::Save)}
                                                }
                                                button {
                                                    class: "bg-gray-300 hover:bg-gray-400 text-gray-700 px-4 py-2 rounded",
                                                    onclick: move |_| show_add_form.set(false),
                                                    {i18n.t(Key::Cancel)}
                                                }
                                            }
                                        }
                                    }

                                    // Entries table
                                    if entries.read().is_empty() {
                                        p { class: "text-gray-600", {i18n.t(Key::ConfigNoEntries)} }
                                    } else {
                                        table { class: "w-full",
                                            thead { tr { class: "border-b text-left",
                                                th { class: "py-2 px-3", {i18n.t(Key::ConfigKey)} }
                                                th { class: "py-2 px-3", {i18n.t(Key::ConfigValue)} }
                                                th { class: "py-2 px-3", {i18n.t(Key::ConfigValueType)} }
                                                th { class: "py-2 px-3 w-32", "" }
                                            }}
                                            tbody {
                                                for entry in entries.read().iter() {
                                                    {
                                                        let key = entry.key.clone();
                                                        let value = entry.value.clone();
                                                        let vtype = entry.value_type.clone();
                                                        let is_editing = editing_key.read().as_ref() == Some(&key);
                                                        let is_secret = vtype == "secret";
                                                        let key_edit = key.clone();
                                                        let key_save = key.clone();
                                                        let key_del = key.clone();
                                                        rsx! {
                                                            tr { class: "border-b hover:bg-gray-50",
                                                                td { class: "py-2 px-3 font-mono text-sm", "{key}" }
                                                                td { class: "py-2 px-3",
                                                                    if is_editing {
                                                                        input {
                                                                            class: "w-full border rounded px-2 py-1",
                                                                            r#type: if is_secret { "password" } else { "text" },
                                                                            value: "{edit_value}",
                                                                            oninput: move |e| edit_value.set(e.value()),
                                                                        }
                                                                    } else if is_secret {
                                                                        span { class: "text-gray-400", "***" }
                                                                    } else {
                                                                        "{value}"
                                                                    }
                                                                }
                                                                td { class: "py-2 px-3",
                                                                    if is_editing {
                                                                        select {
                                                                            class: "border rounded px-2 py-1",
                                                                            value: "{edit_value_type}",
                                                                            onchange: move |e| edit_value_type.set(e.value()),
                                                                            option { value: "string", {i18n.t(Key::ConfigTypeString)} }
                                                                            option { value: "int", {i18n.t(Key::ConfigTypeInt)} }
                                                                            option { value: "bool", {i18n.t(Key::ConfigTypeBool)} }
                                                                            option { value: "secret", {i18n.t(Key::ConfigTypeSecret)} }
                                                                        }
                                                                    } else {
                                                                        span { class: "text-sm text-gray-500 bg-gray-100 px-2 py-1 rounded",
                                                                            "{vtype}"
                                                                        }
                                                                    }
                                                                }
                                                                td { class: "py-2 px-3",
                                                                    if is_editing {
                                                                        div { class: "flex space-x-1",
                                                                            button {
                                                                                class: "text-green-600 hover:text-green-800 text-sm px-2 py-1",
                                                                                onclick: move |_| {
                                                                                    let key = key_save.clone();
                                                                                    let value = edit_value.read().clone();
                                                                                    let vtype = edit_value_type.read().clone();
                                                                                    spawn(async move {
                                                                                        saving.set(true);
                                                                                        let config = CONFIG.read().clone();
                                                                                        match api::set_config_entry(&config, &key, &value, &vtype).await {
                                                                                            Ok(_) => {
                                                                                                editing_key.set(None);
                                                                                                reload();
                                                                                            }
                                                                                            Err(e) => {
                                                                                                error.set(Some(format!("{}", e)));
                                                                                            }
                                                                                        }
                                                                                        saving.set(false);
                                                                                    });
                                                                                },
                                                                                {i18n.t(Key::Save)}
                                                                            }
                                                                            button {
                                                                                class: "text-gray-600 hover:text-gray-800 text-sm px-2 py-1",
                                                                                onclick: move |_| editing_key.set(None),
                                                                                {i18n.t(Key::Cancel)}
                                                                            }
                                                                        }
                                                                    } else {
                                                                        div { class: "flex space-x-1",
                                                                            button {
                                                                                class: "text-blue-600 hover:text-blue-800 text-sm px-2 py-1",
                                                                                onclick: move |_| {
                                                                                    editing_key.set(Some(key_edit.clone()));
                                                                                    edit_value.set(String::new());
                                                                                    edit_value_type.set(vtype.clone());
                                                                                },
                                                                                {i18n.t(Key::Edit)}
                                                                            }
                                                                            button {
                                                                                class: "text-red-600 hover:text-red-800 text-sm px-2 py-1",
                                                                                onclick: move |_| {
                                                                                    let key = key_del.clone();
                                                                                    spawn(async move {
                                                                                        let config = CONFIG.read().clone();
                                                                                        match api::delete_config_entry(&config, &key).await {
                                                                                            Ok(_) => reload(),
                                                                                            Err(e) => error.set(Some(format!("{}", e))),
                                                                                        }
                                                                                    });
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
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
