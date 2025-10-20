use crate::api;
use crate::service::config::CONFIG;
use crate::state::Container;
use dioxus::prelude::*;
use rest_types::ContainerTO;
use uuid::Uuid;

pub static CONTAINERS: GlobalSignal<Container> = GlobalSignal::new(Container::default);

#[derive(Debug)]
#[allow(dead_code)]
pub enum ContainerService {
    LoadContainers,
    GetContainer(Uuid),
    CreateContainer(ContainerTO),
    UpdateContainer(ContainerTO),
    DeleteContainer(Uuid),
}

pub fn container_service() {
    spawn(async move {
        // Initialize containers loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            CONTAINERS.write().loading = true;
            match api::get_containers(&config).await {
                Ok(containers) => {
                    CONTAINERS.write().items = containers;
                    CONTAINERS.write().error = None;
                }
                Err(e) => {
                    CONTAINERS.write().error = Some(format!("Failed to load containers: {}", e));
                }
            }
            CONTAINERS.write().loading = false;
        }
    });
}
