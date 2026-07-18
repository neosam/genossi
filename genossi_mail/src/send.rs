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
pub fn build_message(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    html_body: Option<&str>,
    attachments: &[LoadedAttachment],
    in_reply_to: Option<&str>,
    encoding: MailEncoding,
) -> Result<Message, MailServiceError> {
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
}
