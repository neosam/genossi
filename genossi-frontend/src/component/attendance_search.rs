//! Debounced substring search input for AttendanceList.
//!
//! Phase 4 Plan 04 — Component-First; reused by `/helper/attendance` and
//! `/assemblies/{id}` Anwesenheits-Tab.
//!
//! Hard constraints:
//! - 500ms debounce (UI-SPEC §AttendanceSearch).
//! - Pulse-dot indicator while debounce window is open.
//! - i18n via `Key::AttendanceSearch` placeholder — never hard-coded user-facing strings.
//! - No outbound side effects: parent owns the actual fetch via `on_change`.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::i18n::{use_i18n, Key};

const DEBOUNCE_MS: u32 = 500;

#[component]
pub fn AttendanceSearch(on_change: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let mut query = use_signal(String::new);
    // Generation counter — incremented on every keystroke so older debounce
    // tasks can detect they are stale and skip firing the callback.
    let mut debounce_gen = use_signal(|| 0u64);
    let mut pending = use_signal(|| false);

    let placeholder = i18n.t(Key::AttendanceSearch).to_string();

    rsx! {
        div { class: "relative w-full mb-4",
            span {
                class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400",
                "\u{1F50D}"
            }
            input {
                class: "w-full pl-10 pr-10 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                r#type: "text",
                placeholder: "{placeholder}",
                value: "{query}",
                oninput: move |e| {
                    let value = e.value();
                    query.set(value.clone());
                    pending.set(true);
                    let gen = {
                        let mut g = debounce_gen.write();
                        *g += 1;
                        *g
                    };
                    let on_change_evt = on_change;
                    spawn(async move {
                        TimeoutFuture::new(DEBOUNCE_MS).await;
                        // Cancel-stale: only fire if generation hasn't moved forward.
                        if *debounce_gen.read() == gen {
                            pending.set(false);
                            on_change_evt.call(value.clone());
                        }
                    });
                },
            }
            if pending() {
                span {
                    class: "absolute right-3 top-1/2 -translate-y-1/2",
                    span {
                        class: "block h-2 w-2 rounded-full bg-blue-400 animate-pulse"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_window_is_500ms() {
        // ROADMAP Phase 4 SC#3 / UI-SPEC §AttendanceSearch — keep search debouncing
        // exactly at 500ms; tweaking this is a UX-relevant decision and must not
        // happen silently.
        assert_eq!(DEBOUNCE_MS, 500);
    }

    #[test]
    fn debounce_constant_is_within_human_perception_bounds() {
        // Sanity bounds: should be high enough to coalesce typing yet not feel laggy.
        assert!(
            DEBOUNCE_MS >= 200,
            "debounce too aggressive — feels twitchy"
        );
        assert!(DEBOUNCE_MS <= 1000, "debounce too long — feels laggy");
    }
}
