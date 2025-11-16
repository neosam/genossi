use dioxus::prelude::*;
use tracing::info;

const TARA_STORAGE_KEY: &str = "inventurly_custom_tara_grams";

#[derive(Clone, Default)]
pub struct TaraStore {
    /// Custom tara weight in grams (e.g., body weight)
    pub tara_grams: i64,
}

pub static TARA: GlobalSignal<TaraStore> = Signal::global(|| {
    // Load from localStorage on initialization
    let tara = load_tara_from_storage();
    TaraStore { tara_grams: tara }
});

/// Load tara weight from browser localStorage
fn load_tara_from_storage() -> i64 {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item(TARA_STORAGE_KEY) {
                if let Ok(grams) = value.parse::<i64>() {
                    info!("Loaded custom tara from localStorage: {}g", grams);
                    return grams;
                }
            }
        }
    }
    0
}

/// Save tara weight to browser localStorage
pub fn save_tara_to_storage(grams: i64) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(TARA_STORAGE_KEY, &grams.to_string());
            info!("Saved custom tara to localStorage: {}g", grams);
        }
    }
}

/// Clear tara weight from browser localStorage
pub fn clear_tara_from_storage() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(TARA_STORAGE_KEY);
            info!("Cleared custom tara from localStorage");
        }
    }
}

/// Set the custom tara weight and save to localStorage
pub fn set_tara(grams: i64) {
    save_tara_to_storage(grams);
    *TARA.write() = TaraStore { tara_grams: grams };
}

/// Clear the custom tara weight
pub fn clear_tara() {
    clear_tara_from_storage();
    *TARA.write() = TaraStore { tara_grams: 0 };
}

/// Get the current tara weight in grams
pub fn get_tara_grams() -> i64 {
    TARA.read().tara_grams
}

/// Apply tara subtraction to a gross weight
/// Returns the net weight (gross - tara)
pub fn apply_tara(gross_weight_grams: i64) -> i64 {
    let tara = get_tara_grams();
    gross_weight_grams - tara
}

/// Check if a custom tara is set
pub fn has_tara() -> bool {
    get_tara_grams() > 0
}
