use super::Key;
use std::rc::Rc;

pub fn translate(key: Key) -> Rc<str> {
    match key {
        Key::AppTitle => "Genossi".into(),
        Key::Loading => "Loading...".into(),
        Key::Save => "Save".into(),
        Key::Cancel => "Cancel".into(),
        Key::Delete => "Delete".into(),
        Key::Edit => "Edit".into(),
        Key::Create => "New".into(),
        Key::Search => "Search...".into(),
        Key::Back => "Back".into(),

        Key::Login => "Login".into(),
        Key::Logout => "Logout".into(),
        Key::NotAuthenticated => "Not Authenticated".into(),
        Key::WelcomeTitle => "Welcome to Genossi".into(),
        Key::PleaseLogin => "Please log in to continue.".into(),
        Key::AccessDenied => "Access Denied".into(),

        Key::Home => "Home".into(),
        Key::Members => "Members".into(),
        Key::Permissions => "Permissions".into(),

        Key::MemberNumber => "Member No.".into(),
        Key::FirstName => "First Name".into(),
        Key::LastName => "Last Name".into(),
        Key::Email => "Email".into(),
        Key::Company => "Company".into(),
        Key::Comment => "Comment".into(),
        Key::Street => "Street".into(),
        Key::HouseNumber => "No.".into(),
        Key::PostalCode => "Postal Code".into(),
        Key::City => "City".into(),
        Key::JoinDate => "Join Date".into(),
        Key::SharesAtJoining => "Shares (Joining)".into(),
        Key::CurrentShares => "Current Shares".into(),
        Key::CurrentBalance => "Balance (Cents)".into(),
        Key::ExitDate => "Exit Date".into(),
        Key::BankAccount => "Bank Account (IBAN)".into(),
        Key::CreateMember => "New Member".into(),
        Key::EditMember => "Edit Member".into(),

        Key::NoDataFound => "No data found.".into(),
        Key::ErrorLoadingData => "Error loading data.".into(),
        Key::ConfirmDelete => "Are you sure you want to delete this?".into(),
    }
}
