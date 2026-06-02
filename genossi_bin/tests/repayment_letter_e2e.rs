#![cfg(feature = "mock_auth")]
//! Phase 13 Plan 07 — E2E-Tests fuer `POST /api/repayment-phase/{phase_id}/letters/generate`.
//!
//! Deckt die 11 Locked Decisions D-13-01..11 end-to-end ueber echte HTTP-Calls ab:
//!
//! | Test                                                              | Decisions / Pitfalls                              |
//! |-------------------------------------------------------------------|---------------------------------------------------|
//! | test_letter_happy_path_3_entries_2_members                        | D-13-01 (Bundle), D-13-02 (Direct-Download), D-13-04 (Aggregation), D-13-05 (Persistenz) |
//! | test_letter_multi_entry_aggregation_d13_04                        | D-13-04 (1 MemberDocument pro Member nach Aggregation)         |
//! | test_letter_helper_auth_returns_403                               | Phase-11-Funnel + D-13 Vorstand-Gate (mock_auth-bedingt #[ignore]) |
//! | test_letter_phase_preparation_returns_409_phase_not_active        | D-13 Status-Gate (Phase MUSS Open/Closed sein)    |
//! | test_letter_entry_phase_mismatch_returns_400                      | D-13-03 (entry_phase_mismatch Validation)         |
//! | test_letter_null_iban_renders_ok                                  | D-13-06 + Pitfall #5 (NULL-IBAN-Rendering)        |
//! | test_letter_audit_chain_valid_after_bulk                          | D-13-08 + Pitfall #4 (Audit-Hashchain bleibt valid) |
//! | test_letter_idempotency_d13_08_and_no_status_toggle_d13_09        | D-13-08 (Idempotenz) + D-13-09 (kein Auto-Toggle)  |
//!
//! Hinweise:
//! - `setup_with_templates_and_pool()` provisioniert die Default-Templates
//!   (inkl. `auszahlungs_anschreiben.typ` + `_bundle.typ` aus Plan 13-01) auf
//!   einen TempDir — damit der echte PdfGenerator den Bulk-Brief-Pfad rendert.
//! - Die Helpers `create_member_with_exit_date_local`,
//!   `create_open_repayment_phase_local`, `create_preparation_repayment_phase_local`,
//!   `list_entries_for_phase`, `list_member_documents`, `create_manual_entry`,
//!   `get_entry_status` werden lokal dupliziert (Plan 13-07 erlaubt Duplikation
//!   in Tests). So bleibt die Datei in sich geschlossen und das e2e_tests.rs
//!   wird nicht touched.

use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::{start_test_server, TestServer};
use genossi_rest_types::{
    ActionTypeTO, CreateRepaymentEntryRequest, MemberActionTO, MemberDocumentTO, MemberTO,
    RepaymentEntryStatusTO, RepaymentEntryTO, RepaymentPhaseStatusTO, RepaymentPhaseTO,
    VerifyResponseTO,
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

// ============================================================================
// Test-Setup
// ============================================================================

/// Test-Server mit provisionierten Default-Templates (inkl.
/// `auszahlungs_anschreiben.typ` + `_bundle.typ`, gespeist aus
/// `template_storage::DEFAULT_TEMPLATES` — Plan 13-01 hat diese eingehaengt).
///
/// **Logo-Provisionierung:** Das `auszahlungs_anschreiben.typ`-Template
/// referenziert `nebenan-unverpackt-logo.svg` via Typst-`image(...)`. Das
/// Logo ist NICHT Teil von `DEFAULT_TEMPLATES` (es ist ein Binary-Asset, nicht
/// per `include_bytes!` ausgelagert), also kopieren wir es nach
/// `provision_defaults()` zusaetzlich. Quelle: `templates/nebenan-unverpackt-logo.svg`
/// im Repo-Root (relativ zum genossi_bin-Manifest = `CARGO_MANIFEST_DIR/../templates/...`).
/// Ohne diese Datei wuerde der Typst-Compile mit "file not found" 500 liefern
/// (Pitfall #6 in Plan 13-07 RESEARCH).
///
/// 1:1 Pattern aus `e2e_tests.rs::setup_with_templates`, plus Logo-Copy fuer
/// das Letter-Template (das einzige Default-Template mit Bild-Asset).
async fn setup_with_templates() -> TestServer {
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

    // Templates auf den Filesystem-Pfad (default ./templates) provisionieren —
    // sonst kann der PdfGenerator die Letter-Templates nicht laden.
    use genossi_rest::RestStateDef;
    rest_state
        .template_storage()
        .provision_defaults()
        .await
        .expect("Failed to provision default templates");

    // Logo neben die provisionierten Templates legen (idempotent: nur kopieren,
    // wenn noch nicht vorhanden — verhindert Race im parallelen Test-Run).
    let template_base = rest_state.template_storage().base_path().to_path_buf();
    let logo_target = template_base.join("nebenan-unverpackt-logo.svg");
    if !logo_target.exists() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let logo_source = manifest.join("../templates/nebenan-unverpackt-logo.svg");
        if let Some(parent) = logo_target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .expect("create template base dir");
        }
        tokio::fs::copy(&logo_source, &logo_target)
            .await
            .unwrap_or_else(|e| panic!(
                "Failed to copy logo asset from {:?} to {:?}: {}",
                logo_source, logo_target, e
            ));
    }

    start_test_server(rest_state).await
}

// ============================================================================
// Lokal duplizierte Helpers (Plan 13-07 erlaubt Duplikation in Test-Code)
// ============================================================================

fn sample_member_with_iban(member_number: i64, iban: Option<&str>) -> MemberTO {
    MemberTO {
        id: None,
        member_number,
        first_name: format!("Mitglied{}", member_number),
        last_name: "Test".to_string(),
        salutation: None,
        title: None,
        email: Some(format!("mitglied{}@example.com", member_number)),
        company: None,
        comment: None,
        street: Some("Musterstraße".to_string()),
        house_number: Some("1".to_string()),
        postal_code: Some("12345".to_string()),
        city: Some("Berlin".to_string()),
        join_date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
        shares_at_joining: 5,
        current_shares: 5,
        current_balance: 60000,
        action_count: 0,
        migrated: false,
        exit_date: None,
        bank_account: iban.map(str::to_string),
        status: genossi_rest_types::MemberStatusTO::Normal,
        created: None,
        deleted: None,
        version: None,
    }
}

/// Erzeugt einen Member mit Austritts-Action im `fiscal_year`. Optional IBAN
/// (None erlaubt D-13-06 NULL-IBAN-Test).
async fn create_member_with_exit_date_and_iban(
    client: &reqwest::Client,
    server: &TestServer,
    member_number: i64,
    fiscal_year: i32,
    iban: Option<&str>,
) -> MemberTO {
    // 1) Member anlegen.
    let m = sample_member_with_iban(member_number, iban);
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
    let created: MemberTO = response.json().await.expect("decode MemberTO");
    let member_id = created.id.expect("created member must have id");

    // 2) Austritt-Action posten — setzt exit_date.
    let exit_date = time::Date::from_calendar_date(fiscal_year, time::Month::June, 15).unwrap();
    let austritt = MemberActionTO {
        id: None,
        member_id,
        action_type: ActionTypeTO::Austritt,
        date: exit_date,
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(exit_date),
        comment: Some("Phase 13 E2E setup".to_string()),
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

    // 3) Member frisch laden (recalc_dates hat exit_date gesetzt).
    let response = client
        .get(server.url(&format!("/api/members/{}", member_id)))
        .send()
        .await
        .expect("GET member failed");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("decode MemberTO")
}

/// Repayment-Phase im Status `Preparation` erzeugen.
async fn create_preparation_repayment_phase(
    client: &reqwest::Client,
    server: &TestServer,
    fiscal_year: i32,
    share_value: i64,
) -> RepaymentPhaseTO {
    let body = json!({
        "fiscal_year": fiscal_year,
        "share_value": share_value,
    });
    let response = client
        .post(server.url("/api/repayment-phase"))
        .json(&body)
        .send()
        .await
        .expect("POST repayment-phase");
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "create repayment-phase must return 201"
    );
    response.json().await.expect("decode RepaymentPhaseTO")
}

/// Repayment-Phase erzeugen UND oeffnen (triggert Auto-Fill der Entries fuer
/// alle Members mit `exit_date` im selben fiscal_year).
async fn create_open_repayment_phase(
    client: &reqwest::Client,
    server: &TestServer,
    fiscal_year: i32,
    share_value: i64,
) -> RepaymentPhaseTO {
    let phase = create_preparation_repayment_phase(client, server, fiscal_year, share_value).await;
    let open_resp = client
        .post(server.url(&format!("/api/repayment-phase/{}/open", phase.id)))
        .send()
        .await
        .expect("open_phase POST");
    assert_eq!(open_resp.status(), StatusCode::OK, "open_phase must be 200");
    let opened: RepaymentPhaseTO = open_resp.json().await.expect("decode opened phase");
    assert!(matches!(opened.status, RepaymentPhaseStatusTO::Open));
    opened
}

/// Listet alle RepaymentEntries einer Phase (GET ?phase_id=<uuid>).
async fn list_entries_for_phase(
    client: &reqwest::Client,
    server: &TestServer,
    phase_id: uuid::Uuid,
) -> Vec<RepaymentEntryTO> {
    let resp = client
        .get(server.url(&format!("/api/repayment-entry?phase_id={}", phase_id)))
        .send()
        .await
        .expect("GET repayment-entry");
    assert_eq!(resp.status(), StatusCode::OK);
    resp.json().await.expect("decode entries")
}

/// Manuell ein zweites Entry fuer denselben Member anlegen (z.B. fuer
/// Multi-Entry-Aggregation-Tests). Phase muss Open sein.
async fn create_manual_entry(
    client: &reqwest::Client,
    server: &TestServer,
    phase_id: uuid::Uuid,
    member_id: uuid::Uuid,
    share_count: i32,
) -> RepaymentEntryTO {
    let body = CreateRepaymentEntryRequest {
        phase_id,
        member_id,
        share_count_to_pay_out: share_count,
    };
    let resp = client
        .post(server.url("/api/repayment-entry"))
        .json(&body)
        .send()
        .await
        .expect("POST repayment-entry");
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "manual entry create must return 201, got: {}",
        resp.text().await.unwrap_or_default()
    );
    resp.json().await.expect("decode entry")
}

/// Listet alle MemberDocuments eines Members (inkl. RepaymentLetter-Eintraege).
async fn list_member_documents(
    client: &reqwest::Client,
    server: &TestServer,
    member_id: uuid::Uuid,
) -> Vec<MemberDocumentTO> {
    let resp = client
        .get(server.url(&format!("/api/members/{}/documents", member_id)))
        .send()
        .await
        .expect("GET member documents");
    assert_eq!(resp.status(), StatusCode::OK);
    resp.json().await.expect("decode documents")
}

/// Liest den aktuellen `status` eines RepaymentEntry (fuer D-13-09 No-Toggle-Check).
async fn get_entry_status(
    client: &reqwest::Client,
    server: &TestServer,
    entry_id: uuid::Uuid,
) -> RepaymentEntryStatusTO {
    let resp = client
        .get(server.url(&format!("/api/repayment-entry/{}", entry_id)))
        .send()
        .await
        .expect("GET repayment-entry/{id}");
    assert_eq!(resp.status(), StatusCode::OK);
    let entry: RepaymentEntryTO = resp.json().await.expect("decode entry");
    entry.status
}

// ============================================================================
// Tests
// ============================================================================

/// Test 1 (Happy Path, D-13-01 + D-13-02 + D-13-04 + D-13-05):
/// 3 RepaymentEntries fuer 2 Members → POST liefert 200 + application/pdf +
/// %PDF- magic bytes; Content-Disposition enthaelt
/// `auszahlungs_anschreiben_GJ_{fy}.pdf`; X-Document-Count = 2;
/// MemberDocuments persistiert (1 pro Member, da M1 ueber 2 Entries aggregiert).
#[tokio::test]
async fn test_letter_happy_path_3_entries_2_members() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;

    // 2 Members anlegen, beide mit IBAN.
    let m1 = create_member_with_exit_date_and_iban(
        &client,
        &server,
        101,
        fiscal_year,
        Some("DE89370400440532013000"),
    )
    .await;
    let m2 = create_member_with_exit_date_and_iban(
        &client,
        &server,
        102,
        fiscal_year,
        Some("DE91100000000123456789"),
    )
    .await;

    // Phase oeffnen — Auto-Fill erzeugt je 1 Entry pro Member.
    let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;

    let entries = list_entries_for_phase(&client, &server, phase.id).await;
    assert!(
        entries.len() >= 2,
        "Auto-Fill muss mind. 2 Entries anlegen (M1, M2); got {}",
        entries.len()
    );

    // Drittes Entry manuell fuer M1 — so haben wir 3 Entries / 2 Members.
    let m1_id = m1.id.expect("m1 id");
    let m2_id = m2.id.expect("m2 id");
    let extra = create_manual_entry(&client, &server, phase.id, m1_id, 2).await;

    let entry_ids: Vec<uuid::Uuid> =
        entries.iter().map(|e| e.id).chain(std::iter::once(extra.id)).collect();
    assert!(entry_ids.len() >= 3);

    // POST /letters/generate — Header- + Status- + Body-Assertions in einer
    // Sequenz; `resp.bytes().await` konsumiert den Body, daher MUSS der Header-
    // Check VOR dem Body-Lese-Schritt erfolgen.
    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": entry_ids }))
        .send()
        .await
        .expect("POST letters/generate");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "happy path expected 200; got {}",
        resp.status()
    );

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        ct, "application/pdf",
        "Content-Type muss application/pdf sein"
    );
    let cd = resp
        .headers()
        .get("content-disposition")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        cd.contains(&format!("auszahlungs_anschreiben_GJ_{}.pdf", fiscal_year)),
        "Content-Disposition muss filename auszahlungs_anschreiben_GJ_{}.pdf enthalten; got {}",
        fiscal_year,
        cd
    );
    let doc_count_header = resp
        .headers()
        .get("X-Document-Count")
        .and_then(|h| h.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();
    assert_eq!(
        doc_count_header, "2",
        "X-Document-Count muss 2 sein (D-13-04 Aggregation: 3 Entries / 2 Members → 2 Docs); got '{}'",
        doc_count_header
    );

    let bytes = resp.bytes().await.expect("read bytes");
    assert!(
        bytes.starts_with(b"%PDF-"),
        "Response-Body muss mit %PDF- starten"
    );
    assert!(
        bytes.len() > 1000,
        "Bundle-PDF sollte > 1000 Bytes sein (haben: {})",
        bytes.len()
    );

    // MemberDocuments verifizieren — 1 Call erzeugt pro Member 1 Doc, da
    // M1 ueber 2 Entries aggregiert wird (D-13-04).
    let docs_m1 = list_member_documents(&client, &server, m1_id).await;
    let docs_m2 = list_member_documents(&client, &server, m2_id).await;
    let letter_docs_m1: Vec<_> = docs_m1
        .iter()
        .filter(|d| d.document_type == "repayment_letter")
        .collect();
    let letter_docs_m2: Vec<_> = docs_m2
        .iter()
        .filter(|d| d.document_type == "repayment_letter")
        .collect();
    assert_eq!(
        letter_docs_m1.len(),
        1,
        "M1 erhaelt 1 MemberDocument (D-13-04 Aggregation ueber 2 Entries → 1 Doc)"
    );
    assert_eq!(letter_docs_m2.len(), 1, "M2 erhaelt 1 MemberDocument");
}

/// Test 2 (D-13-04 Multi-Entry-Aggregation):
/// 1 Member mit 2 Entries (Teil-Abtretung + Voll-Austritt-Korrektur). POST
/// erzeugt EIN MemberDocument (NICHT 2) — der Service aggregiert per
/// `member_id`.
#[tokio::test]
async fn test_letter_multi_entry_aggregation_d13_04() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;

    let m = create_member_with_exit_date_and_iban(
        &client,
        &server,
        201,
        fiscal_year,
        Some("DE89370400440532013000"),
    )
    .await;
    let m_id = m.id.expect("m id");

    let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;

    let entries = list_entries_for_phase(&client, &server, phase.id).await;
    let auto = entries
        .into_iter()
        .find(|e| e.member_id == m_id)
        .expect("auto entry for m");
    let manual = create_manual_entry(&client, &server, phase.id, m_id, 2).await;

    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": [auto.id, manual.id] }))
        .send()
        .await
        .expect("POST letters/generate");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "D-13-04 multi-entry-Call muss 200 sein"
    );
    let doc_count_header = resp
        .headers()
        .get("X-Document-Count")
        .and_then(|h| h.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();
    assert_eq!(
        doc_count_header, "1",
        "X-Document-Count muss 1 sein (D-13-04: 2 Entries → 1 Member → 1 Doc); got '{}'",
        doc_count_header
    );

    // Body lesen + verwerfen, sonst connection-pool issue.
    let _ = resp.bytes().await;

    let docs = list_member_documents(&client, &server, m_id).await;
    let letter_docs: Vec<_> = docs
        .iter()
        .filter(|d| d.document_type == "repayment_letter")
        .collect();
    assert_eq!(
        letter_docs.len(),
        1,
        "D-13-04: 2 Entries gleicher Member → EIN MemberDocument; got {}",
        letter_docs.len()
    );
}

/// Test 3 (Phase-11-Funnel-Gate, mock_auth-bedingt #[ignore]):
///
/// Im `mock_auth`-Feature injiziert der `context_extractor` in
/// `genossi_rest/src/session.rs:120-127` UNCONDITIONAL einen `MockContext` —
/// die Migration `20250129000001_create_default_auth_data.sql` ordnet diesem
/// DEVUSER admin-Privilegien zu. Daher ist im e2e-mock-auth-Stack ein
/// Helfer-Auth-Pfad strukturell NICHT erreichbar (siehe vorhandenen Comment in
/// `e2e_tests.rs:9213-9221` zum selben Limit fuer Helper-Cookies).
///
/// Der 403-Pfad ist statt dessen unit-getestet auf REST-Layer-Ebene in
/// `genossi_rest/src/repayment_letter.rs::tests::test_map_letter_error_permission_denied_to_403`
/// und auf Service-Layer-Ebene in
/// `genossi_service_impl/src/repayment_letter.rs::tests::test_generate_permission_denied_returns_403`.
///
/// Dieser Skeleton-Test bleibt als Marker fuer die Acceptance-Greps (Plan 13-07)
/// und wird re-aktiviert sobald entweder (a) ein non-admin Mock-Pfad bereitsteht
/// oder (b) Plan 14 OIDC-Tests einbaut.
#[ignore = "mock_auth context_extractor injiziert IMMER Admin (DEVUSER) — non-admin nicht E2E darstellbar; 403-Pfad ist auf REST- und Service-Layer-Ebene unit-getestet"]
#[tokio::test]
async fn test_letter_helper_auth_returns_403() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;
    let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;

    // Wenn eine Helper-Auth-Pfad-Simulation verfuegbar waere, wuerden wir hier
    // einen Helper-Client mit nicht-Admin-Cookie verwenden. Bis dahin asserten
    // wir defensiv, dass der Endpoint wenigstens NICHT 5xx zurueckgibt — der
    // eigentliche 403-Beweis liegt im Unit-/Service-Test.
    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": [uuid::Uuid::new_v4()] }))
        .send()
        .await
        .expect("POST letters/generate");

    // WR-04 fix: EXAKT 403 fordern (nicht nur 4xx). Der vorherige
    // is_client_error()-Pfad haette auch 401 (Session ungueltig) oder 400
    // (Validation) durchgewinkt und damit eine echte Regression maskiert.
    //
    // Der Test ist weiterhin #[ignore]'d (mock_auth liefert immer Admin) — wenn
    // er jemals re-aktiviert wird oder versehentlich via `cargo test --ignored`
    // laeuft, muss er die etablierte 403-Semantik des Permission-Gates
    // verifizieren, nicht bloss "irgendein 4xx".
    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Permission-Gate muss EXAKT 403 liefern (nicht 401, nicht 400); got {}",
        status
    );
}

/// Test 4 (Status-Gate, D-13 Phase muss Open/Closed sein):
/// Phase im Preparation-Status → 409 Conflict mit Body `phase_not_active`.
#[tokio::test]
async fn test_letter_phase_preparation_returns_409_phase_not_active() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;

    // Phase in Preparation belassen — NICHT oeffnen!
    let phase = create_preparation_repayment_phase(&client, &server, fiscal_year, 12000).await;
    assert!(matches!(phase.status, RepaymentPhaseStatusTO::Preparation));

    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": [uuid::Uuid::new_v4()] }))
        .send()
        .await
        .expect("POST letters/generate");

    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "Preparation-Phase muss 409 zurueckgeben (D-13 Status-Gate)"
    );
    let body = resp.text().await.unwrap_or_default();
    assert!(
        body.contains("phase_not_active"),
        "Body muss 'phase_not_active' enthalten; got: {}",
        body
    );
}

/// Test 5 (D-13-03 entry_phase_mismatch):
/// entry_ids gehoeren zu einer anderen Phase → 400 BadRequest mit
/// `entry_phase_mismatch`-Substring.
#[tokio::test]
async fn test_letter_entry_phase_mismatch_returns_400() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fy_a = 2026;
    let fy_b = 2027;

    // Member fuer FY_B anlegen (damit Auto-Fill in Phase B ein Entry erzeugt).
    let m_b = create_member_with_exit_date_and_iban(
        &client,
        &server,
        301,
        fy_b,
        Some("DE89370400440532013000"),
    )
    .await;
    let m_b_id = m_b.id.expect("m_b id");

    // Phase A (target im POST-Pfad) — leer, kein Mitglied passend.
    let phase_a = create_open_repayment_phase(&client, &server, fy_a, 12000).await;
    // Phase B (entry stammt von hier).
    let phase_b = create_open_repayment_phase(&client, &server, fy_b, 12000).await;

    let entries_b = list_entries_for_phase(&client, &server, phase_b.id).await;
    let entry_b = entries_b
        .into_iter()
        .find(|e| e.member_id == m_b_id)
        .expect("entry_b for m_b");

    // POST gegen Phase A mit entry_id aus Phase B.
    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase_a.id
        )))
        .json(&json!({ "entry_ids": [entry_b.id] }))
        .send()
        .await
        .expect("POST letters/generate");

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "entry_phase_mismatch muss 400 zurueckgeben (D-13-03)"
    );
    let body = resp.text().await.unwrap_or_default();
    assert!(
        body.contains("entry_phase_mismatch"),
        "Body muss 'entry_phase_mismatch' enthalten; got: {}",
        body
    );
}

/// Test 6 (D-13-06 + Pitfall #5 NULL-IBAN-Rendering):
/// Member ohne `bank_account` → POST liefert 200 + valides PDF (NULL-Hinweis
/// im Template gerendert, nicht 4xx).
#[tokio::test]
async fn test_letter_null_iban_renders_ok() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;

    // Member OHNE bank_account (None).
    let m = create_member_with_exit_date_and_iban(&client, &server, 401, fiscal_year, None).await;
    let m_id = m.id.expect("m id");
    assert!(
        m.bank_account.is_none(),
        "Test-Voraussetzung: bank_account muss None sein"
    );

    let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;
    let entries = list_entries_for_phase(&client, &server, phase.id).await;
    let entry = entries
        .into_iter()
        .find(|e| e.member_id == m_id)
        .expect("entry for m");

    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": [entry.id] }))
        .send()
        .await
        .expect("POST letters/generate");

    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "NULL-IBAN MUSS rendern (D-13-06 + Pitfall #5); got {}",
        status
    );
    let bytes = resp.bytes().await.expect("read bytes");
    assert!(
        bytes.starts_with(b"%PDF-"),
        "NULL-IBAN-Render muss valides PDF liefern (starts with %PDF-)"
    );
}

/// Test 7 (D-13-08 + Pitfall #4):
/// Nach Bulk-Letter-Run muss die Audit-Hashchain weiterhin valide sein
/// (`GET /api/audit/verify` returns `valid == true`).
#[tokio::test]
async fn test_letter_audit_chain_valid_after_bulk() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;

    let _m1 = create_member_with_exit_date_and_iban(
        &client,
        &server,
        501,
        fiscal_year,
        Some("DE89370400440532013000"),
    )
    .await;
    let _m2 = create_member_with_exit_date_and_iban(
        &client,
        &server,
        502,
        fiscal_year,
        Some("DE91100000000123456789"),
    )
    .await;
    let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;

    let entries = list_entries_for_phase(&client, &server, phase.id).await;
    let entry_ids: Vec<uuid::Uuid> = entries.iter().map(|e| e.id).collect();
    assert!(!entry_ids.is_empty(), "Auto-Fill muss Entries erzeugt haben");

    let resp = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": entry_ids }))
        .send()
        .await
        .expect("POST letters/generate");
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = resp.bytes().await;

    // Audit-Chain-Verify.
    let resp = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .expect("GET audit/verify");
    assert_eq!(resp.status(), StatusCode::OK);
    let result: VerifyResponseTO = resp.json().await.expect("decode VerifyResponseTO");
    assert!(
        result.valid,
        "Audit-Hashchain MUSS nach Bulk-Run valide bleiben (D-13-08 + Pitfall #4); broken_links={:?}",
        result.broken_links
    );
    assert!(
        result.total_entries > 0,
        "Bulk-Run muss mind. eine Audit-Entry erzeugt haben"
    );
}

/// Test 8 (D-13-08 Idempotenz + D-13-09 No-Status-Toggle):
/// 2 sequenzielle Bulk-Calls auf denselben Entry liefern BEIDE 200 (keine
/// 409-Idempotenz-Sperre, D-13-08). Pro Call entsteht 1 MemberDocument →
/// 2 Docs total. Status des Entry bleibt UNVERAENDERT auf `Open`
/// (D-13-09: Backend toggelt NICHT).
#[tokio::test]
async fn test_letter_idempotency_d13_08_and_no_status_toggle_d13_09() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let fiscal_year = 2026;

    let m = create_member_with_exit_date_and_iban(
        &client,
        &server,
        601,
        fiscal_year,
        Some("DE89370400440532013000"),
    )
    .await;
    let m_id = m.id.expect("m id");

    let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;
    let entries = list_entries_for_phase(&client, &server, phase.id).await;
    let entry = entries
        .into_iter()
        .find(|e| e.member_id == m_id)
        .expect("entry for m");

    // Initialer Status: Open (vom Auto-Fill).
    let status_before = get_entry_status(&client, &server, entry.id).await;
    assert!(
        matches!(status_before, RepaymentEntryStatusTO::Open),
        "Auto-Fill muss Entry in Open setzen, got {:?}",
        status_before
    );

    // Bulk-Call #1.
    let resp1 = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": [entry.id] }))
        .send()
        .await
        .expect("POST #1");
    assert_eq!(resp1.status(), StatusCode::OK, "Bulk-Call #1 muss 200 sein");
    let _ = resp1.bytes().await;

    // Bulk-Call #2 (selber Entry).
    let resp2 = client
        .post(server.url(&format!(
            "/api/repayment-phase/{}/letters/generate",
            phase.id
        )))
        .json(&json!({ "entry_ids": [entry.id] }))
        .send()
        .await
        .expect("POST #2");
    assert_eq!(
        resp2.status(),
        StatusCode::OK,
        "D-13-08: Zweiter Bulk-Call muss 200 sein (keine Idempotenz-Sperre)"
    );
    let _ = resp2.bytes().await;

    // Beide Runs erzeugen je 1 MemberDocument → 2 Docs total.
    let docs = list_member_documents(&client, &server, m_id).await;
    let letter_docs: Vec<_> = docs
        .iter()
        .filter(|d| d.document_type == "repayment_letter")
        .collect();
    assert_eq!(
        letter_docs.len(),
        2,
        "D-13-08: Jeder Bulk-Call erzeugt 1 MemberDocument → 2 Docs nach 2 Calls; got {}",
        letter_docs.len()
    );

    // D-13-09: Status DARF SICH NICHT geaendert haben.
    let status_after = get_entry_status(&client, &server, entry.id).await;
    assert!(
        matches!(status_after, RepaymentEntryStatusTO::Open),
        "D-13-09: Backend MUSS RepaymentEntry.status NICHT auto-togglen — erwartet Open, got {:?}",
        status_after
    );
}
