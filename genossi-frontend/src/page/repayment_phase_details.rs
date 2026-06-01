use dioxus::prelude::*;

/// Phase 12 Plan 12-05 implementiert das 3-Tab-Layout. Stub damit der Router
/// in Plan 12-03 schon eine Ziel-Component mit `id: String`-Prop hat.
#[component]
pub fn RepaymentPhaseDetails(id: String) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6",
            h1 { class: "text-2xl font-bold", "Phase-Details" }
            p { class: "text-gray-500 mt-4", "TODO Plan 12-05: 3-Tab-Layout (UI-02). Phase-ID: {id}" }
        }
    }
}
