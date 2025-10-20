use super::Key;
use std::rc::Rc;

pub fn translate(key: Key) -> Rc<str> {
    match key {
        // General
        Key::AppTitle => "Inventurly",
        Key::Loading => "Loading...",
        Key::Save => "Save",
        Key::Cancel => "Cancel",
        Key::Delete => "Delete",
        Key::Edit => "Edit",
        Key::Create => "Create",
        Key::Search => "Search",
        Key::Back => "Back",
        Key::Confirm => "Confirm",
        Key::Actions => "Actions",

        // Authentication
        Key::Login => "Login",
        Key::Logout => "Logout",
        Key::Username => "Username",
        Key::Password => "Password",
        Key::LoginFailed => "Login failed",
        Key::NotAuthenticated => "Not authenticated",
        Key::WelcomeTitle => "Welcome to Inventurly",
        Key::PleaseLogin => "Please login to continue",

        // Navigation
        Key::Home => "Home",
        Key::Products => "Products",
        Key::Racks => "Racks",
        Key::Containers => "Containers",
        Key::Persons => "Persons",
        Key::Permissions => "Permissions",

        // Product fields
        Key::ProductName => "Name",
        Key::ProductEan => "EAN",
        Key::ProductShortName => "Short Name",
        Key::ProductSalesUnit => "Sales Unit",
        Key::ProductPrice => "Price",
        Key::ProductRequiresWeighing => "Requires Weighing",
        Key::ProductCreated => "Created",
        Key::ProductDeleted => "Deleted",

        // Rack fields
        Key::RackName => "Name",
        Key::RackDescription => "Description",
        Key::RackCreated => "Created",
        Key::RackDeleted => "Deleted",

        // Container fields
        Key::ContainerName => "Name",
        Key::ContainerWeightGrams => "Weight (grams)",
        Key::ContainerDescription => "Description",
        Key::ContainerCreated => "Created",
        Key::ContainerDeleted => "Deleted",

        // Product-Rack fields
        Key::ProductRackQuantity => "Quantity",
        Key::ProductRackRelationship => "Product-Rack Relationship",
        Key::AddProductToRack => "Add Product to Rack",
        Key::RemoveProductFromRack => "Remove Product from Rack",
        Key::UpdateQuantity => "Update Quantity",
        Key::SelectProduct => "Select Product",
        Key::SelectRack => "Select Rack",
        Key::Quantity => "Quantity",
        Key::RacksForProduct => "Racks for Product",
        Key::ProductsInRack => "Products in Rack",

        // Person fields
        Key::PersonName => "Name",
        Key::PersonAge => "Age",
        Key::PersonCreated => "Created",
        Key::PersonDeleted => "Deleted",

        // Permission fields
        Key::PermissionName => "Name",
        Key::PermissionDescription => "Description",
        Key::PermissionCreated => "Created",
        Key::PermissionDeleted => "Deleted",

        // Messages
        Key::NoDataFound => "No data found",
        Key::ErrorLoadingData => "Error loading data",
        Key::ItemCreated => "Item created successfully",
        Key::ItemUpdated => "Item updated successfully",
        Key::ItemDeleted => "Item deleted successfully",
        Key::ConfirmDelete => "Are you sure you want to delete this item?",

        // CSV Import
        Key::CsvImport => "CSV Import",
        Key::SelectFile => "Select File",
        Key::ImportButton => "Import",
        Key::ImportSuccess => "Import successful",
        Key::ImportError => "Import failed",

        // Duplicate Detection
        Key::CheckDuplicates => "Check for Duplicates",
        Key::DuplicatesFound => "Potential duplicates found",
        Key::NoDuplicatesFound => "No duplicates found",
        Key::SimilarityScore => "Similarity Score",
    }
    .into()
}
