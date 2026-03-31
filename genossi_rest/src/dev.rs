use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use genossi_service::member::MemberService;
use genossi_service::permission::Authentication;
use std::sync::Arc;
use time::Month;
use crate::RestStateDef;

pub fn api_doc() -> utoipa::openapi::OpenApi {
    utoipa::openapi::OpenApiBuilder::new()
        .paths(
            utoipa::openapi::PathsBuilder::new().path(
                "/api/dev/generate-test-data",
                utoipa::openapi::PathItem::new(
                    utoipa::openapi::HttpMethod::Post,
                    utoipa::openapi::path::OperationBuilder::new()
                        .tag("Dev")
                        .summary(Some("Generate test member data (debug builds only)"))
                        .response(
                            "201",
                            utoipa::openapi::ResponseBuilder::new()
                                .description("Test data generated"),
                        )
                        .response(
                            "200",
                            utoipa::openapi::ResponseBuilder::new()
                                .description("Test data already exists"),
                        )
                        .response(
                            "500",
                            utoipa::openapi::ResponseBuilder::new()
                                .description("Internal server error"),
                        ),
                ),
            ),
        )
        .build()
}

struct TestMember {
    member_number: i64,
    first_name: &'static str,
    last_name: &'static str,
    email: Option<&'static str>,
    company: Option<&'static str>,
    comment: Option<&'static str>,
    street: Option<&'static str>,
    house_number: Option<&'static str>,
    postal_code: Option<&'static str>,
    city: Option<&'static str>,
    join_year: i32,
    join_month: Month,
    join_day: u8,
    shares_at_joining: i32,
    current_shares: i32,
    current_balance: i64,
    exit_year: Option<i32>,
    exit_month: Option<Month>,
    exit_day: Option<u8>,
    bank_account: Option<&'static str>,
}

fn test_members() -> Vec<TestMember> {
    vec![
        TestMember {
            member_number: 1001,
            first_name: "Anna",
            last_name: "Mueller",
            email: Some("anna.mueller@example.com"),
            company: Some("Mueller GmbH"),
            comment: Some("Gruendungsmitglied"),
            street: Some("Hauptstrasse"),
            house_number: Some("12"),
            postal_code: Some("80331"),
            city: Some("Muenchen"),
            join_year: 2020,
            join_month: Month::March,
            join_day: 15,
            shares_at_joining: 5,
            current_shares: 10,
            current_balance: 50000,
            exit_year: None,
            exit_month: None,
            exit_day: None,
            bank_account: Some("DE89370400440532013000"),
        },
        TestMember {
            member_number: 1002,
            first_name: "Thomas",
            last_name: "Schmidt",
            email: Some("thomas.schmidt@example.com"),
            company: None,
            comment: None,
            street: Some("Berliner Allee"),
            house_number: Some("7a"),
            postal_code: Some("10115"),
            city: Some("Berlin"),
            join_year: 2021,
            join_month: Month::June,
            join_day: 1,
            shares_at_joining: 3,
            current_shares: 3,
            current_balance: 15000,
            exit_year: None,
            exit_month: None,
            exit_day: None,
            bank_account: Some("DE27100777770209299700"),
        },
        TestMember {
            member_number: 1003,
            first_name: "Maria",
            last_name: "Fischer",
            email: None,
            company: None,
            comment: None,
            street: Some("Gartenweg"),
            house_number: Some("3"),
            postal_code: Some("50667"),
            city: Some("Koeln"),
            join_year: 2022,
            join_month: Month::January,
            join_day: 10,
            shares_at_joining: 1,
            current_shares: 2,
            current_balance: 10000,
            exit_year: None,
            exit_month: None,
            exit_day: None,
            bank_account: None,
        },
        TestMember {
            member_number: 1004,
            first_name: "Klaus",
            last_name: "Weber",
            email: Some("klaus.weber@example.com"),
            company: Some("Weber & Soehne"),
            comment: Some("Ausgetreten wegen Umzug"),
            street: Some("Lindenstrasse"),
            house_number: Some("45"),
            postal_code: Some("70173"),
            city: Some("Stuttgart"),
            join_year: 2020,
            join_month: Month::September,
            join_day: 20,
            shares_at_joining: 2,
            current_shares: 2,
            current_balance: 10000,
            exit_year: Some(2025),
            exit_month: Some(Month::December),
            exit_day: Some(31),
            bank_account: Some("DE75512108001245126199"),
        },
        TestMember {
            member_number: 1005,
            first_name: "Sabine",
            last_name: "Braun",
            email: Some("sabine.braun@example.com"),
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_year: 2023,
            join_month: Month::April,
            join_day: 5,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 5000,
            exit_year: None,
            exit_month: None,
            exit_day: None,
            bank_account: None,
        },
        TestMember {
            member_number: 1006,
            first_name: "Peter",
            last_name: "Hoffmann",
            email: Some("peter.hoffmann@example.com"),
            company: Some("Hoffmann IT Solutions"),
            comment: None,
            street: Some("Am Marktplatz"),
            house_number: Some("1"),
            postal_code: Some("20095"),
            city: Some("Hamburg"),
            join_year: 2021,
            join_month: Month::November,
            join_day: 12,
            shares_at_joining: 4,
            current_shares: 8,
            current_balance: 40000,
            exit_year: None,
            exit_month: None,
            exit_day: None,
            bank_account: Some("DE02120300000000202051"),
        },
        TestMember {
            member_number: 1007,
            first_name: "Lisa",
            last_name: "Wagner",
            email: None,
            company: None,
            comment: Some("Zahlt per Ueberweisung"),
            street: Some("Schulstrasse"),
            house_number: Some("22b"),
            postal_code: Some("60311"),
            city: Some("Frankfurt"),
            join_year: 2024,
            join_month: Month::February,
            join_day: 28,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 5000,
            exit_year: None,
            exit_month: None,
            exit_day: None,
            bank_account: None,
        },
    ]
}

fn build_member(t: &TestMember) -> genossi_service::member::Member {
    let join_date =
        time::Date::from_calendar_date(t.join_year, t.join_month, t.join_day).unwrap();
    let exit_date = t.exit_year.map(|y| {
        time::Date::from_calendar_date(y, t.exit_month.unwrap(), t.exit_day.unwrap()).unwrap()
    });
    let now = time::OffsetDateTime::now_utc();
    let created = time::PrimitiveDateTime::new(now.date(), now.time());

    genossi_service::member::Member {
        id: uuid::Uuid::new_v4(),
        member_number: t.member_number,
        first_name: Arc::from(t.first_name),
        last_name: Arc::from(t.last_name),
        email: t.email.map(Arc::from),
        company: t.company.map(Arc::from),
        comment: t.comment.map(Arc::from),
        street: t.street.map(Arc::from),
        house_number: t.house_number.map(Arc::from),
        postal_code: t.postal_code.map(Arc::from),
        city: t.city.map(Arc::from),
        join_date,
        shares_at_joining: t.shares_at_joining,
        current_shares: t.current_shares,
        current_balance: t.current_balance,
        action_count: 0,
        migrated: false,
        exit_date,
        bank_account: t.bank_account.map(Arc::from),
        created,
        deleted: None,
        version: uuid::Uuid::new_v4(),
    }
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new().route("/generate-test-data", post(generate_test_data::<RestState>))
}

#[utoipa::path(
    post,
    tag = "Dev",
    path = "/generate-test-data",
    responses(
        (status = 201, description = "Test data generated"),
        (status = 200, description = "Test data already exists"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn generate_test_data<RestState: RestStateDef>(
    rest_state: State<RestState>,
) -> impl IntoResponse {
    let member_service = rest_state.member_service();
    let existing = member_service
        .get_all(Authentication::Full, None)
        .await;

    match existing {
        Ok(members) if !members.is_empty() => {
            (StatusCode::OK, Json(serde_json::json!({"message": "Test data already exists", "count": members.len()}))).into_response()
        }
        Ok(_) => {
            let test_data = test_members();
            let mut created_count = 0;
            for t in &test_data {
                let member = build_member(t);
                if let Err(e) = member_service.create(&member, Authentication::Full, None).await {
                    tracing::error!("Failed to create test member {}: {:?}", t.first_name, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": format!("Failed to create member: {:?}", e)})),
                    ).into_response();
                }
                created_count += 1;
            }
            (StatusCode::CREATED, Json(serde_json::json!({"message": "Test data generated", "count": created_count}))).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to check existing members: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to check members: {:?}", e)})),
            ).into_response()
        }
    }
}
