//! ToastContainer + show_toast (Phase 4 Plan 06) — Component-First-Extraction
//! aus members.rs:49-62 inline-Helper.
//!
//! Used in helper_attendance.rs (Plan 09) for toggle-error toasts (D-17),
//! assembly_details.rs (Plan 08) for form errors, assemblies.rs (Plan 08) for create errors.
//!
//! NOTE: members.rs has NOT been migrated in this plan — keeping blast-radius small
//! per Plan-06 instruction. The inline helper there remains; future refactor can migrate.
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

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
