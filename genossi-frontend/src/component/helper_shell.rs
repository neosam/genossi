//! HelperShell Layout (Phase 4 Plan 05) — minimaler Wrapper für `/helper*`-Routes (D-07).
//!
//! Hard rule (T-04-24): KEIN `<TopBar />`, KEIN `<Footer />`. Helfer dürfen keine
//! Vorstand-Navigation (Members, Audit, Mail, ...) sehen — Datenschutz + Verwirrungs-Risiko.
//! Die Hard-Rule wird in Plan 10 per `grep -E "TopBar|Footer"` verifiziert.
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

#[cfg(test)]
mod tests {
    //! Hard-rule tests: HelperShell darf keinerlei Referenz auf TopBar oder Footer haben.
    //! Der Source-File-Inhalt wird statisch eingebunden und durchsucht — wenn jemand
    //! versehentlich `<TopBar />` oder `<Footer />` einfügt, schlägt der Test fehl.

    const SOURCE: &str = include_str!("helper_shell.rs");

    #[test]
    fn source_does_not_reference_topbar() {
        // Doc-comment darf "TopBar" enthalten (Erklärung der Hard-Rule); RSX-Code nicht.
        // Wir schließen Doc-Kommentar-Zeilen aus.
        for (idx, line) in SOURCE.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            assert!(
                !line.contains("TopBar"),
                "HelperShell source line {} references TopBar (D-07 violation): {line}",
                idx + 1
            );
        }
    }

    #[test]
    fn source_does_not_reference_footer_component() {
        // `Footer` als Component-Referenz wäre `Footer {}` oder `<Footer ...>`.
        // Wir prüfen auf das Wort als RSX-Tag (außerhalb von Kommentaren).
        for (idx, line) in SOURCE.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            assert!(
                !line.contains("Footer"),
                "HelperShell source line {} references Footer (D-07 violation): {line}",
                idx + 1
            );
        }
    }
}
