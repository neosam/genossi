use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, MailJobDetailTO, MailJobTO};
use crate::component::{
    is_no_repayment_letter_failure, show_toast, ErrorAlert, MailRecipientRenderedContent,
    MailRecipientStatusBadge, NoRepaymentLetterAction, ToastContainer,
};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

// Quick 260614-ckn: status→i18n-Key / status→Tailwind-Color Helper. Werden von
// MailJobsList (Job-Liste) UND von MailJobDetail (mail_page.rs Deep-Link-Seite)
// genutzt → `pub(crate)`, DRY statt Duplikat.
pub(crate) fn job_status_key(status: &str) -> Key {
    match status {
        "running" => Key::MailJobRunning,
        "done" => Key::MailJobDone,
        "failed" => Key::MailJobFailed,
        _ => Key::MailJobPending,
    }
}

pub(crate) fn job_status_color(status: &str) -> &'static str {
    match status {
        "running" => "text-blue-600",
        "done" => "text-green-600",
        "failed" => "text-red-600",
        _ => "text-gray-600",
    }
}

/// Quick 260614-ckn: Wiederverwendbare Mail-Job-Listen-Komponente. Self-contained
/// — besitzt ihren eigenen State (Jobs, Loading, Error, Expand, Detail, Toasts) und
/// lädt die Job-Liste beim Mount selbst. Wird von `MailJobsPage` (/mail/jobs)
/// gerendert; die Versand-Seite (/mail) verlinkt nur noch hierher.
#[component]
pub fn MailJobsList() -> Element {
    let i18n = use_i18n();
    let mut jobs = use_signal(Vec::<MailJobTO>::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);
    let mut expanded_job_id = use_signal(|| None::<String>);
    let mut job_detail = use_signal(|| None::<MailJobDetailTO>);

    // Quick 260603-evf: toast state for the NoRepaymentLetterAction recovery flow.
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);

    let reload_jobs = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_mail_jobs(&config).await {
                Ok(data) => {
                    jobs.set(data);
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
        reload_jobs();
    });

    rsx! {
        // Error message
        if let Some(ref err) = *error.read() {
            ErrorAlert {
                error: err.clone(),
                on_dismiss: move |_| error.set(None),
            }
        }

        // Mail jobs history
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-xl font-semibold mb-4", {i18n.t(Key::MailJobs)} }
            if *loading.read() {
                p { class: "text-gray-600", {i18n.t(Key::Loading)} }
            } else if jobs.read().is_empty() {
                p { class: "text-gray-600", {i18n.t(Key::MailNoHistory)} }
            } else {
                div { class: "space-y-3",
                    for job in jobs.read().iter() {
                        {
                            let job_id = job.id.clone();
                            let job_id_expand = job.id.clone();
                            let job_id_retry = job.id.clone();
                            let status_color = job_status_color(&job.status);
                            let status_key = job_status_key(&job.status);
                            let progress_pct = if job.total_count > 0 {
                                ((job.sent_count + job.failed_count) as f64 / job.total_count as f64 * 100.0) as i64
                            } else {
                                0
                            };
                            let is_expanded = expanded_job_id.read().as_ref() == Some(&job.id);
                            let has_failures = job.failed_count > 0;
                            let is_retryable = has_failures && job.status != "running";
                            let progress_bar_color = if has_failures { "#ef4444" } else { "#22c55e" };
                            let progress_style = format!("width: {}%; background-color: {};", progress_pct, progress_bar_color);
                            let failed_text = format!("{} {}", job.failed_count, i18n.t(Key::MailJobFailed));
                            rsx! {
                                div { class: "border rounded-lg p-4",
                                    // Job header
                                    div {
                                        class: "flex items-center justify-between cursor-pointer",
                                        onclick: move |_| {
                                            let current = expanded_job_id.read().clone();
                                            if current.as_ref() == Some(&job_id_expand) {
                                                expanded_job_id.set(None);
                                                job_detail.set(None);
                                            } else {
                                                expanded_job_id.set(Some(job_id_expand.clone()));
                                                let id = job_id_expand.clone();
                                                spawn(async move {
                                                    let config = CONFIG.read().clone();
                                                    if let Ok(detail) = api::get_mail_job_detail(&config, &id).await {
                                                        job_detail.set(Some(detail));
                                                    }
                                                });
                                            }
                                        },
                                        div { class: "flex-1",
                                            div { class: "flex items-center gap-3",
                                                span { class: "font-medium", "{job.subject}" }
                                                span { class: "{status_color} text-sm font-medium",
                                                    {i18n.t(status_key)}
                                                }
                                            }
                                            // Progress bar
                                            div { class: "mt-2 flex items-center gap-3",
                                                div { class: "flex-1 bg-gray-200 rounded-full h-2",
                                                    div {
                                                        class: "h-2 rounded-full transition-all",
                                                        style: "{progress_style}",
                                                    }
                                                }
                                                span { class: "text-sm text-gray-600 whitespace-nowrap",
                                                    "{job.sent_count + job.failed_count}/{job.total_count}"
                                                }
                                                if has_failures {
                                                    span { class: "text-sm text-red-500",
                                                        "{failed_text}"
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "flex items-center gap-2 ml-4",
                                            if is_retryable {
                                                button {
                                                    class: "bg-amber-500 hover:bg-amber-600 text-white px-3 py-1 rounded text-sm",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        let id = job_id_retry.clone();
                                                        spawn(async move {
                                                            let config = CONFIG.read().clone();
                                                            match api::retry_mail_job(&config, &id).await {
                                                                Ok(_) => reload_jobs(),
                                                                Err(e) => error.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                    {i18n.t(Key::MailRetry)}
                                                }
                                            }
                                            span { class: "text-gray-400 text-sm",
                                                if is_expanded { "▲" } else { "▼" }
                                            }
                                        }
                                    }

                                    // Expanded recipients
                                    if is_expanded {
                                        if let Some(detail) = job_detail.read().as_ref() {
                                            if detail.job.id == job_id {
                                                div { class: "mt-4 border-t pt-3",
                                                    h3 { class: "text-sm font-medium text-gray-700 mb-2",
                                                        {i18n.t(Key::MailRecipients)}
                                                    }
                                                    div { class: "max-h-60 overflow-y-auto",
                                                        table { class: "w-full text-sm",
                                                            thead { tr { class: "border-b text-left text-gray-500",
                                                                th { class: "py-1 px-2", {i18n.t(Key::MailTo)} }
                                                                th { class: "py-1 px-2", {i18n.t(Key::MailStatus)} }
                                                                th { class: "py-1 px-2", {i18n.t(Key::MailError)} }
                                                                // Quick 260603-evf: action column (empty header).
                                                                th { class: "py-1 px-2", "" }
                                                            }}
                                                            tbody {
                                                                for r in detail.recipients.iter() {
                                                                    {
                                                                        // Quick 260603-evf: Badge-Rendering wandert in
                                                                        // `MailRecipientStatusBadge` (Component-First).
                                                                        let error_text = r.error.clone().unwrap_or_default();
                                                                        // Quick 260603-evf: resolve (member_id, phase_id)
                                                                        // required by NoRepaymentLetterAction. Gated on
                                                                        // `is_no_repayment_letter_failure` so the button
                                                                        // only appears for the recoverable failure mode.
                                                                        let mid: Option<Uuid> = r.member_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                                                                        let pid: Option<Uuid> = detail.job.repayment_phase_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                                                                        let show_action = is_no_repayment_letter_failure(&r.status, r.error.as_deref());
                                                                        let job_id_for_action = detail.job.id.clone();
                                                                        let recipient_id_for_action = r.id.clone();
                                                                        let i18n_for_action = i18n.clone();
                                                                        rsx! {
                                                                            tr { class: "border-b last:border-b-0",
                                                                                td { class: "py-1 px-2", "{r.to_address}" }
                                                                                td { class: "py-1 px-2",
                                                                                    MailRecipientStatusBadge {
                                                                                        status: r.status.clone(),
                                                                                        error: r.error.clone(),
                                                                                    }
                                                                                }
                                                                                td { class: "py-1 px-2 text-red-500 text-xs", "{error_text}" }
                                                                                td { class: "py-1 px-2",
                                                                                    if show_action {
                                                                                        if let (Some(mid), Some(pid)) = (mid, pid) {
                                                                                            NoRepaymentLetterAction {
                                                                                                job_id: job_id_for_action,
                                                                                                recipient_id: recipient_id_for_action,
                                                                                                member_id: mid,
                                                                                                phase_id: pid,
                                                                                                on_done: move |_| {
                                                                                                    show_toast(
                                                                                                        &mut toast_messages,
                                                                                                        &mut toast_counter,
                                                                                                        i18n_for_action.t(Key::MailGenerateLetterAndRetrySuccess).to_string(),
                                                                                                    );
                                                                                                    reload_jobs();
                                                                                                },
                                                                                                on_error: move |msg: String| {
                                                                                                    show_toast(
                                                                                                        &mut toast_messages,
                                                                                                        &mut toast_counter,
                                                                                                        msg,
                                                                                                    );
                                                                                                },
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            // Quick 260614-9zf: per-recipient rendered
                                                                            // subject/body (Component-First; renders
                                                                            // nothing when both are None).
                                                                            tr {
                                                                                td { colspan: 4, class: "px-2",
                                                                                    MailRecipientRenderedContent {
                                                                                        rendered_subject: r.rendered_subject.clone(),
                                                                                        rendered_body: r.rendered_body.clone(),
                                                                                        rendered_reconstructed: r.rendered_reconstructed,
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
                                        } else {
                                            div { class: "mt-4 text-gray-500 text-sm",
                                                {i18n.t(Key::Loading)}
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
        // Quick 260603-evf: toasts for the NoRepaymentLetterAction recovery flow.
        ToastContainer { messages: toast_messages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_key_maps_known_states() {
        assert_eq!(job_status_key("running"), Key::MailJobRunning);
        assert_eq!(job_status_key("done"), Key::MailJobDone);
        assert_eq!(job_status_key("failed"), Key::MailJobFailed);
        assert_eq!(job_status_key("anything_else"), Key::MailJobPending);
    }

    #[test]
    fn job_status_color_maps_known_states() {
        assert_eq!(job_status_color("running"), "text-blue-600");
        assert_eq!(job_status_color("done"), "text-green-600");
        assert_eq!(job_status_color("failed"), "text-red-600");
        assert_eq!(job_status_color("pending"), "text-gray-600");
    }
}
