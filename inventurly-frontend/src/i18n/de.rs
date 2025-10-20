use super::Key;
use std::rc::Rc;

pub fn translate(key: Key) -> Rc<str> {
    match key {
        // General
        Key::AppTitle => "Inventurly",
        Key::Loading => "Lädt...",
        Key::Save => "Speichern",
        Key::Cancel => "Abbrechen",
        Key::Delete => "Löschen",
        Key::Edit => "Bearbeiten",
        Key::Create => "Erstellen",
        Key::Search => "Suchen",
        Key::Back => "Zurück",
        Key::Confirm => "Bestätigen",
        Key::Actions => "Aktionen",

        // Authentication
        Key::Login => "Anmelden",
        Key::Logout => "Abmelden",
        Key::Username => "Benutzername",
        Key::Password => "Passwort",
        Key::LoginFailed => "Anmeldung fehlgeschlagen",
        Key::NotAuthenticated => "Nicht angemeldet",
        Key::WelcomeTitle => "Willkommen bei Inventurly",
        Key::PleaseLogin => "Bitte melden Sie sich an, um fortzufahren",

        // Navigation
        Key::Home => "Startseite",
        Key::Products => "Produkte",
        Key::Racks => "Regale",
        Key::Containers => "Container",
        Key::Persons => "Personen",
        Key::Permissions => "Berechtigungen",

        // Product fields
        Key::ProductName => "Name",
        Key::ProductEan => "EAN",
        Key::ProductShortName => "Kurzname",
        Key::ProductSalesUnit => "Verkaufseinheit",
        Key::ProductPrice => "Preis",
        Key::ProductRequiresWeighing => "Wiegeartikel",
        Key::ProductCreated => "Erstellt",
        Key::ProductDeleted => "Gelöscht",

        // Rack fields
        Key::RackName => "Name",
        Key::RackDescription => "Beschreibung",
        Key::RackCreated => "Erstellt",
        Key::RackDeleted => "Gelöscht",

        // Container fields
        Key::ContainerName => "Name",
        Key::ContainerWeightGrams => "Gewicht (Gramm)",
        Key::ContainerDescription => "Beschreibung",
        Key::ContainerCreated => "Erstellt",
        Key::ContainerDeleted => "Gelöscht",

        // Product-Rack fields
        Key::ProductRackQuantity => "Menge",
        Key::ProductRackRelationship => "Produkt-Regal-Zuordnung",
        Key::AddProductToRack => "Produkt zum Regal hinzufügen",
        Key::RemoveProductFromRack => "Produkt aus Regal entfernen",
        Key::UpdateQuantity => "Menge aktualisieren",
        Key::SelectProduct => "Produkt auswählen",
        Key::SelectRack => "Regal auswählen",
        Key::Quantity => "Menge",
        Key::RacksForProduct => "Regale für Produkt",
        Key::ProductsInRack => "Produkte im Regal",

        // Person fields
        Key::PersonName => "Name",
        Key::PersonAge => "Alter",
        Key::PersonCreated => "Erstellt",
        Key::PersonDeleted => "Gelöscht",

        // Permission fields
        Key::PermissionName => "Name",
        Key::PermissionDescription => "Beschreibung",
        Key::PermissionCreated => "Erstellt",
        Key::PermissionDeleted => "Gelöscht",

        // Messages
        Key::NoDataFound => "Keine Daten gefunden",
        Key::ErrorLoadingData => "Fehler beim Laden der Daten",
        Key::ItemCreated => "Element erfolgreich erstellt",
        Key::ItemUpdated => "Element erfolgreich aktualisiert",
        Key::ItemDeleted => "Element erfolgreich gelöscht",
        Key::ConfirmDelete => "Sind Sie sicher, dass Sie dieses Element löschen möchten?",

        // CSV Import
        Key::CsvImport => "CSV-Import",
        Key::SelectFile => "Datei auswählen",
        Key::ImportButton => "Importieren",
        Key::ImportSuccess => "Import erfolgreich",
        Key::ImportError => "Import fehlgeschlagen",

        // Duplicate Detection
        Key::CheckDuplicates => "Auf Duplikate prüfen",
        Key::DuplicatesFound => "Mögliche Duplikate gefunden",
        Key::NoDuplicatesFound => "Keine Duplikate gefunden",
        Key::SimilarityScore => "Ähnlichkeitswert",
    }
    .into()
}
