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
        Key::AccessDenied => "Zugriff verweigert",
        Key::InsufficientPrivileges => "Sie haben nicht die erforderlichen Berechtigungen für diese Seite.",
        Key::BackToHome => "Zur Startseite",

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

        // Product filtering
        Key::FilterProducts => "Nach Name, EAN oder Kurzname filtern...",
        Key::ClearFilter => "Filter zurücksetzen",
        Key::NoProductsFound => "Keine Produkte gefunden, die Ihren Filter entsprechen",
        Key::ScanBarcode => "Barcode scannen",
        Key::SearchingForEAN => "Suche nach EAN",
        Key::MoreFilters => "Mehr Filter",
        Key::ShowFilters => "Filter anzeigen",
        Key::HideFilters => "Filter ausblenden",
        Key::MinimumPrice => "Mindestpreis",
        Key::MaximumPrice => "Höchstpreis",
        Key::Yes => "Ja",
        Key::No => "Nein",
        Key::Both => "Beide",
        Key::ClearAllFilters => "Alle Filter zurücksetzen",
        Key::SelectAll => "Alle auswählen",
        Key::DeselectAll => "Alle abwählen",

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
        
        // Duplicate Detection Page
        Key::DetectionMode => "Erkennungsmodus",
        Key::ScanAllProducts => "Alle Produkte scannen",
        Key::CheckSpecificProduct => "Bestimmtes Produkt prüfen",
        Key::CheckNewProduct => "Neues Produkt prüfen",
        Key::DetectionSettings => "Erkennungseinstellungen",
        Key::SimilarityThreshold => "Ähnlichkeitsschwellenwert",
        Key::ExactMatchWeight => "Gewichtung Exakte Übereinstimmung",
        Key::WordOrderWeight => "Gewichtung Wortreihenfolge",
        Key::LevenshteinWeight => "Gewichtung Levenshtein",
        Key::JaroWinklerWeight => "Gewichtung Jaro-Winkler",
        Key::CategoryAware => "Kategoriebasierte Erkennung",
        Key::CategoryAwareDescription => "Kategoriebasierte Übereinstimmung (berücksichtigt Verkaufseinheit und Wiegeerfordernis)",
        Key::ResetToDefaults => "Auf Standardwerte zurücksetzen",
        Key::ThresholdDescription => "Niedrigere Werte finden mehr potentielle Duplikate, höhere Werte sind konservativer",
        
        // Duplicate Detection Actions
        Key::StartScan => "Scan starten",
        Key::Scanning => "Scannen...",
        Key::CheckProduct => "Produkt prüfen",
        Key::Checking => "Prüfen...",
        Key::CheckForDuplicates => "Auf Duplikate prüfen",
        Key::Processing => "Verarbeitung...",
        
        // Duplicate Detection Messages
        Key::NoScanPerformed => "Noch kein Scan durchgeführt",
        Key::ClickStartScanDescription => "Klicken Sie auf 'Scan starten', um alle Duplikate in Ihrer Produktdatenbank zu finden",
        Key::NoProductChecked => "Noch kein Produkt geprüft",
        Key::SelectProductDescription => "Wählen Sie ein Produkt aus und klicken Sie auf 'Produkt prüfen', um Duplikate zu finden",
        Key::EnterProductDescription => "Geben Sie Produktdetails ein und klicken Sie auf 'Auf Duplikate prüfen'",
        
        // Product Form
        Key::ProductNamePlaceholder => "Produktname",
        Key::SalesUnitPlaceholder => "Verkaufseinheit (z.B. 100g)",
        Key::RequiresWeighing => "Wiegeartikel",
        Key::SelectProductOption => "Produkt auswählen...",
        
        // Algorithm Names
        Key::ExactMatch => "Exakte Übereinstimmung",
        Key::WordOrder => "Wortreihenfolge",
        Key::Levenshtein => "Levenshtein",
        Key::JaroWinkler => "Jaro-Winkler",
        Key::Category => "Kategorie",
        Key::AlgorithmBreakdown => "Algorithmus-Aufschlüsselung",
        
        // Confidence Levels
        Key::VeryHigh => "Sehr hoch",
        Key::High => "Hoch",
        Key::Medium => "Mittel",
        Key::Low => "Niedrig",
        
        // Actions
        Key::ViewProduct => "Produkt anzeigen",
        Key::SuggestMerge => "Zusammenführung vorschlagen",
        
        // Results
        Key::PotentialDuplicateMatches => "Potentielle Duplikatsübereinstimmungen",
        Key::OriginalProduct => "Originalprodukt",
        Key::PotentialDuplicatesFound => "potentielle Duplikate gefunden",
        
        // Page Description
        Key::DuplicateDetectionDescription => "Finden Sie potentielle Duplikatsprodukte mit fortschrittlichen Ähnlichkeitsalgorithmen",
        
        // Expandable UI
        Key::ShowDetails => "Details anzeigen",
        Key::HideDetails => "Details ausblenden",
        Key::ExpandAll => "Alle erweitern",
        Key::CollapseAll => "Alle einklappen",
        Key::Summary => "Zusammenfassung",
        Key::Details => "Details",
    }
    .into()
}
