use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, ApplicationStatusTO, ApplicationTO};
use crate::service::config::CONFIG;

const MAX_RESULTS: usize = 10;

pub fn filter_applications<'a>(
    applications: &'a [ApplicationTO],
    query: &str,
) -> Vec<&'a ApplicationTO> {
    if query.is_empty() {
        return Vec::new();
    }
    let query_lower = query.to_lowercase();
    let mut results: Vec<&ApplicationTO> = applications
        .iter()
        .filter(|a| {
            a.status == ApplicationStatusTO::Offen
                && (a.first_name.to_lowercase().contains(&query_lower)
                    || a.last_name.to_lowercase().contains(&query_lower))
        })
        .collect();
    results.sort_by(|a, b| a.last_name.cmp(&b.last_name));
    results.truncate(MAX_RESULTS);
    results
}

fn format_application(a: &ApplicationTO) -> String {
    format!("{} {} ({} Anteile)", a.first_name, a.last_name, a.shares)
}

#[component]
pub fn ApplicationSearch(
    on_select: EventHandler<Option<Uuid>>,
    selected_id: Option<Uuid>,
) -> Element {
    let mut query = use_signal(|| String::new());
    let mut show_dropdown = use_signal(|| false);
    let mut applications = use_signal(|| Vec::<ApplicationTO>::new());

    // Load open applications on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::get_applications(&config, Some("Offen")).await {
                Ok(apps) => applications.set(apps),
                Err(e) => tracing::error!("Failed to load applications: {}", e),
            }
        });
    });

    let apps = applications.read();

    // Find selected application for display
    let selected_app: Option<&ApplicationTO> = selected_id
        .and_then(|sid| apps.iter().find(|a| a.id == sid));

    // Filter results
    let filtered = filter_applications(&apps, &query.read());

    rsx! {
        div {
            class: "relative",
            onfocusout: move |_| {
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(150).await;
                    show_dropdown.set(false);
                });
            },

            if let Some(app) = selected_app {
                div {
                    class: "flex items-center gap-2 w-full px-3 py-2 border border-gray-300 rounded-md bg-gray-50",
                    span { class: "flex-1", "{format_application(app)}" }
                    button {
                        class: "text-gray-500 hover:text-gray-700 font-bold",
                        r#type: "button",
                        onclick: move |e| {
                            e.stop_propagation();
                            query.set(String::new());
                            on_select.call(None);
                        },
                        "\u{2715}"
                    }
                }
            } else {
                input {
                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    placeholder: "Name suchen...",
                    value: "{query}",
                    oninput: move |e| {
                        query.set(e.value().clone());
                        show_dropdown.set(!e.value().is_empty());
                    },
                    onfocus: move |_| {
                        if !query.read().is_empty() {
                            show_dropdown.set(true);
                        }
                    },
                }

                if *show_dropdown.read() && !filtered.is_empty() {
                    div {
                        class: "absolute z-20 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-y-auto",
                        for app in filtered.iter() {
                            {
                                let app_id = app.id;
                                let display = format_application(app);
                                rsx! {
                                    button {
                                        class: "w-full text-left px-3 py-2 hover:bg-blue-50 cursor-pointer border-b border-gray-100 last:border-b-0",
                                        r#type: "button",
                                        onmousedown: move |e| {
                                            e.stop_propagation();
                                            show_dropdown.set(false);
                                            query.set(String::new());
                                            on_select.call(Some(app_id));
                                        },
                                        "{display}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app(id: Uuid, first: &str, last: &str, shares: i32, status: ApplicationStatusTO) -> ApplicationTO {
        ApplicationTO {
            id,
            first_name: first.to_string(),
            last_name: last.to_string(),
            salutation: None,
            title: None,
            email: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            shares,
            status,
            created: None,
            deleted: None,
            version: None,
        }
    }

    fn test_applications() -> Vec<ApplicationTO> {
        vec![
            make_app(Uuid::from_u128(1), "Anna", "Weber", 3, ApplicationStatusTO::Offen),
            make_app(Uuid::from_u128(2), "Karl", "Schmidt", 1, ApplicationStatusTO::Offen),
            make_app(Uuid::from_u128(3), "Maria", "Müller", 5, ApplicationStatusTO::Bestaetigt),
            make_app(Uuid::from_u128(4), "Hans", "Weber", 2, ApplicationStatusTO::Abgelehnt),
            make_app(Uuid::from_u128(5), "Fritz", "Weber", 1, ApplicationStatusTO::Offen),
        ]
    }

    #[test]
    fn test_filter_only_open() {
        let apps = test_applications();
        let results = filter_applications(&apps, "weber");
        // Only Anna and Fritz are Offen, Hans is Abgelehnt
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|a| a.status == ApplicationStatusTO::Offen));
    }

    #[test]
    fn test_filter_by_first_name() {
        let apps = test_applications();
        let results = filter_applications(&apps, "anna");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].first_name, "Anna");
    }

    #[test]
    fn test_filter_empty_query() {
        let apps = test_applications();
        let results = filter_applications(&apps, "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_no_match() {
        let apps = test_applications();
        let results = filter_applications(&apps, "zzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_excludes_confirmed() {
        let apps = test_applications();
        let results = filter_applications(&apps, "müller");
        // Maria Müller is Bestaetigt, should be excluded
        assert!(results.is_empty());
    }

    #[test]
    fn test_format_application() {
        let app = make_app(Uuid::from_u128(1), "Anna", "Weber", 3, ApplicationStatusTO::Offen);
        assert_eq!(format_application(&app), "Anna Weber (3 Anteile)");
    }
}
