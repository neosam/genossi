//! ToastContainer + show_toast (Phase 4 Plan 06) — Component-First-Extraction
//! aus members.rs:49-62 inline-Helper.
//!
//! Used in helper_attendance.rs (Plan 09) for toggle-error toasts (D-17),
//! assembly_details.rs (Plan 08) for form errors, assemblies.rs (Plan 08) for create errors.
//!
//! NOTE: members.rs has NOT been migrated in this plan — keeping blast-radius small
//! per Plan-06 instruction. The inline helper there remains; future refactor can migrate.
//!
//! Phase 18 (this file): added ToastVariant + show_success_toast + SuccessToastContainer
//! without touching the existing red-error API (Zero Blast Radius for v1.0/v1.1 callsites).
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

/// Phase 18 — Variant of a toast (visual styling).
/// `Error` is the legacy default (red); `Success` is green per UI-SPEC.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToastVariant {
    Success,
    Error,
}

// ─── Existing API (unchanged) ─────────────────────────────────────
// All v1.0/v1.1 callsites use these — bg-red-600.

/// Push a new toast onto the toast list. Auto-removes after 5 seconds.
/// Pattern unverändert aus members.rs:49-62.
pub fn show_toast(
    toast_messages: &mut Signal<Vec<(u64, String)>>,
    toast_counter: &mut Signal<u64>,
    msg: String,
) {
    let id = *toast_counter.read();
    *toast_counter.write() += 1;
    toast_messages.write().push((id, msg));
    let mut toast_messages = toast_messages.clone();
    spawn(async move {
        TimeoutFuture::new(5_000).await;
        toast_messages.write().retain(|(tid, _)| *tid != id);
    });
}

#[component]
pub fn ToastContainer(messages: ReadOnlySignal<Vec<(u64, String)>>) -> Element {
    let msgs = messages.read();
    if msgs.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            // z-[60] > TopBar z-50 — sonst werden Desktop-Toasts (md:top-4) verdeckt (UAT Defekt #2).
            class: "fixed z-[60] bottom-4 left-1/2 -translate-x-1/2 md:bottom-auto md:left-auto md:top-4 md:right-4 md:translate-x-0 flex flex-col gap-2 max-w-md print:hidden",
            for (id, msg) in msgs.iter() {
                div {
                    key: "{id}",
                    class: "bg-red-600 text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3",
                    "{msg}"
                }
            }
        }
    }
}

// ─── Phase 18 — Success-Toast API (additive, separate Signal-Bucket) ────

/// Phase 18 D-18-08 — Push a green success toast onto a dedicated success-bucket.
/// Same auto-dismiss behaviour as `show_toast` (5s).
pub fn show_success_toast(
    toast_messages: &mut Signal<Vec<(u64, String)>>,
    toast_counter: &mut Signal<u64>,
    msg: String,
) {
    let id = *toast_counter.read();
    *toast_counter.write() += 1;
    toast_messages.write().push((id, msg));
    let mut toast_messages = toast_messages.clone();
    spawn(async move {
        TimeoutFuture::new(5_000).await;
        toast_messages.write().retain(|(tid, _)| *tid != id);
    });
}

/// Phase 18 D-18-08 — Green-styled container for success messages.
/// Caller mounts BOTH `ToastContainer` and `SuccessToastContainer` on the page
/// (with separate `Signal<Vec<(u64, String)>>` signals) to display both variants.
#[component]
pub fn SuccessToastContainer(messages: ReadOnlySignal<Vec<(u64, String)>>) -> Element {
    let msgs = messages.read();
    if msgs.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            // Slightly offset to avoid overlap when both Error + Success toasts are visible.
            // On mobile: stacks above the error container; on desktop: stacks below.
            class: "fixed z-[60] bottom-20 left-1/2 -translate-x-1/2 md:bottom-auto md:left-auto md:top-20 md:right-4 md:translate-x-0 flex flex-col gap-2 max-w-md print:hidden",
            for (id, msg) in msgs.iter() {
                div {
                    key: "{id}",
                    class: "bg-green-600 text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3",
                    "{msg}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_variant_distinct_values() {
        assert_ne!(ToastVariant::Success, ToastVariant::Error);
    }

    #[test]
    fn toast_variant_copy_and_compare() {
        let v = ToastVariant::Success;
        let w = v; // Copy
        assert_eq!(v, w);
        assert_eq!(format!("{:?}", v), "Success");
        assert_eq!(format!("{:?}", ToastVariant::Error), "Error");
    }
}
