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
        Key::AccessDenied => "Access Denied",
        Key::InsufficientPrivileges => "You do not have sufficient privileges to access this page.",
        Key::BackToHome => "Back to Home",
        Key::InventurTokenLoginTitle => "Inventory Login",
        Key::EnterYourName => "Enter your name",
        Key::NamePlaceholder => "Your name",
        Key::InventurTokenLoginFailed => "Login failed. Please check your name and try again.",

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

        // Product filtering
        Key::FilterProducts => "Filter by name, EAN, or short name...",
        Key::ClearFilter => "Clear filter",
        Key::NoProductsFound => "No products found matching your filter",
        Key::ScanBarcode => "Scan Barcode",
        Key::SearchingForEAN => "Searching for EAN",
        Key::MoreFilters => "More Filters",
        Key::ShowFilters => "Show Filters",
        Key::HideFilters => "Hide Filters",
        Key::MinimumPrice => "Minimum Price",
        Key::MaximumPrice => "Maximum Price",
        Key::Yes => "Yes",
        Key::No => "No",
        Key::Both => "Both",
        Key::ClearAllFilters => "Clear All Filters",
        Key::SelectAll => "Select All",
        Key::DeselectAll => "Deselect All",
        Key::RackAssignment => "Rack Assignment",
        Key::Assigned => "Assigned",
        Key::Unassigned => "Unassigned",

        // Rack fields
        Key::RackName => "Name",
        Key::RackDescription => "Description",
        Key::RackCreated => "Created",
        Key::RackDeleted => "Deleted",

        // Container fields
        Key::Container => "Container",
        Key::ContainerName => "Name",
        Key::ContainerWeightGrams => "Weight (grams)",
        Key::ContainerDescription => "Description",
        Key::ContainerCreated => "Created",
        Key::ContainerDeleted => "Deleted",

        // Product-Rack fields
        Key::ProductRackQuantity => "Quantity",
        Key::ProductRackRelationship => "Product-Rack Relationship",
        Key::AddProductToRack => "Add Product to Rack",
        Key::RemoveProductFromRack => "Remove",
        Key::UpdateQuantity => "Update Quantity",
        Key::SelectProduct => "Select Product",
        Key::SelectRack => "Select Rack",
        Key::Quantity => "Quantity",
        Key::RacksForProduct => "Racks for Product",
        Key::ProductsInRack => "Products in Rack",
        Key::MoveUp => "Move Up",
        Key::MoveDown => "Move Down",
        Key::MoveAbove => "Move here",
        Key::MoveBelow => "Move here",
        Key::Order => "Order",

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
        Key::ImportInProgress => "Importing... This may take several minutes.",
        Key::RemoveUnlisted => "Remove unlisted products",
        Key::RemoveUnlistedDescription => "Delete products from the database that are not in the CSV file",
        Key::ProductsCreated => "Created",
        Key::ProductsUpdated => "Updated",
        Key::ProductsDeleted => "Deleted",
        Key::ImportErrors => "Errors",

        // Duplicate Detection
        Key::CheckDuplicates => "Check for Duplicates",
        Key::DuplicatesFound => "Potential duplicates found",
        Key::NoDuplicatesFound => "No duplicates found",
        Key::SimilarityScore => "Similarity Score",
        
        // Duplicate Detection Page
        Key::DetectionMode => "Detection Mode",
        Key::ScanAllProducts => "Scan All Products",
        Key::CheckSpecificProduct => "Check Specific Product",
        Key::CheckNewProduct => "Check New Product",
        Key::DetectionSettings => "Detection Settings",
        Key::SimilarityThreshold => "Similarity Threshold",
        Key::ExactMatchWeight => "Exact Match Weight",
        Key::WordOrderWeight => "Word Order Weight",
        Key::LevenshteinWeight => "Levenshtein Weight",
        Key::JaroWinklerWeight => "Jaro-Winkler Weight",
        Key::CategoryAware => "Category-aware matching",
        Key::CategoryAwareDescription => "Category-aware matching (considers sales unit and weighing requirements)",
        Key::ResetToDefaults => "Reset to Defaults",
        Key::ThresholdDescription => "Lower values find more potential duplicates, higher values are more conservative",
        
        // Duplicate Detection Actions
        Key::StartScan => "Start Scan",
        Key::Scanning => "Scanning...",
        Key::CheckProduct => "Check Product",
        Key::Checking => "Checking...",
        Key::CheckForDuplicates => "Check for Duplicates",
        Key::Processing => "Processing...",
        
        // Duplicate Detection Messages
        Key::NoScanPerformed => "No scan performed yet",
        Key::ClickStartScanDescription => "Click 'Start Scan' to find all duplicates in your product database",
        Key::NoProductChecked => "No product checked yet",
        Key::SelectProductDescription => "Select a product and click 'Check Product' to find duplicates",
        Key::EnterProductDescription => "Enter product details and click 'Check for Duplicates'",
        
        // Product Form
        Key::ProductNamePlaceholder => "Product name",
        Key::SalesUnitPlaceholder => "Sales unit (e.g., 100g)",
        Key::RequiresWeighing => "Requires weighing",
        Key::SelectProductOption => "Select a product...",
        
        // Algorithm Names
        Key::ExactMatch => "Exact Match",
        Key::WordOrder => "Word Order",
        Key::Levenshtein => "Levenshtein",
        Key::JaroWinkler => "Jaro-Winkler",
        Key::Category => "Category",
        Key::AlgorithmBreakdown => "Algorithm Breakdown",
        
        // Confidence Levels
        Key::VeryHigh => "Very High",
        Key::High => "High",
        Key::Medium => "Medium",
        Key::Low => "Low",
        
        // Actions
        Key::ViewProduct => "View Product",
        Key::SuggestMerge => "Suggest Merge",
        
        // Results
        Key::PotentialDuplicateMatches => "Potential Duplicate Matches",
        Key::OriginalProduct => "Original Product",
        Key::PotentialDuplicatesFound => "potential duplicates found",
        
        // Page Description
        Key::DuplicateDetectionDescription => "Find potential duplicate products using advanced similarity algorithms",
        
        // Expandable UI
        Key::ShowDetails => "Show Details",
        Key::HideDetails => "Hide Details",
        Key::ExpandAll => "Expand All",
        Key::CollapseAll => "Collapse All",
        Key::Summary => "Summary",
        Key::Details => "Details",

        // Inventur
        Key::Inventurs => "Inventories",
        Key::Inventur => "Inventory",
        Key::InventurDetails => "Inventory Details",
        Key::CreateInventur => "Create Inventory",
        Key::EditInventur => "Edit Inventory",
        Key::DeleteInventur => "Delete Inventory",
        Key::ViewMeasurements => "View Measurements",
        Key::RecordMeasurement => "Record Measurement",
        Key::StartInventur => "Start Inventory",
        Key::CompleteInventur => "Complete Inventory",
        Key::CancelInventur => "Cancel Inventory",

        // Inventur fields
        Key::InventurName => "Name",
        Key::InventurDescription => "Description",
        Key::InventurStartDate => "Start Date",
        Key::InventurEndDate => "End Date",
        Key::InventurStatus => "Status",
        Key::InventurCreatedBy => "Created By",

        // Inventur status
        Key::StatusDraft => "Draft",
        Key::StatusActive => "Active",
        Key::StatusCompleted => "Completed",
        Key::StatusCancelled => "Cancelled",

        // Measurements
        Key::Measurements => "Measurements",
        Key::MeasurementCount => "Count",
        Key::MeasurementWeight => "Weight",
        Key::MeasurementWeightGrams => "Weight (grams)",
        Key::MeasurementNotes => "Notes",
        Key::MeasuredBy => "Measured By",
        Key::MeasuredAt => "Measured At",
        Key::NoMeasurementsFound => "No measurements found",
        Key::NoInventursFound => "No inventories found",

        // Date filtering
        Key::FilterByDate => "Filter by Date",
        Key::DateFrom => "From",
        Key::DateTo => "To",

        // Rack measurement
        Key::MeasureRack => "Measure Rack",
        Key::ProductsMeasured => "Products Measured",
        Key::NotMeasured => "Not Measured",
        Key::Measured => "Measured",
        Key::QuickMeasure => "Quick Measure",
        Key::MeasurementProgress => "Measurement Progress",
        Key::ViewRackProgress => "View Rack Progress",
        Key::EnterCount => "Enter Count",
        Key::EnterWeight => "Enter Weight",

        // Rack selection
        Key::MeasureByRack => "Measure by Rack",
        Key::ProductCount => "Products",
        Key::NotStarted => "Not Started",
        Key::InProgress => "In Progress",
        Key::Complete => "Complete",
        Key::NoProductsInRack => "No products in this rack",

        // Custom entries
        Key::CustomEntries => "Custom Entries",
        Key::AddCustomEntry => "Add Custom Entry",
        Key::EditCustomEntry => "Edit Custom Entry",
        Key::CustomProductName => "Product Name",
        Key::Count => "Count",
        Key::WeightGrams => "Weight (grams)",
        Key::Notes => "Notes",
        Key::DeleteCustomEntry => "Delete",
        Key::ConfirmDeleteCustomEntry => "Confirm Delete",
        Key::CustomEntry => "Custom Entry",
        Key::ScanToSelectProduct => "Scan to select product",
        Key::ProductNotFound => "Product not found",

        // QR Codes
        Key::PrintQRCodes => "Print QR Codes",
        Key::ScanToLogin => "Scan to Login",
        Key::ScanToMeasure => "Scan to Measure",
        Key::LoginQRCode => "Login QR Code",
        Key::RackQRCode => "Rack QR Code",
        Key::Rack => "Rack",

        // Custom Tara
        Key::CustomTara => "Custom Tara",
        Key::TaraWeight => "Tara Weight",
        Key::TaraDescription => "Set a custom tara (e.g., body weight) that will be automatically subtracted from all weight measurements. This value is stored locally in your browser and never sent to the server.",
        Key::TaraHint => "Leave empty to clear the tara",
        Key::CurrentTara => "Current Tara:",
        Key::ClearTara => "Clear Tara",
    }
    .into()
}
