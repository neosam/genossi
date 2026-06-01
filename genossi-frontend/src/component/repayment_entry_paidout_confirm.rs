//! PaidOut-Bulk-Confirm-Modal (Phase 12 Plan 12-10, UI-05).
//!
//! Single-Backend-Endpoint (D-15) -> Sequential-Loop pro Entry.
//! Confirm-Inhalt: Listentabelle + Gesamtsumme + 3-Punkt-Warnung (D-16).
//! Pro Loop-Fehler ein show_toast (D-17); am Ende 1 Summary-Toast (D-15).
//! Nach Loop: refresh_members (Pitfall 3 — current_shares-Cascade).
//!
//! ## Pure Helper
//!
//! `sum_payout_amounts(entries, share_value_cents)` ist die testbare
//! Reine-Funktion fuer die D-16 Gesamt-Summe-Anzeige im Modal.

use dioxus::prelude::*;

use crate::api::{self, RepaymentEntryTO};
use crate::component::repayment_format::format_payout_eur;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::MEMBERS;

/// D-16: Gesamt-Auszahlungs-Summe in Cent.
///
/// Pure function: total = sum(entry.share_count_to_pay_out * share_value_cents).
/// Wird im Modal-Footer als "Summe: X €" angezeigt.
pub fn sum_payout_amounts(entries: &[RepaymentEntryTO], share_value_cents: i64) -> i64 {
    entries
        .iter()
        .map(|e| (e.share_count_to_pay_out as i64) * share_value_cents)
        .sum()
}

/// D-15/D-16/D-17 Bulk-Confirm-Modal-Component.
///
/// Wird in der Detail-Page via `Modal { RepaymentEntryPaidOutConfirm { ... } }`
/// gemountet, sobald RepaymentEntryList die `on_paidout_request`-Liste an die
/// Page weiterreicht.
///
/// Inhalt: Listentabelle (Mitgl.-Nr. + Name + Anteile + Betrag) — Gesamtsumme —
/// 3-Punkt-Warnliste in rot — Cancel + roter "Endgueltig markieren"-Button.
///
/// Klick auf Endgueltig markieren startet einen Sequential-Loop ueber alle
/// Entries und ruft pro Entry `api::mark_repayment_entry_paid_out`. Pro Fehler
/// wird `on_error` gerufen (D-17 Toast). Am Ende `refresh_members().await`
/// (Pitfall 3 — current_shares-Cascade) und `on_complete((success, failure))`
/// (Summary-Toast wird vom Caller formuliert).
#[component]
pub fn RepaymentEntryPaidOutConfirm(
    entries: Vec<RepaymentEntryTO>,
    share_value_cents: i64,
    on_close: EventHandler<()>,
    on_complete: EventHandler<(usize, usize)>, // (success_count, failure_count)
    on_error: EventHandler<String>,            // per-entry-error Toasts
) -> Element {
    let i18n = use_i18n();
    let mut submitting = use_signal(|| false);
    let entries_for_render = entries.clone();
    let entries_for_submit = entries.clone();
    let total_sum = sum_payout_amounts(&entries, share_value_cents);

    rsx! {
        div { class: "flex flex-col gap-4",
            h2 { class: "text-xl font-semibold text-red-700",
                "{i18n.t(Key::RepaymentEntryPaidOutConfirmTitle)}"
            }

            // D-16: Listentabelle der ausgewaehlten Eintraege
            table { class: "min-w-full text-sm",
                thead {
                    tr {
                        th { class: "px-2 py-1 text-left", "{i18n.t(Key::RepaymentEntryColMemberNumber)}" }
                        th { class: "px-2 py-1 text-left", "{i18n.t(Key::RepaymentEntryColName)}" }
                        th { class: "px-2 py-1 text-right", "{i18n.t(Key::RepaymentEntryColShares)}" }
                        th { class: "px-2 py-1 text-right", "{i18n.t(Key::RepaymentEntryColAmount)}" }
                    }
                }
                tbody {
                    for entry in entries_for_render.iter() {
                        {
                            // Member-Lookup pro Row (defensive '—'-Fallback wenn MEMBERS
                            // noch nicht geladen ist — analog Plan 12-08 Pitfall 8).
                            let members_state = MEMBERS.read();
                            let member = members_state
                                .items
                                .iter()
                                .find(|m| m.id == Some(entry.member_id));
                            let member_number = member
                                .map(|m| m.member_number.to_string())
                                .unwrap_or_else(|| "—".into());
                            let name = member
                                .map(|m| format!("{} {}", m.first_name, m.last_name))
                                .unwrap_or_else(|| "—".into());
                            let amount =
                                format_payout_eur(entry.share_count_to_pay_out, share_value_cents);
                            let shares = entry.share_count_to_pay_out;
                            let entry_id = entry.id;
                            rsx! {
                                tr { key: "{entry_id}",
                                    td { class: "px-2 py-1", "{member_number}" }
                                    td { class: "px-2 py-1", "{name}" }
                                    td { class: "px-2 py-1 text-right", "{shares}" }
                                    td { class: "px-2 py-1 text-right", "{amount}" }
                                }
                            }
                        }
                    }
                }
            }

            // Gesamtsumme (D-16) — format_payout_eur(1, total_sum) ergibt "X,YY €"
            // weil total_sum bereits in Cent ist (share_count=1 * total_sum_cents).
            div { class: "text-right font-bold text-lg",
                "{i18n.t(Key::RepaymentEntryPaidOutConfirmSum)} {format_payout_eur(1, total_sum)}"
            }

            // D-16: 3-Punkt-Warnliste in roter Farbe
            ul { class: "text-sm text-red-700 list-disc list-inside bg-red-50 p-3 rounded",
                li { "{i18n.t(Key::RepaymentEntryPaidOutConfirmWarn1)}" }
                li { "{i18n.t(Key::RepaymentEntryPaidOutConfirmWarn2)}" }
                li { "{i18n.t(Key::RepaymentEntryPaidOutConfirmWarn3)}" }
            }

            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                    disabled: *submitting.read(),
                    onclick: move |_| on_close.call(()),
                    "{i18n.t(Key::Cancel)}"
                }
                button {
                    r#type: "button",
                    class: "bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded disabled:opacity-50 min-h-[44px]",
                    disabled: *submitting.read(),
                    onclick: move |_| {
                        submitting.set(true);
                        let entries = entries_for_submit.clone();
                        spawn(async move {
                            let config = CONFIG.read().clone();
                            let mut success_count = 0usize;
                            let mut failure_count = 0usize;
                            for entry in entries.iter() {
                                match api::mark_repayment_entry_paid_out(&config, entry.id).await {
                                    Ok(_) => success_count += 1,
                                    Err(e) => {
                                        failure_count += 1;
                                        // D-17: per-Entry-Toast (deutsche Meldung)
                                        on_error.call(format!(
                                            "Eintrag {}: {}",
                                            entry.id, e.message
                                        ));
                                    }
                                }
                            }
                            // Pitfall 3: nach Bulk-Cascade MEMBERS refreshen
                            // (current_shares hat sich serverseitig geaendert).
                            crate::service::member::refresh_members().await;
                            // D-15: Summary-Toast-Trigger via Caller-Discretion.
                            on_complete.call((success_count, failure_count));
                        });
                    },
                    "{i18n.t(Key::RepaymentEntryPaidOutConfirmButton)}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::RepaymentEntryStatusTO;
    use uuid::Uuid;

    fn make_entry(share_count: i32) -> RepaymentEntryTO {
        RepaymentEntryTO {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: share_count,
            status: RepaymentEntryStatusTO::Open,
            created: None,
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn sum_single_entry() {
        assert_eq!(sum_payout_amounts(&[make_entry(1)], 10_000), 10_000);
        assert_eq!(sum_payout_amounts(&[make_entry(5)], 10_000), 50_000);
    }

    #[test]
    fn sum_multiple_entries() {
        let entries = vec![make_entry(2), make_entry(3), make_entry(1)];
        assert_eq!(sum_payout_amounts(&entries, 10_000), 60_000);
    }

    #[test]
    fn sum_empty_returns_zero() {
        assert_eq!(sum_payout_amounts(&[], 10_000), 0);
    }

    #[test]
    fn sum_zero_share_count_defensive() {
        assert_eq!(sum_payout_amounts(&[make_entry(0)], 10_000), 0);
    }

    #[test]
    fn sum_realistic_phase_total() {
        // 5 Mitglieder mit je 3 Anteilen, 100 EUR pro Anteil = 1.500 EUR
        let entries: Vec<_> = (0..5).map(|_| make_entry(3)).collect();
        assert_eq!(sum_payout_amounts(&entries, 10_000), 150_000); // 1500,00 EUR in Cent
    }
}
