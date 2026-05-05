//! HelperShell Layout (Phase 4 Plan 05) — minimaler Wrapper für `/helper*`-Routes (D-07).
//!
//! Hard rule (T-04-24, Datenschutz): Diese Component rendert KEIN globales App-Chrome
//! (kein Top-Navigations-Element, kein Seiten-Fuß-Branding-Element). Helfer dürfen
//! keine Vorstand-Navigation sehen (Members, Audit, Mail, ...). Plan 10 verifiziert
//! diese Regel automatisch durch eine Source-Inspektion.
//!
//! W-07 / D-19: Helfer-View ist DACH-Deutsch — die Component forciert beim Mount
//! `Locale::De` über den globalen `I18N`-Signal, unabhängig vom Browser-Default.
//! Das ist auch dann konsistent, wenn der Browser auf Englisch steht (z.B. iPad mit
//! englischer System-Sprache, das im Vereinsheim genutzt wird).

use dioxus::prelude::*;

use crate::i18n::{use_i18n, I18n, Key, Locale, I18N};

#[component]
pub fn HelperShell(
    assembly_name: Option<String>,
    on_logout: EventHandler<()>,
    children: Element,
) -> Element {
    // W-07 / D-19: Locale auf De forcen. Wir machen das beim Mount via use_effect,
    // damit die Component nicht in einer Render-Schleife landet, falls I18N anderswo
    // gesetzt wird (use_effect feuert nur wenn Reactive-Reads sich ändern; hier
    // hängt es von keiner Signal-Read ab → läuft genau einmal beim Mount).
    use_effect(move || {
        *I18N.write() = I18n::new(Locale::De);
    });
    let i18n = use_i18n();
    let display_name = assembly_name.unwrap_or_else(|| "...".to_string());
    rsx! {
        div { class: "min-h-screen bg-gray-50 flex flex-col",
            header { class: "bg-white border-b border-gray-200 px-4 py-3 flex items-center justify-between print:hidden",
                h1 { class: "text-lg font-semibold truncate", "{display_name}" }
                button {
                    class: "text-sm text-gray-600 hover:text-gray-900 underline min-h-[44px]",
                    onclick: move |_| on_logout.call(()),
                    "{i18n.t(Key::HelperShellLogout)}"
                }
            }
            main { class: "flex-1 px-4 py-6 max-w-3xl mx-auto w-full",
                {children}
            }
        }
    }
}
