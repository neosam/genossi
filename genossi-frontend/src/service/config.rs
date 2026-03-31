use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::{service::error::ERROR_STORE, state::Config};

use super::auth;

pub async fn load_config() {
    let config = crate::api::fetch_config().await;
    match config {
        Ok(config) => {
            *CONFIG.write() = config;
        }
        Err(err) => {
            ERROR_STORE.write().set_error(err.to_string());
        }
    }
    auth::load_auth_info().await;
}

pub static CONFIG: GlobalSignal<Config> = Signal::global(|| Config::default());

#[allow(dead_code)]
pub enum ConfigAction {
    LoadConfig,
}

pub async fn config_service(mut rx: UnboundedReceiver<ConfigAction>) {
    load_config().await;

    while let Some(action) = rx.next().await {
        match action {
            ConfigAction::LoadConfig => {
                load_config().await;
            }
        }
    }
}
