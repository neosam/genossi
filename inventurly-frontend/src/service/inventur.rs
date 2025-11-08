use crate::api;
use crate::service::config::CONFIG;
use crate::state::{Inventur, InventurMeasurement};
use dioxus::prelude::*;
use rest_types::{InventurTO, InventurMeasurementTO};
use uuid::Uuid;

pub static INVENTURS: GlobalSignal<Inventur> = GlobalSignal::new(Inventur::default);
pub static MEASUREMENTS: GlobalSignal<InventurMeasurement> = GlobalSignal::new(InventurMeasurement::default);

#[derive(Debug)]
#[allow(dead_code)]
pub enum InventurService {
    LoadInventurs,
    GetInventur(Uuid),
    CreateInventur(InventurTO),
    UpdateInventur(InventurTO),
    DeleteInventur(Uuid),
    ChangeStatus(Uuid, String),
    LoadMeasurements(Uuid),
    CreateMeasurement(InventurMeasurementTO),
    UpdateMeasurement(InventurMeasurementTO),
    DeleteMeasurement(Uuid),
}

pub fn inventur_service() {
    spawn(async move {
        // Initialize inventurs loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            INVENTURS.write().loading = true;
            match api::get_inventurs(&config).await {
                Ok(inventurs) => {
                    INVENTURS.write().items = inventurs;
                    INVENTURS.write().error = None;
                }
                Err(e) => {
                    INVENTURS.write().error = Some(format!("Failed to load inventurs: {}", e));
                }
            }
            INVENTURS.write().loading = false;
        }
    });
}
