use genossi_dao::backup::{ActionBackupRow, MemberBackupRow};

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

pub fn generate_members_csv(members: &[MemberBackupRow]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    buf.extend_from_slice(UTF8_BOM);

    let mut wtr = csv::Writer::from_writer(&mut buf);
    wtr.write_record([
        "Mitgliedsnummer",
        "Anrede",
        "Titel",
        "Vorname",
        "Nachname",
        "Firma",
        "Strasse",
        "Hausnummer",
        "PLZ",
        "Ort",
        "Email",
        "Bankverbindung",
        "Beitrittsdatum",
        "Austrittsdatum",
        "Anteile bei Beitritt",
        "Anteile am Stichtag",
        "Kommentar",
    ])
    .map_err(|e| e.to_string())?;

    for m in members.iter() {
        wtr.write_record([
            m.member_number.to_string(),
            m.salutation.as_deref().unwrap_or("").to_string(),
            m.title.as_deref().unwrap_or("").to_string(),
            m.first_name.to_string(),
            m.last_name.to_string(),
            m.company.as_deref().unwrap_or("").to_string(),
            m.street.as_deref().unwrap_or("").to_string(),
            m.house_number.as_deref().unwrap_or("").to_string(),
            m.postal_code.as_deref().unwrap_or("").to_string(),
            m.city.as_deref().unwrap_or("").to_string(),
            m.email.as_deref().unwrap_or("").to_string(),
            m.bank_account.as_deref().unwrap_or("").to_string(),
            m.join_date.to_string(),
            m.exit_date.as_deref().unwrap_or("").to_string(),
            m.shares_at_joining.to_string(),
            m.shares_at_date.to_string(),
            m.comment.as_deref().unwrap_or("").to_string(),
        ])
        .map_err(|e| e.to_string())?;
    }

    wtr.flush().map_err(|e| e.to_string())?;
    drop(wtr);

    Ok(buf)
}

pub fn generate_actions_csv(actions: &[ActionBackupRow]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    buf.extend_from_slice(UTF8_BOM);

    let mut wtr = csv::Writer::from_writer(&mut buf);
    wtr.write_record([
        "Mitgliedsnummer",
        "Vorname",
        "Nachname",
        "Aktionstyp",
        "Datum",
        "Anteileaenderung",
        "Uebertragung-Mitgliedsnummer",
        "Wirksamkeitsdatum",
        "Kommentar",
    ])
    .map_err(|e| e.to_string())?;

    for a in actions.iter() {
        wtr.write_record([
            a.member_number.to_string(),
            a.first_name.to_string(),
            a.last_name.to_string(),
            a.action_type.to_string(),
            a.date.to_string(),
            a.shares_change.to_string(),
            a.transfer_member_number
                .map(|n| n.to_string())
                .unwrap_or_default(),
            a.effective_date.as_deref().unwrap_or("").to_string(),
            a.comment.as_deref().unwrap_or("").to_string(),
        ])
        .map_err(|e| e.to_string())?;
    }

    wtr.flush().map_err(|e| e.to_string())?;
    drop(wtr);

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_generate_members_csv_empty() {
        let result = generate_members_csv(&[]).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(text.contains("Mitgliedsnummer"));
        assert!(text.contains("Vorname"));
    }

    #[test]
    fn test_generate_members_csv_with_data() {
        let members = vec![MemberBackupRow {
            member_number: 1,
            salutation: Some(Arc::from("Herr")),
            title: None,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            company: None,
            street: Some(Arc::from("Hauptstr.")),
            house_number: Some(Arc::from("1")),
            postal_code: Some(Arc::from("12345")),
            city: Some(Arc::from("Berlin")),
            email: Some(Arc::from("hans@example.com")),
            bank_account: None,
            join_date: Arc::from("2020-01-15"),
            exit_date: None,
            shares_at_joining: 1,
            shares_at_date: 3,
            comment: None,
        }];
        let result = generate_members_csv(&members).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(text.contains("Hans"));
        assert!(text.contains("Müller"));
        assert!(text.contains("2020-01-15"));
    }

    #[test]
    fn test_generate_actions_csv_empty() {
        let result = generate_actions_csv(&[]).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(text.contains("Aktionstyp"));
    }

    #[test]
    fn test_generate_actions_csv_with_data() {
        let actions = vec![ActionBackupRow {
            member_number: 1,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            action_type: Arc::from("Beitritt"),
            date: Arc::from("2020-01-15"),
            shares_change: 1,
            transfer_member_number: None,
            effective_date: None,
            comment: Some(Arc::from("Erstbeitritt")),
        }];
        let result = generate_actions_csv(&actions).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(text.contains("Beitritt"));
        assert!(text.contains("Erstbeitritt"));
    }
}
