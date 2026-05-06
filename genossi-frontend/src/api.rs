use std::collections::HashMap;
use std::rc::Rc;

use rest_types::{
    MemberActionTO, MemberDocumentTO, MemberTO, MigrationStatusTO, UserTO, ValidationResultTO,
};
use tracing::info;
use uuid::Uuid;

use crate::state::{AuthInfo, Config};

// ── AppError ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct AppError {
    pub status: Option<u16>,
    pub message: String,
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(status: Option<u16>, message: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            status,
            message: message.into(),
            detail,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // reqwest::Error umfasst Network-, Build- und Decode-Fehler. Connection-Banner
        // soll nur bei echten Netzwerk-Problemen erscheinen — JSON-Decode-Fehler
        // sind meistens Schema-Mismatches zwischen Backend-TO und Frontend-TO und
        // kommen NICHT durch eine fehlende Internetverbindung. Trenne die Klassen,
        // damit der Fehler-Toast eine sinnvolle Meldung zeigt.
        let status = e.status().map(|s| s.as_u16());
        let detail = Some(e.to_string());
        // reqwest's wasm32-Backend exponiert KEIN is_connect()/is_body() —
        // wir nutzen nur die Klassifier, die in beiden Targets stabil sind:
        // is_decode, is_timeout, is_request, is_redirect, is_status.
        let message = if e.is_decode() {
            "Antwort vom Server konnte nicht gelesen werden (JSON-Parse-Fehler)".into()
        } else if e.is_timeout() {
            "Zeitüberschreitung — Server antwortet nicht".into()
        } else if e.is_request() && status.is_none() {
            // is_request() ohne Status erfasst sowohl Build-Fehler (URL ungültig)
            // als auch Network-Layer-Fehler im wasm-Target (fetch() rejected, CORS,
            // Backend nicht erreichbar). Das ist die echte Connection-Schiene.
            "Verbindungsfehler — bitte Internetverbindung prüfen".into()
        } else {
            // Fallback: kein klares Signal aus reqwest. Zeig die Detail-Message
            // mit, statt fälschlich als Connection-Problem zu interpretieren.
            format!("Fehler beim API-Aufruf: {}", e)
        };
        AppError {
            status,
            message,
            detail,
        }
    }
}

fn status_to_message(status: u16) -> &'static str {
    match status {
        400 => "Ungültige Anfrage",
        401 => "Keine Berechtigung — bitte erneut anmelden",
        403 => "Keine Berechtigung für diese Aktion",
        404 => "Nicht gefunden",
        409 => "Konflikt — das Element wurde zwischenzeitlich geändert",
        410 => "Bereits eingelöst",
        415 => "Dateityp nicht erlaubt",
        422 => "Validierungsfehler",
        429 => "Zu viele Anfragen — bitte warten",
        500..=599 => "Serverfehler — bitte später erneut versuchen",
        _ => "Unbekannter Fehler",
    }
}

async fn map_response_error(response: reqwest::Response) -> AppError {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();

    let message = if status == 415 {
        parse_415_message(&body)
    } else {
        status_to_message(status).to_string()
    };

    AppError {
        status: Some(status),
        message,
        detail: if body.is_empty() { None } else { Some(body) },
    }
}

fn parse_415_message(body: &str) -> String {
    #[derive(serde::Deserialize)]
    struct FileTypeError {
        allowed_extensions: Option<Vec<String>>,
    }
    if let Ok(parsed) = serde_json::from_str::<FileTypeError>(body) {
        if let Some(exts) = parsed.allowed_extensions {
            return format!("Dateityp nicht erlaubt. Erlaubte Typen: {}", exts.join(", "));
        }
    }
    "Dateityp nicht erlaubt".to_string()
}

async fn check_response(response: reqwest::Response) -> Result<reqwest::Response, AppError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(map_response_error(response).await)
    }
}

async fn map_web_response_error(resp: &web_sys::Response) -> AppError {
    use wasm_bindgen_futures::JsFuture;
    let status = resp.status();
    let body = if let Ok(text_promise) = resp.text() {
        JsFuture::from(text_promise)
            .await
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let message = if status == 415 {
        parse_415_message(&body)
    } else {
        status_to_message(status).to_string()
    };

    AppError {
        status: Some(status),
        message,
        detail: if body.is_empty() { None } else { Some(body) },
    }
}

// Config API
pub async fn fetch_config() -> Result<Config, AppError> {
    info!("Fetching config");
    let window = web_sys::window().unwrap();
    let origin = window.location().origin().unwrap();
    let url = format!("{}/assets/config.json", origin);
    let response = check_response(reqwest::get(url).await?).await?;
    let config: Config = response.json().await?;
    info!("Config fetched: {:?}", config);
    Ok(config)
}

// Authentication API
pub async fn fetch_auth_info(backend_url: Rc<str>) -> Result<Option<AuthInfo>, AppError> {
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
pub async fn get_members(config: &Config) -> Result<Vec<MemberTO>, AppError> {
    info!("Fetching members");
    let url = format!("{}/api/members", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_member(config: &Config, id: Uuid) -> Result<MemberTO, AppError> {
    info!("Fetching member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn create_member(config: &Config, member: MemberTO) -> Result<MemberTO, AppError> {
    info!("Creating member");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::Client::new()
        .post(url)
        .json(&member)
        .send()
        .await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn update_member(config: &Config, member: MemberTO) -> Result<MemberTO, AppError> {
    info!("Updating member {:?}", member.id);
    let id = member.id.unwrap();
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::Client::new().put(url).json(&member).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn delete_member(config: &Config, id: Uuid) -> Result<(), AppError> {
    info!("Deleting member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}

// Member Action API
pub async fn get_member_actions(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<MemberActionTO>, AppError> {
    info!("Fetching actions for member {member_id}");
    let url = format!("{}/api/members/{member_id}/actions", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn create_member_action(
    config: &Config,
    member_id: Uuid,
    action: MemberActionTO,
) -> Result<MemberActionTO, AppError> {
    info!("Creating action for member {member_id}");
    let url = format!("{}/api/members/{member_id}/actions", config.backend);
    let response = reqwest::Client::new()
        .post(url)
        .json(&action)
        .send()
        .await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn update_member_action(
    config: &Config,
    member_id: Uuid,
    action_id: Uuid,
    action: MemberActionTO,
) -> Result<MemberActionTO, AppError> {
    info!("Updating action {action_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/{action_id}",
        config.backend
    );
    let response = reqwest::Client::new().put(url).json(&action).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn delete_member_action(
    config: &Config,
    member_id: Uuid,
    action_id: Uuid,
) -> Result<(), AppError> {
    info!("Deleting action {action_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/{action_id}",
        config.backend
    );
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn get_migration_status(
    config: &Config,
    member_id: Uuid,
) -> Result<MigrationStatusTO, AppError> {
    info!("Fetching migration status for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/migration-status",
        config.backend
    );
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn confirm_migration(config: &Config, member_id: Uuid) -> Result<(), AppError> {
    info!("Confirming migration for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/confirm-migration",
        config.backend
    );
    let response = reqwest::Client::new().post(url).send().await?;
    check_response(response).await?;
    Ok(())
}

// Member Document API
pub async fn get_member_documents(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<MemberDocumentTO>, AppError> {
    info!("Fetching documents for member {member_id}");
    let url = format!("{}/api/members/{member_id}/documents", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn upload_member_document(
    config: &Config,
    member_id: Uuid,
    document_type: &str,
    description: Option<&str>,
    file: web_sys::File,
) -> Result<MemberDocumentTO, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!("{}/api/members/{member_id}/documents", config.backend);

    let form_data = web_sys::FormData::new()
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    form_data
        .append_with_str("document_type", document_type)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    if let Some(desc) = description {
        form_data
            .append_with_str("description", desc)
            .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    }
    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.body(Some(&form_data));

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let json = JsFuture::from(resp.json().unwrap())
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let doc: MemberDocumentTO = serde_wasm_bindgen::from_value(json)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    Ok(doc)
}

pub async fn delete_member_document(
    config: &Config,
    member_id: Uuid,
    document_id: Uuid,
) -> Result<(), AppError> {
    info!("Deleting document {document_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/documents/{document_id}",
        config.backend
    );
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
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
) -> Result<MemberDocumentTO, AppError> {
    info!("Generating document {document_type} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/documents/generate/{document_type}",
        config.backend
    );
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn get_member_document_counts(
    config: &Config,
    document_type: &str,
) -> Result<HashMap<Uuid, i64>, AppError> {
    info!("Fetching document counts for type {document_type}");
    let url = format!(
        "{}/api/member-documents/counts?type={document_type}",
        config.backend
    );
    let response = check_response(reqwest::get(&url).await?).await?;
    let string_counts: HashMap<String, i64> = response.json().await?;
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

pub async fn get_templates(config: &Config) -> Result<Vec<FileTreeEntry>, AppError> {
    info!("Fetching templates");
    let url = format!("{}/api/templates", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_template_content(config: &Config, path: &str) -> Result<String, AppError> {
    info!("Fetching template content: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.text().await?)
}

pub async fn save_template(config: &Config, path: &str, content: &str) -> Result<(), AppError> {
    info!("Saving template: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .put(url)
        .header("Content-Type", "text/plain")
        .body(content.to_string())
        .send()
        .await?;
    check_response(response).await?;
    Ok(())
}

pub async fn upload_template_file(
    config: &Config,
    path: &str,
    bytes: Vec<u8>,
) -> Result<(), AppError> {
    info!("Uploading template file: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .put(url)
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await?;
    check_response(response).await?;
    Ok(())
}

pub async fn delete_template(config: &Config, path: &str) -> Result<(), AppError> {
    info!("Deleting template: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}

pub fn template_render_url(config: &Config, path: &str, member_id: Uuid) -> String {
    format!(
        "{}/api/templates/render/{}/{}",
        config.backend, path, member_id
    )
}

pub async fn render_template_pdf(
    config: &Config,
    path: &str,
    member_id: Uuid,
) -> Result<String, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = template_render_url(config, path, member_id);
    info!("Rendering template PDF: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let blob: web_sys::Blob = blob
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    Ok(blob_url)
}

pub fn template_render_application_url(
    config: &Config,
    path: &str,
    application_id: Uuid,
) -> String {
    format!(
        "{}/api/templates/render-application/{}/{}",
        config.backend, path, application_id
    )
}

pub async fn render_template_pdf_application(
    config: &Config,
    path: &str,
    application_id: Uuid,
) -> Result<String, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = template_render_application_url(config, path, application_id);
    info!("Rendering application template PDF: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let blob: web_sys::Blob = blob
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

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

pub async fn get_config_entries(config: &Config) -> Result<Vec<ConfigEntryTO>, AppError> {
    info!("Fetching config entries");
    let url = format!("{}/api/config", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn set_config_entry(
    config: &Config,
    key: &str,
    value: &str,
    value_type: &str,
) -> Result<ConfigEntryTO, AppError> {
    info!("Setting config entry: {key}");
    let url = format!("{}/api/config/{}", config.backend, key);
    let body = SetConfigRequest {
        value: value.to_string(),
        value_type: value_type.to_string(),
    };
    let response = reqwest::Client::new().put(url).json(&body).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn delete_config_entry(config: &Config, key: &str) -> Result<(), AppError> {
    info!("Deleting config entry: {key}");
    let url = format!("{}/api/config/{}", config.backend, key);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GenerateApiKeyResponse {
    pub key: String,
}

pub async fn generate_api_key(config: &Config) -> Result<String, AppError> {
    info!("Generating API key");
    let url = format!("{}/api/config/generate-api-key", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
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
) -> Result<Vec<ApplicationTO>, AppError> {
    info!("Fetching applications");
    let url = match status_filter {
        Some(status) => format!("{}/api/applications?status={}", config.backend, status),
        None => format!("{}/api/applications", config.backend),
    };
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_application(config: &Config, id: Uuid) -> Result<ApplicationTO, AppError> {
    info!("Fetching application {id}");
    let url = format!("{}/api/applications/{}", config.backend, id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn confirm_application(
    config: &Config,
    id: Uuid,
) -> Result<ApplicationTO, AppError> {
    info!("Confirming application {id}");
    let url = format!("{}/api/applications/{}/confirm", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn create_application(
    config: &Config,
    request: &AdminCreateApplicationRequest,
) -> Result<ApplicationTO, AppError> {
    info!("Creating application");
    let url = format!("{}/api/applications", config.backend);
    let response = reqwest::Client::new()
        .post(url)
        .json(request)
        .send()
        .await?;
    let response = check_response(response).await?;
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
) -> Result<ApplicationTO, AppError> {
    info!("Updating application {id}");
    let url = format!("{}/api/applications/{}", config.backend, id);
    let response = reqwest::Client::new().put(url).json(request).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn reject_application(
    config: &Config,
    id: Uuid,
) -> Result<ApplicationTO, AppError> {
    info!("Rejecting application {id}");
    let url = format!("{}/api/applications/{}/reject", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
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
) -> Result<MailJobTO, AppError> {
    info!("Sending bulk mail to {} recipients", recipients.len());
    let url = format!("{}/api/mail/send-bulk", config.backend);
    let req = SendBulkMailRequest {
        to_addresses: recipients.to_vec(),
        subject: subject.to_string(),
        body: body.to_string(),
        attachment_ids: attachment_ids.to_vec(),
        static_document_ids: static_document_ids.to_vec(),
    };
    let response = reqwest::Client::new().post(url).json(&req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
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
) -> Result<PreviewResponse, AppError> {
    let url = format!("{}/api/mail/preview", config.backend);
    let req = PreviewRequest {
        subject: subject.to_string(),
        body: body.to_string(),
        member_id: member_id.to_string(),
    };
    let response = reqwest::Client::new().post(url).json(&req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn get_mail_jobs(config: &Config) -> Result<Vec<MailJobTO>, AppError> {
    info!("Fetching mail jobs");
    let url = format!("{}/api/mail/jobs", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_mail_job_detail(config: &Config, id: &str) -> Result<MailJobDetailTO, AppError> {
    info!("Fetching mail job detail: {id}");
    let url = format!("{}/api/mail/jobs/{}", config.backend, id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn retry_mail_job(config: &Config, id: &str) -> Result<MailJobTO, AppError> {
    info!("Retrying mail job: {id}");
    let url = format!("{}/api/mail/jobs/{}/retry", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn get_members_not_reached_by(
    config: &Config,
    job_id: &str,
) -> Result<Vec<MemberTO>, AppError> {
    info!("Fetching members not reached by job {job_id}");
    let url = format!("{}/api/members/not-reached-by/{}", config.backend, job_id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn send_test_mail(config: &Config, to_address: &str) -> Result<(), AppError> {
    info!("Sending test mail to: {to_address}");
    let url = format!("{}/api/mail/test", config.backend);
    let req = serde_json::json!({ "to_address": to_address });
    let response = reqwest::Client::new().post(url).json(&req).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn test_webdav_connection(config: &Config) -> Result<(), AppError> {
    info!("Testing WebDAV connection");
    let url = format!("{}/api/backup/test-webdav", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    check_response(response).await?;
    Ok(())
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct FooterResponse {
    pub footer: String,
}

pub async fn get_mail_footer(config: &Config) -> Result<String, AppError> {
    info!("Fetching mail footer");
    let url = format!("{}/api/mail/footer", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    let footer: FooterResponse = response.json().await?;
    Ok(footer.footer)
}

// User Preferences API
pub async fn get_user_preference(
    config: &Config,
    key: &str,
) -> Result<Option<rest_types::UserPreferenceTO>, AppError> {
    info!("Fetching user preference: {key}");
    let url = format!("{}/api/user-preferences/{}", config.backend, key);
    let response = reqwest::get(url).await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = check_response(response).await?;
    Ok(Some(response.json().await?))
}

pub async fn set_user_preference(
    config: &Config,
    key: &str,
    value: &str,
) -> Result<rest_types::UserPreferenceTO, AppError> {
    info!("Setting user preference: {key}");
    let url = format!("{}/api/user-preferences/{}", config.backend, key);
    let body = rest_types::UserPreferenceTO {
        id: None,
        key: None,
        value: value.to_string(),
        created: None,
        version: None,
    };
    let response = reqwest::Client::new().put(url).json(&body).send().await?;
    let response = check_response(response).await?;
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

pub async fn get_all_users(config: &Config) -> Result<Vec<UserResponseTO>, AppError> {
    info!("Fetching all users");
    let url = format!("{}/api/permission/user", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_user_roles(
    config: &Config,
    username: &str,
) -> Result<Vec<RoleResponseTO>, AppError> {
    info!("Fetching roles for user: {username}");
    let url = format!("{}/api/permission/user/{}/roles", config.backend, username);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn assign_user_role(
    config: &Config,
    user: &str,
    role: &str,
) -> Result<(), AppError> {
    info!("Assigning role {role} to user {user}");
    let url = format!("{}/api/permission/user-role", config.backend);
    let body = UserRoleTO {
        user: user.to_string(),
        role: role.to_string(),
    };
    let response = reqwest::Client::new().post(url).json(&body).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn remove_user_role(
    config: &Config,
    user: &str,
    role: &str,
) -> Result<(), AppError> {
    info!("Removing role {role} from user {user}");
    let url = format!("{}/api/permission/user-role", config.backend);
    let body = UserRoleTO {
        user: user.to_string(),
        role: role.to_string(),
    };
    let response = reqwest::Client::new()
        .delete(url)
        .json(&body)
        .send()
        .await?;
    check_response(response).await?;
    Ok(())
}

pub async fn get_user_preference_admin(
    config: &Config,
    username: &str,
    key: &str,
) -> Result<Option<rest_types::UserPreferenceTO>, AppError> {
    info!("Admin fetching preference {key} for user {username}");
    let url = format!(
        "{}/api/permission/user/{}/preferences/{}",
        config.backend, username, key
    );
    let response = reqwest::get(url).await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = check_response(response).await?;
    Ok(Some(response.json().await?))
}

pub async fn set_user_preference_admin(
    config: &Config,
    username: &str,
    key: &str,
    value: &str,
) -> Result<rest_types::UserPreferenceTO, AppError> {
    info!("Admin setting preference {key} for user {username}");
    let url = format!(
        "{}/api/permission/user/{}/preferences/{}",
        config.backend, username, key
    );
    let body = rest_types::UserPreferenceTO {
        id: None,
        key: None,
        value: value.to_string(),
        created: None,
        version: None,
    };
    let response = reqwest::Client::new().put(url).json(&body).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

// Validation API
pub async fn get_validation(config: &Config) -> Result<ValidationResultTO, AppError> {
    info!("Fetching validation results");
    let url = format!("{}/api/validation", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
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
) -> Result<Vec<StaticDocumentTO>, AppError> {
    info!("Fetching static documents");
    let url = format!("{}/api/static-documents", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn upload_static_document(
    config: &Config,
    name: &str,
    file: web_sys::File,
) -> Result<StaticDocumentTO, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!("{}/api/static-documents", config.backend);

    let form_data = web_sys::FormData::new()
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    form_data
        .append_with_str("name", name)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.body(Some(&form_data));

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let json = JsFuture::from(resp.json().unwrap())
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let doc: StaticDocumentTO = serde_wasm_bindgen::from_value(json)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    Ok(doc)
}

pub async fn delete_static_document(config: &Config, id: &str) -> Result<(), AppError> {
    info!("Deleting static document {id}");
    let url = format!("{}/api/static-documents/{id}", config.backend);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
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

pub async fn list_mail_templates(config: &Config) -> Result<Vec<MailTemplateTO>, AppError> {
    info!("Fetching mail templates");
    let url = format!("{}/api/mail/templates", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn create_mail_template(
    config: &Config,
    name: &str,
    subject: &str,
    body: &str,
) -> Result<MailTemplateTO, AppError> {
    info!("Creating mail template: {name}");
    let url = format!("{}/api/mail/templates", config.backend);
    let req = serde_json::json!({
        "name": name,
        "subject": subject,
        "body": body,
    });
    let response = reqwest::Client::new().post(url).json(&req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn update_mail_template(
    config: &Config,
    id: &str,
    name: &str,
    subject: &str,
    body: &str,
    version: &str,
) -> Result<MailTemplateTO, AppError> {
    info!("Updating mail template: {id}");
    let url = format!("{}/api/mail/templates/{id}", config.backend);
    let req = serde_json::json!({
        "name": name,
        "subject": subject,
        "body": body,
        "version": version,
    });
    let response = reqwest::Client::new().put(url).json(&req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn delete_mail_template(config: &Config, id: &str) -> Result<(), AppError> {
    info!("Deleting mail template: {id}");
    let url = format!("{}/api/mail/templates/{id}", config.backend);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
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

pub async fn get_imap_folders(config: &Config) -> Result<Vec<String>, AppError> {
    let url = format!("{}/api/inbox/folders", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_inbox(config: &Config) -> Result<Vec<InboundMailTO>, AppError> {
    let url = format!("{}/api/inbox", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_inbox_detail(config: &Config, id: &str) -> Result<InboundMailDetailTO, AppError> {
    let url = format!("{}/api/inbox/{}", config.backend, id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn assign_inbox_mail(
    config: &Config,
    id: &str,
    member_id: &str,
) -> Result<InboundMailTO, AppError> {
    let url = format!("{}/api/inbox/{}/assign", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .json(&AssignMemberReq {
            member_id: member_id.to_string(),
        })
        .send()
        .await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn unassign_inbox_mail(config: &Config, id: &str) -> Result<InboundMailTO, AppError> {
    let url = format!("{}/api/inbox/{}/unassign", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn mark_inbox_mail_read(config: &Config, id: &str) -> Result<(), AppError> {
    let url = format!("{}/api/inbox/{}/mark-read", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn archive_inbox_mail(config: &Config, id: &str) -> Result<InboundMailTO, AppError> {
    let url = format!("{}/api/inbox/{}/archive", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn done_inbox_mail(config: &Config, id: &str) -> Result<InboundMailTO, AppError> {
    let url = format!("{}/api/inbox/{}/done", config.backend, id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn reply_inbox_mail(
    config: &Config,
    id: &str,
    subject: &str,
    body: &str,
) -> Result<(), AppError> {
    let url = format!("{}/api/inbox/{}/reply", config.backend, id);
    let payload = serde_json::json!({
        "subject": subject,
        "body": body,
    });
    let response = reqwest::Client::new().post(url).json(&payload).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn get_member_communications(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<rest_types::CommunicationEntryTO>, AppError> {
    let url = format!(
        "{}/api/members/{}/communications",
        config.backend, member_id
    );
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_audit_log(
    config: &Config,
    params: &std::collections::HashMap<String, String>,
    page: i64,
    size: i64,
) -> Result<rest_types::PagedAuditLogTO, AppError> {
    let mut all_params = params.clone();
    all_params.insert("page".to_string(), page.to_string());
    all_params.insert("size".to_string(), size.to_string());
    let query_string: String = all_params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");
    let url = format!("{}/api/audit?{}", config.backend, query_string);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_audit_by_entity(
    config: &Config,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<Vec<rest_types::AuditLogEntryTO>, AppError> {
    let url = format!("{}/api/audit/{}/{}", config.backend, entity_type, entity_id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn verify_audit_chain(
    config: &Config,
) -> Result<rest_types::VerifyResponseTO, AppError> {
    let url = format!("{}/api/audit/verify", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_timestamps(
    config: &Config,
) -> Result<Vec<rest_types::TimestampResponseTO>, AppError> {
    let url = format!("{}/api/audit/timestamps", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn create_timestamp(
    config: &Config,
) -> Result<rest_types::TimestampCreateResponseTO, AppError> {
    let url = format!("{}/api/audit/timestamps", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn verify_timestamp(
    config: &Config,
    id: uuid::Uuid,
) -> Result<rest_types::TimestampVerifyResponseTO, AppError> {
    let url = format!("{}/api/audit/timestamps/{}/verify", config.backend, id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_open_applications_count(config: &Config) -> Option<usize> {
    get_applications(config, Some("Offen"))
        .await
        .ok()
        .map(|v| v.len())
}

pub async fn get_open_inbox_count(config: &Config) -> Option<usize> {
    get_inbox(config)
        .await
        .ok()
        .map(|mails| mails.iter().filter(|m| !m.done).count())
}

// Session Management API
pub async fn revoke_all_sessions(
    config: &Config,
) -> Result<rest_types::SessionRevokeResponse, AppError> {
    let url = format!("{}/api/session/revoke-all", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn revoke_user_sessions(
    config: &Config,
    user_id: &str,
) -> Result<rest_types::SessionRevokeResponse, AppError> {
    let url = format!("{}/api/session/revoke/{}", config.backend, user_id);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

// ─── Phase 4 ─── Assembly TOs ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AssemblyStatusTO {
    Preparation,
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AssemblyTO {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    pub status: AssemblyStatusTO,
    #[serde(default)]
    pub opened_at: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub deleted: Option<String>,
    #[serde(default)]
    pub version: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAssemblyRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateAssemblyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub version: Uuid,
}

// ─── Phase 4 ─── Helper Token TOs ───────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HelperTokenStatusTO {
    Open,
    Used,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HelperTokenTO {
    pub id: Uuid,
    pub assembly_id: Uuid,
    pub memo: String,
    pub status: HelperTokenStatusTO,
    #[serde(default)]
    pub used_at: Option<String>,
    #[serde(default)]
    pub revoked_at: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    pub version: Uuid,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HelperTokenCreateResponseTO {
    pub token: HelperTokenTO,
    pub code: String,
    pub qr_svg: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateHelperTokenRequest {
    pub memo: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RedeemRequest {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RedeemResponse {
    pub assembly_id: Uuid,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HelperSessionTO {
    pub assembly_id: Uuid,
    pub assembly_name: String,
    pub expires_at: String,
}

// ─── Phase 4 ─── Attendance TOs (PII Whitelist — exactly 7 fields) ──

/// Reduced helper-view of a member.
///
/// **WHITELIST CONTRACT:** This struct mirrors the backend
/// `AttendanceMemberTO` (Phase 3 D-24) which is enforced at the DAO
/// layer to expose ONLY 7 fields (`member_number`, `first_name`,
/// `last_name`, `salutation`, `title`, `is_present`, `member_id`).
/// DO NOT add fields here — the frontend acts as the last line of
/// defence against accidental PII leaks.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AttendanceMemberTO {
    pub member_number: i64,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub salutation: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub is_present: bool,
    pub member_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AttendanceStatsTO {
    pub present: u64,
    pub total: u64,
}

// ─── Phase 4 ─── Assembly endpoints ─────────────────────────────────

pub async fn list_assemblies(config: &Config) -> Result<Vec<AssemblyTO>, AppError> {
    info!("Fetching assemblies");
    let url = format!("{}/api/assembly", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn get_assembly(config: &Config, id: Uuid) -> Result<AssemblyTO, AppError> {
    // Backend returns AssemblyDetailTO { assembly, snapshot_member_count } from
    // GET /api/assembly/{id} — NOT a flat AssemblyTO. The list endpoint
    // /api/assembly returns Vec<AssemblyTO> directly; only the detail endpoint
    // wraps. We unwrap the wrapper here so callers continue to work with the
    // flat AssemblyTO they expect.
    #[derive(serde::Deserialize)]
    struct AssemblyDetailWrapper {
        assembly: AssemblyTO,
        #[serde(default, rename = "snapshot_member_count")]
        _snapshot_member_count: Option<u64>,
    }
    info!("Fetching assembly {id}");
    let url = format!("{}/api/assembly/{id}", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    let detail: AssemblyDetailWrapper = response.json().await?;
    Ok(detail.assembly)
}

pub async fn create_assembly(
    config: &Config,
    req: &CreateAssemblyRequest,
) -> Result<AssemblyTO, AppError> {
    info!("Creating assembly");
    let url = format!("{}/api/assembly", config.backend);
    let response = reqwest::Client::new().post(url).json(req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn update_assembly(
    config: &Config,
    id: Uuid,
    req: &UpdateAssemblyRequest,
) -> Result<AssemblyTO, AppError> {
    info!("Updating assembly {id}");
    let url = format!("{}/api/assembly/{id}", config.backend);
    let response = reqwest::Client::new().put(url).json(req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn open_assembly(config: &Config, id: Uuid) -> Result<AssemblyTO, AppError> {
    info!("Opening assembly {id}");
    let url = format!("{}/api/assembly/{id}/open", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn close_assembly(config: &Config, id: Uuid) -> Result<AssemblyTO, AppError> {
    info!("Closing assembly {id}");
    let url = format!("{}/api/assembly/{id}/close", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

// ─── Phase 4 ─── Helper-Token endpoints ─────────────────────────────

pub async fn list_helper_tokens(
    config: &Config,
    assembly_id: Uuid,
) -> Result<Vec<HelperTokenTO>, AppError> {
    info!("Listing helper tokens for assembly {assembly_id}");
    let url = format!(
        "{}/api/assembly/{assembly_id}/helper-tokens",
        config.backend
    );
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn create_helper_token(
    config: &Config,
    assembly_id: Uuid,
    memo: String,
) -> Result<HelperTokenCreateResponseTO, AppError> {
    // SECURITY (T-04-13): do NOT log the memo (PII).
    info!("Creating helper token for assembly {assembly_id}");
    let url = format!(
        "{}/api/assembly/{assembly_id}/helper-tokens",
        config.backend
    );
    let body = CreateHelperTokenRequest { memo };
    let response = reqwest::Client::new()
        .post(url)
        .json(&body)
        .send()
        .await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn revoke_helper_token(
    config: &Config,
    assembly_id: Uuid,
    token_id: Uuid,
) -> Result<HelperTokenTO, AppError> {
    info!("Revoking helper token {token_id}");
    let url = format!(
        "{}/api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke",
        config.backend
    );
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn redeem_helper_token(
    config: &Config,
    code: String,
) -> Result<RedeemResponse, AppError> {
    // SECURITY (T-04-13): do NOT log the code (one-time secret).
    info!("Redeeming helper token");
    let url = format!("{}/api/helper/redeem", config.backend);
    let body = RedeemRequest { code };
    let response = reqwest::Client::new().post(url).json(&body).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn get_helper_session(config: &Config) -> Result<HelperSessionTO, AppError> {
    info!("Checking helper session");
    let url = format!("{}/api/helper/session", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn helper_logout(config: &Config) -> Result<(), AppError> {
    info!("Logging out helper");
    let url = format!("{}/api/helper/logout", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    check_response(response).await?;
    Ok(())
}

// ─── Phase 4 ─── Attendance endpoints ───────────────────────────────

pub async fn list_attendance_members(
    config: &Config,
    assembly_id: Uuid,
    search: Option<&str>,
) -> Result<Vec<AttendanceMemberTO>, AppError> {
    info!("Listing attendance members for assembly {assembly_id}");
    let url = match search {
        Some(q) if !q.is_empty() => format!(
            "{}/api/attendance/{assembly_id}/members?q={}",
            config.backend,
            js_sys::encode_uri_component(q)
        ),
        _ => format!("{}/api/attendance/{assembly_id}/members", config.backend),
    };
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn mark_present(
    config: &Config,
    assembly_id: Uuid,
    member_id: Uuid,
) -> Result<(), AppError> {
    info!("Marking present: {member_id} in {assembly_id}");
    let url = format!(
        "{}/api/attendance/{assembly_id}/{member_id}",
        config.backend
    );
    let response = reqwest::Client::new().put(url).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn mark_absent(
    config: &Config,
    assembly_id: Uuid,
    member_id: Uuid,
) -> Result<(), AppError> {
    info!("Marking absent: {member_id} in {assembly_id}");
    let url = format!(
        "{}/api/attendance/{assembly_id}/{member_id}",
        config.backend
    );
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}

pub async fn get_assembly_stats(
    config: &Config,
    assembly_id: Uuid,
) -> Result<AttendanceStatsTO, AppError> {
    // Polled every ~5s by LiveCounter — kept at info! to match neighbouring
    // endpoints; can be lowered to debug! later if log noise becomes an issue.
    let url = format!("{}/api/assembly/{assembly_id}/stats", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_to_message_known_codes() {
        assert_eq!(status_to_message(400), "Ungültige Anfrage");
        assert_eq!(
            status_to_message(401),
            "Keine Berechtigung — bitte erneut anmelden"
        );
        assert_eq!(
            status_to_message(403),
            "Keine Berechtigung für diese Aktion"
        );
        assert_eq!(status_to_message(404), "Nicht gefunden");
        assert_eq!(
            status_to_message(409),
            "Konflikt — das Element wurde zwischenzeitlich geändert"
        );
        assert_eq!(status_to_message(410), "Bereits eingelöst");
        assert_eq!(status_to_message(415), "Dateityp nicht erlaubt");
        assert_eq!(status_to_message(422), "Validierungsfehler");
        assert_eq!(
            status_to_message(429),
            "Zu viele Anfragen — bitte warten"
        );
        assert_eq!(
            status_to_message(500),
            "Serverfehler — bitte später erneut versuchen"
        );
        assert_eq!(
            status_to_message(502),
            "Serverfehler — bitte später erneut versuchen"
        );
    }

    #[test]
    fn test_status_to_message_unknown_code() {
        assert_eq!(status_to_message(418), "Unbekannter Fehler");
    }

    #[test]
    fn test_parse_415_with_extensions() {
        let body = r#"{"error":"File type 'exe' is not allowed","allowed_extensions":["pdf","png","jpg"]}"#;
        assert_eq!(
            parse_415_message(body),
            "Dateityp nicht erlaubt. Erlaubte Typen: pdf, png, jpg"
        );
    }

    #[test]
    fn test_parse_415_without_extensions() {
        let body = "Unsupported Media Type";
        assert_eq!(parse_415_message(body), "Dateityp nicht erlaubt");
    }

    #[test]
    fn test_parse_415_json_without_extensions_field() {
        let body = r#"{"error":"not allowed"}"#;
        assert_eq!(parse_415_message(body), "Dateityp nicht erlaubt");
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError::new(Some(404), "Nicht gefunden", None);
        assert_eq!(format!("{}", err), "Nicht gefunden");
    }

    #[test]
    fn test_app_error_display_with_detail() {
        let err = AppError::new(Some(500), "Serverfehler", Some("internal".into()));
        assert_eq!(format!("{}", err), "Serverfehler");
    }

    #[test]
    fn test_app_error_new() {
        let err = AppError::new(Some(422), "Validierungsfehler", Some("field invalid".into()));
        assert_eq!(err.status, Some(422));
        assert_eq!(err.message, "Validierungsfehler");
        assert_eq!(err.detail.as_deref(), Some("field invalid"));
    }

    #[test]
    fn test_app_error_new_no_detail() {
        let err = AppError::new(None, "Verbindungsfehler", None);
        assert_eq!(err.status, None);
        assert!(err.detail.is_none());
    }
}
