use crate::i18n::Key;
use rest_types::MemberTO;

#[derive(Clone)]
pub struct ColumnDef {
    pub key: &'static str,
    pub label_key: Key,
    pub editable: bool,
    pub render: fn(&MemberTO, &crate::i18n::I18n) -> String,
}

fn render_member_number(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.member_number.to_string()
}
fn render_last_name(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.last_name.clone()
}
fn render_first_name(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.first_name.clone()
}
fn render_email(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.email.clone().unwrap_or_default()
}
fn render_company(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.company.clone().unwrap_or_default()
}
fn render_street(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.street.clone().unwrap_or_default()
}
fn render_house_number(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.house_number.clone().unwrap_or_default()
}
fn render_postal_code(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.postal_code.clone().unwrap_or_default()
}
fn render_city(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.city.clone().unwrap_or_default()
}
fn render_current_shares(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.current_shares.to_string()
}
fn render_current_balance(m: &MemberTO, i18n: &crate::i18n::I18n) -> String {
    i18n.format_price(m.current_balance)
}
fn render_shares_at_joining(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.shares_at_joining.to_string()
}
fn render_bank_account(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.bank_account.clone().unwrap_or_default()
}
fn render_comment(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String {
    m.comment.clone().unwrap_or_default()
}
fn render_join_date(m: &MemberTO, i18n: &crate::i18n::I18n) -> String {
    i18n.format_date(&m.join_date)
}
fn render_exit_date(m: &MemberTO, i18n: &crate::i18n::I18n) -> String {
    m.exit_date.as_ref().map(|d| i18n.format_date(d)).unwrap_or_default()
}

fn render_migrated(m: &MemberTO, i18n: &crate::i18n::I18n) -> String {
    if m.migrated {
        i18n.t(Key::Migrated).to_string()
    } else {
        i18n.t(Key::Pending).to_string()
    }
}

pub static ALL_COLUMNS: &[ColumnDef] = &[
    ColumnDef { key: "member_number", label_key: Key::MemberNumber, editable: true, render: render_member_number },
    ColumnDef { key: "last_name", label_key: Key::LastName, editable: true, render: render_last_name },
    ColumnDef { key: "first_name", label_key: Key::FirstName, editable: true, render: render_first_name },
    ColumnDef { key: "email", label_key: Key::Email, editable: true, render: render_email },
    ColumnDef { key: "company", label_key: Key::Company, editable: true, render: render_company },
    ColumnDef { key: "street", label_key: Key::Street, editable: true, render: render_street },
    ColumnDef { key: "house_number", label_key: Key::HouseNumber, editable: true, render: render_house_number },
    ColumnDef { key: "postal_code", label_key: Key::PostalCode, editable: true, render: render_postal_code },
    ColumnDef { key: "city", label_key: Key::City, editable: true, render: render_city },
    ColumnDef { key: "current_shares", label_key: Key::CurrentShares, editable: false, render: render_current_shares },
    ColumnDef { key: "current_balance", label_key: Key::CurrentBalance, editable: true, render: render_current_balance },
    ColumnDef { key: "shares_at_joining", label_key: Key::SharesAtJoining, editable: true, render: render_shares_at_joining },
    ColumnDef { key: "bank_account", label_key: Key::BankAccount, editable: true, render: render_bank_account },
    ColumnDef { key: "comment", label_key: Key::Comment, editable: true, render: render_comment },
    ColumnDef { key: "join_date", label_key: Key::JoinDate, editable: false, render: render_join_date },
    ColumnDef { key: "exit_date", label_key: Key::ExitDate, editable: false, render: render_exit_date },
    ColumnDef { key: "migrated", label_key: Key::MigrationStatus, editable: false, render: render_migrated },
];

pub const DEFAULT_COLUMNS: &[&str] = &[
    "member_number",
    "last_name",
    "first_name",
    "city",
    "current_shares",
    "join_date",
    "exit_date",
    "migrated",
];

pub fn columns_for_keys(keys: &[String]) -> Vec<&'static ColumnDef> {
    keys.iter()
        .filter_map(|key| ALL_COLUMNS.iter().find(|c| c.key == key.as_str()))
        .collect()
}

pub fn default_column_keys() -> Vec<String> {
    DEFAULT_COLUMNS.iter().map(|s| s.to_string()).collect()
}
