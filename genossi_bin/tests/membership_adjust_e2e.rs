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
use uuid::Uuid;

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
        account_holder: None,
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

// ============================================================================
// Quick 260608-jb1: cancel_membership creates RepaymentEntry (bug-fix)
// ============================================================================
//
// Symmetrischer Fix analog `partial_repayment` Step 9+12: gekuendigte Members
// landen direkt in der offenen RepaymentPhase ihres fiscal_year. Vorher passierte
// das nur via Auto-Fill in `open_repayment_phase` beim Preparation->Open-State-
// Transition; bei bereits-Open-Phasen fielen Members durch.
//
// 4 Tests:
// A) Phase ist Open -> Entry wird angelegt
// B) Keine Phase vorhanden -> Auto-Create (Open) + Entry
// C) Phase ist Closed -> 409 Conflict
// D) Entry existiert bereits (z.B. aus partial_repayment) -> kein Duplikat

/// Test A — Phase open, cancel -> RepaymentEntry angelegt.
#[tokio::test]
async fn test_cancel_membership_creates_repayment_entry_when_phase_open() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1300, "CancEntryOpen").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();

    // Phase fuer fiscal_year anlegen + auf Open transitionieren.
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().expect("phase.id");
    let r_open = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase_id)))
        .send()
        .await
        .expect("open phase");
    assert_eq!(
        r_open.status(),
        StatusCode::OK,
        "open phase: {}",
        r_open.text().await.unwrap_or_default()
    );

    // Cancel.
    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("POST cancel");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200, got {}: {}",
        status,
        body_text
    );

    // GET /api/repayment-entry?phase_id=... liefert genau einen Entry fuer Member.
    let r_list = client
        .get(server.url(&format!("/api/repayment-entry?phase_id={}", phase_id)))
        .send()
        .await
        .expect("list entries");
    assert_eq!(r_list.status(), StatusCode::OK);
    let entries: Vec<Value> = r_list.json().await.expect("decode entries");
    let member_id_str = member_id.to_string();
    let entries_for_member: Vec<&Value> = entries
        .iter()
        .filter(|e| e["member_id"].as_str() == Some(&member_id_str))
        .collect();
    assert_eq!(
        entries_for_member.len(),
        1,
        "expected exactly 1 entry for cancelled member; got {} entries: {:?}",
        entries_for_member.len(),
        entries_for_member
    );
    assert_eq!(
        entries_for_member[0]["share_count_to_pay_out"], 3,
        "share_count_to_pay_out muss member.current_shares (3) sein"
    );
    assert_eq!(
        entries_for_member[0]["status"], "Open",
        "neuer Entry muss Status Open haben"
    );
}

/// Test B — Keine Phase vorhanden, cancel mit H2-Datum -> Phase + Entry werden
/// auto-angelegt; Phase ist Open mit DEFAULT_SHARE_VALUE_CENT (10000) und
/// fiscal_year = aktuelles Jahr + 1.
#[tokio::test]
async fn test_cancel_membership_auto_creates_phase_when_none_exists() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1301, "CancAutoPhase").await;
    let m = put_member_current_shares(&client, &server, &m, 2).await;
    let member_id = m.id.expect("id");

    // H2 -> fiscal_year = naechstes Jahr; keine Phase vor-erzeugt.
    let h2_date = today_august_15();
    let target_fy = h2_date.year() + 1;

    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h2_date.to_string()))
        .send()
        .await
        .expect("POST cancel");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200, got {}: {}",
        status,
        body_text
    );

    // GET /api/repayment-phase -> die auto-angelegte Phase muss existieren.
    let r_phases = client
        .get(server.url("/api/repayment-phase"))
        .send()
        .await
        .expect("list phases");
    assert_eq!(r_phases.status(), StatusCode::OK);
    let phases: Vec<Value> = r_phases.json().await.expect("decode phases");
    let auto_phase = phases
        .iter()
        .find(|p| p["fiscal_year"].as_i64() == Some(target_fy as i64))
        .unwrap_or_else(|| {
            panic!(
                "no phase for fiscal_year={} found; phases: {:?}",
                target_fy, phases
            )
        });
    assert_eq!(
        auto_phase["status"], "Open",
        "auto-created phase MUST be Open (analog partial_repayment Step 9)"
    );
    assert_eq!(
        auto_phase["share_value"], 10000,
        "DEFAULT_SHARE_VALUE_CENT fallback wenn keine Vorgaenger-Phase"
    );
    assert!(
        !auto_phase["opened_at"].is_null(),
        "auto-created phase muss opened_at gesetzt haben"
    );

    let phase_id = auto_phase["id"].as_str().expect("auto_phase.id");

    // GET /api/repayment-entry?phase_id=... liefert genau einen Entry fuer Member.
    let r_list = client
        .get(server.url(&format!("/api/repayment-entry?phase_id={}", phase_id)))
        .send()
        .await
        .expect("list entries");
    assert_eq!(r_list.status(), StatusCode::OK);
    let entries: Vec<Value> = r_list.json().await.expect("decode entries");
    let member_id_str = member_id.to_string();
    let entries_for_member: Vec<&Value> = entries
        .iter()
        .filter(|e| e["member_id"].as_str() == Some(&member_id_str))
        .collect();
    assert_eq!(
        entries_for_member.len(),
        1,
        "expected exactly 1 entry for cancelled member"
    );
    assert_eq!(
        entries_for_member[0]["share_count_to_pay_out"], 2,
        "share_count_to_pay_out = member.current_shares"
    );
}

/// Test C — Phase ist Closed -> cancel gibt 409 zurueck; Member.exit_date bleibt null;
/// keine neue Austritt-Action.
#[tokio::test]
async fn test_cancel_membership_closed_phase_returns_409() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1302, "CancClosed").await;
    // current_shares = 1 reicht; share-count ist fuer 409 unerheblich.
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();

    // Phase: Preparation -> Open -> Closed.
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().expect("phase.id");
    let r_open = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase_id)))
        .send()
        .await
        .expect("open phase");
    assert_eq!(r_open.status(), StatusCode::OK);
    let r_close = client
        .post(server.url(&format!("/api/repayment-phase/{}/close", phase_id)))
        .send()
        .await
        .expect("close phase");
    assert_eq!(
        r_close.status(),
        StatusCode::OK,
        "close phase: {}",
        r_close.text().await.unwrap_or_default()
    );

    // Cancel -> 409.
    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("POST cancel");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "expected 409 for closed phase, got {}: {}",
        status,
        body_text
    );
    assert!(
        body_text.contains("Closed") || body_text.contains("closed"),
        "expected body to mention closed; got: {}",
        body_text
    );

    // Tx muss rolled-back sein: Member.exit_date == null.
    let r_member = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .expect("GET member");
    assert_eq!(r_member.status(), StatusCode::OK);
    let member_json: Value = r_member.json().await.expect("decode member");
    assert!(
        member_json["exit_date"].is_null(),
        "Tx rolled back -> exit_date muss null sein, got: {:?}",
        member_json["exit_date"]
    );
}

/// Test D — Idempotenz: Member hat bereits einen Entry in Phase (z.B. aus
/// partial_repayment) -> cancel legt KEINEN zweiten Entry an; der existierende
/// Wert wird NICHT ueberschrieben.
#[tokio::test]
async fn test_cancel_membership_skips_when_entry_exists() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1303, "CancSkip").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();

    // Phase auf Open (damit partial_repayment laeuft).
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().expect("phase.id");
    let r_open = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase_id)))
        .send()
        .await
        .expect("open phase");
    assert_eq!(r_open.status(), StatusCode::OK);

    // Partial-repayment: legt Entry mit share_count=1 an.
    let r_part = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send()
        .await
        .expect("partial-repayment");
    assert_eq!(
        r_part.status(),
        StatusCode::OK,
        "partial-repayment: {}",
        r_part.text().await.unwrap_or_default()
    );

    // Cancel -> 200; kein Duplikat-Entry; existing Wert bleibt.
    let r_cancel = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("cancel");
    assert_eq!(
        r_cancel.status(),
        StatusCode::OK,
        "cancel: {}",
        r_cancel.text().await.unwrap_or_default()
    );

    // GET entries: genau ein Entry fuer Member, share_count_to_pay_out=1 (nicht 3).
    let r_list = client
        .get(server.url(&format!("/api/repayment-entry?phase_id={}", phase_id)))
        .send()
        .await
        .expect("list entries");
    assert_eq!(r_list.status(), StatusCode::OK);
    let entries: Vec<Value> = r_list.json().await.expect("decode entries");
    let member_id_str = member_id.to_string();
    let entries_for_member: Vec<&Value> = entries
        .iter()
        .filter(|e| e["member_id"].as_str() == Some(&member_id_str))
        .collect();
    assert_eq!(
        entries_for_member.len(),
        1,
        "Skip-Pattern: kein Duplikat-Entry erlaubt; got {} entries: {:?}",
        entries_for_member.len(),
        entries_for_member
    );
    assert_eq!(
        entries_for_member[0]["share_count_to_pay_out"], 1,
        "existing Wert bleibt (cancel ueberschreibt NICHT)"
    );
}

// ============================================================================
// Phase 16: partial_repayment E2E tests
// ============================================================================
//
// Test-Liste (gemaess 16-CONTEXT.md §specifics + 16-04-PLAN must_haves):
// 1. H1 happy-path (existing phase, sum-check OK)
// 2. H2 happy-path mit Auto-Anlegen-Phase
// 3. Sum-Check-Block 400 (pre-existing partial_repayment fuellt Quota)
// 4. Auto-Fill-Skip nach v1.2 partial-repayment (Plan 03 Pattern)
// 5. Full-Return-Block 400 (D-16-11)
// 6. Cancelled-Member-Block 409 (D-16-10, DIVERGENT von Phase 15 UPGD-04)
// 7. Audit-Chain-Verify (audited_create! auf Entry + ggf. Phase)
// 8. Auto-Create-Phase mit DEFAULT_SHARE_VALUE_CENT-Fallback (D-16-06/07)

fn partial_repayment_body(willensbekundung: &str, shares: i32) -> Value {
    serde_json::json!({
        "willensbekundung_date": willensbekundung,
        "shares": shares
    })
}

/// PUT Member mit angepasstem `current_shares`. Phase 15 pattern; benoetigt, weil
/// `sample_member()` `current_shares=1` setzt, partial_repayment-Tests aber
/// h&auml;ufig `current_shares=3` brauchen.
async fn put_member_current_shares(
    client: &reqwest::Client,
    server: &TestServer,
    member: &MemberTO,
    target_shares: i32,
) -> MemberTO {
    let mut updated = member.clone();
    updated.current_shares = target_shares;
    let res = client
        .put(server.url(&format!(
            "/api/members/{}",
            member.id.expect("member must have id")
        )))
        .json(&updated)
        .send()
        .await
        .expect("PUT member");
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "PUT member failed: {}",
        res.text().await.unwrap_or_default()
    );
    res.json().await.expect("decode updated MemberTO")
}

/// POST /api/repayment-phase mit fiscal_year + share_value. Phase wird in
/// Status `Preparation` angelegt (Service-default); fuer `partial_repayment`
/// reicht das aus — der Service findet die Phase per fiscal_year unabhaengig
/// vom Status (D-16-05). Returns die JSON-Phase (id + version etc.).
async fn create_repayment_phase(
    client: &reqwest::Client,
    server: &TestServer,
    fiscal_year: i32,
    share_value: i64,
) -> Value {
    let body = serde_json::json!({
        "fiscal_year": fiscal_year,
        "share_value": share_value,
    });
    let res = client
        .post(server.url("/api/repayment-phase"))
        .json(&body)
        .send()
        .await
        .expect("POST repayment-phase");
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "create phase: {}",
        res.text().await.unwrap_or_default()
    );
    res.json().await.expect("decode phase")
}

#[tokio::test]
async fn test_partial_repayment_happy_path_h1() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1100, "PartH1").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let _phase = create_repayment_phase(&client, &server, target_fy, 10000).await;

    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send()
        .await
        .expect("POST partial-repayment");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(status, StatusCode::OK, "expected 200, got {}: {}", status, body_text);

    let body: Value = serde_json::from_str(&body_text).expect("decode response");
    assert_eq!(body["entry"]["share_count_to_pay_out"], 1);
    assert_eq!(body["entry"]["status"], "Open");
    // D-16-16: phase == null wenn existing reused.
    assert!(
        body["phase"].is_null(),
        "existing phase reused; phase must be null, got {:?}",
        body["phase"]
    );
}

#[tokio::test]
async fn test_partial_repayment_happy_path_h2_with_auto_create_phase() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1101, "PartH2").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    // H2 -> Ziel-Phase ist NAECHSTES Jahr; keine Phase vor-erzeugt -> Auto-Create.
    let h2_date = today_august_15();
    let target_fy = h2_date.year() + 1;

    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h2_date.to_string(), 2))
        .send()
        .await
        .expect("POST partial-repayment");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(status, StatusCode::OK, "expected 200, got {}: {}", status, body_text);

    let body: Value = serde_json::from_str(&body_text).expect("decode");
    assert_eq!(body["entry"]["share_count_to_pay_out"], 2);
    assert!(
        !body["phase"].is_null(),
        "phase must be auto-created; got {:?}",
        body["phase"]
    );
    assert_eq!(body["phase"]["fiscal_year"], target_fy);
    // D-16-01 Variante B: Auto-Created Phase ist Open (NICHT Preparation).
    assert_eq!(body["phase"]["status"], "Open");
}

#[tokio::test]
async fn test_partial_repayment_sum_check_block_400() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1102, "PartSum").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let _phase = create_repayment_phase(&client, &server, target_fy, 10000).await;

    // 1. partial_repayment mit shares=2 → legt Open-Entry mit share_count=2 an.
    //    Service ist Status-agnostisch (anders als create_repayment_entry mit
    //    D-11.1 Phase-Open-Guard).
    let r1 = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 2))
        .send()
        .await
        .expect("first partial-repayment");
    assert_eq!(
        r1.status(),
        StatusCode::OK,
        "pre-fill entry: {}",
        r1.text().await.unwrap_or_default()
    );

    // 2. partial_repayment mit shares=2 → sum 2+2=4 > current_shares=3 → 400.
    let r2 = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 2))
        .send()
        .await
        .expect("second partial-repayment");
    assert_eq!(r2.status(), StatusCode::BAD_REQUEST);
    let body_text = r2.text().await.unwrap_or_default();
    assert!(
        body_text.contains("sum of open repayments"),
        "expected sum-check message, got: {}",
        body_text
    );
}

#[tokio::test]
async fn test_partial_repayment_auto_fill_skip_after_v12() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1103, "PartSkip").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().expect("phase.id");

    // 1) partial_repayment legt einen Entry mit share_count=1 in Phase an.
    let r1 = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send()
        .await
        .expect("partial-repayment");
    assert_eq!(r1.status(), StatusCode::OK);

    // 2) Phase oeffnen → triggert Auto-Fill. Plan 03 Skip-Pattern darf KEINEN
    //    Duplikat-Entry fuer diesen Member erzeugen.
    let r_open = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase_id)))
        .send()
        .await
        .expect("open phase");
    assert_eq!(
        r_open.status(),
        StatusCode::OK,
        "open phase: {}",
        r_open.text().await.unwrap_or_default()
    );

    // 3) Liste Entries der Phase, filtere nach Member-ID.
    let r_list = client
        .get(server.url(&format!("/api/repayment-entry?phase_id={}", phase_id)))
        .send()
        .await
        .expect("list entries");
    assert_eq!(r_list.status(), StatusCode::OK);
    let entries: Vec<Value> = r_list.json().await.expect("decode entries");
    let member_id_str = member_id.to_string();
    let entries_for_member: Vec<&Value> = entries
        .iter()
        .filter(|e| e["member_id"].as_str() == Some(&member_id_str))
        .collect();
    assert_eq!(
        entries_for_member.len(),
        1,
        "Auto-fill skip MUST prevent duplicate; got {} entries: {:?}",
        entries_for_member.len(),
        entries_for_member
    );
    // Der erhaltene Entry soll der partial-repayment-Entry (share_count=1) sein,
    // NICHT der Auto-Fill-Default (current_shares=3).
    assert_eq!(
        entries_for_member[0]["share_count_to_pay_out"], 1,
        "partial-repayment entry preserved, NOT replaced by auto-fill"
    );
}

#[tokio::test]
async fn test_partial_repayment_full_return_block_400() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1104, "PartFull").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let _phase = create_repayment_phase(&client, &server, target_fy, 10000).await;

    // shares == current_shares (3) → 400 mit cancel_membership-Hinweis (D-16-11).
    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 3))
        .send()
        .await
        .expect("partial-repayment");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body_text = resp.text().await.unwrap_or_default();
    assert!(
        body_text.contains("cancel_membership"),
        "D-16-11 hint missing; body: {}",
        body_text
    );
}

#[tokio::test]
async fn test_partial_repayment_cancelled_member_block_409() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1105, "PartCanc").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let _phase = create_repayment_phase(&client, &server, target_fy, 10000).await;

    // 1) Member kuendigen → setzt exit_date.
    let r_cancel = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&cancel_body(&h1_date.to_string()))
        .send()
        .await
        .expect("cancel");
    assert_eq!(r_cancel.status(), StatusCode::OK);

    // 2) partial-repayment auf gekuendigtes Member → 409 (D-16-10, NICHT 400
    //    wie Phase 15 UPGD-04).
    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send()
        .await
        .expect("partial-repayment");
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "D-16-10: cancelled member MUST return 409 (NOT 400 like Phase 15 UPGD-04). body: {}",
        resp.text().await.unwrap_or_default()
    );
}

#[tokio::test]
async fn test_partial_repayment_audit_chain_verify() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1106, "PartAudit").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let _phase = create_repayment_phase(&client, &server, target_fy, 10000).await;

    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send()
        .await
        .expect("partial-repayment");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("decode");
    let entry_id = body["entry"]["id"].as_str().expect("entry.id");

    // AUDT-01: Hashchain bleibt valid.
    let r_verify = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("verify");
    assert_eq!(r_verify.status(), StatusCode::OK);
    let verify: Value = r_verify.json().await.expect("verify json");
    assert_eq!(
        verify["valid"], true,
        "audit-chain corrupted after partial_repayment: {:?}",
        verify
    );

    // Audit-Eintraege fuer den neuen Entry sind via PARTIAL_REPAYMENT_PROCESS
    // angelegt; GET /api/audit/{entity_type}/{entity_id} liefert sie.
    let r_audit = client
        .get(server.url(&format!("/api/audit/repayment_entry/{}", entry_id)))
        .send()
        .await
        .expect("audit by entry");
    assert_eq!(r_audit.status(), StatusCode::OK);
    let audit_body = r_audit.text().await.unwrap_or_default();
    assert!(
        audit_body.contains("member-adjust.partial-repayment"),
        "audit entries must reference PARTIAL_REPAYMENT_PROCESS; body: {}",
        audit_body
    );
}

#[tokio::test]
async fn test_partial_repayment_auto_creates_phase_with_default_share_value() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1107, "PartDefault").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    // Frisches Setup — KEINE Phase vor-erzeugt → Auto-Create triggert
    // DEFAULT_SHARE_VALUE_CENT = 10000 (D-16-06/07).
    let h2_date = today_august_15();
    let target_fy = h2_date.year() + 1;

    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h2_date.to_string(), 1))
        .send()
        .await
        .expect("partial-repayment");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "expected 200: {}",
        resp.text().await.unwrap_or_default()
    );
    let body: Value = resp.json().await.expect("decode");
    assert!(!body["phase"].is_null(), "phase auto-created; got null");
    assert_eq!(
        body["phase"]["share_value"], 10000,
        "D-16-06/07: DEFAULT_SHARE_VALUE_CENT fallback"
    );
    assert_eq!(body["phase"]["fiscal_year"], target_fy);
    assert_eq!(body["phase"]["status"], "Open");
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

/// Phase 16.05 / CR-01 — Closed phase darf KEINEN partial_repayment-Entry aufnehmen.
/// Sequenz: Phase create (Preparation) -> POST /open -> POST /close -> POST partial-repayment
/// -> erwartet HTTP 409 Conflict mit Body-Substring "closed".
#[tokio::test]
async fn test_partial_repayment_closed_phase_returns_409() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1108, "PartClosed").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let member_id = m.id.expect("id");

    let h1_date = today_march_15();
    let target_fy = h1_date.year();

    // 1) Phase fuer target_fy anlegen (Preparation).
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().expect("phase.id");

    // 2) Phase oeffnen.
    let r_open = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase_id)))
        .send()
        .await
        .expect("open phase");
    assert_eq!(
        r_open.status(),
        StatusCode::OK,
        "open phase: {}",
        r_open.text().await.unwrap_or_default()
    );

    // 3) Phase schliessen.
    let r_close = client
        .post(server.url(&format!("/api/repayment-phase/{}/close", phase_id)))
        .send()
        .await
        .expect("close phase");
    assert_eq!(
        r_close.status(),
        StatusCode::OK,
        "close phase: {}",
        r_close.text().await.unwrap_or_default()
    );

    // 4) partial-repayment auf geschlossene Phase -> 409 Conflict (D-11.1 Guard).
    let resp = client
        .post(server.url(&format!("/api/members/{}/partial-repayment", member_id)))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send()
        .await
        .expect("POST partial-repayment");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "expected 409 Conflict for closed phase, got {}: {}",
        status,
        body_text
    );
    assert!(
        body_text.contains("closed"),
        "expected body to mention 'closed', got: {}",
        body_text
    );
    assert!(
        body_text.contains(&target_fy.to_string()),
        "expected body to mention fiscal_year {}, got: {}",
        target_fy,
        body_text
    );
}

// ============================================================
// Phase 17 - Uebertrag (Transfer Shares)
// SC #5: 8 E2E-Tests fuer POST /api/members/{from_id}/transfer-shares
// Decken TRSF-01..05, TRSF-07, AUDT-02, PERM-03 ab + D-17-06 Race-Patterns.
// ============================================================

/// JSON-Body fuer POST /api/members/{from_id}/transfer-shares.
/// Spiegelt `TransferSharesRequestTO` aus genossi_rest_types (Plan 17-03).
fn transfer_shares_body(to: &Uuid, shares: i32, transfer_date: &str) -> Value {
    serde_json::json!({
        "to_member_id": to.to_string(),
        "shares": shares,
        "transfer_date": transfer_date,
    })
}

/// Test 1 (TRSF-01 / TRSF-04) — Teil-Uebertrag Happy-Path.
///
/// Setup: A.current_shares=5, B.current_shares=1. Transfer shares=2.
/// Assertion: 200, actions.len()==2 (Abgabe + Empfang),
/// from.current_shares==3, from.exit_date is null (kein Voll-Uebertrag),
/// to.current_shares==3 (=1+2), to bleibt aktiv.
#[tokio::test]
async fn test_transfer_shares_partial_happy_path() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1200, "FromA").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let b = create_active_member(&client, &server, 1201, "ToB").await;

    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");

    let transfer_date = today_march_15();

    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&b_id, 2, &transfer_date.to_string()))
        .send()
        .await
        .expect("POST transfer-shares");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "Happy-Path muss 200 returnen; body: {}",
        body_text
    );

    let body: Value = serde_json::from_str(&body_text).expect("decode TransferSharesResponseTO");

    let actions = body["actions"].as_array().expect("actions array");
    assert_eq!(actions.len(), 2, "Teil-Uebertrag = 2 Actions (Abgabe + Empfang)");

    // from-Mitglied bleibt aktiv mit reduzierten Anteilen.
    assert_eq!(
        body["from"]["current_shares"].as_i64().expect("from.current_shares"),
        3,
        "from.current_shares = 5 - 2"
    );
    assert!(
        body["from"]["exit_date"].is_null(),
        "Teil-Uebertrag darf KEIN exit_date setzen"
    );

    // to-Mitglied bekommt +2 Anteile (sample_member shares_at_joining = 1, +2 = 3).
    assert_eq!(
        body["to"]["current_shares"].as_i64().expect("to.current_shares"),
        3,
        "to.current_shares = 1 + 2"
    );
    assert!(body["to"]["exit_date"].is_null(), "to bleibt aktiv");
}

/// Test 2 (TRSF-03 / TRSF-05 / D-17-01..03) — Voll-Uebertrag mit exit_date-Cascade.
///
/// Setup: A.current_shares=3, B.current_shares=1. Transfer shares=3 (Voll-Uebertrag).
/// Assertion: 200, actions.len()==3 (Abgabe + Empfang + Austritt),
/// 3. Action ist Austritt mit transfer_member_id=Some(b_id) + effective_date=transfer_date,
/// from.current_shares==0, from.exit_date==transfer_date (recalc_dates Cascade).
#[tokio::test]
async fn test_transfer_shares_full_with_exit_date_cascade() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1210, "FullFromA").await;
    let a = put_member_current_shares(&client, &server, &a, 3).await;
    let b = create_active_member(&client, &server, 1211, "FullToB").await;

    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");
    let transfer_date = today_march_15();

    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&b_id, 3, &transfer_date.to_string()))
        .send()
        .await
        .expect("POST transfer-shares full");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "Voll-Uebertrag muss 200 returnen; body: {}",
        body_text
    );

    let body: Value = serde_json::from_str(&body_text).expect("decode");
    let actions = body["actions"].as_array().expect("actions");

    // D-17-01: Voll-Uebertrag = 3 Actions.
    assert_eq!(actions.len(), 3, "Voll-Uebertrag = 3 Actions");

    // D-17-03: 3. Action ist Austritt mit transfer_member_id=Some(b_id),
    // effective_date=Some(transfer_date).
    let last_action = &actions[2];
    assert_eq!(
        last_action["action_type"].as_str().expect("action_type"),
        "Austritt",
        "3. Action = Austritt"
    );
    assert_eq!(
        last_action["transfer_member_id"].as_str(),
        Some(b_id.to_string().as_str()),
        "Austritt.transfer_member_id = b_id (D-17-03 — divergiert von Phase-15 CANC)"
    );
    assert_eq!(
        last_action["effective_date"].as_str().expect("effective_date"),
        transfer_date.to_string().as_str(),
        "Austritt.effective_date = transfer_date (TRSF-05 sofort wirksam)"
    );

    // D-17-02 + TRSF-04: from.current_shares == 0; from.exit_date == transfer_date
    // (via recalc_dates Cascade).
    assert_eq!(
        body["from"]["current_shares"].as_i64().expect("from.current_shares"),
        0,
        "Voll-Uebertrag: from.current_shares = 0"
    );
    assert_eq!(
        body["from"]["exit_date"].as_str().expect("from.exit_date"),
        transfer_date.to_string().as_str(),
        "Voll-Uebertrag: from.exit_date = transfer_date"
    );

    // to bleibt aktiv und bekommt +3.
    assert_eq!(
        body["to"]["current_shares"].as_i64().expect("to.current_shares"),
        4,
        "to.current_shares = 1 + 3"
    );
    assert!(body["to"]["exit_date"].is_null(), "to bleibt aktiv");
}

/// Quick 260608-jb1 — Voll-Uebertrag erzeugt RepaymentEntry fuer entleerten Sender.
///
/// Setup: A.current_shares=3, B aktiv. RepaymentPhase fuer aktuelles fiscal_year
/// (transfer_date=today_march_15 -> H1, fiscal_year = aktuelles Jahr) auf Open.
/// Action: transfer A->B mit shares=3 (Voll-Uebertrag, A wird leer).
/// Assertion: A.current_shares=0, A.exit_date=transfer_date, RepaymentEntry
/// fuer A.id mit share_count_to_pay_out=3 (Wert VOR Decrement) und status=Open.
/// B bleibt aktiv ohne exit_date.
#[tokio::test]
async fn test_transfer_shares_full_creates_repayment_entry() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1212, "FullTransRepayA").await;
    let a = put_member_current_shares(&client, &server, &a, 3).await;
    let b = create_active_member(&client, &server, 1213, "FullTransRepayB").await;

    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");

    let transfer_date = today_march_15();
    let target_fy = transfer_date.year();

    // RepaymentPhase fuer aktuelles fiscal_year anlegen + auf Open transitionieren.
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().expect("phase.id");
    let r_open = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase_id)))
        .send()
        .await
        .expect("open phase");
    assert_eq!(
        r_open.status(),
        StatusCode::OK,
        "open phase: {}",
        r_open.text().await.unwrap_or_default()
    );

    // Voll-Uebertrag A -> B.
    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&b_id, 3, &transfer_date.to_string()))
        .send()
        .await
        .expect("POST transfer-shares full");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "Voll-Uebertrag muss 200 returnen; body: {}",
        body_text
    );

    let body: Value = serde_json::from_str(&body_text).expect("decode");

    // A wird leer + exit_date.
    assert_eq!(
        body["from"]["current_shares"].as_i64().expect("from.current_shares"),
        0,
        "A.current_shares = 0"
    );
    assert_eq!(
        body["from"]["exit_date"].as_str().expect("from.exit_date"),
        transfer_date.to_string().as_str(),
        "A.exit_date = transfer_date"
    );
    // B bleibt aktiv.
    assert!(body["to"]["exit_date"].is_null(), "B bleibt aktiv");

    // RepaymentEntry fuer A muss in der Phase existieren mit share_count=3.
    let r_list = client
        .get(server.url(&format!("/api/repayment-entry?phase_id={}", phase_id)))
        .send()
        .await
        .expect("list entries");
    assert_eq!(r_list.status(), StatusCode::OK);
    let entries: Vec<Value> = r_list.json().await.expect("decode entries");
    let a_id_str = a_id.to_string();
    let entries_for_a: Vec<&Value> = entries
        .iter()
        .filter(|e| e["member_id"].as_str() == Some(&a_id_str))
        .collect();
    assert_eq!(
        entries_for_a.len(),
        1,
        "expected exactly 1 RepaymentEntry for A; got {} entries: {:?}",
        entries_for_a.len(),
        entries_for_a
    );
    assert_eq!(
        entries_for_a[0]["share_count_to_pay_out"], 3,
        "share_count_to_pay_out muss shares=3 (Wert VOR Decrement) sein, NICHT 0"
    );
    assert_eq!(
        entries_for_a[0]["status"], "Open",
        "neuer Entry muss Status Open haben"
    );
}

/// Test 3 (TRSF-07 / D-17-08) — Self-Transfer 400.
///
/// Setup: A.current_shares=5. POST mit body.to_member_id == a_id.
/// Assertion: 400, body enthaelt "cannot transfer to self".
#[tokio::test]
async fn test_transfer_shares_self_transfer_400() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1220, "Self").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let a_id = a.id.expect("a.id");
    let transfer_date = today_march_15();

    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&a_id, 1, &transfer_date.to_string()))
        .send()
        .await
        .expect("POST self-transfer");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "Self-Transfer muss 400 returnen (TRSF-07 / D-17-08)"
    );
    let body = resp.text().await.unwrap_or_default();
    assert!(
        body.contains("cannot transfer to self"),
        "Body muss Hinweis enthalten; got: {}",
        body
    );
}

/// Test 4 (PERM-03 / D-17-07) — Recipient cancelled 409.
///
/// Setup: A aktiv, B gekuendigt (POST /cancel mit willensbekundung_date).
/// Action: POST transfer A->B.
/// Assertion: 409, body enthaelt "recipient already cancelled".
#[tokio::test]
async fn test_transfer_shares_recipient_cancelled_409() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1230, "ActiveFrom").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let b = create_active_member(&client, &server, 1231, "ToBeCancelledB").await;

    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");

    // B kuendigen (Phase-15 cancel-Endpoint).
    // Pattern aus test_partial_repayment_cancelled_member_block_409 (Z. 746-780).
    let cancel_resp = client
        .post(server.url(&format!("/api/members/{}/cancel", b_id)))
        .json(&cancel_body(&today_march_15().to_string()))
        .send()
        .await
        .expect("POST cancel B");
    assert_eq!(
        cancel_resp.status(),
        StatusCode::OK,
        "Cancel B muss 200; got body: {}",
        cancel_resp.text().await.unwrap_or_default()
    );

    // Jetzt Transfer A -> B versuchen.
    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&b_id, 2, &today_march_15().to_string()))
        .send()
        .await
        .expect("POST transfer A->B");
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "PERM-03: gekuendigter Empfaenger muss 409 (D-17-07)"
    );
    let body = resp.text().await.unwrap_or_default();
    assert!(
        body.contains("recipient already cancelled"),
        "Body muss Hinweis enthalten; got: {}",
        body
    );
}

/// Test 5 (D-17-10) — Recipient not found 404.
///
/// Action: POST transfer mit to_member_id = Uuid::new_v4() (nicht existent).
/// Assertion: 404.
#[tokio::test]
async fn test_transfer_shares_recipient_not_found_404() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1240, "FromA").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let a_id = a.id.expect("a.id");
    let fake_b_id = Uuid::new_v4();

    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&fake_b_id, 2, &today_march_15().to_string()))
        .send()
        .await
        .expect("POST transfer fake recipient");
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Nicht-existierender Empfaenger muss 404"
    );
}

/// D-17-05 / AUDT-02 Doppel-Assertion fuer den Uebertrag-Audit-Trail.
///
/// **Plan-Drift-Notiz (Rule 1 auto-fix):** Plan-Truth #6 nahm an, alle
/// Transfer-Audit-Eintraege teilen EINE `transaction_id`. Realitaet
/// (`genossi_service_impl/src/audit_log.rs:65`): `build_audit_entries`
/// generiert eine NEUE `transaction_id` PRO `audited_*!`-Macro-Aufruf.
/// `transaction_id` gruppiert nur die Field-Level-Rows einer EINZELNEN
/// Macro-Invocation, nicht den gesamten Service-Pipeline-Aufruf.
///
/// Die Plan-Intention "Atomarity" wird stattdessen ueber drei robustere
/// Kriterien sichergestellt:
/// (a) Audit-Hashchain bleibt valid — der staerkste Atomarity-Beweis: alle
///     Eintraege wurden in EINER DB-Transaction comitted (sonst waere bei
///     einem Partial-Commit die Hashchain gebrochen).
/// (b) Anzahl distinkter `transaction_id`s entspricht der erwarteten Anzahl
///     `audited_*!`-Macro-Aufrufe (Teil-Uebertrag: 4 = abgabe + empfang
///     + from + to; Voll-Uebertrag: 5 = + austritt).
/// (c) Anzahl distinkter MemberAction-`entity_id`s = expected_action_count
///     (Teil=2, Voll=3) — bewahrt Plan-Truth #6 Teil-(b).
///
/// JSON-Schema (verifiziert in genossi_rest_types/src/lib.rs:1741-1783):
///   root: { entries: [AuditLogEntryTO], total, page, size }
///   entry: { id, timestamp, user_id, process, transaction_id, entity_type,
///            entity_id, action, field_name, old_value?, new_value? }
///
/// Default-Page-Size=50; ?size=200 schuetzt vor Silent-Pass (WARNING #4),
/// auch wenn die Audit-Tabelle waechst.
async fn assert_transfer_audit_trail(
    client: &reqwest::Client,
    server: &TestServer,
    expected_action_count: usize,
) {
    let resp = client
        .get(server.url("/api/audit?size=200"))
        .send()
        .await
        .expect("GET /api/audit");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("decode audit list");
    let entries = body["entries"].as_array().expect("entries array");

    // Filter auf Phase-17-Process.
    let transfer_entries: Vec<&Value> = entries
        .iter()
        .filter(|e| e["process"].as_str() == Some("member-adjust.transfer"))
        .collect();

    assert!(
        !transfer_entries.is_empty(),
        "es muessen Transfer-Audit-Entries existieren"
    );

    // WARNING #4 — Empty-Array-Schutz gegen Silent-Pass.
    // AUDT-02 erwartet: pro MemberAction-create ~1 Audit-Row pro Feld + pro
    // Member-Update ~1 Audit-Row pro veraendertem Feld.
    // Teil-Uebertrag: 2 Action-creates + 2 Member-Updates (mind. current_shares)
    //                 >= 4 Rows (typisch deutlich mehr, da Action mehrere Felder hat).
    // Voll-Uebertrag: 3 Action-creates + 2 Member-Updates (current_shares + exit_date)
    //                 >= 5 Rows.
    // Falls weniger: JSON-Schema-Mismatch (z.B. Field-Name "tx_id" statt
    // "transaction_id"), und Folge-Checks koennten silent passen.
    assert!(
        transfer_entries.len() >= 4,
        "expected >=4 audit rows for 2 actions + 2 member updates per AUDT-02; \
         got {} — JSON schema mismatch?",
        transfer_entries.len()
    );

    // (a) Audit-Hashchain bleibt valid — bester Beweis, dass alle Writes in
    //     EINER DB-Transaction commit gemacht haben (sonst waere bei einem
    //     Partial-Commit die Hashchain-Verlinkung gebrochen).
    let verify_resp = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("GET /api/audit/verify");
    let verify_body: Value = verify_resp.json().await.expect("decode verify");
    assert_eq!(
        verify_body["valid"].as_bool(),
        Some(true),
        "Audit-Hashchain muss valid bleiben (Atomarity-Beweis)"
    );

    // (b) Anzahl distinkter transaction_ids entspricht der Anzahl
    //     audited_*!-Macro-Aufrufe. Teil-Uebertrag = 4 (abgabe-create +
    //     empfang-create + from-update + to-update). Voll-Uebertrag = 5
    //     (zusaetzlicher austritt-create).
    let expected_tx_count = if expected_action_count == 2 { 4 } else { 5 };
    let mut distinct_tx_ids = std::collections::HashSet::new();
    for e in &transfer_entries {
        let tx = e["transaction_id"]
            .as_str()
            .expect("transaction_id")
            .to_string();
        distinct_tx_ids.insert(tx);
    }
    assert_eq!(
        distinct_tx_ids.len(),
        expected_tx_count,
        "AUDT-02: erwarte {} distinkte transaction_ids (1 pro audited_*!-Aufruf), \
         got {}",
        expected_tx_count,
        distinct_tx_ids.len()
    );

    // (c) MemberAction-Entity-Count == expected_action_count.
    //     Jede MemberAction kann mehrere Audit-Rows haben (1 pro changed-field).
    //     Gruppe nach entity_id, dann zaehle distinkte entity_ids.
    //     entity_type-Schluessel = "member_action" per
    //     `genossi_dao/src/member_action.rs:68` (NICHT "MemberAction" — vgl.
    //     `impl Auditable for MemberActionEntity`).
    let member_action_entries: Vec<&Value> = transfer_entries
        .iter()
        .filter(|e| e["entity_type"].as_str() == Some("member_action"))
        .copied()
        .collect();
    let mut distinct_ids = std::collections::HashSet::new();
    for e in &member_action_entries {
        if let Some(id) = e["entity_id"].as_str() {
            distinct_ids.insert(id.to_string());
        }
    }
    assert_eq!(
        distinct_ids.len(),
        expected_action_count,
        "D-17-05 (b): MemberAction-Count == {} (Teil=2, Voll=3)",
        expected_action_count
    );
}

/// Test 6 (AUDT-02 / D-17-05) — Audit-Trail-Verifikation fuer Teil-Uebertrag.
///
/// Nach erfolgreichem Teil-Uebertrag (2 MemberAction-Eintraege):
/// (a) Audit-Hashchain bleibt valid (Atomarity-Beweis);
/// (b) 4 distinkte transaction_ids (1 pro audited_*!-Aufruf:
///     abgabe-create + empfang-create + from-update + to-update);
/// (c) 2 distinkte MemberAction-entity_ids.
///
/// Details zur Plan-Drift siehe `assert_transfer_audit_trail`-Doc.
#[tokio::test]
async fn test_transfer_shares_audit_pair_verify_doppel_assertion() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1250, "AuditFrom").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let b = create_active_member(&client, &server, 1251, "AuditTo").await;
    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");

    // Teil-Uebertrag: 2 MemberAction-Eintraege erwartet.
    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&b_id, 2, &today_march_15().to_string()))
        .send()
        .await
        .expect("POST transfer for audit-verify");
    assert_eq!(resp.status(), StatusCode::OK);

    assert_transfer_audit_trail(&client, &server, 2).await;
}

/// Test 7 (D-17-06) — Race Same-Direction, SQLITE_BUSY-Pfad.
///
/// Setup: A.current_shares=4, B aktiv. Bei shares=2 koennen NICHT beide
/// gleichzeitig durchgehen (A waere sonst auf -1 reduziert).
///
/// Pattern (analog test_mark_paid_out_race_one_succeeds_one_conflicts):
///   - Pool-Warm-up via 1ms tokio::sleep (Pitfall #11).
///   - tokio::join! zwei identische POSTs.
///   - sortierte Statuses [200, 409|500]; NIE [200, 200].
///
/// Post-Konsistenz: A.current_shares = 4 - 2 = 2 (genau eine Cascade gelang).
/// Audit-Chain bleibt valid (Verlierer-Tx sauber rolled-back).
#[tokio::test]
async fn test_transfer_shares_race_same_direction_sqlite_busy() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1260, "RaceFromA").await;
    let a = put_member_current_shares(&client, &server, &a, 4).await;
    let b = create_active_member(&client, &server, 1261, "RaceToB").await;
    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");

    let url = server.url(&format!("/api/members/{}/transfer-shares", a_id));
    let body = transfer_shares_body(&b_id, 2, &today_march_15().to_string());

    // Pool-Warm-up (Pitfall #11).
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;

    // D-17-06: Beide POSTs parallel via tokio::join!.
    let (resp_a, resp_b) = tokio::join!(
        client.post(&url).json(&body).send(),
        client.post(&url).json(&body).send(),
    );
    let r_a = resp_a.expect("race a");
    let r_b = resp_b.expect("race b");
    let status_a = r_a.status();
    let status_b = r_b.status();

    let mut statuses = [status_a, status_b];
    statuses.sort_by_key(|s| s.as_u16());

    assert_eq!(
        statuses[0],
        StatusCode::OK,
        "D-17-06: genau ein Race-Aufruf muss 200 sein; got {:?}",
        statuses
    );
    assert!(
        statuses[1] == StatusCode::CONFLICT || statuses[1] == StatusCode::INTERNAL_SERVER_ERROR,
        "D-17-06: Race-Verlierer muss 409 ODER 500 sein; got {:?}",
        statuses
    );
    assert!(
        !(status_a == StatusCode::OK && status_b == StatusCode::OK),
        "D-17-06: NIE [200, 200] (waere Double-Cascade)"
    );

    // Post-Konsistenz: A.current_shares == 4 - 2 == 2 (genau 1 erfolgreiche Cascade).
    let a_after_resp = client
        .get(server.url(&format!("/api/members/{}", a_id)))
        .send()
        .await
        .expect("GET A");
    let a_after: Value = a_after_resp.json().await.expect("decode A");
    assert_eq!(
        a_after["current_shares"].as_i64().expect("a.current_shares"),
        2,
        "Race-Sieger reduziert A.current_shares genau einmal"
    );

    // Audit-Chain bleibt valid.
    let verify: Value = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("GET verify")
        .json()
        .await
        .expect("decode verify");
    assert_eq!(verify["valid"].as_bool(), Some(true), "Audit-Chain valid");
}

/// Test 8 (D-17-06) — Race Cross-Direction Consistency-Check.
///
/// Setup: A.current_shares=5, B.current_shares=5. Cross-Transfer 2 in beide
/// Richtungen (A->B und B->A) parallel.
///
/// Per D-17-06 / threat_model T-17-04-07: [200, 200] ist bei Cross-Direction
/// ERLAUBT (keine konkurrierenden Locks auf dem gleichen Member-Row).
/// Akzeptierte Statuses: [(200, 200), (200, 409|500)].
/// VERBOTEN: [409|500, 409|500] (waere Total-Deadlock).
///
/// Post-Konsistenz: Anteile-Summe A+B bleibt erhalten (Start: 5+5=10).
/// Audit-Chain bleibt valid.
#[tokio::test]
async fn test_transfer_shares_race_cross_direction_consistency_check() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let a = create_active_member(&client, &server, 1270, "CrossA").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let b = create_active_member(&client, &server, 1271, "CrossB").await;
    let b = put_member_current_shares(&client, &server, &b, 5).await;
    let a_id = a.id.expect("a.id");
    let b_id = b.id.expect("b.id");

    let url_ab = server.url(&format!("/api/members/{}/transfer-shares", a_id));
    let url_ba = server.url(&format!("/api/members/{}/transfer-shares", b_id));
    let body_ab = transfer_shares_body(&b_id, 2, &today_march_15().to_string());
    let body_ba = transfer_shares_body(&a_id, 2, &today_march_15().to_string());

    // Pool-Warm-up (Pitfall #11).
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;

    let (resp_ab, resp_ba) = tokio::join!(
        client.post(&url_ab).json(&body_ab).send(),
        client.post(&url_ba).json(&body_ba).send(),
    );
    let r_ab = resp_ab.expect("race ab");
    let r_ba = resp_ba.expect("race ba");
    let status_ab = r_ab.status();
    let status_ba = r_ba.status();

    // D-17-06: NIE Total-Deadlock (beide failen).
    let both_failed = (status_ab == StatusCode::CONFLICT
        || status_ab == StatusCode::INTERNAL_SERVER_ERROR)
        && (status_ba == StatusCode::CONFLICT
            || status_ba == StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        !both_failed,
        "D-17-06: NIE Total-Deadlock; got [{}, {}]",
        status_ab, status_ba
    );

    // Konsistenz: Anteile-Summe bleibt erhalten (a_start + b_start = 10).
    let a_after: Value = client
        .get(server.url(&format!("/api/members/{}", a_id)))
        .send()
        .await
        .expect("GET A")
        .json()
        .await
        .expect("decode A");
    let b_after: Value = client
        .get(server.url(&format!("/api/members/{}", b_id)))
        .send()
        .await
        .expect("GET B")
        .json()
        .await
        .expect("decode B");
    let total_after = a_after["current_shares"].as_i64().expect("a.current_shares")
        + b_after["current_shares"].as_i64().expect("b.current_shares");
    assert_eq!(
        total_after, 10,
        "Cross-Race: Anteile-Summe muss erhalten bleiben (5+5=10); got A={}, B={}",
        a_after["current_shares"], b_after["current_shares"]
    );

    // Audit-Chain bleibt valid.
    let verify: Value = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("GET verify")
        .json()
        .await
        .expect("decode verify");
    assert_eq!(verify["valid"].as_bool(), Some(true), "Audit-Chain valid");
}
