use dioxus::prelude::*;
use uuid::Uuid;
use rest_types::ProductTO;
use crate::i18n::{use_i18n, Key};
use crate::service::product::{PRODUCTS, ProductService};

#[component]
pub fn SearchableProductSelector(
    selected_product_id: Option<Uuid>,
    disabled: bool,
    on_product_selected: EventHandler<Option<Uuid>>,
) -> Element {
    let i18n = use_i18n();
    let product_service = use_coroutine_handle::<ProductService>();
    
    let mut search_query = use_signal(|| String::new());
    let mut selected_product = use_signal(|| None::<ProductTO>);
    let mut show_results = use_signal(|| false);
    let mut search_timer = use_signal(|| None::<gloo_timers::callback::Timeout>);
    
    // Read global state directly for proper Dioxus reactivity
    let products_state = PRODUCTS.read();
    let search_results = &products_state.search_results;
    let search_loading = products_state.search_loading;
    let error = &products_state.error;

    // Load the selected product on mount if provided
    use_effect({
        let selected_product_id = selected_product_id;
        move || {
            if let Some(product_id) = selected_product_id {
                // Find the product in the loaded products
                let products_state = PRODUCTS.read();
                if let Some(product) = products_state.items.iter().find(|p| p.id == Some(product_id)) {
                    selected_product.set(Some(product.clone()));
                }
            }
        }
    });


    let mut handle_input_change = move |value: String| {
        search_query.set(value.clone());
        
        // Cancel any existing timer
        if let Some(timer) = search_timer.write().take() {
            drop(timer); // Cancel the previous timer
        }
        
        if value.is_empty() {
            selected_product.set(None);
            on_product_selected.call(None);
            show_results.set(false);
        } else if value.len() >= 2 {
            // Set a new timer for 500ms
            let timer_value = value.clone();
            let product_service_clone = product_service.clone();
            
            let timer = gloo_timers::callback::Timeout::new(500, move || {
                // Send search event after 500ms of no typing
                product_service_clone.send(ProductService::SearchProducts(timer_value));
            });
            
            search_timer.set(Some(timer));
            show_results.set(true);
        } else {
            show_results.set(false);
        }
    };

    let mut handle_product_select = move |product: ProductTO| {
        selected_product.set(Some(product.clone()));
        search_query.set(format!("{} ({})", product.name, product.ean));
        show_results.set(false);
        on_product_selected.call(product.id);
    };

    let mut handle_clear = move |_| {
        search_query.set(String::new());
        selected_product.set(None);
        show_results.set(false);
        on_product_selected.call(None);
    };

    let input_value = if let Some(product) = selected_product() {
        format!("{} ({})", product.name, product.ean)
    } else {
        search_query()
    };

    rsx! {
        div { class: "relative",
            // Search input
            div { class: "relative",
                input {
                    r#type: "text",
                    class: "w-full px-3 py-2 border rounded-md pr-8",
                    placeholder: i18n.t(Key::Search).to_string() + "...",
                    value: input_value,
                    disabled: disabled,
                    oninput: move |event| {
                        if !disabled {
                            handle_input_change(event.value());
                        }
                    },
                    onfocus: move |_| {
                        if !search_query().is_empty() && search_query().len() >= 2 {
                            show_results.set(true);
                        }
                    }
                }
                
                // Clear button
                if !search_query().is_empty() && !disabled {
                    button {
                        class: "absolute right-2 top-1/2 transform -translate-y-1/2 text-gray-400 hover:text-gray-600",
                        onclick: handle_clear,
                        "×"
                    }
                }
            }

            // Loading indicator
            if search_loading {
                div { class: "absolute top-full left-0 right-0 bg-white border border-t-0 rounded-b-md p-2 z-10",
                    div { class: "text-sm text-gray-500 text-center",
                        {i18n.t(Key::Loading)}
                    }
                }
            }

            // Error message
            if let Some(error_msg) = error {
                div { class: "absolute top-full left-0 right-0 bg-white border border-t-0 rounded-b-md p-2 z-10",
                    div { class: "text-sm text-red-600",
                        {error_msg.clone()}
                    }
                }
            }

            // Search results
            if show_results() && !search_loading && error.is_none() {
                div { class: "absolute top-full left-0 right-0 bg-white border border-t-0 rounded-b-md max-h-64 overflow-y-auto z-10 shadow-lg",
                    if search_results.is_empty() {
                        div { class: "p-3 text-sm text-gray-500 text-center",
                            {i18n.t(Key::NoDataFound)}
                        }
                    } else {
                        for (idx, product) in search_results.iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "p-3 hover:bg-gray-50 cursor-pointer border-b last:border-b-0",
                                onclick: {
                                    let product = product.clone();
                                    move |_| handle_product_select(product.clone())
                                },
                                
                                div { class: "font-medium text-sm",
                                    {product.name.clone()}
                                }
                                div { class: "text-xs text-gray-500",
                                    "EAN: {product.ean.clone()}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}