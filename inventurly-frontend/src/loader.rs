use dioxus::prelude::*;

#[allow(dead_code)]
pub fn use_loader<T, F>(_loader_fn: F) -> Signal<Option<T>>
where
    T: 'static,
    F: Fn() -> T + 'static,
{
    use_signal(|| None)
}