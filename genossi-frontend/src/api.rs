use std::collections::HashMap;
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

pub async fn generate_member_document(
    config: &Config,
    member_id: Uuid,
    document_type: &str,
) -> Result<MemberDocumentTO, String> {
    info!("Generating document {document_type} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/documents/generate/{document_type}",
        config.backend
    );
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 409 {
        return Err("Document of this type already exists".to_string());
    }

    let response = response.error_for_status().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_member_document_counts(
    config: &Config,
    document_type: &str,
) -> Result<HashMap<Uuid, i64>, String> {
    info!("Fetching document counts for type {document_type}");
    let url = format!(
        "{}/api/member-documents/counts?type={document_type}",
        config.backend
    );
    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    let string_counts: HashMap<String, i64> = response.json().await.map_err(|e| e.to_string())?;
    // Convert String keys back to Uuid
    let counts = string_counts
        .into_iter()
        .filter_map(|(k, v)| Uuid::parse_str(&k).ok().map(|id| (id, v)))
        .collect();
    Ok(counts)
}

// Template API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum FileTreeEntry {
    #[serde(rename = "file")]
    File { name: String, path: String },
    #[serde(rename = "directory")]
    Directory {
        name: String,
        path: String,
        children: Vec<FileTreeEntry>,
    },
}

pub async fn get_templates(config: &Config) -> Result<Vec<FileTreeEntry>, reqwest::Error> {
    info!("Fetching templates");
    let url = format!("{}/api/templates", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_template_content(config: &Config, path: &str) -> Result<String, String> {
    info!("Fetching template content: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.text().await.map_err(|e| e.to_string())
}

pub async fn save_template(config: &Config, path: &str, content: &str) -> Result<(), String> {
    info!("Saving template: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .put(url)
        .header("Content-Type", "text/plain")
        .body(content.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

pub async fn upload_template_file(config: &Config, path: &str, bytes: Vec<u8>) -> Result<(), String> {
    info!("Uploading template file: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .put(url)
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

pub async fn delete_template(config: &Config, path: &str) -> Result<(), String> {
    info!("Deleting template: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .delete(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

pub fn template_render_url(config: &Config, path: &str, member_id: Uuid) -> String {
    format!(
        "{}/api/templates/render/{}/{}",
        config.backend, path, member_id
    )
}

pub async fn render_template_pdf(config: &Config, path: &str, member_id: Uuid) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = template_render_url(config, path, member_id);
    info!("Rendering template PDF: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");

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
        return Err(format!("{}: {}", status, text));
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| format!("Failed to read blob: {:?}", e))?;

    let blob: web_sys::Blob = blob
        .dyn_into()
        .map_err(|_| "Not a Blob".to_string())?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create blob URL: {:?}", e))?;

    Ok(blob_url)
}

pub fn template_render_application_url(config: &Config, path: &str, application_id: Uuid) -> String {
    format!(
        "{}/api/templates/render-application/{}/{}",
        config.backend, path, application_id
    )
}

pub async fn render_template_pdf_application(config: &Config, path: &str, application_id: Uuid) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = template_render_application_url(config, path, application_id);
    info!("Rendering application template PDF: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");

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
        return Err(format!("{}: {}", status, text));
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| format!("Failed to read blob: {:?}", e))?;

    let blob: web_sys::Blob = blob
        .dyn_into()
        .map_err(|_| "Not a Blob".to_string())?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create blob URL: {:?}", e))?;

    Ok(blob_url)
}

// Config API (backend config store)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConfigEntryTO {
    pub key: String,
    pub value: String,
    pub value_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetConfigRequest {
    pub value: String,
    pub value_type: String,
}

pub async fn get_config_entries(config: &Config) -> Result<Vec<ConfigEntryTO>, reqwest::Error> {
    info!("Fetching config entries");
    let url = format!("{}/api/config", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn set_config_entry(
    config: &Config,
    key: &str,
    value: &str,
    value_type: &str,
) -> Result<ConfigEntryTO, reqwest::Error> {
    info!("Setting config entry: {key}");
    let url = format!("{}/api/config/{}", config.backend, key);
    let body = SetConfigRequest {
        value: value.to_string(),
        value_type: value_type.to_string(),
    };
    let response = reqwest::Client::new().put(url).json(&body).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn delete_config_entry(config: &Config, key: &str) -> Result<(), reqwest::Error> {
    info!("Deleting config entry: {key}");
    let url = format!("{}/api/config/{}", config.backend, key);
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GenerateApiKeyResponse {
    pub key: String,
}

pub async fn generate_api_key(config: &Config) -> Result<String, reqwest::Error> {
    info!("Generating API key");
    let url = format!("{}/api/config/generate-api-key", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    response.error_for_status_ref()?;
    let resp: GenerateApiKeyResponse = response.json().await?;
    Ok(resp.key)
}

// Application API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ApplicationStatusTO {
    Offen,
    Bestaetigt,
    Abgelehnt,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApplicationTO {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<rest_types::SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub city: Option<String>,
    pub shares: i32,
    pub status: ApplicationStatusTO,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub deleted: Option<String>,
    #[serde(default)]
    pub version: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminCreateApplicationRequest {
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salutation: Option<rest_types::SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    pub shares: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_mail: Option<bool>,
}

pub async fn get_applications(
    config: &Config,
    status_filter: Option<&str>,
) -> Result<Vec<ApplicationTO>, reqwest::Error> {
    info!("Fetching applications");
    let url = match status_filter {
        Some(status) => format!("{}/api/applications?status={}", config.backend, status),
        None => format!("{}/api/applications", config.backend),
    };
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_application(
    config: &Config,
    id: Uuid,
) -> Result<ApplicationTO, reqwest::Error> {
    info!("Fetching application {id}");
    let url = format!("{}/api/applications/{}", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn confirm_application(
    config: &Config,
    id: Uuid,
) -> Result<ApplicationTO, reqwest::Error> {
    info!("Confirming application {id}");
    let url = format!("{}/api/applications/{}/confirm", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn create_application(
    config: &Config,
    request: &AdminCreateApplicationRequest,
) -> Result<ApplicationTO, reqwest::Error> {
    info!("Creating application");
    let url = format!("{}/api/applications", config.backend);
    let response = reqwest::Client::new().post(url).json(request).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateApplicationRequest {
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salutation: Option<rest_types::SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    pub shares: i32,
    pub version: Uuid,
}

pub async fn update_application(
    config: &Config,
    id: Uuid,
    request: &UpdateApplicationRequest,
) -> Result<ApplicationTO, reqwest::Error> {
    info!("Updating application {id}");
    let url = format!("{}/api/applications/{}", config.backend, id);
    let response = reqwest::Client::new().put(url).json(request).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn reject_application(
    config: &Config,
    id: Uuid,
) -> Result<ApplicationTO, reqwest::Error> {
    info!("Rejecting application {id}");
    let url = format!("{}/api/applications/{}/reject", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

// Mail API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailJobTO {
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailRecipientTO {
    pub id: String,
    pub to_address: String,
    pub member_id: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub sent_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailJobDetailTO {
    #[serde(flatten)]
    pub job: MailJobTO,
    pub recipients: Vec<MailRecipientTO>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendMailRequest {
    pub to_address: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BulkRecipient {
    pub address: String,
    pub member_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendBulkMailRequest {
    pub to_addresses: Vec<BulkRecipient>,
    pub subject: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub static_document_ids: Vec<String>,
}

pub async fn send_bulk_mail(
    config: &Config,
    recipients: &[BulkRecipient],
    subject: &str,
    body: &str,
    attachment_ids: &[String],
    static_document_ids: &[String],
) -> Result<MailJobTO, String> {
    info!("Sending bulk mail to {} recipients", recipients.len());
    let url = format!("{}/api/mail/send-bulk", config.backend);
    let req = SendBulkMailRequest {
        to_addresses: recipients.to_vec(),
        subject: subject.to_string(),
        body: body.to_string(),
        attachment_ids: attachment_ids.to_vec(),
        static_document_ids: static_document_ids.to_vec(),
    };
    let response = reqwest::Client::new()
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PreviewRequest {
    pub subject: String,
    pub body: String,
    pub member_id: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PreviewResponse {
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub errors: Vec<String>,
}

pub async fn preview_mail(
    config: &Config,
    subject: &str,
    body: &str,
    member_id: &str,
) -> Result<PreviewResponse, String> {
    let url = format!("{}/api/mail/preview", config.backend);
    let req = PreviewRequest {
        subject: subject.to_string(),
        body: body.to_string(),
        member_id: member_id.to_string(),
    };
    let response = reqwest::Client::new()
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_mail_jobs(config: &Config) -> Result<Vec<MailJobTO>, String> {
    info!("Fetching mail jobs");
    let url = format!("{}/api/mail/jobs", config.backend);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_mail_job_detail(config: &Config, id: &str) -> Result<MailJobDetailTO, String> {
    info!("Fetching mail job detail: {id}");
    let url = format!("{}/api/mail/jobs/{}", config.backend, id);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn retry_mail_job(config: &Config, id: &str) -> Result<MailJobTO, String> {
    info!("Retrying mail job: {id}");
    let url = format!("{}/api/mail/jobs/{}/retry", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_members_not_reached_by(
    config: &Config,
    job_id: &str,
) -> Result<Vec<MemberTO>, String> {
    info!("Fetching members not reached by job {job_id}");
    let url = format!("{}/api/members/not-reached-by/{}", config.backend, job_id);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn send_test_mail(config: &Config, to_address: &str) -> Result<(), String> {
    info!("Sending test mail to: {to_address}");
    let url = format!("{}/api/mail/test", config.backend);
    let req = serde_json::json!({ "to_address": to_address });
    let response = reqwest::Client::new()
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

pub async fn test_webdav_connection(config: &Config) -> Result<(), String> {
    info!("Testing WebDAV connection");
    let url = format!("{}/api/backup/test-webdav", config.backend);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct FooterResponse {
    pub footer: String,
}

pub async fn get_mail_footer(config: &Config) -> Result<String, String> {
    info!("Fetching mail footer");
    let url = format!("{}/api/mail/footer", config.backend);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    let footer: FooterResponse = response.json().await.map_err(|e| e.to_string())?;
    Ok(footer.footer)
}

// User Preferences API
pub async fn get_user_preference(config: &Config, key: &str) -> Result<Option<rest_types::UserPreferenceTO>, reqwest::Error> {
    info!("Fetching user preference: {key}");
    let url = format!("{}/api/user-preferences/{}", config.backend, key);
    let response = reqwest::get(url).await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response.error_for_status_ref()?;
    Ok(Some(response.json().await?))
}

pub async fn set_user_preference(config: &Config, key: &str, value: &str) -> Result<rest_types::UserPreferenceTO, reqwest::Error> {
    info!("Setting user preference: {key}");
    let url = format!("{}/api/user-preferences/{}", config.backend, key);
    let body = rest_types::UserPreferenceTO {
        id: None,
        key: None,
        value: value.to_string(),
        created: None,
        version: None,
    };
    let client = reqwest::Client::new();
    let response = client.put(url).json(&body).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

// Permission API

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserResponseTO {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleResponseTO {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserRoleTO {
    pub user: String,
    pub role: String,
}

pub async fn get_all_users(config: &Config) -> Result<Vec<UserResponseTO>, reqwest::Error> {
    info!("Fetching all users");
    let url = format!("{}/api/permission/user", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_user_roles(config: &Config, username: &str) -> Result<Vec<RoleResponseTO>, reqwest::Error> {
    info!("Fetching roles for user: {username}");
    let url = format!("{}/api/permission/user/{}/roles", config.backend, username);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn assign_user_role(config: &Config, user: &str, role: &str) -> Result<(), reqwest::Error> {
    info!("Assigning role {role} to user {user}");
    let url = format!("{}/api/permission/user-role", config.backend);
    let body = UserRoleTO {
        user: user.to_string(),
        role: role.to_string(),
    };
    let client = reqwest::Client::new();
    let response = client.post(url).json(&body).send().await?;
    response.error_for_status_ref()?;
    Ok(())
}

pub async fn remove_user_role(config: &Config, user: &str, role: &str) -> Result<(), reqwest::Error> {
    info!("Removing role {role} from user {user}");
    let url = format!("{}/api/permission/user-role", config.backend);
    let body = UserRoleTO {
        user: user.to_string(),
        role: role.to_string(),
    };
    let client = reqwest::Client::new();
    let response = client.delete(url).json(&body).send().await?;
    response.error_for_status_ref()?;
    Ok(())
}

pub async fn get_user_preference_admin(config: &Config, username: &str, key: &str) -> Result<Option<rest_types::UserPreferenceTO>, reqwest::Error> {
    info!("Admin fetching preference {key} for user {username}");
    let url = format!("{}/api/permission/user/{}/preferences/{}", config.backend, username, key);
    let response = reqwest::get(url).await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response.error_for_status_ref()?;
    Ok(Some(response.json().await?))
}

pub async fn set_user_preference_admin(config: &Config, username: &str, key: &str, value: &str) -> Result<rest_types::UserPreferenceTO, reqwest::Error> {
    info!("Admin setting preference {key} for user {username}");
    let url = format!("{}/api/permission/user/{}/preferences/{}", config.backend, username, key);
    let body = rest_types::UserPreferenceTO {
        id: None,
        key: None,
        value: value.to_string(),
        created: None,
        version: None,
    };
    let client = reqwest::Client::new();
    let response = client.put(url).json(&body).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

// Validation API
pub async fn get_validation(config: &Config) -> Result<ValidationResultTO, reqwest::Error> {
    info!("Fetching validation results");
    let url = format!("{}/api/validation", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

// -------- Static Documents --------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticDocumentTO {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created: String,
}

pub async fn list_static_documents(
    config: &Config,
) -> Result<Vec<StaticDocumentTO>, reqwest::Error> {
    info!("Fetching static documents");
    let url = format!("{}/api/static-documents", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn upload_static_document(
    config: &Config,
    name: &str,
    file: web_sys::File,
) -> Result<StaticDocumentTO, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!("{}/api/static-documents", config.backend);

    let form_data =
        web_sys::FormData::new().map_err(|e| format!("Failed to create FormData: {:?}", e))?;
    form_data
        .append_with_str("name", name)
        .map_err(|e| format!("Failed to append name: {:?}", e))?;
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

    let doc: StaticDocumentTO = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Failed to deserialize: {:?}", e))?;

    Ok(doc)
}

pub async fn delete_static_document(
    config: &Config,
    id: &str,
) -> Result<(), reqwest::Error> {
    info!("Deleting static document {id}");
    let url = format!("{}/api/static-documents/{id}", config.backend);
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

pub fn static_document_download_url(config: &Config, id: &str) -> String {
    format!("{}/api/static-documents/{id}", config.backend)
}

// ── Mail Templates API ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailTemplateTO {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub body: String,
    pub version: String,
}

pub async fn list_mail_templates(config: &Config) -> Result<Vec<MailTemplateTO>, String> {
    info!("Fetching mail templates");
    let url = format!("{}/api/mail/templates", config.backend);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn create_mail_template(
    config: &Config,
    name: &str,
    subject: &str,
    body: &str,
) -> Result<MailTemplateTO, String> {
    info!("Creating mail template: {name}");
    let url = format!("{}/api/mail/templates", config.backend);
    let req = serde_json::json!({
        "name": name,
        "subject": subject,
        "body": body,
    });
    let response = reqwest::Client::new()
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn update_mail_template(
    config: &Config,
    id: &str,
    name: &str,
    subject: &str,
    body: &str,
    version: &str,
) -> Result<MailTemplateTO, String> {
    info!("Updating mail template: {id}");
    let url = format!("{}/api/mail/templates/{id}", config.backend);
    let req = serde_json::json!({
        "name": name,
        "subject": subject,
        "body": body,
        "version": version,
    });
    let response = reqwest::Client::new()
        .put(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn delete_mail_template(config: &Config, id: &str) -> Result<(), String> {
    info!("Deleting mail template: {id}");
    let url = format!("{}/api/mail/templates/{id}", config.backend);
    let response = reqwest::Client::new()
        .delete(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

// ── Inbox API ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InboundMailTO {
    pub id: String,
    pub from_address: String,
    pub subject: String,
    pub received_at: String,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub replied: bool,
    pub done: bool,
    pub archived: bool,
    pub assigned_member_id: Option<String>,
    pub assigned_member_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InboundMailDetailTO {
    pub id: String,
    pub from_address: String,
    pub subject: String,
    pub received_at: String,
    pub body_text: String,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub replied: bool,
    pub done: bool,
    pub archived: bool,
    pub assigned_member_id: Option<String>,
    pub assigned_member_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AssignMemberReq {
    member_id: String,
}

pub async fn get_imap_folders(config: &Config) -> Result<Vec<String>, String> {
    let url = format!("{}/api/inbox/folders", config.backend);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_inbox(config: &Config) -> Result<Vec<InboundMailTO>, String> {
    let url = format!("{}/api/inbox", config.backend);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_inbox_detail(config: &Config, id: &str) -> Result<InboundMailDetailTO, String> {
    let url = format!("{}/api/inbox/{}", config.backend, id);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn assign_inbox_mail(
    config: &Config,
    id: &str,
    member_id: &str,
) -> Result<InboundMailTO, String> {
    let url = format!("{}/api/inbox/{}/assign", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .json(&AssignMemberReq {
            member_id: member_id.to_string(),
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn unassign_inbox_mail(config: &Config, id: &str) -> Result<InboundMailTO, String> {
    let url = format!("{}/api/inbox/{}/unassign", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn mark_inbox_mail_read(config: &Config, id: &str) -> Result<(), String> {
    let url = format!("{}/api/inbox/{}/mark-read", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn archive_inbox_mail(config: &Config, id: &str) -> Result<InboundMailTO, String> {
    let url = format!("{}/api/inbox/{}/archive", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn done_inbox_mail(config: &Config, id: &str) -> Result<InboundMailTO, String> {
    let url = format!("{}/api/inbox/{}/done", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

pub async fn reply_inbox_mail(
    config: &Config,
    id: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let url = format!("{}/api/inbox/{}/reply", config.backend, id);
    let payload = serde_json::json!({
        "subject": subject,
        "body": body,
    });
    let response = reqwest::Client::new()
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status_ref().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_member_communications(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<rest_types::CommunicationEntryTO>, reqwest::Error> {
    let url = format!(
        "{}/api/members/{}/communications",
        config.backend, member_id
    );
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_audit_log(
    config: &Config,
    params: &std::collections::HashMap<String, String>,
) -> Result<Vec<rest_types::AuditLogEntryTO>, reqwest::Error> {
    let query_string: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");
    let url = if query_string.is_empty() {
        format!("{}/api/audit", config.backend)
    } else {
        format!("{}/api/audit?{}", config.backend, query_string)
    };
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_audit_by_entity(
    config: &Config,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<Vec<rest_types::AuditLogEntryTO>, reqwest::Error> {
    let url = format!(
        "{}/api/audit/{}/{}",
        config.backend, entity_type, entity_id
    );
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn verify_audit_chain(
    config: &Config,
) -> Result<rest_types::VerifyResponseTO, reqwest::Error> {
    let url = format!("{}/api/audit/verify", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_timestamps(
    config: &Config,
) -> Result<Vec<rest_types::TimestampResponseTO>, reqwest::Error> {
    let url = format!("{}/api/audit/timestamps", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn create_timestamp(
    config: &Config,
) -> Result<rest_types::TimestampCreateResponseTO, reqwest::Error> {
    let url = format!("{}/api/audit/timestamps", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn verify_timestamp(
    config: &Config,
    id: uuid::Uuid,
) -> Result<rest_types::TimestampVerifyResponseTO, reqwest::Error> {
    let url = format!("{}/api/audit/timestamps/{}/verify", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}
