use crate::router::Route;
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let nav = navigator();

    use_effect(move || {
        nav.replace(Route::Members {});
    });

    rsx! {
        div {}
    }
}
