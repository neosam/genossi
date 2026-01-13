use crate::state::CustomEntry;
use dioxus::prelude::*;

pub static CUSTOM_ENTRIES: GlobalSignal<CustomEntry> = GlobalSignal::new(CustomEntry::default);
