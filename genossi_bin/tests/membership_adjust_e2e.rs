#![cfg(feature = "mock_auth")]
//! Phase 15 Plan 04 — E2E-Tests fuer `POST /api/members/{id}/cancel` und
//! `POST /api/members/{id}/increase-shares`.
//!
//! Verifiziert die volle Stack-Konsistenz inkl.:
//! - REST-Handler -> MembershipAdjustService -> audited_*! Macros -> DAOs
//! - `recalc_dates`-Hook setzt `Member.exit_date` end-to-end (CANC-04)
//! - Audit-Hashchain `/api/audit/verify` bleibt `valid=true` (AUDT-01)
//! - 409-Conflict bei doppelter Kuendigung (Already-Cancelled-Pfad)
//! - 400-Validation bei Datum-Bounds (Vorjahr / Uebernaechstes Jahr)
//! - 400-ValidationError bei increase_shares auf gekuendigtes Member (UPGD-04)
//!
//! BLOCKER 5 (D-15-12 Resolution): Permission-Denied-Pfade sind im
//! `mock_auth`-Stack NICHT direkt E2E-testbar — die Migration
//! `20250129000001_create_default_auth_data.sql` weist DEVUSER (den vom
//! `MockUserService` zurueckgegebenen User) das `admin`-Privileg fest zu.
//! Daher sind `test_cancel_membership_permission_denied` und
//! `test_increase_shares_permission_denied` mit `#[ignore]` markiert; der
//! 401-Pfad ist auf Service-Layer-Ebene unit-getestet in
//! `genossi_service_impl/src/membership_adjust.rs::service_tests::{
//! test_cancel_membership_permission_denied, test_increase_shares_permission_denied
//! }` (assertieren `ServiceError::PermissionDenied`, was per
//! `genossi_rest/src/lib.rs:115` globalem Mapping zu HTTP 401 wird).
//!
//! Test-Naming-Konvention folgt §specifics aus 15-CONTEXT.md.

use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::{start_test_server, TestServer};
use genossi_rest_types::{MemberStatusTO, MemberTO};
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;

// ============================================================================
// Test-Setup (1:1 aus transfer_recipients_e2e.rs uebernommen)
// ============================================================================

async fn setup() -> TestServer {
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

// ============================================================================
// Member-Helper
// ============================================================================

fn sample_member(member_number: i64, first_name: &str) -> MemberTO {
    MemberTO {
        id: None,
        member_number,
        first_name: first_name.to_string(),
        last_name: "TestUser".to_string(),
        salutation: None,
        title: None,
        email: Some(format!("user{}@example.com", member_number)),
        company: None,
        comment: None,
        street: Some("Musterstraße".to_string()),
        house_number: Some("1".to_string()),
        postal_code: Some("12345".to_string()),
        city: Some("Berlin".to_string()),
        join_date: time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap(),
        shares_at_joining: 1,
        current_shares: 1,
        current_balance: 0,
        action_count: 0,
        migrated: false,
        exit_date: None,
        bank_account: Some("DE89370400440532013000".to_string()),
        status: MemberStatusTO::Normal,
        created: None,
        deleted: None,
        version: None,
    }
}

async fn create_active_member(
    client: &reqwest::Client,
    server: &TestServer,
    member_number: i64,
    first_name: &str,
) -> MemberTO {
    let m = sample_member(member_number, first_name);
    let response = client
        .post(server.url("/api/members"))
        .json(&m)
        .send()
        .await
        .expect("create_member POST failed");
    assert!(
        response.status().is_success(),
        "create_member expected 2xx, got {}: {}",
        response.status(),
        response.text().await.unwrap_or_default()
    );
    response.json().await.expect("decode MemberTO")
}

// ============================================================================
// Body-Helpers fuer Phase-15-Endpoints
// ============================================================================

fn cancel_body(willensbekundung: &str) -> Value {
    serde_json::json!({ "willensbekundung_date": willensbekundung })
}

fn increase_body(willensbekundung: &str, shares: i32) -> Value {
    serde_json::json!({
        "willensbekundung_date": willensbekundung,
        "shares": shares
    })
}

/// Date-Fragility-Fix: leite Test-Daten relativ zu `OffsetDateTime::now_utc().date()`
/// ab, damit Tests nicht beim Jahres-Rollover brechen.
fn today_march_15() -> time::Date {
    let today = time::OffsetDateTime::now_utc().date();
    time::Date::from_calendar_date(today.year(), time::Month::March, 15)
        .expect("YYYY-03-15 always valid")
}

fn today_august_15() -> time::Date {
    let today = time::OffsetDateTime::now_utc().date();
    time::Date::from_calendar_date(today.year(), time::Month::August, 15)
        .expect("YYYY-08-15 always valid")
}

fn current_year_dec_31() -> time::Date {
    let today = time::OffsetDateTime::now_utc().date();
    time::Date::from_calendar_date(today.year(), time::Month::December, 31)
        .expect("YYYY-12-31 always valid")
}

fn next_year_dec_31() -> time::Date {
    let today = time::OffsetDateTime::now_utc().date();
    time::Date::from_calendar_date(today.year() + 1, time::Month::December, 31)
        .expect("(YYYY+1)-12-31 always valid")
}

// ============================================================================
// Kuendigung — 5 Tests (Happy-Path H1, Happy-Path H2, Permission-Denied,
// Already-Cancelled, Audit-Chain-Verify)
// ============================================================================

#[tokio::test]
async fn test_cancel_membership_happy_path_h1() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1001, "Kuendigung").await;
    let member_id = m.id.expect("created member must have id");

    let h1_date = today_march_15();
    let expected_effective = current_year_dec_31();

    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("POST cancel failed");

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(status, StatusCode::OK, "expected 200, body: {}", body_text);

    let response: Value = serde_json::from_str(&body_text).expect("decode response");
    let action = &response["action"];
    let member = &response["member"];

    assert_eq!(action["action_type"], "Austritt");
    assert_eq!(action["shares_change"], 0);
    // H1 (Maerz): effective_date = 31.12. desselben Jahres
    assert_eq!(action["effective_date"], expected_effective.to_string());
    // CANC-04 end-to-end-Beweis: recalc_dates hat exit_date gesetzt.
    assert_eq!(member["exit_date"], expected_effective.to_string());
}

#[tokio::test]
async fn test_cancel_membership_happy_path_h2() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1002, "H2").await;
    let member_id = m.id.expect("id");

    let h2_date = today_august_15();
    let expected_effective = next_year_dec_31();

    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h2_date.to_string()))
        .send()
        .await
        .expect("POST cancel failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let response: Value = resp.json().await.expect("decode");
    // H2 (August): effective_date = 31.12. NAECHSTES Jahr.
    assert_eq!(
        response["action"]["effective_date"],
        expected_effective.to_string()
    );
    assert_eq!(response["member"]["exit_date"], expected_effective.to_string());
}

/// Permission-Denied (assertet 401 per globalem Mapping in
/// genossi_rest/src/lib.rs:115). Im mock_auth-Stack ist DEVUSER per Migration
/// admin -> nicht direkt E2E reproduzierbar. Service-Layer-Unit-Test
/// `test_cancel_membership_permission_denied` deckt den
/// `ServiceError::PermissionDenied -> 401`-Pfad ab.
#[ignore = "mock_auth context_extractor injiziert IMMER Admin (DEVUSER) — non-admin nicht E2E darstellbar; 401-Pfad ist auf Service-Layer-Ebene unit-getestet (genossi_service_impl/src/membership_adjust.rs::service_tests::test_cancel_membership_permission_denied)"]
#[tokio::test]
async fn test_cancel_membership_permission_denied() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 9001, "PermDeny").await;
    let member_id = m.id.expect("id");
    let h1_date = today_march_15();

    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("POST cancel");

    // BLOCKER 5: assert StatusCode::UNAUTHORIZED (401, NICHT 403).
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_cancel_membership_already_cancelled() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1003, "Doppel").await;
    let member_id = m.id.expect("id");
    let h1_date = today_march_15();

    // 1. cancel -> 200
    let resp1 = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("first cancel");
    assert_eq!(resp1.status(), StatusCode::OK);

    // 2. cancel -> 409 (D-15-12 Conflict-Mapping)
    let resp2 = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("second cancel");
    assert_eq!(resp2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_cancel_membership_audit_chain_verify() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1004, "AuditTest").await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();

    // cancel -> 200, audit-Entries via audited_create! mit process="member-adjust.cancel"
    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("cancel failed");
    assert_eq!(resp.status(), StatusCode::OK);

    // AUDT-01: Audit-Hashchain global valid.
    let verify_resp = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("verify");
    assert_eq!(verify_resp.status(), StatusCode::OK);
    let verify_body: Value = verify_resp.json().await.expect("verify json");
    assert_eq!(
        verify_body["valid"], true,
        "audit-chain corrupted after cancel: {:?}",
        verify_body
    );
    // Mindestens 1 broken_links-Eintrag waere ein Bruch -> wir erwarten 0.
    assert_eq!(
        verify_body["broken_links"].as_array().map(|a| a.len()),
        Some(0),
        "expected no broken links"
    );
}

// ============================================================================
// Aufstockung — 4 Tests (Happy-Path, Cancelled-Member-Block, Permission-Denied,
// Audit-Chain-Verify)
// ============================================================================

#[tokio::test]
async fn test_increase_shares_happy_path() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1005, "Upgrade").await;
    let member_id = m.id.expect("id");

    let date_str = today_march_15().to_string();

    let resp = client
        .post(server.url(&format!("/api/members/{}/increase-shares", member_id)))
        .json(&increase_body(&date_str, 3))
        .send()
        .await
        .expect("upgrade");

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(status, StatusCode::OK, "expected 200, body: {}", body_text);

    let response: Value = serde_json::from_str(&body_text).expect("decode");
    assert_eq!(response["action"]["action_type"], "Aufstockung");
    assert_eq!(response["action"]["shares_change"], 3);
    // UPGD-02: effective_date None (sofort wirksam, kein H1/H2)
    // MemberActionTO serde verwendet skip_serializing_if = "Option::is_none"
    // -> effective_date Feld ist NICHT im JSON-Output.
    assert!(
        response["action"].get("effective_date").is_none(),
        "expected effective_date to be absent (skip_serializing_if=is_none), got: {:?}",
        response["action"]
    );
    // sample_member current_shares = 1, +3 = 4
    assert_eq!(response["member"]["current_shares"], 4);
}

#[tokio::test]
async fn test_increase_shares_cancelled_member_blocked() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1006, "BlockedUp").await;
    let member_id = m.id.expect("id");
    let h1_date = today_march_15();

    // 1. cancel -> 200
    let r1 = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("cancel");
    assert_eq!(r1.status(), StatusCode::OK);

    // 2. increase_shares -> 400 (UPGD-04: ValidationError, NICHT 409)
    let r2 = client
        .post(server.url(&format!("/api/members/{}/increase-shares", member_id)))
        .json(&increase_body(&h1_date.to_string(), 3))
        .send()
        .await
        .expect("upgrade on cancelled");
    assert_eq!(r2.status(), StatusCode::BAD_REQUEST);

    // Body sollte die "cancelled"-Begruendung enthalten.
    let body_text = r2.text().await.unwrap_or_default();
    assert!(
        body_text.contains("cancelled"),
        "expected body containing 'cancelled', got: {}",
        body_text
    );
}

#[ignore = "mock_auth context_extractor injiziert IMMER Admin (DEVUSER) — non-admin nicht E2E darstellbar; 401-Pfad ist auf Service-Layer-Ebene unit-getestet (genossi_service_impl/src/membership_adjust.rs::service_tests::test_increase_shares_permission_denied)"]
#[tokio::test]
async fn test_increase_shares_permission_denied() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 9002, "PermDenyUp").await;
    let member_id = m.id.expect("id");
    let h1_date = today_march_15();

    let resp = client
        .post(server.url(&format!("/api/members/{}/increase-shares", member_id)))
        .json(&increase_body(&h1_date.to_string(), 2))
        .send()
        .await
        .expect("upgrade");

    // BLOCKER 5: assert StatusCode::UNAUTHORIZED (401, NICHT 403).
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_increase_shares_audit_chain_verify() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1007, "AuditUp").await;
    let member_id = m.id.expect("id");

    let date_str = today_march_15().to_string();

    // increase_shares -> 200, zwei audited_*! Calls in einer Tx
    // (audited_create! fuer Action + audited_update! fuer Member.current_shares)
    // mit identischem process="member-adjust.upgrade".
    let resp = client
        .post(server.url(&format!("/api/members/{}/increase-shares", member_id)))
        .json(&increase_body(&date_str, 2))
        .send()
        .await
        .expect("upgrade");
    assert_eq!(resp.status(), StatusCode::OK);

    // AUDT-01: Audit-Hashchain global valid nach Multi-Write-Tx.
    let verify_resp = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("verify");
    assert_eq!(verify_resp.status(), StatusCode::OK);
    let verify_body: Value = verify_resp.json().await.expect("verify json");
    assert_eq!(
        verify_body["valid"], true,
        "audit-chain corrupted after upgrade: {:?}",
        verify_body
    );
}

// ============================================================================
// Datum-Bounds-Edge-Cases (PERM-02 / D-15-06)
// ============================================================================

#[tokio::test]
async fn test_cancel_membership_date_in_previous_year_rejected() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1008, "VorjahrTest").await;
    let member_id = m.id.expect("id");

    // Zwei Jahre in der Vergangenheit -> ausserhalb [current_fy, current_fy+1]
    let today = time::OffsetDateTime::now_utc().date();
    let two_years_ago =
        time::Date::from_calendar_date(today.year() - 2, time::Month::June, 15).unwrap();

    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&two_years_ago.to_string()))
        .send()
        .await
        .expect("cancel");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cancel_membership_date_in_overnext_year_rejected() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1009, "UebernaechstTest").await;
    let member_id = m.id.expect("id");

    // Uebernaechstes Jahr (today.year() + 2) -> ausserhalb [current_fy, current_fy+1]
    let today = time::OffsetDateTime::now_utc().date();
    let overnext_year =
        time::Date::from_calendar_date(today.year() + 2, time::Month::January, 15).unwrap();

    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&overnext_year.to_string()))
        .send()
        .await
        .expect("cancel");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
