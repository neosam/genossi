//! Live attendance counter for `/helper/attendance` and `/assemblies/{id}` Anwesenheits-Tab.
//!
//! Phase 4 Plan 04 — D-11 / D-14 / D-16 + UI-SPEC §LiveCounter + RESEARCH Pattern 1.
//!
//! Polls `GET /api/assembly/{id}/stats` every 5s; emits `ConnState`
//! transitions (`Healthy` / `Warning` / `Lost`) so the parent
//! (`HelperShell` or `assembly_details`) can render the
//! [`crate::component::connection_banner::ConnectionBanner`].
//!
//! Hard constraints:
//! - Display reads literally `"X von Y anwesend"` (or `"X present of Y"` for `en`)
//!   — ROADMAP Phase 4 SC#3.
//! - `Y` always shown when known (Member-Universe-Snapshot is stable for the whole GV).
//! - `X` dashed during ≥2-failure state (RESEARCH §Polling-Pattern recovery).
//! - Polling stops on unmount — Dioxus drops `use_future` automatically when the
//!   component is dropped, which terminates the awaiting `TimeoutFuture`.
//!
//! Wave 1 04-03 i18n deviation (intentional): `Key::AttendanceCounterLong`
//! holds only the word `"anwesend"` / `"present"`; the full sentence is composed
//! inline because German and English use different word orders
//! (`X von Y anwesend` vs `X of Y present`).

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use uuid::Uuid;

use crate::api::{self, AttendanceStatsTO};
use crate::i18n::{use_i18n, I18n, Key, Locale};
use crate::service::config::CONFIG;

const POLL_INTERVAL_MS: u32 = 5_000;
const FAILURES_BEFORE_LOST: u32 = 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConnState {
    /// Last poll succeeded.
    Healthy,
    /// One consecutive failure — silent (no banner, last successful counter still displayed).
    Warning,
    /// Two or more consecutive failures — banner shown, counter X dashed.
    Lost,
}

/// Pure display-logic mirror used both by the component and by the unit tests.
///
/// Rules:
/// - No stats yet → `AttendanceCounterUnknown` ("Anwesenheit lädt…").
/// - Stats present and `consecutive_failures < FAILURES_BEFORE_LOST` → "X von Y anwesend".
/// - Stats present and `consecutive_failures >= FAILURES_BEFORE_LOST` → "— von Y anwesend".
pub fn render_counter_text(
    stats: Option<&AttendanceStatsTO>,
    consecutive_failures: u32,
    i18n: &I18n,
) -> String {
    match (stats, consecutive_failures) {
        (None, _) => i18n.t(Key::AttendanceCounterUnknown).to_string(),
        (Some(s), n) if n < FAILURES_BEFORE_LOST => format_counter(s.present, s.total, i18n),
        (Some(s), _) => format_counter_dashed(s.total, i18n),
    }
}

/// Format the healthy "X von Y anwesend" / "X of Y present" string.
fn format_counter(present: u64, total: u64, i18n: &I18n) -> String {
    let word = i18n.t(Key::AttendanceCounterLong);
    match locale_of(i18n) {
        Locale::De => format!("{present} von {total} {word}"),
        Locale::En => format!("{present} of {total} {word}"),
    }
}

/// Format the degraded "— von Y anwesend" / "— of Y present" string.
fn format_counter_dashed(total: u64, i18n: &I18n) -> String {
    let word = i18n.t(Key::AttendanceCounterLongLoading);
    match locale_of(i18n) {
        Locale::De => format!("\u{2014} von {total} {word}"),
        Locale::En => format!("\u{2014} of {total} {word}"),
    }
}

/// Best-effort locale read — falls back to `De` if the i18n probe disagrees.
/// Reads the i18n locale via the textual fingerprint of `AttendanceCounterUnknown`
/// (`"Anwesenheit lädt…"` is German-only); no public locale getter exists on `I18n`.
fn locale_of(i18n: &I18n) -> Locale {
    let unknown = i18n.t(Key::AttendanceCounterUnknown);
    if unknown.starts_with("Anwesenheit") {
        Locale::De
    } else {
        Locale::En
    }
}

#[component]
pub fn LiveCounter(
    assembly_id: Uuid,
    polling_enabled: bool,
    on_connection_state: EventHandler<ConnState>,
) -> Element {
    let i18n = use_i18n();
    let mut stats = use_signal(|| Option::<AttendanceStatsTO>::None);
    let mut consecutive_failures = use_signal(|| 0u32);

    // Polling loop — auto-cancelled on unmount when the use_future task is dropped.
    //
    // Closed-GV fix (2026-05-06): the initial fetch ALWAYS runs, even when
    // `polling_enabled=false`, so a closed GV displays the final attendance
    // snapshot ("X von Y anwesend") instead of staying on "Anwesenheit lädt…".
    // After the first fetch, the gate is enforced: closed GVs idle without
    // re-fetching; open GVs continue the 5s polling cadence.
    use_future(move || async move {
        let mut has_initial_load = false;
        loop {
            if !polling_enabled && has_initial_load {
                // Idle: re-check the gate once per second; cheap and avoids dropping
                // the polling loop when a parent toggles the prop transiently.
                TimeoutFuture::new(1_000).await;
                continue;
            }
            let config = CONFIG.read().clone();
            match api::get_assembly_stats(&config, assembly_id).await {
                Ok(s) => {
                    let was_lost = *consecutive_failures.read() >= FAILURES_BEFORE_LOST;
                    stats.set(Some(s));
                    consecutive_failures.set(0);
                    if was_lost {
                        on_connection_state.call(ConnState::Healthy);
                    }
                }
                Err(_) => {
                    let n = {
                        let mut g = consecutive_failures.write();
                        *g += 1;
                        *g
                    };
                    if n == 1 {
                        on_connection_state.call(ConnState::Warning);
                    } else if n == FAILURES_BEFORE_LOST {
                        // Only emit `Lost` exactly on the *transition* to N failures —
                        // avoids spamming the banner re-mount on every subsequent failure.
                        on_connection_state.call(ConnState::Lost);
                    }
                }
            }
            has_initial_load = true;
            TimeoutFuture::new(POLL_INTERVAL_MS).await;
        }
    });

    let stats_read = stats.read();
    let display = render_counter_text(stats_read.as_ref(), *consecutive_failures.read(), &i18n);
    let label = i18n.t(Key::AttendanceCounterLabel).to_string();

    rsx! {
        div {
            class: "bg-white border border-gray-200 rounded-lg p-6 mb-4 flex items-baseline justify-between",
            span {
                class: "text-sm font-medium text-gray-500 uppercase tracking-wider",
                "{label}"
            }
            span {
                class: "text-4xl font-bold text-gray-900",
                "{display}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn de() -> I18n {
        I18n::new(Locale::De)
    }

    fn en() -> I18n {
        I18n::new(Locale::En)
    }

    #[test]
    fn poll_interval_is_5_seconds() {
        // ROADMAP/UI-SPEC: ~5s polling. Anything else is a UX-relevant change.
        assert_eq!(POLL_INTERVAL_MS, 5_000);
    }

    #[test]
    fn lost_threshold_is_two_consecutive_failures() {
        // D-16: banner appears at ≥2 failures, not 1 (kein Alarm bei einem Wackler).
        assert_eq!(FAILURES_BEFORE_LOST, 2);
    }

    #[test]
    fn render_when_no_stats_yet_uses_loading_text_de() {
        let text = render_counter_text(None, 0, &de());
        assert_eq!(text, "Anwesenheit lädt…");
    }

    #[test]
    fn render_when_no_stats_yet_uses_loading_text_en() {
        let text = render_counter_text(None, 0, &en());
        assert_eq!(text, "Attendance loading…");
    }

    #[test]
    fn render_healthy_de_uses_literal_von_anwesend() {
        // ROADMAP SC#3 hard-constraint: literal " von " between X and Y, NOT "X/Y".
        let stats = AttendanceStatsTO {
            present: 12,
            total: 47,
        };
        let text = render_counter_text(Some(&stats), 0, &de());
        assert_eq!(text, "12 von 47 anwesend");
        assert!(
            text.contains(" von "),
            "must contain literal ' von ' separator"
        );
        assert!(
            text.ends_with("anwesend"),
            "must end with the word 'anwesend'"
        );
    }

    #[test]
    fn render_healthy_en_uses_literal_of_present() {
        let stats = AttendanceStatsTO {
            present: 12,
            total: 47,
        };
        let text = render_counter_text(Some(&stats), 0, &en());
        assert_eq!(text, "12 of 47 present");
    }

    #[test]
    fn render_one_failure_still_shows_last_x_de() {
        // D-16: a single failed poll must NOT dash X — banner appears only at 2+.
        let stats = AttendanceStatsTO {
            present: 12,
            total: 47,
        };
        let text = render_counter_text(Some(&stats), 1, &de());
        assert_eq!(text, "12 von 47 anwesend");
    }

    #[test]
    fn render_two_failures_dashes_x_keeps_y_de() {
        // D-16 + UI-SPEC state table: dash X but keep Y (Member-Universe-Snapshot is stable).
        let stats = AttendanceStatsTO {
            present: 12,
            total: 47,
        };
        let text = render_counter_text(Some(&stats), 2, &de());
        assert_eq!(text, "\u{2014} von 47 anwesend");
        assert!(
            text.contains("47"),
            "Y (47) must remain visible during outage"
        );
    }

    #[test]
    fn render_many_failures_keeps_dashed_state() {
        let stats = AttendanceStatsTO {
            present: 12,
            total: 47,
        };
        let text = render_counter_text(Some(&stats), 17, &de());
        assert_eq!(text, "\u{2014} von 47 anwesend");
    }
}
