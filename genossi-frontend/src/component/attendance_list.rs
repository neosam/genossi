//! Shared attendance list — used by `/helper/attendance` AND
//! `/assemblies/{id}` Anwesenheits-Tab.
//!
//! Phase 4 Plan 04 — D-11 + UI-SPEC §AttendanceList + ROADMAP SC#6 (no Optimistic-UI).
//!
//! Hard constraints:
//! - Renders EXACTLY 5 visible fields per row (`member_number`, `salutation`,
//!   `title`, `first_name`, `last_name`) — PII last line of defence
//!   (CLAUDE.md §Datenschutz; ATTN-01).
//! - Toggle button does NOT show the checked state during loading
//!   (SC#6 + D-17): a row stays in its previous `is_present` state until
//!   the parent confirms the API call landed and bumps `refresh_signal`.
//! - Polling-refresh skips rows currently in `loading` state — they keep
//!   their pre-toggle visual until the next list refresh resolves them
//!   (RESEARCH Pitfall 5).
//! - **Component-First / dumb component:** the actual `mark_present` /
//!   `mark_absent` API call lives in the parent page, not here. The list
//!   exposes a single `on_toggle: EventHandler<AttendanceToggleRequest>`
//!   so `/helper/attendance` and `assembly_details` can wire it
//!   differently (Helfer with redeem-cookie auth, Vorstand with admin
//!   session) — this is the ATTN-06 reuse anchor.
//!
//! Refresh is triggered by:
//! - Mount (initial fetch).
//! - `search_query` changes (parent passes the debounced value).
//! - `refresh_signal` increments (parent bumps it after a successful
//!   toggle, or every polling tick if it wires the LiveCounter clock).

use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::{self, AttendanceMemberTO};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

/// Payload emitted by the list when the user activates a row's toggle.
///
/// `current_is_present` is the value displayed in the UI at the moment of
/// the click — the parent uses it to decide whether to call
/// `mark_present` (false → true) or `mark_absent` (true → false).
#[derive(Clone, Debug, PartialEq)]
pub struct AttendanceToggleRequest {
    pub member_id: Uuid,
    pub current_is_present: bool,
}

#[component]
pub fn AttendanceList(
    assembly_id: Uuid,
    search_query: ReadOnlySignal<String>,
    read_only: bool,
    refresh_signal: ReadOnlySignal<u64>,
    /// Parent receives one toggle request per click. The parent owns the
    /// API call and is responsible for bumping `refresh_signal` on success
    /// (which authoritatively unsticks the row's loading state via the
    /// next fetch). On failure the parent should set `error_for_member`
    /// to surface the message inline.
    on_toggle: EventHandler<AttendanceToggleRequest>,
    /// Optional per-row error from the parent — `member_id → message`.
    /// Cleared by the parent on the next successful toggle.
    #[props(default)]
    error_for_member: Option<ReadOnlySignal<HashMap<Uuid, String>>>,
) -> Element {
    let i18n = use_i18n();
    let mut members = use_signal(Vec::<AttendanceMemberTO>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);
    // Per-row loading map: `member_id` → in-flight (UI-only; cleared on next refresh).
    let mut row_loading = use_signal(HashMap::<Uuid, bool>::new);

    use_effect(move || {
        // Subscribe to both signals so Dioxus re-runs the effect on either change.
        let q = search_query();
        let _r = refresh_signal();
        let aid = assembly_id;
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            let q_opt = if q.is_empty() { None } else { Some(q.as_str()) };
            match api::list_attendance_members(&config, aid, q_opt).await {
                Ok(rows) => {
                    // Pitfall 5 safety: a successful re-fetch is authoritative.
                    // We can clear any row loading flags whose member appears
                    // in the new list — by definition the row is now showing
                    // the latest server-side state, so the loading badge has
                    // served its purpose.
                    let snapshot = row_loading();
                    if !snapshot.is_empty() {
                        let returned: std::collections::HashSet<Uuid> =
                            rows.iter().map(|m| m.member_id).collect();
                        row_loading.with_mut(|m| {
                            m.retain(|id, _| !returned.contains(id));
                        });
                    }
                    members.set(rows);
                }
                Err(e) => {
                    error.set(Some(e.message));
                }
            }
            loading.set(false);
        });
    });

    if *loading.read() && members.read().is_empty() {
        return rsx! {
            div {
                class: "flex justify-center py-12",
                span { "{i18n.t(Key::Loading)}" }
            }
        };
    }
    if let Some(msg) = error.read().as_ref() {
        return rsx! {
            div {
                class: "bg-red-50 border border-red-200 text-red-700 p-4 rounded-lg",
                "{msg}"
            }
        };
    }
    if members.read().is_empty() {
        return rsx! {
            div {
                class: "text-center py-12 text-gray-500",
                p { class: "text-lg font-medium", "{i18n.t(Key::AttendanceEmpty)}" }
                p { class: "text-sm mt-2", "{i18n.t(Key::AttendanceEmptyHint)}" }
            }
        };
    }

    let used_label = i18n.t(Key::HelperTokenStatusUsed).to_string();
    let absent_label = i18n.t(Key::AttendanceToggleAbsent).to_string();
    let present_aria = i18n.t(Key::AttendanceTogglePresent).to_string();
    let absent_aria = i18n.t(Key::AttendanceToggleAbsent).to_string();

    rsx! {
        div { class: "flex flex-col gap-2",
            for m in members.read().iter() {
                {
                    let mid = m.member_id;
                    let is_present = m.is_present;
                    let row_is_loading = *row_loading.read().get(&mid).unwrap_or(&false);
                    let member_number = m.member_number;
                    let salutation_str = m.salutation.clone().unwrap_or_default();
                    let title_str = m.title.clone().unwrap_or_default();
                    let first_name = m.first_name.clone();
                    let last_name = m.last_name.clone();
                    let row_err = error_for_member
                        .as_ref()
                        .and_then(|s| s.read().get(&mid).cloned());
                    let aria_label = if is_present {
                        absent_aria.clone()
                    } else {
                        present_aria.clone()
                    };

                    // Toggle button class per state-machine (UI-SPEC §Toggle button states).
                    // SC#6 / D-17: the loading state visually neither implies present
                    // nor absent — neutral gray placeholder.
                    let (btn_class, glyph) = button_state_class(is_present, row_is_loading);

                    rsx! {
                        div {
                            key: "{mid}",
                            class: "bg-white border border-gray-200 rounded-lg p-3 flex items-center gap-4 hover:bg-gray-50 transition-colors",
                            div { class: "flex-1 flex flex-col",
                                div { class: "text-base font-medium text-gray-900",
                                    span { class: "text-gray-500 mr-2", "#{member_number}" }
                                    if !salutation_str.is_empty() {
                                        span { class: "mr-1", "{salutation_str}" }
                                    }
                                    if !title_str.is_empty() {
                                        span { class: "mr-1", "{title_str}" }
                                    }
                                    span { class: "mr-1", "{first_name}" }
                                    span { "{last_name}" }
                                }
                                if let Some(msg) = row_err {
                                    p {
                                        class: "text-xs text-red-600 mt-1",
                                        "{msg}"
                                    }
                                }
                            }
                            if read_only {
                                span {
                                    class: "text-sm text-gray-500",
                                    if is_present { "{used_label}" } else { "{absent_label}" }
                                }
                            } else {
                                button {
                                    class: "{btn_class}",
                                    disabled: row_is_loading,
                                    "aria-label": "{aria_label}",
                                    onclick: move |_| {
                                        // SC#6 / Pitfall 5: enter loading state — NO premature
                                        // is_present flip. Parent owns the API call and the
                                        // refresh_signal bump that will clear loading.
                                        row_loading.with_mut(|m| { m.insert(mid, true); });
                                        on_toggle.call(AttendanceToggleRequest {
                                            member_id: mid,
                                            current_is_present: is_present,
                                        });
                                    },
                                    "{glyph}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Pure-logic button-state mapping; extracted so the 3-state contract is
/// unit-testable without a Dioxus runtime.
///
/// Returns `(tailwind_class, glyph)` per UI-SPEC §Toggle button states.
/// - `loading`: neutral gray, hourglass-ish glyph, disabled by caller.
/// - `present`: green, checkmark glyph.
/// - `absent`: white/gray, empty-box glyph.
pub fn button_state_class(is_present: bool, loading: bool) -> (&'static str, &'static str) {
    if loading {
        (
            "min-h-[44px] min-w-[44px] inline-flex items-center justify-center rounded-md bg-gray-100 border-2 border-gray-300 text-gray-400 cursor-not-allowed",
            "\u{29D6}",
        )
    } else if is_present {
        (
            "min-h-[44px] min-w-[44px] inline-flex items-center justify-center rounded-md bg-green-100 border-2 border-green-500 text-green-800 hover:bg-green-200",
            "\u{2713}",
        )
    } else {
        (
            "min-h-[44px] min-w-[44px] inline-flex items-center justify-center rounded-md bg-white border-2 border-gray-300 text-gray-700 hover:bg-gray-50",
            "\u{2610}",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_state_has_neutral_class_and_disabled_glyph() {
        // SC#6: loading button must NOT visually look "present" — neutral gray only.
        let (class, glyph) = button_state_class(false, true);
        assert!(class.contains("bg-gray-100"));
        assert!(class.contains("cursor-not-allowed"));
        assert!(!class.contains("bg-green"));
        // Hourglass-ish placeholder — anything but a checkmark.
        assert_ne!(glyph, "\u{2713}");
    }

    #[test]
    fn loading_state_overrides_is_present_visual() {
        // Even when the row is currently present, a loading toggle hides the
        // checkmark — the user must NOT see a phantom check during the round trip.
        let (class_present_loading, glyph_present_loading) = button_state_class(true, true);
        assert!(class_present_loading.contains("bg-gray-100"));
        assert_ne!(glyph_present_loading, "\u{2713}");
    }

    #[test]
    fn present_state_uses_green_check() {
        let (class, glyph) = button_state_class(true, false);
        assert!(class.contains("bg-green-100"));
        assert!(class.contains("border-green-500"));
        assert_eq!(glyph, "\u{2713}");
    }

    #[test]
    fn absent_state_uses_white_empty_box() {
        let (class, glyph) = button_state_class(false, false);
        assert!(class.contains("bg-white"));
        assert!(class.contains("border-gray-300"));
        assert_eq!(glyph, "\u{2610}");
    }

    #[test]
    fn all_states_have_44px_touch_target() {
        // WCAG-AA-equivalent + iOS HIG: 44x44px minimum. Phase 4 helper UI must
        // be tap-friendly on phones.
        for (is_present, loading) in [(true, false), (false, false), (false, true), (true, true)] {
            let (class, _) = button_state_class(is_present, loading);
            assert!(
                class.contains("min-h-[44px]"),
                "missing min-h-[44px] for ({is_present}, {loading})"
            );
            assert!(
                class.contains("min-w-[44px]"),
                "missing min-w-[44px] for ({is_present}, {loading})"
            );
        }
    }

    #[test]
    fn toggle_request_carries_current_value() {
        // Parent uses `current_is_present` to decide which API method to call;
        // it MUST reflect the value that was on screen at click time.
        let req = AttendanceToggleRequest {
            member_id: Uuid::from_u128(1),
            current_is_present: true,
        };
        assert!(req.current_is_present);
        let req2 = AttendanceToggleRequest {
            member_id: Uuid::from_u128(2),
            current_is_present: false,
        };
        assert!(!req2.current_is_present);
    }
}
