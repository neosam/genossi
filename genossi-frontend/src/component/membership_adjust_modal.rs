//! Phase 18 — MembershipAdjustModal: Single-Component-Modal mit 4 flat Sub-Choice-Buttons
//! und 4 Operation-Sub-Views (Kuendigung, Teil-Rueckgabe, Uebertrag, Aufstockung).
//!
//! Architektur (D-18-02 + D-18-03):
//! - ModalStep-Enum + match-rsx im selben File
//! - Single-File ~700 LOC ist explizit gewollt; Sub-Component-Split haette Prop-Drilling-Overhead
//! - Component-First-Prinzip: Modal selbst ist die Shared-Component, interne Sub-Views sind privat
//! - Sub-Views als private `fn render_*_sub_view(i18n: I18n, ...)` (I18n: Clone verifiziert in
//!   i18n/mod.rs:908)
//!
//! SC-2 Default-today(): Jede Sub-View initialisiert ihr Datum-Signal als
//! `use_signal(|| Some(today))` — Datepicker zeigt direkt heute vor. Caller (Page) reicht
//! `today` als Prop in den Modal hinein.
//!
//! Closing-Anker fuer Roadmap-SC-1 (Modal als shared Component), SC-3 (Vorschau-Section),
//! CANC-06 (Vorschau-Confirm).

use dioxus::prelude::*;
use rest_types::{MemberSlimTO, MemberStatusTO, MemberTO};
use uuid::Uuid;

use crate::api::{self, AppError};
use crate::component::{is_valid_fiscal_year_date, ErrorAlert, FiscalYearDateInput, MemberSearch};
use crate::i18n::{use_i18n, I18n, Key};
use crate::service::config::CONFIG;

// ─── Pure helpers (testable) ────────────────────────────────────────

/// Phase 18 D-18 Claude's-Discretion — Frontend mirror of Backend `compute_effective_date`
/// (genossi_service_impl/src/membership_adjust.rs:700-720).
///
/// Returns `(target_fiscal_year, effective_date)` where effective_date is the December 31
/// of either the current calendar year (H1: Jan-Jun) or the next year (H2: Jul-Dec).
///
/// Used in Cancel + PartialRepayment previews to show "Stichtag" and "Phase FY{YYYY}"
/// without backend round-trip. Backend stays single source of truth — the actual Action
/// uses Backend's compute_effective_date.
pub fn compute_effective_date_mirror(willensbekundung: time::Date) -> (i32, time::Date) {
    let year = willensbekundung.year();
    if willensbekundung.month() <= time::Month::June {
        (
            year,
            time::Date::from_calendar_date(year, time::Month::December, 31).unwrap(),
        )
    } else {
        (
            year + 1,
            time::Date::from_calendar_date(year + 1, time::Month::December, 31).unwrap(),
        )
    }
}

/// Phase 18 D-18-07 — Returns true iff transferring `shares` from a source with
/// `from_current_shares` would leave the source at exactly 0 shares (= full transfer).
/// Requires `shares >= 1` (zero or negative shares are invalid, return false).
pub fn is_voll_uebertrag(shares: i32, from_current_shares: i32) -> bool {
    shares >= 1 && from_current_shares - shares == 0
}

/// Phase 18 D-18-13 — Adapter: MemberSlimTO (DSGVO-konformer Slim, 7 fields) → MemberTO
/// (full struct, all PII fields set to None/default).
/// Used to feed MemberSearch the Transfer-Recipients without leaking PII.
pub fn to_member_to(slim: &MemberSlimTO) -> MemberTO {
    MemberTO {
        id: Some(slim.id),
        member_number: slim.member_number,
        first_name: slim.first_name.clone(),
        last_name: slim.last_name.clone(),
        salutation: slim.salutation.clone(),
        title: slim.title.clone(),
        email: None,
        company: None,
        comment: None,
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        join_date: time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
        shares_at_joining: 0,
        current_shares: 0,
        current_balance: 0,
        action_count: 0,
        migrated: false,
        exit_date: None,
        bank_account: None,
        status: MemberStatusTO::Normal,
        created: None,
        deleted: None,
        version: None,
    }
}

/// Phase 18 — Format `time::Date` as `DD.MM.YYYY` (always German format, independent of
/// i18n locale, because the preview text is mixed German + numbers and uses German format
/// consistently).
pub fn format_date_german(d: &time::Date) -> String {
    format!("{:02}.{:02}.{:04}", d.day(), d.month() as u8, d.year())
}

// ─── Modal-Step state machine ───────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum ModalStep {
    SubChoice,
    Cancel,
    PartialRepayment,
    Transfer,
    Upgrade,
}

// ─── Component ──────────────────────────────────────────────────────

#[component]
pub fn MembershipAdjustModal(
    member: MemberTO,
    today: time::Date,
    on_close: EventHandler<()>,
    on_success: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut step = use_signal(|| ModalStep::SubChoice);

    // SC-2 — Default today(): each Sub-View initialises with `Some(today)`.
    // Shared signal across sub-views (persists when user goes back and re-enters).
    let date_signal = use_signal::<Option<time::Date>>(|| Some(today));
    // Operation-specific state (reset on Sub-View switch via Sub-Choice renderer).
    let shares_signal = use_signal::<i32>(|| 1);
    let recipient_id_signal = use_signal::<Option<Uuid>>(|| None);

    let submitting = use_signal(|| false);
    let error_signal = use_signal::<Option<AppError>>(|| None);

    let header_title = i18n.t(Key::MembershipAdjustModalTitle).to_string();

    rsx! {
        div { class: "flex flex-col gap-4",
            // ── Modal header (always visible) ──
            div { class: "flex items-center justify-between border-b border-gray-200 pb-3",
                h2 { class: "text-xl font-semibold text-gray-900",
                    "{header_title}"
                }
                button {
                    r#type: "button",
                    class: "text-gray-500 hover:text-gray-700 px-2 py-1",
                    onclick: move |_| on_close.call(()),
                    "\u{2715}"
                }
            }

            // ── Error display (always above Sub-View body) — AppError is Clone (api.rs:14) ──
            {
                let err_opt = error_signal.read().clone();
                if let Some(err) = err_opt {
                    let mut err_sig = error_signal;
                    rsx! {
                        ErrorAlert {
                            error: err,
                            on_dismiss: Some(EventHandler::new(move |_| err_sig.set(None))),
                        }
                    }
                } else {
                    rsx! {}
                }
            }

            // ── Step body ── (I18n: Clone verified in i18n/mod.rs:908)
            {
                let current = *step.read();
                match current {
                    ModalStep::SubChoice => render_sub_choice(
                        i18n.clone(),
                        step,
                        shares_signal,
                        recipient_id_signal,
                    ),
                    ModalStep::Cancel => render_cancel_sub_view(
                        i18n.clone(),
                        member.clone(),
                        today,
                        date_signal,
                        submitting,
                        error_signal,
                        on_success,
                        step,
                    ),
                    ModalStep::PartialRepayment => render_partial_sub_view(
                        i18n.clone(),
                        member.clone(),
                        today,
                        date_signal,
                        shares_signal,
                        submitting,
                        error_signal,
                        on_success,
                        step,
                    ),
                    ModalStep::Transfer => render_transfer_sub_view(
                        i18n.clone(),
                        member.clone(),
                        today,
                        date_signal,
                        shares_signal,
                        recipient_id_signal,
                        submitting,
                        error_signal,
                        on_success,
                        step,
                    ),
                    ModalStep::Upgrade => render_upgrade_sub_view(
                        i18n.clone(),
                        member.clone(),
                        today,
                        date_signal,
                        shares_signal,
                        submitting,
                        error_signal,
                        on_success,
                        step,
                    ),
                }
            }
        }
    }
}

// ─── Sub-Choice (4 flat Buttons) ────────────────────────────────────

fn render_sub_choice(
    i18n: I18n,
    mut step: Signal<ModalStep>,
    mut shares_signal: Signal<i32>,
    mut recipient_id_signal: Signal<Option<Uuid>>,
) -> Element {
    // Reset operation-specific fields on every Sub-Choice display.
    // (date_signal is NOT reset — persists across sub-view re-entry per CONTEXT.md Discretion)
    shares_signal.set(1);
    recipient_id_signal.set(None);

    let q = i18n.t(Key::MembershipAdjustSubChoiceQuestion).to_string();
    let cancel_label = i18n.t(Key::MembershipAdjustSubChoiceCancel).to_string();
    let cancel_desc = i18n.t(Key::MembershipAdjustSubChoiceCancelDesc).to_string();
    let partial_label = i18n
        .t(Key::MembershipAdjustSubChoicePartialRepayment)
        .to_string();
    let partial_desc = i18n
        .t(Key::MembershipAdjustSubChoicePartialRepaymentDesc)
        .to_string();
    let transfer_label = i18n.t(Key::MembershipAdjustSubChoiceTransfer).to_string();
    let transfer_desc = i18n
        .t(Key::MembershipAdjustSubChoiceTransferDesc)
        .to_string();
    let upgrade_label = i18n.t(Key::MembershipAdjustSubChoiceUpgrade).to_string();
    let upgrade_desc = i18n
        .t(Key::MembershipAdjustSubChoiceUpgradeDesc)
        .to_string();

    rsx! {
        div { class: "flex flex-col gap-4",
            p { class: "text-sm text-gray-700", "{q}" }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                button {
                    r#type: "button",
                    class: "flex flex-col items-start p-6 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition text-left",
                    onclick: move |_| step.set(ModalStep::Cancel),
                    span { class: "text-lg font-semibold", "{cancel_label}" }
                    span { class: "text-sm text-gray-600 mt-1", "{cancel_desc}" }
                }
                button {
                    r#type: "button",
                    class: "flex flex-col items-start p-6 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition text-left",
                    onclick: move |_| step.set(ModalStep::PartialRepayment),
                    span { class: "text-lg font-semibold", "{partial_label}" }
                    span { class: "text-sm text-gray-600 mt-1", "{partial_desc}" }
                }
                button {
                    r#type: "button",
                    class: "flex flex-col items-start p-6 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition text-left",
                    onclick: move |_| step.set(ModalStep::Transfer),
                    span { class: "text-lg font-semibold", "{transfer_label}" }
                    span { class: "text-sm text-gray-600 mt-1", "{transfer_desc}" }
                }
                button {
                    r#type: "button",
                    class: "flex flex-col items-start p-6 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition text-left",
                    onclick: move |_| step.set(ModalStep::Upgrade),
                    span { class: "text-lg font-semibold", "{upgrade_label}" }
                    span { class: "text-sm text-gray-600 mt-1", "{upgrade_desc}" }
                }
            }
        }
    }
}

// ─── Sub-View: Cancel ───────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_cancel_sub_view(
    i18n: I18n,
    member: MemberTO,
    today: time::Date,
    mut date_signal: Signal<Option<time::Date>>,
    mut submitting: Signal<bool>,
    mut error_signal: Signal<Option<AppError>>,
    on_success: EventHandler<()>,
    mut step: Signal<ModalStep>,
) -> Element {
    let back_label = i18n.t(Key::MembershipAdjustBack).to_string();
    let title = i18n.t(Key::MembershipAdjustCancelTitle).to_string();
    let date_label = i18n.t(Key::MembershipAdjustCancelDateLabel).to_string();
    let preview_label = i18n.t(Key::MembershipAdjustPreviewLabel).to_string();
    let submit_label = i18n.t(Key::MembershipAdjustCancelSubmit).to_string();
    let cancel_label = i18n.t(Key::MembershipAdjustCancelButton).to_string();

    // Preview text live
    let preview_text = {
        let date_val = *date_signal.read();
        match date_val {
            None => String::new(),
            Some(d) => {
                if !is_valid_fiscal_year_date(d, today) {
                    String::new()
                } else {
                    let (fy, eff) = compute_effective_date_mirror(d);
                    let half_year = if d.month() <= time::Month::June {
                        i18n.t(Key::MembershipAdjustHalfYearH1).to_string()
                    } else {
                        i18n.t(Key::MembershipAdjustHalfYearH2).to_string()
                    };
                    let template = i18n.t(Key::MembershipAdjustCancelPreview);
                    let name = format!("{} {}", member.first_name, member.last_name);
                    template
                        .replace("{name}", &name)
                        .replace("{shares}", &member.current_shares.to_string())
                        .replace("{effective_date}", &format_date_german(&eff))
                        .replace("{half_year}", &half_year)
                        .replace("{fiscal_year}", &fy.to_string())
                }
            }
        }
    };

    let member_id = member.id;
    let date_for_submit = *date_signal.read();
    let is_valid = date_for_submit.map_or(false, |d| is_valid_fiscal_year_date(d, today));
    let is_submitting = *submitting.read();
    let disabled = !is_valid || is_submitting;

    rsx! {
        div { class: "flex flex-col gap-4",
            // Sub-view header
            div { class: "flex items-center gap-2",
                button {
                    r#type: "button",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{back_label}"
                }
                span { class: "text-gray-400", "\u{00B7}" }
                h3 { class: "text-xl font-semibold text-red-700", "{title}" }
            }

            // Form
            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{date_label}" }
                FiscalYearDateInput {
                    value: date_signal,
                    on_change: move |d: time::Date| { date_signal.set(Some(d)); },
                    today: today,
                }
            }

            // Preview
            if !preview_text.is_empty() {
                div { class: "mt-2 p-4 bg-blue-50 border border-blue-200 rounded",
                    h4 { class: "text-xs font-semibold uppercase text-blue-900 mb-2", "{preview_label}" }
                    p { class: "text-sm", "{preview_text}" }
                }
            }

            // Action row
            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded",
                    disabled: is_submitting,
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{cancel_label}"
                }
                button {
                    r#type: "button",
                    class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-semibold disabled:bg-gray-300 disabled:cursor-not-allowed",
                    disabled: disabled,
                    onclick: move |_| {
                        let Some(id) = member_id else { return; };
                        let Some(d) = date_for_submit else { return; };
                        submitting.set(true);
                        error_signal.set(None);
                        spawn(async move {
                            let config = CONFIG.read().clone();
                            match api::cancel_membership(&config, id, d).await {
                                Ok(_resp) => {
                                    submitting.set(false);
                                    on_success.call(());
                                }
                                Err(e) => {
                                    submitting.set(false);
                                    error_signal.set(Some(e));
                                }
                            }
                        });
                    },
                    if is_submitting { "\u{2026}" } else { "{submit_label}" }
                }
            }
        }
    }
}

// ─── Sub-View: PartialRepayment ─────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_partial_sub_view(
    i18n: I18n,
    member: MemberTO,
    today: time::Date,
    mut date_signal: Signal<Option<time::Date>>,
    mut shares_signal: Signal<i32>,
    mut submitting: Signal<bool>,
    mut error_signal: Signal<Option<AppError>>,
    on_success: EventHandler<()>,
    mut step: Signal<ModalStep>,
) -> Element {
    let back_label = i18n.t(Key::MembershipAdjustBack).to_string();
    let title = i18n
        .t(Key::MembershipAdjustPartialRepaymentTitle)
        .to_string();
    let date_label = i18n
        .t(Key::MembershipAdjustPartialRepaymentDateLabel)
        .to_string();
    let shares_label = i18n
        .t(Key::MembershipAdjustPartialRepaymentSharesLabel)
        .to_string();
    let preview_label = i18n.t(Key::MembershipAdjustPreviewLabel).to_string();
    let submit_label = i18n
        .t(Key::MembershipAdjustPartialRepaymentSubmit)
        .to_string();
    let cancel_label = i18n.t(Key::MembershipAdjustCancelButton).to_string();
    let shares_positive_err = i18n
        .t(Key::MembershipAdjustSharesMustBePositive)
        .to_string();
    let shares_exceed_err = i18n
        .t(Key::MembershipAdjustPartialRepaymentSharesExceed)
        .to_string();

    let shares_now = *shares_signal.read();
    let current = member.current_shares;

    let inline_error_text: Option<String> = if shares_now <= 0 {
        Some(shares_positive_err.clone())
    } else if shares_now >= current {
        Some(shares_exceed_err.clone())
    } else {
        None
    };

    let preview_text = {
        let date_val = *date_signal.read();
        match date_val {
            None => String::new(),
            Some(d) if !is_valid_fiscal_year_date(d, today) => String::new(),
            Some(_) if inline_error_text.is_some() => String::new(),
            Some(d) => {
                let (fy, eff) = compute_effective_date_mirror(d);
                let new_shares = current - shares_now;
                let template = i18n.t(Key::MembershipAdjustPartialRepaymentPreview);
                let name = format!("{} {}", member.first_name, member.last_name);
                template
                    .replace("{name}", &name)
                    .replace("{current_shares}", &current.to_string())
                    .replace("{new_shares}", &new_shares.to_string())
                    .replace("{effective_date}", &format_date_german(&eff))
                    .replace("{fiscal_year}", &fy.to_string())
            }
        }
    };

    let member_id = member.id;
    let date_for_submit = *date_signal.read();
    let is_valid = date_for_submit.map_or(false, |d| is_valid_fiscal_year_date(d, today))
        && shares_now >= 1
        && shares_now < current;
    let is_submitting = *submitting.read();
    let disabled = !is_valid || is_submitting;

    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "flex items-center gap-2",
                button {
                    r#type: "button",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{back_label}"
                }
                span { class: "text-gray-400", "\u{00B7}" }
                h3 { class: "text-xl font-semibold text-gray-900", "{title}" }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{date_label}" }
                FiscalYearDateInput {
                    value: date_signal,
                    on_change: move |d: time::Date| { date_signal.set(Some(d)); },
                    today: today,
                }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{shares_label}" }
                input {
                    r#type: "number",
                    min: "1",
                    max: "{current}",
                    value: "{shares_now}",
                    class: "w-full px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            shares_signal.set(n);
                        }
                    }
                }
                if let Some(err) = inline_error_text.as_ref() {
                    span { class: "text-red-600 text-sm", "{err}" }
                }
            }

            if !preview_text.is_empty() {
                div { class: "mt-2 p-4 bg-blue-50 border border-blue-200 rounded",
                    h4 { class: "text-xs font-semibold uppercase text-blue-900 mb-2", "{preview_label}" }
                    p { class: "text-sm", "{preview_text}" }
                }
            }

            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded",
                    disabled: is_submitting,
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{cancel_label}"
                }
                button {
                    r#type: "button",
                    class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-semibold disabled:bg-gray-300 disabled:cursor-not-allowed",
                    disabled: disabled,
                    onclick: move |_| {
                        let Some(id) = member_id else { return; };
                        let Some(d) = date_for_submit else { return; };
                        let n = shares_now;
                        submitting.set(true);
                        error_signal.set(None);
                        spawn(async move {
                            let config = CONFIG.read().clone();
                            match api::partial_repayment(&config, id, n, d).await {
                                Ok(_resp) => {
                                    submitting.set(false);
                                    on_success.call(());
                                }
                                Err(e) => {
                                    submitting.set(false);
                                    error_signal.set(Some(e));
                                }
                            }
                        });
                    },
                    if is_submitting { "\u{2026}" } else { "{submit_label}" }
                }
            }
        }
    }
}

// ─── Sub-View: Transfer ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_transfer_sub_view(
    i18n: I18n,
    member: MemberTO,
    today: time::Date,
    mut date_signal: Signal<Option<time::Date>>,
    mut shares_signal: Signal<i32>,
    mut recipient_id_signal: Signal<Option<Uuid>>,
    mut submitting: Signal<bool>,
    mut error_signal: Signal<Option<AppError>>,
    on_success: EventHandler<()>,
    mut step: Signal<ModalStep>,
) -> Element {
    let back_label = i18n.t(Key::MembershipAdjustBack).to_string();
    let title = i18n.t(Key::MembershipAdjustTransferTitle).to_string();
    let date_label = i18n.t(Key::MembershipAdjustTransferDateLabel).to_string();
    let shares_label = i18n.t(Key::MembershipAdjustTransferSharesLabel).to_string();
    let recipient_label = i18n.t(Key::MembershipAdjustTransferRecipientLabel).to_string();
    let recipient_load_err = i18n
        .t(Key::MembershipAdjustTransferRecipientLoadError)
        .to_string();
    let no_recipients = i18n.t(Key::MembershipAdjustNoRecipients).to_string();
    let loading_label = i18n.t(Key::MembershipAdjustLoading).to_string();
    let preview_label = i18n.t(Key::MembershipAdjustPreviewLabel).to_string();
    let submit_label = i18n.t(Key::MembershipAdjustTransferSubmit).to_string();
    let cancel_label = i18n.t(Key::MembershipAdjustCancelButton).to_string();
    let self_error = i18n.t(Key::MembershipAdjustTransferSelfError).to_string();
    let shares_positive_err = i18n
        .t(Key::MembershipAdjustSharesMustBePositive)
        .to_string();

    let from_id_opt = member.id;
    let from_id = match from_id_opt {
        Some(id) => id,
        None => return rsx! { div { "Member has no id; cannot transfer." } },
    };

    let recipients_resource = use_resource(move || async move {
        let config = CONFIG.read().clone();
        api::get_transfer_recipients(&config, from_id).await
    });

    let shares_now = *shares_signal.read();
    let current = member.current_shares;
    let recipient_id_val = *recipient_id_signal.read();

    // Self-Transfer-Block (Frontend mirror of TRSF-07 backend)
    let inline_self_err: Option<String> = if recipient_id_val == Some(from_id) {
        Some(self_error.clone())
    } else {
        None
    };
    let inline_shares_err: Option<String> = if shares_now < 1 || shares_now > current {
        Some(shares_positive_err.clone())
    } else {
        None
    };

    // Voll-Uebertrag detection (D-18-07)
    let is_full = is_voll_uebertrag(shares_now, current);

    // Preview text (live)
    let preview_text = {
        let date_val = *date_signal.read();
        let recipient_to: Option<MemberTO> =
            match (&*recipients_resource.read(), recipient_id_val) {
                (Some(Ok(list)), Some(rid)) => list.iter().find(|s| s.id == rid).map(to_member_to),
                _ => None,
            };
        match (date_val, recipient_to) {
            (None, _) | (_, None) => String::new(),
            (Some(d), Some(_))
                if !is_valid_fiscal_year_date(d, today)
                    || inline_self_err.is_some()
                    || inline_shares_err.is_some() =>
            {
                String::new()
            }
            (Some(d), Some(to)) => {
                let from_new = current - shares_now;
                let to_current = to.current_shares;
                let to_new = to_current + shares_now;
                let template = i18n.t(Key::MembershipAdjustTransferPreview);
                let from_name = format!("{} {}", member.first_name, member.last_name);
                let to_name = format!("{} {}", to.first_name, to.last_name);
                template
                    .replace("{from_name}", &from_name)
                    .replace("{from_shares}", &current.to_string())
                    .replace("{from_new}", &from_new.to_string())
                    .replace("{to_name}", &to_name)
                    .replace("{to_shares}", &to_current.to_string())
                    .replace("{to_new}", &to_new.to_string())
                    .replace("{transfer_date}", &format_date_german(&d))
            }
        }
    };

    let voll_warning_text = if is_full && !preview_text.is_empty() {
        let date_val = *date_signal.read();
        match date_val {
            Some(d) => {
                let template = i18n.t(Key::MembershipAdjustTransferFullExitWarning);
                let from_name = format!("{} {}", member.first_name, member.last_name);
                template
                    .replace("{from_name}", &from_name)
                    .replace("{transfer_date}", &format_date_german(&d))
            }
            None => String::new(),
        }
    } else {
        String::new()
    };

    let date_for_submit = *date_signal.read();
    let is_valid = date_for_submit.map_or(false, |d| is_valid_fiscal_year_date(d, today))
        && shares_now >= 1
        && shares_now <= current
        && recipient_id_val.is_some()
        && recipient_id_val != Some(from_id);
    let is_submitting = *submitting.read();
    let disabled = !is_valid || is_submitting;

    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "flex items-center gap-2",
                button {
                    r#type: "button",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{back_label}"
                }
                span { class: "text-gray-400", "\u{00B7}" }
                h3 { class: "text-xl font-semibold text-gray-900", "{title}" }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{date_label}" }
                FiscalYearDateInput {
                    value: date_signal,
                    on_change: move |d: time::Date| { date_signal.set(Some(d)); },
                    today: today,
                }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{shares_label}" }
                input {
                    r#type: "number",
                    min: "1",
                    max: "{current}",
                    value: "{shares_now}",
                    class: "w-full px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            shares_signal.set(n);
                        }
                    }
                }
                if let Some(err) = inline_shares_err.as_ref() {
                    span { class: "text-red-600 text-sm", "{err}" }
                }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{recipient_label}" }
                {
                    match &*recipients_resource.read() {
                        None => rsx! { p { class: "text-sm text-gray-500", "{loading_label}" } },
                        Some(Err(_)) => rsx! { p { class: "text-sm text-red-600", "{recipient_load_err}" } },
                        Some(Ok(list)) if list.is_empty() => rsx! {
                            p { class: "text-sm text-gray-500", "{no_recipients}" }
                        },
                        Some(Ok(list)) => {
                            let adapted: Vec<MemberTO> = list.iter().map(to_member_to).collect();
                            rsx! {
                                MemberSearch {
                                    on_select: move |maybe_id: Option<Uuid>| {
                                        recipient_id_signal.set(maybe_id);
                                    },
                                    selected_id: recipient_id_val,
                                    exclude_id: Some(from_id),
                                    members_override: Some(adapted),
                                }
                            }
                        }
                    }
                }
                if let Some(err) = inline_self_err.as_ref() {
                    span { class: "text-red-600 text-sm", "{err}" }
                }
            }

            if !preview_text.is_empty() {
                div { class: "mt-2 p-4 bg-blue-50 border border-blue-200 rounded",
                    h4 { class: "text-xs font-semibold uppercase text-blue-900 mb-2", "{preview_label}" }
                    p { class: "text-sm", "{preview_text}" }
                    if !voll_warning_text.is_empty() {
                        p { class: "mt-2 text-orange-700 font-bold", "{voll_warning_text}" }
                    }
                }
            }

            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded",
                    disabled: is_submitting,
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{cancel_label}"
                }
                button {
                    r#type: "button",
                    class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-semibold disabled:bg-gray-300 disabled:cursor-not-allowed",
                    disabled: disabled,
                    onclick: move |_| {
                        let Some(d) = date_for_submit else { return; };
                        let Some(to_id) = recipient_id_val else { return; };
                        let n = shares_now;
                        submitting.set(true);
                        error_signal.set(None);
                        spawn(async move {
                            let config = CONFIG.read().clone();
                            match api::transfer_shares(&config, from_id, to_id, n, d).await {
                                Ok(_resp) => {
                                    submitting.set(false);
                                    on_success.call(());
                                }
                                Err(e) => {
                                    submitting.set(false);
                                    error_signal.set(Some(e));
                                }
                            }
                        });
                    },
                    if is_submitting { "\u{2026}" } else { "{submit_label}" }
                }
            }
        }
    }
}

// ─── Sub-View: Upgrade ──────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_upgrade_sub_view(
    i18n: I18n,
    member: MemberTO,
    today: time::Date,
    mut date_signal: Signal<Option<time::Date>>,
    mut shares_signal: Signal<i32>,
    mut submitting: Signal<bool>,
    mut error_signal: Signal<Option<AppError>>,
    on_success: EventHandler<()>,
    mut step: Signal<ModalStep>,
) -> Element {
    let back_label = i18n.t(Key::MembershipAdjustBack).to_string();
    let title = i18n.t(Key::MembershipAdjustUpgradeTitle).to_string();
    let date_label = i18n.t(Key::MembershipAdjustUpgradeDateLabel).to_string();
    let shares_label = i18n.t(Key::MembershipAdjustUpgradeSharesLabel).to_string();
    let preview_label = i18n.t(Key::MembershipAdjustPreviewLabel).to_string();
    let submit_label = i18n.t(Key::MembershipAdjustUpgradeSubmit).to_string();
    let cancel_label = i18n.t(Key::MembershipAdjustCancelButton).to_string();
    let shares_err = i18n
        .t(Key::MembershipAdjustSharesMustBePositive)
        .to_string();

    let shares_now = *shares_signal.read();
    let current = member.current_shares;

    let inline_err: Option<String> = if shares_now < 1 { Some(shares_err) } else { None };

    let preview_text = {
        let date_val = *date_signal.read();
        match date_val {
            None => String::new(),
            Some(d) if !is_valid_fiscal_year_date(d, today) => String::new(),
            Some(_) if inline_err.is_some() => String::new(),
            Some(d) => {
                let new_shares = current + shares_now;
                let template = i18n.t(Key::MembershipAdjustUpgradePreview);
                let name = format!("{} {}", member.first_name, member.last_name);
                template
                    .replace("{name}", &name)
                    .replace("{current_shares}", &current.to_string())
                    .replace("{new_shares}", &new_shares.to_string())
                    .replace("{date}", &format_date_german(&d))
            }
        }
    };

    let member_id = member.id;
    let date_for_submit = *date_signal.read();
    let is_valid =
        date_for_submit.map_or(false, |d| is_valid_fiscal_year_date(d, today)) && shares_now >= 1;
    let is_submitting = *submitting.read();
    let disabled = !is_valid || is_submitting;

    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "flex items-center gap-2",
                button {
                    r#type: "button",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{back_label}"
                }
                span { class: "text-gray-400", "\u{00B7}" }
                h3 { class: "text-xl font-semibold text-gray-900", "{title}" }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{date_label}" }
                FiscalYearDateInput {
                    value: date_signal,
                    on_change: move |d: time::Date| { date_signal.set(Some(d)); },
                    today: today,
                }
            }

            div { class: "flex flex-col gap-2",
                label { class: "block text-sm font-medium text-gray-700", "{shares_label}" }
                input {
                    r#type: "number",
                    min: "1",
                    value: "{shares_now}",
                    class: "w-full px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            shares_signal.set(n);
                        }
                    }
                }
                if let Some(err) = inline_err.as_ref() {
                    span { class: "text-red-600 text-sm", "{err}" }
                }
            }

            if !preview_text.is_empty() {
                div { class: "mt-2 p-4 bg-blue-50 border border-blue-200 rounded",
                    h4 { class: "text-xs font-semibold uppercase text-blue-900 mb-2", "{preview_label}" }
                    p { class: "text-sm", "{preview_text}" }
                }
            }

            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded",
                    disabled: is_submitting,
                    onclick: move |_| step.set(ModalStep::SubChoice),
                    "{cancel_label}"
                }
                button {
                    r#type: "button",
                    class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-semibold disabled:bg-gray-300 disabled:cursor-not-allowed",
                    disabled: disabled,
                    onclick: move |_| {
                        let Some(id) = member_id else { return; };
                        let Some(d) = date_for_submit else { return; };
                        let n = shares_now;
                        submitting.set(true);
                        error_signal.set(None);
                        spawn(async move {
                            let config = CONFIG.read().clone();
                            match api::increase_shares(&config, id, n, d).await {
                                Ok(_resp) => {
                                    submitting.set(false);
                                    on_success.call(());
                                }
                                Err(e) => {
                                    submitting.set(false);
                                    error_signal.set(Some(e));
                                }
                            }
                        });
                    },
                    if is_submitting { "\u{2026}" } else { "{submit_label}" }
                }
            }
        }
    }
}

// ─── Tests for pure helpers ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month};

    fn d(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).unwrap()
    }

    // ── compute_effective_date_mirror — 6 tests ──

    #[test]
    fn compute_effective_h1_june_30_boundary() {
        let (fy, eff) = compute_effective_date_mirror(d(2026, Month::June, 30));
        assert_eq!(fy, 2026);
        assert_eq!(eff, d(2026, Month::December, 31));
    }

    #[test]
    fn compute_effective_h2_july_1_boundary() {
        let (fy, eff) = compute_effective_date_mirror(d(2026, Month::July, 1));
        assert_eq!(fy, 2027);
        assert_eq!(eff, d(2027, Month::December, 31));
    }

    #[test]
    fn compute_effective_h2_year_end() {
        let (fy, eff) = compute_effective_date_mirror(d(2026, Month::December, 31));
        assert_eq!(fy, 2027);
        assert_eq!(eff, d(2027, Month::December, 31));
    }

    #[test]
    fn compute_effective_h1_year_start() {
        let (fy, eff) = compute_effective_date_mirror(d(2026, Month::January, 1));
        assert_eq!(fy, 2026);
        assert_eq!(eff, d(2026, Month::December, 31));
    }

    #[test]
    fn compute_effective_h1_leap_year_feb_29() {
        let (fy, eff) = compute_effective_date_mirror(d(2024, Month::February, 29));
        assert_eq!(fy, 2024);
        assert_eq!(eff, d(2024, Month::December, 31));
    }

    #[test]
    fn compute_effective_h1_mid_year() {
        let (fy, eff) = compute_effective_date_mirror(d(2026, Month::June, 15));
        assert_eq!(fy, 2026);
        assert_eq!(eff, d(2026, Month::December, 31));
    }

    // ── is_voll_uebertrag — 3 tests ──

    #[test]
    fn voll_uebertrag_eq_returns_true() {
        assert!(is_voll_uebertrag(5, 5));
    }

    #[test]
    fn voll_uebertrag_lt_returns_false() {
        assert!(!is_voll_uebertrag(3, 5));
    }

    #[test]
    fn voll_uebertrag_zero_shares_returns_false() {
        assert!(!is_voll_uebertrag(0, 5));
    }

    // ── to_member_to — 1 test ──

    #[test]
    fn to_member_to_drops_pii_fields() {
        let slim = MemberSlimTO {
            id: Uuid::from_u128(99),
            member_number: 42,
            salutation: None,
            title: None,
            first_name: "Anna".into(),
            last_name: "Weber".into(),
        };
        let mt = to_member_to(&slim);
        assert_eq!(mt.id, Some(slim.id));
        assert_eq!(mt.member_number, 42);
        assert_eq!(mt.first_name, "Anna");
        assert!(mt.email.is_none(), "email must be None");
        assert!(mt.street.is_none(), "street must be None");
        assert!(mt.postal_code.is_none(), "postal_code must be None");
        assert!(mt.city.is_none(), "city must be None");
        assert!(mt.bank_account.is_none(), "bank_account must be None");
        assert_eq!(mt.current_shares, 0);
        assert_eq!(mt.status, MemberStatusTO::Normal);
    }

    // ── format_date_german — 2 tests ──

    #[test]
    fn format_date_german_simple() {
        assert_eq!(format_date_german(&d(2026, Month::June, 15)), "15.06.2026");
    }

    #[test]
    fn format_date_german_year_end() {
        assert_eq!(
            format_date_german(&d(2026, Month::December, 31)),
            "31.12.2026"
        );
    }
}
