use dioxus::prelude::*;
use rest_types::DuplicateDetectionConfigTO;
use crate::i18n::{use_i18n, Key};

#[derive(Props, Clone, PartialEq)]
pub struct DuplicateConfigFormProps {
    pub config: DuplicateDetectionConfigTO,
    pub on_config_change: EventHandler<DuplicateDetectionConfigTO>,
}

#[component]
pub fn DuplicateConfigForm(props: DuplicateConfigFormProps) -> Element {
    let i18n = use_i18n();
    let config = props.config.clone();
    let on_change = props.on_config_change.clone();
    
    rsx! {
        div { class: "bg-white border border-gray-200 rounded-lg p-4",
            h3 { class: "text-lg font-semibold text-gray-900 mb-4",
                {i18n.t(Key::DetectionSettings)}
            }
            
            div { class: "space-y-4",
                // Similarity Threshold
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        "{i18n.t(Key::SimilarityThreshold)} ({config.similarity_threshold:.2})"
                    }
                    input {
                        r#type: "range",
                        class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer",
                        min: "0.3",
                        max: "0.9",
                        step: "0.05",
                        value: "{config.similarity_threshold}",
                        oninput: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |evt: dioxus::prelude::Event<dioxus::prelude::FormData>| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    let mut new_config = config.clone();
                                    new_config.similarity_threshold = val;
                                    on_change.call(new_config);
                                }
                            }
                        },
                    }
                    div { class: "text-xs text-gray-500 mt-1",
                        {i18n.t(Key::ThresholdDescription)}
                    }
                }

                // Exact Match Weight
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        "{i18n.t(Key::ExactMatchWeight)} ({config.exact_match_weight:.2})"
                    }
                    input {
                        r#type: "range",
                        class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer",
                        min: "0.0",
                        max: "1.0",
                        step: "0.1",
                        value: "{config.exact_match_weight}",
                        oninput: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |evt: dioxus::prelude::Event<dioxus::prelude::FormData>| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    let mut new_config = config.clone();
                                    new_config.exact_match_weight = val;
                                    on_change.call(new_config);
                                }
                            }
                        },
                    }
                }

                // Word Order Weight
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        "{i18n.t(Key::WordOrderWeight)} ({config.word_order_weight:.2})"
                    }
                    input {
                        r#type: "range",
                        class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer",
                        min: "0.0",
                        max: "1.0",
                        step: "0.1",
                        value: "{config.word_order_weight}",
                        oninput: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |evt: dioxus::prelude::Event<dioxus::prelude::FormData>| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    let mut new_config = config.clone();
                                    new_config.word_order_weight = val;
                                    on_change.call(new_config);
                                }
                            }
                        },
                    }
                }

                // Levenshtein Weight
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        "{i18n.t(Key::LevenshteinWeight)} ({config.levenshtein_weight:.2})"
                    }
                    input {
                        r#type: "range",
                        class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer",
                        min: "0.0",
                        max: "1.0",
                        step: "0.1",
                        value: "{config.levenshtein_weight}",
                        oninput: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |evt: dioxus::prelude::Event<dioxus::prelude::FormData>| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    let mut new_config = config.clone();
                                    new_config.levenshtein_weight = val;
                                    on_change.call(new_config);
                                }
                            }
                        },
                    }
                }

                // Jaro-Winkler Weight
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        "{i18n.t(Key::JaroWinklerWeight)} ({config.jaro_winkler_weight:.2})"
                    }
                    input {
                        r#type: "range",
                        class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer",
                        min: "0.0",
                        max: "1.0",
                        step: "0.1",
                        value: "{config.jaro_winkler_weight}",
                        oninput: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |evt: dioxus::prelude::Event<dioxus::prelude::FormData>| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    let mut new_config = config.clone();
                                    new_config.jaro_winkler_weight = val;
                                    on_change.call(new_config);
                                }
                            }
                        },
                    }
                }

                // Category Aware
                div {
                    label { class: "flex items-center",
                        input {
                            r#type: "checkbox",
                            class: "mr-2",
                            checked: config.category_aware,
                            onchange: {
                                let config = config.clone();
                                let on_change = on_change.clone();
                                move |evt: dioxus::prelude::Event<dioxus::prelude::FormData>| {
                                    let mut new_config = config.clone();
                                    new_config.category_aware = evt.checked();
                                    on_change.call(new_config);
                                }
                            },
                        }
                        span { class: "text-sm font-medium text-gray-700",
                            {i18n.t(Key::CategoryAwareDescription)}
                        }
                    }
                }

                // Reset button
                button {
                    class: "w-full px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 text-sm",
                    onclick: {
                        let on_change = on_change.clone();
                        move |_: dioxus::prelude::Event<dioxus::prelude::MouseData>| {
                            let default_config = DuplicateDetectionConfigTO {
                                similarity_threshold: 0.55,
                                exact_match_weight: 0.3,
                                word_order_weight: 0.4,
                                levenshtein_weight: 0.2,
                                jaro_winkler_weight: 0.1,
                                category_aware: true,
                            };
                            on_change.call(default_config);
                        }
                    },
                    {i18n.t(Key::ResetToDefaults)}
                }
            }
        }
    }
}