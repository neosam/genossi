use dioxus::prelude::*;

#[component]
pub fn InboxStatusBadge(replied: bool, done: bool, archived: bool) -> Element {
    rsx! {
        span { class: "flex gap-1 items-center",
            if done {
                span { class: "text-xs text-green-600", "Erledigt" }
            }
            if replied {
                span { class: "text-xs text-purple-600", "Beantwortet" }
            }
            if archived {
                span { class: "text-xs text-gray-500", "Archiviert" }
            }
            if !done && !replied && !archived {
                span { class: "text-xs text-blue-600", "Offen" }
            }
        }
    }
}
