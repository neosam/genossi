//! Add-Entry-Modal (Phase 12 Plan 12-09, UI-04).
//!
//! Member-Picker via MemberSearch (D-21, direct-reuse).
//! share_count_to_pay_out wird beim Select mit current_shares vorbefuellt (D-22).
//!
//! Form-in-Modal-Pattern (analog assemblies.rs::CreateAssemblyForm Z. 96-184):
//! `form { onsubmit: e.prevent_default() ZUERST + spawn(async ...) DANACH }`.
//! D-01-Pattern: alle button-Tags mit `r#type:` explizit.
//!
//! Client-Validation D-23 minimal: Submit-Button disabled bei
//!   member_id == None ODER share_count <= 0.
//! Backend-Validation (Service + DB-CHECK) bleibt Backstop —
//! Frontend zeigt Toast bei Backend-Error.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, CreateRepaymentEntryRequest};
use crate::component::member_search::MemberSearch;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::MEMBERS;

/// D-23 minimal client-side validation. Returns true wenn Submit erlaubt.
pub fn validate_create_entry(member_id: Option<Uuid>, share_count: i32) -> bool {
    member_id.is_some() && share_count > 0
}

#[component]
pub fn RepaymentEntryAddModal(
    phase_id: Uuid,
    on_close: EventHandler<()>,
    on_created: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut selected_member_id = use_signal(|| Option::<Uuid>::None);
    let mut share_count = use_signal(|| 0_i32);
    let mut submitting = use_signal(|| false);

    let invalid_msg = "Bitte Mitglied auswaehlen und Anteile > 0 angeben".to_string();

    rsx! {
        form {
            class: "flex flex-col gap-4",
            onsubmit: move |e| {
                e.prevent_default();
                let mid = *selected_member_id.read();
                let sc = *share_count.read();
                if !validate_create_entry(mid, sc) {
                    on_error.call(invalid_msg.clone());
                    return;
                }
                let member_id = mid.unwrap();
                submitting.set(true);
                let req = CreateRepaymentEntryRequest {
                    phase_id,
                    member_id,
                    share_count_to_pay_out: sc,
                };
                spawn(async move {
                    let config = CONFIG.read().clone();
                    match api::create_repayment_entry(&config, &req).await {
                        Ok(_) => on_created.call(()),
                        Err(e) => on_error.call(e.message),
                    }
                    submitting.set(false);
                });
            },
            h2 { class: "text-xl font-semibold", "{i18n.t(Key::RepaymentEntryAdd)}" }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "Mitglied" }
                MemberSearch {
                    on_select: move |id: Option<Uuid>| {
                        selected_member_id.set(id);
                        // D-22: bei Member-Select mit current_shares vorbefuellen
                        if let Some(uid) = id {
                            let members = MEMBERS.read();
                            if let Some(m) = members.items.iter().find(|m| m.id == Some(uid)) {
                                share_count.set(m.current_shares);
                            }
                        }
                    },
                    selected_id: *selected_member_id.read(),
                    exclude_id: None,
                }
            }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::RepaymentEntryColShares)}" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "number",
                    min: "1",
                    value: "{share_count}",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            share_count.set(n);
                        }
                    },
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
                    disabled: *submitting.read() || !validate_create_entry(*selected_member_id.read(), *share_count.read()),
                    "{i18n.t(Key::Save)}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_member() {
        assert!(!validate_create_entry(None, 5));
    }

    #[test]
    fn validate_requires_positive_count() {
        let mid = Some(Uuid::new_v4());
        assert!(!validate_create_entry(mid, 0));
        assert!(!validate_create_entry(mid, -1));
    }

    #[test]
    fn validate_accepts_valid() {
        let mid = Some(Uuid::new_v4());
        assert!(validate_create_entry(mid, 1));
        assert!(validate_create_entry(mid, 100));
    }
}
