//! Quick 260614-b1t: single source of truth for per-recipient template rendering.
//!
//! Extracted verbatim from the inline render block that used to live in
//! `worker.rs::start_mail_worker` (~378-577). Both the live mail worker AND the
//! startup backfill (`crate::backfill::run_rendered_backfill`) call this one
//! function, so the rendered subject/body is computed identically in both paths
//! (DRY — must-have truth "Die Render-Logik existiert nur einmal").
//!
//! Behavior contract (byte-for-byte identical to the pre-refactor worker path):
//! - `recipient.member_id == None` → plain passthrough `(job.subject, job.body)`.
//! - `recipient.member_id == Some(id)` → load member via MemberResolver:
//!     - member not found / resolver error → `Err(RenderFailure)`.
//!     - `job.repayment_phase_id == Some(phase_id)` → open a read tx, look up the
//!       phase + entries, derive `share_value_str` from `phase.share_value`, and:
//!         - aggregate Ok → merge the repayment context into the member context.
//!         - aggregate Err(EntityNotFound) → leave context unmerged (strict-env
//!           render fails downstream if the template references repayment vars —
//!           the intended D-05 behavior).
//!         - aggregate Err(other) → `Err(RenderFailure)`.
//!     - render subject + body with the (possibly merged) context.

use genossi_service::repayment_context::RepaymentContextResolver;
use genossi_service::ServiceError;

use crate::dao::{MailJob, MailRecipient};
use crate::template::{
    member_to_template_context, merge_repayment_context, render_html_template, render_template,
    MemberResolver,
};

/// Failure cause from `resolve_rendered_content`. The caller (worker) maps the
/// `message` onto `mark_recipient_failed`; the backfill logs it and skips the row.
#[derive(Debug, Clone)]
pub struct RenderFailure {
    pub message: String,
}

impl RenderFailure {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Phase 23 D-08/D-09: per-recipient render result. `body_html` is `Some(...)`
/// iff the job carries a `body_html` template AND the recipient resolves to a
/// member context; otherwise `None` (never `Some("")`, per Pitfall 4).
///
/// Plan 04 wires this into the worker: `body_html` is persisted verbatim in
/// `mail_recipients.rendered_html_body`, and `body_html.as_deref()` is passed
/// into `build_message` for the `multipart/alternative` branch.
#[derive(Debug, Clone)]
pub struct RenderedContent {
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
}

/// Render the subject + body + optional HTML body a single recipient should
/// receive.
///
/// Returns `Ok(RenderedContent { subject, body, body_html })` on success —
/// `body_html` is `Some(rendered)` iff `job.body_html.is_some()` AND the
/// recipient has a member context; otherwise `None` (D-09, Pitfall 4).
/// Returns `Err(RenderFailure)` on member-resolution / repayment-aggregation /
/// template-render errors.
#[allow(clippy::too_many_arguments)]
pub async fn resolve_rendered_content<M, RE, RP, TX, RCR>(
    recipient: &MailRecipient,
    job: &MailJob,
    member_resolver: &M,
    repayment_entry_dao: &RE,
    repayment_phase_dao: &RP,
    transaction_dao: &TX,
    repayment_context_resolver: &RCR,
) -> Result<RenderedContent, RenderFailure>
where
    M: MemberResolver,
    RE: genossi_dao::repayment_entry::RepaymentEntryDao + Send + Sync,
    RP: genossi_dao::repayment_phase::RepaymentPhaseDao<Transaction = RE::Transaction>
        + Send
        + Sync,
    TX: genossi_dao::TransactionDao<Transaction = RE::Transaction> + Send + Sync,
    RCR: RepaymentContextResolver<Transaction = RE::Transaction> + Send + Sync,
{
    let Some(member_id) = recipient.member_id else {
        // No member_id — plain text passthrough (no template interpolation).
        // body_html stays None because we never render into an empty context
        // (D-09: no member context ⇒ no HTML render).
        return Ok(RenderedContent {
            subject: job.subject.to_string(),
            body: job.body.to_string(),
            body_html: None,
        });
    };

    let member = match member_resolver.find_member_by_id(member_id).await {
        Ok(Some(member)) => member,
        Ok(None) => {
            return Err(RenderFailure::new(format!(
                "Member {} not found for template rendering",
                member_id
            )));
        }
        Err(e) => {
            return Err(RenderFailure::new(format!(
                "Failed to load member for template rendering: {:?}",
                e
            )));
        }
    };

    let mut ctx = member_to_template_context(&member);

    if let Some(phase_id) = job.repayment_phase_id {
        let agg_tx = transaction_dao.transaction().await.map_err(|e| {
            RenderFailure::new(format!(
                "Worker: cannot open tx for repayment context: {:?}",
                e
            ))
        })?;

        let phase_opt = match repayment_phase_dao
            .find_by_id(phase_id, agg_tx.clone())
            .await
        {
            Ok(p) => p,
            Err(e) => {
                return Err(RenderFailure::new(format!(
                    "Worker: repayment_phase lookup failed: {:?}",
                    e
                )));
            }
        };

        if let Some(phase) = phase_opt {
            let entries = match repayment_entry_dao
                .find_by_phase_id(phase_id, agg_tx.clone())
                .await
            {
                Ok(es) => es,
                Err(e) => {
                    return Err(RenderFailure::new(format!(
                        "Worker: repayment_entry lookup failed: {:?}",
                        e
                    )));
                }
            };

            // Phase-wide Anteilswert (German euro string) derived locally —
            // RepaymentContext does not carry it (Quick 260602-r2i).
            let share_value_str =
                format!("{},{:02}", phase.share_value / 100, phase.share_value % 100);

            match repayment_context_resolver.aggregate(&phase, &entries, member.id) {
                Ok(rc) => {
                    ctx = merge_repayment_context(
                        ctx,
                        &rc.payout_amount,
                        rc.share_count,
                        &share_value_str,
                        rc.fiscal_year,
                    );
                }
                Err(ServiceError::EntityNotFound(_)) => {
                    // D-05 edge-case: no Open/Contacted entries — leave context
                    // unmerged. Strict-env render fails on referenced repayment
                    // vars (intended), preserving pre-refactor behavior.
                }
                Err(e) => {
                    return Err(RenderFailure::new(format!(
                        "Worker: repayment_context aggregate failed: {:?}",
                        e
                    )));
                }
            }
        }

        // Release the read tx — best-effort, errors ignored (read-only).
        let _ = transaction_dao.commit(agg_tx).await;
    }

    let subject = render_template(&job.subject, &ctx).map_err(|e| {
        RenderFailure::new(format!("Template render error (subject): {}", e.message))
    })?;
    let body = render_template(&job.body, &ctx)
        .map_err(|e| RenderFailure::new(format!("Template render error (body): {}", e.message)))?;

    // D-09 / Pitfall 4: only render HTML when the job actually carries a
    // body_html source — otherwise leave body_html as None (never Some("")).
    let body_html = match job.body_html.as_deref() {
        Some(html_src) => Some(
            render_html_template(html_src, &ctx)
                .map_err(|e| RenderFailure::new(format!("HTML render error: {}", e.message)))?,
        ),
        None => None,
    };

    // Quick 260718-html-to-plain-derivation: wenn wir einen HTML-Body haben, ist
    // der Frontend-supplied `body` bloß `element.innerText()` — verliert Bullets,
    // Nummerierung und Titel-Unterstreichung bei WYSIWYG-Formatierung. Wir leiten
    // deshalb den Plain-Text im Render-Layer aus dem gerenderten HTML ab. Der
    // Send-Layer (build_message) bleibt HTML-02-treu: `body` ist raw plain, nur
    // die Quelle ist jetzt html2text statt inner_text. body_html: None → alter
    // Pfad, Frontend-`body` unverändert (Backward-Compat mit v1.4-Plaintext-Mails).
    let body = match body_html.as_deref() {
        Some(html) => plain_from_html(html),
        None => body,
    };

    Ok(RenderedContent {
        subject,
        body,
        body_html,
    })
}

/// Quick 260718-html-to-plain-derivation: rendert HTML-Body zu strukturiertem
/// Plain-Text (Listen mit Bullets/Nummern, Überschriften mit Unterstreichung,
/// Blockquotes mit `>`-Prefix). Wird für den Text-Teil der `multipart/alternative`
/// verwendet, wenn body_html Some ist. 78-Zeichen-Breite = klassische Mail-Grenze
/// (RFC 2822-freundlich).
///
/// Phase 27 (IMG-06, RESEARCH Anti-Pattern): `<img>`-Tags werden VOR der
/// html2text-Ableitung entfernt, damit weder `cid:`-URLs noch `img`-Reste in
/// den Text-Teil der `multipart/alternative` lecken. html2text droppt `<img>`
/// zwar meist von selbst, aber das explizite Strippen ist die belt-and-suspenders
/// Garantie (Threat T-27-13).
pub(crate) fn plain_from_html(html: &str) -> String {
    let stripped = strip_img_tags(html);
    html2text::from_read(stripped.as_bytes(), 78)
        .unwrap_or_else(|_| String::new())
        .trim_end()
        .to_string()
}

/// Phase 27 (IMG-06): eine im Send-Pfad referenzierte Bild-Asset-Referenz.
/// De-dupliziert pro distinktem Asset-ID — ein `multipart/related`-Part bedient
/// N `<img>`-Vorkommen desselben Assets (Pitfall 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetRef {
    pub asset_id: uuid::Uuid,
    /// Der BARE Content-ID-String (ohne spitze Klammern), z. B. `asset-1@genossi`.
    /// Exakt derselbe String wird in `<img src="cid:...">` UND in
    /// `Attachment::new_inline(...)` verwendet (Pitfall 6, CID/Content-ID-Match).
    pub cid: String,
}

/// Entfernt alle `<img ...>`-Tags aus dem HTML (bounded, ohne Re-Serialisierung).
/// Wird für die Plain-Text-Ableitung genutzt, damit kein Bild-Rest im Text-Teil
/// landet.
fn strip_img_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < html.len() {
        if html[i..].starts_with("<img") {
            // Finde das schließende '>' des img-Tags. Bilder in sanitized HTML
            // sind void-Elemente ohne verschachtelte '>' in Attributwerten
            // (data-genossi-asset-id ist ein UUID, src ist gestript).
            if let Some(rel_end) = html[i..].find('>') {
                i += rel_end + 1;
                continue;
            } else {
                // Kein schließendes '>' — Rest wörtlich übernehmen.
                out.push_str(&html[i..]);
                break;
            }
        }
        // Ein UTF-8-sicheres Zeichen übernehmen.
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Phase 27 (IMG-06, RESEARCH Open Question 3): reine Funktion, die sanitized
/// HTML mit `<img data-genossi-asset-id="X">` in `<img src="cid:asset-N@genossi">`
/// transformiert und die referenzierten Asset-IDs zurückgibt.
///
/// - Die HTML ist bereits von 27-02 sanitized, sodass auf `<img>` NUR
///   `data-genossi-asset-id` steht — ein gezielter, begrenzter Attribut-Rewrite
///   ist sicher (RESEARCH Don't-Hand-Roll); es wird NICHT voll re-serialisiert.
/// - Pro DISTINKTEM Asset-ID wird eine fortlaufende Nummer vergeben (asset-1,
///   asset-2, …), sodass CIDs kurz und eindeutig sind (Pitfall 6).
/// - Malformed IDs (keine gültige UUID) werden unangetastet gelassen (kein Panic).
/// - Rückgabe: rewritten HTML + de-duplizierter `Vec<AssetRef>` (ein Eintrag pro
///   distinktem Asset-ID). Kein `<img data-genossi-asset-id>` ⇒ HTML unverändert
///   + leerer Vec (Backward-Compat-Signal für den byte-identischen No-Image-Pfad).
pub(crate) fn rewrite_img_cids(html: &str) -> (String, Vec<AssetRef>) {
    let mut out = String::with_capacity(html.len());
    let mut refs: Vec<AssetRef> = Vec::new();
    // Mappt Asset-ID → zugewiesene fortlaufende Nummer (für De-Dup).
    let mut id_to_num: std::collections::HashMap<uuid::Uuid, usize> =
        std::collections::HashMap::new();
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < html.len() {
        if html[i..].starts_with("<img") {
            // Ganzen img-Tag isolieren.
            let Some(rel_end) = html[i..].find('>') else {
                // Kein schließendes '>' — Rest wörtlich übernehmen.
                out.push_str(&html[i..]);
                break;
            };
            let tag = &html[i..i + rel_end + 1];

            match extract_asset_id(tag) {
                Some(asset_id) => {
                    let num = match id_to_num.get(&asset_id) {
                        Some(n) => *n,
                        None => {
                            let n = id_to_num.len() + 1;
                            id_to_num.insert(asset_id, n);
                            refs.push(AssetRef {
                                asset_id,
                                cid: format!("asset-{}@genossi", n),
                            });
                            n
                        }
                    };
                    out.push_str(&format!(r#"<img src="cid:asset-{}@genossi">"#, num));
                }
                None => {
                    // Malformed / keine Asset-ID — Tag unangetastet übernehmen.
                    out.push_str(tag);
                }
            }
            i += rel_end + 1;
            continue;
        }
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }

    (out, refs)
}

/// Liest den `data-genossi-asset-id="{uuid}"`-Wert aus einem `<img>`-Tag und
/// parst ihn als UUID. `None`, wenn das Attribut fehlt oder der Wert keine
/// gültige UUID ist.
fn extract_asset_id(tag: &str) -> Option<uuid::Uuid> {
    const ATTR: &str = "data-genossi-asset-id";
    let attr_pos = tag.find(ATTR)?;
    let after = &tag[attr_pos + ATTR.len()..];
    // Erwartetes Muster: [=]["]{uuid}["]. Whitespace um '=' tolerieren.
    let after = after.trim_start();
    let after = after.strip_prefix('=')?;
    let after = after.trim_start();
    // Wert ist quoted (single oder double).
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after[1..];
    let end = rest.find(quote)?;
    let value = &rest[..end];
    uuid::Uuid::parse_str(value).ok()
}

/// UTF-8 Byte-Länge eines Zeichens anhand des Leadbytes.
fn utf8_char_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead < 0xE0 {
        2
    } else if lead < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::MockMemberResolver;
    use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
    use genossi_dao::repayment_entry::{MockRepaymentEntryDao, RepaymentEntryEntity};
    use genossi_dao::repayment_phase::{MockRepaymentPhaseDao, RepaymentPhaseEntity};
    use genossi_dao::{MockTransaction, MockTransactionDao};
    use genossi_service::repayment_context::{MockRepaymentContextResolver, RepaymentContext};
    use std::sync::Arc;
    use uuid::Uuid;

    fn sample_datetime() -> time::PrimitiveDateTime {
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        )
    }

    fn make_member() -> MemberEntity {
        let date = time::Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberEntity {
            id: Uuid::new_v4(),
            member_number: 42,
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mustermann"),
            salutation: Some(Salutation::Herr),
            title: Some(Arc::from("Dr.")),
            email: Some(Arc::from("max@example.com")),
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 3,
            current_balance: 15000,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: MemberStatus::Normal,
            account_holder: None,
            postal_status: genossi_dao::member::PostalStatus::Erreichbar,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn make_recipient(member_id: Option<Uuid>) -> MailRecipient {
        MailRecipient {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            mail_job_id: Uuid::new_v4(),
            to_address: Arc::from("max@example.com"),
            member_id,
            application_id: None,
            status: Arc::from("pending"),
            error: None,
            sent_at: None,
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_html_body: None,
            rendered_reconstructed: false,
        }
    }

    fn make_job(subject: &str, body: &str, repayment_phase_id: Option<Uuid>) -> MailJob {
        MailJob {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from(subject),
            body: Arc::from(body),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id,
            attach_repayment_letter: false,
            body_html: None,
        }
    }

    fn make_phase(fiscal_year: i32, share_value: i64) -> RepaymentPhaseEntity {
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year,
            share_value,
            status: genossi_dao::repayment_phase::RepaymentPhaseStatus::Open,
            opened_at: None,
            closed_at: None,
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_member_only() {
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let job = make_job("Hallo {{ first_name }}", "Lieber {{ last_name }}", None);

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let rendered = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(rendered.subject, "Hallo Max");
        assert_eq!(rendered.body, "Lieber Mustermann");
        assert!(
            rendered.body_html.is_none(),
            "body_html must be None when job.body_html is None (D-09)"
        );
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_repayment_merge() {
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let phase_id = Uuid::new_v4();
        let job = make_job(
            "Auszahlung {{ payout_amount }}",
            "{{ share_count }} Anteile a {{ share_value }} EUR GJ {{ fiscal_year }}",
            Some(phase_id),
        );

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        let mut tx_dao = MockTransactionDao::new();
        tx_dao.expect_transaction().returning(|| {
            // The render path clones the tx for each DAO call; supply a tx whose
            // clone() yields fresh clonable mocks.
            fn clonable_tx() -> MockTransaction {
                let mut tx = MockTransaction::new();
                tx.expect_clone().returning(clonable_tx);
                tx
            }
            Ok(clonable_tx())
        });
        tx_dao.expect_commit().returning(|_| Ok(()));

        let mut phase_dao = MockRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(make_phase(2026, 2000))));

        let mut entry_dao = MockRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(|_, _| Ok(Vec::<RepaymentEntryEntity>::new().into()));

        let mut rcr = MockRepaymentContextResolver::new();
        rcr.expect_aggregate().returning(|_, _, _| {
            Ok(RepaymentContext {
                share_count: 3,
                payout_amount: "60,00".to_string(),
                fiscal_year: 2026,
            })
        });

        let rendered = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(rendered.subject, "Auszahlung 60,00");
        // share_value derived from phase.share_value=2000 → "20,00".
        assert_eq!(rendered.body, "3 Anteile a 20,00 EUR GJ 2026");
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_plain_passthrough_no_member() {
        let recipient = make_recipient(None);
        let job = make_job("Plain Subject", "Plain Body", None);

        let resolver = MockMemberResolver::new();
        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let rendered = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(rendered.subject, "Plain Subject");
        assert_eq!(rendered.body, "Plain Body");
        assert!(
            rendered.body_html.is_none(),
            "plain passthrough must yield body_html=None (D-09)"
        );
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_missing_member_returns_err() {
        let recipient = make_recipient(Some(Uuid::new_v4()));
        let job = make_job("Hallo {{ first_name }}", "Body", None);

        let mut resolver = MockMemberResolver::new();
        resolver.expect_find_member_by_id().returning(|_| Ok(None));

        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let result = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not found"));
    }

    // ============================================================
    // Phase 23 (HTML-04, D-08/D-09): body_html wiring — Some when
    // job.body_html is Some, None otherwise (Pitfall 4).
    // ============================================================

    #[tokio::test]
    async fn resolve_rendered_content_renders_html_body() {
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let mut job = make_job("Hi", "Text body", None);
        job.body_html = Some(Arc::from("<p>Hallo {{ first_name }}</p>"));

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let rendered = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(rendered.body_html.as_deref(), Some("<p>Hallo Max</p>"));
        // Quick 260718-html-to-plain-derivation: body wird jetzt aus dem
        // gerenderten HTML abgeleitet (statt Frontend-supplied "Text body"),
        // damit Plain-Text-Empfänger strukturierten Fallback sehen.
        assert_eq!(rendered.body, "Hallo Max");
    }

    #[tokio::test]
    async fn resolve_rendered_content_body_html_none_when_job_body_html_none() {
        // D-09 wire: job.body_html = None ⇒ rendered.body_html = None,
        // never Some(""). Pitfall 4 pin.
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let job = make_job("Hi", "Text body", None);
        assert!(job.body_html.is_none(), "precondition: job.body_html None");

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let rendered = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert!(
            rendered.body_html.is_none(),
            "body_html must be None (D-09), got: {:?}",
            rendered.body_html
        );
        // Quick 260718-html-to-plain-derivation: body_html=None ⇒ Frontend-supplied
        // body unverändert (Backward-Compat mit v1.4-Plaintext-Mails, HTML-02-treu).
        assert_eq!(rendered.body, "Text body");
    }

    // ============================================================
    // Quick 260718-html-to-plain-derivation: Plain-Text-Alternative
    // aus gerendertem HTML ableiten, damit Empfänger mit reinem Text-
    // Client (Terminal, Screen-Reader, HTML-off) strukturierte Listen,
    // Überschriften und Blockquotes sehen — nicht nur `inner_text()`.
    // ============================================================

    #[test]
    fn plain_from_html_unordered_list_has_bullets() {
        let out = plain_from_html("<ul><li>Apfel</li><li>Birne</li></ul>");
        assert!(
            out.contains("* Apfel") && out.contains("* Birne"),
            "expected `* Apfel` and `* Birne` in output, got:\n{out}"
        );
    }

    #[test]
    fn plain_from_html_ordered_list_is_numbered() {
        let out = plain_from_html("<ol><li>Eins</li><li>Zwei</li></ol>");
        assert!(
            out.contains("1. Eins") && out.contains("2. Zwei"),
            "expected `1. Eins` and `2. Zwei` in output, got:\n{out}"
        );
    }

    #[test]
    fn plain_from_html_headings_are_marked() {
        // html2text 0.17 default: `# H1`, `## H2`, `### H3` (markdown-style).
        // Assertion tolerant: only require the text plus at least one leading `#`.
        let out = plain_from_html("<h1>Titel1</h1><h2>Titel2</h2><h3>Titel3</h3>");
        assert!(
            out.contains("Titel1") && out.contains("Titel2") && out.contains("Titel3"),
            "expected all three headings in output, got:\n{out}"
        );
        assert!(
            out.contains("# Titel1") || out.contains("Titel1\n===") || out.contains("TITEL1"),
            "expected h1 marker (# or === or upper), got:\n{out}"
        );
    }

    #[test]
    fn plain_from_html_blockquote_prefixed() {
        let out = plain_from_html("<blockquote>Zitat hier</blockquote>");
        assert!(
            out.contains("> Zitat hier"),
            "expected `> Zitat hier` in output, got:\n{out}"
        );
    }

    #[test]
    fn plain_from_html_empty_input_is_empty() {
        assert_eq!(plain_from_html(""), "");
    }

    #[test]
    fn plain_from_html_bold_becomes_markdown_stars() {
        let out = plain_from_html("<p>Hallo <b>Welt</b>!</p>");
        // Belt-and-suspenders — accept either **Welt** (default) or plain Welt.
        assert!(
            out.contains("Welt"),
            "expected `Welt` in output, got:\n{out}"
        );
    }

    // ============================================================
    // Phase 27 (IMG-06): rewrite_img_cids — CID-Transform + Asset-ID-
    // Sammlung. Reine Funktion; De-Dup pro Asset-ID; Plain-Text-Teil
    // trägt keine Bild-Leckage (T-27-13).
    // ============================================================

    #[test]
    fn rewrite_img_cids_single_image_produces_cid_and_ref() {
        let id = Uuid::new_v4();
        let html = format!(r#"<p>x</p><img data-genossi-asset-id="{}">"#, id);
        let (rewritten, refs) = rewrite_img_cids(&html);

        assert!(
            rewritten.contains(r#"src="cid:asset-1@genossi""#),
            "rewritten HTML must contain bare cid src, got:\n{rewritten}"
        );
        assert!(
            !rewritten.contains("<cid:") && !rewritten.contains("cid:<"),
            "cid must have no angle brackets, got:\n{rewritten}"
        );
        assert_eq!(refs.len(), 1, "one distinct asset ⇒ one AssetRef");
        assert_eq!(refs[0].asset_id, id);
        assert_eq!(refs[0].cid, "asset-1@genossi");
    }

    #[test]
    fn rewrite_img_cids_same_id_dedups_to_one_ref_both_imgs_same_cid() {
        let id = Uuid::new_v4();
        let html = format!(
            r#"<img data-genossi-asset-id="{}"><p>mid</p><img data-genossi-asset-id="{}">"#,
            id, id
        );
        let (rewritten, refs) = rewrite_img_cids(&html);

        assert_eq!(refs.len(), 1, "same asset id de-dups to ONE AssetRef");
        // Both <img> must carry the SAME cid.
        let count = rewritten.matches(r#"src="cid:asset-1@genossi""#).count();
        assert_eq!(count, 2, "both <img> get the same cid, got:\n{rewritten}");
    }

    #[test]
    fn rewrite_img_cids_distinct_ids_get_sequential_cids() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let html = format!(
            r#"<img data-genossi-asset-id="{}"><img data-genossi-asset-id="{}">"#,
            id1, id2
        );
        let (rewritten, refs) = rewrite_img_cids(&html);

        assert_eq!(refs.len(), 2, "two distinct ids ⇒ two AssetRefs");
        assert_eq!(refs[0].asset_id, id1);
        assert_eq!(refs[0].cid, "asset-1@genossi");
        assert_eq!(refs[1].asset_id, id2);
        assert_eq!(refs[1].cid, "asset-2@genossi");
        assert!(rewritten.contains(r#"src="cid:asset-1@genossi""#));
        assert!(rewritten.contains(r#"src="cid:asset-2@genossi""#));
    }

    #[test]
    fn rewrite_img_cids_no_image_returns_input_unchanged_and_empty_vec() {
        let html = "<p>Hallo <b>Welt</b></p><ul><li>a</li></ul>";
        let (rewritten, refs) = rewrite_img_cids(html);
        assert_eq!(
            rewritten, html,
            "no <img data-genossi-asset-id> ⇒ HTML unchanged (backward-compat signal)"
        );
        assert!(refs.is_empty(), "no images ⇒ empty Vec");
    }

    #[test]
    fn rewrite_img_cids_drops_data_attribute() {
        let id = Uuid::new_v4();
        let html = format!(r#"<img data-genossi-asset-id="{}">"#, id);
        let (rewritten, _refs) = rewrite_img_cids(&html);
        assert!(
            !rewritten.contains("data-genossi-asset-id"),
            "the data attribute must be replaced by the cid src, got:\n{rewritten}"
        );
    }

    #[test]
    fn rewrite_img_cids_leaves_malformed_id_untouched() {
        // A non-UUID value must not panic and must leave the tag as-is.
        let html = r#"<img data-genossi-asset-id="not-a-uuid">"#;
        let (rewritten, refs) = rewrite_img_cids(html);
        assert!(refs.is_empty(), "malformed id yields no AssetRef");
        assert!(
            rewritten.contains("not-a-uuid"),
            "malformed tag left untouched, got:\n{rewritten}"
        );
    }

    #[test]
    fn plain_from_html_strips_image_no_cid_no_img_leak() {
        // T-27-13: der Plain-Text-Teil darf weder `cid:` noch `img` enthalten.
        // Wir geben die BEREITS cid-rewritten HTML rein (worst case) und stellen
        // sicher, dass plain_from_html das <img> strippt.
        let html = r#"<p>Vorher</p><img src="cid:asset-1@genossi"><p>Nachher</p>"#;
        let plain = plain_from_html(html);
        assert!(
            !plain.contains("cid:"),
            "plain text must not leak cid:, got:\n{plain}"
        );
        assert!(
            !plain.to_lowercase().contains("img"),
            "plain text must not leak img token, got:\n{plain}"
        );
        assert!(
            plain.contains("Vorher") && plain.contains("Nachher"),
            "surrounding text must survive, got:\n{plain}"
        );
    }
}
