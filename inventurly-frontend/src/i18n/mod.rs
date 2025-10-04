pub mod en;

use std::rc::Rc;
use dioxus::prelude::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
}

impl Default for Locale {
    fn default() -> Self {
        Self::En
    }
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
        }
    }
    
    pub fn format_date(&self, date: time::Date) -> String {
        match self.locale {
            Locale::En => {
                format!("{:04}-{:02}-{:02}", date.year(), date.month() as u8, date.day())
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
        }
    }
    
    pub fn format_price(&self, cents: i64) -> String {
        let euros = cents as f64 / 100.0;
        match self.locale {
            Locale::En => format!("€{:.2}", euros),
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

static I18N: GlobalSignal<I18n> = GlobalSignal::new(|| I18n::new(Locale::En));

pub fn use_i18n() -> I18n {
    I18N.read().clone()
}