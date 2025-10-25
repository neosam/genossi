use std::rc::Rc;

use rest_types::{
    AddProductToRackRequestTO, CheckDuplicateRequestTO, ContainerTO, DuplicateDetectionResultTO,
    DuplicateMatchTO, ProductRackTO, ProductTO, RackTO, UserTO,
};
use tracing::info;
use uuid::Uuid;

use crate::state::{AuthInfo, Config};

// Authentication API
pub async fn fetch_auth_info(backend_url: Rc<str>) -> Result<Option<AuthInfo>, reqwest::Error> {
    info!("Fetching auth info");
    let response = reqwest::get(format!("{}/auth/info", backend_url)).await?;
    if response.status() != 200 {
        return Ok(None);
    }
    let user: UserTO = response.json().await?;
    let auth_info = AuthInfo {
        user: user.username.into(),
        roles: user.roles.into_iter().map(|r| r.into()).collect(),
        privileges: user.privileges.into_iter().map(|p| p.into()).collect(),
        authenticated: true,
    };
    info!("Auth info fetched");
    Ok(Some(auth_info))
}

#[allow(dead_code)]
pub async fn login(
    backend_url: Rc<str>,
    username: &str,
    password: &str,
) -> Result<bool, reqwest::Error> {
    info!("Logging in user: {}", username);
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/login", backend_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await?;

    Ok(response.status() == 200)
}

#[allow(dead_code)]
pub async fn logout(backend_url: Rc<str>) -> Result<(), reqwest::Error> {
    info!("Logging out");
    let client = reqwest::Client::new();
    client
        .post(format!("{}/auth/logout", backend_url))
        .send()
        .await?;
    Ok(())
}

// Config API
pub async fn load_config() -> Result<Config, reqwest::Error> {
    info!("Loading config.json");
    let protocol = web_sys::window()
        .expect("no window")
        .location()
        .protocol()
        .expect("no protocol");
    let host = web_sys::window()
        .expect("no window")
        .location()
        .host()
        .expect("no host");
    let url = format!("{protocol}//{host}/assets/config.json");
    info!("Config URL: {url}");
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res: Config = response.json().await?;
    info!("Config loaded");
    Ok(res)
}

// Product API
pub async fn get_products(config: &Config) -> Result<Vec<ProductTO>, reqwest::Error> {
    info!("Fetching products");
    let url = format!("{}/products", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Products fetched");
    Ok(res)
}

#[allow(dead_code)]
pub async fn get_product(config: &Config, id: Uuid) -> Result<ProductTO, reqwest::Error> {
    info!("Fetching product {id}");
    let url = format!("{}/products/{}", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product fetched");
    Ok(res)
}

#[allow(dead_code)]
pub async fn create_product(
    config: &Config,
    product: ProductTO,
) -> Result<ProductTO, reqwest::Error> {
    info!("Creating product");
    let url = format!("{}/products", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&product).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product created");
    Ok(res)
}

#[allow(dead_code)]
pub async fn update_product(
    config: &Config,
    product: ProductTO,
) -> Result<ProductTO, reqwest::Error> {
    info!("Updating product {:?}", product.id);
    let url = format!("{}/products/{}", config.backend, product.id.unwrap());
    let client = reqwest::Client::new();
    let response = client.put(url).json(&product).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product updated");
    Ok(res)
}

#[allow(dead_code)]
pub async fn delete_product(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting product {id}");
    let url = format!("{}/products/{}", config.backend, id);
    let client = reqwest::Client::new();
    let response = client.delete(url).send().await?;
    response.error_for_status_ref()?;
    info!("Product deleted");
    Ok(())
}

// Search products
pub async fn search_products(
    config: &Config,
    query: &str,
) -> Result<Vec<ProductTO>, reqwest::Error> {
    info!("Searching products: {query}");
    let url = format!("{}/products/search?q={}", config.backend, query);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Search completed");
    Ok(res)
}

// Duplicate detection
#[allow(dead_code)]
pub async fn check_duplicates(
    config: &Config,
    request: CheckDuplicateRequestTO,
) -> Result<Vec<DuplicateMatchTO>, reqwest::Error> {
    info!("Checking for duplicate products");
    let url = format!("{}/duplicate-detection/check", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&request).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Duplicate check completed");
    Ok(res)
}

#[allow(dead_code)]
pub async fn find_all_duplicates(
    config: &Config,
) -> Result<Vec<DuplicateDetectionResultTO>, reqwest::Error> {
    info!("Finding all duplicate products");
    let url = format!("{}/duplicate-detection/products", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Find all duplicates completed");
    Ok(res)
}

#[allow(dead_code)]
pub async fn find_duplicates_by_ean(
    config: &Config,
    ean: &str,
) -> Result<DuplicateDetectionResultTO, reqwest::Error> {
    info!("Finding duplicates for product: {ean}");
    let url = format!("{}/duplicate-detection/products/{}", config.backend, ean);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Find duplicates by EAN completed");
    Ok(res)
}

// CSV Import
#[allow(dead_code)]
pub async fn import_csv(
    config: &Config,
    csv_data: String,
) -> Result<Vec<ProductTO>, reqwest::Error> {
    info!("Importing CSV data");
    let url = format!("{}/csv-import/products", config.backend);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "text/csv")
        .body(csv_data)
        .send()
        .await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("CSV import completed");
    Ok(res)
}

// Rack API
pub async fn get_racks(config: &Config) -> Result<Vec<RackTO>, reqwest::Error> {
    info!("Fetching racks");
    let url = format!("{}/racks", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Racks fetched");
    Ok(res)
}

pub async fn get_rack(config: &Config, id: Uuid) -> Result<RackTO, reqwest::Error> {
    info!("Fetching rack {id}");
    let url = format!("{}/racks/{}", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Rack fetched");
    Ok(res)
}

pub async fn create_rack(config: &Config, rack: RackTO) -> Result<RackTO, reqwest::Error> {
    info!("Creating rack");
    let url = format!("{}/racks", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&rack).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Rack created");
    Ok(res)
}

pub async fn update_rack(config: &Config, rack: RackTO) -> Result<RackTO, reqwest::Error> {
    info!("Updating rack {:?}", rack.id);
    let url = format!("{}/racks/{}", config.backend, rack.id.unwrap());
    let client = reqwest::Client::new();
    let response = client.put(url).json(&rack).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Rack updated");
    Ok(res)
}

pub async fn delete_rack(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting rack {id}");
    let url = format!("{}/racks/{}", config.backend, id);
    let client = reqwest::Client::new();
    let response = client.delete(url).send().await?;
    response.error_for_status_ref()?;
    info!("Rack deleted");
    Ok(())
}

// Container API
pub async fn get_containers(config: &Config) -> Result<Vec<ContainerTO>, reqwest::Error> {
    info!("Fetching containers");
    let url = format!("{}/containers", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Containers fetched");
    Ok(res)
}

pub async fn get_container(config: &Config, id: Uuid) -> Result<ContainerTO, reqwest::Error> {
    info!("Fetching container {id}");
    let url = format!("{}/containers/{}", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Container fetched");
    Ok(res)
}

pub async fn create_container(
    config: &Config,
    container: ContainerTO,
) -> Result<ContainerTO, reqwest::Error> {
    info!("Creating container");
    let url = format!("{}/containers", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&container).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Container created");
    Ok(res)
}

pub async fn update_container(
    config: &Config,
    container: ContainerTO,
) -> Result<ContainerTO, reqwest::Error> {
    info!("Updating container {:?}", container.id);
    let url = format!("{}/containers/{}", config.backend, container.id.unwrap());
    let client = reqwest::Client::new();
    let response = client.put(url).json(&container).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Container updated");
    Ok(res)
}

pub async fn delete_container(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting container {id}");
    let url = format!("{}/containers/{}", config.backend, id);
    let client = reqwest::Client::new();
    let response = client.delete(url).send().await?;
    response.error_for_status_ref()?;
    info!("Container deleted");
    Ok(())
}

pub async fn search_containers(
    config: &Config,
    query: &str,
) -> Result<Vec<ContainerTO>, reqwest::Error> {
    info!("Searching containers: {query}");
    let url = format!("{}/containers?q={}", config.backend, query);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Container search completed");
    Ok(res)
}

// Product-Rack API
pub async fn add_product_to_rack(
    config: &Config,
    product_id: Uuid,
    rack_id: Uuid,
) -> Result<ProductRackTO, reqwest::Error> {
    info!("Adding product {product_id} to rack {rack_id}");
    let url = format!("{}/product-racks", config.backend);
    let request = AddProductToRackRequestTO {
        product_id,
        rack_id,
    };
    let client = reqwest::Client::new();
    let response = client.post(url).json(&request).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product added to rack");
    Ok(res)
}

pub async fn remove_product_from_rack(
    config: &Config,
    product_id: Uuid,
    rack_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Removing product {product_id} from rack {rack_id}");
    let url = format!(
        "{}/product-racks/{}/{}",
        config.backend, product_id, rack_id
    );
    let client = reqwest::Client::new();
    let response = client.delete(url).send().await?;
    response.error_for_status_ref()?;
    info!("Product removed from rack");
    Ok(())
}

pub async fn get_product_rack_relationship(
    config: &Config,
    product_id: Uuid,
    rack_id: Uuid,
) -> Result<Option<ProductRackTO>, reqwest::Error> {
    info!("Fetching product-rack relationship {product_id}-{rack_id}");
    let url = format!(
        "{}/product-racks/{}/{}",
        config.backend, product_id, rack_id
    );
    let response = reqwest::get(url).await?;
    if response.status() == 404 {
        return Ok(None);
    }
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product-rack relationship fetched");
    Ok(Some(res))
}

pub async fn get_racks_for_product(
    config: &Config,
    product_id: Uuid,
) -> Result<Vec<ProductRackTO>, reqwest::Error> {
    info!("Fetching racks for product {product_id}");
    let url = format!("{}/product-racks/product/{}", config.backend, product_id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Racks for product fetched");
    Ok(res)
}

pub async fn get_products_in_rack(
    config: &Config,
    rack_id: Uuid,
) -> Result<Vec<ProductRackTO>, reqwest::Error> {
    info!("Fetching products in rack {rack_id}");
    let url = format!("{}/product-racks/rack/{}", config.backend, rack_id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Products in rack fetched");
    Ok(res)
}

#[allow(dead_code)]
pub async fn get_all_product_rack_relationships(
    config: &Config,
) -> Result<Vec<ProductRackTO>, reqwest::Error> {
    info!("Fetching all product-rack relationships");
    let url = format!("{}/product-racks/all", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("All product-rack relationships fetched");
    Ok(res)
}
