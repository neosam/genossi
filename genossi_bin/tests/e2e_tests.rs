#![cfg(feature = "mock_auth")]

use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::start_test_server;
use genossi_rest_types::{
    ActionTypeTO, AdminCreateApplicationRequest, ApplicationStatusTO, ApplicationTO,
    MemberActionTO, MemberDocumentTO, MemberImportResultTO, MemberTO, MigrationStatusTO,
    PublicJoinRequest, PublicJoinResponse, SalutationTO, UserPreferenceTO, ValidationResultTO,
};
use std::collections::HashMap;
use genossi_config::rest::{ConfigEntryTO, SetConfigRequest};
use genossi_mail::rest::{SendBulkMailRequest, BulkRecipient, SendMailRequest, MailJobTO, MailJobDetailTO, TestMailRequest};
use genossi_rest::mail_footer::FooterResponse;
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
        salutation: None,
        title: None,
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
        status: genossi_rest_types::MemberStatusTO::Normal,
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

    // Create members for recipients
    let mut m1 = sample_member();
    m1.member_number = 1;
    m1.first_name = "Alice".to_string();
    m1.email = Some("alice@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m1).send().await.unwrap();
    let created1: MemberTO = resp.json().await.unwrap();
    let id1 = created1.id.unwrap();

    let mut m2 = sample_member();
    m2.member_number = 2;
    m2.first_name = "Bob".to_string();
    m2.email = Some("bob@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m2).send().await.unwrap();
    let created2: MemberTO = resp.json().await.unwrap();
    let id2 = created2.id.unwrap();

    let mut m3 = sample_member();
    m3.member_number = 3;
    m3.first_name = "Carol".to_string();
    m3.email = Some("carol@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m3).send().await.unwrap();
    let created3: MemberTO = resp.json().await.unwrap();
    let id3 = created3.id.unwrap();

    // Create bulk mail job
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient { address: "alice@example.com".to_string(), member_id: Some(id1.to_string()) },
                BulkRecipient { address: "bob@example.com".to_string(), member_id: Some(id2.to_string()) },
                BulkRecipient { address: "carol@example.com".to_string(), member_id: Some(id3.to_string()) },
            ],
            subject: "Bulk Test".to_string(),
            body: "Hello everyone".to_string(),
            attachment_ids: vec![],
            static_document_ids: vec![],
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
            attachment_ids: vec![],
            static_document_ids: vec![],
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

    // Create members
    let mut m1 = sample_member();
    m1.member_number = 1;
    m1.first_name = "Alice".to_string();
    m1.email = Some("a@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m1).send().await.unwrap();
    let c1: MemberTO = resp.json().await.unwrap();

    let mut m2 = sample_member();
    m2.member_number = 2;
    m2.first_name = "Bob".to_string();
    m2.email = Some("b@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m2).send().await.unwrap();
    let c2: MemberTO = resp.json().await.unwrap();

    // Create a job
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient { address: "a@example.com".to_string(), member_id: Some(c1.id.unwrap().to_string()) },
                BulkRecipient { address: "b@example.com".to_string(), member_id: Some(c2.id.unwrap().to_string()) },
            ],
            subject: "Retry Test".to_string(),
            body: "Hello".to_string(),
            attachment_ids: vec![],
            static_document_ids: vec![],
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
            attachment_ids: vec![],
            static_document_ids: vec![],
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
            attachment_ids: vec![],
            static_document_ids: vec![],
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

// ===== Mail Attachment E2E Tests =====

async fn upload_test_document(
    client: &reqwest::Client,
    server: &genossi_rest::test_server::test_support::TestServer,
    member_id: uuid::Uuid,
) -> MemberDocumentTO {
    let file_content = b"fake pdf content for attachment";
    let file_part = reqwest::multipart::Part::bytes(file_content.to_vec())
        .file_name("test_attachment.pdf")
        .mime_str("application/pdf")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_confirmation")
        .part("file", file_part);

    let response = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.unwrap()
}

#[tokio::test]
async fn test_mail_send_with_attachment() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();
    let doc = upload_test_document(&client, &server, member_id).await;
    let doc_id = doc.id.unwrap();

    // Send mail with attachment
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![BulkRecipient {
                address: "max@example.com".to_string(),
                member_id: Some(member_id.to_string()),
            }],
            subject: "With Attachment".to_string(),
            body: "See attached".to_string(),
            attachment_ids: vec![doc_id.to_string()],
            static_document_ids: vec![],
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();
    assert_eq!(job.total_count, 1);

    // Verify attachment shows in job detail
    let response = client
        .get(server.url(&format!("/api/mail/jobs/{}", job.id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let detail: MailJobDetailTO = response.json().await.unwrap();
    assert_eq!(detail.recipients.len(), 1);
}

#[tokio::test]
async fn test_mail_attachment_wrong_member() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create two members
    let member1 = create_test_member(&client, &server).await;
    let member1_id = member1.id.unwrap();

    let mut member2_data = sample_member();
    member2_data.member_number = 2;
    member2_data.first_name = "Other".to_string();
    member2_data.email = Some("other@example.com".to_string());
    let response = client
        .post(server.url("/api/members"))
        .json(&member2_data)
        .send()
        .await
        .unwrap();
    let member2: MemberTO = response.json().await.unwrap();
    let member2_id = member2.id.unwrap();

    // Upload document to member1
    let doc = upload_test_document(&client, &server, member1_id).await;
    let doc_id = doc.id.unwrap();

    // Try to send to member2 with member1's document — should fail
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![BulkRecipient {
                address: "other@example.com".to_string(),
                member_id: Some(member2_id.to_string()),
            }],
            subject: "Wrong Attachment".to_string(),
            body: "This should fail".to_string(),
            attachment_ids: vec![doc_id.to_string()],
            static_document_ids: vec![],
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 500); // DataAccess error -> 500
}

#[tokio::test]
async fn test_mail_attachments_rejected_for_multiple_recipients() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();
    let doc = upload_test_document(&client, &server, member_id).await;
    let doc_id = doc.id.unwrap();

    // Create a second member
    let mut m2 = sample_member();
    m2.member_number = 2;
    m2.first_name = "Other".to_string();
    m2.email = Some("other@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m2).send().await.unwrap();
    let c2: MemberTO = resp.json().await.unwrap();

    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient {
                    address: "max@example.com".to_string(),
                    member_id: Some(member_id.to_string()),
                },
                BulkRecipient {
                    address: "other@example.com".to_string(),
                    member_id: Some(c2.id.unwrap().to_string()),
                },
            ],
            subject: "Multi + Attachment".to_string(),
            body: "Should fail".to_string(),
            attachment_ids: vec![doc_id.to_string()],
            static_document_ids: vec![],
        })
        .send()
        .await
        .unwrap();

    // Attachments are only supported for single-recipient sends -> 400 (TemplateValidation captures this before service layer)
    assert!(response.status() == 400 || response.status() == 500);
}

#[tokio::test]
async fn test_mail_without_attachment_unchanged() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create members
    let mut m1 = sample_member();
    m1.member_number = 1;
    m1.first_name = "Alice".to_string();
    m1.email = Some("a@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m1).send().await.unwrap();
    let c1: MemberTO = resp.json().await.unwrap();

    let mut m2 = sample_member();
    m2.member_number = 2;
    m2.first_name = "Bob".to_string();
    m2.email = Some("b@example.com".to_string());
    let resp = client.post(server.url("/api/members")).json(&m2).send().await.unwrap();
    let c2: MemberTO = resp.json().await.unwrap();

    // Send mail without attachments (existing behavior)
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![
                BulkRecipient { address: "a@example.com".to_string(), member_id: Some(c1.id.unwrap().to_string()) },
                BulkRecipient { address: "b@example.com".to_string(), member_id: Some(c2.id.unwrap().to_string()) },
            ],
            subject: "No Attachments".to_string(),
            body: "Plain mail".to_string(),
            attachment_ids: vec![],
            static_document_ids: vec![],
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();
    assert_eq!(job.total_count, 2);
    assert_eq!(job.status, "running");
}

#[tokio::test]
async fn test_create_member_with_salutation_and_title() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.salutation = Some(SalutationTO::Herr);
    member.title = Some("Dr.".to_string());

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    assert_eq!(created.salutation, Some(SalutationTO::Herr));
    assert_eq!(created.title.as_deref(), Some("Dr."));

    // Read back
    let id = created.id.unwrap();
    let response = client
        .get(server.url(&format!("/api/members/{}", id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let fetched: MemberTO = response.json().await.unwrap();
    assert_eq!(fetched.salutation, Some(SalutationTO::Herr));
    assert_eq!(fetched.title.as_deref(), Some("Dr."));
}

#[tokio::test]
async fn test_update_member_salutation_and_title() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create without salutation/title
    let response = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();

    let created: MemberTO = response.json().await.unwrap();
    let id = created.id.unwrap();
    assert_eq!(created.salutation, None);
    assert_eq!(created.title, None);

    // Update with salutation and title
    let mut updated = created.clone();
    updated.salutation = Some(SalutationTO::Frau);
    updated.title = Some("Prof. Dr.".to_string());

    let response = client
        .put(server.url(&format!("/api/members/{}", id)))
        .json(&updated)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: MemberTO = response.json().await.unwrap();
    assert_eq!(result.salutation, Some(SalutationTO::Frau));
    assert_eq!(result.title.as_deref(), Some("Prof. Dr."));
}

#[tokio::test]
async fn test_create_member_with_firma_salutation() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.salutation = Some(SalutationTO::Firma);
    member.title = None;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    assert_eq!(created.salutation, Some(SalutationTO::Firma));
    assert_eq!(created.title, None);
}

// ===== Public Member Count Tests =====

#[tokio::test]
async fn test_public_member_count_403_when_config_not_set() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/public/member-count"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_public_member_count_403_when_config_false() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set config to false
    client
        .put(server.url("/api/config/public_stats_enabled"))
        .json(&SetConfigRequest {
            value: "false".to_string(),
            value_type: "bool".to_string(),
        })
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/public/member-count"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_public_member_count_returns_count_when_enabled() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Enable public stats
    client
        .put(server.url("/api/config/public_stats_enabled"))
        .json(&SetConfigRequest {
            value: "true".to_string(),
            value_type: "bool".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Create two members
    let member1 = sample_member();
    client
        .post(server.url("/api/members"))
        .json(&member1)
        .send()
        .await
        .unwrap();

    let mut member2 = sample_member();
    member2.member_number = 2;
    member2.first_name = "Erika".to_string();
    client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/public/member-count"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["count"], 2);
}

#[tokio::test]
async fn test_public_member_count_excludes_exited_and_deleted() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Enable public stats
    client
        .put(server.url("/api/config/public_stats_enabled"))
        .json(&SetConfigRequest {
            value: "true".to_string(),
            value_type: "bool".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Create active member
    let member1 = sample_member();
    client
        .post(server.url("/api/members"))
        .json(&member1)
        .send()
        .await
        .unwrap();

    // Create member that will exit
    let mut member2 = sample_member();
    member2.member_number = 2;
    member2.first_name = "Exited".to_string();
    let response = client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap();
    let created2: MemberTO = response.json().await.unwrap();

    // Create Austritt action with past date to make member2 exited
    let austritt = MemberActionTO {
        id: None,
        member_id: created2.id.clone().unwrap(),
        action_type: ActionTypeTO::Austritt,
        date: time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap(),
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap()),
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };
    client
        .post(server.url(&format!("/api/members/{}/actions", created2.id.unwrap())))
        .json(&austritt)
        .send()
        .await
        .unwrap();

    // Create and delete a member
    let mut member3 = sample_member();
    member3.member_number = 3;
    member3.first_name = "Deleted".to_string();
    let response = client
        .post(server.url("/api/members"))
        .json(&member3)
        .send()
        .await
        .unwrap();
    let created3: MemberTO = response.json().await.unwrap();

    client
        .delete(server.url(&format!("/api/members/{}", created3.id.unwrap())))
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/public/member-count"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    // Only the active member (member1) should be counted
    // member2 has past exit_date, member3 is deleted
    assert_eq!(body["count"], 1, "Only active members should be counted, got: {}", body);
}

#[tokio::test]
async fn test_public_member_count_no_auth_required() {
    let server = setup().await;
    // Use a plain client with no auth headers
    let client = reqwest::Client::new();

    // Enable public stats
    client
        .put(server.url("/api/config/public_stats_enabled"))
        .json(&SetConfigRequest {
            value: "true".to_string(),
            value_type: "bool".to_string(),
        })
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/public/member-count"))
        .send()
        .await
        .unwrap();

    // Should succeed without any auth
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["count"], 0);
}

// ----- Static Documents -----

async fn upload_static_document(
    server: &genossi_rest::test_server::test_support::TestServer,
    name: &str,
    filename: &str,
    content_type: &str,
    data: Vec<u8>,
) -> serde_json::Value {
    let client = reqwest::Client::new();
    let part = reqwest::multipart::Part::bytes(data)
        .file_name(filename.to_string())
        .mime_str(content_type)
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .text("name", name.to_string())
        .part("file", part);
    let response = client
        .post(server.url("/api/static-documents"))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.unwrap()
}

#[tokio::test]
async fn test_static_document_crud_happy_path() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Upload
    let pdf_bytes = b"%PDF-1.4 fake pdf content".to_vec();
    let doc = upload_static_document(
        &server,
        "Satzung",
        "satzung.pdf",
        "application/pdf",
        pdf_bytes.clone(),
    )
    .await;
    let doc_id = doc["id"].as_str().unwrap().to_string();
    assert_eq!(doc["name"], "Satzung");
    assert_eq!(doc["filename"], "satzung.pdf");
    assert_eq!(doc["content_type"], "application/pdf");
    assert_eq!(doc["size_bytes"].as_i64().unwrap(), pdf_bytes.len() as i64);

    // List
    let list: Vec<serde_json::Value> = client
        .get(server.url("/api/static-documents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], doc_id);

    // Download
    let download = client
        .get(server.url(&format!("/api/static-documents/{}", doc_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(
        download.headers().get("content-type").unwrap(),
        "application/pdf"
    );
    let disposition = download
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.contains("satzung.pdf"));
    let body_bytes = download.bytes().await.unwrap().to_vec();
    assert_eq!(body_bytes, pdf_bytes);

    // Delete
    let delete = client
        .delete(server.url(&format!("/api/static-documents/{}", doc_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);

    // List is empty, download 404
    let list: Vec<serde_json::Value> = client
        .get(server.url("/api/static-documents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(list.is_empty());
    let download = client
        .get(server.url(&format!("/api/static-documents/{}", doc_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(download.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_static_document_rejects_disallowed_content_type() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let part = reqwest::multipart::Part::bytes(vec![0xDE, 0xAD, 0xBE, 0xEF])
        .file_name("bad.exe")
        .mime_str("application/x-msdownload")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .text("name", "Bad".to_string())
        .part("file", part);
    let response = client
        .post(server.url("/api/static-documents"))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_bulk_mail_with_static_document_ids_succeeds() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a member so bulk mail has a recipient with member_id (template rendering needs it)
    let mut member = sample_member();
    member.email = Some("m@example.com".to_string());
    let create_resp = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = create_resp.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Upload a static document
    let doc = upload_static_document(
        &server,
        "Flyer",
        "flyer.pdf",
        "application/pdf",
        b"%PDF-1.4 flyer".to_vec(),
    )
    .await;
    let doc_id = doc["id"].as_str().unwrap().to_string();

    // Bulk send referencing the static document
    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![BulkRecipient {
                address: "m@example.com".to_string(),
                member_id: Some(member_id.to_string()),
            }],
            subject: "Hallo".to_string(),
            body: "Anbei die Unterlagen.".to_string(),
            attachment_ids: vec![],
            static_document_ids: vec![doc_id],
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    let job: MailJobTO = response.json().await.unwrap();
    assert_eq!(job.total_count, 1);
}

#[tokio::test]
async fn test_bulk_mail_with_unknown_static_document_id_fails() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.email = Some("m@example.com".to_string());
    let create_resp = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    let created: MemberTO = create_resp.json().await.unwrap();
    let member_id = created.id.unwrap();

    let response = client
        .post(server.url("/api/mail/send-bulk"))
        .json(&SendBulkMailRequest {
            to_addresses: vec![BulkRecipient {
                address: "m@example.com".to_string(),
                member_id: Some(member_id.to_string()),
            }],
            subject: "Hallo".to_string(),
            body: "Body".to_string(),
            attachment_ids: vec![],
            static_document_ids: vec![uuid::Uuid::new_v4().to_string()],
        })
        .send()
        .await
        .unwrap();
    // NotFound maps to 404 in mail rest error_handler
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ── Inbox E2E ────────────────────────────────────────────────────────────

/// Seed a row in `inbound_mails` directly, bypassing IMAP, so we can exercise
/// the REST side (list / detail / assign / done) without a real mail server.
async fn seed_inbound_mail(
    pool: &sqlx::SqlitePool,
    uid: i64,
    from: &str,
    subject: &str,
) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    let version = uuid::Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let now_str = now
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap();
    sqlx::query(
        "INSERT INTO inbound_mails (id, created, version, uid_validity, imap_uid, from_address, subject, received_at, body_text, has_attachments, has_html_body, raw_html_body, in_reply_to, replied, done, archived, assigned_member_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, NULL, NULL, 0, 0, 0, NULL)",
    )
    .bind(id.as_bytes().to_vec())
    .bind(&now_str)
    .bind(version.as_bytes().to_vec())
    .bind(1i64)
    .bind(uid)
    .bind(from)
    .bind(subject)
    .bind(&now_str)
    .bind("Hallo, hier meine Antwort.")
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn test_inbox_list_empty() {
    let (server, _pool) = setup_with_pool().await;
    let client = reqwest::Client::new();
    let response = client.get(server.url("/api/inbox")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_inbox_list_returns_seeded_rows() {
    let (server, pool) = setup_with_pool().await;
    seed_inbound_mail(&pool, 10, "alice@example.com", "Re: Beitrag").await;
    seed_inbound_mail(&pool, 11, "bob@example.com", "Re: Einladung").await;

    let client = reqwest::Client::new();
    let response = client.get(server.url("/api/inbox")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 2);
    let subjects: Vec<_> = body
        .iter()
        .map(|v| v["subject"].as_str().unwrap().to_string())
        .collect();
    assert!(subjects.iter().any(|s| s == "Re: Beitrag"));
    assert!(subjects.iter().any(|s| s == "Re: Einladung"));
}

#[tokio::test]
async fn test_inbox_detail() {
    let (server, pool) = setup_with_pool().await;
    let id = seed_inbound_mail(&pool, 5, "sender@example.com", "Hallo").await;
    let client = reqwest::Client::new();
    let response = client
        .get(server.url(&format!("/api/inbox/{}", id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["from_address"], "sender@example.com");
    assert_eq!(body["subject"], "Hallo");
    assert_eq!(body["replied"], false);
    assert_eq!(body["done"], false);
    assert_eq!(body["archived"], false);
    assert!(body["body_text"].as_str().unwrap().contains("Antwort"));
}

#[tokio::test]
async fn test_inbox_assign_and_unassign() {
    let (server, pool) = setup_with_pool().await;
    let client = reqwest::Client::new();

    // Create a member to assign to
    let member_resp = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();
    assert_eq!(member_resp.status(), StatusCode::OK);
    let created: MemberTO = member_resp.json().await.unwrap();
    let member_id = created.id.unwrap().to_string();

    let mail_id = seed_inbound_mail(&pool, 7, "max@example.com", "Frage").await;

    // Assign
    let r = client
        .post(server.url(&format!("/api/inbox/{}/assign", mail_id)))
        .json(&serde_json::json!({ "member_id": member_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["assigned_member_id"], member_id);
    assert!(body["assigned_member_name"]
        .as_str()
        .unwrap()
        .contains("Max"));

    // Unassign
    let r = client
        .post(server.url(&format!("/api/inbox/{}/unassign", mail_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: serde_json::Value = r.json().await.unwrap();
    assert!(body["assigned_member_id"].is_null());
}

#[tokio::test]
async fn test_inbox_done_marks_mail() {
    let (server, pool) = setup_with_pool().await;
    let id = seed_inbound_mail(&pool, 3, "spam@example.com", "Werbung").await;
    let client = reqwest::Client::new();

    let r = client
        .post(server.url(&format!("/api/inbox/{}/done", id)))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["done"], true);

    // Done mails still appear in the list (frontend filters client-side)
    let list: Vec<serde_json::Value> = client
        .get(server.url("/api/inbox"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["done"], true);
}

#[tokio::test]
async fn test_inbox_detail_not_found() {
    let (server, _pool) = setup_with_pool().await;
    let client = reqwest::Client::new();
    let id = uuid::Uuid::new_v4();
    let r = client
        .get(server.url(&format!("/api/inbox/{}", id)))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

// ============================================================
// Mail Footer E2E Tests
// ============================================================

#[tokio::test]
async fn test_mail_footer_no_config() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/mail/footer"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let footer: FooterResponse = response.json().await.unwrap();
    assert!(footer.footer.is_empty());
}

#[tokio::test]
async fn test_mail_footer_with_config_and_sender_name() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set mail_footer config
    client
        .put(server.url("/api/config/mail_footer"))
        .json(&SetConfigRequest {
            value: "Mit freundlichen Grüßen\n{{ sender_name }}".to_string(),
            value_type: "string".to_string(),
        })
        .send()
        .await
        .unwrap();

    // Set sender_name user preference
    client
        .put(server.url("/api/user-preferences/sender_name"))
        .json(&UserPreferenceTO {
            id: None,
            key: Some("sender_name".to_string()),
            value: "Anna Schmidt".to_string(),
            created: None,
            version: None,
        })
        .send()
        .await
        .unwrap();

    // Get footer
    let response = client
        .get(server.url("/api/mail/footer"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let footer: FooterResponse = response.json().await.unwrap();
    assert_eq!(footer.footer, "Mit freundlichen Grüßen\nAnna Schmidt");
}

// Admin User Preference Tests

#[tokio::test]
async fn test_admin_get_user_preference_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/permission/user/DEVUSER/preferences/sender_name"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_upsert_user_preference() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: "Max Mustermann".to_string(),
        created: None,
        version: None,
    };

    let response = client
        .put(server.url("/api/permission/user/DEVUSER/preferences/sender_name"))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let pref: UserPreferenceTO = response.json().await.unwrap();
    assert_eq!(pref.value, "Max Mustermann");
    assert_eq!(pref.key.as_deref(), Some("sender_name"));
}

#[tokio::test]
async fn test_admin_get_user_preference_after_upsert() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create preference
    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: "Test User".to_string(),
        created: None,
        version: None,
    };

    client
        .put(server.url("/api/permission/user/DEVUSER/preferences/sender_name"))
        .json(&body)
        .send()
        .await
        .unwrap();

    // Read it back
    let response = client
        .get(server.url("/api/permission/user/DEVUSER/preferences/sender_name"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let pref: UserPreferenceTO = response.json().await.unwrap();
    assert_eq!(pref.value, "Test User");
}

#[tokio::test]
async fn test_admin_upsert_user_preference_update() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: "Original Name".to_string(),
        created: None,
        version: None,
    };

    client
        .put(server.url("/api/permission/user/DEVUSER/preferences/sender_name"))
        .json(&body)
        .send()
        .await
        .unwrap();

    // Update
    let body2 = UserPreferenceTO {
        id: None,
        key: None,
        value: "Updated Name".to_string(),
        created: None,
        version: None,
    };

    let response = client
        .put(server.url("/api/permission/user/DEVUSER/preferences/sender_name"))
        .json(&body2)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let pref: UserPreferenceTO = response.json().await.unwrap();
    assert_eq!(pref.value, "Updated Name");
}

#[tokio::test]
async fn test_admin_upsert_preference_for_other_user() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set preference for "admin" user (different from DEVUSER)
    let body = UserPreferenceTO {
        id: None,
        key: None,
        value: "Admin Person".to_string(),
        created: None,
        version: None,
    };

    let response = client
        .put(server.url("/api/permission/user/admin/preferences/sender_name"))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let pref: UserPreferenceTO = response.json().await.unwrap();
    assert_eq!(pref.value, "Admin Person");

    // Read it back
    let response = client
        .get(server.url("/api/permission/user/admin/preferences/sender_name"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let pref: UserPreferenceTO = response.json().await.unwrap();
    assert_eq!(pref.value, "Admin Person");
}

// ── Communication timeline e2e tests ────────────────────────────────────

#[tokio::test]
async fn test_communication_timeline_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a member
    let resp = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let member: MemberTO = resp.json().await.unwrap();
    let member_id = member.id.unwrap();

    // Get communications — should be empty
    let resp = client
        .get(server.url(&format!(
            "/api/members/{}/communications",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn test_communication_timeline_with_outbound_and_inbound() {
    let (server, pool) = setup_with_pool().await;
    let client = reqwest::Client::new();

    // Create a member
    let resp = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let member: MemberTO = resp.json().await.unwrap();
    let member_id = member.id.unwrap();

    // Seed an outbound mail recipient linked to this member
    let job_id = uuid::Uuid::new_v4();
    let recipient_id = uuid::Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let earlier = now - time::Duration::hours(2);
    let earlier_str = earlier
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap();
    let now_str = now
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap();

    sqlx::query(
        "INSERT INTO mail_jobs (id, created, version, subject, body, status, total_count, sent_count, failed_count) \
         VALUES (?, ?, ?, ?, ?, 'done', 1, 1, 0)",
    )
    .bind(job_id.as_bytes().to_vec())
    .bind(&earlier_str)
    .bind(uuid::Uuid::new_v4().as_bytes().to_vec())
    .bind("Einladung HV")
    .bind("Liebe Mitglieder...")
    .execute(pool.as_ref())
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO mail_recipients (id, created, version, mail_job_id, to_address, member_id, status, sent_at) \
         VALUES (?, ?, ?, ?, ?, ?, 'sent', ?)",
    )
    .bind(recipient_id.as_bytes().to_vec())
    .bind(&earlier_str)
    .bind(uuid::Uuid::new_v4().as_bytes().to_vec())
    .bind(job_id.as_bytes().to_vec())
    .bind("max@example.com")
    .bind(member_id.as_bytes().to_vec())
    .bind(&earlier_str)
    .execute(pool.as_ref())
    .await
    .unwrap();

    // Seed an inbound mail assigned to this member (more recent)
    let inbound_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO inbound_mails (id, created, version, uid_validity, imap_uid, from_address, subject, received_at, body_text, has_attachments, has_html_body, replied, done, archived, assigned_member_id) \
         VALUES (?, ?, ?, 1, 99, ?, ?, ?, ?, 0, 0, 0, 1, 0, ?)",
    )
    .bind(inbound_id.as_bytes().to_vec())
    .bind(&now_str)
    .bind(uuid::Uuid::new_v4().as_bytes().to_vec())
    .bind("max@example.com")
    .bind("Re: Einladung HV")
    .bind(&now_str)
    .bind("Vielen Dank!")
    .bind(member_id.as_bytes().to_vec())
    .execute(pool.as_ref())
    .await
    .unwrap();

    // Get communications
    let resp = client
        .get(server.url(&format!(
            "/api/members/{}/communications",
            member_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(entries.len(), 2);

    // Newest first: inbound (now) then outbound (2h ago)
    assert_eq!(entries[0]["direction"], "inbound");
    assert_eq!(entries[0]["subject"], "Re: Einladung HV");
    assert_eq!(entries[0]["inbox_id"], inbound_id.to_string());
    assert_eq!(entries[0]["inbound_status"]["done"], true);
    assert_eq!(entries[0]["inbound_status"]["replied"], false);

    assert_eq!(entries[1]["direction"], "outbound");
    assert_eq!(entries[1]["subject"], "Einladung HV");
    assert_eq!(entries[1]["mail_job_id"], job_id.to_string());
    assert_eq!(entries[1]["outbound_status"], "sent");
}

#[tokio::test]
async fn test_communication_timeline_invalid_member_id() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(server.url("/api/members/not-a-uuid/communications"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// === Member Status Tests ===

#[tokio::test]
async fn test_create_member_with_fehlerhaft_erfasst_status() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.status = genossi_rest_types::MemberStatusTO::FehlerhaftErfasst;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    assert_eq!(created.status, genossi_rest_types::MemberStatusTO::FehlerhaftErfasst);
}

#[tokio::test]
async fn test_create_member_without_status_defaults_to_normal() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Send JSON without status field to test default
    let json = serde_json::json!({
        "member_number": 1,
        "first_name": "Test",
        "last_name": "User",
        "join_date": "2024-01-15",
        "shares_at_joining": 1,
        "current_shares": 1,
        "current_balance": 0
    });

    let response = client
        .post(server.url("/api/members"))
        .json(&json)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    assert_eq!(created.status, genossi_rest_types::MemberStatusTO::Normal);
}

#[tokio::test]
async fn test_update_member_status_to_fehlerhaft_erfasst() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a normal member
    let member = sample_member();
    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut created: MemberTO = response.json().await.unwrap();
    assert_eq!(created.status, genossi_rest_types::MemberStatusTO::Normal);

    // Update status to FehlerhaftErfasst
    created.status = genossi_rest_types::MemberStatusTO::FehlerhaftErfasst;
    let id = created.id.unwrap();
    let response = client
        .put(server.url(&format!("/api/members/{}", id)))
        .json(&created)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: MemberTO = response.json().await.unwrap();
    assert_eq!(updated.status, genossi_rest_types::MemberStatusTO::FehlerhaftErfasst);

    // Verify it persisted
    let response = client
        .get(server.url(&format!("/api/members/{}", id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: MemberTO = response.json().await.unwrap();
    assert_eq!(fetched.status, genossi_rest_types::MemberStatusTO::FehlerhaftErfasst);
}

#[tokio::test]
async fn test_fehlerhaft_erfasst_excluded_from_public_member_count() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Enable public member count
    let config_req = SetConfigRequest {
        value: "true".to_string(),
        value_type: "bool".to_string(),
    };
    client
        .put(server.url("/api/config/public_stats_enabled"))
        .json(&config_req)
        .send()
        .await
        .unwrap();

    // Create a normal member
    let mut member1 = sample_member();
    member1.member_number = 1;
    client
        .post(server.url("/api/members"))
        .json(&member1)
        .send()
        .await
        .unwrap();

    // Create a fehlerhaft erfasst member
    let mut member2 = sample_member();
    member2.member_number = 2;
    member2.status = genossi_rest_types::MemberStatusTO::FehlerhaftErfasst;
    client
        .post(server.url("/api/members"))
        .json(&member2)
        .send()
        .await
        .unwrap();

    // Check public member count - should be 1, not 2
    let response = client
        .get(server.url("/api/public/member-count"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["count"], 1, "FehlerhaftErfasst members should not be counted as active");
}

#[tokio::test]
async fn test_create_note_action() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let note = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Note,
        date: time::Date::from_calendar_date(2024, time::Month::April, 12).unwrap(),
        shares_change: 0,
        transfer_member_id: None,
        effective_date: None,
        comment: Some("E-Mail Adresse korrigiert".to_string()),
        created: None,
        deleted: None,
        version: None,
    };

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&note)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberActionTO = response.json().await.unwrap();
    assert!(created.id.is_some());
    assert!(matches!(created.action_type, ActionTypeTO::Note));
    assert_eq!(created.comment.as_deref(), Some("E-Mail Adresse korrigiert"));
    assert_eq!(created.shares_change, 0);

    // Verify it appears in the actions list
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert!(actions.iter().any(|a| matches!(a.action_type, ActionTypeTO::Note)));
}

#[tokio::test]
async fn test_note_action_validation_missing_comment() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let note = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Note,
        date: time::Date::from_calendar_date(2024, time::Month::April, 12).unwrap(),
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
        .json(&note)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_note_action_validation_nonzero_shares() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    let note = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Note,
        date: time::Date::from_calendar_date(2024, time::Month::April, 12).unwrap(),
        shares_change: 5,
        transfer_member_id: None,
        effective_date: None,
        comment: Some("test".to_string()),
        created: None,
        deleted: None,
        version: None,
    };

    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&note)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_fehlerhaft_erfasst_member_no_auto_actions() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let mut member = sample_member();
    member.status = genossi_rest_types::MemberStatusTO::FehlerhaftErfasst;

    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // current_shares should be 0 regardless of shares_at_joining
    assert_eq!(created.current_shares, 0);

    // No auto-created actions should exist
    let response = client
        .get(server.url(&format!("/api/members/{}/actions", member_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let actions: Vec<MemberActionTO> = response.json().await.unwrap();
    assert!(actions.is_empty());
}

#[tokio::test]
async fn test_normal_member_still_gets_auto_actions() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = sample_member();
    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    let member_id = created.id.unwrap();

    // Normal member should have Eintritt + Aufstockung auto-created
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
}

// ─── Document Counts Tests ───

#[tokio::test]
async fn test_document_counts_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/member-documents/counts?type=join_declaration"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let counts: HashMap<String, i64> = response.json().await.unwrap();
    assert!(counts.is_empty());
}

#[tokio::test]
async fn test_document_counts_with_data() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create two members
    let member1 = create_test_member(&client, &server).await;
    let member1_id = member1.id.unwrap();

    let mut member2_to = sample_member();
    member2_to.member_number = 0;
    member2_to.first_name = "Erika".to_string();
    let resp = client
        .post(server.url("/api/members"))
        .json(&member2_to)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let member2: MemberTO = resp.json().await.unwrap();
    let member2_id = member2.id.unwrap();

    // Upload join_declaration for member1
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"pdf1".to_vec())
                .file_name("be1.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let resp = client
        .post(server.url(&format!("/api/members/{}/documents", member1_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Upload join_declaration for member2
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"pdf2".to_vec())
                .file_name("be2.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let resp = client
        .post(server.url(&format!("/api/members/{}/documents", member2_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Get counts for join_declaration
    let response = client
        .get(server.url("/api/member-documents/counts?type=join_declaration"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let counts: HashMap<String, i64> = response.json().await.unwrap();
    assert_eq!(counts.len(), 2);
    assert_eq!(counts[&member1_id.to_string()], 1);
    assert_eq!(counts[&member2_id.to_string()], 1);

    // Counts for other type should be empty
    let response = client
        .get(server.url("/api/member-documents/counts?type=join_confirmation"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let counts: HashMap<String, i64> = response.json().await.unwrap();
    assert!(counts.is_empty());
}

#[tokio::test]
async fn test_document_counts_excludes_deleted() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Upload document
    let form = reqwest::multipart::Form::new()
        .text("document_type", "join_declaration")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"pdf".to_vec())
                .file_name("be.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );
    let resp = client
        .post(server.url(&format!("/api/members/{}/documents", member_id)))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let doc: MemberDocumentTO = resp.json().await.unwrap();
    let doc_id = doc.id.unwrap();

    // Delete it
    let resp = client
        .delete(server.url(&format!(
            "/api/members/{}/documents/{}",
            member_id, doc_id
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Counts should be empty
    let response = client
        .get(server.url("/api/member-documents/counts?type=join_declaration"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let counts: HashMap<String, i64> = response.json().await.unwrap();
    assert!(counts.is_empty());
}

#[tokio::test]
async fn test_document_counts_invalid_type() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/member-documents/counts?type=invalid_type"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_document_counts_missing_type() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/member-documents/counts"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ===== Backup Export Tests =====

#[tokio::test]
async fn test_backup_members_csv_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/backup/members?date=2026-04-12"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/csv"));
    let bytes = response.bytes().await.unwrap();
    // Should start with UTF-8 BOM
    assert_eq!(&bytes[..3], b"\xEF\xBB\xBF");
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("Mitgliedsnummer"));
    assert!(body.contains("Anteile am Stichtag"));
}

#[tokio::test]
async fn test_backup_members_csv_with_data() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a member (shares_at_joining=1, auto-creates Eintritt + Aufstockung(1))
    let member = create_test_member(&client, &server).await;
    let member_id = member.id.unwrap();

    // Create an additional Aufstockung action on 2025-06-01
    let action = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Aufstockung,
        date: time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap(),
        shares_change: 2,
        transfer_member_id: None,
        effective_date: None,
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };
    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&action)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Export members at a date BEFORE the extra action
    // SUM(shares_change) = 0 (Eintritt) + 1 (initial Aufstockung) = 1
    let response = client
        .get(server.url("/api/backup/members?date=2025-03-01"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    // Parse CSV to check shares_at_date column (column index 15, 0-based)
    let mut rdr = csv::Reader::from_reader(body.as_bytes());
    let record = rdr.records().next().unwrap().unwrap();
    assert_eq!(record.get(15).unwrap(), "1"); // shares_at_date

    // Export members at a date AFTER the extra action
    // SUM(shares_change) = 0 (Eintritt) + 1 (initial Aufstockung) + 2 (manual) = 3
    let response = client
        .get(server.url("/api/backup/members?date=2025-07-01"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    let mut rdr = csv::Reader::from_reader(body.as_bytes());
    let record = rdr.records().next().unwrap().unwrap();
    assert_eq!(record.get(15).unwrap(), "3"); // shares_at_date
}

#[tokio::test]
async fn test_backup_members_csv_excludes_exited() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a member, then update to set exit date
    let created = create_test_member(&client, &server).await;
    let member_id = created.id.unwrap();
    let mut updated = created.clone();
    updated.exit_date = Some(time::Date::from_calendar_date(2025, time::Month::March, 15).unwrap());
    let response = client
        .put(server.url(&format!("/api/members/{}", member_id)))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Export BEFORE exit - should include
    let response = client
        .get(server.url("/api/backup/members?date=2025-03-01"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    let rdr = csv::Reader::from_reader(body.as_bytes());
    assert_eq!(rdr.into_records().count(), 1); // 1 data row

    // Export AFTER exit - should exclude
    let response = client
        .get(server.url("/api/backup/members?date=2025-04-01"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    let rdr = csv::Reader::from_reader(body.as_bytes());
    assert_eq!(rdr.into_records().count(), 0); // 0 data rows
}

#[tokio::test]
async fn test_backup_members_csv_missing_date() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/backup/members"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_backup_actions_csv_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/backup/actions"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/csv"));
    let bytes = response.bytes().await.unwrap();
    assert_eq!(&bytes[..3], b"\xEF\xBB\xBF");
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("Mitgliedsnummer"));
    assert!(body.contains("Aktionstyp"));
}

#[tokio::test]
async fn test_backup_actions_csv_with_data() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let _member = create_test_member(&client, &server).await;

    let response = client
        .get(server.url("/api/backup/actions"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    let rdr = csv::Reader::from_reader(body.as_bytes());
    let records: Vec<_> = rdr.into_records().collect();
    // 2 auto-created actions (Eintritt + Aufstockung)
    assert_eq!(records.len(), 2);
    // Check member name is included
    assert!(body.contains("Max"));
    assert!(body.contains("Mustermann"));
}

#[tokio::test]
async fn test_backup_documents_zip_empty() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/api/backup/documents"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("application/zip"));
}

#[tokio::test]
async fn test_backup_webdav_config_persists() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Set all WebDAV backup config entries
    let entries = vec![
        ("backup_webdav_enabled", "true", "bool"),
        ("backup_webdav_url", "https://cloud.example/remote.php/dav/files/user/", "string"),
        ("backup_webdav_username", "backup-user", "string"),
        ("backup_webdav_password", "app-token-secret", "secret"),
        ("backup_webdav_directory", "genossi-export", "string"),
        ("backup_interval_hours", "12", "int"),
    ];

    for (key, value, value_type) in &entries {
        let response = client
            .put(server.url(&format!("/api/config/{}", key)))
            .json(&SetConfigRequest {
                value: value.to_string(),
                value_type: value_type.to_string(),
            })
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "Failed to set {}", key);
    }

    // Verify all entries are stored
    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let all_entries: Vec<ConfigEntryTO> = response.json().await.unwrap();

    let find_entry = |key: &str| -> Option<ConfigEntryTO> {
        all_entries.iter().find(|e| e.key == key).cloned()
    };

    // Check enabled flag
    let enabled = find_entry("backup_webdav_enabled").expect("backup_webdav_enabled not found");
    assert_eq!(enabled.value, "true");

    // Check URL
    let url = find_entry("backup_webdav_url").expect("backup_webdav_url not found");
    assert_eq!(url.value, "https://cloud.example/remote.php/dav/files/user/");

    // Check password is masked
    let password = find_entry("backup_webdav_password").expect("backup_webdav_password not found");
    assert_eq!(password.value, "***");

    // Check interval
    let interval = find_entry("backup_interval_hours").expect("backup_interval_hours not found");
    assert_eq!(interval.value, "12");
}

#[tokio::test]
async fn test_backup_webdav_config_interval_validation() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Setting non-integer should fail validation
    let response = client
        .put(server.url("/api/config/backup_interval_hours"))
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
async fn test_backup_test_webdav_missing_config() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Without any WebDAV config, the test endpoint should return 400
    let response = client
        .post(server.url("/api/backup/test-webdav"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ===== Backup Communication Tests =====

#[tokio::test]
async fn test_backup_documents_zip_contains_communications() {
    let (server, pool) = setup_with_pool().await;
    let client = reqwest::Client::new();

    // Create a member via API
    let response = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let member: MemberTO = response.json().await.unwrap();
    let member_id = member.id.unwrap();

    // Insert a sent outbound mail directly into DB
    let job_id = uuid::Uuid::new_v4();
    let recipient_id = uuid::Uuid::new_v4();
    let version_id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO mail_jobs (id, created, version, subject, body, status, total_count, sent_count, failed_count) \
         VALUES (?, '2026-03-15 14:30:00', ?, 'Willkommen', 'Hallo Max, willkommen!', 'completed', 1, 1, 0)",
    )
    .bind(job_id.as_bytes().as_slice())
    .bind(version_id.as_bytes().as_slice())
    .execute(&*pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO mail_recipients (id, created, version, mail_job_id, to_address, member_id, status, sent_at) \
         VALUES (?, '2026-03-15 14:30:00', ?, ?, 'max@example.com', ?, 'sent', '2026-03-15 14:30:05')",
    )
    .bind(recipient_id.as_bytes().as_slice())
    .bind(version_id.as_bytes().as_slice())
    .bind(job_id.as_bytes().as_slice())
    .bind(member_id.as_bytes().as_slice())
    .execute(&*pool)
    .await
    .unwrap();

    // Download the backup ZIP
    let response = client
        .get(server.url("/api/backup/documents"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let zip_bytes = response.bytes().await.unwrap();
    let reader = std::io::Cursor::new(zip_bytes.as_ref());
    let mut archive = zip::ZipArchive::new(reader).unwrap();

    // Find a communication file in the ZIP
    let mut found_communication = false;
    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        if name.contains("kommunikation/") && name.ends_with(".txt") {
            found_communication = true;
            assert!(name.contains("001_Mustermann_Max"));
            assert!(name.contains("ausgehend"));
            assert!(name.contains("Willkommen"));
            break;
        }
    }
    assert!(found_communication, "Expected communication .txt file in ZIP");
}

#[tokio::test]
async fn test_backup_documents_zip_excludes_unassigned_mails() {
    let (server, pool) = setup_with_pool().await;
    let client = reqwest::Client::new();

    // Insert a sent outbound mail WITHOUT member_id
    let job_id = uuid::Uuid::new_v4();
    let recipient_id = uuid::Uuid::new_v4();
    let version_id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO mail_jobs (id, created, version, subject, body, status, total_count, sent_count, failed_count) \
         VALUES (?, '2026-03-15 14:30:00', ?, 'Orphan Mail', 'No member assigned', 'completed', 1, 1, 0)",
    )
    .bind(job_id.as_bytes().as_slice())
    .bind(version_id.as_bytes().as_slice())
    .execute(&*pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO mail_recipients (id, created, version, mail_job_id, to_address, member_id, status, sent_at) \
         VALUES (?, '2026-03-15 14:30:00', ?, ?, 'nobody@example.com', NULL, 'sent', '2026-03-15 14:30:05')",
    )
    .bind(recipient_id.as_bytes().as_slice())
    .bind(version_id.as_bytes().as_slice())
    .bind(job_id.as_bytes().as_slice())
    .execute(&*pool)
    .await
    .unwrap();

    // Download the backup ZIP
    let response = client
        .get(server.url("/api/backup/documents"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let zip_bytes = response.bytes().await.unwrap();
    let reader = std::io::Cursor::new(zip_bytes.as_ref());
    let mut archive = zip::ZipArchive::new(reader).unwrap();

    // Ensure no communication files are in the ZIP
    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        assert!(
            !file.name().contains("kommunikation/"),
            "Unassigned mail should not appear in backup"
        );
    }
}

// --- Application (public join) E2E Tests ---

async fn setup_api_key(server: &genossi_rest::test_server::test_support::TestServer, client: &reqwest::Client) -> String {
    let api_key = "test-api-key-12345";
    let response = client
        .put(server.url("/api/config/public_api_key"))
        .json(&SetConfigRequest {
            value: api_key.to_string(),
            value_type: "secret".to_string(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    api_key.to_string()
}

fn sample_join_request() -> PublicJoinRequest {
    PublicJoinRequest {
        first_name: "Max".to_string(),
        last_name: "Mustermann".to_string(),
        salutation: Some(SalutationTO::Herr),
        email: "max@example.com".to_string(),
        street: "Musterstraße".to_string(),
        house_number: "42".to_string(),
        postal_code: "12345".to_string(),
        city: "Berlin".to_string(),
        shares: 2,
    }
}

#[tokio::test]
async fn test_public_join_success() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: PublicJoinResponse = response.json().await.unwrap();
    assert_eq!(body.message, "Beitrittserklärung eingegangen");
}

#[tokio::test]
async fn test_public_join_missing_api_key() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/public/join"))
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_public_join_invalid_api_key() {
    let server = setup().await;
    let client = reqwest::Client::new();
    setup_api_key(&server, &client).await;

    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", "wrong-key")
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_public_join_missing_fields() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    let mut request = sample_join_request();
    request.email = "".to_string();
    request.first_name = "".to_string();

    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_public_join_shares_zero() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    let mut request = sample_join_request();
    request.shares = 0;

    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_applications() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit an application
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    // List all applications
    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].first_name, "Max");
    assert_eq!(apps[0].last_name, "Mustermann");
    assert_eq!(apps[0].shares, 2);
    assert!(matches!(apps[0].status, ApplicationStatusTO::Offen));
}

#[tokio::test]
async fn test_list_applications_filter_status() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit an application
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    // Filter by Offen
    let response = client
        .get(server.url("/api/applications?status=Offen"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 1);

    // Filter by Bestaetigt (should be empty)
    let response = client
        .get(server.url("/api/applications?status=Bestaetigt"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert!(apps.is_empty());
}

#[tokio::test]
async fn test_get_application() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    // List to get ID
    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    let id = apps[0].id;

    // Get by ID
    let response = client
        .get(server.url(&format!("/api/applications/{}", id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let app: ApplicationTO = response.json().await.unwrap();
    assert_eq!(app.first_name, "Max");
    assert_eq!(app.email, Some("max@example.com".to_string()));
}

#[tokio::test]
async fn test_confirm_application_creates_member() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    // Get application ID
    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    let app_id = apps[0].id;

    // Confirm
    let response = client
        .post(server.url(&format!("/api/applications/{}/confirm", app_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let confirmed: ApplicationTO = response.json().await.unwrap();
    assert!(matches!(confirmed.status, ApplicationStatusTO::Bestaetigt));

    // Verify member was created
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].first_name, "Max");
    assert_eq!(members[0].last_name, "Mustermann");
    assert_eq!(members[0].shares_at_joining, 2);
    assert!(members[0].email.as_deref() == Some("max@example.com"));
}

#[tokio::test]
async fn test_confirm_already_confirmed() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit and confirm
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    let app_id = apps[0].id;

    // First confirm
    client
        .post(server.url(&format!("/api/applications/{}/confirm", app_id)))
        .send()
        .await
        .unwrap();

    // Second confirm → 409
    let response = client
        .post(server.url(&format!("/api/applications/{}/confirm", app_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_reject_application() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    let app_id = apps[0].id;

    // Reject
    let response = client
        .post(server.url(&format!("/api/applications/{}/reject", app_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let rejected: ApplicationTO = response.json().await.unwrap();
    assert!(matches!(rejected.status, ApplicationStatusTO::Abgelehnt));

    // Verify no member was created
    let response = client
        .get(server.url("/api/members"))
        .send()
        .await
        .unwrap();
    let members: Vec<MemberTO> = response.json().await.unwrap();
    assert!(members.is_empty());
}

#[tokio::test]
async fn test_reject_already_confirmed() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit and confirm
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    let app_id = apps[0].id;

    // Confirm
    client
        .post(server.url(&format!("/api/applications/{}/confirm", app_id)))
        .send()
        .await
        .unwrap();

    // Reject confirmed → 409
    let response = client
        .post(server.url(&format!("/api/applications/{}/reject", app_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_confirm_rejected() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit and reject
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    let app_id = apps[0].id;

    // Reject
    client
        .post(server.url(&format!("/api/applications/{}/reject", app_id)))
        .send()
        .await
        .unwrap();

    // Confirm rejected → 409
    let response = client
        .post(server.url(&format!("/api/applications/{}/confirm", app_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_generate_api_key() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/config/generate-api-key"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: genossi_config::rest::GenerateApiKeyResponse = response.json().await.unwrap();
    assert!(!body.key.is_empty());

    // Use generated key to submit application
    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &body.key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_generate_api_key_regenerates() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Generate first key
    let response = client
        .post(server.url("/api/config/generate-api-key"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let first: genossi_config::rest::GenerateApiKeyResponse = response.json().await.unwrap();

    // Generate second key (should overwrite)
    let response = client
        .post(server.url("/api/config/generate-api-key"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let second: genossi_config::rest::GenerateApiKeyResponse = response.json().await.unwrap();

    // Keys should be different
    assert_ne!(first.key, second.key);

    // Old key should no longer work
    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &first.key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // New key should work
    let response = client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &second.key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_wordpress_config_entries_save_and_load() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Save WordPress-relevant config entries
    let entries = vec![
        ("share_value_cents", "5000", "int"),
        ("bank_iban", "DE89 3704 0044 0532 0130 00", "string"),
        ("bank_name", "GLS Bank", "string"),
        ("bank_bic", "GENODEM1GLS", "string"),
        ("genossenschaft_name", "Muster eG", "string"),
    ];

    for (key, value, vtype) in &entries {
        let response = client
            .put(server.url(&format!("/api/config/{}", key)))
            .json(&SetConfigRequest {
                value: value.to_string(),
                value_type: vtype.to_string(),
            })
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "Failed to set {}", key);
    }

    // Load all config entries and verify
    let response = client
        .get(server.url("/api/config"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let config_entries: Vec<serde_json::Value> = response.json().await.unwrap();

    for (key, expected_value, expected_type) in &entries {
        let entry = config_entries
            .iter()
            .find(|e| e["key"].as_str() == Some(key))
            .unwrap_or_else(|| panic!("Config entry '{}' not found", key));
        // Secret values are masked, others should match
        if *expected_type != "secret" {
            assert_eq!(entry["value"].as_str(), Some(*expected_value), "Value mismatch for {}", key);
        }
        assert_eq!(entry["value_type"].as_str(), Some(*expected_type), "Type mismatch for {}", key);
    }
}

#[tokio::test]
async fn test_reject_application_changes_status() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit an application
    client
        .post(server.url("/api/public/join"))
        .header("X-Api-Key", &api_key)
        .json(&sample_join_request())
        .send()
        .await
        .unwrap();

    // Get the application
    let response = client
        .get(server.url("/api/applications"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 1);
    assert!(matches!(apps[0].status, ApplicationStatusTO::Offen));

    // Reject it
    let response = client
        .post(server.url(&format!("/api/applications/{}/reject", apps[0].id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let rejected: ApplicationTO = response.json().await.unwrap();
    assert!(matches!(rejected.status, ApplicationStatusTO::Abgelehnt));

    // Verify filtered list
    let response = client
        .get(server.url("/api/applications?status=Abgelehnt"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 1);

    let response = client
        .get(server.url("/api/applications?status=Offen"))
        .send()
        .await
        .unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 0);
}

#[tokio::test]
async fn test_application_full_workflow() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let api_key = setup_api_key(&server, &client).await;

    // Submit two applications
    let mut req1 = sample_join_request();
    req1.first_name = "Anna".to_string();
    req1.last_name = "Schmidt".to_string();
    req1.email = "anna@example.com".to_string();

    let mut req2 = sample_join_request();
    req2.first_name = "Bob".to_string();
    req2.last_name = "Mueller".to_string();
    req2.email = "bob@example.com".to_string();

    for req in [&req1, &req2] {
        client
            .post(server.url("/api/public/join"))
            .header("X-Api-Key", &api_key)
            .json(req)
            .send()
            .await
            .unwrap();
    }

    // List all - should have 2
    let response = client.get(server.url("/api/applications")).send().await.unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 2);

    // Confirm first, reject second
    let anna_id = apps.iter().find(|a| a.first_name == "Anna").unwrap().id;
    let bob_id = apps.iter().find(|a| a.first_name == "Bob").unwrap().id;

    let response = client
        .post(server.url(&format!("/api/applications/{}/confirm", anna_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post(server.url(&format!("/api/applications/{}/reject", bob_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify: 0 open, 1 confirmed, 1 rejected
    let response = client.get(server.url("/api/applications?status=Offen")).send().await.unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 0);

    let response = client.get(server.url("/api/applications?status=Bestaetigt")).send().await.unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].first_name, "Anna");

    let response = client.get(server.url("/api/applications?status=Abgelehnt")).send().await.unwrap();
    let apps: Vec<ApplicationTO> = response.json().await.unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].first_name, "Bob");
}

// --- Admin create application tests ---

#[tokio::test]
async fn test_admin_create_application_minimal_fields() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let request = AdminCreateApplicationRequest {
        first_name: "Hans".to_string(),
        last_name: "Meier".to_string(),
        salutation: None,
        email: None,
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        shares: 1,
        send_mail: None,
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let app: ApplicationTO = response.json().await.unwrap();
    assert_eq!(app.first_name, "Hans");
    assert_eq!(app.last_name, "Meier");
    assert_eq!(app.shares, 1);
    assert_eq!(app.status, ApplicationStatusTO::Offen);
    assert!(app.email.is_none());
    assert!(app.street.is_none());
}

#[tokio::test]
async fn test_admin_create_application_all_fields() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let request = AdminCreateApplicationRequest {
        first_name: "Anna".to_string(),
        last_name: "Schmidt".to_string(),
        salutation: Some(SalutationTO::Frau),
        email: Some("anna@example.com".to_string()),
        street: Some("Hauptstraße".to_string()),
        house_number: Some("1".to_string()),
        postal_code: Some("10115".to_string()),
        city: Some("Berlin".to_string()),
        shares: 3,
        send_mail: Some(false),
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let app: ApplicationTO = response.json().await.unwrap();
    assert_eq!(app.first_name, "Anna");
    assert_eq!(app.email, Some("anna@example.com".to_string()));
    assert_eq!(app.street, Some("Hauptstraße".to_string()));
    assert_eq!(app.shares, 3);
    assert_eq!(app.status, ApplicationStatusTO::Offen);
}

#[tokio::test]
async fn test_admin_create_application_send_mail_without_email() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let request = AdminCreateApplicationRequest {
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        salutation: None,
        email: None,
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        shares: 1,
        send_mail: Some(true),
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_create_application_send_mail_with_email() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let request = AdminCreateApplicationRequest {
        first_name: "Eva".to_string(),
        last_name: "Müller".to_string(),
        salutation: None,
        email: Some("eva@example.com".to_string()),
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        shares: 1,
        send_mail: Some(true),
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();

    // Should succeed (mail will fail silently since no SMTP configured in tests)
    assert_eq!(response.status(), StatusCode::CREATED);
    let app: ApplicationTO = response.json().await.unwrap();
    assert_eq!(app.first_name, "Eva");
    assert_eq!(app.email, Some("eva@example.com".to_string()));
}

#[tokio::test]
async fn test_admin_create_application_missing_first_name() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let request = AdminCreateApplicationRequest {
        first_name: "".to_string(),
        last_name: "Meier".to_string(),
        salutation: None,
        email: None,
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        shares: 1,
        send_mail: None,
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_create_application_shares_zero() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let request = AdminCreateApplicationRequest {
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        salutation: None,
        email: None,
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        shares: 0,
        send_mail: None,
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// --- Application Template Render Tests ---

#[tokio::test]
async fn test_render_application_template_pdf() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create an application
    let request = AdminCreateApplicationRequest {
        first_name: "Erika".to_string(),
        last_name: "Musterfrau".to_string(),
        salutation: Some(SalutationTO::Frau),
        email: Some("erika@example.com".to_string()),
        street: Some("Testweg".to_string()),
        house_number: Some("7".to_string()),
        postal_code: Some("54321".to_string()),
        city: Some("Teststadt".to_string()),
        shares: 3,
        send_mail: None,
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let app: ApplicationTO = response.json().await.unwrap();
    let app_id = app.id;

    // Create a template that uses application data
    let template = r#"
#set page(paper: "a4")
#set text(size: 12pt)
#let app = json.decode(sys.inputs.at("application"))
#let today = sys.inputs.at("today")
Zahlungsaufforderung an #app.first_name #app.last_name
Anteile: #app.shares
Status: #app.status
"#;

    let response = client
        .put(server.url("/api/templates/zahlungsaufforderung.typ"))
        .header("Content-Type", "text/plain")
        .body(template)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Render the template for the application
    let response = client
        .post(server.url(&format!(
            "/api/templates/render-application/zahlungsaufforderung.typ/{}",
            app_id
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
async fn test_render_application_template_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create an application
    let request = AdminCreateApplicationRequest {
        first_name: "Max".to_string(),
        last_name: "Test".to_string(),
        salutation: None,
        email: None,
        street: None,
        house_number: None,
        postal_code: None,
        city: None,
        shares: 1,
        send_mail: None,
    };

    let response = client
        .post(server.url("/api/applications"))
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let app: ApplicationTO = response.json().await.unwrap();

    // Try to render a non-existent template
    let response = client
        .post(server.url(&format!(
            "/api/templates/render-application/nonexistent.typ/{}",
            app.id
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_render_application_template_application_not_found() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Create a template
    let template = r#"
#set page(paper: "a4")
#let app = json.decode(sys.inputs.at("application"))
Test
"#;
    client
        .put(server.url("/api/templates/test.typ"))
        .header("Content-Type", "text/plain")
        .body(template)
        .send()
        .await
        .unwrap();

    // Try to render with a non-existent application ID
    let fake_id = uuid::Uuid::new_v4();
    let response = client
        .post(server.url(&format!(
            "/api/templates/render-application/test.typ/{}",
            fake_id
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
