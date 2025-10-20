pub mod de;
pub mod en;

use dioxus::prelude::*;
use std::rc::Rc;
use web_sys;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    De,
}

impl Default for Locale {
    fn default() -> Self {
        Self::En
    }
}

fn detect_browser_locale() -> Locale {
    // Try to detect browser language preference
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();

        // First try the primary language
        if let Some(language) = navigator.language() {
            if is_german_language(&language) {
                return Locale::De;
            }
        }

        // Then try the languages array for broader preferences
        let languages = navigator.languages();
        for i in 0..languages.length() {
            if let Some(lang) = languages.get(i).as_string() {
                if is_german_language(&lang) {
                    return Locale::De;
                }
            }
        }
    }

    // Default fallback to English
    Locale::En
}

fn is_german_language(lang: &str) -> bool {
    let lang_lower = lang.to_lowercase();
    lang_lower == "de"
        || lang_lower.starts_with("de-")
        || lang_lower == "de-de"
        || lang_lower == "de-at"
        || lang_lower == "de-ch"
}

#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    // General
    AppTitle,
    Loading,
    Save,
    Cancel,
    Delete,
    Edit,
    Create,
    Search,
    Back,
    Confirm,
    Actions,

    // Authentication
    Login,
    Logout,
    Username,
    Password,
    LoginFailed,
    NotAuthenticated,
    WelcomeTitle,
    PleaseLogin,

    // Navigation
    Home,
    Products,
    Racks,
    Containers,
    Persons,
    Permissions,

    // Product fields
    ProductName,
    ProductEan,
    ProductShortName,
    ProductSalesUnit,
    ProductPrice,
    ProductRequiresWeighing,
    ProductCreated,
    ProductDeleted,

    // Rack fields
    RackName,
    RackDescription,
    RackCreated,
    RackDeleted,

    // Container fields
    ContainerName,
    ContainerWeightGrams,
    ContainerDescription,
    ContainerCreated,
    ContainerDeleted,

    // Product-Rack fields
    ProductRackQuantity,
    ProductRackRelationship,
    AddProductToRack,
    RemoveProductFromRack,
    UpdateQuantity,
    SelectProduct,
    SelectRack,
    Quantity,
    RacksForProduct,
    ProductsInRack,

    // Person fields
    PersonName,
    PersonAge,
    PersonCreated,
    PersonDeleted,

    // Permission fields
    PermissionName,
    PermissionDescription,
    PermissionCreated,
    PermissionDeleted,

    // Messages
    NoDataFound,
    ErrorLoadingData,
    ItemCreated,
    ItemUpdated,
    ItemDeleted,
    ConfirmDelete,

    // CSV Import
    CsvImport,
    SelectFile,
    ImportButton,
    ImportSuccess,
    ImportError,

    // Duplicate Detection
    CheckDuplicates,
    DuplicatesFound,
    NoDuplicatesFound,
    SimilarityScore,
}

pub struct I18n {
    locale: Locale,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn t(&self, key: Key) -> Rc<str> {
        match self.locale {
            Locale::En => en::translate(key),
            Locale::De => de::translate(key),
        }
    }

    #[allow(dead_code)]
    pub fn format_date(&self, date: time::Date) -> String {
        match self.locale {
            Locale::En => {
                format!(
                    "{:04}-{:02}-{:02}",
                    date.year(),
                    date.month() as u8,
                    date.day()
                )
            }
            Locale::De => {
                format!(
                    "{:02}.{:02}.{:04}",
                    date.day(),
                    date.month() as u8,
                    date.year()
                )
            }
        }
    }

    pub fn format_datetime(&self, datetime: time::PrimitiveDateTime) -> String {
        match self.locale {
            Locale::En => {
                let date = datetime.date();
                let time = datetime.time();
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    date.year(),
                    date.month() as u8,
                    date.day(),
                    time.hour(),
                    time.minute(),
                    time.second()
                )
            }
            Locale::De => {
                let date = datetime.date();
                let time = datetime.time();
                format!(
                    "{:02}.{:02}.{:04} {:02}:{:02}:{:02}",
                    date.day(),
                    date.month() as u8,
                    date.year(),
                    time.hour(),
                    time.minute(),
                    time.second()
                )
            }
        }
    }

    pub fn format_price(&self, cents: i64) -> String {
        let euros = cents as f64 / 100.0;
        match self.locale {
            Locale::En => format!("€{:.2}", euros),
            Locale::De => format!("{:.2} €", euros).replace('.', ","),
        }
    }
}

impl Clone for I18n {
    fn clone(&self) -> Self {
        Self {
            locale: self.locale,
        }
    }
}

static I18N: GlobalSignal<I18n> = GlobalSignal::new(|| I18n::new(detect_browser_locale()));

pub fn use_i18n() -> I18n {
    I18N.read().clone()
}
