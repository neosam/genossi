use dioxus::prelude::*;
use rest_types::{CommunicationDirection, CommunicationEntryTO};

use crate::i18n::{use_i18n, Key};
use crate::router::Route;

#[component]
pub fn CommunicationTimeline(
    entries: Vec<CommunicationEntryTO>,
    /// Optional, additiver Klick-Handler (D-06). Wenn gesetzt, wird die
    /// Betreff-Zelle als klickbares `span` (statt hartem `Link`) gerendert und
    /// ruft den Handler mit dem geklickten Eintrag auf. Ohne Handler bleibt der
    /// bestehende `Link`-Pfad exakt erhalten (Member-Nutzung unveraendert).
    #[props(default)]
    on_entry_click: Option<EventHandler<CommunicationEntryTO>>,
) -> Element {
    let i18n = use_i18n();

    if entries.is_empty() {
        return rsx! {
            p { class: "text-gray-500 italic", {i18n.t(Key::CommunicationNone)} }
        };
    }

    rsx! {
        table { class: "min-w-full divide-y divide-gray-200",
            thead { class: "bg-gray-50",
                tr {
                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase",
                        {i18n.t(Key::Date)}
                    }
                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase",
                        ""
                    }
                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase",
                        {i18n.t(Key::MailSubject)}
                    }
                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase",
                        {i18n.t(Key::MailStatus)}
                    }
                }
            }
            tbody { class: "bg-white divide-y divide-gray-200",
                for entry in entries.iter() {
                    {render_entry(&i18n, entry, on_entry_click.as_ref())}
                }
            }
        }
    }
}

fn render_entry(
    i18n: &crate::i18n::I18n,
    entry: &CommunicationEntryTO,
    on_entry_click: Option<&EventHandler<CommunicationEntryTO>>,
) -> Element {
    let is_inbound = entry.direction == CommunicationDirection::Inbound;
    let direction_label = if is_inbound {
        i18n.t(Key::CommunicationInbound)
    } else {
        i18n.t(Key::CommunicationOutbound)
    };
    let direction_icon = if is_inbound { "\u{2190}" } else { "\u{2192}" };
    let direction_color = if is_inbound {
        "text-blue-600"
    } else {
        "text-green-600"
    };

    let address = if is_inbound {
        entry.from_address.clone().unwrap_or_default()
    } else {
        entry.to_address.clone().unwrap_or_default()
    };

    let status_badges = render_status(i18n, entry);
    let date_str = i18n.format_datetime(&entry.date);
    let subject = entry.subject.clone();

    // Deep link to specific mail detail
    let link_route = if is_inbound {
        Route::InboxDetail {
            id: entry.inbox_id.clone().unwrap_or_default(),
        }
    } else {
        Route::MailJobDetail {
            id: entry.mail_job_id.clone().unwrap_or_default(),
        }
    };

    rsx! {
        tr { class: "hover:bg-gray-50",
            td { class: "px-4 py-3 text-sm text-gray-600 whitespace-nowrap",
                "{date_str}"
            }
            td { class: "px-4 py-3 text-sm whitespace-nowrap",
                span { class: "font-medium {direction_color}",
                    "{direction_icon} {direction_label}"
                }
                span { class: "text-gray-400 text-xs ml-2",
                    "{address}"
                }
            }
            td { class: "px-4 py-3 text-sm",
                if let Some(handler) = on_entry_click {
                    {
                        let handler = *handler;
                        let entry_owned = entry.clone();
                        rsx! {
                            span {
                                class: "text-blue-600 hover:underline cursor-pointer",
                                onclick: move |_| handler.call(entry_owned.clone()),
                                "{subject}"
                            }
                        }
                    }
                } else {
                    Link {
                        to: link_route,
                        class: "text-blue-600 hover:underline",
                        "{subject}"
                    }
                }
            }
            td { class: "px-4 py-3 text-sm",
                {status_badges}
            }
        }
    }
}

fn render_status(i18n: &crate::i18n::I18n, entry: &CommunicationEntryTO) -> Element {
    if entry.direction == CommunicationDirection::Inbound {
        if let Some(ref status) = entry.inbound_status {
            let mut badges = Vec::new();
            if status.done {
                badges.push((
                    "bg-green-100 text-green-800",
                    i18n.t(Key::CommunicationStatusDone),
                ));
            }
            if status.replied {
                badges.push((
                    "bg-blue-100 text-blue-800",
                    i18n.t(Key::CommunicationStatusReplied),
                ));
            }
            if status.archived {
                badges.push((
                    "bg-gray-100 text-gray-800",
                    i18n.t(Key::CommunicationStatusArchived),
                ));
            }
            if badges.is_empty() {
                badges.push((
                    "bg-yellow-100 text-yellow-800",
                    i18n.t(Key::CommunicationStatusPending),
                ));
            }
            return rsx! {
                div { class: "flex gap-1 flex-wrap",
                    for (color, label) in badges {
                        span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {color}",
                            {label}
                        }
                    }
                }
            };
        }
    }

    // Outbound status
    if let Some(ref status) = entry.outbound_status {
        let (color, label) = match status.as_str() {
            "sent" => (
                "bg-green-100 text-green-800",
                i18n.t(Key::CommunicationStatusSent),
            ),
            "failed" => (
                "bg-red-100 text-red-800",
                i18n.t(Key::CommunicationStatusFailed),
            ),
            _ => (
                "bg-yellow-100 text-yellow-800",
                i18n.t(Key::CommunicationStatusPending),
            ),
        };
        return rsx! {
            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {color}",
                {label}
            }
        };
    }

    rsx! {}
}
