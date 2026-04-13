use genossi_dao::backup::{ActionBackupRow, CommunicationBackupRow, MemberBackupRow};

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

/// Transliterate German umlauts and strip non-ASCII/non-alphanumeric chars.
/// Spaces become underscores, result is truncated to `max_len` characters.
pub fn sanitize_filename(input: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'ä' => result.push_str("ae"),
            'ö' => result.push_str("oe"),
            'ü' => result.push_str("ue"),
            'Ä' => result.push_str("Ae"),
            'Ö' => result.push_str("Oe"),
            'Ü' => result.push_str("Ue"),
            'ß' => result.push_str("ss"),
            ' ' => result.push('_'),
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => result.push(c),
            _ => {}
        }
    }
    if result.len() > max_len {
        result.truncate(max_len);
    }
    result
}

/// Generate the filename for a communication backup file.
/// Pattern: `{YYYY-MM-DD}_{HHmm}_{direction}_{subject}.txt`
/// If `collision_suffix` is provided, it is appended before the extension.
pub fn generate_communication_filename(
    date: &str,
    direction: &str,
    subject: &str,
    collision_suffix: Option<&str>,
) -> String {
    // Parse date to extract YYYY-MM-DD and HHmm
    // Expected formats: "2026-03-15 14:30:00" or ISO8601 "2026-03-15T14:30:00..."
    let date_part = &date[..10.min(date.len())];
    let time_part = if date.len() >= 16 {
        let t = &date[11..16];
        t.replace(':', "")
    } else {
        "0000".to_string()
    };

    let sanitized_subject = sanitize_filename(subject, 50);

    match collision_suffix {
        Some(suffix) => {
            format!(
                "{}_{}_{}_{}_{}",
                date_part, time_part, direction, sanitized_subject, suffix
            )
        }
        None => {
            format!(
                "{}_{}_{}_{}",
                date_part, time_part, direction, sanitized_subject
            )
        }
    }
}

const SEPARATOR: &str = "───────────────────────────────────────";

/// Generate the .txt content for a single communication entry.
pub fn generate_communication_txt(row: &CommunicationBackupRow) -> String {
    let mut content = String::new();

    content.push_str(&format!("Richtung: {}\n", capitalize_direction(&row.direction)));
    content.push_str(&format!("Datum: {}\n", format_date_for_display(&row.date)));

    if row.direction.as_ref() == "eingehend" {
        if let Some(from) = &row.from_address {
            content.push_str(&format!("Von: {}\n", from));
        }
    } else {
        if let Some(to) = &row.to_address {
            content.push_str(&format!("An: {}\n", to));
        }
    }

    content.push_str(&format!("Betreff: {}\n", row.subject));
    content.push('\n');
    content.push_str(SEPARATOR);
    content.push_str("\n\n");
    content.push_str(&row.body);

    // Ensure trailing newline
    if !content.ends_with('\n') {
        content.push('\n');
    }

    content
}

fn capitalize_direction(dir: &str) -> &str {
    match dir {
        "eingehend" => "Eingehend",
        "ausgehend" => "Ausgehend",
        other => other,
    }
}

fn format_date_for_display(date: &str) -> String {
    // Convert ISO format to display format
    // "2026-03-15T14:30:00" or "2026-03-15 14:30:00" → "2026-03-15 14:30:00"
    if date.len() >= 19 {
        let mut d = date[..19].to_string();
        if d.contains('T') {
            d = d.replace('T', " ");
        }
        d
    } else {
        date.to_string()
    }
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

    // ─── sanitize_filename tests ───────────────────────────────────────

    #[test]
    fn test_sanitize_filename_umlauts() {
        assert_eq!(sanitize_filename("Ärger über Öffnung", 50), "Aerger_ueber_Oeffnung");
    }

    #[test]
    fn test_sanitize_filename_eszett() {
        assert_eq!(sanitize_filename("Straße", 50), "Strasse");
    }

    #[test]
    fn test_sanitize_filename_special_chars() {
        assert_eq!(sanitize_filename("Hello! @World# (2026)", 50), "Hello_World_2026");
    }

    #[test]
    fn test_sanitize_filename_truncation() {
        let long = "a".repeat(100);
        assert_eq!(sanitize_filename(&long, 50).len(), 50);
    }

    #[test]
    fn test_sanitize_filename_spaces_to_underscores() {
        assert_eq!(sanitize_filename("Hello World Test", 50), "Hello_World_Test");
    }

    #[test]
    fn test_sanitize_filename_preserves_hyphens() {
        assert_eq!(sanitize_filename("some-thing", 50), "some-thing");
    }

    // ─── generate_communication_filename tests ─────────────────────────

    #[test]
    fn test_generate_communication_filename_basic() {
        let result = generate_communication_filename(
            "2026-03-15 14:30:00",
            "ausgehend",
            "Willkommen bei uns",
            None,
        );
        assert_eq!(result, "2026-03-15_1430_ausgehend_Willkommen_bei_uns");
    }

    #[test]
    fn test_generate_communication_filename_with_collision() {
        let result = generate_communication_filename(
            "2026-03-15 14:30:00",
            "eingehend",
            "Test",
            Some("a1b2c3d4"),
        );
        assert_eq!(result, "2026-03-15_1430_eingehend_Test_a1b2c3d4");
    }

    #[test]
    fn test_generate_communication_filename_iso_format() {
        let result = generate_communication_filename(
            "2026-03-15T14:30:00.000Z",
            "ausgehend",
            "Hallo",
            None,
        );
        assert_eq!(result, "2026-03-15_1430_ausgehend_Hallo");
    }

    // ─── generate_communication_txt tests ──────────────────────────────

    #[test]
    fn test_generate_communication_txt_outbound() {
        let row = CommunicationBackupRow {
            member_number: 1,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            direction: Arc::from("ausgehend"),
            date: Arc::from("2026-03-15T14:30:00"),
            subject: Arc::from("Willkommen"),
            body: Arc::from("Hallo Hans,\n\nwillkommen!"),
            from_address: None,
            to_address: Some(Arc::from("hans@example.com")),
            mail_id: uuid::Uuid::new_v4(),
            mail_type: Arc::from("outbound"),
        };
        let txt = generate_communication_txt(&row);
        assert!(txt.contains("Richtung: Ausgehend"));
        assert!(txt.contains("Datum: 2026-03-15 14:30:00"));
        assert!(txt.contains("An: hans@example.com"));
        assert!(txt.contains("Betreff: Willkommen"));
        assert!(txt.contains(SEPARATOR));
        assert!(txt.contains("Hallo Hans,\n\nwillkommen!"));
    }

    #[test]
    fn test_generate_communication_txt_inbound() {
        let row = CommunicationBackupRow {
            member_number: 2,
            first_name: Arc::from("Anna"),
            last_name: Arc::from("Schmidt"),
            direction: Arc::from("eingehend"),
            date: Arc::from("2026-04-01 09:15:00"),
            subject: Arc::from("Frage zu Anteilen"),
            body: Arc::from("Guten Tag,\n\nich hätte eine Frage..."),
            from_address: Some(Arc::from("anna@example.com")),
            to_address: None,
            mail_id: uuid::Uuid::new_v4(),
            mail_type: Arc::from("inbound"),
        };
        let txt = generate_communication_txt(&row);
        assert!(txt.contains("Richtung: Eingehend"));
        assert!(txt.contains("Datum: 2026-04-01 09:15:00"));
        assert!(txt.contains("Von: anna@example.com"));
        assert!(txt.contains("Betreff: Frage zu Anteilen"));
        assert!(txt.contains("ich hätte eine Frage..."));
        assert!(!txt.contains("An:"));
    }
}
