//! Shared, pure MIME construction for outgoing mail (Phase 22, MAIL-01/02/05).
//!
//! All three send paths (bulk worker, `send_test_mail`, `send_test_mail_with_body`
//! — the last of which the digest inherits) call [`build_message`] to produce a
//! [`lettre::Message`]. This is the single place where the text part's
//! `Content-Type` and `Content-Transfer-Encoding` are decided, so the historic
//! test-mail/digest charset bug cannot resurface (see 22-CONTEXT.md D-04).
//!
//! The function is intentionally synchronous — it takes already-loaded
//! attachment bytes so the caller can perform any async I/O (like
//! `DocumentStorage::load`) outside this factory (22-CONTEXT.md D-02/D-03).

use std::sync::Arc;

use lettre::message::header::{ContentTransferEncoding, ContentType};
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::Message;

use crate::service::{MailEncoding, MailServiceError};

/// A single attachment whose bytes are already resident in memory.
///
/// Mirrors [`crate::dao::MailRecipientAttachment`] but drops the DAO-only
/// fields (`recipient_id`, `document_id`) — this struct is pure MIME input
/// (22-CONTEXT.md D-02).
#[derive(Clone)]
pub struct LoadedAttachment {
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub bytes: Vec<u8>,
}

/// Phase 27 (IMG-06): a single inline image whose bytes are already resident in
/// memory, ready to become a `multipart/related` inline part.
///
/// Mirrors [`LoadedAttachment`] but carries a `cid` (the BARE Content-ID string,
/// e.g. `asset-1@genossi`) instead of a filename. The same `cid` string appears
/// both in the rewritten HTML (`src="cid:asset-1@genossi"`) and in
/// `Attachment::new_inline(cid)` (which emits `Content-ID: <asset-1@genossi>`),
/// so the two match exactly (Pitfall 6).
#[derive(Clone)]
pub struct LoadedInlineImage {
    pub cid: String,
    pub mime_type: Arc<str>,
    pub bytes: Vec<u8>,
}

/// Phase 27 (IMG-08, user decision D-02): the 25 MB total-size limit checked
/// against the BASE64-ENCODED wire size (SMTP SIZE limits apply to the encoded
/// message, not the raw payload).
const MAX_ENCODED_MAIL_BYTES: usize = 25 * 1024 * 1024;

/// Base64 encodes 3 raw bytes into 4 output chars. `div_ceil(3) * 4` is the
/// exact encoded length (including padding) for `raw_len` bytes.
fn base64_encoded_len(raw_len: usize) -> usize {
    raw_len.div_ceil(3) * 4
}

/// Build a [`lettre::Message`] with a consistent, charset-preserving text part.
///
/// * `from` / `to` are `&str` — this function centralises the three `.parse()`
///   sites that used to live in the worker and the two test-mail paths
///   (22-CONTEXT.md D-06).
/// * `body` is the raw plain-text body (never derived from `html_body` per
///   HTML-02).
/// * `html_body` — optional HTML sibling; when `Some`, produces a
///   `multipart/alternative` with text first, then HTML, per HTML-01/D-09.
/// * `attachments` may be empty — the resulting message is then a `SinglePart`
///   text message (or `multipart/alternative` if `html_body` is `Some`).
///   Otherwise the text part becomes the first slot of a `multipart/mixed`
///   message (or a nested `multipart/alternative` if `html_body` is `Some`).
/// * `in_reply_to` is the *bare* Message-ID (no angle brackets); when present,
///   both `In-Reply-To` and `References` are populated with the bracketed form.
/// * `encoding` decides between quoted-printable (default, MAIL-05
///   backward-compat) and 8bit (opt-in, MAIL-02) — the ONE place in the crate
///   where the Content-Transfer-Encoding is chosen (22-CONTEXT.md D-07/D-09).
///   Applies uniformly to text AND HTML part (D-01).
///
/// * `inline_images` (Phase 27, IMG-06) — optional `multipart/related` inline
///   parts. When non-empty, the `multipart/alternative` is wrapped in a
///   `multipart/related` and each image is added as an inline part whose
///   `Content-ID` matches the `cid:` reference in the HTML. When EMPTY, the
///   existing 4-branch `(html_body, attachments)` matrix runs byte-identically
///   (IMG-09).
#[allow(clippy::too_many_arguments)]
pub fn build_message(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    html_body: Option<&str>,
    attachments: &[LoadedAttachment],
    inline_images: &[LoadedInlineImage],
    in_reply_to: Option<&str>,
    encoding: MailEncoding,
) -> Result<Message, MailServiceError> {
    // Phase 27 (IMG-08, D-02): reject oversized mail BEFORE assembly. The basis
    // is the BASE64-ENCODED wire size because SMTP SIZE limits apply to the
    // encoded message, not the raw payload. We sum the encoded size of every
    // binary part (inline images + document attachments) plus the encoded body
    // and HTML length. This fires before any address parsing / part building so
    // the caller gets a clear app-level error instead of a late SMTP 552.
    let mut encoded_total: usize = base64_encoded_len(body.len());
    if let Some(html) = html_body {
        encoded_total = encoded_total.saturating_add(base64_encoded_len(html.len()));
    }
    for img in inline_images {
        encoded_total = encoded_total.saturating_add(base64_encoded_len(img.bytes.len()));
    }
    for att in attachments {
        encoded_total = encoded_total.saturating_add(base64_encoded_len(att.bytes.len()));
    }
    if encoded_total > MAX_ENCODED_MAIL_BYTES {
        return Err(MailServiceError::BadRequest(Arc::from(
            "Mail exceeds 25 MB limit (base64-encoded wire size)",
        )));
    }

    // Address parsing — the exact "Invalid from address" / "Invalid to address"
    // error strings are preserved from the pre-Phase-22 worker/service call
    // sites so downstream diagnostics (and any log-scraping) keep working.
    let from_addr = from.parse().map_err(|e: lettre::address::AddressError| {
        MailServiceError::SmtpError(Arc::from(format!("Invalid from address: {}", e)))
    })?;
    let to_addr = to.parse().map_err(|e: lettre::address::AddressError| {
        MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
    })?;

    // Build the text part explicitly in BOTH branches — no `SinglePart::plain`
    // fallback — so the CTE decision is visible on a single-line diff (per
    // 22-RESEARCH § Alternatives Considered).
    let cte = match encoding {
        MailEncoding::QuotedPrintable => ContentTransferEncoding::QuotedPrintable,
        MailEncoding::EightBit => ContentTransferEncoding::EightBit,
    };
    let text_part = SinglePart::builder()
        .header(ContentType::TEXT_PLAIN)
        .header(cte)
        .body(body.to_string());

    // HTML sibling — same CTE choice as the text part (D-01: encoding config
    // applies uniformly to both parts of an alternative message).
    let html_part_opt: Option<SinglePart> = html_body.map(|html| {
        SinglePart::builder()
            .header(ContentType::TEXT_HTML)
            .header(cte)
            .body(html.to_string())
    });

    // `message_id(None)` asks lettre to auto-generate a Message-ID; the worker
    // reads it back via `email.headers().get_raw("Message-ID")` before sending.
    let mut builder = Message::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(subject)
        .message_id(None);

    if let Some(ref_id) = in_reply_to {
        let bracketed = format!("<{}>", ref_id);
        builder = builder.in_reply_to(bracketed.clone()).references(bracketed);
    }

    // Phase 27 (IMG-06 / IMG-09): images present → build the multipart/related
    // tree; images absent → fall through to the byte-identical 4-branch matrix.
    if !inline_images.is_empty() {
        // The plain-text part is reused; the alternative needs a real HTML part.
        // If images are referenced but there is no HTML body, that is a caller
        // error (the cid: refs live in HTML) — surface it clearly.
        let Some(html_part) = html_part_opt else {
            return Err(MailServiceError::BadRequest(Arc::from(
                "inline images require an HTML body (cid: references live in the HTML part)",
            )));
        };

        let alternative = MultiPart::alternative()
            .singlepart(text_part)
            .singlepart(html_part);

        let mut related = MultiPart::related().multipart(alternative);
        for img in inline_images {
            // Pitfall 6: Attachment::new_inline("asset-1@genossi") emits
            // `Content-ID: <asset-1@genossi>`; the HTML says
            // `src="cid:asset-1@genossi"` — the SAME bare string in both places.
            let content_type = ContentType::parse(&img.mime_type)
                .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;
            let inline =
                Attachment::new_inline(img.cid.clone()).body(img.bytes.clone(), content_type);
            related = related.singlepart(inline);
        }

        // Document attachments (if any) wrap the related tree in multipart/mixed;
        // otherwise the related tree is the message body.
        if attachments.is_empty() {
            return builder
                .multipart(related)
                .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())));
        }
        let mut mixed = MultiPart::mixed().multipart(related);
        for att in attachments {
            let content_type = ContentType::parse(&att.mime_type).unwrap_or_else(|_| {
                ContentType::parse("application/octet-stream")
                    .expect("application/octet-stream is a valid MIME type")
            });
            let attachment =
                Attachment::new(att.file_name.to_string()).body(att.bytes.clone(), content_type);
            mixed = mixed.singlepart(attachment);
        }
        return builder
            .multipart(mixed)
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())));
    }

    // 4-branch decision tree — (html_body, attachments) matrix per HTML-01/D-10.
    // Text-FIRST ordering in the alternative wrapper is pinned by RFC 2046 §5.1.4
    // and RESEARCH Pitfall 5: the LAST part is the "richest"; HTML must come
    // second so HTML-capable clients render HTML.
    match (html_part_opt, attachments.is_empty()) {
        // (None, true) — Phase-22 legacy singlepart text (byte-identical).
        (None, true) => builder
            .singlepart(text_part)
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string()))),
        // (None, false) — Phase-22 legacy multipart/mixed{text, attachments}.
        (None, false) => {
            let mut multipart = MultiPart::mixed().singlepart(text_part);
            for att in attachments {
                let content_type = ContentType::parse(&att.mime_type).unwrap_or_else(|_| {
                    ContentType::parse("application/octet-stream")
                        .expect("application/octet-stream is a valid MIME type")
                });
                let attachment = Attachment::new(att.file_name.to_string())
                    .body(att.bytes.clone(), content_type);
                multipart = multipart.singlepart(attachment);
            }
            builder
                .multipart(multipart)
                .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))
        }
        // (Some, true) — multipart/alternative{text-first, html-second}.
        (Some(html_part), true) => {
            let alternative = MultiPart::alternative()
                .singlepart(text_part)
                .singlepart(html_part);
            builder
                .multipart(alternative)
                .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))
        }
        // (Some, false) — multipart/mixed{ multipart/alternative{text, html}, attachments }.
        (Some(html_part), false) => {
            let alternative = MultiPart::alternative()
                .singlepart(text_part)
                .singlepart(html_part);
            let mut multipart = MultiPart::mixed().multipart(alternative);
            for att in attachments {
                let content_type = ContentType::parse(&att.mime_type).unwrap_or_else(|_| {
                    ContentType::parse("application/octet-stream")
                        .expect("application/octet-stream is a valid MIME type")
                });
                let attachment = Attachment::new(att.file_name.to_string())
                    .body(att.bytes.clone(), content_type);
                multipart = multipart.singlepart(attachment);
            }
            builder
                .multipart(multipart)
                .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatted(msg: &Message) -> String {
        String::from_utf8_lossy(&msg.formatted()).to_string()
    }

    #[test]
    fn build_message_qp_has_utf8_charset_and_non_7bit_cte() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "Hallo Jürgen, schöne Grüße! ä ö ü ß",
            None,
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build QP mail");

        let text = formatted(&email);

        assert!(
            text.contains("charset=utf-8"),
            "QP mail must declare charset=utf-8, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-Transfer-Encoding: quoted-printable")
                || text.contains("Content-Transfer-Encoding: base64"),
            "QP mail must declare a non-7bit transfer encoding, got:\n{}",
            text
        );
        assert!(
            !text.contains("Content-Transfer-Encoding: 8bit"),
            "QP mail must NOT declare 8bit CTE, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_8bit_has_utf8_charset_and_8bit_cte() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "Hallo Jürgen, schöne Grüße! ä ö ü ß",
            None,
            &[],
            &[],
            None,
            MailEncoding::EightBit,
        )
        .expect("build 8bit mail");

        let text = formatted(&email);

        assert!(
            text.contains("charset=utf-8"),
            "8bit mail must declare charset=utf-8, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-Transfer-Encoding: 8bit"),
            "8bit mail must declare CTE=8bit exactly, got:\n{}",
            text
        );
        assert!(
            !text.contains("Content-Transfer-Encoding: quoted-printable"),
            "8bit mail must NOT declare CTE=quoted-printable, got:\n{}",
            text
        );
        // Build the two-char sequence at runtime so it does not appear literally
        // in the source (avoids comment-text discipline conflicts).
        let qp_softbreak = format!("={}", "\r\n");
        assert!(
            !text.contains(&qp_softbreak),
            "8bit body must not contain a QP soft-line-break sequence, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_multipart_text_part_has_utf8_charset() {
        let attachment = LoadedAttachment {
            file_name: Arc::from("test.pdf"),
            mime_type: Arc::from("application/pdf"),
            bytes: b"%PDF-fake".to_vec(),
        };

        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "Anbei die Bescheinigung für Herrn Müller.",
            None,
            std::slice::from_ref(&attachment),
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build multipart mail");

        let text = formatted(&email);

        assert!(
            text.contains("charset=utf-8"),
            "multipart text part must declare charset=utf-8, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-Transfer-Encoding: quoted-printable")
                || text.contains("Content-Transfer-Encoding: base64"),
            "multipart text part must declare a non-7bit transfer encoding, got:\n{}",
            text
        );
        assert!(
            text.contains("multipart/mixed"),
            "attachment mail must be multipart/mixed, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_reply_includes_in_reply_to_and_references() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Re: Test",
            "reply body",
            None,
            &[],
            &[],
            Some("abc.123@example.com"),
            MailEncoding::QuotedPrintable,
        )
        .expect("build reply mail");

        let text = formatted(&email);

        assert!(
            text.contains("In-Reply-To: <abc.123@example.com>"),
            "reply mail must contain In-Reply-To header, got:\n{}",
            text
        );
        assert!(
            text.contains("References: <abc.123@example.com>"),
            "reply mail must contain References header, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_non_reply_omits_in_reply_to() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "body",
            None,
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build non-reply mail");

        let text = formatted(&email);

        assert!(
            !text.contains("In-Reply-To:"),
            "non-reply mail must not contain In-Reply-To header, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_exposes_auto_generated_message_id() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "hi",
            None,
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build mail");

        let raw = email
            .headers()
            .get_raw("Message-ID")
            .expect("lettre should auto-set Message-ID");
        let normalized = crate::dao::normalize_message_id(raw).expect("Message-ID must normalise");

        assert!(
            !normalized.contains('<') && !normalized.contains('>'),
            "normalized Message-ID must not contain angle brackets: {normalized}"
        );
        assert!(
            normalized.contains('@'),
            "Message-ID should contain '@': {normalized}"
        );
    }

    #[test]
    fn build_message_rejects_malformed_from_address() {
        let result = build_message(
            "not-an-address",
            "recipient@example.com",
            "s",
            "b",
            None,
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        );

        assert!(
            matches!(result, Err(MailServiceError::SmtpError(_))),
            "malformed from address must yield SmtpError, got: {:?}",
            result
        );
        let err_str = format!("{:?}", result.err().unwrap());
        assert!(
            err_str.contains("Invalid from address"),
            "error must mention 'Invalid from address', got: {}",
            err_str
        );
    }

    // ---- Phase 23 Plan 03: multipart/alternative and mixed{alternative,attach} ----

    #[test]
    fn build_message_alternative_text_then_html_no_attachments() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "Hallo Jürgen (plain).",
            Some("<p>Hallo Jürgen (html).</p>"),
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build alternative mail");

        let text = formatted(&email);

        assert!(
            text.contains("multipart/alternative"),
            "alternative mail must declare multipart/alternative, got:\n{}",
            text
        );
        assert!(
            !text.contains("multipart/mixed"),
            "no-attachment alternative must NOT be wrapped in multipart/mixed, got:\n{}",
            text
        );
        // Pitfall 5 — text FIRST, HTML SECOND (byte-offset assertion).
        let text_pos = text
            .find("text/plain")
            .expect("text/plain must appear in alternative output");
        let html_pos = text
            .find("text/html")
            .expect("text/html must appear in alternative output");
        assert!(
            text_pos < html_pos,
            "text/plain (offset {}) must appear BEFORE text/html (offset {}) — RFC 2046 §5.1.4 preference order.\nfull output:\n{}",
            text_pos,
            html_pos,
            text
        );
    }

    #[test]
    fn build_message_alternative_text_part_is_verbatim_body() {
        // HTML-02: the plain-text part is the raw `body`, NOT derived from HTML.
        // Two distinct strings prove no derivation happens — the text part
        // carries "plain-verbatim-marker", the HTML part carries the HTML.
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain-verbatim-marker",
            Some("<p>totally-different-html-marker</p>"),
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build alternative mail");

        let text = formatted(&email);

        assert!(
            text.contains("plain-verbatim-marker"),
            "text part must contain the verbatim body string, got:\n{}",
            text
        );
        assert!(
            text.contains("totally-different-html-marker"),
            "html part must contain the HTML body string, got:\n{}",
            text
        );
        // The text part must not appear inside the HTML tag content — i.e. the
        // body is NOT derived by round-tripping the HTML. Assert the plain marker
        // is not inside the <p>…</p> HTML wrapper.
        assert!(
            !text.contains("<p>plain-verbatim-marker</p>"),
            "plain body must not be embedded in HTML wrapper — no derivation (HTML-02), got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_mixed_wraps_alternative_when_attach() {
        let attachment = LoadedAttachment {
            file_name: Arc::from("test.pdf"),
            mime_type: Arc::from("application/pdf"),
            bytes: b"%PDF-fake".to_vec(),
        };

        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain body",
            Some("<p>html body</p>"),
            std::slice::from_ref(&attachment),
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build mixed alternative mail");

        let text = formatted(&email);

        assert!(
            text.contains("multipart/mixed"),
            "attach + html must be multipart/mixed at the outer layer, got:\n{}",
            text
        );
        assert!(
            text.contains("multipart/alternative"),
            "attach + html must nest a multipart/alternative, got:\n{}",
            text
        );
        // Outer wrapper is mixed: the top-level Content-Type header must be
        // multipart/mixed, and multipart/alternative appears LATER in the body.
        let mixed_pos = text
            .find("multipart/mixed")
            .expect("multipart/mixed must appear");
        let alt_pos = text
            .find("multipart/alternative")
            .expect("multipart/alternative must appear");
        assert!(
            mixed_pos < alt_pos,
            "multipart/mixed (outer, offset {}) must appear before multipart/alternative (nested, offset {}), got:\n{}",
            mixed_pos,
            alt_pos,
            text
        );
        // Attachment payload present too.
        assert!(
            text.contains("application/pdf") || text.contains("test.pdf"),
            "attachment must be present, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_legacy_singlepart_text_unchanged() {
        // HTML-01 regression: html_body=None + no attachments -> legacy singlepart.
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "just text",
            None,
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build legacy singlepart");

        let text = formatted(&email);

        assert!(
            !text.contains("multipart/"),
            "legacy singlepart must NOT contain any multipart/ declarations, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-Type: text/plain"),
            "legacy singlepart must declare Content-Type: text/plain at the top, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_html_part_declares_text_html_charset_utf8() {
        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain",
            Some("<p>Hallo Jürgen — Umlaute ä ö ü ß</p>"),
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build alternative mail");

        let text = formatted(&email);

        assert!(
            text.contains("text/html"),
            "html part must declare Content-Type: text/html, got:\n{}",
            text
        );
        // MAIL-01 style regression: charset=utf-8 must apply on the HTML part.
        // Assert charset=utf-8 appears somewhere AFTER the text/html declaration
        // so we know the charset attaches to the HTML part (not just the text part).
        let html_pos = text.find("text/html").expect("text/html must appear");
        let after_html = &text[html_pos..];
        assert!(
            after_html.contains("charset=utf-8"),
            "html part must declare charset=utf-8 next to text/html, got (from html-pos):\n{}",
            after_html
        );
    }

    // ---- Phase 27 Plan 03: multipart/related inline images + 25 MB base64 guard ----

    fn png_bytes(n: usize) -> Vec<u8> {
        // A minimal PNG-ish blob (magic bytes + padding). Content is irrelevant
        // to the MIME structure assertions; only the length matters for IMG-08.
        let mut v = vec![0x89, 0x50, 0x4E, 0x47];
        v.resize(n.max(4), 0x00);
        v
    }

    #[test]
    fn build_message_related_structure_matches_cid_and_content_id() {
        // IMG-06: one inline image + html referencing its cid produces a
        // multipart/related > multipart/alternative tree with a matching
        // Content-ID and cid: reference (Pitfall 6).
        let img = LoadedInlineImage {
            cid: "asset-1@genossi".to_string(),
            mime_type: Arc::from("image/png"),
            bytes: png_bytes(16),
        };

        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain",
            Some(r#"<p>Logo:</p><img src="cid:asset-1@genossi">"#),
            &[],
            std::slice::from_ref(&img),
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build related mail");

        let text = formatted(&email);

        assert!(
            text.contains("multipart/related"),
            "image mail must declare multipart/related, got:\n{}",
            text
        );
        assert!(
            text.contains("multipart/alternative"),
            "image mail must nest a multipart/alternative, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-ID: <asset-1@genossi>"),
            "inline part must declare Content-ID: <asset-1@genossi>, got:\n{}",
            text
        );
        assert!(
            text.contains("cid:asset-1@genossi"),
            "html part must reference cid:asset-1@genossi, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_empty_inline_images_is_byte_identical_no_related() {
        // IMG-09: an empty inline_images slice must NOT introduce a related
        // wrapper — the output is the existing alternative shape.
        let with_empty = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain",
            Some("<p>html</p>"),
            &[],
            &[],
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build alternative mail");

        let text = formatted(&with_empty);
        assert!(
            !text.contains("multipart/related"),
            "empty inline_images must NOT produce multipart/related, got:\n{}",
            text
        );
        assert!(
            text.contains("multipart/alternative"),
            "no-image html mail stays multipart/alternative, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_rejects_when_base64_encoded_size_exceeds_25mb() {
        // IMG-08 (D-02): the guard is on the BASE64-ENCODED wire size. Choose a
        // raw size UNDER 25 MB whose base64 encoding EXCEEDS 25 MB. base64
        // inflates by 4/3, so a raw payload of 20 MB encodes to ~26.6 MB.
        // 20 MB raw < 25 MB (a raw-byte guard would WRONGLY accept it), but
        // 20 MB * 4/3 ≈ 26.6 MB > 25 MB (the base64 guard MUST reject it).
        let raw = 20 * 1024 * 1024;
        assert!(
            raw < MAX_ENCODED_MAIL_BYTES,
            "precondition: raw payload must be under the 25 MB limit"
        );
        assert!(
            base64_encoded_len(raw) > MAX_ENCODED_MAIL_BYTES,
            "precondition: base64-encoded payload must exceed the 25 MB limit"
        );

        let img = LoadedInlineImage {
            cid: "asset-1@genossi".to_string(),
            mime_type: Arc::from("image/png"),
            bytes: png_bytes(raw),
        };

        let result = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain",
            Some(r#"<img src="cid:asset-1@genossi">"#),
            &[],
            std::slice::from_ref(&img),
            None,
            MailEncoding::QuotedPrintable,
        );

        assert!(
            matches!(result, Err(MailServiceError::BadRequest(_))),
            "base64-encoded oversize must yield BadRequest before assembly, got: {:?}",
            result.map(|_| "Ok(Message)")
        );
    }

    #[test]
    fn build_message_mixed_wraps_related_when_attachments_present() {
        // IMG-06 + attachments: multipart/mixed > related > alternative + inline
        // parts + attachment parts.
        let img = LoadedInlineImage {
            cid: "asset-1@genossi".to_string(),
            mime_type: Arc::from("image/png"),
            bytes: png_bytes(16),
        };
        let attachment = LoadedAttachment {
            file_name: Arc::from("test.pdf"),
            mime_type: Arc::from("application/pdf"),
            bytes: b"%PDF-fake".to_vec(),
        };

        let email = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain",
            Some(r#"<img src="cid:asset-1@genossi">"#),
            std::slice::from_ref(&attachment),
            std::slice::from_ref(&img),
            None,
            MailEncoding::QuotedPrintable,
        )
        .expect("build mixed>related mail");

        let text = formatted(&email);

        let mixed_pos = text.find("multipart/mixed").expect("mixed must appear");
        let related_pos = text.find("multipart/related").expect("related must appear");
        let alt_pos = text
            .find("multipart/alternative")
            .expect("alternative must appear");
        assert!(
            mixed_pos < related_pos && related_pos < alt_pos,
            "nesting must be mixed(outer) > related > alternative, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-ID: <asset-1@genossi>"),
            "inline image Content-ID must be present, got:\n{}",
            text
        );
        assert!(
            text.contains("application/pdf") || text.contains("test.pdf"),
            "document attachment must be present, got:\n{}",
            text
        );
    }

    #[test]
    fn build_message_images_without_html_body_is_rejected() {
        // Caller error: cid: references live in HTML; images with no HTML body
        // must not silently drop the images.
        let img = LoadedInlineImage {
            cid: "asset-1@genossi".to_string(),
            mime_type: Arc::from("image/png"),
            bytes: png_bytes(16),
        };
        let result = build_message(
            "sender@example.com",
            "recipient@example.com",
            "Test",
            "plain only",
            None,
            &[],
            std::slice::from_ref(&img),
            None,
            MailEncoding::QuotedPrintable,
        );
        assert!(
            matches!(result, Err(MailServiceError::BadRequest(_))),
            "images without an HTML body must be rejected, got: {:?}",
            result.map(|_| "Ok(Message)")
        );
    }
}
