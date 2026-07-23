//! Shared HTML sanitizer for author-supplied Vorstand HTML.
//!
//! Phase 23 (HTML-05): every entry point that persists author-HTML runs the
//! payload through [`sanitize_html`] before it reaches storage. The Vorstand
//! writes `body_html` for templates + test-mails; a compromised or malicious
//! actor could smuggle `<script>`, `onclick`, or `javascript:` URLs — this
//! module is the single choke-point that neutralises those payloads.
//!
//! Design (23-CONTEXT.md D-01/D-02, RESEARCH § "Pattern 2"; Phase 27 IMG-05):
//!   - Starts from the permissive ammonia default (`Builder::default()`):
//!     `<h1>`..`<h6>`, `<p>`, `<a>`, `<b>/<i>/<strong>/<em>`, `<ul>/<ol>/<li>`,
//!     and tables survive; `<script>`, event handlers (`on*`),
//!     `javascript:`/`data:` URL schemes are stripped; `target="_blank"` gets
//!     `rel="noopener noreferrer"` auto-added.
//!   - Phase 27 (IMG-05) hardens the `<img>` rule: the sanitizer is NO LONGER
//!     the permissive default — it uses a custom `ammonia::Builder` that
//!     restricts `<img>` to the single `data-genossi-asset-id` attribute and
//!     strips every resolvable/scriptable reference (`src`, `srcset`, `alt`,
//!     `width`, `height`, `loading`). External `http(s)` src, `data:` URIs, and
//!     `<svg>` are all dropped — no resolvable image reference is ever stored.
//!     The `src` is injected downstream at preview time (`/bytes` URL, 27-04)
//!     and at send time (`cid:` URL, 27-03); it is never persisted.
//!   - NOT a mail sender — sending stays `lettre` (D-02). This module is a
//!     safety net for the store-side pathway only (D-03 lists the four entry
//!     points wired in Plan 23-04: `create_job`, `MailTemplateService::create`,
//!     `MailTemplateService::update`, `send_test_mail_with_body`).
//!   - NO `bool` toggle — behaviour is fixed by the custom Builder policy,
//!     matching the project rule "Immer Enum statt Boolean".
//!
//! Jinja placeholders inside **text content** (`<p>Hallo {{ first_name }}</p>`)
//! survive ammonia intact (RESEARCH Pitfall 1). Placeholders inside HTML
//! attributes (e.g. `<a href="{{ link }}">`) are OUT OF CONTRACT: ammonia will
//! reject `{{ link }}` as an invalid URL and strip the attribute. Phase-24
//! editor is expected to enforce the text-content-only invariant.

use std::sync::OnceLock;

/// The shared, restricted sanitizer. Constructed exactly once — `Builder`
/// construction is not free, so it is cached in a `OnceLock` (Plan 27-02 /
/// RESEARCH Pattern 3). Starts from `Builder::default()` (keeping all Phase
/// 23/26 tag/attr/scheme guarantees) and tightens only the `<img>` rule.
fn builder() -> &'static ammonia::Builder<'static> {
    static BUILDER: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    BUILDER.get_or_init(|| {
        let mut builder = ammonia::Builder::default();
        builder
            // Strip every default <img> attribute that could carry a
            // resolvable or scriptable reference (external src, data: URI, …).
            .rm_tag_attributes("img", &["src", "srcset", "alt", "width", "height", "loading"])
            // Whitelist ONLY the asset-id data attribute on <img>. This is the
            // sole reference persisted; the resolvable `src` is injected
            // downstream (27-03 cid: / 27-04 /bytes), never stored.
            .add_tag_attributes("img", &["data-genossi-asset-id"])
            // Forbid `data:` URIs anywhere (SVG-data-URI / exfiltration vector).
            .rm_url_schemes(&["data"]);
        // NOTE: <svg> is deliberately NOT added — it is absent from ammonia's
        // default tag set, so it stays stripped (a survival test proves this).
        builder
    })
}

/// Sanitize author-supplied HTML with the hardened `ammonia` policy.
///
/// Runs the payload through the cached custom [`builder`] (IMG-05): only
/// `<img data-genossi-asset-id="…">` survives; external `src`, `data:` URIs and
/// `<svg>` are stripped. Called by every store-side entry point that accepts
/// `body_html` from the Vorstand (Plan 23-04 D-03).
pub fn sanitize_html(html: &str) -> String {
    builder().clean(html).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_script_tag() {
        let input = "<p>Hallo</p><script>alert(1)</script>";
        let output = sanitize_html(input);
        assert!(
            output.contains("<p>Hallo</p>"),
            "expected author markup to survive, got: {output}"
        );
        assert!(
            !output.contains("<script>"),
            "expected <script> to be stripped, got: {output}"
        );
        assert!(
            !output.contains("alert(1)"),
            "expected script contents to be stripped, got: {output}"
        );
    }

    #[test]
    fn sanitize_strips_event_handlers() {
        let input = r#"<p onclick="alert(1)">Hi</p>"#;
        let output = sanitize_html(input);
        assert!(
            !output.contains("onclick"),
            "expected onclick attribute to be stripped, got: {output}"
        );
        assert!(
            output.contains("Hi"),
            "expected inner text to survive, got: {output}"
        );
    }

    #[test]
    fn sanitize_strips_dangerous_url_schemes() {
        // javascript: scheme
        let js_input = r#"<a href="javascript:alert(1)">click</a>"#;
        let js_output = sanitize_html(js_input);
        assert!(
            !js_output.contains("javascript:"),
            "expected javascript: URL scheme to be stripped, got: {js_output}"
        );

        // data: scheme (analogous data URI attack surface)
        let data_input = r#"<a href="data:text/html,<script>alert(1)</script>">click</a>"#;
        let data_output = sanitize_html(data_input);
        assert!(
            !data_output.contains(r#"href="data:"#),
            "expected data: URL scheme to be stripped, got: {data_output}"
        );
    }

    #[test]
    fn sanitize_preserves_jinja_placeholder_in_text_content() {
        // Pitfall 1: {{ first_name }} appearing as TEXT CONTENT (not attribute)
        // must survive ammonia unchanged — otherwise stored templates break.
        let input = "<p>Hallo {{ first_name }}</p>";
        let output = sanitize_html(input);
        assert!(
            output.contains("{{ first_name }}"),
            "expected Jinja placeholder in text content to survive, got: {output}"
        );
        assert!(
            output.contains("<p>") && output.contains("</p>"),
            "expected <p> wrapper to survive, got: {output}"
        );
    }

    // Phase 26 EDIT-06 (D-01/D-03/D-04): ammonia default must preserve UL/LI
    // so the WYSIWYG toolbar's Unordered-List button round-trips through
    // sanitize_html unchanged. sanitize.rs is intentionally unmodified
    // outside the tests module (D-04 backward-compat).
    #[test]
    fn sanitize_preserves_unordered_list() {
        let input = "<ul><li>a</li><li>b</li></ul>";
        let output = sanitize_html(input);
        assert!(
            output.contains("<ul>"),
            "expected <ul> to survive, got: {output}"
        );
        assert!(
            output.contains("<li>a</li>"),
            "expected <li>a</li> to survive, got: {output}"
        );
        assert!(
            output.contains("<li>b</li>"),
            "expected <li>b</li> to survive, got: {output}"
        );
    }

    // Phase 26 EDIT-07 (D-01/D-03/D-04): ammonia default must preserve OL/LI
    // so the WYSIWYG toolbar's Ordered-List button round-trips through
    // sanitize_html unchanged.
    #[test]
    fn sanitize_preserves_ordered_list() {
        let input = "<ol><li>1</li><li>2</li></ol>";
        let output = sanitize_html(input);
        assert!(
            output.contains("<ol>"),
            "expected <ol> to survive, got: {output}"
        );
        assert!(
            output.contains("<li>1</li>"),
            "expected <li>1</li> to survive, got: {output}"
        );
        assert!(
            output.contains("<li>2</li>"),
            "expected <li>2</li> to survive, got: {output}"
        );
    }

    // Phase 26 EDIT-08 (D-01/D-03/D-04): ammonia default must preserve H1/H2/H3
    // so the WYSIWYG toolbar's heading buttons round-trip through
    // sanitize_html unchanged. H1 is explicitly covered per D-01 (H1 toolbar
    // button stays in the frontend).
    #[test]
    fn sanitize_preserves_headings_h1_h2_h3() {
        let input = "<h1>A</h1><h2>B</h2><h3>C</h3>";
        let output = sanitize_html(input);
        for token in ["<h1>", "</h1>", "<h2>", "</h2>", "<h3>", "</h3>"] {
            assert!(
                output.contains(token),
                "expected {token} to survive, got: {output}"
            );
        }
    }

    // Phase 27 IMG-05 / RESEARCH Pitfall 2 & Assumption A2: the asset reference
    // (`data-genossi-asset-id`) is the ONLY thing that may survive on an <img>.
    // ammonia drops `data-*` unless explicitly whitelisted — this test is the
    // arbiter of which Builder lever ammonia 4 needs (add_tag_attributes vs
    // add_generic_attribute_prefixes fallback).
    #[test]
    fn sanitize_preserves_img_data_genossi_asset_id() {
        let input = r#"<img data-genossi-asset-id="abc">"#;
        let output = sanitize_html(input);
        assert!(
            output.contains(r#"data-genossi-asset-id="abc""#),
            "expected data-genossi-asset-id to survive, got: {output}"
        );
    }

    // Phase 27 IMG-05 / T-27-07 (SSRF / tracking pixel): external http(s) src
    // must be stripped while the asset-id survives.
    #[test]
    fn sanitize_strips_external_http_img_src_keeps_asset_id() {
        let input = r#"<img src="https://evil.example/x.png" data-genossi-asset-id="abc">"#;
        let output = sanitize_html(input);
        assert!(
            output.contains(r#"data-genossi-asset-id="abc""#),
            "expected data-genossi-asset-id to survive, got: {output}"
        );
        assert!(
            !output.contains("src="),
            "expected external src attribute to be stripped, got: {output}"
        );
        assert!(
            !output.contains("https://evil"),
            "expected external URL to be stripped, got: {output}"
        );
    }

    // Phase 27 IMG-05 / T-27-08 (data: URI exfiltration): a data: src must be
    // stripped entirely.
    #[test]
    fn sanitize_strips_data_uri_img_src() {
        let input = r#"<img src="data:image/png;base64,AAAA">"#;
        let output = sanitize_html(input);
        assert!(
            !output.contains("data:"),
            "expected data: URI scheme to be stripped, got: {output}"
        );
    }

    // Phase 27 IMG-05 / T-27-06 (SVG-as-image XSS): <svg> is not in ammonia's
    // default tag set and the Builder does not add it — it must be dropped.
    #[test]
    fn sanitize_strips_svg() {
        let input = "<svg><rect/></svg>";
        let output = sanitize_html(input);
        assert!(
            !output.contains("<svg"),
            "expected <svg> to be dropped, got: {output}"
        );
    }
}
