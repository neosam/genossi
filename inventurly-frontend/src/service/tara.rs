use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

const TARA_STORAGE_KEY: &str = "inventurly_custom_tara_grams";
const WEIGHT_UNIT_KEY: &str = "inventurly_preferred_weight_unit";
const LAST_CONTAINER_KEY: &str = "inventurly_last_container_id";

/// Weight unit for measurements
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WeightUnit {
    Kilogram,
    Gram,
    // Future units can be added here (e.g., Pound, Ounce)
}

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

/// Load preferred weight unit from browser localStorage
pub fn get_preferred_weight_unit() -> WeightUnit {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item(WEIGHT_UNIT_KEY) {
                if let Ok(unit) = serde_json::from_str::<WeightUnit>(&value) {
                    info!("Loaded preferred weight unit from localStorage: {:?}", unit);
                    return unit;
                }
            }
        }
    }
    WeightUnit::Kilogram // Default to kg
}

/// Save preferred weight unit to browser localStorage
pub fn set_preferred_weight_unit(unit: WeightUnit) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(&unit) {
                let _ = storage.set_item(WEIGHT_UNIT_KEY, &json);
                info!("Saved preferred weight unit to localStorage: {:?}", unit);
            }
        }
    }
}

/// Load last used container ID from browser localStorage
pub fn get_last_container_id() -> Option<Uuid> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item(LAST_CONTAINER_KEY) {
                if let Ok(uuid) = Uuid::parse_str(&value) {
                    info!("Loaded last container ID from localStorage: {}", uuid);
                    return Some(uuid);
                }
            }
        }
    }
    None
}

/// Save last used container ID to browser localStorage
pub fn set_last_container_id(id: Uuid) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(LAST_CONTAINER_KEY, &id.to_string());
            info!("Saved last container ID to localStorage: {}", id);
        }
    }
}

/// Clear last used container ID from browser localStorage
pub fn clear_last_container_id() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(LAST_CONTAINER_KEY);
            info!("Cleared last container ID from localStorage");
        }
    }
}
