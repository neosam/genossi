use async_trait::async_trait;
use calamine::{Data, Reader};
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::{ActionType, MemberActionDao, MemberActionEntity};
use genossi_dao::TransactionDao;
use genossi_service::member_import::{MemberImportError, MemberImportResult, MemberImportService};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::ServiceError;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use time::{Date, Month};

use crate::gen_service_impl;

const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";
const MEMBER_IMPORT_PROCESS: &str = "member-import";

gen_service_impl! {
    struct MemberImportServiceImpl: MemberImportService = MemberImportServiceDeps {
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const REQUIRED_COLUMNS: &[&str] = &["ID1", "Nachname", "Vorname(n)", "Beitritt"];

/// Parse a date from various formats.
/// Supports: Excel serial number, DD.MM.YYYY, YYYY-MM-DD
pub fn parse_date(value: &Data) -> Result<Date, String> {
    match value {
        Data::Float(f) => parse_date_from_serial(*f),
        Data::Int(i) => parse_date_from_serial(*i as f64),
        Data::DateTime(ref dt) => {
            // ExcelDateTime as_f64() converts to serial number
            let serial = dt.as_f64();
            parse_date_from_serial(serial)
        }
        Data::DateTimeIso(s) => parse_date_from_string(s),
        Data::DurationIso(s) => parse_date_from_string(s),
        Data::String(s) => parse_date_from_string(s),
        _ => Err(format!("Unexpected cell type for date: {:?}", value)),
    }
}

/// Parse an optional date (empty = None)
pub fn parse_optional_date(value: &Data) -> Result<Option<Date>, String> {
    match value {
        Data::Empty => Ok(None),
        Data::String(s) if s.trim().is_empty() => Ok(None),
        _ => parse_date(value).map(Some),
    }
}

/// Convert an Excel serial number to a Date.
/// Excel uses 1900-01-01 as day 1 (with the Lotus 1-2-3 leap year bug for 1900-02-29).
fn parse_date_from_serial(serial: f64) -> Result<Date, String> {
    let days = serial as i64;
    if days < 1 {
        return Err(format!("Invalid Excel serial date: {}", serial));
    }
    // Excel epoch is 1899-12-30 (accounting for the Lotus 1-2-3 bug)
    let base = Date::from_calendar_date(1899, Month::December, 30)
        .map_err(|e| format!("Date calculation error: {}", e))?;
    let duration = time::Duration::days(days);
    base.checked_add(duration)
        .ok_or_else(|| format!("Date overflow for serial: {}", serial))
}

/// Parse a date string in DD.MM.YYYY or YYYY-MM-DD format.
pub fn parse_date_from_string(s: &str) -> Result<Date, String> {
    let s = s.trim();

    // Try DD.MM.YYYY
    if let Some((day_str, rest)) = s.split_once('.') {
        if let Some((month_str, year_str)) = rest.split_once('.') {
            if let (Ok(day), Ok(month), Ok(year)) = (
                day_str.parse::<u8>(),
                month_str.parse::<u8>(),
                year_str.parse::<i32>(),
            ) {
                let month = Month::try_from(month)
                    .map_err(|_| format!("Invalid month: {}", month))?;
                return Date::from_calendar_date(year, month, day)
                    .map_err(|e| format!("Invalid date {}: {}", s, e));
            }
        }
    }

    // Try YYYY-MM-DD
    if let Some((year_str, rest)) = s.split_once('-') {
        if let Some((month_str, day_str)) = rest.split_once('-') {
            if let (Ok(year), Ok(month), Ok(day)) = (
                year_str.parse::<i32>(),
                month_str.parse::<u8>(),
                day_str.parse::<u8>(),
            ) {
                let month = Month::try_from(month)
                    .map_err(|_| format!("Invalid month: {}", month))?;
                return Date::from_calendar_date(year, month, day)
                    .map_err(|e| format!("Invalid date {}: {}", s, e));
            }
        }
    }

    Err(format!("Could not parse date: '{}'", s))
}

fn get_string(value: &Data) -> Option<Arc<str>> {
    match value {
        Data::String(s) if !s.trim().is_empty() => Some(Arc::from(s.trim())),
        Data::Float(f) => Some(Arc::from(f.to_string().as_str())),
        Data::Int(i) => Some(Arc::from(i.to_string().as_str())),
        _ => None,
    }
}

fn get_i64(value: &Data) -> Result<i64, String> {
    match value {
        Data::Float(f) => Ok(*f as i64),
        Data::Int(i) => Ok(*i),
        Data::String(s) => s
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("Expected integer, got '{}'", s)),
        _ => Err(format!("Expected integer, got {:?}", value)),
    }
}

fn get_i32(value: &Data) -> Result<i32, String> {
    get_i64(value).map(|v| v as i32)
}

/// Check if a row is completely empty.
fn is_row_empty(row: &[Data]) -> bool {
    row.iter().all(|cell| matches!(cell, Data::Empty))
}

/// Build column index map from header row.
fn build_column_index(header: &[Data]) -> HashMap<String, usize> {
    header
        .iter()
        .enumerate()
        .filter_map(|(i, cell)| match cell {
            Data::String(s) => Some((s.trim().to_string(), i)),
            _ => None,
        })
        .collect()
}

/// Check that all required columns are present.
fn check_required_columns(col_index: &HashMap<String, usize>) -> Result<(), String> {
    let missing: Vec<&str> = REQUIRED_COLUMNS
        .iter()
        .filter(|col| !col_index.contains_key(**col))
        .copied()
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("Missing required columns: {}", missing.join(", ")))
    }
}

/// Get cell value by column name, returns Data::Empty if column not found.
fn get_cell<'a>(row: &'a [Data], col_index: &HashMap<String, usize>, col_name: &str) -> &'a Data {
    col_index
        .get(col_name)
        .and_then(|&i| row.get(i))
        .unwrap_or(&Data::Empty)
}

/// Parse a single row into a partial Member (without id/version/created).
fn parse_row(
    row: &[Data],
    col_index: &HashMap<String, usize>,
) -> Result<ParsedMemberRow, String> {
    let member_number = get_i64(get_cell(row, col_index, "ID1"))
        .map_err(|e| format!("ID1: {}", e))?;
    let last_name = get_string(get_cell(row, col_index, "Nachname"))
        .ok_or_else(|| "Nachname is empty".to_string())?;
    let first_name = get_string(get_cell(row, col_index, "Vorname(n)"))
        .ok_or_else(|| "Vorname(n) is empty".to_string())?;
    let join_date = parse_date(get_cell(row, col_index, "Beitritt"))
        .map_err(|e| format!("Beitritt: {}", e))?;

    let shares_at_joining = match get_cell(row, col_index, "Anteile Beitritt") {
        Data::Empty => 0,
        v => get_i32(v).map_err(|e| format!("Anteile Beitritt: {}", e))?,
    };
    let current_shares = match get_cell(row, col_index, "Anteile aktuell") {
        Data::Empty => 0,
        v => get_i32(v).map_err(|e| format!("Anteile aktuell: {}", e))?,
    };
    let current_balance = match get_cell(row, col_index, "Guthaben aktuell") {
        Data::Empty => 0,
        v => get_i64(v).map_err(|e| format!("Guthaben aktuell: {}", e))?,
    };
    let action_count = match get_cell(row, col_index, "Anzahl Aktionen") {
        Data::Empty => 0,
        v => get_i32(v).map_err(|e| format!("Anzahl Aktionen: {}", e))?,
    };
    let exit_date = parse_optional_date(get_cell(row, col_index, "Austritt"))
        .map_err(|e| format!("Austritt: {}", e))?;

    Ok(ParsedMemberRow {
        member_number,
        first_name,
        last_name,
        street: get_string(get_cell(row, col_index, "Straße")),
        house_number: get_string(get_cell(row, col_index, "Nr#")),
        postal_code: get_string(get_cell(row, col_index, "PLZ")),
        city: get_string(get_cell(row, col_index, "Ort")),
        join_date,
        shares_at_joining,
        current_shares,
        current_balance,
        action_count,
        exit_date,
        email: get_string(get_cell(row, col_index, "Email")),
        company: get_string(get_cell(row, col_index, "Firma")),
        comment: get_string(get_cell(row, col_index, "Kommentar")),
        bank_account: get_string(get_cell(row, col_index, "Bankverbindung")),
    })
}

struct ParsedMemberRow {
    member_number: i64,
    first_name: Arc<str>,
    last_name: Arc<str>,
    street: Option<Arc<str>>,
    house_number: Option<Arc<str>>,
    postal_code: Option<Arc<str>>,
    city: Option<Arc<str>>,
    join_date: Date,
    shares_at_joining: i32,
    current_shares: i32,
    current_balance: i64,
    action_count: i32,
    exit_date: Option<Date>,
    email: Option<Arc<str>>,
    company: Option<Arc<str>>,
    comment: Option<Arc<str>>,
    bank_account: Option<Arc<str>>,
}

/// Parse a spreadsheet file from bytes and return parsed rows with errors.
/// Supports xlsx, xls, ods, and xlsb formats.
fn parse_spreadsheet(data: &[u8]) -> Result<(Vec<(usize, ParsedMemberRow)>, Vec<MemberImportError>, usize), ServiceError> {
    let cursor = Cursor::new(data);
    let mut workbook = calamine::open_workbook_auto_from_rs(cursor)
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("Failed to open spreadsheet: {}", e))))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| ServiceError::InternalError(Arc::from("No sheets found in xlsx")))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("Failed to read sheet: {}", e))))?;

    let mut rows = range.rows();
    let header = rows
        .next()
        .ok_or_else(|| ServiceError::InternalError(Arc::from("Empty spreadsheet - no header row")))?;

    let col_index = build_column_index(header);
    check_required_columns(&col_index).map_err(|e| {
        ServiceError::ValidationError(vec![genossi_service::ValidationFailureItem {
            field: Arc::from("columns"),
            message: Arc::from(e),
        }])
    })?;

    let mut parsed_rows = Vec::new();
    let mut errors = Vec::new();
    let mut skipped = 0;

    for (i, row) in rows.enumerate() {
        let row_num = i + 2; // 1-indexed, header is row 1
        if is_row_empty(row) {
            skipped += 1;
            continue;
        }
        match parse_row(row, &col_index) {
            Ok(parsed) => parsed_rows.push((row_num, parsed)),
            Err(e) => errors.push(MemberImportError {
                row: row_num,
                error: e,
            }),
        }
    }

    Ok((parsed_rows, errors, skipped))
}

#[async_trait]
impl<Deps: MemberImportServiceDeps> MemberImportService for MemberImportServiceImpl<Deps> {
    type Context = Deps::Context;

    async fn import_members(
        &self,
        data: &[u8],
        context: Authentication<Self::Context>,
    ) -> Result<MemberImportResult, ServiceError> {
        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let (parsed_rows, errors, skipped) = parse_spreadsheet(data)?;

        let tx = self.transaction_dao.use_transaction(None).await?;

        let mut imported = 0;
        let mut updated = 0;

        for (_row_num, parsed) in &parsed_rows {
            let existing = self
                .member_dao
                .find_by_member_number(parsed.member_number, tx.clone())
                .await?;

            match existing {
                Some(mut entity) => {
                    entity.first_name = parsed.first_name.clone();
                    entity.last_name = parsed.last_name.clone();
                    entity.street = parsed.street.clone();
                    entity.house_number = parsed.house_number.clone();
                    entity.postal_code = parsed.postal_code.clone();
                    entity.city = parsed.city.clone();
                    entity.join_date = parsed.join_date;
                    entity.shares_at_joining = parsed.shares_at_joining;
                    entity.current_shares = parsed.current_shares;
                    entity.current_balance = parsed.current_balance;
                    entity.action_count = parsed.action_count;
                    entity.exit_date = parsed.exit_date;
                    entity.email = parsed.email.clone();
                    entity.company = parsed.company.clone();
                    entity.comment = parsed.comment.clone();
                    entity.bank_account = parsed.bank_account.clone();

                    self.member_dao
                        .update(&entity, MEMBER_IMPORT_PROCESS, tx.clone())
                        .await?;
                    updated += 1;
                }
                None => {
                    let now = time::OffsetDateTime::now_utc();
                    let entity = genossi_dao::member::MemberEntity {
                        id: self.uuid_service.new_v4().await,
                        member_number: parsed.member_number,
                        first_name: parsed.first_name.clone(),
                        last_name: parsed.last_name.clone(),
                        email: parsed.email.clone(),
                        company: parsed.company.clone(),
                        comment: parsed.comment.clone(),
                        street: parsed.street.clone(),
                        house_number: parsed.house_number.clone(),
                        postal_code: parsed.postal_code.clone(),
                        city: parsed.city.clone(),
                        join_date: parsed.join_date,
                        shares_at_joining: parsed.shares_at_joining,
                        current_shares: parsed.current_shares,
                        current_balance: parsed.current_balance,
                        action_count: parsed.action_count,
                        exit_date: parsed.exit_date,
                        bank_account: parsed.bank_account.clone(),
                        created: time::PrimitiveDateTime::new(now.date(), now.time()),
                        deleted: None,
                        version: self.uuid_service.new_v4().await,
                    };

                    self.member_dao
                        .create(&entity, MEMBER_IMPORT_PROCESS, tx.clone())
                        .await?;

                    // Auto-migration: create Eintritt + Aufstockung for members
                    // with action_count == 0 and shares_at_joining == current_shares
                    if parsed.action_count == 0
                        && parsed.shares_at_joining == parsed.current_shares
                    {
                        let eintritt = MemberActionEntity {
                            id: self.uuid_service.new_v4().await,
                            member_id: entity.id,
                            action_type: ActionType::Eintritt,
                            date: parsed.join_date,
                            shares_change: 0,
                            transfer_member_id: None,
                            effective_date: None,
                            comment: Some(Arc::from("Auto-migration from Excel import")),
                            created: entity.created,
                            deleted: None,
                            version: self.uuid_service.new_v4().await,
                        };
                        self.member_action_dao
                            .create(&eintritt, MEMBER_IMPORT_PROCESS, tx.clone())
                            .await?;

                        let aufstockung = MemberActionEntity {
                            id: self.uuid_service.new_v4().await,
                            member_id: entity.id,
                            action_type: ActionType::Aufstockung,
                            date: parsed.join_date,
                            shares_change: parsed.shares_at_joining,
                            transfer_member_id: None,
                            effective_date: None,
                            comment: Some(Arc::from("Auto-migration from Excel import")),
                            created: entity.created,
                            deleted: None,
                            version: self.uuid_service.new_v4().await,
                        };
                        self.member_action_dao
                            .create(&aufstockung, MEMBER_IMPORT_PROCESS, tx.clone())
                            .await?;
                    }

                    imported += 1;
                }
            }
        }

        self.transaction_dao.commit(tx).await?;

        Ok(MemberImportResult {
            imported,
            updated,
            skipped,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_german_format() {
        let data = Data::String("01.03.2023".to_string());
        let date = parse_date(&data).unwrap();
        assert_eq!(date, Date::from_calendar_date(2023, Month::March, 1).unwrap());
    }

    #[test]
    fn test_parse_date_iso_format() {
        let data = Data::String("2023-03-01".to_string());
        let date = parse_date(&data).unwrap();
        assert_eq!(date, Date::from_calendar_date(2023, Month::March, 1).unwrap());
    }

    #[test]
    fn test_parse_date_excel_serial() {
        // 44927 = 2023-01-01 in Excel serial
        let data = Data::Float(44927.0);
        let date = parse_date(&data).unwrap();
        assert_eq!(date, Date::from_calendar_date(2023, Month::January, 1).unwrap());
    }

    #[test]
    fn test_parse_date_excel_serial_int() {
        let data = Data::Int(44927);
        let date = parse_date(&data).unwrap();
        assert_eq!(date, Date::from_calendar_date(2023, Month::January, 1).unwrap());
    }

    #[test]
    fn test_parse_date_invalid_string() {
        let data = Data::String("not-a-date".to_string());
        assert!(parse_date(&data).is_err());
    }

    #[test]
    fn test_parse_date_invalid_month() {
        let data = Data::String("01.13.2023".to_string());
        assert!(parse_date(&data).is_err());
    }

    #[test]
    fn test_parse_optional_date_empty() {
        let data = Data::Empty;
        assert_eq!(parse_optional_date(&data).unwrap(), None);
    }

    #[test]
    fn test_parse_optional_date_empty_string() {
        let data = Data::String("".to_string());
        assert_eq!(parse_optional_date(&data).unwrap(), None);
    }

    #[test]
    fn test_parse_optional_date_with_value() {
        let data = Data::String("15.06.2024".to_string());
        let result = parse_optional_date(&data).unwrap();
        assert_eq!(result, Some(Date::from_calendar_date(2024, Month::June, 15).unwrap()));
    }

    #[test]
    fn test_build_column_index() {
        let header = vec![
            Data::String("ID1".to_string()),
            Data::String("Nachname".to_string()),
            Data::String("Vorname(n)".to_string()),
        ];
        let index = build_column_index(&header);
        assert_eq!(index.get("ID1"), Some(&0));
        assert_eq!(index.get("Nachname"), Some(&1));
        assert_eq!(index.get("Vorname(n)"), Some(&2));
    }

    #[test]
    fn test_check_required_columns_all_present() {
        let mut index = HashMap::new();
        index.insert("ID1".to_string(), 0);
        index.insert("Nachname".to_string(), 1);
        index.insert("Vorname(n)".to_string(), 2);
        index.insert("Beitritt".to_string(), 3);
        assert!(check_required_columns(&index).is_ok());
    }

    #[test]
    fn test_check_required_columns_missing() {
        let mut index = HashMap::new();
        index.insert("ID1".to_string(), 0);
        index.insert("Nachname".to_string(), 1);
        let result = check_required_columns(&index);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Vorname(n)"));
        assert!(err.contains("Beitritt"));
    }

    #[test]
    fn test_is_row_empty() {
        assert!(is_row_empty(&[Data::Empty, Data::Empty]));
        assert!(!is_row_empty(&[Data::Empty, Data::String("a".to_string())]));
    }

    #[test]
    fn test_parse_row_valid() {
        let header = vec![
            Data::String("ID1".to_string()),
            Data::String("Nachname".to_string()),
            Data::String("Vorname(n)".to_string()),
            Data::String("Beitritt".to_string()),
            Data::String("Anteile Beitritt".to_string()),
            Data::String("Anteile aktuell".to_string()),
            Data::String("Guthaben aktuell".to_string()),
        ];
        let col_index = build_column_index(&header);

        let row = vec![
            Data::Int(42),
            Data::String("Müller".to_string()),
            Data::String("Hans".to_string()),
            Data::String("01.01.2020".to_string()),
            Data::Int(3),
            Data::Int(5),
            Data::Int(15000),
        ];

        let parsed = parse_row(&row, &col_index).unwrap();
        assert_eq!(parsed.member_number, 42);
        assert_eq!(&*parsed.last_name, "Müller");
        assert_eq!(&*parsed.first_name, "Hans");
        assert_eq!(parsed.shares_at_joining, 3);
        assert_eq!(parsed.current_shares, 5);
        assert_eq!(parsed.current_balance, 15000);
    }

    #[test]
    fn test_parse_row_missing_name() {
        let header = vec![
            Data::String("ID1".to_string()),
            Data::String("Nachname".to_string()),
            Data::String("Vorname(n)".to_string()),
            Data::String("Beitritt".to_string()),
        ];
        let col_index = build_column_index(&header);

        let row = vec![
            Data::Int(1),
            Data::Empty,
            Data::String("Hans".to_string()),
            Data::String("01.01.2020".to_string()),
        ];

        assert!(parse_row(&row, &col_index).is_err());
    }

    #[test]
    fn test_parse_row_extra_columns_ignored() {
        let header = vec![
            Data::String("ID1".to_string()),
            Data::String("Nachname".to_string()),
            Data::String("Vorname(n)".to_string()),
            Data::String("Beitritt".to_string()),
            Data::String("BE HiDrive".to_string()),
            Data::String("Anzahl Aktionen".to_string()),
        ];
        let col_index = build_column_index(&header);

        let row = vec![
            Data::Int(1),
            Data::String("Test".to_string()),
            Data::String("User".to_string()),
            Data::String("2020-01-01".to_string()),
            Data::String("ja".to_string()),
            Data::Int(5),
        ];

        let parsed = parse_row(&row, &col_index).unwrap();
        assert_eq!(parsed.member_number, 1);
    }
}
