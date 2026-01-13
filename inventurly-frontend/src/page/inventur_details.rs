use crate::api;
use crate::component::{InventurForm, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::auth::AUTH;
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use rest_types::InventurTO;
use uuid::Uuid;

// Note: Route::InventurQRCodes is now available for QR code navigation

#[component]
pub fn InventurDetails(id: String) -> Element {
    let i18n = use_i18n();

    // Parse the UUID or treat "new" as None
    let inventur_id = if id == "new" {
        None
    } else {
        Uuid::parse_str(&id).ok()
    };

    let nav = navigator();
    let mut inventur = use_signal(|| None::<InventurTO>);

    // Load inventur to check status
    use_effect(move || {
        if let Some(id) = inventur_id {
            spawn(async move {
                let config = CONFIG.read().clone();
                if let Ok(inventur_data) = api::get_inventur(&config, id).await {
                    inventur.set(Some(inventur_data));
                }
            });
        }
    });

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                div { class: "flex justify-between items-center mb-6",
                    h1 { class: "text-3xl font-bold",
                        if inventur_id.is_some() {
                            {i18n.t(Key::EditInventur)}
                        } else {
                            {i18n.t(Key::CreateInventur)}
                        }
                    }
                    if inventur_id.is_some() {
                        if let Some(inv) = inventur.read().as_ref() {
                            div { class: "flex gap-2",
                                {
                                    let auth = AUTH.read();
                                    let can_edit = auth.auth_info.as_ref()
                                        .map(|a| a.can_edit_inventur(&inv.status))
                                        .unwrap_or(false);
                                    if can_edit {
                                        rsx! {
                                            button {
                                                class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors",
                                                onclick: {
                                                    let inventur_id_str = id.clone();
                                                    move |_| {
                                                        nav.push(Route::InventurRackSelection { id: inventur_id_str.clone() });
                                                    }
                                                },
                                                {i18n.t(Key::MeasureByRack)}
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            button {
                                                class: "px-4 py-2 bg-gray-400 text-white rounded cursor-not-allowed",
                                                disabled: true,
                                                title: "Inventur must be active to measure",
                                                {i18n.t(Key::MeasureByRack)}
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors",
                                    onclick: {
                                        let inventur_id_str = id.clone();
                                        move |_| {
                                            nav.push(Route::InventurQRCodes { id: inventur_id_str.clone() });
                                        }
                                    },
                                    {i18n.t(Key::PrintQRCodes)}
                                }
                            }
                        }
                    }
                }
                InventurForm { inventur_id }
            }
        }
    }
}
