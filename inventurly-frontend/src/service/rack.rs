use dioxus::prelude::*;
use rest_types::RackTO;
use uuid::Uuid;
use crate::api;
use crate::service::config::CONFIG;
use crate::state::Rack;

pub static RACKS: GlobalSignal<Rack> = GlobalSignal::new(Rack::default);

#[derive(Debug)]
pub enum RackService {
    LoadRacks,
    GetRack(Uuid),
    CreateRack(RackTO),
    UpdateRack(RackTO),
    DeleteRack(Uuid),
}

pub fn rack_service() {
    spawn(async move {
        // Initialize racks loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            RACKS.write().loading = true;
            match api::get_racks(&config).await {
                Ok(racks) => {
                    RACKS.write().items = racks;
                    RACKS.write().error = None;
                }
                Err(e) => {
                    RACKS.write().error = Some(format!("Failed to load racks: {}", e));
                }
            }
            RACKS.write().loading = false;
        }
    });
}