use crate::api;
use crate::service::config::CONFIG;
use crate::state::ContainerRack;
use dioxus::prelude::*;
use rest_types::ContainerRackTO;
use uuid::Uuid;

pub static CONTAINER_RACKS: GlobalSignal<ContainerRack> = GlobalSignal::new(ContainerRack::default);

#[derive(Debug)]
#[allow(dead_code)]
pub enum ContainerRackService {
    LoadContainerRacks,
    AddContainerToRack(Uuid, Uuid),
    RemoveContainerFromRack(Uuid, Uuid),
    GetRacksForContainer(Uuid),
    GetContainersInRack(Uuid),
}

pub fn container_rack_service() {
    spawn(async move {
        // Initialize container racks loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            CONTAINER_RACKS.write().loading = true;
            match api::get_all_container_rack_relationships(&config).await {
                Ok(container_racks) => {
                    CONTAINER_RACKS.write().items = container_racks;
                    CONTAINER_RACKS.write().error = None;
                }
                Err(e) => {
                    CONTAINER_RACKS.write().error =
                        Some(format!("Failed to load container-rack relationships: {}", e));
                }
            }
            CONTAINER_RACKS.write().loading = false;
        }
    });
}

pub async fn add_container_to_rack_action(container_id: Uuid, rack_id: Uuid) -> Result<(), String> {
    let config = CONFIG.read().clone();

    match api::add_container_to_rack(&config, container_id, rack_id).await {
        Ok(new_relationship) => {
            // Add to local state
            CONTAINER_RACKS.write().items.push(new_relationship);
            CONTAINER_RACKS.write().error = None;
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to add container to rack: {}", e);
            CONTAINER_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn remove_container_from_rack_action(
    container_id: Uuid,
    rack_id: Uuid,
) -> Result<(), String> {
    let config = CONFIG.read().clone();

    match api::remove_container_from_rack(&config, container_id, rack_id).await {
        Ok(()) => {
            // Remove from local state
            CONTAINER_RACKS
                .write()
                .items
                .retain(|item| !(item.container_id == container_id && item.rack_id == rack_id));
            CONTAINER_RACKS.write().error = None;
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to remove container from rack: {}", e);
            CONTAINER_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

#[allow(dead_code)]
pub async fn get_racks_for_container_action(container_id: Uuid) -> Result<Vec<ContainerRackTO>, String> {
    let config = CONFIG.read().clone();

    match api::get_racks_for_container(&config, container_id).await {
        Ok(racks) => {
            CONTAINER_RACKS.write().error = None;
            Ok(racks)
        }
        Err(e) => {
            let error_msg = format!("Failed to get racks for container: {}", e);
            CONTAINER_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn get_containers_in_rack_action(rack_id: Uuid) -> Result<Vec<ContainerRackTO>, String> {
    let config = CONFIG.read().clone();

    match api::get_containers_in_rack(&config, rack_id).await {
        Ok(containers) => {
            CONTAINER_RACKS.write().error = None;
            Ok(containers)
        }
        Err(e) => {
            let error_msg = format!("Failed to get containers in rack: {}", e);
            CONTAINER_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn set_container_position_action(
    container_id: Uuid,
    rack_id: Uuid,
    position: i32,
) -> Result<ContainerRackTO, String> {
    let config = CONFIG.read().clone();

    match api::set_container_position(&config, container_id, rack_id, position).await {
        Ok(updated_relationship) => {
            CONTAINER_RACKS.write().error = None;
            Ok(updated_relationship)
        }
        Err(e) => {
            let error_msg = format!("Failed to update container position: {}", e);
            CONTAINER_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn reorder_containers_in_rack_action(
    rack_id: Uuid,
    container_order: Vec<Uuid>,
) -> Result<Vec<ContainerRackTO>, String> {
    let config = CONFIG.read().clone();

    match api::reorder_containers_in_rack(&config, rack_id, container_order).await {
        Ok(updated_relationships) => {
            CONTAINER_RACKS.write().error = None;
            Ok(updated_relationships)
        }
        Err(e) => {
            let error_msg = format!("Failed to reorder containers: {}", e);
            CONTAINER_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}
