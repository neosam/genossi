use dioxus::prelude::*;

use crate::i18n::{I18n, Locale};

pub static I18N: GlobalSignal<I18n> = GlobalSignal::new(|| I18n::new(Locale::En));

pub async fn i18n_service(_rx: UnboundedReceiver<()>) {
    // Simple i18n service - just maintains the global state
    // Language is fixed to English for now
}