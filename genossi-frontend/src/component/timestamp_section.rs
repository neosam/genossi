use dioxus::prelude::*;
use rest_types::{TimestampResponseTO, TimestampVerifyResponseTO};

use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[component]
pub fn TimestampSection() -> Element {
    let i18n = use_i18n();
    let mut tsa_enabled = use_signal(|| false);
    let mut config_loaded = use_signal(|| false);
    let mut timestamps = use_signal(|| Vec::<TimestampResponseTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut create_loading = use_signal(|| false);
    let mut create_message = use_signal(|| None::<String>);
    let mut verify_result = use_signal(|| None::<(uuid::Uuid, TimestampVerifyResponseTO)>);
    let mut verify_loading = use_signal(|| None::<uuid::Uuid>);

    let load_timestamps = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();

            // Check if TSA is enabled
            if let Ok(entries) = api::get_config_entries(&config).await {
                let enabled = entries
                    .iter()
                    .find(|e| e.key == "tsa_enabled")
                    .map(|e| e.value == "true")
                    .unwrap_or(false);
                tsa_enabled.set(enabled);
                config_loaded.set(true);

                if !enabled {
                    loading.set(false);
                    return;
                }
            }

            match api::get_timestamps(&config).await {
                Ok(data) => {
                    timestamps.set(data);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect({
        let load = load_timestamps.clone();
        move || {
            load();
        }
    });

    // Don't render anything if config is loaded and TSA is not enabled
    if *config_loaded.read() && !*tsa_enabled.read() {
        return rsx! {};
    }

    let on_create = {
        let load = load_timestamps.clone();
        move |_| {
            let load = load.clone();
            spawn(async move {
                create_loading.set(true);
                create_message.set(None);
                let config = CONFIG.read().clone();
                match api::create_timestamp(&config).await {
                    Ok(result) => {
                        create_message.set(Some(result.message.clone()));
                        if result.created {
                            load();
                        }
                    }
                    Err(e) => {
                        create_message.set(Some(format!("Fehler: {}", e)));
                    }
                }
                create_loading.set(false);
            });
        }
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6 mb-6",
            h2 { class: "text-2xl font-bold mb-4", {i18n.t(Key::TimestampTitle)} }

            // Create button
            div { class: "mb-4 flex items-center gap-4",
                button {
                    class: "bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 disabled:opacity-50",
                    disabled: *create_loading.read(),
                    onclick: on_create,
                    if *create_loading.read() {
                        {i18n.t(Key::TimestampCreating)}
                    } else {
                        {i18n.t(Key::TimestampCreateButton)}
                    }
                }
                if let Some(msg) = create_message.read().as_ref() {
                    span { class: "text-sm text-gray-600", "{msg}" }
                }
            }

            if *loading.read() {
                p { class: "text-gray-600", {i18n.t(Key::Loading)} }
            } else if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                    "{err}"
                }
            } else if timestamps.read().is_empty() {
                p { class: "text-gray-500 italic", {i18n.t(Key::TimestampNoTimestamps)} }
            } else {
                // Timestamp table
                div { class: "overflow-x-auto",
                    table { class: "w-full text-sm",
                        thead {
                            tr { class: "border-b text-left bg-gray-50",
                                th { class: "py-2 px-3", {i18n.t(Key::TimestampDate)} }
                                th { class: "py-2 px-3", {i18n.t(Key::TimestampHash)} }
                                th { class: "py-2 px-3", {i18n.t(Key::TimestampEntryCount)} }
                                th { class: "py-2 px-3", {i18n.t(Key::TimestampStatus)} }
                                th { class: "py-2 px-3", "" }
                            }
                        }
                        tbody {
                            for ts in timestamps.read().iter() {
                                {render_timestamp_row(ts, &i18n, &verify_result, &verify_loading)}
                            }
                        }
                    }
                }
            }

            // Verification result display
            if let Some((id, ref result)) = *verify_result.read() {
                div { class: "mt-4 p-4 bg-gray-50 rounded border",
                    h3 { class: "font-bold mb-2", "Verifikation: {id}" }
                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-2",
                        div { class: if result.token_valid { "text-green-700" } else { "text-red-700" },
                            if result.token_valid {
                                {i18n.t(Key::TimestampTokenValid)}
                            } else {
                                {i18n.t(Key::TimestampTokenInvalid)}
                            }
                        }
                        div { class: if result.hash_matches { "text-green-700" } else { "text-red-700" },
                            if result.hash_matches {
                                {i18n.t(Key::TimestampHashMatches)}
                            } else {
                                {i18n.t(Key::TimestampHashMismatch)}
                            }
                        }
                        div { class: if result.audit_log_consistent { "text-green-700" } else { "text-red-700" },
                            if result.audit_log_consistent {
                                {i18n.t(Key::TimestampAuditConsistent)}
                            } else {
                                {i18n.t(Key::TimestampAuditInconsistent)}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_timestamp_row(
    ts: &TimestampResponseTO,
    i18n: &crate::i18n::I18n,
    verify_result: &Signal<Option<(uuid::Uuid, TimestampVerifyResponseTO)>>,
    verify_loading: &Signal<Option<uuid::Uuid>>,
) -> Element {
    let id = ts.id;
    let status_class = match ts.status.as_str() {
        "success" => "text-green-700 bg-green-100",
        "tsa_failed" => "text-red-700 bg-red-100",
        "upload_failed" => "text-yellow-700 bg-yellow-100",
        _ => "text-gray-700 bg-gray-100",
    };
    let status_text = match ts.status.as_str() {
        "success" => i18n.t(Key::TimestampStatusSuccess),
        "tsa_failed" => i18n.t(Key::TimestampStatusFailed),
        "upload_failed" => i18n.t(Key::TimestampStatusUploadFailed),
        _ => ts.status.clone().into(),
    };
    let hash_short = if ts.audit_hash.len() > 12 {
        format!("{}...", &ts.audit_hash[..12])
    } else {
        ts.audit_hash.clone()
    };
    let is_verifying = *verify_loading.read() == Some(id);

    let mut verify_result = verify_result.clone();
    let mut verify_loading = verify_loading.clone();

    let timestamp_display = i18n.format_datetime_long(&ts.timestamp);

    rsx! {
        tr { class: "border-b hover:bg-gray-50",
            td { class: "py-2 px-3", "{timestamp_display}" }
            td { class: "py-2 px-3 font-mono text-xs", title: "{ts.audit_hash}",
                "{hash_short}"
            }
            td { class: "py-2 px-3", "{ts.audit_entry_count}" }
            td { class: "py-2 px-3",
                span { class: "px-2 py-1 rounded text-xs {status_class}",
                    {status_text}
                }
            }
            td { class: "py-2 px-3",
                if ts.status == "success" {
                    button {
                        class: "bg-gray-200 text-gray-700 px-2 py-1 rounded text-xs hover:bg-gray-300 disabled:opacity-50",
                        disabled: is_verifying,
                        onclick: move |_| {
                            let id = id;
                            spawn(async move {
                                verify_loading.set(Some(id));
                                let config = CONFIG.read().clone();
                                match api::verify_timestamp(&config, id).await {
                                    Ok(result) => {
                                        verify_result.set(Some((id, result)));
                                    }
                                    Err(_) => {}
                                }
                                verify_loading.set(None);
                            });
                        },
                        if is_verifying {
                            {i18n.t(Key::TimestampVerifying)}
                        } else {
                            {i18n.t(Key::TimestampVerify)}
                        }
                    }
                }
            }
        }
    }
}
