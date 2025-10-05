use dioxus::prelude::*;
use uuid::Uuid;
use rest_types::RackTO;
use crate::i18n::{use_i18n, Key};
use crate::service::product_rack::add_product_to_rack_action;
use crate::service::config::CONFIG;
use crate::component::SearchableProductSelector;
use crate::api;

#[component]
pub fn ProductRackForm(
    product_id: Option<Uuid>,
    rack_id: Option<Uuid>,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    
    let mut selected_product = use_signal(|| product_id);
    let mut selected_rack = use_signal(|| rack_id);
    let racks = use_signal(|| Vec::<RackTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let saving = use_signal(|| false);

    // Load racks on mount (we still need this for rack selection)
    use_effect(move || {
        spawn({
            let mut racks = racks.clone();
            let mut loading = loading.clone();
            let mut error = error.clone();
            
            async move {
                loading.set(true);
                let config = CONFIG.read().clone();
                
                match api::get_racks(&config).await {
                    Ok(rack_list) => {
                        racks.set(rack_list);
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load racks: {}", e)));
                    }
                }
                
                loading.set(false);
            }
        });
    });

    let handle_product_selected = move |product_id: Option<Uuid>| {
        selected_product.set(product_id);
    };

    let handle_save = move |_| {
        if let (Some(prod_id), Some(rack_id)) = (selected_product(), selected_rack()) {
            spawn({
                let mut saving = saving.clone();
                let mut error = error.clone();
                let on_saved = on_saved.clone();
                
                async move {
                    saving.set(true);
                    error.set(None);
                    
                    match add_product_to_rack_action(prod_id, rack_id).await {
                        Ok(()) => {
                            on_saved.call(());
                        }
                        Err(e) => {
                            error.set(Some(e));
                        }
                    }
                    
                    saving.set(false);
                }
            });
        }
    };

    let is_valid = selected_product().is_some() && selected_rack().is_some();

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-bold",
                {i18n.t(Key::AddProductToRack)}
            }
            
            if loading() {
                div { class: "text-center py-4",
                    {i18n.t(Key::Loading)}
                }
            } else {
                div { class: "space-y-4",
                    
                    if let Some(error_msg) = error() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                            {error_msg}
                        }
                    }

                    // Product selection with searchable selector
                    div {
                        label { class: "block text-sm font-medium mb-1",
                            {i18n.t(Key::SelectProduct)}
                        }
                        SearchableProductSelector {
                            selected_product_id: selected_product(),
                            disabled: product_id.is_some() || saving(),
                            on_product_selected: handle_product_selected
                        }
                    }

                    // Rack selection (keep as dropdown since fewer items)
                    div {
                        label { class: "block text-sm font-medium mb-1",
                            {i18n.t(Key::SelectRack)}
                        }
                        select {
                            class: "w-full px-3 py-2 border rounded-md",
                            disabled: rack_id.is_some() || saving(),
                            value: selected_rack().map(|id| id.to_string()).unwrap_or_default(),
                            onchange: move |event| {
                                if let Ok(uuid) = Uuid::parse_str(&event.value()) {
                                    selected_rack.set(Some(uuid));
                                } else {
                                    selected_rack.set(None);
                                }
                            },
                            
                            option { value: "", "-- {i18n.t(Key::SelectRack)} --" }
                            
                            for rack in racks().iter() {
                                option { 
                                    value: rack.id.unwrap().to_string(),
                                    selected: Some(rack.id.unwrap()) == selected_rack(),
                                    "{rack.name} - {rack.description}"
                                }
                            }
                        }
                    }

                    // Action buttons
                    div { class: "flex space-x-2 pt-4",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: !is_valid || saving(),
                            onclick: handle_save,
                            
                            if saving() {
                                {i18n.t(Key::Loading)}
                            } else {
                                {i18n.t(Key::Save)}
                            }
                        }
                        
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600",
                            disabled: saving(),
                            onclick: move |_| on_cancel.call(()),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }
    }
}