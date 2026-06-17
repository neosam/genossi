//! Repayment-Entry-Liste (Phase 12 Plan 12-08, UI-03).
//!
//! 7-Spalten-Tabelle + Multi-Select + Status-Filter + Inline-Cell-Edit.
//! Component-First: reusen MemberSearch-Pattern (mail_page.rs), Modal,
//! ToastContainer, EditableShareCountCell, RepaymentEntryStatusBadge,
//! format_payout_eur.
//!
//! Default-Sort: Mitgliedsnummer ASC, created ASC (D-14).
//! Status-Filter ist client-side (Backend liefert immer alle, Phase 8 D-10).
//!
//! Component-Vertrag (EventHandler-Props):
//!   - on_changed: Reload-Trigger nach Mutation
//!   - on_add: Open Add-Entry-Modal (Plan 12-09 mountet das Modal in der Detail-Page)
//!   - on_paidout_request: Open PaidOut-Confirm (Plan 12-10)
//!   - on_mail_request: Redirect zu /mail (Plan 12-13)
//!   - on_error: Toast-Trigger
//!
//! D-08 readonly_mode: bei phase.status == Closed wird die Bulk-Action-Leiste,
//! die Checkbox-Spalte, der Inline-Edit und die Trash-Action ausgeblendet —
//! die Tabelle wird zur reinen Lese-Ansicht. Tab-Filter bleibt aktiv.

use dioxus::prelude::*;
use rest_types::MemberTO;
use uuid::Uuid;

use crate::api::{
    self, BatchStatusRequest, RepaymentEntryStatusTO, RepaymentEntryTO, RepaymentPhaseStatusTO,
    RepaymentPhaseTO, UpdateRepaymentEntryRequest,
};
use crate::component::repayment_format::format_payout_eur;
use crate::component::{EditableShareCountCell, Modal, RepaymentEntryStatusBadge};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::MEMBERS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Open,
    Contacted,
    PaidOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCounts {
    pub all: usize,
    pub open: usize,
    pub contacted: usize,
    pub paidout: usize,
}

// ─── Pure helpers ────────────────────────────────────────────────────

/// D-12: Client-side Filter — Backend liefert immer alle Entries (Phase 8 D-10).
pub fn filter_entries_by_status(
    entries: &[RepaymentEntryTO],
    filter: StatusFilter,
) -> Vec<RepaymentEntryTO> {
    entries
        .iter()
        .filter(|e| match filter {
            StatusFilter::All => true,
            StatusFilter::Open => matches!(e.status, RepaymentEntryStatusTO::Open),
            StatusFilter::Contacted => matches!(e.status, RepaymentEntryStatusTO::Contacted),
            StatusFilter::PaidOut => matches!(e.status, RepaymentEntryStatusTO::PaidOut),
        })
        .cloned()
        .collect()
}

/// D-12: Count-Badges fuer die Tab-Strip-im-Tab Status-Filter.
pub fn entry_counts_by_status(entries: &[RepaymentEntryTO]) -> StatusCounts {
    let mut c = StatusCounts {
        all: entries.len(),
        open: 0,
        contacted: 0,
        paidout: 0,
    };
    for e in entries {
        match e.status {
            RepaymentEntryStatusTO::Open => c.open += 1,
            RepaymentEntryStatusTO::Contacted => c.contacted += 1,
            RepaymentEntryStatusTO::PaidOut => c.paidout += 1,
        }
    }
    c
}

/// Client-Side-Join Member ↔ Entry via MEMBERS-Global-Signal (D-10).
/// Bei Member-Mismatch (member_id nicht in MEMBERS-Liste) liefert None —
/// Caller rendert dann "—" als defensive UX (Pitfall 8).
pub fn member_for_entry<'a>(
    entry: &RepaymentEntryTO,
    members: &'a [MemberTO],
) -> Option<&'a MemberTO> {
    members.iter().find(|m| m.id == Some(entry.member_id))
}

/// D-14 Default-Sort: Mitgliedsnummer ASC, created ASC sekundaer.
/// Entries ohne Member-Match (member_for_entry == None) sortieren ans Ende.
pub fn sort_entries_default(
    entries: &[RepaymentEntryTO],
    members: &[MemberTO],
) -> Vec<RepaymentEntryTO> {
    let mut result: Vec<RepaymentEntryTO> = entries.to_vec();
    result.sort_by(|a, b| {
        let ma = member_for_entry(a, members);
        let mb = member_for_entry(b, members);
        match (ma, mb) {
            (Some(a_m), Some(b_m)) => a_m
                .member_number
                .cmp(&b_m.member_number)
                .then_with(|| a.created.cmp(&b.created)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.created.cmp(&b.created),
        }
    });
    result
}

// ─── Component ───────────────────────────────────────────────────────

/// RepaymentEntryList (Phase 12 Plan 12-08, erweitert Plan 12-09).
///
/// ## Reload-Trigger
///
/// `reload_trigger: u64` ist ein einfacher Counter-Prop. Der Caller (z.B.
/// Detail-Page) erhoeht den Counter um 1 nach jeder Mutation, die NICHT
/// vom Component selbst ausgeloest wurde (z.B. nach Add-Modal-on_created).
/// Der Component lieset den Counter im `use_effect` als implizite Read-
/// Dependency — jede Counter-Aenderung loest ein neues load_entries() aus.
#[component]
pub fn RepaymentEntryList(
    phase: RepaymentPhaseTO,
    reload_trigger: u64,
    on_changed: EventHandler<()>,
    on_add: EventHandler<()>,
    on_paidout_request: EventHandler<Vec<RepaymentEntryTO>>,
    on_mail_request: EventHandler<Vec<Uuid>>,
    /// Phase 13 D-13-03: Callback fuer Bulk-Letter-Request.
    /// Receives `entry_ids` (NICHT `member_ids`) — Server aggregiert
    /// pro Member via Resolver (D-13-04).
    on_letter_request: EventHandler<Vec<Uuid>>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let phase_id = phase.id;
    let phase_status = phase.status;
    let share_value = phase.share_value;
    // D-08: bei phase.status == Closed wird die Component zur reinen Lese-Ansicht
    let readonly_mode = matches!(phase_status, RepaymentPhaseStatusTO::Closed);

    let mut entries = use_signal(Vec::<RepaymentEntryTO>::new);
    let mut loading = use_signal(|| true);
    let mut selected_ids = use_signal(Vec::<Uuid>::new);
    let mut status_filter = use_signal(|| StatusFilter::All);
    let mut delete_confirm_for = use_signal(|| Option::<Uuid>::None);

    let load_entries = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_repayment_entries(&config, phase_id).await {
                Ok(list) => entries.set(list),
                Err(e) => on_error.call(e.message),
            }
            loading.set(false);
        });
    };

    // Pitfall 8: parallel MEMBERS-Refresh damit Client-Side-Join nicht stale ist
    use_effect(move || {
        // Plan 12-09: reload_trigger als implizite Dep mitlesen → Counter-Aenderung loest Re-Run aus.
        // Bei jeder Counter-Inkrementierung vom Caller (z.B. nach Add-Modal-on_created) wird der
        // Effect erneut gefeuert und ein frischer load_entries() angestossen.
        let _ = reload_trigger;
        spawn(async move {
            crate::service::member::refresh_members().await;
        });
        load_entries();
    });

    // Computed: Member-Liste fuer Client-Side-Join
    let members_state = MEMBERS.read();
    let members_vec = members_state.items.clone();
    let entries_vec = entries.read().clone();
    let counts = entry_counts_by_status(&entries_vec);
    let filtered = filter_entries_by_status(&entries_vec, *status_filter.read());
    let sorted = sort_entries_default(&filtered, &members_vec);

    let selected_count = selected_ids.read().len();

    rsx! {
        div { class: "flex flex-col gap-4",

            // ── Status-Filter-Tab-Strip-im-Tab (D-12) ──
            div { class: "flex gap-2 border-b pb-2",
                StatusFilterTab {
                    label: format!("{} ({})", i18n.t(Key::RepaymentEntryFilterAll), counts.all),
                    is_selected: matches!(*status_filter.read(), StatusFilter::All),
                    on_click: move |_| status_filter.set(StatusFilter::All),
                }
                StatusFilterTab {
                    label: format!("{} ({})", i18n.t(Key::RepaymentEntryStatusOpen), counts.open),
                    is_selected: matches!(*status_filter.read(), StatusFilter::Open),
                    on_click: move |_| status_filter.set(StatusFilter::Open),
                }
                StatusFilterTab {
                    label: format!("{} ({})", i18n.t(Key::RepaymentEntryStatusContacted), counts.contacted),
                    is_selected: matches!(*status_filter.read(), StatusFilter::Contacted),
                    on_click: move |_| status_filter.set(StatusFilter::Contacted),
                }
                StatusFilterTab {
                    label: format!("{} ({})", i18n.t(Key::RepaymentEntryStatusPaidOut), counts.paidout),
                    is_selected: matches!(*status_filter.read(), StatusFilter::PaidOut),
                    on_click: move |_| status_filter.set(StatusFilter::PaidOut),
                }
            }

            // ── Header-Action-Leiste (D-11) ──
            if !readonly_mode {
                div { class: "flex flex-wrap gap-2 items-center",
                    button {
                        r#type: "button",
                        class: "bg-blue-600 hover:bg-blue-700 text-white px-3 py-2 rounded text-sm min-h-[44px]",
                        onclick: move |_| on_add.call(()),
                        "{i18n.t(Key::RepaymentEntryAdd)}"
                    }
                    button {
                        r#type: "button",
                        class: if selected_count == 0 {
                            "bg-gray-200 text-gray-500 px-3 py-2 rounded text-sm cursor-not-allowed min-h-[44px]"
                        } else {
                            "bg-blue-600 hover:bg-blue-700 text-white px-3 py-2 rounded text-sm min-h-[44px]"
                        },
                        disabled: selected_count == 0,
                        onclick: move |_| {
                            // UAT-Defekt #4 Fix: on_mail_request erwartet MEMBER-IDs
                            // (build_mail_redirect_url → /mail?members=<member-uuid>...)
                            // — selected_ids enthält aber ENTRY-IDs. Wir mappen
                            // pro Entry auf entry.member_id.
                            let selected_set = selected_ids.read().clone();
                            let member_ids: Vec<Uuid> = entries
                                .read()
                                .iter()
                                .filter(|e| selected_set.contains(&e.id))
                                .map(|e| e.member_id)
                                .collect();
                            on_mail_request.call(member_ids);
                        },
                        "{i18n.t(Key::RepaymentEntryBulkMailButton)} ({selected_count})"
                    }
                    // Phase 13 D-13-01..03: Bulk-Letter-Button.
                    // KRITISCH r#type: "button" Phase 12 D-01 — Page-Reload-Bug.
                    // Visueller Unterschied zum Mail-Button (Purple statt Blau),
                    // damit der Vorstand die beiden Bulk-Aktionen klar trennen kann.
                    button {
                        r#type: "button",
                        class: if selected_count == 0 {
                            "bg-gray-200 text-gray-500 px-3 py-2 rounded text-sm cursor-not-allowed min-h-[44px]"
                        } else {
                            "bg-purple-600 hover:bg-purple-700 text-white px-3 py-2 rounded text-sm min-h-[44px]"
                        },
                        disabled: selected_count == 0,
                        onclick: move |_| {
                            // D-13-03: entry_ids (NICHT member_ids) — Server
                            // aggregiert via Resolver D-13-04. Selection bleibt
                            // hier UNVERAENDERT (D-13-09 Selection-Preservation),
                            // damit der Vorstand direkt anschliessend mit dem
                            // Phase-8-Batch-Endpoint "Als angeschrieben markieren"
                            // auf der gleichen Auswahl fortsetzen kann.
                            let selected_set = selected_ids.read().clone();
                            let ids: Vec<Uuid> = entries
                                .read()
                                .iter()
                                .filter(|e| selected_set.contains(&e.id))
                                .map(|e| e.id)
                                .collect();
                            on_letter_request.call(ids);
                        },
                        "{i18n.t(Key::RepaymentEntryBulkLetterButton)} ({selected_count})"
                    }
                    button {
                        r#type: "button",
                        class: if selected_count == 0 {
                            "bg-gray-200 text-gray-500 px-3 py-2 rounded text-sm cursor-not-allowed min-h-[44px]"
                        } else {
                            "bg-blue-600 hover:bg-blue-700 text-white px-3 py-2 rounded text-sm min-h-[44px]"
                        },
                        disabled: selected_count == 0,
                        onclick: move |_| {
                            let ids = selected_ids.read().clone();
                            spawn(async move {
                                let config = CONFIG.read().clone();
                                let req = BatchStatusRequest {
                                    entry_ids: ids,
                                    target_status: RepaymentEntryStatusTO::Contacted,
                                };
                                match api::batch_toggle_repayment_status(&config, &req).await {
                                    Ok(()) => {
                                        selected_ids.set(Vec::new());
                                        on_changed.call(());
                                    }
                                    Err(e) => on_error.call(e.message),
                                }
                            });
                        },
                        "{i18n.t(Key::RepaymentEntryMarkContacted)} ({selected_count})"
                    }
                    button {
                        r#type: "button",
                        class: if selected_count == 0 {
                            "bg-gray-200 text-gray-500 px-3 py-2 rounded text-sm cursor-not-allowed min-h-[44px]"
                        } else {
                            "bg-red-600 hover:bg-red-700 text-white px-3 py-2 rounded text-sm min-h-[44px]"
                        },
                        disabled: selected_count == 0,
                        onclick: move |_| {
                            let ids: Vec<Uuid> = selected_ids.read().clone();
                            let selected_entries: Vec<RepaymentEntryTO> = entries
                                .read()
                                .iter()
                                .filter(|e| ids.contains(&e.id))
                                .cloned()
                                .collect();
                            on_paidout_request.call(selected_entries);
                        },
                        "{i18n.t(Key::RepaymentEntryMarkPaidOut)} ({selected_count})"
                    }
                }
            }

            // ── Tabelle ──
            if *loading.read() {
                p { class: "text-gray-500 text-center py-8", "{i18n.t(Key::Loading)}" }
            } else if sorted.is_empty() {
                div { class: "text-center py-12 text-gray-500",
                    if matches!(*status_filter.read(), StatusFilter::All) && entries_vec.is_empty() {
                        "{i18n.t(Key::RepaymentEntryEmptyAutoFill)}"
                    } else {
                        "{i18n.t(Key::RepaymentEntryEmptyFilter)}"
                    }
                }
            } else {
                table { class: "min-w-full divide-y divide-gray-200 text-sm",
                    thead { class: "bg-gray-50",
                        tr {
                            if !readonly_mode {
                                th { class: "px-2 py-2",
                                    // Header-Checkbox "Alle auswaehlen" (D-11)
                                    input {
                                        r#type: "checkbox",
                                        checked: {
                                            let sorted_ids: Vec<Uuid> = sorted.iter().map(|e| e.id).collect();
                                            !sorted_ids.is_empty()
                                                && sorted_ids.iter().all(|id| selected_ids.read().contains(id))
                                        },
                                        onchange: {
                                            let sorted_ids: Vec<Uuid> = sorted.iter().map(|e| e.id).collect();
                                            move |ev: Event<FormData>| {
                                                let checked = ev.value() == "true";
                                                if checked {
                                                    selected_ids.set(sorted_ids.clone());
                                                } else {
                                                    selected_ids.set(Vec::new());
                                                }
                                            }
                                        },
                                    }
                                }
                            }
                            th { class: "px-2 py-2 text-left", "{i18n.t(Key::RepaymentEntryColMemberNumber)}" }
                            th { class: "px-2 py-2 text-left", "{i18n.t(Key::RepaymentEntryColName)}" }
                            th { class: "px-2 py-2 text-right", "{i18n.t(Key::RepaymentEntryColShares)}" }
                            th { class: "px-2 py-2 text-right", "{i18n.t(Key::RepaymentEntryColAmount)}" }
                            th { class: "px-2 py-2 text-left", "{i18n.t(Key::RepaymentEntryColIban)}" }
                            th { class: "px-2 py-2 text-left", "{i18n.t(Key::RepaymentEntryColStatus)}" }
                            if !readonly_mode {
                                th { class: "px-2 py-2 text-right", "{i18n.t(Key::RepaymentEntryColActions)}" }
                            }
                        }
                    }
                    tbody { class: "divide-y divide-gray-200 bg-white",
                        for e in sorted.iter() {
                            {
                                let entry = e.clone();
                                let member = member_for_entry(&entry, &members_vec).cloned();
                                let member_number = member
                                    .as_ref()
                                    .map(|m| m.member_number.to_string())
                                    .unwrap_or_else(|| "—".into());
                                let name = member
                                    .as_ref()
                                    .map(|m| format!("{} {}", m.first_name, m.last_name))
                                    .unwrap_or_else(|| "—".into());
                                let iban = member
                                    .as_ref()
                                    .and_then(|m| m.bank_account.clone())
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or_else(|| "—".into());
                                let amount_str = format_payout_eur(entry.share_count_to_pay_out, share_value);
                                let entry_id = entry.id;
                                let entry_version = entry.version;
                                let entry_status = entry.status;
                                let entry_share_count = entry.share_count_to_pay_out;
                                let is_paidout = matches!(entry_status, RepaymentEntryStatusTO::PaidOut);
                                let is_selected = selected_ids.read().contains(&entry_id);
                                rsx! {
                                    tr { key: "{entry_id}",
                                        if !readonly_mode {
                                            td { class: "px-2 py-2",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_selected,
                                                    onchange: move |ev: Event<FormData>| {
                                                        let checked = ev.value() == "true";
                                                        if checked {
                                                            selected_ids.write().push(entry_id);
                                                        } else {
                                                            selected_ids.write().retain(|id| *id != entry_id);
                                                        }
                                                    },
                                                }
                                            }
                                        }
                                        td { class: "px-2 py-2", "{member_number}" }
                                        td { class: "px-2 py-2", "{name}" }
                                        td { class: "px-2 py-2 text-right",
                                            if readonly_mode || is_paidout {
                                                span { "{entry_share_count}" }
                                            } else {
                                                EditableShareCountCell {
                                                    value: entry_share_count,
                                                    disabled: false,
                                                    on_save: move |new_count: i32| {
                                                        let Some(version) = entry_version else {
                                                            on_error.call(
                                                                "Eintrag hat keine Version — bitte neu laden".into(),
                                                            );
                                                            return;
                                                        };
                                                        let req = UpdateRepaymentEntryRequest {
                                                            share_count_to_pay_out: Some(new_count),
                                                            status: None,
                                                            version,
                                                        };
                                                        spawn(async move {
                                                            let config = CONFIG.read().clone();
                                                            match api::update_repayment_entry(&config, entry_id, &req).await {
                                                                Ok(_) => on_changed.call(()),
                                                                Err(e) => on_error.call(e.message),
                                                            }
                                                        });
                                                    },
                                                }
                                            }
                                        }
                                        td { class: "px-2 py-2 text-right", "{amount_str}" }
                                        td { class: "px-2 py-2", "{iban}" }
                                        td { class: "px-2 py-2",
                                            RepaymentEntryStatusBadge { status: entry_status }
                                        }
                                        if !readonly_mode {
                                            td { class: "px-2 py-2 text-right",
                                                if !is_paidout {
                                                    button {
                                                        r#type: "button",
                                                        class: "text-red-600 hover:text-red-800 px-2",
                                                        title: i18n.t(Key::RepaymentEntryDelete).to_string(),
                                                        onclick: move |_| delete_confirm_for.set(Some(entry_id)),
                                                        "🗑"
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

            // ── Delete-Confirm-Modal (D-14) ──
            if let Some(eid) = *delete_confirm_for.read() {
                Modal {
                    div { class: "flex flex-col gap-4",
                        h2 { class: "text-xl font-semibold", "{i18n.t(Key::RepaymentEntryDeleteConfirm)}" }
                        div { class: "flex gap-2 justify-end",
                            button {
                                r#type: "button",
                                class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                                onclick: move |_| delete_confirm_for.set(None),
                                "{i18n.t(Key::Cancel)}"
                            }
                            button {
                                r#type: "button",
                                class: "bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded min-h-[44px]",
                                onclick: move |_| {
                                    delete_confirm_for.set(None);
                                    spawn(async move {
                                        let config = CONFIG.read().clone();
                                        match api::delete_repayment_entry(&config, eid).await {
                                            Ok(()) => on_changed.call(()),
                                            Err(e) => on_error.call(e.message),
                                        }
                                    });
                                },
                                "{i18n.t(Key::RepaymentEntryDelete)}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusFilterTab(label: String, is_selected: bool, on_click: EventHandler<()>) -> Element {
    let class = if is_selected {
        "px-3 py-1 rounded bg-blue-500 text-white text-sm min-h-[44px]"
    } else {
        "px-3 py-1 rounded bg-gray-200 hover:bg-gray-300 text-gray-700 text-sm min-h-[44px]"
    };
    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            onclick: move |_| on_click.call(()),
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rest_types::MemberStatusTO;
    use time::macros::date;
    use uuid::Uuid;

    /// Option A (Issue #3): MemberTO hat KEIN #[derive(Default)]
    /// (verifiziert via `rg "impl Default for MemberTO|#\[derive\([^)]*Default" rest-types/src/lib.rs` → 0).
    /// Daher listet make_member ALLE Felder explizit auf — KEIN ..Default::default()-Spread.
    /// Wenn das Backend-Schema waechst (neues Feld), bricht der Compiler hier — bewusste Pflicht-Sync.
    fn make_member(id: Uuid, number: i64) -> MemberTO {
        MemberTO {
            id: Some(id),
            member_number: number,
            first_name: format!("First{number}"),
            last_name: format!("Last{number}"),
            salutation: None,
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date!(2020 - 01 - 01),
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            account_holder: None,
            status: MemberStatusTO::Normal,
            postal_status: rest_types::PostalStatusTO::Erreichbar,
            created: None,
            deleted: None,
            version: None,
        }
    }

    fn make_entry(
        id: Uuid,
        member_id: Uuid,
        status: RepaymentEntryStatusTO,
        created_iso: &str,
    ) -> RepaymentEntryTO {
        RepaymentEntryTO {
            id,
            member_id,
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 1,
            status,
            created: Some(created_iso.into()),
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn filter_by_status_open() {
        let e1 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Contacted,
            "2026-01-02T00:00:00Z",
        );
        let e3 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-03T00:00:00Z",
        );
        let filtered = filter_entries_by_status(&[e1, e2, e3], StatusFilter::Open);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_by_status_all_returns_all() {
        let e1 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::PaidOut,
            "2026-01-02T00:00:00Z",
        );
        let filtered = filter_entries_by_status(&[e1, e2], StatusFilter::All);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn counts_correct() {
        let e1 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-02T00:00:00Z",
        );
        let e3 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Contacted,
            "2026-01-03T00:00:00Z",
        );
        let e4 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::PaidOut,
            "2026-01-04T00:00:00Z",
        );
        let c = entry_counts_by_status(&[e1, e2, e3, e4]);
        assert_eq!(c.all, 4);
        assert_eq!(c.open, 2);
        assert_eq!(c.contacted, 1);
        assert_eq!(c.paidout, 1);
    }

    #[test]
    fn counts_empty_returns_zeros() {
        let c = entry_counts_by_status(&[]);
        assert_eq!(c.all, 0);
        assert_eq!(c.open, 0);
        assert_eq!(c.contacted, 0);
        assert_eq!(c.paidout, 0);
    }

    #[test]
    fn member_for_entry_finds_match() {
        let mid = Uuid::new_v4();
        let m = make_member(mid, 42);
        let e = make_entry(
            Uuid::new_v4(),
            mid,
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(
            member_for_entry(&e, &[m]).map(|m| m.member_number),
            Some(42)
        );
    }

    #[test]
    fn member_for_entry_returns_none_on_mismatch() {
        let m = make_member(Uuid::new_v4(), 42);
        let e = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        assert!(member_for_entry(&e, &[m]).is_none());
    }

    #[test]
    fn sort_by_member_number_asc() {
        let m1 = make_member(Uuid::new_v4(), 100);
        let m2 = make_member(Uuid::new_v4(), 50);
        let m3 = make_member(Uuid::new_v4(), 75);
        let e1 = make_entry(
            Uuid::new_v4(),
            m1.id.unwrap(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            m2.id.unwrap(),
            RepaymentEntryStatusTO::Open,
            "2026-01-02T00:00:00Z",
        );
        let e3 = make_entry(
            Uuid::new_v4(),
            m3.id.unwrap(),
            RepaymentEntryStatusTO::Open,
            "2026-01-03T00:00:00Z",
        );
        let members = vec![m1.clone(), m2.clone(), m3.clone()];
        let sorted = sort_entries_default(&[e1, e2, e3], &members);
        assert_eq!(
            member_for_entry(&sorted[0], &members)
                .unwrap()
                .member_number,
            50
        );
        assert_eq!(
            member_for_entry(&sorted[1], &members)
                .unwrap()
                .member_number,
            75
        );
        assert_eq!(
            member_for_entry(&sorted[2], &members)
                .unwrap()
                .member_number,
            100
        );
    }

    #[test]
    fn sort_entries_without_member_at_end() {
        let m1 = make_member(Uuid::new_v4(), 50);
        let m1_id = m1.id.unwrap();
        let unknown_member_id = Uuid::new_v4();
        let e1 = make_entry(
            Uuid::new_v4(),
            unknown_member_id,
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            m1_id,
            RepaymentEntryStatusTO::Open,
            "2026-01-02T00:00:00Z",
        );
        let sorted = sort_entries_default(&[e1, e2], &[m1]);
        // Member-matched first, unmatched last
        assert_eq!(sorted[0].member_id, m1_id);
        assert_eq!(sorted[1].member_id, unknown_member_id);
    }
}
