//! Inline-Cell-Edit fuer share_count_to_pay_out in der RepaymentEntryList (D-13).
//!
//! Phase-12-Eigen-Design — KEIN Codebase-Analog:
//! - `member_details.rs` nutzt Page-Level-Edit-Toggle (alle Felder gleichzeitig)
//! - Diese Component ist Cell-Level (Click-Zelle -> Input -> Save/Cancel)
//!
//! Spezialisiert auf i32 (Open-Question 3): wenn weitere Inline-Cell-Edit-Cases
//! auftauchen (v1.2+), refactor zu `EditableCell<T>`. Aktuell ein Use-Case.

use dioxus::prelude::*;

/// Backend-Constraint (Phase 8 D-11.3 + CHECK-Constraint): share_count_to_pay_out > 0.
pub fn is_share_count_valid(n: i32) -> bool {
    n > 0
}

#[component]
pub fn EditableShareCountCell(
    value: i32,
    #[props(default)] disabled: bool,
    on_save: EventHandler<i32>,
) -> Element {
    let mut editing = use_signal(|| false);
    let mut local_value = use_signal(move || value);

    // Wenn die value-Prop sich aendert (Parent reloaded), lokalen Wert synchronisieren.
    // Dioxus 0.6: use_effect mit dependencies via Closure-Capture (value).
    use_effect(move || {
        local_value.set(value);
    });

    // D-13: Status=PaidOut blockt Inline-Edit — reiner Anzeige-Modus.
    if disabled {
        return rsx! {
            span { class: "text-gray-700", "{value}" }
        };
    }

    if *editing.read() {
        rsx! {
            div { class: "flex items-center gap-1",
                input {
                    r#type: "number",
                    class: "w-16 px-2 py-1 border border-gray-300 rounded",
                    value: "{local_value.read()}",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            local_value.set(n);
                        }
                    },
                }
                button {
                    r#type: "button",
                    class: "text-green-600 hover:text-green-800 px-2",
                    title: "Speichern",
                    disabled: !is_share_count_valid(*local_value.read()),
                    onclick: move |_| {
                        let v = *local_value.read();
                        if is_share_count_valid(v) {
                            on_save.call(v);
                            editing.set(false);
                        }
                    },
                    "✓"
                }
                button {
                    r#type: "button",
                    class: "text-red-600 hover:text-red-800 px-2",
                    title: "Abbrechen",
                    onclick: move |_| {
                        local_value.set(value);
                        editing.set(false);
                    },
                    "✗"
                }
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                class: "hover:bg-blue-50 cursor-pointer px-2 py-1 rounded",
                onclick: move |_| editing.set(true),
                "{value}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_count_valid_positive() {
        assert!(is_share_count_valid(1));
        assert!(is_share_count_valid(100));
        assert!(is_share_count_valid(i32::MAX));
    }

    #[test]
    fn share_count_invalid_zero() {
        assert!(!is_share_count_valid(0));
    }

    #[test]
    fn share_count_invalid_negative() {
        assert!(!is_share_count_valid(-1));
        assert!(!is_share_count_valid(i32::MIN));
    }
}
