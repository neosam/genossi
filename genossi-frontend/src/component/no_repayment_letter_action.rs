//! Quick 260603-evf — NoRepaymentLetterAction: one-click recovery for bulk-mail
//! recipients that failed with `error="no_repayment_letter"`. Resolves the
//! recipient's `member_id` against the phase's `RepaymentEntry`-list, generates
//! the missing letter (POST `/api/repayment-phase/{phase_id}/letters/generate`),
//! revokes the bundle blob URL (we only want the server-side `MemberDocument`
//! persist side-effect — no download triggered), then calls `retry_mail_job`.
//!
//! Component-First — used by `mail_page.rs::MailPage` Expanded-Row AND
//! `mail_page.rs::MailJobDetail`. Single source of truth for the recovery flow.
use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, RepaymentEntryTO};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

/// Quick 260603-evf: scan the phase's entries and pick the first matching
/// the given `member_id`. Deterministic: the backend resolver dedupes per
/// member, but if duplicates ever leak we still produce a stable choice.
pub fn find_entry_for_member(
    entries: &[RepaymentEntryTO],
    member_id: Uuid,
) -> Option<RepaymentEntryTO> {
    entries.iter().find(|e| e.member_id == member_id).cloned()
}

/// Quick 260603-evf: 3-state machine for the action button.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonState {
    Idle,
    Loading,
    Done,
}

/// Quick 260603-evf: pure helper — maps the button state to its i18n key.
/// Kept pure (returns the Key, not the rendered string) so unit-tests run
/// without spinning up the I18n GlobalSignal.
pub fn button_label_for_state(state: ButtonState) -> Key {
    match state {
        ButtonState::Idle => Key::MailGenerateLetterAndRetry,
        ButtonState::Loading => Key::MailGenerateLetterAndRetryRunning,
        ButtonState::Done => Key::MailGenerateLetterAndRetrySuccess,
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct NoRepaymentLetterActionProps {
    /// MailJob id — passed to `retry_mail_job` after the letter is generated.
    pub job_id: String,
    /// Recipient id — currently only used for debug/logging context. Kept in
    /// the API so future callers can correlate UI clicks with backend retries.
    pub recipient_id: String,
    /// Member id resolved from `MailRecipient.member_id` — looked up against
    /// the phase entries to determine the `entry_id` to pass to
    /// `generate_repayment_letters`.
    pub member_id: Uuid,
    /// Phase id resolved from `MailJob.repayment_phase_id` — addresses the
    /// `/api/repayment-phase/{phase_id}/letters/generate` endpoint.
    pub phase_id: Uuid,
    /// Fired on full-success (letter generated + retry triggered). Parent
    /// reloads jobs and emits a success toast.
    pub on_done: EventHandler<()>,
    /// Fired on any failure (list-entries fail, no matching entry, generate
    /// fail, retry fail). Parent emits an error toast with this message.
    pub on_error: EventHandler<String>,
}

#[component]
pub fn NoRepaymentLetterAction(props: NoRepaymentLetterActionProps) -> Element {
    let i18n = use_i18n();
    let mut state = use_signal(|| ButtonState::Idle);

    let on_done = props.on_done;
    let on_error = props.on_error;
    let job_id = props.job_id.clone();
    let phase_id = props.phase_id;
    let member_id = props.member_id;

    let onclick = move |_| {
        let job_id = job_id.clone();
        let on_done = on_done;
        let on_error = on_error;
        // Pre-resolve the no-entry i18n string before crossing the spawn
        // boundary — `I18n` is not Send so we cannot read it inside the
        // async block on the wasm runtime.
        let no_entry_msg = use_i18n().t(Key::MailGenerateLetterAndRetryNoEntry).to_string();
        state.set(ButtonState::Loading);
        spawn(async move {
            let config = CONFIG.read().clone();
            let entries = match api::list_repayment_entries(&config, phase_id).await {
                Ok(e) => e,
                Err(err) => {
                    state.set(ButtonState::Idle);
                    on_error.call(err.message.clone());
                    return;
                }
            };
            let entry = match find_entry_for_member(&entries, member_id) {
                Some(e) => e,
                None => {
                    state.set(ButtonState::Idle);
                    on_error.call(no_entry_msg);
                    return;
                }
            };
            let gen = match api::generate_repayment_letters(&config, phase_id, vec![entry.id]).await
            {
                Ok(r) => r,
                Err(err) => {
                    state.set(ButtonState::Idle);
                    on_error.call(err.message.clone());
                    return;
                }
            };
            // Memory-safe blob handling: we ignore the bundle PDF (only the
            // server-side MemberDocument-persist side-effect matters).
            let _ = web_sys::Url::revoke_object_url(&gen.blob_url);
            match api::retry_mail_job(&config, &job_id).await {
                Ok(_) => {
                    state.set(ButtonState::Done);
                    on_done.call(());
                }
                Err(err) => {
                    state.set(ButtonState::Idle);
                    on_error.call(err.message.clone());
                }
            }
        });
    };

    let current_state = *state.read();
    let is_disabled = current_state != ButtonState::Idle;
    let label_key = button_label_for_state(current_state);
    let label = i18n.t(label_key);
    let class = if is_disabled {
        "bg-amber-300 text-white px-2 py-1 rounded text-xs font-medium cursor-not-allowed"
    } else {
        "bg-amber-500 hover:bg-amber-600 text-white px-2 py-1 rounded text-xs font-medium"
    };
    rsx! {
        button {
            // Memory: `feedback_dioxus_button_type` — explicit r#type="button"
            // prevents the Dioxus form-onsubmit/page-reload bug (Hotfix
            // e245013-Pattern). Without this the click would re-submit the
            // enclosing implicit form and reload the page.
            r#type: "button",
            class: "{class}",
            disabled: is_disabled,
            onclick: onclick,
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::RepaymentEntryStatusTO;

    fn entry(member_id: Uuid) -> RepaymentEntryTO {
        RepaymentEntryTO {
            id: Uuid::new_v4(),
            member_id,
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 1,
            status: RepaymentEntryStatusTO::Open,
            created: None,
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn find_entry_returns_matching_member() {
        let target = Uuid::new_v4();
        let other = Uuid::new_v4();
        let entries = vec![entry(other), entry(target)];
        let found = find_entry_for_member(&entries, target).expect("must find target");
        assert_eq!(found.member_id, target);
    }

    #[test]
    fn find_entry_returns_none_for_empty() {
        let entries: Vec<RepaymentEntryTO> = vec![];
        assert!(find_entry_for_member(&entries, Uuid::new_v4()).is_none());
    }

    #[test]
    fn find_entry_returns_first_match_when_duplicates() {
        let target = Uuid::new_v4();
        let first = entry(target);
        let second = entry(target);
        let first_id = first.id;
        let entries = vec![first, second];
        let found = find_entry_for_member(&entries, target).expect("must find first");
        assert_eq!(
            found.id, first_id,
            "must return the first matching entry deterministically",
        );
    }

    #[test]
    fn button_label_for_state_maps_each_state() {
        assert_eq!(
            button_label_for_state(ButtonState::Idle),
            Key::MailGenerateLetterAndRetry,
        );
        assert_eq!(
            button_label_for_state(ButtonState::Loading),
            Key::MailGenerateLetterAndRetryRunning,
        );
        assert_eq!(
            button_label_for_state(ButtonState::Done),
            Key::MailGenerateLetterAndRetrySuccess,
        );
    }
}
