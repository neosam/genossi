//! Shared HTML sanitizer for author-supplied Vorstand HTML.
//!
//! Phase 23 (HTML-05): every entry point that persists author-HTML runs the
//! payload through [`sanitize_html`] before it reaches storage. The Vorstand
//! writes `body_html` for templates + test-mails; a compromised or malicious
//! actor could smuggle `<script>`, `onclick`, or `javascript:` URLs — this
//! module is the single choke-point that neutralises those payloads.
//!
//! Design (23-CONTEXT.md D-01/D-02, RESEARCH § "Pattern 2"):
//!   - Permissive ammonia default (via `Builder::default()` internally):
//!     `<h1>`..`<h6>`, `<p>`, `<a>`, `<b>/<i>/<strong>/<em>`, `<ul>/<ol>/<li>`,
//!     tables, and external `<img src="https://…">` survive; `<script>`, event
//!     handlers (`on*`), `javascript:`/`data:` URL schemes are stripped;
//!     `target="_blank"` gets `rel="noopener noreferrer"` auto-added.
//!   - NOT a mail sender — sending stays `lettre` (D-02). This module is a
//!     safety net for the store-side pathway only (D-03 lists the four entry
//!     points wired in Plan 23-04: `create_job`, `MailTemplateService::create`,
//!     `MailTemplateService::update`, `send_test_mail_with_body`).
//!   - NO custom `Builder` policy — the permissive default is the user-locked
//!     choice (23-CONTEXT.md D-01 rationale: more formatting freedom, less
//!     custom code).
//!   - NO `bool` toggle — behaviour is fixed by ammonia's default, matching
//!     the project rule "Immer Enum statt Boolean".
//!
//! Jinja placeholders inside **text content** (`<p>Hallo {{ first_name }}</p>`)
//! survive ammonia intact (RESEARCH Pitfall 1). Placeholders inside HTML
//! attributes (e.g. `<a href="{{ link }}">`) are OUT OF CONTRACT: ammonia will
//! reject `{{ link }}` as an invalid URL and strip the attribute. Phase-24
//! editor is expected to enforce the text-content-only invariant.

/// Sanitize author-supplied HTML with the permissive `ammonia` default filter.
///
/// Delegates to ammonia's permissive default. Called by every store-side entry
/// point that accepts `body_html` from the Vorstand (Plan 23-04 D-03).
pub fn sanitize_html(html: &str) -> String {
    ammonia::clean(html)
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
}
