use crate::i18n::Key;
use rest_types::MemberTO;

#[derive(Clone, Copy, PartialEq)]
pub enum InputType {
    Text,
    Number,
    None, // read-only, no input
}

#[derive(Clone)]
pub struct ColumnDef {
    pub key: &'static str,
    pub label_key: Key,
    pub editable: bool,
    pub input_type: InputType,
    pub render: fn(&MemberTO, &crate::i18n::I18n) -> String,
    pub get_value: fn(&MemberTO) -> String,
    pub set_value: fn(&mut MemberTO, &str),
}

fn noop_set(_m: &mut MemberTO, _v: &str) {}

fn render_member_number(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.member_number.to_string() }
fn render_last_name(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.last_name.clone() }
fn render_first_name(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.first_name.clone() }
fn render_email(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.email.clone().unwrap_or_default() }
fn render_company(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.company.clone().unwrap_or_default() }
fn render_street(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.street.clone().unwrap_or_default() }
fn render_house_number(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.house_number.clone().unwrap_or_default() }
fn render_postal_code(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.postal_code.clone().unwrap_or_default() }
fn render_city(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.city.clone().unwrap_or_default() }
fn render_current_shares(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.current_shares.to_string() }
fn render_current_balance(m: &MemberTO, i18n: &crate::i18n::I18n) -> String { i18n.format_price(m.current_balance) }
fn render_shares_at_joining(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.shares_at_joining.to_string() }
fn render_bank_account(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.bank_account.clone().unwrap_or_default() }
fn render_comment(m: &MemberTO, _i18n: &crate::i18n::I18n) -> String { m.comment.clone().unwrap_or_default() }
fn render_join_date(m: &MemberTO, i18n: &crate::i18n::I18n) -> String { i18n.format_date(&m.join_date) }
fn render_exit_date(m: &MemberTO, i18n: &crate::i18n::I18n) -> String { m.exit_date.as_ref().map(|d| i18n.format_date(d)).unwrap_or_default() }
fn render_migrated(m: &MemberTO, i18n: &crate::i18n::I18n) -> String {
    if m.migrated { i18n.t(Key::Migrated).to_string() } else { i18n.t(Key::Pending).to_string() }
}

fn opt_str(s: &Option<String>) -> String { s.clone().unwrap_or_default() }
fn set_opt(v: &str) -> Option<String> { if v.is_empty() { None } else { Some(v.to_string()) } }

pub static ALL_COLUMNS: &[ColumnDef] = &[
    ColumnDef {
        key: "member_number", label_key: Key::MemberNumber, editable: true, input_type: InputType::Number,
        render: render_member_number,
        get_value: |m| m.member_number.to_string(),
        set_value: |m, v| { if let Ok(n) = v.parse() { m.member_number = n; } },
    },
    ColumnDef {
        key: "last_name", label_key: Key::LastName, editable: true, input_type: InputType::Text,
        render: render_last_name,
        get_value: |m| m.last_name.clone(),
        set_value: |m, v| { m.last_name = v.to_string(); },
    },
    ColumnDef {
        key: "first_name", label_key: Key::FirstName, editable: true, input_type: InputType::Text,
        render: render_first_name,
        get_value: |m| m.first_name.clone(),
        set_value: |m, v| { m.first_name = v.to_string(); },
    },
    ColumnDef {
        key: "email", label_key: Key::Email, editable: true, input_type: InputType::Text,
        render: render_email,
        get_value: |m| opt_str(&m.email),
        set_value: |m, v| { m.email = set_opt(v); },
    },
    ColumnDef {
        key: "company", label_key: Key::Company, editable: true, input_type: InputType::Text,
        render: render_company,
        get_value: |m| opt_str(&m.company),
        set_value: |m, v| { m.company = set_opt(v); },
    },
    ColumnDef {
        key: "street", label_key: Key::Street, editable: true, input_type: InputType::Text,
        render: render_street,
        get_value: |m| opt_str(&m.street),
        set_value: |m, v| { m.street = set_opt(v); },
    },
    ColumnDef {
        key: "house_number", label_key: Key::HouseNumber, editable: true, input_type: InputType::Text,
        render: render_house_number,
        get_value: |m| opt_str(&m.house_number),
        set_value: |m, v| { m.house_number = set_opt(v); },
    },
    ColumnDef {
        key: "postal_code", label_key: Key::PostalCode, editable: true, input_type: InputType::Text,
        render: render_postal_code,
        get_value: |m| opt_str(&m.postal_code),
        set_value: |m, v| { m.postal_code = set_opt(v); },
    },
    ColumnDef {
        key: "city", label_key: Key::City, editable: true, input_type: InputType::Text,
        render: render_city,
        get_value: |m| opt_str(&m.city),
        set_value: |m, v| { m.city = set_opt(v); },
    },
    ColumnDef {
        key: "current_shares", label_key: Key::CurrentShares, editable: false, input_type: InputType::None,
        render: render_current_shares,
        get_value: |m| m.current_shares.to_string(),
        set_value: noop_set,
    },
    ColumnDef {
        key: "current_balance", label_key: Key::CurrentBalance, editable: true, input_type: InputType::Number,
        render: render_current_balance,
        get_value: |m| m.current_balance.to_string(),
        set_value: |m, v| { if let Ok(n) = v.parse() { m.current_balance = n; } },
    },
    ColumnDef {
        key: "shares_at_joining", label_key: Key::SharesAtJoining, editable: true, input_type: InputType::Number,
        render: render_shares_at_joining,
        get_value: |m| m.shares_at_joining.to_string(),
        set_value: |m, v| { if let Ok(n) = v.parse() { m.shares_at_joining = n; } },
    },
    ColumnDef {
        key: "bank_account", label_key: Key::BankAccount, editable: true, input_type: InputType::Text,
        render: render_bank_account,
        get_value: |m| opt_str(&m.bank_account),
        set_value: |m, v| { m.bank_account = set_opt(v); },
    },
    ColumnDef {
        key: "comment", label_key: Key::Comment, editable: true, input_type: InputType::Text,
        render: render_comment,
        get_value: |m| opt_str(&m.comment),
        set_value: |m, v| { m.comment = set_opt(v); },
    },
    ColumnDef {
        key: "join_date", label_key: Key::JoinDate, editable: false, input_type: InputType::None,
        render: render_join_date,
        get_value: |m| format!("{}", m.join_date),
        set_value: noop_set,
    },
    ColumnDef {
        key: "exit_date", label_key: Key::ExitDate, editable: false, input_type: InputType::None,
        render: render_exit_date,
        get_value: |m| m.exit_date.map(|d| format!("{}", d)).unwrap_or_default(),
        set_value: noop_set,
    },
    ColumnDef {
        key: "migrated", label_key: Key::MigrationStatus, editable: false, input_type: InputType::None,
        render: render_migrated,
        get_value: |m| m.migrated.to_string(),
        set_value: noop_set,
    },
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
