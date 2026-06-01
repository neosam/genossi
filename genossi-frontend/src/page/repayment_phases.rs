//! Repayment phases list page (Phase 12 Plan 12-04, UI-01) — admin-only.
//!
//! Pattern aus genossi-frontend/src/page/assemblies.rs (1:1-Klon mit
//! fiscal_year + share_value + Anzahl-Einträge statt name + date + location).
//! Status-Badges via RepaymentPhaseStatusBadge (Plan 12-02).
//! Default-Sort: fiscal_year DESC, created DESC (D-14).
//! Anzahl-Einträge pro Row via use_resource auf list_repayment_entries (UI-01 SC#1; N+1 akzeptabel <20 Phasen).
//! Euro-Parse via crate::component::repayment_format::parse_euro_to_cents (Plan 12-02 kanonisch).
//! Detail-Page-Verlinkung via dioxus_router `Link { to: Route::RepaymentPhaseDetails { id } }`
//! (Route registriert in Plan 12-03).

use dioxus::prelude::*;

use crate::api::{
    self, CreateRepaymentPhaseRequest, RepaymentPhaseTO,
};
use crate::auth::RequirePrivilege;
use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents};
use crate::component::{
    Modal, RepaymentPhaseStatusBadge, ToastContainer, TopBar, show_toast,
};
use crate::i18n::{use_i18n, Key};
use crate::page::access_denied::AccessDeniedPage;
use crate::router::Route;
use crate::service::config::CONFIG;

/// D-14 Claude's Discretion: Default-Sort `fiscal_year DESC, created DESC`
/// (Phase-7 D-08-Notiz: "Frontend (Phase 12) sortiert per `fiscal_year DESC,
/// created DESC` zur Auffindbarkeit").
///
/// Stable bei `fiscal_year + created`-Ties (Rust's sort_by ist stable).
fn sort_phases_default(phases: &[RepaymentPhaseTO]) -> Vec<RepaymentPhaseTO> {
    let mut result: Vec<RepaymentPhaseTO> = phases.to_vec();
    result.sort_by(|a, b| {
        b.fiscal_year
            .cmp(&a.fiscal_year)
            .then_with(|| b.created.cmp(&a.created))
    });
    result
}

#[component]
pub fn RepaymentPhases() -> Element {
    let i18n = use_i18n();

    let mut phases = use_signal(Vec::<RepaymentPhaseTO>::new);
    let mut loading = use_signal(|| true);
    let mut show_create = use_signal(|| false);
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);

    let load = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_repayment_phases(&config).await {
                Ok(list) => phases.set(sort_phases_default(&list)),
                Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            TopBar {}
            div { class: "container mx-auto px-4 py-6",
                div { class: "flex justify-between items-start mb-4",
                    h1 { class: "text-2xl font-bold mb-1", "{i18n.t(Key::RepaymentPhases)}" }
                    button {
                        r#type: "button",
                        class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm min-h-[44px]",
                        onclick: move |_| show_create.set(true),
                        "{i18n.t(Key::RepaymentPhaseCreate)}"
                    }
                }

                if *loading.read() {
                    p { class: "text-gray-500 text-center py-8", "{i18n.t(Key::Loading)}" }
                } else if phases.read().is_empty() {
                    div { class: "text-center py-12",
                        p { class: "text-lg font-medium text-gray-700", "{i18n.t(Key::RepaymentPhaseEmpty)}" }
                        p { class: "text-sm text-gray-500 mt-2 mb-6", "{i18n.t(Key::RepaymentPhaseEmptyHint)}" }
                        button {
                            r#type: "button",
                            class: "bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded min-h-[44px]",
                            onclick: move |_| show_create.set(true),
                            "{i18n.t(Key::RepaymentPhaseCreate)}"
                        }
                    }
                } else {
                    div { class: "flex flex-col gap-2",
                        // ── Header-Row mit Anzahl-Einträge-Spalte (UI-01 SC#1) ──
                        div { class: "grid grid-cols-12 gap-2 px-3 py-2 text-xs font-semibold text-gray-500 border-b",
                            div { class: "col-span-2", "{i18n.t(Key::RepaymentPhaseFiscalYear)}" }
                            div { class: "col-span-3", "{i18n.t(Key::RepaymentPhaseShareValue)}" }
                            div { class: "col-span-2", "Status" }
                            div { class: "col-span-2", "{i18n.t(Key::RepaymentPhaseEntryCount)}" }
                            div { class: "col-span-3" }
                        }
                        for p in phases.read().iter() {
                            RepaymentPhaseListRow {
                                key: "{p.id}",
                                phase: p.clone(),
                            }
                        }
                    }
                }

                if *show_create.read() {
                    Modal {
                        CreateRepaymentPhaseForm {
                            on_close: move |_| show_create.set(false),
                            on_created: move |_| {
                                show_create.set(false);
                                load();
                            },
                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                        }
                    }
                }
            }
            ToastContainer { messages: toast_messages }
        }
    }
}

/// Eine Row in der Listen-Page. Triggert per-Row ein use_resource auf
/// list_repayment_entries(phase_id), um die Anzahl Einträge anzuzeigen.
/// N+1-Queries akzeptabel laut CONTEXT.md `<specifics>` (< 20 Phasen/Jahr).
/// Während fetch: '…' als Loading-Placeholder. Bei Fehler: '?' als Fallback.
#[component]
fn RepaymentPhaseListRow(phase: RepaymentPhaseTO) -> Element {
    let phase_id = phase.id;

    // Per-Row Resource für Anzahl-Einträge (UI-01 SC#1)
    let entries_resource = use_resource(move || async move {
        let config = CONFIG.read().clone();
        api::list_repayment_entries(&config, phase_id).await
    });

    let entry_count_display: String = match &*entries_resource.read_unchecked() {
        None => "…".to_string(),                  // Loading
        Some(Ok(list)) => list.len().to_string(), // Erfolg
        Some(Err(_)) => "?".to_string(),          // Fehler — Defensiv-Fallback
    };

    let created_display = phase.created.clone().unwrap_or_default();

    rsx! {
        Link {
            to: Route::RepaymentPhaseDetails { id: phase.id.to_string() },
            class: "grid grid-cols-12 gap-2 px-3 py-3 items-center bg-white hover:bg-gray-50 border rounded",
            div { class: "col-span-2 font-medium", "{phase.fiscal_year}" }
            div { class: "col-span-3", "{format_payout_eur(1, phase.share_value)} / Anteil" }
            div { class: "col-span-2",
                RepaymentPhaseStatusBadge { status: phase.status }
            }
            div { class: "col-span-2 text-right tabular-nums", "{entry_count_display}" }
            div { class: "col-span-3 text-xs text-gray-500 text-right",
                "{created_display}"
            }
        }
    }
}

#[component]
fn CreateRepaymentPhaseForm(
    on_close: EventHandler<()>,
    on_created: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut fiscal_year = use_signal(|| 0_i32);
    // User entries Euro; persistent ist Cent (D-10 / Phase-7 D-12)
    let mut share_value_euro = use_signal(String::new);
    let mut submitting = use_signal(|| false);

    let invalid_msg = "Bitte gültiges Geschäftsjahr und Anteilswert > 0 angeben".to_string();

    rsx! {
        form {
            class: "flex flex-col gap-4",
            onsubmit: move |e| {
                e.prevent_default();
                let year = *fiscal_year.read();
                // Plan 12-02 Kanonik: parse_euro_to_cents aus repayment_format
                let share_value_cents = match parse_euro_to_cents(&share_value_euro.read()) {
                    Some(c) => c,
                    None => {
                        on_error.call(invalid_msg.clone());
                        return;
                    }
                };
                if !(1900..=9999).contains(&year) {
                    on_error.call(invalid_msg.clone());
                    return;
                }
                submitting.set(true);
                let req = CreateRepaymentPhaseRequest {
                    fiscal_year: year,
                    share_value: share_value_cents,
                };
                spawn(async move {
                    let config = CONFIG.read().clone();
                    match api::create_repayment_phase(&config, &req).await {
                        Ok(_) => on_created.call(()),
                        Err(e) => on_error.call(e.message),
                    }
                    submitting.set(false);
                });
            },
            h2 { class: "text-xl font-semibold", "{i18n.t(Key::RepaymentPhaseCreate)}" }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::RepaymentPhaseFiscalYear)}" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "number",
                    value: "{fiscal_year}",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            fiscal_year.set(n);
                        }
                    },
                }
            }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::RepaymentPhaseShareValue)} (EUR)" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "text",
                    placeholder: "60,00",
                    value: "{share_value_euro}",
                    oninput: move |e| share_value_euro.set(e.value()),
                }
            }
            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                    onclick: move |_| on_close.call(()),
                    "{i18n.t(Key::Cancel)}"
                }
                button {
                    r#type: "submit",
                    class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded disabled:opacity-50 min-h-[44px]",
                    disabled: *submitting.read(),
                    "{i18n.t(Key::Save)}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{RepaymentPhaseStatusTO, RepaymentPhaseTO};
    use uuid::Uuid;

    fn make_phase(year: i32, created_iso: &str) -> RepaymentPhaseTO {
        RepaymentPhaseTO {
            id: Uuid::new_v4(),
            fiscal_year: year,
            share_value: 10_000,
            status: RepaymentPhaseStatusTO::Preparation,
            opened_at: None,
            closed_at: None,
            created: Some(created_iso.into()),
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn sort_by_fiscal_year_desc() {
        let phases = vec![
            make_phase(2023, "2023-01-01T00:00:00Z"),
            make_phase(2025, "2025-01-01T00:00:00Z"),
            make_phase(2024, "2024-01-01T00:00:00Z"),
        ];
        let sorted = sort_phases_default(&phases);
        assert_eq!(sorted[0].fiscal_year, 2025);
        assert_eq!(sorted[1].fiscal_year, 2024);
        assert_eq!(sorted[2].fiscal_year, 2023);
    }

    #[test]
    fn sort_by_created_desc_within_same_year() {
        let phases = vec![
            make_phase(2025, "2025-01-15T00:00:00Z"),
            make_phase(2025, "2025-06-01T00:00:00Z"),
            make_phase(2025, "2025-03-01T00:00:00Z"),
        ];
        let sorted = sort_phases_default(&phases);
        assert_eq!(sorted[0].created.as_deref(), Some("2025-06-01T00:00:00Z"));
        assert_eq!(sorted[1].created.as_deref(), Some("2025-03-01T00:00:00Z"));
        assert_eq!(sorted[2].created.as_deref(), Some("2025-01-15T00:00:00Z"));
    }

    #[test]
    fn sort_empty_returns_empty() {
        let sorted = sort_phases_default(&[]);
        assert_eq!(sorted.len(), 0);
    }
}
