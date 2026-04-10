use dioxus::prelude::*;

fn status_label(s: &str) -> &'static str {
    match s {
        "new" => "Neu",
        "assigned" => "Zugeordnet",
        "replied" => "Beantwortet",
        "archived" => "Archiviert",
        "ignored" => "Ignoriert",
        _ => "?",
    }
}

fn status_color(s: &str) -> &'static str {
    match s {
        "new" => "text-blue-600",
        "assigned" => "text-green-600",
        "replied" => "text-purple-600",
        "archived" => "text-gray-500",
        "ignored" => "text-gray-400",
        _ => "text-gray-600",
    }
}

#[component]
pub fn InboxStatusBadge(status: String) -> Element {
    let color = status_color(&status);
    let label = status_label(&status);
    rsx! {
        span { class: "text-xs {color}", "{label}" }
    }
}
