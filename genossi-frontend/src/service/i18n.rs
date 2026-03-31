use dioxus::prelude::*;

pub async fn i18n_service(_rx: UnboundedReceiver<()>) {
    // Simple i18n service - just maintains the global state
    // Language is automatically detected from browser preferences
}
