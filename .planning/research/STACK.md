# Stack Research

**Domain:** v1.4 — HTML-Mail-Formatierung (8bit + multipart/alternative), WYSIWYG-Editor (Dioxus/WASM), HTML-Sanitization, Antrags-Datei-Upload
**Researched:** 2026-06-29
**Confidence:** HIGH

## TL;DR (für Requirements + Roadmap)

- **(a) 8bit + HTML-Mail:** KEINE neue Crate. `lettre 0.11` (bereits Dependency) kann beides. Manuelle Part-Konstruktion über `SinglePart::builder()` + `ContentTransferEncoding::EightBit` + `MultiPart::alternative()`. Convenience-Helfer `alternative_plain_html()` reicht NICHT, weil er die Kodierung nicht kontrolliert.
- **(b) WYSIWYG-Editor:** **Empfehlung: `contenteditable` + `document.execCommand` über das bereits vorhandene `web-sys`/`js-sys`-Interop** als neue Dioxus-Component. KEINE JS-Library (Quill/TipTap/Trix) einbinden. Robuste Alternative: Markdown-Toolbar + `pulldown-cmark` (Backend-Render).
- **(c) Sanitization:** **`ammonia 4.1`** (neue Backend-Dependency), serverseitig im REST/Service-Layer, Whitelist-basiert. Nicht im Frontend/WASM.
- **(d) Antrags-Upload:** KEINE neue Crate. `axum` multipart-Feature ist schon aktiv; `DocumentStorage`-Trait + der `member_document.rs`-Upload-Handler sind 1:1-Vorlage.

**Netto neue Dependencies: genau eine produktive Crate — `ammonia` (Backend).** Optional eine weitere (`html2text` oder `pulldown-cmark`) je nach Plain-Text-Fallback-Strategie.

## Recommended Stack

### Core Technologies (für die NEUEN Features)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `lettre` | 0.11 (vorhanden) | 8bit-Kodierung + `multipart/alternative` Text/HTML | Schon im Workspace (`Cargo.toml:60`); deckt MIME-Multipart und Content-Transfer-Encoding vollständig ab. Kein zusätzliches MIME-Crate nötig. |
| `ammonia` | 4.1 | Serverseitige HTML-Sanitization der Editor-Ausgabe (XSS-Defense) | Whitelist-basiert, parst mit `html5ever` exakt wie ein Browser → resistent gegen Obfuscation. De-facto-Standard für „untrusted HTML säubern" in Rust. Default-Whitelist passt fast exakt auf den Feature-Umfang (fett/kursiv/Links/Listen). |
| `web-sys` / `js-sys` | 0.3 (vorhanden) | `contenteditable`-Editor: `execCommand`, Selection, Fokus | Bereits Frontend-Dependency; ermöglicht echten WYSIWYG ohne JS-Bundle und ohne fremde Library. |
| `axum` (multipart) | 0.8.3 (vorhanden) | Datei-Upload an `Application` | Feature `multipart` ist in `Cargo.toml:36` bereits aktiviert. Identisches Muster wie `member_document.rs`/`static_document.rs`. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `pulldown-cmark` | 0.12+ | Markdown → HTML (Backend) | NUR falls Editor-Variante „Markdown-Toolbar" gewählt wird statt `contenteditable`. CommonMark-konform, schnell, klein. |
| `html2text` | 0.12+ | HTML → Plain-Text-Fallback ableiten | Optional, falls der Plain-Text-Teil von `multipart/alternative` automatisch aus dem HTML erzeugt werden soll, statt einen separaten Klartext-Body vom User zu führen. |

> **Entscheidung Plain-Text-Fallback:** Da der bestehende `MailBodyEditor` ohnehin schon einen Plain-Body führt, ist die einfachste Variante: **HTML als additives Feld, bestehender Plain-Body bleibt der Text-Teil**. Dann braucht man weder `html2text` noch `pulldown-cmark`. Nur wenn der Plain-Body wegfallen soll, wird einer der beiden Helfer relevant.

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| (keine neuen) | — | Alle Build-Tools (cargo, dx, Tailwind, Nix-Flake) unverändert. ammonia braucht `rustc ≥ 1.80` — Nix-Toolchain prüfen, sonst kein Sonderaufwand. |

## Exakte lettre-0.11-API (verifiziert gegen docs.rs 0.11)

### (a1) 8bit-Text statt quoted-printable

`SinglePart::plain(...)` (heutiger Code, `worker.rs:656`) wählt die Kodierung automatisch → quoted-printable → sichtbare `=`-Soft-Breaks. Für 8bit MUSS die Kodierung explizit gesetzt werden über den Builder:

```rust
use lettre::message::header::{ContentType, ContentTransferEncoding};
use lettre::message::SinglePart;

let text_part = SinglePart::builder()
    .header(ContentType::TEXT_PLAIN)            // text/plain; charset=utf-8
    .header(ContentTransferEncoding::EightBit)  // <- ersetzt quoted-printable
    .body(body.to_string());
```

- **Verifiziert:** `ContentTransferEncoding` ist ein Enum mit `SevenBit | QuotedPrintable | Base64 | EightBit | Binary`. `EightBit` existiert.
- **Verifiziert:** `SinglePart` hat KEINE Convenience-Konstruktoren `eight_bit()`/`seven_bit()` in 0.11 — nur `plain()`, `html()`, `builder()`. (Ältere Code-Beispiele im Netz mit `SinglePart::eight_bit()` beziehen sich auf lettre 0.10.) → **Builder-Form ist der einzige korrekte Weg.**
- `ContentType::TEXT_PLAIN` setzt automatisch `charset=utf-8` — der heutige Grund für `SinglePart::plain` (GMX-Umlaut-Bug, Kommentar `worker.rs:653-655`) bleibt damit erfüllt.

### (a2) HTML-Mail als multipart/alternative

```rust
use lettre::message::{MultiPart, SinglePart};
use lettre::message::header::{ContentType, ContentTransferEncoding};

let text_part = SinglePart::builder()
    .header(ContentType::TEXT_PLAIN)
    .header(ContentTransferEncoding::EightBit)
    .body(plain_body.to_string());

let html_part = SinglePart::builder()
    .header(ContentType::TEXT_HTML)             // text/html; charset=utf-8
    .header(ContentTransferEncoding::EightBit)
    .body(sanitized_html.to_string());          // <- NACH ammonia::clean()

// Reihenfolge: am wenigsten bevorzugt zuerst, bevorzugt zuletzt → Plain, dann HTML
let alternative = MultiPart::alternative()
    .singlepart(text_part)
    .singlepart(html_part);
```

> **Bewusst NICHT `MultiPart::alternative_plain_html(plain, html)` verwenden:** Der Convenience-Helfer baut die Parts intern mit Default-Kodierung (quoted-printable) → reaktiviert genau das `=`-Problem. Manuelle Konstruktion ist Pflicht, sobald 8bit gewünscht ist.

### (a3) Verschachtelung mit Attachments (Antrags-Anhang + HTML)

Wenn HTML UND Dateianhänge zusammenkommen (heute `MultiPart::mixed()` in `worker.rs:675`), gilt: `mixed` außen, `alternative` als erster Part, Attachments danach:

```rust
let mut mixed = MultiPart::mixed().multipart(alternative);  // statt .singlepart(text_part)
for att in attachments {
    mixed = mixed.singlepart(attachment); // unverändert wie heute
}
```

MIME-Regel (verifiziert): Äußere multipart-Parts dürfen nur `7bit`/`8bit`/`binary` tragen; die eigentliche Kodierung sitzt auf den Blatt-Parts. lettre setzt das korrekt; Attachments nutzen weiter `Attachment::new(...).body(bytes, content_type)` (base64).

### SMTP-Caveat zu 8bit (WICHTIG für Roadmap-Risiko)

`EightBit` setzt voraus, dass der SMTP-Server die ESMTP-Extension **`8BITMIME`** ankündigt. Praktisch unterstützt das jeder moderne Mailserver; lettres SMTP-Transport handelt `BODY=8BITMIME` aus, wenn der Server es announced. **Vor dem Rollout gegen den produktiven SMTP-Server (`shifty.nebenan-unverpackt.de`-Konfig) verifizieren** — sonst theoretisch Risiko mangelhafter Zustellung. Mitigation, falls nötig: Fallback auf `QuotedPrintable` konfigurierbar machen. (Niedriges Restrisiko, aber explizit in Pitfalls aufnehmen.)

### Betroffene Stellen (geteilter Helfer)

Heute existiert die Body-Konstruktion **dreimal**: `worker.rs:656` (`send_mail_for_recipient`), `service.rs:436` (`send_test_mail`, nutzt `.body()`), `service.rs:479` (`send_test_mail_with_body`, nutzt `.body()`). Die letzten beiden nutzen `Message::builder().body()` → setzen weder charset noch 8bit. → **Ein geteilter Helfer** `build_mail_body(plain, html: Option<&str>) -> MultiPart/SinglePart` in `genossi_mail`, von allen drei Pfaden genutzt. Deckt sich mit dem PROJECT.md-Ziel „geteilter Helfer in `genossi_mail`".

## WYSIWYG-Editor: Optionen-Vergleich (Dioxus 0.6 / WASM)

| Option | Neue Deps | Echter WYSIWYG | Dioxus-Integration | Output-Sauberkeit | Bewertung |
|--------|-----------|----------------|--------------------|--------------------|-----------|
| **A. `contenteditable` + `execCommand`** (web-sys) | keine | ja | mittel (uncontrolled `div`, `onmounted`-Ref) | mittel (browserabhängig) → ammonia normalisiert | **EMPFOHLEN** |
| B. JS-Lib (Quill/TipTap/Trix) via wasm-bindgen | JS-Bundle + Glue | ja | schwer (Mount/Lifecycle, Asset-Mgmt, manganis) | gut | Overkill |
| C. Pure-Rust/Dioxus-WYSIWYG | — | — | — | — | existiert nicht reif; sehr hoher Aufwand |
| D. Markdown-Toolbar + `pulldown-cmark` | `pulldown-cmark` (Backend) | nein (Quelle sichtbar) | einfach (`textarea`) | sehr gut (deterministisch) | **starke Alternative / Fallback** |

### Empfehlung: Option A (contenteditable + execCommand)

**Warum:**
- **Null neue Frontend-Deps, kein JS-Bundle.** `web-sys`/`js-sys` sind bereits vorhanden; passt zum Component-First-Prinzip (neue Component `mail_compose/rich_body_editor.rs`).
- **Echter WYSIWYG** — genau das Ziel „Vorstände ohne HTML-Kenntnisse". Board-Mitglieder sehen Fett/Kursiv direkt.
- `document.execCommand("bold"|"italic"|"createLink"|"insertUnorderedList"|"insertOrderedList")` ist zwar „deprecated", aber in allen Browsern stabil unterstützt — für diesen kleinen Funktionsumfang die pragmatischste Lösung.
- **Schmutziges/uneinheitliches HTML ist unkritisch**, weil ammonia serverseitig ohnehin normalisiert und whitelistet.

**Konkrete Integrationspunkte / Pitfalls (für Plan-Phase):**
- web-sys-Features ergänzen: `Document` (vorhanden) plus `exec_command`/`query_command_state` — `exec_command` liegt auf `Document`; ggf. `"Selection"`, `"Range"` zur Feature-Liste ergänzen (Cursor/Link-Handling).
- **`styleWithCSS` erzwingen:** Vor dem Editieren `document.exec_command("styleWithCSS", false, "false")` aufrufen, damit Browser semantische Tags (`<b>`,`<i>`) statt `<span style="font-weight:bold">` erzeugen. **Sonst entfernt ammonia (das `style`-Attribute per Default strippt) die Formatierung wieder!** Das ist der wichtigste Stolperstein.
- `contenteditable`-`div` ist „uncontrolled": Wert NICHT per `value:`-Binding zurückschreiben (Cursor-Reset). Statt `oninput` den `innerHTML` via Ref/`onmounted` auslesen und an `on_change` geben (analog zur bestehenden `MailBodyEditor`-Signatur `on_change: EventHandler<String>`).
- Dioxus-Reload-Bug beachten (Memory: `r#type: "button"`): Toolbar-Buttons als `button { r#type: "button", onclick: ... }`, niemals form-submit.
- Component-First: Toolbar als eigene Sub-Component; `MailBodyEditor` und Inbox-`reply_form.rs` sollen denselben Editor teilen (heute beide Plain-`textarea`).

**Fallback (Option D)** falls contenteditable in der UAT zu fummelig/inkonsistent wirkt: Toolbar-Buttons fügen Markdown-Syntax in eine `textarea` ein, Backend rendert via `pulldown-cmark` → HTML → ammonia. Deterministisch und testbar, aber Quelltext bleibt sichtbar (weniger „WYSIWYG").

## HTML-Sanitization: ammonia (Detail)

- **Crate:** `ammonia = "4.1"` (Backend, z. B. in `genossi_mail` oder einem geteilten Service-Crate).
- **Wo:** **Serverseitig, vor dem Versand UND vor jeder Anzeige.** Niemals nur im Frontend (umgehbar). Idealerweise im Service-Layer (`genossi_mail`), damit auch der Test-Mail-Pfad (`service.rs`) abgedeckt ist.
- **Default-Whitelist passt:** erlaubt u. a. `a, b, i, em, strong, p, br, ul, ol, li, blockquote, code` und strippt `script`, `style`, Event-Handler (`onclick` …), gefährliche URL-Schemata. Für Links setzt ammonia per Default `rel="noopener noreferrer"` und beschränkt `href` auf sichere Schemata (`http`, `https`, `mailto`).
- **Anpassung nötig:** ammonia entfernt per Default das `style`-Attribut → siehe `styleWithCSS`-Pitfall oben. Falls Inline-CSS für Mail-Clients gebraucht wird, müsste die Whitelist erweitert werden (`Builder::add_tag_attributes`) — für den Funktionsumfang fett/kursiv/Links/Listen ist das **nicht** nötig, semantische Tags genügen.
- `html5ever`-basiert; zieht `markup5ever`/`tendril` als transitive Deps. Reiner Backend-Code, **nicht** in WASM kompilieren.

## Antrags-Datei-Upload (Bestätigung)

**Nichts Neues hinzuzufügen.** Alles vorhanden:
- `axum` mit `features = ["multipart"]` ist aktiv (`Cargo.toml:36`).
- `DocumentStorage`-Trait (`genossi_service/src/document_storage.rs:24`, `save`/`load`) + Filesystem-Impl existieren.
- **Direkte Vorlage:** `genossi_rest/src/member_document.rs:115` `upload_document` (Multipart-Felder lesen, Extension-Whitelist, Client-MIME ignorieren, Server leitet MIME aus Extension ab, `DefaultBodyLimit`).
- Auto-Übernahme beim Aktivieren: `Application` und `MemberDocument` sind beide auditpflichtig → **Audit-Macros Pflicht** (`audited_create!`/`audited_update!`), atomar in einer Tx (Muster wie v1.1/v1.2-Cascades).

## Installation

```toml
# genossi_mail/Cargo.toml (oder geteiltes Service-Crate) — NEU
ammonia = "4.1"

# Optional, NUR bei Markdown-Variante (Editor-Option D):
# pulldown-cmark = "0.12"
# Optional, NUR falls Plain-Text-Fallback aus HTML abgeleitet wird:
# html2text = "0.12"

# lettre: KEINE Änderung — 0.11 bereits vorhanden, Features ausreichend
# axum multipart: KEINE Änderung — Feature bereits aktiv
# Frontend: KEINE neue Crate — web-sys/js-sys vorhanden
#   ggf. web-sys-Feature-Liste um "Selection","Range" ergänzen (exec_command ist auf Document)
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `lettre` manuelle Parts (8bit) | `MultiPart::alternative_plain_html()` | Nie für dieses Ziel — reaktiviert quoted-printable. Nur wenn 8bit egal wäre. |
| contenteditable + execCommand | JS-WYSIWYG (Quill/TipTap) | Erst wenn Tabellen/Bilder/Embeds/komplexe Formatierung gefordert sind — weit außerhalb v1.4-Scope. |
| contenteditable + execCommand | Markdown-Toolbar + `pulldown-cmark` | Wenn UAT zeigt, dass contenteditable zu inkonsistent ist, oder deterministischer/testbarer Output gewünscht. |
| `ammonia` | eigene Regex/String-Filter | Nie — Regex-HTML-Filter sind notorisch umgehbar (XSS). |
| bestehender Plain-Body als Text-Teil | `html2text`/`pulldown-cmark`-abgeleiteter Text | Nur wenn der separate Plain-Body aus der UI entfernt werden soll. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| JS-WYSIWYG-Library (Quill/TipTap/Trix) via wasm-bindgen | JS-Bundle + Asset-Management (manganis) + Dioxus-Mount/Lifecycle-Reibung; Wartungslast für simplen Bedarf | contenteditable + execCommand (web-sys) |
| Eigenes MIME-/Multipart-Crate | lettre 0.11 deckt 8bit + multipart/alternative + Attachments komplett ab | lettre `SinglePart::builder()` / `MultiPart` |
| `MultiPart::alternative_plain_html()` (für 8bit-Ziel) | Erzeugt Parts mit quoted-printable → `=`-Soft-Breaks zurück | Manuelle Part-Konstruktion mit `ContentTransferEncoding::EightBit` |
| Sanitization nur im Frontend/WASM | Clientseitig umgehbar; ammonia in WASM zudem schwergewichtig | `ammonia` serverseitig im Service-Layer |
| `SinglePart::eight_bit()` | Existiert in 0.11 NICHT (war lettre 0.10) | `SinglePart::builder().header(ContentTransferEncoding::EightBit)` |
| Neue Upload-/Storage-Abstraktion | `DocumentStorage` + axum-multipart bereits etabliert | `member_document.rs`-Upload-Muster wiederverwenden |
| Direkte DAO-Calls für Antrags-Datei/Member-Übernahme | Application/MemberDocument sind auditpflichtig | `audited_create!`/`audited_update!` in einer Tx |

## Stack Patterns by Variant

**Wenn Plain-Body in der UI erhalten bleibt (empfohlen, kleinster Eingriff):**
- HTML ist additiv; bestehender Plain-Body = Text-Teil von `multipart/alternative`
- Keine `html2text`/`pulldown-cmark`-Abhängigkeit nötig

**Wenn nur noch HTML editiert wird (Plain entfällt):**
- Text-Teil muss generiert werden → `html2text` (HTML→Text) ODER bei Markdown-Variante der Markdown-Quelltext als Text-Teil

**Wenn contenteditable in UAT durchfällt:**
- Auf Markdown-Toolbar + `pulldown-cmark` wechseln; ammonia bleibt unverändert nachgeschaltet

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `ammonia@4.1` | `rustc ≥ 1.80` | Nix-Toolchain-Version prüfen (Memory: erst `/nix/store` checken, bevor „fehlt" gemeldet wird). Zieht `html5ever`/`markup5ever`/`tendril`. |
| `lettre@0.11` | vorhandene Features `builder` | `MultiPart`/`SinglePart`/`header::*` sind alle im `builder`-Feature; kein Feature-Update nötig. |
| `web-sys@0.3` | `exec_command` auf `Document` | Feature `Document` vorhanden; ggf. `"Selection"`,`"Range"` für Cursor/Link-Handling ergänzen. |
| `axum@0.8.3` | Feature `multipart` | Bereits aktiv; `DefaultBodyLimit` für Upload-Größe wie in bestehenden Handlern. |

## Sources

- docs.rs `lettre 0.11` — `ContentTransferEncoding` (EightBit verifiziert), `SinglePart` (kein `eight_bit()`-Helfer, nur `plain/html/builder`) — HIGH
- crates.io / lib.rs `ammonia` — Version 4.1, html5ever-basiert, Whitelist-Verhalten — HIGH
- RFC 1341/1521 (Content-Transfer-Encoding-Regel für multipart: nur 7bit/8bit/binary außen) — HIGH
- Codebase: `genossi_mail/src/worker.rs:627-720`, `service.rs:415-488`, `genossi_rest/src/member_document.rs:115`, `genossi_service/src/document_storage.rs:24`, `genossi-frontend/src/component/mail_compose/body_editor.rs`, `Cargo.toml:36,60`, frontend `Cargo.toml` — HIGH
- MDN (Kenntnis): `document.execCommand` deprecated aber browserweit unterstützt; `styleWithCSS`-Verhalten — MEDIUM

---
*Stack research for: v1.4 Mail-Formatierung & Antrags-Dokumente*
*Researched: 2026-06-29*
