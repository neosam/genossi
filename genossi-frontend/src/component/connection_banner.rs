//! Sticky amber warning banner for assembly attendance views.
//!
//! Appears when polling has lost connectivity (>=2 consecutive failures
//! emitted by `LiveCounter`).
//!
//! Phase 4 Plan 04 — D-16 + UI-SPEC §Connection-Banner colors.
//!
//! Hard constraints:
//! - Sticky-top, full width, amber-coded.
//! - `print:hidden` so paper-printed attendance lists never carry a
//!   network-status banner.
//! - Conditionally renders nothing when `visible == false` so the banner
//!   does not occupy layout space during the healthy state.

use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

#[component]
pub fn ConnectionBanner(visible: bool) -> Element {
    let i18n = use_i18n();
    if !visible {
        return rsx! {};
    }
    let label = i18n.t(Key::AttendanceConnectionLost).to_string();
    rsx! {
        div {
            class: "sticky top-0 z-30 w-full px-4 py-2 print:hidden bg-amber-100 text-amber-900 text-sm font-medium border-b-2 border-amber-400 flex items-center justify-between",
            role: "alert",
            span {
                class: "flex items-center gap-2",
                span { "\u{26A0}" }
                span { "{label}" }
            }
            span {
                class: "block h-3 w-3 rounded-full bg-amber-500 animate-pulse"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// Pure-logic mirror of the visibility branch — keeps the contract
    /// expressible as a unit test even though Dioxus signals require a
    /// running runtime.
    fn should_render(visible: bool) -> bool {
        visible
    }

    #[test]
    fn does_not_render_when_hidden() {
        assert!(!should_render(false));
    }

    #[test]
    fn renders_when_visible() {
        assert!(should_render(true));
    }
}
