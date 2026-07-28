//! Phase 28 — Device-Vorschau für den Mail-Editor.
//!
//! Diese Datei liefert die Primitive der Vorschau als eigenständige, nativ
//! testbare Einheiten: die Geometrie (`PreviewMode`), die `src`-Injektion aus
//! `data-genossi-asset-id` (D-06), das in sich geschlossene Vorschau-Dokument
//! mit Baseline-Stylesheet (D-09/D-10) und die `MailPreviewFrame`-Component
//! mit Device-Rahmen und Sandbox (D-07/D-15).
//!
//! Bewusst KEINE Editor-Integration — Plan 28-03 verkabelt diese Bausteine.
//! Dadurch sind Asset-Rewrite, Dokument-Aufbau und die Sandbox-Invariante
//! automatisiert prüfbar, ohne Browser und ohne wasm32-Target.

// Plan 28-03 hat das Modul-Level `#![allow(dead_code)]` wieder entfernt: seit
// der Verkabelung im `WysiwygEditor` hat jedes Symbol dieser Datei einen
// Produktions-Konsumenten, der Build bleibt ohne das Allow warnungsfrei.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::component::mail_compose::wysiwyg_toolbar::asset_bytes_url;
use crate::i18n::{use_i18n, Key};

/// Desktop-Vorschau-Breite in CSS-px (Roadmap: „~640 px", D-12).
pub const PREVIEW_WIDTH_DESKTOP_PX: u32 = 640;

/// Mobile-Vorschau-Breite in CSS-px (Roadmap: „~360 px", D-12).
pub const PREVIEW_WIDTH_MOBILE_PX: u32 = 360;

/// Feste Viewport-Höhe des iframes.
///
/// Bewusst konstant statt am Inhalt gemessen (Pitfall 2 der Phase): eine
/// Auto-Messung bräuchte das `HtmlIFrameElement`-Feature, einen nativen
/// `load`-Listener (das Dokument-Attribut wird asynchron geparst) und ein
/// Re-Measure bei jedem Dokument-Update. Für eine *Device*-Simulation ist ein
/// fester Viewport ohnehin die semantisch richtigere Wahl — echte Mail-Clients
/// haben auch einen. Ohne explizite Höhe wäre der iframe per CSS-Default
/// 150 px hoch und die Vorschau sähe im ersten Eindruck kaputt aus.
pub const PREVIEW_HEIGHT_PX: u32 = 640;

/// Anzeige-Modus des Mail-Editors (Projekt-Regel „Enum statt Boolean" — ein
/// Paar `preview: bool` + `mobile: bool` könnte den unmöglichen Zustand
/// „kein Preview, aber mobil" ausdrücken).
///
/// Hinweis: In `crate::page::templates` existiert ein gleichnamiger, aber
/// vollkommen unverwandter privater `PreviewMode` (Member-/Application-
/// Umschaltung). Es ist ein anderer Typ in einem anderen Modul. Konsumenten
/// importieren den hier definierten Typ über den vollqualifizierten Pfad
/// `crate::component::mail_compose::mail_preview_frame::PreviewMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreviewMode {
    Edit,
    Desktop,
    Mobile,
}

impl PreviewMode {
    /// Breite des Vorschau-Viewports. `None` für [`PreviewMode::Edit`] — im
    /// Bearbeiten-Modus gibt es keinen iframe, also auch keine Breite.
    pub fn width_px(self) -> Option<u32> {
        match self {
            PreviewMode::Edit => None,
            PreviewMode::Desktop => Some(PREVIEW_WIDTH_DESKTOP_PX),
            PreviewMode::Mobile => Some(PREVIEW_WIDTH_MOBILE_PX),
        }
    }

    /// Wahr für die beiden Device-Modi. Steuert in Plan 28-03 die
    /// Toolbar-Sichtbarkeit (D-14: im Vorschau-Modus ausgeblendet).
    pub fn is_preview(self) -> bool {
        !matches!(self, PreviewMode::Edit)
    }
}

/// Entscheidet, ob ein Modus-Wechsel einen neuen Preview-Request auslösen muss
/// (D-05).
///
/// Wahr genau dann, wenn `to` ein Vorschau-Modus ist und `from` keiner war.
/// Der Wechsel Desktop ↔ Mobile ändert nur die Breite und lässt das
/// Vorschau-Dokument unangetastet — Dioxus' Attribut-Diff überspringt das
/// Dokument-Attribut dann, der iframe lädt nicht neu und die Vorschau
/// flackert nicht. Das spart einen Roundtrip und ist exakt die Lesart von
/// D-05 („ein Request pro Umschaltung" = pro Wechsel *in* einen Vorschau-Modus).
pub(crate) fn preview_needs_fetch(from: PreviewMode, to: PreviewMode) -> bool {
    to.is_preview() && !from.is_preview()
}

/// Fügt jedem `<img data-genossi-asset-id="{uuid}">` ein
/// `src="{backend}/api/mail/assets/{uuid}/bytes"` hinzu (D-06, PREV-03).
///
/// Spiegel von `genossi_mail::render::rewrite_img_cids`, nur dass das
/// Asset-Attribut **erhalten bleibt** statt ersetzt zu werden. Der übrige
/// Tag-Inhalt kommt Byte-für-Byte unverändert heraus; HTML ohne `<img` ist
/// byte-identisch mit der Eingabe — das ist die Backward-Compat-Garantie für
/// v1.4-Templates.
///
/// Warum String-Scan und nicht DOM-Parsing: Der Input ist ammonia-Output, und
/// auf `<img>` überlebt ausschließlich `data-genossi-asset-id`
/// (`genossi_mail/src/sanitize.rs`). Die Tag-Form ist damit vollständig
/// vorhersagbar. DOM-Parsing bräuchte zusätzliche `web-sys`-Features und wäre
/// nicht nativ unit-testbar. Das Backend hat für dasselbe Problem denselben
/// Weg gewählt — Symmetrie schlägt Eleganz.
///
/// Robustheit: Fehlt einem `<img` das schließende `>`, wird der Rest
/// unverändert angehängt und die Funktion kehrt zurück. Kein Panic, keine
/// Endlosschleife.
pub(crate) fn inject_asset_src(html: &str, backend: &str) -> String {
    const ATTR: &str = "data-genossi-asset-id";
    let mut out = String::with_capacity(html.len() + 64);
    let mut rest = html;

    while let Some(start) = rest.find("<img") {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let Some(end) = tail.find('>') else {
            // Unvollständiger Tag: Rest unverändert übernehmen und raus.
            out.push_str(tail);
            return out;
        };
        let tag = &tail[..=end];

        match extract_asset_uuid(tag, ATTR) {
            // Einschub unmittelbar vor dem schließenden '>' — der restliche
            // Tag-Inhalt inklusive des data-Attributs bleibt 1:1 erhalten.
            Some(id) => {
                out.push_str(&tag[..tag.len() - 1]);
                out.push_str(&format!(
                    r#" src="{}">"#,
                    asset_bytes_url(backend, &id.to_string())
                ));
            }
            None => out.push_str(tag),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Liest den Wert von `attr` aus `tag` und gibt ihn nur zurück, wenn er eine
/// gültige UUID ist.
///
/// SICHERHEIT (nicht Kosmetik): Ohne diese Validierung könnte ein präparierter
/// Attributwert mit einem Anführungszeichen aus dem eingefügten `src`-Attribut
/// ausbrechen und einen Event-Handler anhängen. ammonia und die fehlende
/// Script-Erlaubnis der iframe-Sandbox würden das heute abfangen — aber beides
/// sind Umgebungsannahmen, die eine spätere Änderung aushebeln kann. Mit
/// `Uuid::parse_str` ist die Injektion strukturell ausgeschlossen, unabhängig
/// von beiden anderen Schichten. Das Muster stammt 1:1 aus
/// `genossi_mail::render::extract_asset_id`, das in Produktion läuft.
///
/// Toleriert Whitespace um das `=` sowie einfache und doppelte
/// Anführungszeichen als Begrenzer. Jede Abweichung liefert `None`, wodurch
/// [`inject_asset_src`] den Tag unangetastet lässt.
fn extract_asset_uuid(tag: &str, attr: &str) -> Option<Uuid> {
    let after = &tag[tag.find(attr)? + attr.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let (quote, after) = match after.chars().next()? {
        q @ ('"' | '\'') => (q, &after[1..]),
        _ => return None,
    };
    let value = &after[..after.find(quote)?];
    Uuid::parse_str(value).ok()
}

/// „Mail-Client-Baseline"-Stylesheet für das Vorschau-Dokument (D-10, PREV-05).
///
/// BEWUSST NICHT identisch mit `.mail-html-render` aus `input.css`: Sähe die
/// Vorschau exakt wie der Editor aus, wäre der Zweck der Phase — Diskrepanzen
/// zwischen Editor-DOM und Empfänger-Sicht sichtbar zu machen (PREV-02) —
/// unterlaufen. Nackte Browser-Defaults wären andererseits Times New Roman
/// 16 px, was kein realer Mail-Client so zeigt. Die Werte orientieren sich an
/// dem, was Thunderbird/Outlook/Gmail für HTML-Mails ohne eigene Styles
/// rendern.
const MAIL_PREVIEW_BASELINE_CSS: &str = "\
html,body{margin:0;padding:0}\
body{font-family:Arial,Helvetica,sans-serif;font-size:14px;line-height:1.45;color:#222;padding:12px;word-wrap:break-word}\
p{margin:0 0 1em}\
h1{font-size:22px;margin:.5em 0}h2{font-size:18px;margin:.6em 0}h3{font-size:16px;margin:.7em 0}\
ul,ol{margin:0 0 1em;padding-left:24px}li{margin:.15em 0}\
blockquote{margin:0 0 1em;padding-left:12px;border-left:3px solid #ccc;color:#555}\
a{color:#1155cc}\
img{max-width:100%;height:auto}\
table{border-collapse:collapse}";

/// Baut das vollständige, in sich geschlossene Vorschau-Dokument (D-09, D-10).
///
/// PREV-05-Invariante: Das Ergebnis referenziert KEIN externes Stylesheet —
/// kein `<link>`, kein `@import`, keine App-CSS-Klasse. Damit ist die
/// CSS-Isolation nicht nur durch den eigenen Browsing-Context des iframes
/// gegeben (eine Browser-Garantie), sondern auch am String selbst
/// überprüfbar: Der Test beweist, dass wir sie nicht selbst unterlaufen.
///
/// Die Kodierungsangabe im Kopf ist Pflicht (Pitfall 8): Ohne sie hängt die
/// Darstellung deutscher Umlaute („Grüße") von Browser-Vererbungsverhalten ab,
/// und die Gegenmaßnahme kostet nichts.
///
/// KEIN Escaping des Ergebnis-Strings — und das ist Absicht, kein Versehen:
/// Dioxus setzt nicht-spezialbehandelte Attribute über
/// `node.setAttribute(field, value)`, also als reinen DOM-String ohne
/// HTML-Quelltext-Parsing. Die aus der MDN-Dokumentation bekannten
/// Escaping-Regeln gelten ausschließlich für den Fall, dass das Attribut im
/// HTML-Quelltext geschrieben wird; die App läuft rein clientseitig, es gibt
/// keinen SSR-Pfad. Zusätzliches Escaping würde im iframe sichtbaren
/// Escape-Text erzeugen — es wäre ein Bug, keine Härtung. Bitte nicht
/// „reparieren".
pub(crate) fn preview_srcdoc(body_html: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
         <style>{css}</style></head><body>{body}</body></html>",
        css = MAIL_PREVIEW_BASELINE_CSS,
        body = body_html
    )
}

/// Device-Rahmen mit sandboxed iframe (D-07, D-15, PREV-04, PREV-05).
///
/// Rendert in dieser Reihenfolge: (1) bei nicht-leerem `errors` den roten
/// Fehler-Block statt des iframes — ohne ihn ist ein Render-Fehler von einer
/// leeren Mail nicht unterscheidbar und der Vorstand sieht nur einen leeren
/// Rahmen (Pitfall 7); (2) im Bearbeiten-Modus nichts; (3) sonst Backdrop,
/// Label und den gerahmten iframe.
///
/// SANDBOX-SEMANTIK: Die Same-Origin-Erlaubnis ist funktional zwingend —
/// fehlt sie, serialisiert der Browser den Origin als `null`, alle
/// Subresource-Requests gelten als cross-site, und das Session-Cookie der App
/// ist `SameSite=Strict`. Die Bilder aus der Assets-Bytes-Route wären dann
/// garantiert tot. Die Erlaubnis zur Script-Ausführung wird ausdrücklich
/// NICHT gesetzt: Erst die Kombination beider Tokens erlaubt dem eingebetteten
/// Dokument, sich selbst aus der Sandbox zu nehmen. Ein Grep-Gate am Dateiende
/// nagelt das dauerhaft fest. Ebenso wenig werden Popup- oder
/// Top-Navigation-Erlaubnisse gesetzt, wodurch Links in der Vorschau nicht
/// klickbar sind — das stützt PREV-04. Scripts werden zusätzlich bereits von
/// ammonia gestrippt; das sind zwei unabhängige Schichten.
#[component]
pub fn MailPreviewFrame(
    mode: PreviewMode,
    srcdoc: String,
    #[props(default)] errors: Vec<String>,
) -> Element {
    let i18n = use_i18n();

    // Optisch identisch zum Fehler-Block in `template_preview.rs`
    // (Component-First: derselbe Zustand sieht überall gleich aus).
    if !errors.is_empty() {
        return rsx! {
            div { class: "bg-red-50 border border-red-200 rounded p-3 text-sm text-red-700",
                p { class: "font-medium mb-1", {i18n.t(Key::MailTemplateError)} }
                for err in errors.iter() {
                    p { "{err}" }
                }
            }
        };
    }

    let Some(width) = mode.width_px() else {
        return rsx! {};
    };

    let label = match mode {
        PreviewMode::Desktop => i18n.t(Key::MailEditorModeDesktopFrameLabel),
        _ => i18n.t(Key::MailEditorModeMobileFrameLabel),
    };

    rsx! {
        // D-15: grauer Backdrop, Label darüber, zentrierter Device-Rahmen.
        div { class: "bg-gray-200 p-4 flex flex-col items-center",
            p { class: "text-xs font-medium text-gray-600 mb-2", {label.clone()} }
            div { class: "border-4 border-gray-500 rounded-lg overflow-hidden bg-white shadow-lg",
                iframe {
                    // D-07 / PREV-05: ausschließlich die Same-Origin-Erlaubnis.
                    // NIEMALS die Erlaubnis zur Script-Ausführung ergänzen —
                    // siehe Sandbox-Semantik im Component-Doc oben.
                    // `sandbox` ist in dioxus-html 0.6.3 auskommentiert, daher
                    // die Quoted-Custom-Attribute-Syntax.
                    "sandbox": "allow-same-origin",
                    // D-09: roher Dokument-String, kein Escaping (siehe
                    // preview_srcdoc).
                    srcdoc: "{srcdoc}",
                    width: "{width}",
                    height: "{PREVIEW_HEIGHT_PX}",
                    // Der sichtbare Rahmen soll der Device-Rahmen sein, nicht
                    // der Browser-Default-Rand des iframes.
                    style: "display:block;border:0;background:#fff",
                    title: "{label}",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{inject_asset_src, preview_needs_fetch, preview_srcdoc, PreviewMode};
    use crate::component::mail_compose::wysiwyg_toolbar::{asset_bytes_url, image_insert_html};

    const UUID_A: &str = "3f2504e0-4f89-41d3-9a0c-0305e82c3301";
    const UUID_B: &str = "9c858901-8a57-4791-81fe-4c455b099bc9";
    /// Basis mit `/api`-Segment — genau die Deployment-Form, die Pitfall 4
    /// beschreibt (auf beta konsumiert ein Proxy dieses Segment).
    const BACKEND: &str = "https://beta.example.test/api";

    // --- Geometrie und Modus (PREV-01, D-12) ---

    #[test]
    fn preview_mode_widths_are_640_and_360() {
        assert_eq!(PreviewMode::Desktop.width_px(), Some(640));
        assert_eq!(PreviewMode::Mobile.width_px(), Some(360));
    }

    #[test]
    fn preview_mode_edit_has_no_width() {
        assert_eq!(PreviewMode::Edit.width_px(), None);
    }

    #[test]
    fn preview_mode_is_preview_only_for_device_modes() {
        assert!(!PreviewMode::Edit.is_preview());
        assert!(PreviewMode::Desktop.is_preview());
        assert!(PreviewMode::Mobile.is_preview());
    }

    // --- Request-Entscheidung (D-05) ---

    #[test]
    fn preview_needs_fetch_on_edit_to_preview() {
        assert!(preview_needs_fetch(PreviewMode::Edit, PreviewMode::Desktop));
        assert!(preview_needs_fetch(PreviewMode::Edit, PreviewMode::Mobile));
    }

    #[test]
    fn preview_needs_fetch_false_between_device_modes() {
        // Desktop <-> Mobile aendert nur die Breite; das Dokument-Attribut
        // bleibt gleich, der iframe laedt nicht neu, die Vorschau flackert
        // nicht.
        assert!(!preview_needs_fetch(
            PreviewMode::Desktop,
            PreviewMode::Mobile
        ));
        assert!(!preview_needs_fetch(
            PreviewMode::Mobile,
            PreviewMode::Desktop
        ));
        assert!(!preview_needs_fetch(
            PreviewMode::Desktop,
            PreviewMode::Edit
        ));
        assert!(!preview_needs_fetch(PreviewMode::Mobile, PreviewMode::Edit));
        assert!(!preview_needs_fetch(PreviewMode::Edit, PreviewMode::Edit));
    }

    // --- Asset-Injektion (PREV-03, D-06, Pitfall 3, Pitfall 4) ---

    #[test]
    fn inject_asset_src_adds_src_and_keeps_asset_id() {
        let html = format!(r#"<img data-genossi-asset-id="{UUID_A}">"#);
        let out = inject_asset_src(&html, BACKEND);
        assert!(
            out.contains(&format!(r#"data-genossi-asset-id="{UUID_A}""#)),
            "das data-Attribut muss erhalten bleiben (nicht ersetzt wie im Backend): {out}"
        );
        assert!(
            out.contains("src="),
            "es muss ein src ergaenzt werden: {out}"
        );
    }

    #[test]
    fn inject_asset_src_uses_backend_base_not_relative() {
        let html = format!(r#"<img data-genossi-asset-id="{UUID_A}">"#);
        let out = inject_asset_src(&html, BACKEND);
        let expected = asset_bytes_url(BACKEND, UUID_A);
        assert!(
            out.contains(&format!(r#" src="{expected}">"#)),
            "der injizierte src muss mit der backend-Basis beginnen (Pitfall 4: eine \
             relative URL umgeht config.backend und 404t auf Deployments): {out}"
        );
        assert!(expected.starts_with(BACKEND));
    }

    #[test]
    fn inject_asset_src_ignores_non_uuid_value() {
        let html = r#"<img data-genossi-asset-id="nicht-eine-uuid">"#;
        let out = inject_asset_src(html, BACKEND);
        assert_eq!(out, html, "Nicht-UUID laesst den Tag unangetastet");
        assert!(!out.contains("src="));
    }

    #[test]
    fn inject_asset_src_rejects_quote_injection_payload() {
        // So saehe das Markup aus, wenn jemand den Attributwert
        // `x" onerror="alert(1)` unterzubringen versucht: der Wert bricht aus
        // dem Attribut aus. `extract_asset_uuid` liest bis zum ersten
        // Schluss-Anfuehrungszeichen, bekommt `x`, und `Uuid::parse_str`
        // weist das ab — es wird kein src interpoliert (T-28-07).
        let html = r#"<img data-genossi-asset-id="x" onerror="alert(1)">"#;
        let out = inject_asset_src(html, BACKEND);
        assert_eq!(out, html);
        assert!(!out.contains("src="));
    }

    #[test]
    fn inject_asset_src_handles_multiple_and_duplicate_images() {
        let html = format!(r#"<img data-genossi-asset-id="{UUID_A}"><p>x</p>"#,)
            + &format!(
                r#"<img data-genossi-asset-id="{UUID_B}"><img data-genossi-asset-id="{UUID_A}">"#
            );
        let out = inject_asset_src(&html, BACKEND);
        assert_eq!(
            out.matches("src=").count(),
            3,
            "alle drei Bilder muessen ein src bekommen, auch die doppelte UUID: {out}"
        );
    }

    #[test]
    fn inject_asset_src_leaves_html_without_images_untouched() {
        // v1.4-Backward-Compat-Garantie: Alt-Templates ohne Bilder kommen
        // byte-identisch heraus.
        let html = "<p>Hallo <b>Welt</b></p><ul><li>eins</li><li>zwei</li></ul>";
        assert_eq!(inject_asset_src(html, BACKEND), html);
    }

    #[test]
    fn inject_asset_src_handles_unterminated_tag_without_panic() {
        let html = format!(r#"<p>vorher</p><img data-genossi-asset-id="{UUID_A}"#);
        let out = inject_asset_src(&html, BACKEND);
        assert_eq!(
            out, html,
            "unvollstaendiger Tag wird unveraendert angehaengt"
        );
        assert!(out.contains("<p>vorher</p>"));
    }

    #[test]
    fn inject_asset_src_preserves_surrounding_markup() {
        let html = format!(r#"<p>davor</p><img data-genossi-asset-id="{UUID_A}"><p>danach</p>"#);
        let out = inject_asset_src(&html, BACKEND);
        assert!(out.starts_with("<p>davor</p><img "), "{out}");
        assert!(out.ends_with("><p>danach</p>"), "{out}");
    }

    // --- Dokument-Aufbau und CSS-Isolation (PREV-05, D-09, D-10, Pitfall 8) ---

    #[test]
    fn srcdoc_is_self_contained_no_external_css() {
        let doc = preview_srcdoc("<p>x</p>");
        assert!(!doc.contains("<link"), "kein externes Stylesheet: {doc}");
        assert!(!doc.contains("@import"), "kein @import: {doc}");
        assert!(
            !doc.contains("tailwind"),
            "keine App-Stylesheet-Referenz: {doc}"
        );
        assert!(
            !doc.contains("mail-html-render"),
            "die Editor-CSS-Klasse darf nicht dupliziert werden (D-10): {doc}"
        );
        assert!(
            doc.contains("<style>"),
            "das Baseline-Stylesheet muss inline im Dokument stehen: {doc}"
        );
    }

    #[test]
    fn srcdoc_embeds_body_html_verbatim() {
        // Nagelt D-09 fest: es wird NICHT escaped. Dioxus setzt das Attribut
        // per setAttribute (kein HTML-Quelltext-Parsing); zusaetzliches
        // Escaping wuerde im iframe sichtbaren Escape-Text erzeugen.
        let body = r#"<p class="x">Rot &amp; Gruen</p>"#;
        let raw = "<p>A & B \"zitiert\"</p>";
        let doc = preview_srcdoc(raw);
        assert!(doc.contains(raw), "Body muss 1:1 eingebettet sein: {doc}");
        assert!(
            !doc.contains("&amp;"),
            "kein Escaping des Ampersands: {doc}"
        );
        assert!(!doc.contains("&quot;"), "kein Escaping der Quotes: {doc}");
        // Und ein bereits escapter Body bleibt ebenfalls unveraendert.
        assert!(preview_srcdoc(body).contains(body));
    }

    #[test]
    fn srcdoc_declares_utf8_charset() {
        let doc = preview_srcdoc("<p>Gr&uuml;&szlig;e</p>");
        assert!(
            doc.contains(r#"<meta charset="utf-8">"#),
            "Kodierungsangabe fehlt (Pitfall 8, deutsche Umlaute): {doc}"
        );
        let meta = doc.find("<meta").expect("meta muss existieren");
        let style = doc.find("<style>").expect("style muss existieren");
        assert!(
            meta < style,
            "die Kodierungsangabe muss vor dem Stylesheet stehen: {doc}"
        );
    }

    // --- Konsistenz der beiden Asset-URL-Verwender (D-06) ---

    #[test]
    fn asset_bytes_url_matches_image_insert_html() {
        let url = asset_bytes_url(BACKEND, UUID_A);
        let markup = image_insert_html(BACKEND, UUID_A);
        assert!(
            markup.contains(&url),
            "Editor-Insert und Vorschau muessen dieselbe URL erzeugen — sonst laufen \
             sie bei einer Route-Aenderung auseinander. url={url} markup={markup}"
        );
    }
}

// Grep-gate tests below — module-level docstring intentionally omitted so no
// literal needle bytes live in `production_region()`. The full rationale lives
// in the assertion messages, where it is read when a gate actually trips.
//
// Self-reference hazard defence, two layers (pattern from wysiwyg_editor.rs):
//   (a) slice the source BEFORE the test module marker, so the assertions'
//       own bytes are outside the search range;
//   (b) assemble every needle at runtime from fragments, so no single literal
//       byte sequence in this module could satisfy its own search even if (a)
//       failed.
#[cfg(test)]
mod grep_gate_tests {
    const FRAME_SRC: &str = include_str!("mail_preview_frame.rs");
    const TEST_MODULE_MARKER: &str = "mod grep_gate_tests";

    fn production_region() -> &'static str {
        let cutoff = FRAME_SRC.find(TEST_MODULE_MARKER).expect(
            "BUG: grep-gate test module marker not found; the marker string must appear \
             verbatim before `mod grep_gate_tests` opens",
        );
        &FRAME_SRC[..cutoff]
    }

    #[test]
    fn preview_frame_sets_sandbox_attribute() {
        let region = production_region();
        let attr_needle = format!("{q}sandbo{t}{q}", q = "\"", t = "x");
        let value_needle = format!("allow-same-{t}", t = "origin");
        assert!(
            region.contains(&attr_needle),
            "Grep gate FAILED: expected {attr_needle} on the iframe in \
             mail_preview_frame.rs (production region). Without a sandbox attribute the \
             preview document is not isolated at all."
        );
        assert!(
            region.contains(&value_needle),
            "Grep gate FAILED: expected {value_needle} in the sandbox value in \
             mail_preview_frame.rs (production region). Without the same-origin token the \
             browser serialises the iframe origin as `null`, every subresource request \
             counts as cross-site, and the app's SameSite=Strict session cookie is not \
             sent — every image in the preview would come back 401."
        );
    }

    #[test]
    fn preview_frame_never_allows_scripts() {
        let region = production_region();
        let forbidden = format!("allow-{t}", t = "scripts");
        assert!(
            !region.contains(&forbidden),
            "Grep gate FAILED: {forbidden} appeared in mail_preview_frame.rs (production \
             region). Combined with the same-origin token it lets the embedded document \
             remove the sandbox attribute from itself, which voids the isolation \
             entirely. This is a security invariant (T-28-06), not a style check — do \
             not silence this test, remove the token."
        );
    }

    #[test]
    fn preview_frame_uses_iframe_srcdoc_not_inner_html() {
        let region = production_region();
        let doc_attr = format!("srcdo{t}", t = "c: \"{");
        let raw_html_attr = format!("dangerous_inner_{t}", t = "html");
        assert!(
            region.contains(&doc_attr),
            "Grep gate FAILED: expected the iframe document attribute ({doc_attr}) in \
             mail_preview_frame.rs (production region) — not merely the preview_srcdoc \
             function name. The component must actually feed the document into an iframe."
        );
        assert!(
            !region.contains(&raw_html_attr),
            "Grep gate FAILED: {raw_html_attr} appeared in mail_preview_frame.rs \
             (production region). Falling back to raw-HTML embedding drops the nested \
             browsing context and with it the CSS isolation required by PREV-05 — the \
             preview would inherit the app stylesheet again."
        );
    }

    #[test]
    fn production_region_excludes_test_module() {
        let region = production_region();
        assert!(
            !region.contains(TEST_MODULE_MARKER),
            "BUG: production_region() slice still contains the test module marker — the \
             slice is wrong and every gate above would be a false positive."
        );
        assert!(
            region.len() < FRAME_SRC.len(),
            "BUG: production_region() covers the whole file"
        );
    }
}
