//! Quick 260602-sgp: Bulk-Download bereits persistierter RepaymentLetter-PDFs.
//!
//! Wiederverwendbare Component fuer die RepaymentPhase-Detail-Page. Zwei
//! Format-Optionen (ZIP / Bundle-PDF) als zwei Buttons innerhalb derselben
//! Component, damit die Page KEIN inline-RSX-Markup fuer die Download-Logik
//! braucht (Component-First-Prinzip aus CLAUDE.md +
//! Memory `feedback_component_first.md`).
//!
//! Memory `feedback_dioxus_button_type.md`: beide Buttons setzen
//! `r#type: "button"` + nutzen `onclick`-Handler (NICHT form-onsubmit), damit
//! der Page-Reload-Bug nicht auftritt.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, AppError};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

/// Props fuer den Component.
///
/// `on_toast` ist der Callback fuer Statusmeldungen (Erfolg + Fehler). Die
/// Page rendert die Toast-Container und gibt einen Handler ueber, der den
/// Toast-Signal-Stack des Parent-Components mutiert.
#[derive(Props, PartialEq, Clone)]
pub struct RepaymentLetterDownloadButtonProps {
    /// UUID der RepaymentPhase, deren persistierte Letters geladen werden.
    pub phase_id: Uuid,
    /// fiscal_year — derzeit nicht direkt vom Component genutzt, aber als
    /// API-Vertrag mitgegeben, damit der Page-Mount es nicht zerlegen muss
    /// und kuenftige Filename-Anpassungen (z.B. eigene Filename-Tooltip-Texte)
    /// hier zentral landen.
    pub fiscal_year: i32,
    /// Callback fuer Toast-Messages (Erfolg + Fehler).
    pub on_toast: EventHandler<String>,
}

/// Bulk-Download-Buttons fuer ZIP- und Bundle-PDF-Format.
///
/// Klick triggert `api::download_repayment_letters`, packt das resultierende
/// Blob in ein verstecktes `<a download>`-Element, ruft `.click()`, und
/// gibt die Object-URL via `Url::revoke_object_url` wieder frei.
#[component]
pub fn RepaymentLetterDownloadButton(props: RepaymentLetterDownloadButtonProps) -> Element {
    let i18n = use_i18n();
    let phase_id = props.phase_id;
    let _fiscal_year = props.fiscal_year;
    let on_toast = props.on_toast;

    // i18n-Strings am Top-Level resolven — use_i18n() ist ein Hook und darf
    // NICHT in einer async spawn-Closure laufen (Pattern aus Phase-13-Page).
    let success_singular = i18n
        .t(Key::RepaymentLetterDownloadToastSingular)
        .to_string();
    let success_plural = i18n.t(Key::RepaymentLetterDownloadToastPlural).to_string();
    let skipped_template = i18n.t(Key::RepaymentLetterDownloadToastSkipped).to_string();
    let failure_template = i18n.t(Key::RepaymentLetterDownloadToastFailure).to_string();

    // i18n-Klone fuer beide Button-Closures — jeder Button-onclick muss
    // unabhaengig spawn-able sein, daher pro Button eine eigene Clone-Familie.
    let s1 = success_singular.clone();
    let p1 = success_plural.clone();
    let k1 = skipped_template.clone();
    let f1 = failure_template.clone();
    let toast_zip = on_toast;
    let zip_click = move |_| {
        let s1 = s1.clone();
        let p1 = p1.clone();
        let k1 = k1.clone();
        let f1 = f1.clone();
        let toast_zip = toast_zip;
        spawn(async move {
            handle_download("zip", phase_id, s1, p1, k1, f1, toast_zip).await;
        });
    };

    let s2 = success_singular;
    let p2 = success_plural;
    let k2 = skipped_template;
    let f2 = failure_template;
    let toast_pdf = on_toast;
    let pdf_click = move |_| {
        let s2 = s2.clone();
        let p2 = p2.clone();
        let k2 = k2.clone();
        let f2 = f2.clone();
        let toast_pdf = toast_pdf;
        spawn(async move {
            handle_download("pdf", phase_id, s2, p2, k2, f2, toast_pdf).await;
        });
    };

    rsx! {
        div { class: "flex flex-wrap gap-2",
            button {
                class: "px-3 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                r#type: "button",
                onclick: zip_click,
                {i18n.t(Key::RepaymentLetterDownloadZipButton)}
            }
            button {
                class: "px-3 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                r#type: "button",
                onclick: pdf_click,
                {i18n.t(Key::RepaymentLetterDownloadPdfButton)}
            }
        }
    }
}

/// Helper: shared download-Routine fuer beide Format-Buttons.
async fn handle_download(
    format: &'static str,
    phase_id: Uuid,
    success_singular: String,
    success_plural: String,
    skipped_template: String,
    failure_template: String,
    on_toast: EventHandler<String>,
) {
    let cfg = CONFIG.read().clone();
    match api::download_repayment_letters(&cfg, phase_id, format).await {
        Ok(result) => {
            // Browser-Save via <a download>-Click + revoke_object_url.
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Ok(elem) = document.create_element("a") {
                        let _ = elem.set_attribute("href", &result.blob_url);
                        let _ = elem.set_attribute("download", &result.filename);
                        use wasm_bindgen::JsCast;
                        if let Ok(html_elem) = elem.dyn_into::<web_sys::HtmlElement>() {
                            html_elem.click();
                        }
                    }
                    // T-06-16 mitigation: release blob URL nach click.
                    let _ = web_sys::Url::revoke_object_url(&result.blob_url);
                }
            }
            let count_str = result.document_count.to_string();
            let base_msg = if result.document_count == 1 {
                success_singular
            } else {
                success_plural.replace("{count}", &count_str)
            };
            let msg = if result.skipped_count > 0 {
                let skipped_str = result.skipped_count.to_string();
                let suffix = skipped_template.replace("{skipped}", &skipped_str);
                format!("{} {}", base_msg, suffix)
            } else {
                base_msg
            };
            on_toast.call(msg);
        }
        Err(e) => {
            on_toast.call(error_message(&failure_template, &e));
        }
    }
}

/// Helper: Fehlermeldung mit Backend-Detail kombinieren — wenn das Template
/// einen `{error}`-Placeholder enthaelt wird die Backend-Message eingesetzt,
/// sonst wird sie als Suffix angehaengt.
fn error_message(template: &str, err: &AppError) -> String {
    if template.contains("{error}") {
        template.replace("{error}", &err.message)
    } else {
        format!("{}: {}", template, err.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_message_substitutes_placeholder() {
        let err = AppError::new(Some(500), "boom", None);
        let msg = error_message("Fehler: {error}", &err);
        assert_eq!(msg, "Fehler: boom");
    }

    #[test]
    fn test_error_message_falls_back_to_suffix() {
        let err = AppError::new(Some(500), "boom", None);
        let msg = error_message("Download fehlgeschlagen", &err);
        assert_eq!(msg, "Download fehlgeschlagen: boom");
    }
}
