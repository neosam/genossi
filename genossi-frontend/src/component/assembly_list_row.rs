//! AssemblyListRow (Phase 4 Plan 06) — Card-style row in /assemblies list page.
//!
//! Wraps in dioxus_router::components::Link to navigate to /assemblies/{id}.
//! Pattern-Vorlage: application_list.rs:53-77 (Row-Pattern als Card statt Table).
use dioxus::prelude::*;

use crate::api::AssemblyTO;
use crate::component::AssemblyStatusBadge;
use crate::router::Route;

#[component]
pub fn AssemblyListRow(assembly: AssemblyTO) -> Element {
    let id = assembly.id;
    let date_str = assembly.date.clone().unwrap_or_default();
    let location_str = assembly.location.clone().unwrap_or_default();
    let name = assembly.name.clone();
    let status = assembly.status.clone();
    rsx! {
        Link {
            to: Route::AssemblyDetails { id: id.to_string() },
            class: "block",
            div { class: "flex items-center justify-between bg-white border border-gray-200 rounded-lg px-4 py-3 mb-2 hover:bg-gray-50 transition-colors",
                div { class: "flex flex-col gap-1",
                    h3 { class: "font-medium text-gray-900", "{name}" }
                    span { class: "text-sm text-gray-500",
                        "{date_str}"
                        if !location_str.is_empty() {
                            span { class: "ml-2", "{location_str}" }
                        }
                    }
                }
                AssemblyStatusBadge { status: status }
            }
        }
    }
}
