use crate::auth::RequirePrivilege;
use crate::component::{DuplicateConfigForm, DuplicateResultsList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::duplicate_detection::{
    check_duplicates, clear_error, find_all_duplicates, find_duplicates_by_ean, update_config,
    DUPLICATE_DETECTION,
};
use crate::service::product::PRODUCTS;
use dioxus::prelude::*;
use rest_types::{CheckDuplicateRequestTO, DuplicateDetectionConfigTO};

#[component]
pub fn DuplicateDetection() -> Element {
    let i18n = use_i18n();
    let duplicate_state = DUPLICATE_DETECTION.read();
    let products = PRODUCTS.read();
    
    let mut active_tab = use_signal(|| "scan_all".to_string());
    let mut check_name = use_signal(|| "".to_string());
    let mut check_sales_unit = use_signal(|| "".to_string());
    let mut check_requires_weighing = use_signal(|| false);
    let mut selected_product_ean = use_signal(|| "".to_string());

    let handle_scan_all = move |_| {
        spawn(async move {
            find_all_duplicates().await;
        });
    };

    let handle_check_product = move |_| {
        if selected_product_ean.read().is_empty() {
            return;
        }
        let ean = selected_product_ean.read().clone();
        spawn(async move {
            find_duplicates_by_ean(ean).await;
        });
    };

    let handle_check_new_product = move |_| {
        if check_name.read().trim().is_empty() {
            return;
        }
        let request = CheckDuplicateRequestTO {
            name: check_name.read().clone(),
            sales_unit: check_sales_unit.read().clone(),
            requires_weighing: *check_requires_weighing.read(),
        };
        spawn(async move {
            check_duplicates(request).await;
        });
    };

    let handle_config_change = move |config: DuplicateDetectionConfigTO| {
        update_config(config);
    };

    rsx! {
        RequirePrivilege {
            privilege: "detect_duplicates",
            fallback: rsx! { AccessDeniedPage { required_privilege: "detect_duplicates".to_string() } },
            div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                div { class: "mb-6",
                    h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                        {i18n.t(Key::CheckDuplicates)}
                    }
                    p { class: "text-gray-600",
                        {i18n.t(Key::DuplicateDetectionDescription)}
                    }
                }

                // Error display
                if let Some(error) = &duplicate_state.error {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6",
                        div { class: "flex justify-between items-center",
                            span { {error.clone()} }
                            button {
                                class: "text-red-700 hover:text-red-900",
                                onclick: move |_| clear_error(),
                                "×"
                            }
                        }
                    }
                }

                div { class: "grid grid-cols-1 lg:grid-cols-4 gap-6",
                    // Left sidebar with controls
                    div { class: "lg:col-span-1 space-y-6",
                        // Tab navigation
                        div { class: "bg-white border border-gray-200 rounded-lg p-4",
                            h3 { class: "text-lg font-semibold text-gray-900 mb-4",
                                {i18n.t(Key::DetectionMode)}
                            }
                            div { class: "space-y-2",
                                button {
                                    class: if active_tab.read().as_str() == "scan_all" {
                                        "w-full text-left px-3 py-2 bg-blue-100 text-blue-700 rounded-md"
                                    } else {
                                        "w-full text-left px-3 py-2 text-gray-700 hover:bg-gray-100 rounded-md"
                                    },
                                    onclick: move |_| active_tab.set("scan_all".to_string()),
                                    {i18n.t(Key::ScanAllProducts)}
                                }
                                button {
                                    class: if active_tab.read().as_str() == "check_product" {
                                        "w-full text-left px-3 py-2 bg-blue-100 text-blue-700 rounded-md"
                                    } else {
                                        "w-full text-left px-3 py-2 text-gray-700 hover:bg-gray-100 rounded-md"
                                    },
                                    onclick: move |_| active_tab.set("check_product".to_string()),
                                    {i18n.t(Key::CheckSpecificProduct)}
                                }
                                button {
                                    class: if active_tab.read().as_str() == "check_new" {
                                        "w-full text-left px-3 py-2 bg-blue-100 text-blue-700 rounded-md"
                                    } else {
                                        "w-full text-left px-3 py-2 text-gray-700 hover:bg-gray-100 rounded-md"
                                    },
                                    onclick: move |_| active_tab.set("check_new".to_string()),
                                    {i18n.t(Key::CheckNewProduct)}
                                }
                            }
                        }

                        // Mode-specific controls
                        div { class: "bg-white border border-gray-200 rounded-lg p-4",
                            match active_tab.read().as_str() {
                                "scan_all" => rsx! {
                                    div {
                                        h3 { class: "text-lg font-semibold text-gray-900 mb-3",
                                            {i18n.t(Key::ScanAllProducts)}
                                        }
                                        p { class: "text-sm text-gray-600 mb-4",
                                            "Scannen Sie die gesamte Produktdatenbank nach potenziellen Duplikaten."
                                        }
                                        button {
                                            class: "w-full px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50",
                                            disabled: duplicate_state.loading,
                                            onclick: handle_scan_all,
                                            if duplicate_state.loading {
                                                {i18n.t(Key::Scanning)}
                                            } else {
                                                {i18n.t(Key::StartScan)}
                                            }
                                        }
                                    }
                                },
                                "check_product" => rsx! {
                                    div {
                                        h3 { class: "text-lg font-semibold text-gray-900 mb-3",
                                            {i18n.t(Key::CheckSpecificProduct)}
                                        }
                                        p { class: "text-sm text-gray-600 mb-4",
                                            "Finden Sie Duplikate für ein bestimmtes Produkt anhand der EAN."
                                        }
                                        select {
                                            class: "w-full px-3 py-2 border border-gray-300 rounded-md mb-4",
                                            value: selected_product_ean.read().clone(),
                                            onchange: move |evt| selected_product_ean.set(evt.value()),
                                            option { value: "", {i18n.t(Key::SelectProductOption)} }
                                            for product in products.items.iter() {
                                                option { 
                                                    value: product.ean.clone(),
                                                    "{product.name} ({product.ean})"
                                                }
                                            }
                                        }
                                        button {
                                            class: "w-full px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50",
                                            disabled: duplicate_state.loading || selected_product_ean.read().is_empty(),
                                            onclick: handle_check_product,
                                            if duplicate_state.loading {
                                                {i18n.t(Key::Checking)}
                                            } else {
                                                {i18n.t(Key::CheckProduct)}
                                            }
                                        }
                                    }
                                },
                                "check_new" => rsx! {
                                    div {
                                        h3 { class: "text-lg font-semibold text-gray-900 mb-3",
                                            {i18n.t(Key::CheckNewProduct)}
                                        }
                                        p { class: "text-sm text-gray-600 mb-4",
                                            "Prüfen Sie, ob ein neues Produkt ein Duplikat wäre, bevor Sie es erstellen."
                                        }
                                        div { class: "space-y-3",
                                            input {
                                                class: "w-full px-3 py-2 border border-gray-300 rounded-md",
                                                placeholder: "{i18n.t(Key::ProductNamePlaceholder)}",
                                                value: check_name.read().clone(),
                                                oninput: move |evt| check_name.set(evt.value()),
                                            }
                                            input {
                                                class: "w-full px-3 py-2 border border-gray-300 rounded-md",
                                                placeholder: "{i18n.t(Key::SalesUnitPlaceholder)}",
                                                value: check_sales_unit.read().clone(),
                                                oninput: move |evt| check_sales_unit.set(evt.value()),
                                            }
                                            label { class: "flex items-center",
                                                input {
                                                    r#type: "checkbox",
                                                    class: "mr-2",
                                                    checked: *check_requires_weighing.read(),
                                                    onchange: move |evt| check_requires_weighing.set(evt.checked()),
                                                }
                                                span { class: "text-sm text-gray-700",
                                                    {i18n.t(Key::RequiresWeighing)}
                                                }
                                            }
                                            button {
                                                class: "w-full px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50",
                                                disabled: duplicate_state.loading || check_name.read().trim().is_empty(),
                                                onclick: handle_check_new_product,
                                                if duplicate_state.loading {
                                                    {i18n.t(Key::Checking)}
                                                } else {
                                                    {i18n.t(Key::CheckForDuplicates)}
                                                }
                                            }
                                        }
                                    }
                                },
                                _ => rsx! { div {} }
                            }
                        }

                        // Configuration form
                        DuplicateConfigForm {
                            config: duplicate_state.config.clone(),
                            on_config_change: handle_config_change,
                        }
                    }

                    // Main content area
                    div { class: "lg:col-span-3",
                        div { class: "bg-white border border-gray-200 rounded-lg p-6",
                            // Results display
                            match active_tab.read().as_str() {
                                "scan_all" => {
                                    if duplicate_state.all_duplicates.is_empty() && !duplicate_state.loading {
                                        rsx! {
                                            div { class: "text-center py-12",
                                                div { class: "text-gray-500 text-lg mb-4",
                                                    {i18n.t(Key::NoScanPerformed)}
                                                }
                                                p { class: "text-gray-400",
                                                    {i18n.t(Key::ClickStartScanDescription)}
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div {
                                                DuplicateResultsList {
                                                    results: duplicate_state.all_duplicates.clone(),
                                                }
                                            }
                                        }
                                    }
                                },
                                "check_product" => {
                                    if let Some(result) = &duplicate_state.current_duplicates {
                                        rsx! {
                                            div {
                                                DuplicateResultsList {
                                                    results: vec![result.clone()],
                                                }
                                            }
                                        }
                                    } else if !duplicate_state.loading {
                                        rsx! {
                                            div { class: "text-center py-12",
                                                div { class: "text-gray-500 text-lg mb-4",
                                                    {i18n.t(Key::NoProductChecked)}
                                                }
                                                p { class: "text-gray-400",
                                                    {i18n.t(Key::SelectProductDescription)}
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! { div {} }
                                    }
                                },
                                "check_new" => {
                                    if !duplicate_state.potential_matches.is_empty() {
                                        rsx! {
                                            div {
                                                h3 { class: "text-lg font-semibold text-gray-900 mb-4",
                                                    {i18n.t(Key::PotentialDuplicateMatches)}
                                                }
                                                div { class: "space-y-3",
                                                    for duplicate_match in duplicate_state.potential_matches.iter() {
                                                        crate::component::DuplicateMatch {
                                                            duplicate_match: duplicate_match.clone(),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if !duplicate_state.loading {
                                        rsx! {
                                            div { class: "text-center py-12",
                                                div { class: "text-gray-500 text-lg mb-4",
                                                    {i18n.t(Key::NoProductChecked)}
                                                }
                                                p { class: "text-gray-400",
                                                    {i18n.t(Key::EnterProductDescription)}
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! { div {} }
                                    }
                                },
                                _ => rsx! { div {} }
                            }

                            // Loading indicator
                            if duplicate_state.loading {
                                div { class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                                    div { class: "bg-white rounded-lg p-6 flex items-center space-x-3",
                                        div { class: "animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" }
                                        span { class: "text-gray-700",
                                            {i18n.t(Key::Processing)}
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
    }
}