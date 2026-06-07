# Phase 18: frontend-component-first - Pattern Map

**Mapped:** 2026-06-07
**Files analyzed:** 8 (2 new + 6 modified)
**Analogs found:** 8 / 8 (alle CONTEXT.md-genannten Quellen verifiziert)

> **Lese-Reihenfolge fuer den Planner:** Diese Datei deckt jede Datei in Phase 18 mit konkretem Analog-Code. Jeder Excerpt traegt Datei + Zeilenbereich. Der Planner kann Tasks formulieren, ohne weitere Code-Lookups zu machen — alle Patterns sind hier.

---

## File Classification

| Datei | Status | Role | Data Flow | Closest Analog | Match Quality |
|-------|--------|------|-----------|----------------|---------------|
| `genossi-frontend/src/component/membership_adjust_modal.rs` | NEW | component (modal) | request-response (4 ops) | `component/repayment_entry_paidout_confirm.rs` | exact (Modal-mit-Submit-Loop + on_close/on_complete-Pattern) |
| `genossi-frontend/src/component/fiscal_year_date_input.rs` | NEW | component (form-input) | input/validation | `page/member_details.rs` (Datepicker-Helpers + Eintrittsdatum-Input) | role-match (extracted aus inline-Page-Code) |
| `genossi-frontend/src/api.rs` | MODIFY | api-client | request-response | bestehende `api::update_member` (Z. 214) | exact (gleicher `format!`+POST+JSON-Pattern) |
| `genossi-frontend/src/page/member_details.rs` | MODIFY | page (mount) | UI-state | `page/repayment_phases.rs` (Z. 64-130 Modal-Mount) | exact (lokaler `use_signal<bool>` Toggle + Modal-Wrapper + RequirePrivilege) |
| `genossi-frontend/src/i18n/mod.rs` | MODIFY | i18n (enum-erweiterung) | data | existing `Key`-Varianten (Z. 46-706) | exact (Key-Enum erweitern) |
| `genossi-frontend/src/i18n/de.rs` | MODIFY | i18n (translation) | data | existing `match key` (Z. 4-633) | exact (match-arm pro Key) |
| `genossi-frontend/src/i18n/en.rs` | MODIFY | i18n (translation) | data | existing `match key` | exact |
| `genossi-frontend/src/component/mod.rs` | MODIFY | module re-export | n/a | bestehende `pub use` (Z. 28-46) | exact (Section + `pub use`) |
| `genossi-frontend/rest-types/src/lib.rs` | MODIFY (siehe Landmine L-2) | TO-types | data | bestehende `MemberTO` (Z. 189) | exact (Struct + Derive) |
| `genossi-frontend/src/component/toast.rs` | MODIFY (UI-SPEC Anforderung) | component | UI-state | bestehende `ToastContainer` (Z. 29-48) | exact (Variant-Param + bg-class-Switch) |

---

## Pattern Assignments

### 1) `component/membership_adjust_modal.rs` (NEW)

**Closest analog:** `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` (215 Zeilen — strukturell vollstaendiges Vorbild fuer Modal-mit-Submit-Loop + on_close/on_complete + i18n + Pure-Helper + `#[cfg(test)]`-Tests).

**Importpattern** (Z. 13-19 von paidout_confirm):
```rust
use dioxus::prelude::*;

use crate::api::{self, RepaymentEntryTO};
use crate::component::repayment_format::format_payout_eur;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::MEMBERS;
```
Phase 18 wird zusaetzlich brauchen:
```rust
use crate::component::{ErrorAlert, MemberSearch};
use crate::component::fiscal_year_date_input::FiscalYearDateInput;
use crate::service::member::refresh_members;
use rest_types::{MemberTO, MemberSlimTO};  // ggf. MembershipAdjustResponseTO etc. nach Landmine L-2
```

**Component-Signatur + on_close/on_complete-Pattern** (Z. 46-53 von paidout_confirm):
```rust
#[component]
pub fn RepaymentEntryPaidOutConfirm(
    entries: Vec<RepaymentEntryTO>,
    share_value_cents: i64,
    on_close: EventHandler<()>,
    on_complete: EventHandler<(usize, usize)>,
    on_error: EventHandler<String>,
) -> Element {
```
Phase 18 `MembershipAdjustModal` Props-Vorlage (gemaess D-18-08):
```rust
#[component]
pub fn MembershipAdjustModal(
    member: MemberTO,
    today: time::Date,
    on_close: EventHandler<()>,
    on_success: EventHandler<()>,
) -> Element {
```

**Submit-Button-Pattern (Dioxus-Button-Reload-Bug-Fix — C-18-CF-03)** (Z. 122-163 von paidout_confirm):
```rust
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
                /* ... API-Call ... */
                crate::service::member::refresh_members().await;
                on_complete.call((success_count, failure_count));
            });
        },
        "{i18n.t(Key::RepaymentEntryPaidOutConfirmButton)}"
    }
}
```
**WICHTIG:** `r#type: "button"` (NICHT `type="submit"`) + `onclick`-Handler (NICHT `<form onsubmit>`) — siehe Landmine L-3.

**Pure-Helper + Unit-Tests** (Z. 25-30 + Z. 168-215 von paidout_confirm):
```rust
pub fn sum_payout_amounts(entries: &[RepaymentEntryTO], share_value_cents: i64) -> i64 {
    entries.iter()
        .map(|e| (e.share_count_to_pay_out as i64) * share_value_cents)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    // ... 5 Tests fuer Pure-Function ...
    #[test]
    fn sum_single_entry() {
        assert_eq!(sum_payout_amounts(&[make_entry(1)], 10_000), 10_000);
    }
}
```
Phase 18 spiegelt das fuer:
- `compute_effective_date_mirror(date) -> (i32, time::Date)` mit 6 Tests (30.06, 01.07, 31.12, 01.01, 29.02 Schaltjahr, Jahres-Boundaries) — Backend-Quelle `genossi_service_impl/src/membership_adjust.rs:710-722`.
- `is_voll_uebertrag(shares: i32, from_current_shares: i32) -> bool` mit 3 Tests (eq → true, lt → false, edge zero → false).
- `is_valid_fiscal_year_date(date, today) -> bool` mit 4 Tests (current-year, next-year, prev-year reject, year-after-next reject) — Backend-Quelle `genossi_service_impl/src/membership_adjust.rs:739-756`.
- `to_member_to(slim: &MemberSlimTO) -> MemberTO` mit 1 Test (PII-Felder = None/default).
- `format_date_german(date) -> String` (DD.MM.YYYY) — ODER `i18n.format_date(&date)` mit `Locale::De` direkt nutzen (siehe Landmine L-4).

**Enum-State-Pattern fuer ModalStep (D-18-02, D-18-03)** — analog `member_details.rs:131` `use_signal(|| DocumentTypeTO::JoinDeclaration)` mit folgender Anwendung:
```rust
#[derive(Clone, Copy, PartialEq)]
enum ModalStep { SubChoice, Cancel, PartialRepayment, Transfer, Upgrade }

let mut step = use_signal(|| ModalStep::SubChoice);

rsx! {
    match *step.read() {
        ModalStep::SubChoice => rsx! { /* 4 flat Buttons */ },
        ModalStep::Cancel => rsx! { /* Cancel sub-view */ },
        // ...
    }
}
```

**Sub-Choice-Card-Layout** (gemaess UI-SPEC + D-18 Specifics):
```rust
div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
    button {
        r#type: "button",
        class: "flex flex-col items-start p-6 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition",
        onclick: move |_| step.set(ModalStep::Cancel),
        span { class: "text-lg font-semibold", "{i18n.t(Key::MembershipAdjustSubChoiceCancel)}" }
        span { class: "text-sm text-gray-600", "{i18n.t(Key::MembershipAdjustSubChoiceCancelDesc)}" }
    }
    /* 3 weitere */
}
```

**Vorschau-Box-Pattern** (UI-SPEC bg-blue-50):
```rust
div { class: "mt-4 p-4 bg-blue-50 border border-blue-200 rounded",
    h4 { class: "font-semibold text-sm uppercase text-blue-900 mb-2", "{i18n.t(Key::MembershipAdjustPreviewLabel)}" }
    p { class: "text-sm", "{vorschau_text}" }
    if is_voll_uebertrag {
        p { class: "mt-2 text-orange-700 font-bold", "⚠ {voll_warning}" }
    }
}
```

**Error-Anzeige innerhalb Modal (D-18-08, NICHT Toast)** — analog `member_details.rs:121` `let mut error: Signal<Option<api::AppError>> = use_signal(|| None);`:
```rust
if let Some(err) = error.read().as_ref() {
    ErrorAlert { error: err.clone(), on_dismiss: Some(EventHandler::new(move |_| error.set(None))) }
}
```

---

### 2) `component/fiscal_year_date_input.rs` (NEW)

**Closest analog:** `genossi-frontend/src/page/member_details.rs` (Datepicker-Helpers Z. 30-32 + Z. 63-73, sowie das inline-Eintrittsdatum-Input — wird durch FiscalYearDateInput abgeloest).

**Datepicker-Helper-Pattern** (member_details.rs Z. 30-32 + 63-73 — Phase 18 dupliziert minimal in der neuen Component-Datei ODER macht sie `pub(crate)`; Empfehlung: **duplizieren** weil 5 LOC und keine Risiko-Kopplung zur Page):
```rust
fn format_date_input(d: &time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

fn parse_date_input(s: &str) -> Option<time::Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 { return None; }
    let year: i32 = parts[0].parse().ok()?;
    let month: u8 = parts[1].parse().ok()?;
    let day: u8 = parts[2].parse().ok()?;
    let month: time::Month = month.try_into().ok()?;
    time::Date::from_calendar_date(year, month, day).ok()
}
```

**Component-Signatur** (D-18-10 + UI-SPEC):
```rust
#[component]
pub fn FiscalYearDateInput(
    value: Signal<Option<time::Date>>,
    on_change: EventHandler<time::Date>,
    today: time::Date,
) -> Element {
    let i18n = use_i18n();
    let min_year = today.year();
    let max_year = today.year() + 1;
    let min_str = format!("{:04}-01-01", min_year);
    let max_str = format!("{:04}-12-31", max_year);
    let current_value = value.read().as_ref().map(format_date_input).unwrap_or_default();
    let is_oor = value.read().as_ref()
        .map_or(false, |d| d.year() < min_year || d.year() > max_year);

    rsx! {
        div { class: "flex flex-col gap-1",
            input {
                r#type: "date",
                min: "{min_str}",
                max: "{max_str}",
                value: "{current_value}",
                class: format_args!("w-full px-3 py-2 border rounded {}",
                    if is_oor { "border-red-500" } else { "border-gray-300" }),
                oninput: move |e| {
                    if let Some(d) = parse_date_input(&e.value()) {
                        value.set(Some(d));
                        on_change.call(d);
                    }
                }
            }
            if is_oor {
                span { class: "text-red-600 text-sm", "{i18n.t(Key::FiscalYearDateOutOfRange)}" }
            }
            // Helper-Text mit Format-Args via .replace() (siehe Landmine L-4)
            {
                let template = i18n.t(Key::FiscalYearDateInputHelper);
                let helper = template
                    .replace("{min_year}", &min_year.to_string())
                    .replace("{max_year}", &max_year.to_string());
                rsx! { span { class: "text-gray-500 text-xs", "{helper}" } }
            }
        }
    }
}
```

**Pure-Function `is_valid_fiscal_year_date`** mit Tests (mirror von `genossi_service_impl/src/membership_adjust.rs:739-756`):
```rust
pub(crate) fn is_valid_fiscal_year_date(date: time::Date, today: time::Date) -> bool {
    let current_fy = today.year();
    date.year() == current_fy || date.year() == current_fy + 1
}
```

---

### 3) `api.rs` (MODIFY — 5 neue Funktionen)

**Closest analog:** `genossi-frontend/src/api.rs:188-228` (get_member / create_member / update_member / delete_member — identisches Pattern fuer alle 5 neuen Funktionen).

**Imports** (bereits vorhanden Z. 1-10 — keine Aenderungen noetig, evtl. neue TOs aus `rest_types`):
```rust
use rest_types::{MemberTO, MemberSlimTO, MemberActionTO,
    CancelMembershipRequestTO, IncreaseSharesRequestTO,
    PartialRepaymentRequestTO, PartialRepaymentResponseTO,
    TransferSharesRequestTO, TransferSharesResponseTO,
    MembershipAdjustResponseTO, RepaymentEntryTO, RepaymentPhaseTO};
```
**ACHTUNG Landmine L-2:** Diese Typen muessen erst in `genossi-frontend/rest-types/src/lib.rs` hinzugefuegt werden.

**POST-mit-JSON-Body-Pattern** (Z. 202-212 von create_member):
```rust
pub async fn create_member(config: &Config, member: MemberTO) -> Result<MemberTO, AppError> {
    info!("Creating member");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::Client::new()
        .post(url)
        .json(&member)
        .send()
        .await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}
```

**Konkrete neue Funktionen** (alle nach gleichem Muster):
```rust
/// Phase 18 / Phase 15 D-15-11: POST /api/members/{id}/cancel
pub async fn cancel_membership(
    config: &Config,
    member_id: Uuid,
    willensbekundung_date: time::Date,
) -> Result<MembershipAdjustResponseTO, AppError> {
    info!("Cancelling membership {member_id}");
    let url = format!("{}/api/members/{member_id}/cancel", config.backend);
    let body = CancelMembershipRequestTO { willensbekundung_date };
    let response = reqwest::Client::new().post(url).json(&body).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

/// Phase 18 / Phase 15 D-15-15: POST /api/members/{id}/increase-shares
pub async fn increase_shares(
    config: &Config,
    member_id: Uuid,
    shares: i32,
    willensbekundung_date: time::Date,
) -> Result<MembershipAdjustResponseTO, AppError> { /* analog */ }

/// Phase 18 / Phase 16 D-16-16: POST /api/members/{id}/partial-repayment
pub async fn partial_repayment(
    config: &Config,
    member_id: Uuid,
    shares: i32,
    willensbekundung_date: time::Date,
) -> Result<PartialRepaymentResponseTO, AppError> { /* analog */ }

/// Phase 18 / Phase 17 C-17-CF-07: POST /api/members/{from_id}/transfer-shares
pub async fn transfer_shares(
    config: &Config,
    from_id: Uuid,
    to_member_id: Uuid,
    shares: i32,
    transfer_date: time::Date,
) -> Result<TransferSharesResponseTO, AppError> { /* analog */ }

/// Phase 18 / Phase 14 D-14-12: GET /api/members/transfer-recipients?exclude_self={uuid}
pub async fn get_transfer_recipients(
    config: &Config,
    exclude_self: Uuid,
) -> Result<Vec<MemberSlimTO>, AppError> {
    info!("Fetching transfer recipients (exclude_self={exclude_self})");
    let url = format!("{}/api/members/transfer-recipients?exclude_self={exclude_self}",
        config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}
```

**Backend-Routen-Verifikation** (`genossi_rest/src/member.rs` + `genossi_rest/src/membership_adjust.rs`):
- `POST /api/members/{id}/cancel` → `cancel_membership` (Phase 15) — Response `MembershipAdjustResponseTO { action, member }`
- `POST /api/members/{id}/increase-shares` → `increase_shares` (Phase 15) — Response `MembershipAdjustResponseTO { action, member }`
- `POST /api/members/{id}/partial-repayment` → `partial_repayment` (Phase 16) — Response `PartialRepaymentResponseTO { entry, member, phase: Option<...> }`
- `POST /api/members/{from_id}/transfer-shares` → `transfer_shares` (Phase 17) — Response `TransferSharesResponseTO { actions, from, to }`
- `GET /api/members/transfer-recipients?exclude_self={uuid}` → `get_transfer_recipients` (Phase 14) — Response `Vec<MemberSlimTO>`

---

### 4) `page/member_details.rs` (MODIFY — Button + Modal-Mount)

**Closest analog:** `genossi-frontend/src/page/repayment_phases.rs:64-130` (RequirePrivilege + Button + Modal-mit-conditional-Mount).

**Imports-Erweiterung** (Z. 9 + neue Zeile):
```rust
use crate::component::{CommunicationTimeline, ErrorAlert, MemberSearch, Modal, TopBar,
    MembershipAdjustModal};  // NEU
use crate::auth::RequirePrivilege;  // NEU
use crate::service::member::refresh_members;  // NEU
```

**Lokales State-Signal** (analog Z. 122-130 — bestehende `show_delete_modal`):
```rust
let mut show_adjust_modal = use_signal(|| false);
```

**Today-Berechnung** (Pattern aus member_details.rs:82-90 fuer js_sys::Date):
```rust
let today = {
    let d = js_sys::Date::new_0();
    let year = d.get_full_year() as i32;
    let month: time::Month = (d.get_month() as u8 + 1).try_into()
        .unwrap_or(time::Month::January);
    let day = d.get_date() as u8;
    time::Date::from_calendar_date(year, month, day).unwrap_or_else(|_|
        time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
};
```

**Admin-only Button-Mount** (analog `repayment_phases.rs:65-77`):
```rust
RequirePrivilege {
    privilege: "admin",
    button {
        r#type: "button",
        class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm font-medium min-h-[44px]",
        onclick: move |_| show_adjust_modal.set(true),
        "{i18n.t(Key::MembershipAdjustButtonLabel)}"
    }
}
```
**Privilege-Konstante:** `"admin"` (NICHT `"manage_members"` — C-18-CF-11 / Phase 15 D-15-01 ADMIN_PRIVILEGE-Funnel).

**Conditional Modal-Mount** (analog `repayment_phases.rs:113-128`):
```rust
if *show_adjust_modal.read() {
    Modal {
        MembershipAdjustModal {
            member: member.read().clone(),
            today: today,
            on_close: move |_| show_adjust_modal.set(false),
            on_success: move |_| {
                show_adjust_modal.set(false);
                spawn(async move {
                    refresh_members().await;
                    // Member-Detail-State auch lokal refreshen:
                    if let Some(id) = member.read().id {
                        let config = CONFIG.read().clone();
                        if let Ok(data) = api::get_member(&config, id).await {
                            *member.write() = data;
                        }
                    }
                    show_toast(&mut toast_messages, &mut toast_counter,
                        i18n.t(Key::MembershipAdjustSuccess).to_string());
                });
            },
        }
    }
}
```

**ACHTUNG:** member_details.rs hat aktuell **keinen** Toast-State (`toast_messages`/`toast_counter`). Planner muss diese Signals zur Page hinzufuegen + `ToastContainer { messages: toast_messages.into() }` ans Ende der rsx-Baums anhaengen (analog `repayment_phases.rs:45-46 + Z. 130`).

---

### 5) `i18n/mod.rs` (MODIFY — Key-Enum erweitern)

**Closest analog:** `genossi-frontend/src/i18n/mod.rs:46-706` (Key-Enum mit ~700 bestehenden Varianten in Gruppen via Kommentar-Headers).

**Pattern fuer neue Sektion** (analog Z. 700-706):
```rust
// ─── Phase 18 ─── MembershipAdjustModal + FiscalYearDateInput ────
/// Button-Label auf der Member-Detail-Page.
MembershipAdjustButtonLabel,
/// Modal-Titel (top-level).
MembershipAdjustModalTitle,
/// Sub-Choice-Frage ueber den 4 flat Buttons.
MembershipAdjustSubChoiceQuestion,
/// Sub-Choice-Button-Labels.
MembershipAdjustSubChoiceCancel,
MembershipAdjustSubChoiceCancelDesc,
MembershipAdjustSubChoicePartialRepayment,
MembershipAdjustSubChoicePartialRepaymentDesc,
MembershipAdjustSubChoiceTransfer,
MembershipAdjustSubChoiceTransferDesc,
MembershipAdjustSubChoiceUpgrade,
MembershipAdjustSubChoiceUpgradeDesc,
/// Sub-View-Header.
MembershipAdjustBack,
MembershipAdjustCancelButton,
MembershipAdjustPreviewLabel,
/// Cancel Sub-View.
MembershipAdjustCancelTitle,
MembershipAdjustCancelDateLabel,
/// Format-Args: {name}, {shares}, {effective_date}, {half_year}, {fiscal_year}.
MembershipAdjustCancelPreview,
MembershipAdjustHalfYearH1,
MembershipAdjustHalfYearH2,
MembershipAdjustCancelSubmit,
MembershipAdjustCancelSuccess,
/// Partial-Repayment Sub-View.
MembershipAdjustPartialRepaymentTitle,
MembershipAdjustPartialRepaymentDateLabel,
MembershipAdjustPartialRepaymentSharesLabel,
/// Format-Args: {name}, {current_shares}, {new_shares}, {effective_date}, {fiscal_year}.
MembershipAdjustPartialRepaymentPreview,
/// Format-Args: {fiscal_year}.
MembershipAdjustPartialRepaymentAutoCreateHint,
MembershipAdjustPartialRepaymentSubmit,
MembershipAdjustPartialRepaymentSuccess,
/// Format-Args: {fiscal_year}.
MembershipAdjustPartialRepaymentSuccessAutoCreate,
/// Transfer Sub-View.
MembershipAdjustTransferTitle,
MembershipAdjustTransferDateLabel,
MembershipAdjustTransferSharesLabel,
MembershipAdjustTransferRecipientLabel,
MembershipAdjustTransferRecipientLoadError,
/// Format-Args: {from_name}, {from_shares}, {from_new}, {to_name}, {to_shares}, {to_new}, {transfer_date}.
MembershipAdjustTransferPreview,
/// Format-Args: {from_name}, {transfer_date}.
MembershipAdjustTransferFullExitWarning,
MembershipAdjustTransferSubmit,
MembershipAdjustTransferSuccess,
/// Upgrade Sub-View.
MembershipAdjustUpgradeTitle,
MembershipAdjustUpgradeDateLabel,
MembershipAdjustUpgradeSharesLabel,
/// Format-Args: {name}, {current_shares}, {new_shares}, {date}.
MembershipAdjustUpgradePreview,
MembershipAdjustUpgradeSubmit,
MembershipAdjustUpgradeSuccess,
/// Loading + Validation.
MembershipAdjustLoading,
MembershipAdjustNoRecipients,
MembershipAdjustSharesMustBePositive,
MembershipAdjustPartialRepaymentSharesExceed,
MembershipAdjustTransferSelfError,
/// Generic Success-Toast wenn nicht-spezifischer Op.
MembershipAdjustSuccess,
/// FiscalYearDateInput.
/// Format-Args: {min_year}, {max_year}.
FiscalYearDateInputHelper,
FiscalYearDateOutOfRange,
```

**~46 neue Keys** (uebersteigt UI-SPEC-Mindest-30 leicht; Planner kann entsprechend kuerzen, sofern UI-SPEC-Liste komplett bleibt).

---

### 6) + 7) `i18n/de.rs` + `i18n/en.rs` (MODIFY — Translations)

**Closest analog:** `genossi-frontend/src/i18n/de.rs:4-633` (match-arm pro Key).

**Pattern** (Z. 600-633):
```rust
Key::RepaymentLetterDownloadToastPlural => {
    "{count} Briefe heruntergeladen.".into()
}
Key::RepaymentLetterDownloadToastSkipped => {
    "{skipped} Datei(en) im Storage nicht gefunden — bitte erneut generieren."
        .into()
}
Key::RepaymentLetterDownloadToastFailure => "Download fehlgeschlagen: {error}".into(),
```

**Format-Args sind Platzhalter-Strings `{name}`** — wird im Call-Site via `i18n.t(Key).replace("{placeholder}", &value)` ersetzt (siehe Landmine L-4). Beispiel aus `repayment_letter_download_button.rs:143`:
```rust
let count_str = success_count.to_string();
let msg = success_plural.replace("{count}", &count_str);
```

**DE-Translations** kommen 1:1 aus UI-SPEC Z. 122-173 (Copywriting Contract). EN-Translations idem. **NUR `de.rs` + `en.rs`** — KEIN `cs.rs` (existiert nicht, siehe C-18-CF-02 + genossi-frontend/CLAUDE.md harter Constraint).

---

### 8) `component/mod.rs` (MODIFY — Re-Exports)

**Closest analog:** `genossi-frontend/src/component/mod.rs:112-118` (Section-Header + `pub mod` + `pub use`).

**Pattern** (Z. 112-118):
```rust
// ─── Phase 12 Plan 12-10 ─── RepaymentEntryPaidOutConfirm (UI-05) ────
pub mod repayment_entry_paidout_confirm;
pub use repayment_entry_paidout_confirm::{sum_payout_amounts, RepaymentEntryPaidOutConfirm};
```

**Phase 18 Erweiterung am Ende von mod.rs:**
```rust
// ─── Phase 18 ─── MembershipAdjustModal + FiscalYearDateInput ─────
pub mod membership_adjust_modal;
pub mod fiscal_year_date_input;
pub use membership_adjust_modal::{MembershipAdjustModal,
    compute_effective_date_mirror, is_voll_uebertrag, to_member_to, format_date_german};
pub use fiscal_year_date_input::{FiscalYearDateInput, is_valid_fiscal_year_date};
```

---

### 9) `genossi-frontend/rest-types/src/lib.rs` (MODIFY — Landmine L-2)

**Closest analog:** `genossi-frontend/rest-types/src/lib.rs:189` `pub struct MemberTO { ... }` (Pattern fuer neue Request/Response-Structs).

**Frontend rest-types vs Workspace genossi_rest_types — KRITISCHE LANDMINE:**

Die Frontend-Crate `genossi-frontend/rest-types/` (802 Zeilen) ist **NICHT** identisch mit dem Workspace `genossi_rest_types/` (2666 Zeilen). Sie ist eine separate, manuell synchronisierte Kopie der relevanten DTOs **ohne** `utoipa::ToSchema` und **ohne** Domain-`From<...>`-Konversionen.

**Was in Frontend rest-types FEHLT** (gemaess Grep — wird in Phase 18 benoetigt):
- `MemberSlimTO`
- `CancelMembershipRequestTO`
- `IncreaseSharesRequestTO`
- `MembershipAdjustResponseTO` (gemeinsam fuer Cancel + Upgrade)
- `PartialRepaymentRequestTO`
- `PartialRepaymentResponseTO`
- `TransferSharesRequestTO`
- `TransferSharesResponseTO`
- `RepaymentEntryTO` (Phase 12 — pruefen ob bereits im Frontend, wird im paidout_confirm.rs importiert ⇒ vorhanden)
- `RepaymentPhaseTO` (Phase 12 — idem, wird in repayment_phases.rs importiert ⇒ vorhanden)

**Pattern fuer Hinzufuegen** (gestrippt von `genossi_rest_types/src/lib.rs:512-609`):
```rust
// Phase 18: copy-from genossi_rest_types/src/lib.rs (ohne ToSchema/From-Impls,
// ohne `iso8601_date_required`-Serde — Frontend nutzt time::Date default-serde).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelMembershipRequestTO {
    pub willensbekundung_date: time::Date,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncreaseSharesRequestTO {
    pub willensbekundung_date: time::Date,
    pub shares: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MembershipAdjustResponseTO {
    pub action: MemberActionTO,
    pub member: MemberTO,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialRepaymentRequestTO {
    pub willensbekundung_date: time::Date,
    pub shares: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialRepaymentResponseTO {
    pub entry: RepaymentEntryTO,
    pub member: MemberTO,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub phase: Option<RepaymentPhaseTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferSharesRequestTO {
    pub to_member_id: Uuid,
    pub shares: i32,
    pub transfer_date: time::Date,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferSharesResponseTO {
    pub actions: Vec<MemberActionTO>,
    pub from: MemberTO,
    pub to: MemberTO,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberSlimTO {
    pub id: Uuid,
    pub member_number: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    pub first_name: String,
    pub last_name: String,
}
```

**Achtung Serde-Datums-Format:** Backend nutzt `iso8601_date_required` Custom-Serde fuer `willensbekundung_date` + `transfer_date` (siehe `genossi_rest_types/src/lib.rs:514-520`). Frontend rest-types nutzt aktuell `time::Date` mit `serde-human-readable`-Feature (siehe `genossi-frontend/rest-types/Cargo.toml:9`). **Verifikation noetig:** Stimmt das Default-Serde-Format mit `iso8601_date_required` ueberein (`YYYY-MM-DD`)? Pruefen anhand existing Frontend-DTO `MemberActionTO.date` (rest-types/src/lib.rs:351), das in Phase 4/12 bereits funktioniert.

---

### 10) `component/toast.rs` (MODIFY — UI-SPEC Anforderung)

**Closest analog:** `genossi-frontend/src/component/toast.rs` (komplette 48 Zeilen).

**Aktueller Code** (Z. 38-46 — alle Toasts sind hart **rot**):
```rust
for (id, msg) in msgs.iter() {
    div {
        key: "{id}",
        class: "bg-red-600 text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3",
        "{msg}"
    }
}
```

**Phase 18 muss Success-Toast (gruen) supporten** (UI-SPEC Color-Section + Toast-Variant-Requirement):

**Option (a) — empfohlen (UI-SPEC Z. 186-188):** Variant-Param hinzufuegen:
```rust
#[derive(Clone, Copy, PartialEq)]
pub enum ToastVariant { Success, Error }

pub fn show_toast(/* existing */) { /* default Error variant fuer bestehende Callsites */ }
pub fn show_success_toast(/* same Sig */) { /* push mit Success-Variant */ }

// In ToastContainer: msg-Tupel wird (u64, ToastVariant, String);
// class wird je Variant gewaehlt:
//   ToastVariant::Success => "bg-green-600 text-white ..."
//   ToastVariant::Error   => "bg-red-600 text-white ..."
```
**Blast-Radius:** alle bestehenden `show_toast`-Callsites in der Codebase muessen ggf. den Variant-Param annehmen ODER bleiben default-Error.

**Test-Pattern** wie in toast.rs aktuell — keine `#[cfg(test)]`-Tests; einfache Add-1-Toast-Logik genuegt.

---

## Shared Patterns

### Component-First-Prinzip (C-18-CF-01)
**Source:** `genossi-frontend/CLAUDE.md` (User-Constraint, HART).
**Apply to:** **Beide neuen Components** (`membership_adjust_modal.rs` + `fiscal_year_date_input.rs`) MUESSEN in `component/` liegen. KEINE inline-RSX-Duplikate in `member_details.rs`. Pages komponieren Components.

### Dioxus Button-Reload-Bug-Pattern (C-18-CF-03 / memory feedback_dioxus_button_type)
**Source:** `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs:124, 131` + `genossi-frontend/src/page/repayment_phases.rs:212-215` Kommentar.
**Apply to:** ALLE 4 Submit-Buttons im `MembershipAdjustModal` + Sub-Choice-Buttons + Back-Button + Cancel-Button.
```rust
button {
    r#type: "button",   // NICHT "submit"
    onclick: move |_| { /* spawn async */ },
}
// NICHT: form { onsubmit: ... }  oder  button { r#type: "submit" }
```

### i18n DE/EN-only (C-18-CF-02 / genossi-frontend/CLAUDE.md HART)
**Source:** `genossi-frontend/src/i18n/mod.rs:9-12` (Locale-Enum hat NUR `En` und `De`).
**Apply to:** `i18n/mod.rs` (Key-Enum), `i18n/de.rs`, `i18n/en.rs`. **KEIN** `cs.rs` (existiert nicht).

### Format-Args-Interpolation via `.replace()` (Landmine L-4)
**Source:** `genossi-frontend/src/component/repayment_letter_download_button.rs:143-165` + `genossi-frontend/src/page/repayment_phase_details.rs:297`.
**Apply to:** Alle Phase-18-Keys mit `{placeholder}`-Format-Args (Vorschau, Helper, Toasts).
```rust
let template = i18n.t(Key::MembershipAdjustCancelPreview);
let rendered = template
    .replace("{name}", &member_name)
    .replace("{shares}", &member.current_shares.to_string())
    .replace("{effective_date}", &i18n.format_date(&effective_date))
    .replace("{half_year}", if is_h1 { "H1" } else { "H2" })
    .replace("{fiscal_year}", &fiscal_year.to_string());
```
**Achtung:** Es gibt **kein** `i18n.t_format!` oder `i18n.t_with_args(...)` Macro/Method, obwohl die CONTEXT.md `<specifics>`-Section das Beispiel-Snippet so schrieb. Pure-Funktion `i18n.t()` returnt `Rc<str>`; Format-Args muessen manuell via `.replace()` substituiert werden.

### use_resource fuer async API-Calls
**Source:** Dioxus-Standard (siehe Component-Bibliothek; konkretes Beispiel in `component/repayment_letter_download_button.rs` und communication_timeline).
**Apply to:** TransferSubView `get_transfer_recipients` (D-18-14).
```rust
let recipients = use_resource(move || async move {
    let config = CONFIG.read().clone();
    api::get_transfer_recipients(&config, from_id).await
});

match recipients.read().as_ref() {
    None => rsx! { p { class: "text-sm text-gray-500", "{i18n.t(Key::MembershipAdjustLoading)}" } },
    Some(Err(e)) => rsx! { ErrorAlert { error: e.clone(), on_dismiss: None } },
    Some(Ok(list)) => {
        let adapted: Vec<MemberTO> = list.iter().map(to_member_to).collect();
        rsx! { MemberSearch { on_select: ..., selected_id: ..., exclude_id: None } }
        // Hinweis: MemberSearch nutzt aktuell den GLOBALEN MEMBERS-Signal (siehe Z. 50-51)
        // und akzeptiert KEINEN custom-members-Prop! Adapter via Liste ist nicht trivial.
        // Siehe Landmine L-5 unten.
    }
}
```

### refresh_members After-Success (D-18-08)
**Source:** `genossi-frontend/src/service/member.rs:11-25` + `component/repayment_entry_paidout_confirm.rs:154-156`.
**Apply to:** `on_success`-Handler im member_details-Mount nach jedem 4er-Op.
```rust
crate::service::member::refresh_members().await;
```

### Authentication via RequirePrivilege (C-18-CF-11)
**Source:** `genossi-frontend/src/auth.rs:34-48` + `genossi-frontend/src/page/repayment_phases.rs:65`.
**Apply to:** Button-Mount in `member_details.rs`.
```rust
RequirePrivilege {
    privilege: "admin",
    fallback: rsx! { /* leerer Block — Button verschwindet einfach */ },
    button { /* ... */ }
}
```
**Privilege-String ist `"admin"`** — NICHT `"manage_members"` (siehe member_details.rs:1357 hat alten Mixed-Pattern fuer Edit, aber Phase 18 hart auf `"admin"` per Phase-15 D-15-01 ADMIN_PRIVILEGE).

---

## Landmines (Critical Risks)

### L-1: Dioxus Button-Reload-Bug
**Memory:** `feedback_dioxus_button_type` (Hotfix e245013).
**Problem:** `r#type: "submit"` oder `<form onsubmit>` mit `preventDefault()` triggert trotzdem Page-Reload in Dioxus 0.6.3.
**Mitigation:** Alle Buttons in Modal nutzen `r#type: "button"` + `onclick: move |_|`-Handler. KEINE `<form>`-Wrapper um Submit-Logik.

### L-2: Frontend rest-types ist separate Crate ohne Phase-15-17-Typen
**Befund:** `genossi-frontend/rest-types/src/lib.rs` (802 LOC) ist **NICHT** der Workspace `genossi_rest_types` (2666 LOC). Es ist eine handgepflegte Teilkopie ohne `utoipa::ToSchema`/Domain-`From`-Impls.
**Problem:** `MemberSlimTO`, `CancelMembershipRequestTO`, `IncreaseSharesRequestTO`, `MembershipAdjustResponseTO`, `PartialRepaymentRequestTO`, `PartialRepaymentResponseTO`, `TransferSharesRequestTO`, `TransferSharesResponseTO` **fehlen** und muessen kopiert werden.
**Mitigation:** Erst rest-types erweitern, dann api.rs anpassen. Reihenfolge im Plan beruecksichtigen. Serde-Default-Format fuer `time::Date` mit dem Backend `iso8601_date_required` verifizieren (sollte stimmen — `MemberActionTO.date` funktioniert seit Phase 4).

### L-3: jj statt git
**Memory:** `feedback_use_jj_not_git`.
**Problem:** Projekt ist jj-Repo (.jj/ + .git/-coexistence). `git commit` umgeht jj's working-copy-state.
**Mitigation:** Alle Phase-18-Commits via `jj commit -m "..."`. `jj log` statt `git log`. `jj git push` fuer Remote.

### L-4: i18n hat KEIN `t_format!`/`t_with_args` Macro
**Befund:** `impl I18n` (i18n/mod.rs:712-788) hat NUR `t()`, `format_date()`, `format_price()`, `format_datetime()`. Kein `t_format!`-Macro existiert.
**Problem:** CONTEXT.md `<specifics>`-Section zeigt Beispiele wie `i18n.t_with_args(Key::FiscalYearDateInputHelper, [min_year, max_year])`, aber das gibt es nicht.
**Mitigation:** Format-Args via `i18n.t(Key).replace("{placeholder}", &value)` substituieren — Pattern aus `repayment_letter_download_button.rs:143-165`. Falls Lesbarkeit darunter leidet, kann der Planner einen Pure-Helper `format_template(template: &str, args: &[(&str, String)]) -> String` in `i18n/mod.rs` hinzufuegen — aber das ist Scope-Erweiterung, vorzugsweise inline-replace nutzen.

### L-5: MemberSearch nutzt GLOBALEN `MEMBERS`-Signal, nicht Custom-Liste
**Befund:** `component/member_search.rs:50-51` liest `MEMBERS.read().items` direkt. Es gibt **keinen** `members`-Prop.
**Problem:** D-18-13 Adapter-Pattern verspricht "MemberSlimTO-Liste in MemberTO konvertieren und an MemberSearch reichen" — aber `MemberSearch` akzeptiert keinen `members`-Param. Es wird global gegen `MEMBERS` gefiltert.
**Mitigation-Optionen** (Planner-Discretion):
- **(a)** MemberSearch um optionalen `members_override: Option<Vec<MemberTO>>`-Prop erweitern (Refactor an Phase-12-Code, kleines Risiko da `MEMBERS` Fallback bleibt).
- **(b)** TransferSubView nutzt **NICHT** MemberSearch sondern bauen einen schmalen Picker selbst (Component-First waere violated — nicht empfohlen).
- **(c)** Adapter laedt MemberSlimTOs **nicht** als separate Liste sondern filtert MEMBERS aus dem globalen Store + zeigt nur Server-erlaubte (exit_date IS NULL) IDs via `exclude_id`-Param (aber `exclude_id` nimmt nur 1 ID). Vermutlich nicht passend.

**Empfehlung Planner:** Option (a) — MemberSearch-Prop um `members_override: Option<Vec<MemberTO>>` erweitern. Funktion `filter_members` ist bereits Pure und nimmt `&[MemberTO]` als Param (Z. 9-13), also nur die Component-Body-Lookup-Zeile (Z. 50-51) braucht ein Fallback `members_override.as_ref().unwrap_or(&MEMBERS.read().items)`. **Phase-12-Tests** bleiben gruen, da Default-Pfad unveraendert.

### L-6: member_details.rs hat keinen ToastContainer-Mount
**Befund:** `page/member_details.rs` hat keine `toast_messages` / `toast_counter` Signals und keinen `ToastContainer`-rsx-Block.
**Problem:** After-Success-Toast (D-18-08) braucht Toast-Infrastruktur auf der Seite.
**Mitigation:** Phase-18-Plan fuegt im Header von MemberDetails (analog `repayment_phases.rs:45-46`):
```rust
let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
let mut toast_counter = use_signal(|| 0u64);
```
…und am Ende der rsx-Baums:
```rust
ToastContainer { messages: toast_messages.into() }
```

### L-7: Format-Date-German existiert nicht als Standalone-Helper
**Befund:** `i18n/mod.rs:724-743` `I18n::format_date(&date)` existiert und macht je nach Locale `YYYY-MM-DD` (En) oder `DD.MM.YYYY` (De). Es gibt keine `format_date_german`-Standalone-Funktion.
**Mitigation:** Phase 18 nutzt **direkt** `i18n.format_date(&date)` — die Locale wird vom Browser/User bestimmt. Vorteil: konsistent mit existing Pattern. Falls die Vorschau jedoch IMMER deutsch sein soll (UI-SPEC sagt "VORSCHAU" auch in EN-Variante geht), Planner-Discretion einen `format_date_german()`-Helper im Modal-File hinzufuegen mit 2 Tests.

### L-8: Modal-Component erlaubt nur 1 children-Element
**Befund:** `component/modal.rs:3-7` `ModalProps { pub children: Element }`. Nur ein Element-Param.
**Mitigation:** `MembershipAdjustModal` muss seinen kompletten Inhalt in einer rsx-Wurzel kapseln (z.B. ein `div { class: "flex flex-col gap-4" }`-Wrapper). Pattern aus `repayment_entry_paidout_confirm.rs:60-165` (Z. 60: `rsx! { div { class: "flex flex-col gap-4", ... } }`).

---

## No Analog Found

| Datei | Role | Reason |
|-------|------|--------|
| (keine) | — | Alle 8 Phase-18-Dateien haben starke Analoge im Codebase. Pure-Helper-Funktionen (`compute_effective_date_mirror`, `is_voll_uebertrag`) sind **Backend-Mirror** und nicht Frontend-novel. |

---

## Metadata

**Analog search scope:** `genossi-frontend/src/component/`, `genossi-frontend/src/page/`, `genossi-frontend/src/i18n/`, `genossi-frontend/src/service/`, `genossi-frontend/src/auth.rs`, `genossi-frontend/src/api.rs`, `genossi-frontend/rest-types/`, `genossi_rest_types/src/lib.rs`, `genossi_rest/src/membership_adjust.rs`, `genossi_rest/src/member.rs`, `genossi_service_impl/src/membership_adjust.rs`.

**Files scanned:** ~15.

**Pattern extraction date:** 2026-06-07.

---

## Pattern Mapping Complete — Summary

**Phase:** 18 - frontend-component-first
**Files classified:** 10 (2 new + 8 modified incl. L-2 rest-types + L-6 toast.rs additions)
**Analogs found:** 10 / 10 (exact match fuer 7, role-match fuer 3)

### Coverage
- Files with exact analog: 7
- Files with role-match analog: 3
- Files with no analog: 0

### Key Patterns Identified
- **Modal-mit-Submit-Loop**: `repayment_entry_paidout_confirm.rs` ist 1:1-Vorbild fuer `membership_adjust_modal.rs` — Props (on_close + on_complete + on_error), `use_signal(submitting)`, `spawn(async move)`-Submit, refresh_members am Ende.
- **API-Client-Funktionen**: `format!("{}/api/...", config.backend) + reqwest::Client::new().post(url).json(&body).send().await?` als Standard-Pattern fuer alle 5 neuen api.rs-Funktionen.
- **Modal-Mount auf Page**: `repayment_phases.rs:64-130` ist 1:1-Vorbild fuer `member_details.rs`-Erweiterung — RequirePrivilege + show_modal-Signal + conditional Modal-Mount.
- **i18n Format-Args via .replace()**: Kein t_format!-Macro existiert; `i18n.t(Key).replace("{placeholder}", &val)` ist das etablierte Pattern (`repayment_letter_download_button.rs:143`).
- **Component-First HART**: Beide neuen Components in `component/`, keine inline-RSX-Duplikate. genossi-frontend/CLAUDE.md ist unmissverstaendlich.
- **i18n DE/EN-only HART**: Locale-Enum hat nur `En`+`De`; cs.rs existiert nicht.

### File Created
`.planning/phases/18-frontend-component-first/18-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner kann jetzt konkrete Tasks formulieren ohne weitere Code-Lookups — alle Analog-Snippets, Pure-Helper-Vorlagen, Landmines und Submit-Patterns sind hier dokumentiert.
