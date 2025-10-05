use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::{
    api,
    service::error::ERROR_STORE,
    state::Config,
};

use super::{auth, product::ProductService};

pub async fn load_config() {
    let config = api::load_config().await;
    match config {
        Ok(config) => {
            *CONFIG.write() = config;
        }
        Err(err) => {
            ERROR_STORE.write().set_error(err.to_string());
        }
    }
    *CONFIG.write() = api::load_config().await.unwrap();
    auth::load_auth_info().await;
}

// Config service
pub static CONFIG: GlobalSignal<Config> = Signal::global(|| Config::default());
#[allow(dead_code)]
pub enum ConfigAction {
    LoadConfig,
}
pub async fn config_service(mut rx: UnboundedReceiver<ConfigAction>) {
    load_config().await;
    
    // Send LoadProducts event after config is loaded
    let product_service = use_coroutine_handle::<ProductService>();
    product_service.send(ProductService::LoadProducts);

    while let Some(action) = rx.next().await {
        match action {
            ConfigAction::LoadConfig => {
                load_config().await;
                // Send LoadProducts event after reloading config too
                product_service.send(ProductService::LoadProducts);
            }
        }
    }
}
