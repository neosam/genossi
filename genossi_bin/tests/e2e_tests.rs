use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::start_test_server;
use genossi_rest_types::{MemberImportResultTO, MemberTO};
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
    assert_eq!(created.current_shares, 3);
    assert_eq!(created.current_balance, 15000);

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
                "01.01.2020", "3", "5", "15000", "", "hans@test.de", "", "", "DE123",
            ],
            vec![
                "2", "Schmidt", "Anna", "Nebenstr.", "10", "80331", "München",
                "15.06.2021", "2", "2", "10000", "", "anna@test.de", "Firma GmbH", "", "",
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
            "01.01.2020", "3", "3", "10000", "", "", "", "", "",
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
            "01.01.2020", "3", "5", "20000", "", "new@email.de", "", "", "",
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
    assert_eq!(members[0].current_balance, 20000);
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
                "01.01.2020", "3", "3", "10000", "", "", "", "", "",
            ],
            // Invalid row - bad date
            vec![
                "2", "Schmidt", "Anna", "", "", "", "",
                "not-a-date", "2", "2", "5000", "", "", "", "", "",
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
