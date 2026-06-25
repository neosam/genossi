use dioxus::prelude::*;

use crate::api::{self, ConfigEntryTO};
use crate::auth::RequirePrivilege;
use crate::component::{
    CollapsibleSection, ErrorAlert, TopBar, TsaConfigSection, WordPressIntegrationSection,
};
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
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);
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

    // IMAP form state
    let mut imap_host = use_signal(|| String::new());
    let mut imap_port = use_signal(|| "993".to_string());
    let mut imap_user = use_signal(|| String::new());
    let mut imap_pass = use_signal(|| String::new());
    let mut imap_pass_set = use_signal(|| false);
    let mut imap_mailbox = use_signal(|| "INBOX".to_string());
    let mut imap_archive_mailbox = use_signal(|| String::new());
    let mut imap_poll_interval = use_signal(|| "300".to_string());
    let mut imap_saving = use_signal(|| false);
    let mut imap_folders = use_signal(|| Vec::<String>::new());
    let mut imap_folders_loading = use_signal(|| false);
    let mut imap_folders_loaded = use_signal(|| false);

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

    // Mail footer state
    let mut mail_footer = use_signal(|| String::new());
    let mut mail_footer_saving = use_signal(|| false);

    // Sender name state
    let mut sender_name = use_signal(|| String::new());
    let mut sender_name_saving = use_signal(|| false);

    // WebDAV Backup form state
    let mut webdav_url = use_signal(|| String::new());
    let mut webdav_username = use_signal(|| String::new());
    let mut webdav_pass = use_signal(|| String::new());
    let mut webdav_pass_set = use_signal(|| false);
    let mut webdav_directory = use_signal(|| "genossi-export".to_string());
    let mut webdav_interval = use_signal(|| "24".to_string());
    let mut webdav_enabled = use_signal(|| false);
    let mut webdav_saving = use_signal(|| false);
    let mut webdav_testing = use_signal(|| false);
    let mut webdav_last_run = use_signal(|| None::<String>);
    let mut webdav_last_status = use_signal(|| None::<String>);

    // WordPress integration form state
    let mut wp_share_value_cents = use_signal(|| String::new());
    let mut wp_bank_iban = use_signal(|| String::new());
    let mut wp_bank_name = use_signal(|| String::new());
    let mut wp_bank_bic = use_signal(|| String::new());
    let mut wp_genossenschaft_name = use_signal(|| String::new());

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

                    // Populate IMAP form from entries
                    imap_host.set(get_config_value(&data, "imap_host"));
                    let imap_port_val = get_config_value(&data, "imap_port");
                    if !imap_port_val.is_empty() {
                        imap_port.set(imap_port_val);
                    }
                    imap_user.set(get_config_value(&data, "imap_user"));
                    imap_pass_set.set(has_config_key(&data, "imap_pass"));
                    imap_pass.set(String::new());
                    let mailbox_val = get_config_value(&data, "imap_mailbox");
                    if !mailbox_val.is_empty() {
                        imap_mailbox.set(mailbox_val);
                    }
                    imap_archive_mailbox.set(get_config_value(&data, "imap_archive_mailbox"));
                    let poll_val = get_config_value(&data, "imap_poll_interval_seconds");
                    if !poll_val.is_empty() {
                        imap_poll_interval.set(poll_val);
                    }

                    // Populate mail footer from entries
                    mail_footer.set(get_config_value(&data, "mail_footer"));

                    // Populate WebDAV backup settings
                    webdav_url.set(get_config_value(&data, "backup_webdav_url"));
                    webdav_username.set(get_config_value(&data, "backup_webdav_username"));
                    webdav_pass_set.set(has_config_key(&data, "backup_webdav_password"));
                    webdav_pass.set(String::new());
                    let dir_val = get_config_value(&data, "backup_webdav_directory");
                    if !dir_val.is_empty() {
                        webdav_directory.set(dir_val);
                    }
                    let interval_val = get_config_value(&data, "backup_interval_hours");
                    if !interval_val.is_empty() {
                        webdav_interval.set(interval_val);
                    }
                    webdav_enabled.set(get_config_value(&data, "backup_webdav_enabled") == "true");
                    let last_run = get_config_value(&data, "backup_last_run");
                    webdav_last_run.set(if last_run.is_empty() {
                        None
                    } else {
                        Some(last_run)
                    });
                    let last_status = get_config_value(&data, "backup_last_status");
                    webdav_last_status.set(if last_status.is_empty() {
                        None
                    } else {
                        Some(last_status)
                    });

                    // Populate WordPress integration fields
                    wp_share_value_cents.set(get_config_value(&data, "share_value_cents"));
                    wp_bank_iban.set(get_config_value(&data, "bank_iban"));
                    wp_bank_name.set(get_config_value(&data, "bank_name"));
                    wp_bank_bic.set(get_config_value(&data, "bank_bic"));
                    wp_genossenschaft_name.set(get_config_value(&data, "genossenschaft_name"));

                    // Load sender_name user preference
                    if let Ok(Some(pref)) = api::get_user_preference(&config, "sender_name").await {
                        sender_name.set(pref.value);
                    }

                    entries.set(data);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e));
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
        privilege: crate::auth::PRIVILEGE_ADMIN,
        fallback: rsx! { AccessDeniedPage { required_privilege: crate::auth::PRIVILEGE_ADMIN.to_string() } },
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
                if let Some(ref err) = *error.read() {
                    ErrorAlert {
                        error: err.clone(),
                        on_dismiss: move |_| error.set(None),
                    }
                }

                if *loading.read() {
                    p { class: "text-gray-600", {i18n.t(Key::Loading)} }
                } else {
                    // SMTP Settings Section
                    CollapsibleSection { title: i18n.t(Key::SmtpSettings).to_string(),
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
                                                    error.set(Some(e));
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
                                                        error.set(Some(api::AppError::new(e.status, format!("{}: {}", i18n.t(Key::SmtpTestFailed), e.message), e.detail)));
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

                    // Mail Footer & Sender Name Section
                    CollapsibleSection { title: "Mail-Footer".to_string(),
                        div { class: "space-y-4",
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    "Absendername"
                                }
                                p { class: "text-xs text-gray-500 mb-1",
                                    "Ihr Name, der im Footer als Absender erscheint. Wird pro Benutzer gespeichert."
                                }
                                div { class: "flex gap-2",
                                    input {
                                        class: "flex-1 border rounded px-3 py-2",
                                        r#type: "text",
                                        placeholder: "Anna Schmidt",
                                        value: "{sender_name}",
                                        oninput: move |e| sender_name.set(e.value()),
                                    }
                                    button {
                                        class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                                        disabled: *sender_name_saving.read(),
                                        onclick: {
                                            let i18n = i18n.clone();
                                            move |_| {
                                            let name = sender_name.read().clone();
                                            let i18n = i18n.clone();
                                            spawn(async move {
                                                sender_name_saving.set(true);
                                                error.set(None);
                                                success_msg.set(None);
                                                let config = CONFIG.read().clone();
                                                match api::set_user_preference(&config, "sender_name", &name).await {
                                                    Ok(_) => {
                                                        success_msg.set(Some(i18n.t(Key::Save).to_string()));
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(e));
                                                    }
                                                }
                                                sender_name_saving.set(false);
                                            });
                                        }},
                                        {i18n.t(Key::Save)}
                                    }
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    "Footer-Vorlage"
                                }
                                p { class: "text-xs text-gray-500 mb-1",
                                    "Template für den Mail-Footer. Verfügbare Variable: {{ sender_name }}"
                                }
                                textarea {
                                    class: "w-full border rounded px-3 py-2 font-mono text-sm",
                                    rows: 4,
                                    placeholder: "Mit freundlichen Grüßen\n{{ sender_name }}\nMein Verein e.G.",
                                    value: "{mail_footer}",
                                    oninput: move |e| mail_footer.set(e.value()),
                                }
                                div { class: "flex justify-end mt-2",
                                    button {
                                        class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                                        disabled: *mail_footer_saving.read(),
                                        onclick: {
                                            let i18n = i18n.clone();
                                            move |_| {
                                            let footer = mail_footer.read().clone();
                                            let i18n = i18n.clone();
                                            spawn(async move {
                                                mail_footer_saving.set(true);
                                                error.set(None);
                                                success_msg.set(None);
                                                let config = CONFIG.read().clone();
                                                match api::set_config_entry(&config, "mail_footer", &footer, "string").await {
                                                    Ok(_) => {
                                                        success_msg.set(Some(i18n.t(Key::Save).to_string()));
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(e));
                                                    }
                                                }
                                                mail_footer_saving.set(false);
                                            });
                                        }},
                                        {i18n.t(Key::Save)}
                                    }
                                }
                            }
                        }
                    }

                    // IMAP Settings Section (Posteingang)
                    CollapsibleSection { title: "IMAP Posteingang".to_string(),
                        div { class: "space-y-4",
                            // Host + Port row
                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                                div { class: "md:col-span-2",
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "IMAP Host"
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "text",
                                        placeholder: "imap.example.com",
                                        value: "{imap_host}",
                                        oninput: move |e| imap_host.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Port"
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "number",
                                        placeholder: "993",
                                        value: "{imap_port}",
                                        oninput: move |e| imap_port.set(e.value()),
                                    }
                                }
                            }

                            // Username + Password row
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Benutzername"
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "text",
                                        value: "{imap_user}",
                                        oninput: move |e| imap_user.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Passwort"
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "password",
                                        placeholder: if *imap_pass_set.read() { "********" } else { "" },
                                        value: "{imap_pass}",
                                        oninput: move |e| imap_pass.set(e.value()),
                                    }
                                }
                            }

                            // Load folders button
                            if !*imap_folders_loaded.read() {
                                div { class: "pt-1",
                                    button {
                                        class: "text-sm text-blue-600 hover:underline disabled:opacity-50",
                                        disabled: *imap_folders_loading.read() || imap_host.read().is_empty(),
                                        onclick: move |_| {
                                            spawn(async move {
                                                imap_folders_loading.set(true);
                                                let config = CONFIG.read().clone();
                                                match api::get_imap_folders(&config).await {
                                                    Ok(folders) => {
                                                        imap_folders.set(folders);
                                                        imap_folders_loaded.set(true);
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(api::AppError::new(e.status, format!("Ordner laden fehlgeschlagen: {}", e.message), e.detail)));
                                                    }
                                                }
                                                imap_folders_loading.set(false);
                                            });
                                        },
                                        if *imap_folders_loading.read() {
                                            "Ordner werden geladen…"
                                        } else {
                                            "Ordner vom Server laden"
                                        }
                                    }
                                }
                            }

                            // Mailbox + Archive + Poll interval
                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Postfach"
                                    }
                                    if *imap_folders_loaded.read() {
                                        select {
                                            class: "w-full border rounded px-3 py-2",
                                            value: "{imap_mailbox}",
                                            onchange: move |e| imap_mailbox.set(e.value()),
                                            for folder in imap_folders.read().iter() {
                                                option {
                                                    value: "{folder}",
                                                    selected: *imap_mailbox.read() == *folder,
                                                    "{folder}"
                                                }
                                            }
                                        }
                                    } else {
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "text",
                                            placeholder: "INBOX",
                                            value: "{imap_mailbox}",
                                            oninput: move |e| imap_mailbox.set(e.value()),
                                        }
                                    }
                                }
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Archiv-Ordner"
                                    }
                                    if *imap_folders_loaded.read() {
                                        select {
                                            class: "w-full border rounded px-3 py-2",
                                            value: "{imap_archive_mailbox}",
                                            onchange: move |e| imap_archive_mailbox.set(e.value()),
                                            option {
                                                value: "",
                                                selected: imap_archive_mailbox.read().is_empty(),
                                                "(keiner)"
                                            }
                                            for folder in imap_folders.read().iter() {
                                                option {
                                                    value: "{folder}",
                                                    selected: *imap_archive_mailbox.read() == *folder,
                                                    "{folder}"
                                                }
                                            }
                                        }
                                    } else {
                                        input {
                                            class: "w-full border rounded px-3 py-2",
                                            r#type: "text",
                                            placeholder: "Archive",
                                            value: "{imap_archive_mailbox}",
                                            oninput: move |e| imap_archive_mailbox.set(e.value()),
                                        }
                                    }
                                }
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Poll-Intervall (Sek.)"
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "number",
                                        placeholder: "300",
                                        value: "{imap_poll_interval}",
                                        oninput: move |e| imap_poll_interval.set(e.value()),
                                    }
                                }
                            }

                            // Save button
                            div { class: "flex items-center space-x-4 pt-2",
                                button {
                                    class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                    disabled: *imap_saving.read() || imap_host.read().is_empty(),
                                    onclick: move |_| {
                                        let host = imap_host.read().clone();
                                        let port = imap_port.read().clone();
                                        let user = imap_user.read().clone();
                                        let pass = imap_pass.read().clone();
                                        let pass_set = *imap_pass_set.read();
                                        let mailbox = imap_mailbox.read().clone();
                                        let archive = imap_archive_mailbox.read().clone();
                                        let poll = imap_poll_interval.read().clone();
                                        spawn(async move {
                                            imap_saving.set(true);
                                            error.set(None);
                                            success_msg.set(None);
                                            let config = CONFIG.read().clone();
                                            let mut all_ok = true;

                                            let mut entries_to_save: Vec<(&str, String, &str)> = vec![
                                                ("imap_host", host, "string"),
                                                ("imap_port", port, "int"),
                                                ("imap_user", user, "string"),
                                                ("imap_tls", "true".to_string(), "bool"),
                                                ("imap_mailbox", mailbox, "string"),
                                                ("imap_poll_interval_seconds", poll, "int"),
                                            ];
                                            if !archive.is_empty() {
                                                entries_to_save.push(("imap_archive_mailbox", archive, "string"));
                                            }
                                            if !pass.is_empty() || !pass_set {
                                                entries_to_save.push(("imap_pass", pass, "secret"));
                                            }

                                            for (key, value, vtype) in &entries_to_save {
                                                if let Err(e) = api::set_config_entry(&config, key, value, vtype).await {
                                                    error.set(Some(e));
                                                    all_ok = false;
                                                    break;
                                                }
                                            }

                                            if all_ok {
                                                success_msg.set(Some("IMAP-Einstellungen gespeichert".to_string()));
                                                reload();
                                            }
                                            imap_saving.set(false);
                                        });
                                    },
                                    if *imap_saving.read() {
                                        "Speichere…"
                                    } else {
                                        "Speichern"
                                    }
                                }
                            }
                        }
                    }

                    // WebDAV Backup Settings Section
                    CollapsibleSection { title: i18n.t(Key::WebDavBackup).to_string(),
                        div { class: "space-y-4",
                            // Enabled toggle
                            div {
                                label { class: "inline-flex items-center cursor-pointer",
                                    input {
                                        r#type: "checkbox",
                                        class: "mr-2 w-4 h-4",
                                        checked: *webdav_enabled.read(),
                                        onchange: move |e: Event<FormData>| webdav_enabled.set(e.value() == "true"),
                                    }
                                    span { class: "text-sm font-medium text-gray-700",
                                        {i18n.t(Key::WebDavEnabled)}
                                    }
                                }
                            }

                            // URL
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::WebDavUrl)}
                                }
                                input {
                                    class: "w-full border rounded px-3 py-2",
                                    r#type: "text",
                                    placeholder: "{i18n.t(Key::WebDavUrlPlaceholder)}",
                                    value: "{webdav_url}",
                                    oninput: move |e| webdav_url.set(e.value()),
                                }
                            }

                            // Username + Password row
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::WebDavUsername)}
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "text",
                                        value: "{webdav_username}",
                                        oninput: move |e| webdav_username.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::WebDavPassword)}
                                        if *webdav_pass_set.read() {
                                            span { class: "ml-2 text-xs text-green-600",
                                                "({i18n.t(Key::WebDavPasswordSet)})"
                                            }
                                        }
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "password",
                                        placeholder: if *webdav_pass_set.read() { "********" } else { "" },
                                        value: "{webdav_pass}",
                                        oninput: move |e| webdav_pass.set(e.value()),
                                    }
                                }
                            }

                            // Directory + Interval row
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::WebDavDirectory)}
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "text",
                                        placeholder: "{i18n.t(Key::WebDavDirectoryPlaceholder)}",
                                        value: "{webdav_directory}",
                                        oninput: move |e| webdav_directory.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        {i18n.t(Key::WebDavIntervalHours)}
                                    }
                                    input {
                                        class: "w-full border rounded px-3 py-2",
                                        r#type: "number",
                                        min: "1",
                                        value: "{webdav_interval}",
                                        oninput: move |e| webdav_interval.set(e.value()),
                                    }
                                }
                            }

                            // Save button
                            div { class: "flex items-center space-x-4 pt-2",
                                button {
                                    class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                    disabled: *webdav_saving.read(),
                                    onclick: {
                                        let i18n = i18n.clone();
                                        move |_| {
                                        let url = webdav_url.read().clone();
                                        let username = webdav_username.read().clone();
                                        let pass = webdav_pass.read().clone();
                                        let pass_set = *webdav_pass_set.read();
                                        let directory = webdav_directory.read().clone();
                                        let interval = webdav_interval.read().clone();
                                        let enabled = *webdav_enabled.read();
                                        let i18n = i18n.clone();
                                        spawn(async move {
                                            webdav_saving.set(true);
                                            error.set(None);
                                            success_msg.set(None);
                                            let config = CONFIG.read().clone();
                                            let mut all_ok = true;

                                            let entries_to_save: Vec<(&str, String, &str)> = {
                                                let mut v = vec![
                                                    ("backup_webdav_enabled", if enabled { "true".to_string() } else { "false".to_string() }, "bool"),
                                                    ("backup_webdav_url", url, "string"),
                                                    ("backup_webdav_username", username, "string"),
                                                    ("backup_webdav_directory", directory, "string"),
                                                    ("backup_interval_hours", interval, "int"),
                                                ];
                                                if !pass.is_empty() || !pass_set {
                                                    v.push(("backup_webdav_password", pass, "secret"));
                                                }
                                                v
                                            };

                                            for (key, value, vtype) in &entries_to_save {
                                                if let Err(e) = api::set_config_entry(&config, key, value, vtype).await {
                                                    error.set(Some(e));
                                                    all_ok = false;
                                                    break;
                                                }
                                            }

                                            if all_ok {
                                                success_msg.set(Some(i18n.t(Key::Save).to_string()));
                                                reload();
                                            }
                                            webdav_saving.set(false);
                                        });
                                    }},
                                    if *webdav_saving.read() {
                                        {i18n.t(Key::WebDavSaving)}
                                    } else {
                                        {i18n.t(Key::Save)}
                                    }
                                }
                            }

                            // Test connection button
                            div { class: "border-t pt-4 mt-4",
                                h3 { class: "text-sm font-medium text-gray-700 mb-2",
                                    {i18n.t(Key::WebDavTestConnection)}
                                }
                                button {
                                    class: "bg-gray-500 hover:bg-gray-600 text-white px-4 py-2 rounded text-sm disabled:opacity-50",
                                    disabled: *webdav_testing.read() || webdav_url.read().is_empty(),
                                    onclick: {
                                        let i18n = i18n.clone();
                                        move |_| {
                                        let i18n = i18n.clone();
                                        spawn(async move {
                                            webdav_testing.set(true);
                                            error.set(None);
                                            success_msg.set(None);
                                            let config = CONFIG.read().clone();
                                            match api::test_webdav_connection(&config).await {
                                                Ok(()) => {
                                                    success_msg.set(Some(i18n.t(Key::WebDavTestSuccess).to_string()));
                                                }
                                                Err(e) => {
                                                    error.set(Some(api::AppError::new(e.status, format!("{}: {}", i18n.t(Key::WebDavTestFailed), e.message), e.detail)));
                                                }
                                            }
                                            webdav_testing.set(false);
                                        });
                                    }},
                                    if *webdav_testing.read() {
                                        {i18n.t(Key::WebDavSaving)}
                                    } else {
                                        {i18n.t(Key::WebDavTestConnection)}
                                    }
                                }
                            }

                            // Backup status display
                            div { class: "border-t pt-4 mt-4",
                                h3 { class: "text-sm font-medium text-gray-700 mb-2",
                                    {i18n.t(Key::WebDavLastBackup)}
                                }
                                if let Some(last_run) = webdav_last_run.read().as_ref() {
                                    p { class: "text-sm text-gray-600 mb-1",
                                        "{last_run}"
                                    }
                                    if let Some(status) = webdav_last_status.read().as_ref() {
                                        if status.starts_with("Erfolgreich") || status.starts_with("Success") {
                                            p { class: "text-sm text-green-600", "{status}" }
                                        } else {
                                            p { class: "text-sm text-red-600", "{status}" }
                                        }
                                    }
                                } else {
                                    p { class: "text-sm text-gray-400 italic",
                                        {i18n.t(Key::WebDavNoBackupYet)}
                                    }
                                }
                            }
                        }
                    }

                    // TSA Configuration Section
                    CollapsibleSection { title: i18n.t(Key::TimestampTsaConfig).to_string(),
                        TsaConfigSection {
                            entries: entries,
                            error: error,
                            success_msg: success_msg,
                            on_reload: move |_| reload(),
                        }
                    }

                    // WordPress Integration Section
                    CollapsibleSection { title: i18n.t(Key::WordPressIntegration).to_string(),
                        WordPressIntegrationSection {
                            entries: entries,
                            share_value_cents: wp_share_value_cents,
                            bank_iban: wp_bank_iban,
                            bank_name: wp_bank_name,
                            bank_bic: wp_bank_bic,
                            genossenschaft_name: wp_genossenschaft_name,
                            error: error,
                            success_msg: success_msg,
                            on_reload: move |_| reload(),
                        }
                    }

                    // Advanced Configuration (collapsible)
                    CollapsibleSection { title: i18n.t(Key::AdvancedConfig).to_string(),
                        div {
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
                                                                error.set(Some(e));
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
                                                                                            error.set(Some(e));
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
                                                                                        Err(e) => error.set(Some(e)),
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
