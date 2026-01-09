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
        Key::InventurTokenLoginTitle => "Inventur-Anmeldung",
        Key::EnterYourName => "Geben Sie Ihren Namen ein",
        Key::NamePlaceholder => "Ihr Name",
        Key::InventurTokenLoginFailed => "Anmeldung fehlgeschlagen. Bitte überprüfen Sie Ihren Namen und versuchen Sie es erneut.",

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
        Key::RackAssignment => "Regal-Zuweisung",
        Key::Assigned => "Zugewiesen",
        Key::Unassigned => "Nicht zugewiesen",

        // Rack fields
        Key::RackName => "Name",
        Key::RackDescription => "Beschreibung",
        Key::RackCreated => "Erstellt",
        Key::RackDeleted => "Gelöscht",

        // Container fields
        Key::Container => "Gefäß",
        Key::ContainerName => "Name",
        Key::ContainerWeightGrams => "Gewicht (Gramm)",
        Key::ContainerDescription => "Beschreibung",
        Key::ContainerCreated => "Erstellt",
        Key::ContainerDeleted => "Gelöscht",

        // Product-Rack fields
        Key::ProductRackQuantity => "Menge",
        Key::ProductRackRelationship => "Produkt-Regal-Zuordnung",
        Key::AddProductToRack => "Produkt zum Regal hinzufügen",
        Key::RemoveProductFromRack => "Entfernen",
        Key::UpdateQuantity => "Menge aktualisieren",
        Key::SelectProduct => "Produkt auswählen",
        Key::SelectRack => "Regal auswählen",
        Key::Quantity => "Menge",
        Key::RacksForProduct => "Regale für Produkt",
        Key::ProductsInRack => "Produkte im Regal",
        Key::MoveUp => "Nach oben",
        Key::MoveDown => "Nach unten",
        Key::MoveAbove => "Hierher",
        Key::MoveBelow => "Hierher",
        Key::Order => "Reihenfolge",

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
        Key::ImportInProgress => "Importieren... Dies kann einige Minuten dauern.",
        Key::RemoveUnlisted => "Nicht aufgelistete Produkte entfernen",
        Key::RemoveUnlistedDescription => "Produkte aus der Datenbank löschen, die nicht in der CSV-Datei enthalten sind",
        Key::ProductsCreated => "Erstellt",
        Key::ProductsUpdated => "Aktualisiert",
        Key::ProductsDeleted => "Gelöscht",
        Key::ImportErrors => "Fehler",

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

        // Inventur
        Key::Inventurs => "Inventuren",
        Key::Inventur => "Inventur",
        Key::InventurDetails => "Inventur-Details",
        Key::CreateInventur => "Inventur erstellen",
        Key::EditInventur => "Inventur bearbeiten",
        Key::DeleteInventur => "Inventur löschen",
        Key::ViewMeasurements => "Erfassungen anzeigen",
        Key::RecordMeasurement => "Erfassen",
        Key::StartInventur => "Inventur starten",
        Key::CompleteInventur => "Inventur abschließen",
        Key::CancelInventur => "Inventur abbrechen",

        // Inventur fields
        Key::InventurName => "Name",
        Key::InventurDescription => "Beschreibung",
        Key::InventurStartDate => "Startdatum",
        Key::InventurEndDate => "Enddatum",
        Key::InventurStatus => "Status",
        Key::InventurCreatedBy => "Erstellt von",

        // Inventur status
        Key::StatusDraft => "Entwurf",
        Key::StatusActive => "Aktiv",
        Key::StatusCompleted => "Abgeschlossen",
        Key::StatusCancelled => "Abgebrochen",

        // Measurements
        Key::Measurements => "Erfassungen",
        Key::MeasurementCount => "Anzahl",
        Key::MeasurementWeight => "Gewicht",
        Key::MeasurementWeightGrams => "Gewicht (Gramm)",
        Key::MeasurementNotes => "Notizen",
        Key::MeasuredBy => "Erfasst von",
        Key::MeasuredAt => "Erfasst am",
        Key::NoMeasurementsFound => "Keine Erfassung gefunden",
        Key::NoInventursFound => "Keine Inventuren gefunden",

        // Date filtering
        Key::FilterByDate => "Nach Datum filtern",
        Key::DateFrom => "Von",
        Key::DateTo => "Bis",

        // Rack measurement
        Key::MeasureRack => "Regal erfassen",
        Key::ProductsMeasured => "Produkte erfasst",
        Key::NotMeasured => "Nicht erfasst",
        Key::Measured => "Erfasst",
        Key::QuickMeasure => "Schnellerfassung",
        Key::MeasurementProgress => "Erfassungsfortschritt",
        Key::ViewRackProgress => "Regalfortschritt anzeigen",
        Key::EnterCount => "Anzahl eingeben",
        Key::EnterWeight => "Gewicht eingeben",

        // Rack selection
        Key::MeasureByRack => "Nach Regal erfassen",
        Key::ProductCount => "Produkte",
        Key::NotStarted => "Nicht begonnen",
        Key::InProgress => "In Bearbeitung",
        Key::Complete => "Abgeschlossen",
        Key::NoProductsInRack => "Keine Produkte in diesem Regal",

        // Custom entries
        Key::CustomEntries => "Benutzerdefinierte Einträge",
        Key::AddCustomEntry => "Benutzerdefinierten Eintrag hinzufügen",
        Key::EditCustomEntry => "Benutzerdefinierten Eintrag bearbeiten",
        Key::CustomProductName => "Produktname",
        Key::Count => "Anzahl",
        Key::WeightGrams => "Gewicht (Gramm)",
        Key::Notes => "Notizen",
        Key::DeleteCustomEntry => "Löschen",
        Key::ConfirmDeleteCustomEntry => "Löschen bestätigen",
        Key::CustomEntry => "Eigener Eintrag",
        Key::ScanToSelectProduct => "Scannen um Produkt auszuwählen",
        Key::ProductNotFound => "Produkt nicht gefunden",

        // QR Codes
        Key::PrintQRCodes => "QR-Codes drucken",
        Key::ScanToLogin => "Zum Anmelden scannen",
        Key::ScanToMeasure => "Zum Erfassen scannen",
        Key::LoginQRCode => "Anmelde-QR-Code",
        Key::RackQRCode => "Regal-QR-Code",
        Key::Rack => "Regal",

        // Custom Tara
        Key::CustomTara => "Eigene Tara",
        Key::TaraWeight => "Tara-Gewicht",
        Key::TaraDescription => "Legen Sie eine eigene Tara fest (z.B. Körpergewicht), die automatisch von allen Gewichtserfassungen abgezogen wird. Dieser Wert wird nur lokal in Ihrem Browser gespeichert und nie an den Server gesendet.",
        Key::TaraHint => "Leer lassen um die Tara zu löschen",
        Key::CurrentTara => "Aktuelle Tara:",
        Key::ClearTara => "Tara löschen",

        // Container-Rack Management
        Key::ContainersInRack => "Gefäße im Regal",
        Key::AddContainerToRack => "Gefäß hinzufügen",
        Key::RemoveContainerFromRack => "Entfernen",
        Key::NoContainersInRack => "Keine Gefäße in diesem Regal",
        Key::ConfirmRemoveContainerFromRack => "Sind Sie sicher, dass Sie dieses Gefäß aus dem Regal entfernen möchten?",
        Key::SelectContainerToAdd => "Gefäß zum Hinzufügen auswählen",

        // Tab Labels
        Key::ProductsTab => "Produkte",
        Key::ContainersTab => "Gefäße",
    }
    .into()
}
