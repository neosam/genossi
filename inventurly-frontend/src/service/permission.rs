use dioxus::prelude::*;
use rest_types::PermissionTO;
use uuid::Uuid;
use crate::api;
use crate::service::config::CONFIG;
use crate::state::Permission;

pub static PERMISSIONS: GlobalSignal<Permission> = GlobalSignal::new(Permission::default);

#[derive(Debug)]
pub enum PermissionService {
    LoadPermissions,
    GetPermission(Uuid),
    CreatePermission(PermissionTO),
    UpdatePermission(PermissionTO),
    DeletePermission(Uuid),
}

pub fn permission_service() {
    spawn(async move {
        // Initialize permissions loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            PERMISSIONS.write().loading = true;
            match api::get_permissions(&config).await {
                Ok(permissions) => {
                    PERMISSIONS.write().items = permissions;
                    PERMISSIONS.write().error = None;
                }
                Err(e) => {
                    PERMISSIONS.write().error = Some(format!("Failed to load permissions: {}", e));
                }
            }
            PERMISSIONS.write().loading = false;
        }
    });
}