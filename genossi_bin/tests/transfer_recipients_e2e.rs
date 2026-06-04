#![cfg(feature = "mock_auth")]
//! Phase 14 Plan 04 — E2E-Tests fuer `GET /api/members/transfer-recipients?exclude_self={uuid}`.
//!
//! Verifiziert TRSF-06 end-to-end:
//! - 3 Members angelegt: 1 aktiv, 1 mit `exit_date` (via 3-Step-Setup), 1 self.
//! - Endpoint liefert exakt 1 Empfaenger zurueck (den aktiven Nicht-Self).
//! - Self ist via `?exclude_self=` ausgefiltert.
//! - Gekuendigtes Member ist via `exit_date IS NOT NULL`-Filter ausgefiltert.
//!
//! Adressiert folgende Pitfalls aus 14-RESEARCH.md:
//! - Pitfall 1 (Sub-Route-Ordering): Der echte HTTP-Roundtrip verifiziert, dass
//!   axum `/transfer-recipients` nicht als UUID parsed — sonst waere die
//!   Antwort 400, nicht 200.
//! - Pitfall 3 (3-Step exit_date setup): `create_cancelled_member` postet
//!   Member → Austritt-Action → re-fetched Member, damit `recalc_dates` das
//!   `exit_date` setzt. Direktes `MemberTO.exit_date` im Create wird vom
//!   Service-Layer ignoriert.
//! - PII-Leak-Guard: Response-Body wird gegen sensible Felder (email,
//!   bank_account, street, iban, current_shares) gegrep-asserted.

use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::{start_test_server, TestServer};
use genossi_rest_types::{ActionTypeTO, MemberActionTO, MemberSlimTO, MemberTO};
use reqwest::StatusCode;
use sqlx::SqlitePool;
use std::sync::Arc;

// ============================================================================
// Test-Setup (1:1 aus e2e_tests.rs::setup uebernommen)
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
        join_date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
        shares_at_joining: 1,
        current_shares: 1,
        current_balance: 0,
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

/// Erzeugt ein aktives Mitglied (1 POST). `exit_date` bleibt None — keine
/// Austritt-Action.
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
        "create_member expected 2xx, got {}",
        response.status()
    );
    response.json().await.expect("decode MemberTO")
}

/// Erzeugt ein gekuendigtes Mitglied via 3-Step-Setup (Pitfall 3):
/// 1. POST `/api/members` mit `exit_date: None`.
/// 2. POST `/api/members/{id}/actions` mit `MemberAction::Austritt`.
/// 3. GET `/api/members/{id}` — `recalc_dates` hat `exit_date` jetzt gesetzt.
///
/// Direktes Setzen von `MemberTO.exit_date` im Create wird vom Service-Layer
/// ueberschrieben — daher das 3-Step-Verfahren.
async fn create_cancelled_member(
    client: &reqwest::Client,
    server: &TestServer,
    member_number: i64,
) -> MemberTO {
    // Step 1: create active member.
    let m = create_active_member(client, server, member_number, "Cancelled").await;
    let member_id = m.id.expect("created member must have id");

    // Step 2: POST Austritt-Action — setzt exit_date via recalc_dates.
    let exit_date = time::Date::from_calendar_date(2026, time::Month::June, 15).unwrap();
    let austritt = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Austritt,
        date: exit_date,
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(exit_date),
        comment: Some("Phase 14 E2E exit_date setup".to_string()),
        created: None,
        deleted: None,
        version: None,
    };
    let response = client
        .post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&austritt)
        .send()
        .await
        .expect("POST Austritt action failed");
    assert!(
        response.status().is_success(),
        "POST Austritt expected 2xx, got {}: {}",
        response.status(),
        response.text().await.unwrap_or_default()
    );

    // Step 3: GET member — recalc_dates has now set exit_date.
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .expect("GET member failed");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("decode MemberTO")
}

// ============================================================================
// Test
// ============================================================================

#[tokio::test]
async fn test_transfer_recipients_filters_self_and_cancelled() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // 3 Members anlegen: aktiv, gekuendigt, self.
    let m_active = create_active_member(&client, &server, 1001, "Aktiv").await;
    let m_cancelled = create_cancelled_member(&client, &server, 1002).await;
    let m_self = create_active_member(&client, &server, 1003, "Self").await;

    // Pitfall 3 Sanity-Check: 3-Step-Setup hat das exit_date wirklich gesetzt.
    assert!(
        m_cancelled.exit_date.is_some(),
        "3-Step-Setup MUSS exit_date via recalc_dates populated haben — \
         sonst greift der service-seitige exit_date IS NULL-Filter nicht"
    );

    let self_id = m_self.id.expect("self member must have id");
    let active_id = m_active.id.expect("active member must have id");

    // GET /api/members/transfer-recipients?exclude_self=<self_id>
    let resp = client
        .get(server.url(&format!(
            "/api/members/transfer-recipients?exclude_self={}",
            self_id
        )))
        .send()
        .await
        .expect("GET transfer-recipients failed");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Endpoint must return 200 — Pitfall 1 check: sub-route /transfer-recipients \
         is matched BEFORE /{{id}}, so the literal is not parsed as a UUID"
    );

    let recipients: Vec<MemberSlimTO> = resp.json().await.expect("decode Vec<MemberSlimTO>");

    assert_eq!(
        recipients.len(),
        1,
        "Expected exactly 1 recipient: m_cancelled filtered by exit_date, \
         m_self filtered by exclude_self. Got: {:?}",
        recipients
            .iter()
            .map(|r| (r.member_number, &r.first_name))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        recipients[0].id, active_id,
        "The remaining recipient must be the active non-self member"
    );
    assert_eq!(recipients[0].member_number, 1001);

    // PII-Leak-Guard: response body MUST NOT contain sensitive MemberTO fields.
    // We re-fetch and inspect the raw body to catch silent schema drift if
    // someone ever adds `impl From<&MemberTO> for MemberSlimTO`.
    let resp2 = client
        .get(server.url(&format!(
            "/api/members/transfer-recipients?exclude_self={}",
            self_id
        )))
        .send()
        .await
        .expect("GET transfer-recipients (second pass) failed");
    let body = resp2.text().await.expect("read body text");
    assert!(
        !body.contains("\"iban\""),
        "MemberSlimTO leaked iban field: {}",
        body
    );
    assert!(
        !body.contains("\"email\""),
        "MemberSlimTO leaked email field: {}",
        body
    );
    assert!(
        !body.contains("\"bank_account\""),
        "MemberSlimTO leaked bank_account field: {}",
        body
    );
    assert!(
        !body.contains("\"street\""),
        "MemberSlimTO leaked street field: {}",
        body
    );
    assert!(
        !body.contains("\"current_shares\""),
        "MemberSlimTO leaked current_shares field: {}",
        body
    );
    assert!(
        !body.contains("\"current_balance\""),
        "MemberSlimTO leaked current_balance field: {}",
        body
    );
    assert!(
        !body.contains("\"postal_code\""),
        "MemberSlimTO leaked postal_code field: {}",
        body
    );
}
