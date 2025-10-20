use crate::api;
use crate::component::{modal::Modal, ProductRackForm};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::product_rack::{get_racks_for_product_action, remove_product_from_rack_action};
use dioxus::prelude::*;
use rest_types::{ProductRackTO, RackTO};
use uuid::Uuid;

#[component]
pub fn ProductRacks(product_id: Uuid) -> Element {
    let i18n = use_i18n();

    let racks_for_product = use_signal(|| Vec::<ProductRackTO>::new());
    let racks_map = use_signal(|| std::collections::HashMap::<Uuid, RackTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let show_add_form = use_signal(|| false);

    // Load racks for product
    let load_racks = use_callback({
        let racks_for_product = racks_for_product.clone();
        let racks_map = racks_map.clone();
        let loading = loading.clone();
        let error = error.clone();

        move |_| {
            spawn({
                let racks_for_product = racks_for_product.clone();
                let racks_map = racks_map.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    error.set(None);

                    // Load racks for product and all racks
                    let config = CONFIG.read().clone();
                    let (product_racks_result, all_racks_result) = futures_util::join!(
                        get_racks_for_product_action(product_id),
                        api::get_racks(&config)
                    );

                    match product_racks_result {
                        Ok(product_racks) => {
                            let mut racks_for_product = racks_for_product.clone();
                            racks_for_product.set(product_racks);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load racks for product: {}", e)));
                            loading.set(false);
                            return;
                        }
                    }

                    match all_racks_result {
                        Ok(all_racks) => {
                            let mut map = std::collections::HashMap::new();
                            for rack in all_racks {
                                if let Some(id) = rack.id {
                                    map.insert(id, rack);
                                }
                            }
                            let mut racks_map = racks_map.clone();
                            racks_map.set(map);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load racks: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    // Load racks on mount
    use_effect(move || {
        load_racks.call(());
    });

    let handle_remove_from_rack = move |rack_id: Uuid| {
        spawn({
            let mut error = error.clone();
            let load_racks = load_racks.clone();

            async move {
                error.set(None);

                match remove_product_from_rack_action(product_id, rack_id).await {
                    Ok(()) => {
                        load_racks.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    let handle_rack_saved = move |_| {
        let mut show_add_form = show_add_form.clone();
        show_add_form.set(false);
        load_racks.call(());
    };

    rsx! {
        div { class: "space-y-4",
            div { class: "flex justify-between items-center",
                h3 { class: "text-lg font-semibold",
                    {i18n.t(Key::RacksForProduct)}
                }
                button {
                    class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600",
                    onclick: move |_| {
                        let mut show_add_form = show_add_form.clone();
                        show_add_form.set(true);
                    },
                    {i18n.t(Key::AddProductToRack)}
                }
            }

            if let Some(error_msg) = error() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                    {error_msg}
                }
            }

            if loading() {
                div { class: "text-center py-4",
                    {i18n.t(Key::Loading)}
                }
            } else if racks_for_product().is_empty() {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "min-w-full bg-white border border-gray-200",
                        thead { class: "bg-gray-50",
                            tr {
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::RackName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::RackDescription)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody { class: "bg-white divide-y divide-gray-200",
                            for (idx, product_rack) in racks_for_product().iter().enumerate() {
                                if let Some(rack) = racks_map().get(&product_rack.rack_id) {
                                    tr {
                                        key: "{idx}",
                                        class: "hover:bg-gray-50",
                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                                            {rack.name.clone()}
                                        }
                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                                            {rack.description.clone()}
                                        }
                                        td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium space-x-2",
                                            button {
                                                class: "text-red-600 hover:text-red-900",
                                                onclick: {
                                                    let rack_id = product_rack.rack_id;
                                                    move |_| handle_remove_from_rack(rack_id)
                                                },
                                                {i18n.t(Key::RemoveProductFromRack)}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Add to Rack Modal
            if show_add_form() {
                Modal {
                    ProductRackForm {
                        product_id: Some(product_id),
                        rack_id: None,
                        on_saved: handle_rack_saved,
                        on_cancel: move |_| {
                            let mut show_add_form = show_add_form.clone();
                            show_add_form.set(false);
                        },
                    }
                }
            }

        }
    }
}
