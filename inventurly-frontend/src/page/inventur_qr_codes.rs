use crate::api;
use crate::component::{QRCode, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use rest_types::{InventurTO, RackTO};
use uuid::Uuid;

#[component]
pub fn InventurQRCodes(id: String) -> Element {
    let i18n = use_i18n();

    let inventur_id = Uuid::parse_str(&id).ok();
    let mut inventur = use_signal(|| None::<InventurTO>);
    let mut racks = use_signal(|| Vec::<RackTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    // Load inventur and racks
    use_effect(move || {
        if let Some(inv_id) = inventur_id {
            spawn(async move {
                let config = CONFIG.read().clone();

                // Load inventur details (includes token)
                match api::get_inventur(&config, inv_id).await {
                    Ok(inv_data) => {
                        if inv_data.token.is_none() {
                            error.set(Some("Inventur has no token".to_string()));
                            loading.set(false);
                            return;
                        }
                        inventur.set(Some(inv_data));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load inventur: {}", e)));
                        loading.set(false);
                        return;
                    }
                }

                // Load racks
                match api::get_racks(&config).await {
                    Ok(r) => {
                        racks.set(r);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load racks: {}", e)));
                    }
                }

                loading.set(false);
            });
        }
    });

    // Get base URL for QR codes
    let base_url = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());

    let handle_print = move |_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    };

    if *loading.read() {
        return rsx! {
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 flex items-center justify-center",
                    div { class: "text-xl", {i18n.t(Key::Loading)} }
                }
            }
        };
    }

    if let Some(err) = error.read().as_ref() {
        return rsx! {
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                        "{err}"
                    }
                }
            }
        };
    }

    let inventur_name = inventur
        .read()
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_else(|| "Unknown Inventur".to_string());

    let login_url = inventur
        .read()
        .as_ref()
        .and_then(|i| i.token.as_ref())
        .map(|t| format!("{}/login/{}", base_url, t))
        .unwrap_or_default();

    rsx! {
        div { class: "flex flex-col min-h-screen print:min-h-0",
            div { class: "print:hidden",
                TopBar {}
            }

            // Print button (hidden when printing)
            div { class: "container mx-auto px-4 py-4 print:hidden",
                button {
                    class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition font-semibold",
                    onclick: handle_print,
                    {i18n.t(Key::PrintQRCodes)}
                }
            }

            // QR Code Pages
            div { class: "container mx-auto px-4 print:p-0",
                // Login QR Code Page
                div { class: "qr-page flex flex-col items-center justify-center min-h-screen print:min-h-0 print:h-screen print:break-after-page bg-white",
                    div { class: "text-center",
                        h1 { class: "text-4xl font-bold mb-4", "{inventur_name}" }
                        h2 { class: "text-2xl mb-8 text-gray-600",
                            {i18n.t(Key::ScanToLogin)}
                        }
                        div { class: "flex justify-center mb-8",
                            QRCode { data: login_url.clone(), size: 400 }
                        }
                        p { class: "text-lg text-gray-500",
                            {i18n.t(Key::LoginQRCode)}
                        }
                    }
                }

                // Rack QR Code Pages
                for rack in racks.read().iter() {
                    {
                        let rack_id = rack.id.unwrap().to_string();
                        let rack_name = rack.name.clone();
                        let rack_url = format!(
                            "{}/inventurs/{}/rack/{}/measure",
                            base_url,
                            id,
                            rack_id
                        );

                        rsx! {
                            div {
                                key: "{rack_id}",
                                class: "qr-page flex flex-col items-center justify-center min-h-screen print:min-h-0 print:h-screen print:break-after-page bg-white",
                                div { class: "text-center",
                                    h1 { class: "text-4xl font-bold mb-4", "{inventur_name}" }
                                    h2 { class: "text-2xl mb-2 text-gray-600",
                                        {i18n.t(Key::Rack)}
                                    }
                                    h3 { class: "text-3xl font-semibold mb-8", "{rack_name}" }
                                    div { class: "flex justify-center mb-8",
                                        QRCode { data: rack_url, size: 400 }
                                    }
                                    p { class: "text-lg text-gray-500",
                                        {i18n.t(Key::ScanToMeasure)}
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
