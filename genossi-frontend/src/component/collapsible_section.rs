use dioxus::prelude::*;

/// Flip the open/closed state. Pure logic extracted for unit tests,
/// since Dioxus signals require a running runtime that is not
/// available in plain `#[test]` contexts.
pub fn toggle_open(current: bool) -> bool {
    !current
}

/// Pick the icon glyph that visualises the current state.
pub fn arrow_icon(is_open: bool) -> &'static str {
    if is_open {
        "\u{25B2}"
    } else {
        "\u{25BC}"
    }
}

/// Wiederverwendbare, zusammenklappbare Sektion mit Header und Inhalt.
///
/// Der Header rendert als `<button>` (Tastatur-bedienbar), Klick schaltet den
/// Zustand um. Der Inhalt wird nur gerendert, wenn die Sektion offen ist.
#[component]
pub fn CollapsibleSection(
    title: String,
    #[props(default = false)] default_open: bool,
    children: Element,
) -> Element {
    let mut is_open = use_signal(|| default_open);

    rsx! {
        div { class: "bg-white rounded-lg shadow mb-6",
            button {
                r#type: "button",
                class: "w-full flex items-center justify-between p-6 text-left",
                onclick: move |_| {
                    let current = *is_open.read();
                    is_open.set(toggle_open(current));
                },
                h2 { class: "text-xl font-semibold", "{title}" }
                span { class: "text-gray-400 text-xl",
                    "{arrow_icon(*is_open.read())}"
                }
            }

            if *is_open.read() {
                div { class: "px-6 pb-6", { children } }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_from_closed_opens() {
        assert!(toggle_open(false));
    }

    #[test]
    fn toggle_from_open_closes() {
        assert!(!toggle_open(true));
    }

    #[test]
    fn toggle_twice_returns_to_original() {
        // Default eingeklappt → Klick öffnet → erneuter Klick schließt
        let start = false;
        let after_first_click = toggle_open(start);
        let after_second_click = toggle_open(after_first_click);
        assert!(after_first_click, "nach erstem Klick ist offen");
        assert!(!after_second_click, "nach zweitem Klick wieder zu");
    }

    #[test]
    fn arrow_icon_closed() {
        // Pfeil zeigt nach unten, wenn eingeklappt
        assert_eq!(arrow_icon(false), "\u{25BC}");
    }

    #[test]
    fn arrow_icon_open() {
        // Pfeil zeigt nach oben, wenn aufgeklappt
        assert_eq!(arrow_icon(true), "\u{25B2}");
    }

    #[test]
    fn default_open_true_starts_open() {
        // Der Initialzustand der Signal-Erzeugung ist `default_open` selbst.
        // Wir prüfen dass die Arrow-Darstellung beim Start mit `true`
        // den geöffneten Zustand widerspiegelt.
        let default_open = true;
        assert_eq!(arrow_icon(default_open), "\u{25B2}");
    }

    #[test]
    fn default_open_false_starts_closed() {
        let default_open = false;
        assert_eq!(arrow_icon(default_open), "\u{25BC}");
    }
}
