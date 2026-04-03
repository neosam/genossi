use dioxus::prelude::*;

use crate::api;
use crate::service::config::CONFIG;
use crate::state::MemberState;

pub static MEMBERS: GlobalSignal<MemberState> = Signal::global(MemberState::default);

pub async fn refresh_members() {
    let config = CONFIG.read().clone();
    if !config.backend.is_empty() {
        MEMBERS.write().loading = true;
        match api::get_members(&config).await {
            Ok(members) => {
                MEMBERS.write().items = members;
                MEMBERS.write().error = None;
            }
            Err(e) => {
                MEMBERS.write().error = Some(format!("Failed to load members: {}", e));
            }
        }
        MEMBERS.write().loading = false;
    }
}
