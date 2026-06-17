use dioxus::prelude::*;
use rest_types::PostalStatusTO;

use crate::i18n::{use_i18n, Key};

/// Quick 260625-e14: wiederverwendbarer Select für den postalischen Status eines
/// Mitglieds. Component-First (CLAUDE.md): kein inline-RSX-Duplikat. Spiegelt das
/// Pattern des bestehenden MemberStatus-Selects auf der Member-Detail-Seite,
/// nutzt aber i18n-Keys für die Option-Beschriftung (de/en).
#[component]
pub fn MemberPostalStatusSelect(
    value: PostalStatusTO,
    onchange: EventHandler<PostalStatusTO>,
    #[props(default)] label: Option<String>,
) -> Element {
    let i18n = use_i18n();
    let label_text = label.unwrap_or_else(|| i18n.t(Key::MemberPostalStatus).to_string());
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1",
                {label_text}
            }
            select {
                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                value: value.as_str(),
                onchange: move |e| {
                    if let Some(s) = PostalStatusTO::from_str(&e.value()) {
                        onchange.call(s);
                    }
                },
                for s in PostalStatusTO::all() {
                    option {
                        value: "{s.as_str()}",
                        selected: value == *s,
                        {postal_status_label(&i18n, s)}
                    }
                }
            }
        }
    }
}

fn postal_status_label(i18n: &crate::i18n::I18n, status: &PostalStatusTO) -> String {
    match status {
        PostalStatusTO::Erreichbar => i18n.t(Key::MemberPostalStatusErreichbar).to_string(),
        PostalStatusTO::Unzustellbar => i18n.t(Key::MemberPostalStatusUnzustellbar).to_string(),
    }
}
