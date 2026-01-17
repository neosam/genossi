use dioxus::prelude::*;
use rest_types::InventurStatisticsTO;
use uuid::Uuid;

use crate::api::get_inventur_statistics;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[component]
pub fn InventurStatistics(inventur_id: Uuid) -> Element {
    let i18n = use_i18n();
    let mut statistics = use_signal::<Option<InventurStatisticsTO>>(|| None);
    let mut loading = use_signal(|| true);
    let mut error = use_signal::<Option<String>>(|| None);

    // Load statistics on mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            let config = CONFIG.read().clone();
            match get_inventur_statistics(&config, inventur_id).await {
                Ok(stats) => {
                    statistics.set(Some(stats));
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    });

    let is_loading = *loading.read();
    let error_msg = error.read().clone();
    let stats = statistics.read().clone();

    if is_loading {
        return rsx! {
            div { class: "bg-white shadow rounded-lg p-4",
                div { class: "animate-pulse",
                    div { class: "h-4 bg-gray-200 rounded w-1/4 mb-4" }
                    div { class: "grid grid-cols-3 gap-4",
                        div { class: "h-8 bg-gray-200 rounded" }
                        div { class: "h-8 bg-gray-200 rounded" }
                        div { class: "h-8 bg-gray-200 rounded" }
                    }
                }
            }
        };
    }

    if let Some(err) = error_msg {
        return rsx! {
            div { class: "bg-red-50 border border-red-200 rounded-lg p-4",
                p { class: "text-red-600", "{err}" }
            }
        };
    }

    if let Some(stats) = stats {
        rsx! {
            div { class: "bg-white shadow rounded-lg p-4",
                h3 { class: "text-lg font-semibold text-gray-900 mb-4",
                    "{i18n.t(Key::Statistics)}"
                }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    // Total Value
                    div { class: "bg-blue-50 rounded-lg p-4",
                        p { class: "text-sm text-blue-600 font-medium",
                            "{i18n.t(Key::TotalValue)}"
                        }
                        p { class: "text-2xl font-bold text-blue-900",
                            "{i18n.format_price(stats.total_value_cents)}"
                        }
                    }
                    // Total Entries
                    div { class: "bg-green-50 rounded-lg p-4",
                        p { class: "text-sm text-green-600 font-medium",
                            "{i18n.t(Key::TotalEntries)}"
                        }
                        p { class: "text-2xl font-bold text-green-900",
                            "{stats.total_entries}"
                        }
                    }
                    // Products with Entries
                    div { class: "bg-purple-50 rounded-lg p-4",
                        p { class: "text-sm text-purple-600 font-medium",
                            "{i18n.t(Key::ProductsWithEntries)}"
                        }
                        p { class: "text-2xl font-bold text-purple-900",
                            "{stats.products_with_entries}"
                        }
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}
