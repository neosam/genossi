use crate::component::SimilarityScoreBar;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;
use rest_types::{DuplicateMatchTO, MatchConfidenceTO};

#[derive(Props, Clone, PartialEq)]
pub struct DuplicateMatchProps {
    pub duplicate_match: DuplicateMatchTO,
    pub show_details: Option<bool>,
}

#[component]
pub fn DuplicateMatch(props: DuplicateMatchProps) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    
    // State for toggling detailed view (if not controlled by parent)
    let mut show_details_state = use_signal(|| props.show_details.unwrap_or(false));
    let show_details = props.show_details.unwrap_or_else(|| *show_details_state.read());
    
    let confidence_color = match props.duplicate_match.confidence {
        MatchConfidenceTO::VeryHigh => "text-green-600",
        MatchConfidenceTO::High => "text-blue-600",
        MatchConfidenceTO::Medium => "text-yellow-600",
        MatchConfidenceTO::Low => "text-red-600",
    };

    let confidence_text = match props.duplicate_match.confidence {
        MatchConfidenceTO::VeryHigh => i18n.t(Key::VeryHigh),
        MatchConfidenceTO::High => i18n.t(Key::High), 
        MatchConfidenceTO::Medium => i18n.t(Key::Medium),
        MatchConfidenceTO::Low => i18n.t(Key::Low),
    };

    let product = props.duplicate_match.product.clone();
    let scores = props.duplicate_match.algorithm_scores.clone();

    rsx! {
        div { class: "border border-gray-200 rounded-lg p-4 mb-3 hover:shadow-md transition-shadow",
            
            // Header with product info and overall score
            div { class: "flex justify-between items-start mb-3",
                div { class: "flex-1",
                    h3 { class: "text-lg font-semibold text-gray-900 mb-1",
                        {product.name.clone()}
                    }
                    div { class: "text-sm text-gray-600 space-y-1",
                        p { "EAN: {product.ean}" }
                        p { "Price: {i18n.format_price(product.price.to_cents())}" }
                        p { "Sales Unit: {product.sales_unit}" }
                    }
                }
                div { class: "text-right flex flex-col items-end",
                    div { class: "text-2xl font-bold text-gray-900 mb-1",
                        {format!("{:.1}%", props.duplicate_match.similarity_score * 100.0)}
                    }
                    div { class: format!("text-sm font-medium {}", confidence_color),
                        {confidence_text}
                    }
                    
                    // Toggle details button (only if not controlled by parent)
                    if props.show_details.is_none() {
                        button {
                            class: "mt-2 text-xs text-blue-600 hover:text-blue-800 underline",
                            onclick: move |_| {
                                let current = *show_details_state.read();
                                show_details_state.set(!current);
                            },
                            if show_details {
                                {i18n.t(Key::HideDetails)}
                            } else {
                                {i18n.t(Key::ShowDetails)}
                            }
                        }
                    }
                }
            }

            // Algorithm breakdown - only show if details are enabled
            if show_details {
                div { class: "bg-gray-50 rounded-lg p-3 mb-3",
                    h4 { class: "text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::AlgorithmBreakdown)}
                    }
                    SimilarityScoreBar {
                        score: scores.exact_match,
                        label: i18n.t(Key::ExactMatch).to_string(),
                    }
                    SimilarityScoreBar {
                        score: scores.word_order,
                        label: i18n.t(Key::WordOrder).to_string(),
                    }
                    SimilarityScoreBar {
                        score: scores.levenshtein,
                        label: i18n.t(Key::Levenshtein).to_string(),
                    }
                    SimilarityScoreBar {
                        score: scores.jaro_winkler,
                        label: i18n.t(Key::JaroWinkler).to_string(),
                    }
                    SimilarityScoreBar {
                        score: scores.category_score,
                        label: i18n.t(Key::Category).to_string(),
                    }
                }
            }

            // Action buttons
            div { class: "flex justify-between items-center",
                div { class: "flex space-x-2",
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 text-sm",
                        onclick: move |_| {
                            if let Some(id) = product.id {
                                navigator.push(Route::ProductDetails { id: id.to_string() });
                            }
                        },
                        {i18n.t(Key::ViewProduct)}
                    }
                    button {
                        class: "px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 text-sm",
                        onclick: move |_| {
                            // TODO: Implement merge functionality
                        },
                        {i18n.t(Key::SuggestMerge)}
                    }
                }
                
                // Show summary stats when details are hidden
                if !show_details {
                    div { class: "text-xs text-gray-500",
                        "Exact: {(scores.exact_match * 100.0):.0}% | "
                        "Leven: {(scores.levenshtein * 100.0):.0}% | "
                        "J-W: {(scores.jaro_winkler * 100.0):.0}%"
                    }
                }
            }
        }
    }
}