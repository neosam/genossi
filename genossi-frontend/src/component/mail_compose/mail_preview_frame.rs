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

// Genau deshalb ist in diesem Plan noch KEIN Produktions-Konsument vorhanden:
// jedes Symbol dieser Datei wird ausschließlich von den Tests am Dateiende und
// — ab Plan 28-03 — vom `WysiwygEditor` benutzt. Ohne dieses Allow meldet
// `cargo build` die Konstanten, das Enum und die pure Funktionen als tote
// Symbole. TODO (Plan 28-03): nach der Verkabelung wieder entfernen und
// prüfen, dass der Build warnungsfrei bleibt.
#![allow(dead_code)]

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
