use super::Key;
use std::rc::Rc;

pub fn translate(key: Key) -> Rc<str> {
    match key {
        Key::AppTitle => "Genossi".into(),
        Key::Loading => "Laden...".into(),
        Key::Save => "Speichern".into(),
        Key::Cancel => "Abbrechen".into(),
        Key::Delete => "Löschen".into(),
        Key::Edit => "Bearbeiten".into(),
        Key::Create => "Neu".into(),
        Key::Search => "Suchen...".into(),
        Key::Back => "Zurück".into(),

        Key::Login => "Anmelden".into(),
        Key::Logout => "Abmelden".into(),
        Key::NotAuthenticated => "Nicht angemeldet".into(),
        Key::WelcomeTitle => "Willkommen bei Genossi".into(),
        Key::PleaseLogin => "Bitte melden Sie sich an.".into(),
        Key::AccessDenied => "Zugriff verweigert".into(),

        Key::Home => "Startseite".into(),
        Key::Members => "Mitglieder".into(),
        Key::Permissions => "Berechtigungen".into(),

        Key::MemberNumber => "Mitgliedsnr.".into(),
        Key::FirstName => "Vorname".into(),
        Key::LastName => "Nachname".into(),
        Key::Email => "E-Mail".into(),
        Key::Company => "Firma".into(),
        Key::Comment => "Kommentar".into(),
        Key::Street => "Straße".into(),
        Key::HouseNumber => "Nr.".into(),
        Key::PostalCode => "PLZ".into(),
        Key::City => "Ort".into(),
        Key::JoinDate => "Beitrittsdatum".into(),
        Key::SharesAtJoining => "Anteile (Beitritt)".into(),
        Key::CurrentShares => "Aktuelle Anteile".into(),
        Key::CurrentBalance => "Guthaben (Cent)".into(),
        Key::ExitDate => "Austrittsdatum".into(),
        Key::BankAccount => "Bankverbindung (IBAN)".into(),
        Key::CreateMember => "Neues Mitglied".into(),
        Key::EditMember => "Mitglied bearbeiten".into(),

        Key::NoDataFound => "Keine Daten gefunden.".into(),
        Key::ErrorLoadingData => "Fehler beim Laden der Daten.".into(),
        Key::ConfirmDelete => "Sind Sie sicher, dass Sie dies löschen möchten?".into(),
    }
}
