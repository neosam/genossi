use std::rc::Rc;

use rest_types::{MemberActionTO, MemberDocumentTO, MemberTO, MigrationStatusTO, UserTO, ValidationResultTO};
use tracing::info;
use uuid::Uuid;

use crate::state::{AuthInfo, Config};

// Config API
pub async fn fetch_config() -> Result<Config, reqwest::Error> {
    info!("Fetching config");
    let window = web_sys::window().unwrap();
    let origin = window.location().origin().unwrap();
    let url = format!("{}/assets/config.json", origin);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let config: Config = response.json().await?;
    info!("Config fetched: {:?}", config);
    Ok(config)
}

// Authentication API
pub async fn fetch_auth_info(backend_url: Rc<str>) -> Result<Option<AuthInfo>, reqwest::Error> {
    info!("Fetching auth info");
    let response = reqwest::get(format!("{}/api/auth/info", backend_url)).await?;
    if response.status() != 200 {
        return Ok(None);
    }
    let user: UserTO = response.json().await?;
    let auth_info = AuthInfo {
        user: user.username.into(),
        roles: user.roles.into_iter().map(|r| r.into()).collect(),
        privileges: user.privileges.into_iter().map(|p| p.into()).collect(),
        authenticated: true,
        claims: user.claims.into(),
    };
    info!("Auth info fetched");
    Ok(Some(auth_info))
}

// Member API
pub async fn get_members(config: &Config) -> Result<Vec<MemberTO>, reqwest::Error> {
    info!("Fetching members");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_member(config: &Config, id: Uuid) -> Result<MemberTO, reqwest::Error> {
    info!("Fetching member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn create_member(
    config: &Config,
    member: MemberTO,
) -> Result<MemberTO, reqwest::Error> {
    info!("Creating member");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::Client::new().post(url).json(&member).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn update_member(
    config: &Config,
    member: MemberTO,
) -> Result<MemberTO, reqwest::Error> {
    info!("Updating member {:?}", member.id);
    let id = member.id.unwrap();
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::Client::new().put(url).json(&member).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn delete_member(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    reqwest::Client::new().delete(url).send().await?.error_for_status_ref()?;
    Ok(())
}

// Member Action API
pub async fn get_member_actions(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<MemberActionTO>, reqwest::Error> {
    info!("Fetching actions for member {member_id}");
    let url = format!("{}/api/members/{member_id}/actions", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn create_member_action(
    config: &Config,
    member_id: Uuid,
    action: MemberActionTO,
) -> Result<MemberActionTO, reqwest::Error> {
    info!("Creating action for member {member_id}");
    let url = format!("{}/api/members/{member_id}/actions", config.backend);
    let response = reqwest::Client::new().post(url).json(&action).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn update_member_action(
    config: &Config,
    member_id: Uuid,
    action_id: Uuid,
    action: MemberActionTO,
) -> Result<MemberActionTO, reqwest::Error> {
    info!("Updating action {action_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/{action_id}",
        config.backend
    );
    let response = reqwest::Client::new().put(url).json(&action).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn delete_member_action(
    config: &Config,
    member_id: Uuid,
    action_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Deleting action {action_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/{action_id}",
        config.backend
    );
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

pub async fn get_migration_status(
    config: &Config,
    member_id: Uuid,
) -> Result<MigrationStatusTO, reqwest::Error> {
    info!("Fetching migration status for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/migration-status",
        config.backend
    );
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn confirm_migration(
    config: &Config,
    member_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Confirming migration for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/confirm-migration",
        config.backend
    );
    let client = reqwest::Client::new();
    client.post(url).send().await?.error_for_status_ref()?;
    Ok(())
}

// Member Document API
pub async fn get_member_documents(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<MemberDocumentTO>, reqwest::Error> {
    info!("Fetching documents for member {member_id}");
    let url = format!("{}/api/members/{member_id}/documents", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn upload_member_document(
    config: &Config,
    member_id: Uuid,
    document_type: &str,
    description: Option<&str>,
    file: web_sys::File,
) -> Result<MemberDocumentTO, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!("{}/api/members/{member_id}/documents", config.backend);

    let form_data =
        web_sys::FormData::new().map_err(|e| format!("Failed to create FormData: {:?}", e))?;
    form_data
        .append_with_str("document_type", document_type)
        .map_err(|e| format!("Failed to append document_type: {:?}", e))?;
    if let Some(desc) = description {
        form_data
            .append_with_str("description", desc)
            .map_err(|e| format!("Failed to append description: {:?}", e))?;
    }
    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|e| format!("Failed to append file: {:?}", e))?;

    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.body(Some(&form_data));

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object".to_string())?;

    if !resp.ok() {
        let status = resp.status();
        let text = JsFuture::from(resp.text().unwrap())
            .await
            .map_err(|e| format!("Failed to read error body: {:?}", e))?
            .as_string()
            .unwrap_or_default();
        return Err(format!("Upload failed ({}): {}", status, text));
    }

    let json = JsFuture::from(resp.json().unwrap())
        .await
        .map_err(|e| format!("Failed to parse response: {:?}", e))?;

    let doc: MemberDocumentTO = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Failed to deserialize: {:?}", e))?;

    Ok(doc)
}

pub async fn delete_member_document(
    config: &Config,
    member_id: Uuid,
    document_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Deleting document {document_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/documents/{document_id}",
        config.backend
    );
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

pub fn document_download_url(config: &Config, member_id: Uuid, document_id: Uuid) -> String {
    format!(
        "{}/api/members/{member_id}/documents/{document_id}",
        config.backend
    )
}

// Validation API
pub async fn get_validation(config: &Config) -> Result<ValidationResultTO, reqwest::Error> {
    info!("Fetching validation results");
    let url = format!("{}/api/validation", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}
