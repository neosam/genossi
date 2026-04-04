#![cfg(feature = "mock_auth")]

use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::start_test_server;
use genossi_rest_types::{
    ActionTypeTO, MemberActionTO, MemberDocumentTO, MemberImportResultTO, MemberTO,
    MigrationStatusTO, UserPreferenceTO, ValidationResultTO,
};
use genossi_config::rest::{ConfigEntryTO, SetConfigRequest};
use genossi_mail::rest::{SendBulkMailRequest, BulkRecipient, SendMailRequest, MailJobTO, MailJobDetailTO, TestMailRequest};
use reqwest::StatusCode;
use sqlx::SqlitePool;
use std::sync::Arc;

async fn setup() -> genossi_rest::test_server::test_support::TestServer {
    let pool = Arc::new(
        SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database"),
    );

    sqlx::migrate!("../migrations/sqlite")
        .run(&*pool)
        .await
        .expect("Failed to run migrations");

    let rest_state = RestStateImpl::new(pool);
    start_test_server(rest_state).await
}

fn sample_member() -> MemberTO {
    MemberTO {
        id: None,
        member_number: 1,
        first_name: "Max".to_string(),
        last_name: "Mustermann".to_string(),
        email: Some("max@example.com".to_string()),
        company: None,
        comment: None,
        street: Some("Musterstraße".to_string()),
        house_number: Some("1a".to_string()),
        postal_code: Some("12345".to_string()),
        city: Some("Berlin".to_string()),
        join_date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
        shares_at_joining: 1,
        current_shares: 3,
        current_balance: 15000,
        action_count: 0,
        migrated: false,
        exit_date: None,
        bank_account: Some("DE89370400440532013000".to_string()),
        created: None,
        deleted: None,
        version: None,
    }
}

#[tokio::test]
async fn test_get_all_members_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert!(members.is_empty());
}

#[tokio::test]
async fn test_create_and_get_member() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = sample_member();

    // Create
    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    assert!(created.id.is_some());
    assert_eq!(created.first_name, "Max");
    assert_eq!(created.last_name, "Mustermann");
    assert_eq!(created.member_number, 1);
    assert_eq!(created.current_shares, 1); // set to shares_at_joining by service
    assert_eq!(created.current_balance, 0); // set to 0 by service

    // Get by ID
    let id = created.id.unwrap();
    let response = client
        .get(server.url(&format!("/api/members/{}", id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let fetched: MemberTO = response.json().await.unwrap();
    assert_eq!(fetched.first_name, "Max");
    assert_eq!(fetched.member_number, 1);
}

#[tokio::test]
async fn test_update_member() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create
    let response = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();

    let created: MemberTO = response.json().await.unwrap();
    let id = created.id.unwrap();

    // Update
    let mut updated = created.clone();
    updated.first_name = "Maximilian".to_string();
    updated.current_shares = 5;

    let response = client
        .put(server.url(&format!("/api/members/{}", id)))
        .json(&updated)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: MemberTO = response.json().await.unwrap();
    assert_eq!(result.first_name, "Maximilian");
    assert_eq!(result.current_shares, 5);
}

#[tokio::test]
async fn test_delete_member() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create
    let response = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();

    let created: MemberTO = response.json().await.unwrap();
    let id = created.id.unwrap();

    // Delete
    let response = client
        .delete(server.url(&format!("/api/members/{}", id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify deleted (should return 404)
    let response = client
        .get(server.url(&format!("/api/members/{}", id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_all_members_lists_created() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create two members
    let mut member1 = sample_member();
    member1.member_number = 1;
    member1.first_name = "Alice".to_string();

    let mut member2 = sample_member();
    member2.member_number = 2;
    member2.first_name = "Bob".to_string();

    client
        .post(server.url("/api/members"))
        .json(&member1)
        .send()
        .await
        .unwrap();

    client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap();

    // Get all
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members.len(), 2);
}

#[tokio::test]
async fn test_create_member_validation_empty_name() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.first_name = "".to_string();

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_member_duplicate_member_number() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member1 = sample_member();
    client
        .post(server.url("/api/members"))
        .json(&member1)
        .send()
        .await
        .unwrap();

    // Try to create another with same member_number
    let mut member2 = sample_member();
    member2.first_name = "Other".to_string();

    let response = client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_nonexistent_member() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url(&format!(
            "/api/members/{}",
            uuid::Uuid::new_v4()
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// === Excel Import E2E Tests ===

// Helper: create xlsx bytes with given headers and rows
fn create_xlsx(headers: &[&str], rows: &[Vec<&str>]) -> Vec<u8> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *header)
            .unwrap();
    }

    for (row_idx, row) in rows.iter().enumerate() {
        for (col, value) in row.iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            // Try to write as number first
            if let Ok(num) = value.parse::<f64>() {
                worksheet
                    .write_number((row_idx + 1) as u32, col as u16, num)
                    .unwrap();
            } else {
                worksheet
                    .write_string((row_idx + 1) as u32, col as u16, *value)
                    .unwrap();
            }
        }
    }

    workbook.save_to_buffer().unwrap()
}

fn standard_headers() -> Vec<&'static str> {
    vec![
        "ID1",
        "Nachname",
        "Vorname(n)",
        "Straße",
        "Nr#",
        "PLZ",
        "Ort",
        "Beitritt",
        "Anteile Beitritt",
        "Anteile aktuell",
        "Guthaben aktuell",
        "Anzahl Aktionen",
        "Austritt",
        "Email",
        "Firma",
        "Kommentar",
        "Bankverbindung",
    ]
}

#[tokio::test]
async fn test_import_new_members() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let xlsx = create_xlsx(
        &standard_headers(),
        &[
            vec![
                "1", "Müller", "Hans", "Hauptstr.", "5", "10115", "Berlin",
                "01.01.2020", "3", "5", "150", "1", "", "hans@test.de", "", "", "DE123",
            ],
            vec![
                "2", "Schmidt", "Anna", "Nebenstr.", "10", "80331", "München",
                "15.06.2021", "2", "2", "100", "0", "", "anna@test.de", "Firma GmbH", "", "",
            ],
        ],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: MemberImportResultTO = response.json().await.unwrap();
    assert_eq!(result.imported, 2);
    assert_eq!(result.updated, 0);
    assert_eq!(result.skipped, 0);
    assert!(result.errors.is_empty());

    // Verify members exist
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members.len(), 2);
}

#[tokio::test]
async fn test_import_upsert_existing_members() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // First import
    let xlsx1 = create_xlsx(
        &standard_headers(),
        &[vec![
            "1", "Müller", "Hans", "Hauptstr.", "5", "10115", "Berlin",
            "01.01.2020", "3", "3", "100", "0", "", "", "", "", "",
        ]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx1)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    // Second import with updated data for member_number 1
    let xlsx2 = create_xlsx(
        &standard_headers(),
        &[vec![
            "1", "Müller", "Hans-Peter", "Hauptstr.", "5", "10115", "Berlin",
            "01.01.2020", "3", "5", "200", "1", "", "new@email.de", "", "", "",
        ]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx2)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: MemberImportResultTO = response.json().await.unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 1);

    // Verify updated data
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].first_name, "Hans-Peter");
    assert_eq!(members[0].current_shares, 5);
    assert_eq!(members[0].current_balance, 20000); // 200 Euro = 20000 Cent
}

#[tokio::test]
async fn test_import_missing_required_columns() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Missing "Beitritt" column
    let xlsx = create_xlsx(
        &["ID1", "Nachname", "Vorname(n)"],
        &[vec!["1", "Test", "User"]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_import_with_invalid_data_row() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let xlsx = create_xlsx(
        &standard_headers(),
        &[
            // Valid row
            vec![
                "1", "Müller", "Hans", "", "", "", "",
                "01.01.2020", "3", "3", "100", "0", "", "", "", "", "",
            ],
            // Invalid row - bad date
            vec![
                "2", "Schmidt", "Anna", "", "", "", "",
                "not-a-date", "2", "2", "50", "0", "", "", "", "", "",
            ],
        ],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: MemberImportResultTO = response.json().await.unwrap();
    assert_eq!(result.imported, 1);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].row, 3); // Row 3 (1-indexed, header is 1)
}

#[tokio::test]
async fn test_generate_test_data_creates_members() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // First call should create test data
    let response = client
        .post(server.url("/api/dev/generate-test-data"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["count"].as_u64().unwrap() >= 5);

    // Verify members exist
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert!(members.len() >= 5);

    // Verify at least one has all optional fields set
    let fully_populated = members.iter().any(|m| {
        m.email.is_some()
            && m.company.is_some()
            && m.street.is_some()
            && m.bank_account.is_some()
    });
    assert!(fully_populated, "At least one member should have all optional fields");

    // Verify at least one has exit_date set
    let has_exited = members.iter().any(|m| m.exit_date.is_some());
    assert!(has_exited, "At least one member should have an exit_date");
}

#[tokio::test]
async fn test_generate_test_data_is_idempotent() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // First call creates data
    let response = client
        .post(server.url("/api/dev/generate-test-data"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Get count after first call
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members_after_first: Vec<MemberTO> = response.json().await.unwrap();

    // Second call should not create more data
    let response = client
        .post(server.url("/api/dev/generate-test-data"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Count should be the same
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members_after_second: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members_after_first.len(), members_after_second.len());
}

// === Member Action E2E Tests ===

async fn create_test_member(
    client: &reqwest::Client,
    server: &genossi_rest::test_server::test_support::TestServer,
) -> MemberTO {
    let response = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}

fn sample_action(member_id: uuid::Uuid) -> MemberActionTO {
    MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Aufstockung,
        date: time::Date::from_calendar_date(2024, time::Month::March, 15).unwrap(),
        shares_change: 3,
        transfer_member_id: None,
        effective_date: None,
        comment: Some("Initial purchase".to_string()),
        created: None,
        deleted: None,
        version: None,
    }
}

#[tokio::test]
async fn test_create_and_list_member_actions() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Auto-created Eintritt + Aufstockung should already exist
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 2);
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Eintritt)));
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Aufstockung)));

    // Create an additional Aufstockung action
    let aufstockung = sample_action(member_id);
    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&aufstockung)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberActionTO = response.json().await.unwrap();
    assert!(created.id.is_some());
    assert_eq!(created.shares_change, 3);

    // List actions - should now be 3 (2 auto + 1 manual)
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 3);
}

#[tokio::test]
async fn test_update_member_action() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let action = sample_action(member_id);
    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    let created: MemberActionTO = response.json().await.unwrap();
    let action_id = created.id.unwrap();

    // Update
    let mut updated = created.clone();
    updated.shares_change = 5;
    updated.comment = Some("Updated purchase".to_string());

    let response = client
        .put(server.url(&format!(
            "/api/members/{}/actions/{}",
            member_id, action_id
        )))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result: MemberActionTO = response.json().await.unwrap();
    assert_eq!(result.shares_change, 5);
}

#[tokio::test]
async fn test_delete_member_action() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let action = sample_action(member_id);
    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    let created: MemberActionTO = response.json().await.unwrap();
    let action_id = created.id.unwrap();

    // Delete
    let response = client
        .delete(server.url(&format!(
            "/api/members/{}/actions/{}",
            member_id, action_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify deleted - only auto-created actions remain
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 2); // 2 auto-created (Eintritt + Aufstockung) remain
}

#[tokio::test]
async fn test_action_validation_aufstockung_negative_shares() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let mut action = sample_action(member_id);
    action.shares_change = -3; // Invalid for Aufstockung

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_action_validation_uebertragung_without_transfer_member() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let mut action = sample_action(member_id);
    action.action_type = ActionTypeTO::UebertragungEmpfang;
    action.shares_change = 2;
    action.transfer_member_id = None; // Missing!

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_action_validation_effective_date_on_non_austritt() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let mut action = sample_action(member_id);
    action.effective_date =
        Some(time::Date::from_calendar_date(2024, time::Month::December, 31).unwrap());

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_migration_status_pending() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Update member to set current_shares=5, action_count=1 to force pending status
    let mut updated = created.clone();
    updated.current_shares = 5;
    updated.action_count = 1;
    let response = client
        .put(server.url(&format!("/api/members/{}", member_id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Auto-created actions: Eintritt(0) + Aufstockung(+3) => actual_shares=3, actual_action_count=1
    // expected_shares=5, expected_action_count = action_count(1) + 1 = 2
    let response = client
        .get(server.url(&format!(
            "/api/members/{}/actions/migration-status",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status: MigrationStatusTO = response.json().await.unwrap();
    assert_eq!(status.status, "pending");
    assert_eq!(status.expected_shares, 5);
    assert_eq!(status.actual_shares, 3);
    assert_eq!(status.expected_action_count, 2);
    assert_eq!(status.actual_action_count, 1);
}

#[tokio::test]
async fn test_migration_status_migrated() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Auto-created: Eintritt(0) + Aufstockung(+3)
    // current_shares = shares_at_joining = 3, action_count = 0
    // expected_shares = 3, actual_shares = 3 ✓
    // expected_action_count = action_count(0) + 1 = 1, actual_action_count = 1 ✓
    let response = client
        .get(server.url(&format!(
            "/api/members/{}/actions/migration-status",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status: MigrationStatusTO = response.json().await.unwrap();
    assert_eq!(status.status, "migrated");
    assert_eq!(status.actual_shares, 3);
    assert_eq!(status.expected_shares, 3);
    assert_eq!(status.actual_action_count, 1);
    assert_eq!(status.expected_action_count, 1);
}

#[tokio::test]
async fn test_migration_status_fully_migrated() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Update member to set current_shares=5, action_count=1
    let mut updated = created.clone();
    updated.current_shares = 5;
    updated.action_count = 1;
    let response = client
        .put(server.url(&format!("/api/members/{}", member_id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Additional Aufstockung (+2)
    let aufstockung2 = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Aufstockung,
        date: time::Date::from_calendar_date(2024, time::Month::June, 1).unwrap(),
        shares_change: 2,
        transfer_member_id: None,
        effective_date: None,
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };
    client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&aufstockung2)
        .send()
        .await
        .unwrap();

    // Auto: Eintritt(0) + Aufstockung(+3), Manual: Aufstockung(+2)
    // actual_shares = 3 + 2 = 5 == current_shares (5) ✓
    // expected_action_count = action_count(1) + 1 = 2 == actual_action_count(2) ✓
    let response = client
        .get(server.url(&format!(
            "/api/members/{}/actions/migration-status",
            member_id
        )))
        .send()
        .await
        .unwrap();
    let status: MigrationStatusTO = response.json().await.unwrap();
    assert_eq!(status.status, "migrated");
    assert_eq!(status.actual_shares, 5);
    assert_eq!(status.expected_shares, 5);
    assert_eq!(status.actual_action_count, 2);
    assert_eq!(status.expected_action_count, 2);
}

#[tokio::test]
async fn test_migration_status_exact_match() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Update member to set current_shares=5, action_count=1
    let mut updated = created.clone();
    updated.current_shares = 5;
    updated.action_count = 1;
    let response = client
        .put(server.url(&format!("/api/members/{}", member_id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Additional Aufstockung (+2)
    client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&MemberActionTO {
            id: None,
            member_id,
            action_type: ActionTypeTO::Aufstockung,
            date: time::Date::from_calendar_date(2024, time::Month::June, 1).unwrap(),
            shares_change: 2,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: None,
            deleted: None,
            version: None,
        })
        .send()
        .await
        .unwrap();

    // Auto: Eintritt(0) + Aufstockung(+3), Manual: Aufstockung(+2)
    // actual_shares = 3 + 2 = 5 == current_shares (5) ✓
    // expected_action_count = action_count(1) + 1 = 2 == actual_action_count(2) ✓
    let response = client
        .get(server.url(&format!(
            "/api/members/{}/actions/migration-status",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status: MigrationStatusTO = response.json().await.unwrap();
    assert_eq!(status.status, "migrated");
    assert_eq!(status.expected_shares, 5);
    assert_eq!(status.actual_shares, 5);
    assert_eq!(status.expected_action_count, 2);
    assert_eq!(status.actual_action_count, 2);
}

#[tokio::test]
async fn test_import_auto_migration() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Import member with action_count=0 and shares_at_joining==current_shares
    // Should auto-create Eintritt + Aufstockung actions
    let xlsx = create_xlsx(
        &standard_headers(),
        &[vec![
            "1", "Müller", "Hans", "Hauptstr.", "5", "10115", "Berlin",
            "01.01.2020", "3", "3", "150", "0", "", "hans@test.de", "", "", "DE123",
        ]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Get the member
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(members[0].migrated, "Member should be migrated after auto-migration import");
    let member_id = members[0].id.unwrap();

    // Verify auto-created actions
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 2);

    // Check migration status - should be migrated
    let response = client
        .get(server.url(&format!(
            "/api/members/{}/actions/migration-status",
            member_id
        )))
        .send()
        .await
        .unwrap();
    let status: MigrationStatusTO = response.json().await.unwrap();
    assert_eq!(status.status, "migrated");
    assert_eq!(status.actual_shares, 3);
    assert_eq!(status.expected_shares, 3);
}

#[tokio::test]
async fn test_import_always_creates_eintritt_and_aufstockung() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Import member with action_count > 0 — should still auto-create Eintritt + Aufstockung
    let xlsx = create_xlsx(
        &standard_headers(),
        &[vec![
            "1", "Müller", "Hans", "Hauptstr.", "5", "10115", "Berlin",
            "01.01.2020", "3", "5", "150", "1", "", "", "", "", "",
        ]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    let member_id = members[0].id.unwrap();

    // Eintritt + Aufstockung should always be created
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 2);
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Eintritt)));
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Aufstockung) && a.shares_change == 3));
}

#[tokio::test]
async fn test_import_creates_austritt_when_exit_date_set() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let xlsx = create_xlsx(
        &standard_headers(),
        &[vec![
            "1", "Müller", "Hans", "Hauptstr.", "5", "10115", "Berlin",
            "01.01.2020", "3", "3", "150", "0", "31.12.2024", "", "", "", "",
        ]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    let member_id = members[0].id.unwrap();

    // Eintritt + Aufstockung + Austritt
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 3);
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Eintritt)));
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Aufstockung)));
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Austritt)));
}

#[tokio::test]
async fn test_import_action_count_stored() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let xlsx = create_xlsx(
        &standard_headers(),
        &[vec![
            "1", "Müller", "Hans", "Hauptstr.", "5", "10115", "Berlin",
            "01.01.2020", "3", "5", "150", "7", "", "", "", "", "",
        ]],
    );

    let part = reqwest::multipart::Part::bytes(xlsx)
        .file_name("members.xlsx")
        .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    client
        .post(server.url("/api/members/import"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members[0].action_count, 7);
}

#[tokio::test]
async fn test_austritt_with_effective_date() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let austritt = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Austritt,
        date: time::Date::from_calendar_date(2025, time::Month::June, 15).unwrap(),
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(
            time::Date::from_calendar_date(2025, time::Month::December, 31).unwrap(),
        ),
        comment: Some("Austritt per Satzung".to_string()),
        created: None,
        deleted: None,
        version: None,
    };

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&austritt)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberActionTO = response.json().await.unwrap();
    assert_eq!(
        created.effective_date,
        Some(time::Date::from_calendar_date(2025, time::Month::December, 31).unwrap())
    );
    assert_eq!(created.shares_change, 0);
}

#[tokio::test]
async fn test_action_update_version_conflict() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Create action
    let action = sample_action(member_id);
    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    let created: MemberActionTO = response.json().await.unwrap();
    let action_id = created.id.unwrap();

    // First update succeeds
    let mut updated = created.clone();
    updated.shares_change = 5;
    let response = client
        .put(server.url(&format!(
            "/api/members/{}/actions/{}",
            member_id, action_id
        )))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second update with OLD version should fail (version conflict)
    let mut stale = created.clone();
    stale.shares_change = 7;
    let response = client
        .put(server.url(&format!(
            "/api/members/{}/actions/{}",
            member_id, action_id
        )))
        .json(&stale)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// === Migrated Flag E2E Tests ===

#[tokio::test]
async fn test_migrated_flag_set_after_actions_match() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create member with shares_at_joining=3
    // Auto-creates Eintritt(0) + Aufstockung(+3)
    // current_shares = shares_at_joining = 3, action_count = 0
    // => already migrated after creation
    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Verify migrated flag is true when fetching the member
    // (migrated recalculation happens after auto-action creation)
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let fetched: MemberTO = response.json().await.unwrap();
    assert!(fetched.migrated, "Member should be migrated after creation with auto-created actions");

    // Verify migrated flag is true in member list
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(members[0].migrated, "Member should be migrated after matching actions");
}

#[tokio::test]
async fn test_migrated_flag_false_when_pending() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create member expecting 5 shares and 2 non-status actions
    let mut member = sample_member();
    member.current_shares = 5;
    member.action_count = 1; // expected_action_count = 2

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Add only one Aufstockung (+3) => actual_shares=3 != 5, actual_action_count=1 != 2
    client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&MemberActionTO {
            id: None,
            member_id,
            action_type: ActionTypeTO::Aufstockung,
            date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            shares_change: 3,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: None,
            deleted: None,
            version: None,
        })
        .send()
        .await
        .unwrap();

    // Verify migrated flag is false
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let fetched: MemberTO = response.json().await.unwrap();
    assert!(!fetched.migrated, "Member should not be migrated with mismatched actions");
}

#[tokio::test]
async fn test_migrated_flag_recalc_on_member_update() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create member with shares_at_joining=3
    // Auto-creates Eintritt(0) + Aufstockung(+3) => migrated after creation
    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Confirm migrated after creation
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let fetched: MemberTO = response.json().await.unwrap();
    assert!(fetched.migrated);

    // Now update current_shares to 5 => mismatch => migrated should become false
    let mut updated = fetched.clone();
    updated.current_shares = 5;
    let response = client
        .put(server.url(&format!("/api/members/{}", member_id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify migrated is now false
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let refetched: MemberTO = response.json().await.unwrap();
    assert!(!refetched.migrated, "Member should not be migrated after shares change");
}

// === Confirm Migration E2E Tests ===

#[tokio::test]
async fn test_confirm_migration_resolves_action_count_mismatch() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create member with shares_at_joining=3
    // Auto-creates Eintritt(0) + Aufstockung(+3), current_shares=3, action_count=0 => migrated
    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Update action_count=5 to force pending (action_count mismatch)
    let mut updated = created.clone();
    updated.action_count = 5;
    let response = client
        .put(server.url(&format!("/api/members/{}", member_id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify pending (shares match but action_count mismatch: expected=6, actual=1)
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let fetched: MemberTO = response.json().await.unwrap();
    assert!(!fetched.migrated);

    // Confirm migration
    let response = client
        .post(server.url(&format!(
            "/api/members/{}/actions/confirm-migration",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify now migrated
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let confirmed: MemberTO = response.json().await.unwrap();
    assert!(confirmed.migrated, "Member should be migrated after confirmation");
}

#[tokio::test]
async fn test_confirm_migration_shares_mismatch_stays_pending() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create member: current_shares=5, action_count=5
    let mut member = sample_member();
    member.current_shares = 5;
    member.action_count = 5;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Add Aufstockung(+3) => shares mismatch (3 != 5)
    client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&MemberActionTO {
            id: None,
            member_id,
            action_type: ActionTypeTO::Aufstockung,
            date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            shares_change: 3,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: None,
            deleted: None,
            version: None,
        })
        .send()
        .await
        .unwrap();

    // Confirm migration
    let response = client
        .post(server.url(&format!(
            "/api/members/{}/actions/confirm-migration",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Still not migrated (shares mismatch)
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let fetched: MemberTO = response.json().await.unwrap();
    assert!(!fetched.migrated, "Member should stay pending with shares mismatch");
}

#[tokio::test]
async fn test_confirm_migration_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url(&format!(
            "/api/members/{}/actions/confirm-migration",
            uuid::Uuid::new_v4()
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// === Member Document E2E Tests ===

#[tokio::test]
async fn test_document_upload_list_download_delete() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Upload a document
    let file_content = b"fake pdf content";
    let file_part = reqwest::multipart::Part::bytes(file_content.to_vec())
        .file_name("beitritt.pdf")
        .mime_str("application/pdf")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part("file", file_part);

    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let doc: MemberDocumentTO = response.json().await.unwrap();
    assert_eq!(doc.document_type, "join_declaration");
    assert_eq!(doc.file_name, "beitritt.pdf");
    assert_eq!(doc.mime_type, "application/pdf");
    assert!(doc.id.is_some());
    let doc_id = doc.id.unwrap();

    // List documents
    let response = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let docs: Vec<MemberDocumentTO> = response.json().await.unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].file_name, "beitritt.pdf");

    // Download document
    let response = client
        .get(server.url(&format!(
            "/api/members/{}/documents/{}",
            member_id, doc_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );
    let body = response.bytes().await.unwrap();
    assert_eq!(body.as_ref(), file_content);

    // Delete document
    let response = client
        .delete(server.url(&format!(
            "/api/members/{}/documents/{}",
            member_id, doc_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify deleted - list should be empty
    let response = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .unwrap();
    let docs: Vec<MemberDocumentTO> = response.json().await.unwrap();
    assert!(docs.is_empty());
}

#[tokio::test]
async fn test_document_singleton_blocks_duplicate() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Upload first join_declaration
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"first".to_vec())
                .file_name("first.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Upload second join_declaration (should be blocked with 409)
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"second".to_vec())
                .file_name("second.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // List should still show only the first one
    let response = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .unwrap();
    let docs: Vec<MemberDocumentTO> = response.json().await.unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].file_name, "first.pdf");
}

#[tokio::test]
async fn test_document_singleton_allows_reupload_after_delete() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Upload first join_declaration
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"first".to_vec())
                .file_name("first.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let doc: MemberDocumentTO = response.json().await.unwrap();

    // Delete the document
    let response = client
        .delete(server.url(&format!(
            "/api/members/{}/documents/{}",
            member_id,
            doc.id.unwrap()
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Upload again (should succeed now)
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"second".to_vec())
                .file_name("second.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_document_multi_type_allows_multiple() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Upload two share_increase documents
    for i in 1..=2 {
        let form = reqwest::multipart::Form::new()
            .text("document_type", "share_increase")
            .part(
                "file",
                reqwest::multipart::Part::bytes(format!("content {}", i).into_bytes())
                    .file_name(format!("aufstockung_{}.pdf", i))
                    .mime_str("application/pdf")
                    .unwrap(),
            );
        let response = client
            .post(server.url(&format!("/api/members/{}/documents", member_id)))
            .multipart(form)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // List should show both
    let response = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .unwrap();
    let docs: Vec<MemberDocumentTO> = response.json().await.unwrap();
    assert_eq!(docs.len(), 2);
}

#[tokio::test]
async fn test_document_other_requires_description() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Upload 'other' without description should fail
    let form = reqwest::multipart::Form::new()
        .text("document_type", "other")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"content".to_vec())
                .file_name("doc.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Upload 'other' with description should succeed
    let form = reqwest::multipart::Form::new()
        .text("document_type", "other")
        .text("description", "Vollmacht")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"content".to_vec())
                .file_name("vollmacht.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let doc: MemberDocumentTO = response.json().await.unwrap();
    assert_eq!(doc.description.as_deref(), Some("Vollmacht"));
}

#[tokio::test]
async fn test_document_download_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let response = client
        .get(server.url(&format!(
            "/api/members/{}/documents/{}",
            member_id,
            uuid::Uuid::new_v4()
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_document_empty_list() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let response = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let docs: Vec<MemberDocumentTO> = response.json().await.unwrap();
    assert!(docs.is_empty());
}

// === Document Generation E2E Tests ===

#[tokio::test]
async fn test_generate_document_success() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Generate join_confirmation document
    let response = client
        .post(server.url(&format!(
            "/api/members/{}/documents/generate/join_confirmation",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let doc: MemberDocumentTO = response.json().await.unwrap();
    assert_eq!(doc.document_type, "join_confirmation");
    assert_eq!(doc.file_name, "join_confirmation_1_mustermann_max.pdf");
    assert_eq!(doc.mime_type, "application/pdf");

    // Verify document appears in list
    let response = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .unwrap();
    let docs: Vec<MemberDocumentTO> = response.json().await.unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].document_type, "join_confirmation");
}

#[tokio::test]
async fn test_generate_document_duplicate_blocked() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Generate first
    let response = client
        .post(server.url(&format!(
            "/api/members/{}/documents/generate/join_confirmation",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Generate again — should be blocked
    let response = client
        .post(server.url(&format!(
            "/api/members/{}/documents/generate/join_confirmation",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_generate_document_unknown_type() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let response = client
        .post(server.url(&format!(
            "/api/members/{}/documents/generate/nonexistent_type",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// === Auto Member Creation Tests ===

#[tokio::test]
async fn test_create_member_auto_assigns_member_number() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create first member with member_number=0 (auto-assign)
    let mut member = sample_member();
    member.member_number = 0;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created1: MemberTO = response.json().await.unwrap();
    assert_eq!(created1.member_number, 1);

    // Create second member with member_number=0
    let mut member2 = sample_member();
    member2.member_number = 0;
    member2.first_name = "Erika".to_string();

    let response = client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created2: MemberTO = response.json().await.unwrap();
    assert_eq!(created2.member_number, 2);
}

#[tokio::test]
async fn test_create_member_auto_creates_entry_actions() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.shares_at_joining = 3;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Verify auto-created actions
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert_eq!(actions.len(), 2);

    // First action should be Eintritt
    let eintritt = actions.iter().find(|a| a.action_type == ActionTypeTO::Eintritt);
    assert!(eintritt.is_some(), "Eintritt action should exist");
    let eintritt = eintritt.unwrap();
    assert_eq!(eintritt.shares_change, 0);
    assert_eq!(eintritt.date, created.join_date);

    // Second action should be Aufstockung
    let aufstockung = actions.iter().find(|a| a.action_type == ActionTypeTO::Aufstockung);
    assert!(aufstockung.is_some(), "Aufstockung action should exist");
    let aufstockung = aufstockung.unwrap();
    assert_eq!(aufstockung.shares_change, 3);
    assert_eq!(aufstockung.date, created.join_date);
}

#[tokio::test]
async fn test_create_member_sets_computed_fields() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.shares_at_joining = 5;
    // These should be overridden by the service
    member.current_shares = 99;
    member.current_balance = 999999;
    member.action_count = 42;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();

    assert_eq!(created.current_shares, 5, "current_shares should equal shares_at_joining");
    assert_eq!(created.current_balance, 0, "current_balance should be 0");
    assert_eq!(created.action_count, 0, "action_count should be 0");
}

// === Validation E2E Tests ===

#[tokio::test]
async fn test_validation_empty_database() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/validation"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: ValidationResultTO = response.json().await.unwrap();
    assert!(result.member_number_gaps.is_empty());
    assert!(result.unmatched_transfers.is_empty());
}

#[tokio::test]
async fn test_validation_detects_member_number_gaps() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create member 1
    let mut member = sample_member();
    member.member_number = 1;
    client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    // Create member 3 (skip 2)
    member.member_number = 3;
    client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/validation"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: ValidationResultTO = response.json().await.unwrap();
    assert_eq!(result.member_number_gaps, vec![2]);
}

#[tokio::test]
async fn test_validation_detects_unmatched_transfers() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create two members
    let mut member_a = sample_member();
    member_a.member_number = 1;
    let resp = client
        .post(server.url("/api/members"))
        .json(&member_a)
        .send()
        .await
        .unwrap();
    let created_a: MemberTO = resp.json().await.unwrap();
    let id_a = created_a.id.unwrap();

    let mut member_b = sample_member();
    member_b.member_number = 2;
    let resp = client
        .post(server.url("/api/members"))
        .json(&member_b)
        .send()
        .await
        .unwrap();
    let created_b: MemberTO = resp.json().await.unwrap();
    let id_b = created_b.id.unwrap();

    // Create UebertragungAbgabe for member A (without counterpart on B)
    let action = MemberActionTO {
        id: None,
        member_id: id_a,
        action_type: ActionTypeTO::UebertragungAbgabe,
        date: time::Date::from_calendar_date(2024, time::Month::May, 1).unwrap(),
        shares_change: -3,
        transfer_member_id: Some(id_b),
        effective_date: None,
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };
    let resp = client
        .post(server.url(&format!("/api/members/{}/actions", id_a)))
        .json(&action)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Validate - should find unmatched transfer
    let response = client
        .get(server.url("/api/validation"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: ValidationResultTO = response.json().await.unwrap();
    assert_eq!(result.unmatched_transfers.len(), 1);
    assert_eq!(result.unmatched_transfers[0].member_id, id_a);
    assert_eq!(result.unmatched_transfers[0].shares_change, -3);
}

#[tokio::test]
async fn test_validation_detects_shares_mismatch() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a member (service sets current_shares = shares_at_joining = 1)
    let mut member = sample_member();
    member.shares_at_joining = 3;
    let resp = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = resp.json().await.unwrap();
    let id = created.id.unwrap();

    // The service auto-creates Eintritt (shares_change=0) + Aufstockung (shares_change=shares_at_joining=3)
    // So current_shares=3 matches sum=3 -> no mismatch initially.
    // Now update current_shares to something different via update
    let mut updated = created.clone();
    updated.current_shares = 10; // mismatch: actions sum to 3

    let resp = client
        .put(server.url(&format!("/api/members/{}", id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let response = client
        .get(server.url("/api/validation"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: ValidationResultTO = response.json().await.unwrap();
    assert!(
        result.shares_mismatches.iter().any(|s| s.member_id == id && s.expected == 10 && s.actual == 3),
        "Should detect shares mismatch for member with current_shares=10 but actions sum=3"
    );
}

#[tokio::test]
async fn test_validation_detects_missing_entry_action() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a member (auto-creates Eintritt + Aufstockung)
    let member = sample_member();
    let resp = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let created: MemberTO = resp.json().await.unwrap();
    let id = created.id.unwrap();

    // Get the member's actions
    let resp = client
        .get(server.url(&format!("/api/members/{}/actions", id)))
        .send()
        .await
        .unwrap();
    let actions: Vec<MemberActionTO> = resp.json().await.unwrap();

    // Delete the Eintritt action
    let eintritt = actions.iter().find(|a| a.action_type == ActionTypeTO::Eintritt).unwrap();
    let resp = client
        .delete(server.url(&format!("/api/members/{}/actions/{}", id, eintritt.id.unwrap())))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // Validate
    let response = client
        .get(server.url("/api/validation"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: ValidationResultTO = response.json().await.unwrap();
    assert!(
        result.missing_entry_actions.iter().any(|m| m.member_id == id && m.actual_count == 0),
        "Should detect missing entry action"
    );
}

#[tokio::test]
async fn test_join_date_derived_from_eintritt_action() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.join_date = time::Date::from_calendar_date(2024, time::Month::June, 15).unwrap();

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Reload member to check derived join_date
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let loaded: MemberTO = response.json().await.unwrap();
    assert_eq!(
        loaded.join_date,
        time::Date::from_calendar_date(2024, time::Month::June, 15).unwrap(),
        "join_date should be derived from Eintritt action date"
    );
}

#[tokio::test]
async fn test_exit_date_derived_from_austritt_action() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Create Austritt action with effective_date
    let austritt = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Austritt,
        date: time::Date::from_calendar_date(2024, time::Month::June, 15).unwrap(),
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(
            time::Date::from_calendar_date(2024, time::Month::December, 31).unwrap(),
        ),
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&austritt)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Reload member and check exit_date
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let loaded: MemberTO = response.json().await.unwrap();
    assert_eq!(
        loaded.exit_date,
        Some(time::Date::from_calendar_date(2024, time::Month::December, 31).unwrap()),
        "exit_date should be derived from Austritt effective_date"
    );
}

#[tokio::test]
async fn test_austritt_without_effective_date_fails() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Try to create Austritt without effective_date
    let austritt = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Austritt,
        date: time::Date::from_calendar_date(2024, time::Month::June, 15).unwrap(),
        shares_change: 0,
        transfer_member_id: None,
        effective_date: None,
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&austritt)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Austritt without effective_date should be rejected"
    );
}

#[tokio::test]
async fn test_exit_date_cleared_when_austritt_deleted() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Create Austritt action
    let austritt = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Austritt,
        date: time::Date::from_calendar_date(2024, time::Month::June, 15).unwrap(),
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(
            time::Date::from_calendar_date(2024, time::Month::December, 31).unwrap(),
        ),
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&austritt)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created_austritt: MemberActionTO = response.json().await.unwrap();
    let action_id = created_austritt.id.unwrap();

    // Verify exit_date is set
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let loaded: MemberTO = response.json().await.unwrap();
    assert!(loaded.exit_date.is_some(), "exit_date should be set after Austritt");

    // Delete the Austritt action
    let response = client
        .delete(server.url(&format!(
            "/api/members/{}/actions/{}",
            member_id, action_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify exit_date is cleared
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .unwrap();
    let loaded: MemberTO = response.json().await.unwrap();
    assert_eq!(
        loaded.exit_date, None,
        "exit_date should be None after Austritt action is deleted"
    );
}

// ===== Template API Tests =====

use genossi_rest::RestStateDef;
use genossi_service::template::FileTreeEntry;

async fn setup_with_templates() -> genossi_rest::test_server::test_support::TestServer {
    let pool = Arc::new(
        SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database"),
    );

    sqlx::migrate!("../migrations/sqlite")
        .run(&*pool)
        .await
        .expect("Failed to run migrations");

    let rest_state = RestStateImpl::new(pool);

    // Provision default templates
    rest_state
        .template_storage()
        .provision_defaults()
        .await
        .expect("Failed to provision default templates");

    start_test_server(rest_state).await
}

#[tokio::test]
async fn test_template_list() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/templates"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let tree: Vec<FileTreeEntry> = response.json().await.unwrap();
    // Should have default templates
    assert!(!tree.is_empty());
}

#[tokio::test]
async fn test_template_crud() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    // Create a new template
    let response = client
        .put(server.url("/api/templates/test_template.typ"))
        .body("Hello #sys.inputs.at(\"member\")")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Read it back
    let response = client
        .get(server.url("/api/templates/test_template.typ"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content = response.text().await.unwrap();
    assert_eq!(content, "Hello #sys.inputs.at(\"member\")");

    // Update it
    let response = client
        .put(server.url("/api/templates/test_template.typ"))
        .body("Updated content")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Read updated
    let response = client
        .get(server.url("/api/templates/test_template.typ"))
        .send()
        .await
        .unwrap();
    let content = response.text().await.unwrap();
    assert_eq!(content, "Updated content");

    // Delete it
    let response = client
        .delete(server.url("/api/templates/test_template.typ"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify deleted
    let response = client
        .get(server.url("/api/templates/test_template.typ"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_template_read_nonexistent() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/templates/nonexistent.typ"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_template_path_traversal() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/templates/..%2F..%2Fetc%2Fpasswd"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_template_render_pdf() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    // First create a member
    let member = sample_member();
    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Create a simple template
    let template = r#"
#set page(paper: "a4")
#let member = json.decode(sys.inputs.at("member"))
Hello #member.first_name #member.last_name
"#;
    client
        .put(server.url("/api/templates/simple.typ"))
        .body(template)
        .send()
        .await
        .unwrap();

    // Render it
    let response = client
        .post(server.url(&format!(
            "/api/templates/render/simple.typ/{}",
            member_id
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );
    let bytes = response.bytes().await.unwrap();
    assert!(bytes.starts_with(b"%PDF"));
}

#[tokio::test]
async fn test_template_render_compilation_error() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    // Create a member
    let member = sample_member();
    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Create a broken template
    client
        .put(server.url("/api/templates/broken.typ"))
        .body("#let x = \n// broken")
        .send()
        .await
        .unwrap();

    // Try to render
    let response = client
        .post(server.url(&format!(
            "/api/templates/render/broken.typ/{}",
            member_id
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_template_render_nonexistent_member() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    // Create a valid template
    client
        .put(server.url("/api/templates/valid.typ"))
        .body("#set page(paper: \"a4\")\nHello")
        .send()
        .await
        .unwrap();

    // Render with non-existent member
    let fake_id = uuid::Uuid::new_v4();
    let response = client
        .post(server.url(&format!(
            "/api/templates/render/valid.typ/{}",
            fake_id
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_template_subdirectory() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();

    // Create a template in a subdirectory (directory created automatically)
    let response = client
        .put(server.url("/api/templates/vorstand/einladung.typ"))
        .body("Einladung content")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Read it back
    let response = client
        .get(server.url("/api/templates/vorstand/einladung.typ"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content = response.text().await.unwrap();
    assert_eq!(content, "Einladung content");

    // List should show the directory
    let response = client
        .get(server.url("/api/templates"))
        .send()
        .await
        .unwrap();
    let tree: Vec<FileTreeEntry> = response.json().await.unwrap();
    let has_vorstand = tree.iter().any(|e| {
        matches!(e, FileTreeEntry::Directory { name, .. } if name == "vorstand")
    });
    assert!(has_vorstand, "Should have vorstand directory in tree");
}

// ============================================================
// Config E2E Tests
// ============================================================

#[tokio::test]
async fn test_config_get_all_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let entries: Vec<ConfigEntryTO> = response.json().await.unwrap();
    // Migration seeds mail_send_interval_seconds, so filter it out
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|e| e.key != "mail_send_interval_seconds")
        .collect();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn test_config_set_and_get() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set a config entry
    let response = client
        .put(server.url("/api/config/smtp_host"))
        .json(&SetConfigRequest {
            value: "mail.example.com".to_string(),
            value_type: "string".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let entry: ConfigEntryTO = response.json().await.unwrap();
    assert_eq!(entry.key, "smtp_host");
    assert_eq!(entry.value, "mail.example.com");
    assert_eq!(entry.value_type, "string");

    // Get all and verify
    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let entries: Vec<ConfigEntryTO> = response.json().await.unwrap();
    let entry = entries.iter().find(|e| e.key == "smtp_host").expect("smtp_host entry not found");
    assert_eq!(entry.value, "mail.example.com");
}

#[tokio::test]
async fn test_config_upsert() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set initial value
    client
        .put(server.url("/api/config/smtp_port"))
        .json(&SetConfigRequest {
            value: "587".to_string(),
            value_type: "int".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Update to new value
    let response = client
        .put(server.url("/api/config/smtp_port"))
        .json(&SetConfigRequest {
            value: "465".to_string(),
            value_type: "int".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify updated
    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();
    let entries: Vec<ConfigEntryTO> = response.json().await.unwrap();
    let entry = entries.iter().find(|e| e.key == "smtp_port").expect("smtp_port entry not found");
    assert_eq!(entry.value, "465");
}

#[tokio::test]
async fn test_config_delete() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create entry
    client
        .put(server.url("/api/config/test_key"))
        .json(&SetConfigRequest {
            value: "test_value".to_string(),
            value_type: "string".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Delete it
    let response = client
        .delete(server.url("/api/config/test_key"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify gone
    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();
    let entries: Vec<ConfigEntryTO> = response.json().await.unwrap();
    assert!(entries.iter().all(|e| e.key != "test_key"));
}

#[tokio::test]
async fn test_config_delete_nonexistent() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .delete(server.url("/api/config/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_config_secret_masking() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set a secret
    let response = client
        .put(server.url("/api/config/smtp_pass"))
        .json(&SetConfigRequest {
            value: "supersecretpassword".to_string(),
            value_type: "secret".to_string(),
        })
        .send()
        .await
        .unwrap();

    // The set response returns the value as-is (not masked) since user just provided it
    assert_eq!(response.status(), StatusCode::OK);

    // But GET all should mask it
    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();
    let entries: Vec<ConfigEntryTO> = response.json().await.unwrap();
    let entry = entries.iter().find(|e| e.key == "smtp_pass").expect("smtp_pass entry not found");
    assert_eq!(entry.value, "***");
    assert_eq!(entry.value_type, "secret");
}

#[tokio::test]
async fn test_config_validation_invalid_int() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .put(server.url("/api/config/smtp_port"))
        .json(&SetConfigRequest {
            value: "not_a_number".to_string(),
            value_type: "int".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_config_validation_invalid_bool() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .put(server.url("/api/config/some_flag"))
        .json(&SetConfigRequest {
            value: "yes".to_string(),
            value_type: "bool".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================
// Mail E2E Tests
// ============================================================

#[tokio::test]
async fn test_mail_jobs_list_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/mail/jobs"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let jobs: Vec<MailJobTO> = response.json().await.unwrap();
    assert!(jobs.is_empty());
}

#[tokio::test]
async fn test_mail_create_bulk_job() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create bulk mail job (no SMTP config needed for job creation)
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient { address: "alice@example.com".to_string(), member_id: None },
                BulkRecipient { address: "bob@example.com".to_string(), member_id: None },
                BulkRecipient { address: "carol@example.com".to_string(), member_id: None },
            ],
            subject: "Bulk Test".to_string(),
            body: "Hello everyone".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();
    assert_eq!(job.subject, "Bulk Test");
    assert_eq!(job.status, "running");
    assert_eq!(job.total_count, 3);
    assert_eq!(job.sent_count, 0);
    assert_eq!(job.failed_count, 0);

    // Verify job appears in job list
    let response = client
        .get(server.url("/api/mail/jobs"))
        .send()
        .await
        .unwrap();
    let jobs: Vec<MailJobTO> = response.json().await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].total_count, 3);

    // Verify job detail shows recipients
    let response = client
        .get(server.url(&format!("/api/mail/jobs/{}", job.id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let detail: MailJobDetailTO = response.json().await.unwrap();
    assert_eq!(detail.recipients.len(), 3);
    assert_eq!(detail.recipients[0].to_address, "alice@example.com");
    assert_eq!(detail.recipients[1].to_address, "bob@example.com");
    assert_eq!(detail.recipients[2].to_address, "carol@example.com");
    for r in &detail.recipients {
        assert_eq!(r.status, "pending");
    }
}

#[tokio::test]
async fn test_mail_create_single_job() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/mail/send"))
        .json(&SendMailRequest {
            to_address: "user@example.com".to_string(),
            subject: "Single Test".to_string(),
            body: "Hello".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();
    assert_eq!(job.total_count, 1);
    assert_eq!(job.status, "running");
}

#[tokio::test]
async fn test_mail_send_bulk_empty_list() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![],
            subject: "Empty".to_string(),
            body: "Body".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Empty recipients should return 500 (DataAccess error)
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_mail_retry_job() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a job
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient { address: "a@example.com".to_string(), member_id: None },
                BulkRecipient { address: "b@example.com".to_string(), member_id: None },
            ],
            subject: "Retry Test".to_string(),
            body: "Hello".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();

    // Retry (no failed recipients yet, so no-op)
    let response = client
        .post(server.url(&format!("/api/mail/jobs/{}/retry", job.id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let retried: MailJobTO = response.json().await.unwrap();
    assert_eq!(retried.status, "running");
}

#[tokio::test]
async fn test_mail_job_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/mail/jobs/00000000-0000-0000-0000-000000000000"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_mail_test_missing_config() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/mail/test"))
        .json(&TestMailRequest {
            to_address: "admin@example.com".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mail_test_with_config() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set up SMTP config pointing to unreachable server
    for (key, value, vtype) in [
        ("smtp_host", "127.0.0.1", "string"),
        ("smtp_port", "19999", "int"),
        ("smtp_user", "user", "string"),
        ("smtp_pass", "pass", "secret"),
        ("smtp_from", "sender@example.com", "string"),
        ("smtp_tls", "none", "string"),
    ] {
        client
            .put(server.url(&format!("/api/config/{}", key)))
            .json(&SetConfigRequest {
                value: value.to_string(),
                value_type: vtype.to_string(),
            })
            .send()
            .await
            .unwrap();
    }

    let response = client
        .post(server.url("/api/mail/test"))
        .json(&TestMailRequest {
            to_address: "test@example.com".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Test mail with unreachable server returns 502 (SMTP error)
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

// ============================================================
// Members Not Reached By Mail Job E2E Tests
// ============================================================

async fn setup_with_pool() -> (
    genossi_rest::test_server::test_support::TestServer,
    Arc<SqlitePool>,
) {
    let pool = Arc::new(
        SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database"),
    );

    sqlx::migrate!("../migrations/sqlite")
        .run(&*pool)
        .await
        .expect("Failed to run migrations");

    let rest_state = RestStateImpl::new(pool.clone());
    let server = start_test_server(rest_state).await;
    (server, pool)
}

#[tokio::test]
async fn test_members_not_reached_by_job() {
    let (server, pool) = setup_with_pool().await;
    let client = reqwest::Client::new();

    // Create 3 members
    let mut member1 = sample_member();
    member1.email = Some("alice@example.com".to_string());
    member1.first_name = "Alice".to_string();
    member1.member_number = 1;

    let mut member2 = sample_member();
    member2.email = Some("bob@example.com".to_string());
    member2.first_name = "Bob".to_string();
    member2.member_number = 2;

    let mut member3 = sample_member();
    member3.email = None; // No email
    member3.first_name = "Carol".to_string();
    member3.member_number = 3;

    let m1: MemberTO = client
        .post(server.url("/api/members"))
        .json(&member1)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let m2: MemberTO = client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let _m3: MemberTO = client
        .post(server.url("/api/members"))
        .json(&member3)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Create bulk mail job with member_ids for Alice (sent) and Bob (failed)
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient {
                    address: "alice@example.com".to_string(),
                    member_id: m1.id.map(|id| id.to_string()),
                },
                BulkRecipient {
                    address: "bob@example.com".to_string(),
                    member_id: m2.id.map(|id| id.to_string()),
                },
            ],
            subject: "GV Einladung".to_string(),
            body: "Einladung zur Generalversammlung".to_string(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();

    // Directly update recipient statuses in DB: Alice=sent, Bob=failed
    let alice_member_id = m1.id.unwrap().as_bytes().to_vec();
    let bob_member_id = m2.id.unwrap().as_bytes().to_vec();

    sqlx::query("UPDATE mail_recipients SET status = 'sent' WHERE member_id = ?")
        .bind(&alice_member_id)
        .execute(&*pool)
        .await
        .unwrap();
    sqlx::query("UPDATE mail_recipients SET status = 'failed', error = 'Connection refused' WHERE member_id = ?")
        .bind(&bob_member_id)
        .execute(&*pool)
        .await
        .unwrap();

    // Query not-reached-by endpoint
    let response = client
        .get(server.url(&format!("/api/members/not-reached-by/{}", job.id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let not_reached: Vec<MemberTO> = response.json().await.unwrap();

    // Bob (failed) and Carol (not in job) should be in the list
    // Alice (sent) should NOT be in the list
    assert_eq!(not_reached.len(), 2);
    let names: Vec<&str> = not_reached.iter().map(|m| m.first_name.as_str()).collect();
    assert!(names.contains(&"Bob"), "Bob (failed) should be not-reached");
    assert!(
        names.contains(&"Carol"),
        "Carol (no email, not in job) should be not-reached"
    );
    assert!(
        !names.contains(&"Alice"),
        "Alice (sent) should NOT be not-reached"
    );
}

#[tokio::test]
async fn test_members_not_reached_sent_excluded() {
    let (server, pool) = setup_with_pool().await;
    let client = reqwest::Client::new();

    // Create 1 member
    let mut member = sample_member();
    member.email = Some("only@example.com".to_string());
    member.first_name = "Only".to_string();

    let created: MemberTO = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Create mail job with this member
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![BulkRecipient {
                address: "only@example.com".to_string(),
                member_id: created.id.map(|id| id.to_string()),
            }],
            subject: "Test".to_string(),
            body: "Test".to_string(),
        })
        .send()
        .await
        .unwrap();
    let job: MailJobTO = response.json().await.unwrap();

    // Mark as sent
    let member_id_bytes = created.id.unwrap().as_bytes().to_vec();
    sqlx::query("UPDATE mail_recipients SET status = 'sent' WHERE member_id = ?")
        .bind(&member_id_bytes)
        .execute(&*pool)
        .await
        .unwrap();

    // Query not-reached: should be empty since only member was reached
    let response = client
        .get(server.url(&format!("/api/members/not-reached-by/{}", job.id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let not_reached: Vec<MemberTO> = response.json().await.unwrap();
    assert!(
        not_reached.is_empty(),
        "All members were reached, list should be empty"
    );
}

#[tokio::test]
async fn test_members_not_reached_invalid_job_id() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url(
            "/api/members/not-reached-by/00000000-0000-0000-0000-000000000000",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ===== User Preferences E2E Tests =====

#[tokio::test]
async fn test_get_user_preference_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/user-preferences/member_list_columns"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_upsert_user_preference_create() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: r#"["member_number","last_name","first_name"]"#.to_string(),
        created: None,
        version: None,
    };

    let response = client
        .put(server.url("/api/user-preferences/member_list_columns"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let result: UserPreferenceTO = response.json().await.unwrap();
    assert!(result.id.is_some());
    assert_eq!(result.key.as_deref(), Some("member_list_columns"));
    assert_eq!(
        result.value,
        r#"["member_number","last_name","first_name"]"#
    );
    assert!(result.version.is_some());
}

#[tokio::test]
async fn test_upsert_user_preference_update() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create
    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: r#"["member_number","last_name"]"#.to_string(),
        created: None,
        version: None,
    };
    let response = client
        .put(server.url("/api/user-preferences/member_list_columns"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: UserPreferenceTO = response.json().await.unwrap();

    // Update
    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: r#"["member_number","last_name","city"]"#.to_string(),
        created: None,
        version: None,
    };
    let response = client
        .put(server.url("/api/user-preferences/member_list_columns"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: UserPreferenceTO = response.json().await.unwrap();

    assert_eq!(updated.id, created.id);
    assert_eq!(
        updated.value,
        r#"["member_number","last_name","city"]"#
    );
    // Version should change on update
    assert_ne!(updated.version, created.version);
}

#[tokio::test]
async fn test_get_user_preference_after_upsert() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create preference
    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: r#"["member_number","last_name"]"#.to_string(),
        created: None,
        version: None,
    };
    client
        .put(server.url("/api/user-preferences/member_list_columns"))
        .json(&body)
        .send()
        .await
        .unwrap();

    // Get it back
    let response = client
        .get(server.url("/api/user-preferences/member_list_columns"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let result: UserPreferenceTO = response.json().await.unwrap();
    assert_eq!(result.key.as_deref(), Some("member_list_columns"));
    assert_eq!(result.value, r#"["member_number","last_name"]"#);
}
