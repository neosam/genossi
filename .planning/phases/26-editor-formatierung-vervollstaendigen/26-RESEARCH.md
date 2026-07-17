# Phase 26: Editor-Formatierung vervollständigen - Research

**Researched:** 2026-07-17
**Domain:** Test/Verifikation für Dioxus-WYSIWYG-Editor (execCommand + ammonia), Rust-Source-Invariant-Tests, E2E-Round-Trip, UAT-Nachhol
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (H1-Button bleibt):** H1 heute in `wysiwyg_toolbar.rs` (`formatBlock <h1>`). Spec (EDIT-08) fordert nur H2/H3 — aber H1 ist keine Regression, ammonia lässt `<h1>..<h6>` per Default durch, UAT-Phase 24 zählte 13 Buttons inkl. H1. Nicht entfernen. Round-Trip-Test deckt zusätzlich `<h1>` mit ab.
- **D-02 (Grep-Gate als Rust-`include_str!`-Test):** Neuer Test in `genossi-frontend` (Ziel: inline in `component/mail_compose/mod.rs` ODER `component/mail_compose/wysiwyg_editor.rs`), der `include_str!("wysiwyg_editor.rs")` lädt und asserted: (1) `contains("exec_command_bool(&doc, \"styleWithCSS\", false)")` — Guard existiert exakt; (2) `contains("evt.prevent_default()")` im Kontext der `onpaste`-Closure (Substring-Nachbarschaft: „onpaste" gefolgt von „prevent_default" innerhalb weniger Zeilen). Läuft mit `cargo test`. Kein Husky-Hook, keine CI-Infra. Versioniert im Code.
- **D-03 (Round-Trip via Unit-Test + E2E-Test):** (a) Unit-Tests in `genossi_mail/src/sanitize.rs`: `sanitize_preserves_unordered_list`, `sanitize_preserves_ordered_list`, `sanitize_preserves_headings_h1_h2_h3`. (b) EIN E2E-Test in `genossi_bin/tests/e2e_tests.rs` analog zu `bulk_mail_body_html_sanitized_and_persisted` (Zeile ~14655): `template_body_html_with_lists_and_headings_round_trips` — POST Template mit `body_html: "<h2>Titel</h2><ul><li>a</li></ul><ol><li>b</li></ol><h3>Sub</h3>"`, GET zurück, assert alle Tags/Text erhalten.
- **D-04 (Backward-Compat aus Nicht-Änderung):** `sanitize.rs` wird in Phase 26 **nicht angefasst** (keine neuen `Builder`-Regeln, keine Attribut-Änderungen). Damit ist Success-Criterion #5 („bestehende v1.4-Templates rendern byte-identisch") als Konsequenz erfüllt — keine dedizierten Snapshot-Tests. Ausstiegs-Klausel: wenn Unit-Tests aus D-03 unerwartet failen, muss der Planner Snapshot-Test oder ammonia-Config-Anpassung nachschieben.
- **D-05 (UAT-Checklist = Copy von Phase 24 + neue Steps):** `26-UAT-CHECKLIST.md` = Copy von `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md` (Steps 1-12) + 4 neue Steps 13-16 für UL/OL/H2/H3 (Editor → Toolbar-Klick → innerHTML in DevTools → Save → Reload → innerHTML byte-identisch). Setup-Sektion auf 2026-07 aktualisieren (Skill `run-rust-backend-and-frontend`). Ein Sign-Off-Termin für alle 16 Punkte.
- **D-06 (UAT ist Ship-Gate, nicht Merge-Gate):** Wir benutzen jj — jj-Changes sind eh WIP-detached, kein PR-Gate. UAT MUSS vor v1.5-Milestone-Close (`/gsd-complete-milestone`) abgehakt sein — die Milestone-Audit-Skill prüft das. Innerhalb der Phase ist Code-fertig = Code-Reviews + automatische Tests grün. UAT läuft parallel/nachgelagert. Phase-VERIFICATION.md dokumentiert UAT-Status als „Pending Sign-Off" wenn Smoke noch nicht durch ist; kein Fail-Gate innerhalb der Phase.

### Claude's Discretion

- Exakte Datei/Modul-Location für den Grep-Gate-Test (inline in `wysiwyg_editor.rs::mod tests` vs. `mail_compose/mod.rs` vs. separater `wysiwyg_source_invariants.rs`).
- Regex/Substring-Nachbarschafts-Heuristik im Grep-Gate (wie eng darf „prevent_default" bei „onpaste" stehen?).
- Ammonia-Normalisierungs-Toleranz im E2E-Test (byte-exakt vs. „enthält alle Tags").
- Icon-/Label-Politur beim UAT (falls sichtbar auffällt).
- Ob H2-Icon größer als H3-Icon dargestellt wird (kosmetisch, aktuell alle gleich).

### Deferred Ideas (OUT OF SCOPE)

- Mehr Toolbar-Buttons (Text-Align, Text-Color, Font-Size, Font-Family) — eigene Zukunfts-Phase, nicht v1.5.
- Icon-Politur (SVG-Icons statt Text „B/I/U/S/1./•/H1/H2/H3/¶/❝/🔗/⊘") — kosmetisch, nur wenn beim UAT als störend auffällt.
- Snapshot-Test für existierende v1.4-Templates (fallback für D-04, falls Unit-Tests unerwartet failen).
- H2 visuell größer als H3 in der Toolbar — kosmetisch.
- Bild-Upload → Phase 27 (v1.5).
- Desktop/Mobile-Preview → Phase 28 (v1.5).

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EDIT-06 | Vorstand kann im WYSIWYG-Editor **ungeordnete Listen** (`<ul><li>`) via Toolbar-Button einfügen; überlebt Save/Reload + ammonia. | Toolbar-Button `insertUnorderedList` ist BEREITS in `wysiwyg_toolbar.rs:112-125` verdrahtet. Ammonia-Default erlaubt `<ul>` + `<li>` [VERIFIED: docs.rs/ammonia]. Fehlt: Unit-Test (D-03a) + E2E-Round-Trip (D-03b) + UAT-Step 13 (D-05). |
| EDIT-07 | Vorstand kann im WYSIWYG-Editor **geordnete Listen** (`<ol><li>`) via Toolbar-Button einfügen; überlebt Save/Reload + ammonia. | Toolbar-Button `insertOrderedList` ist BEREITS in `wysiwyg_toolbar.rs:127-140` verdrahtet. Ammonia-Default erlaubt `<ol>` (+ `start`-Attribut) + `<li>` [VERIFIED: docs.rs/ammonia]. Fehlt: Unit-Test (D-03a) + E2E-Round-Trip (D-03b) + UAT-Step 14 (D-05). |
| EDIT-08 | Vorstand kann im WYSIWYG-Editor **Überschriften H2/H3** via Toolbar-Button einfügen; überlebt Save/Reload + ammonia. | Toolbar-Buttons `formatBlock <h2>` (`wysiwyg_toolbar.rs:157-170`) und `formatBlock <h3>` (`:172-185`) sind BEREITS verdrahtet. Ammonia-Default erlaubt `<h1>..<h6>` [VERIFIED: docs.rs/ammonia]. Fehlt: Unit-Test inkl. H1 (D-01+D-03a) + E2E-Round-Trip (D-03b) + UAT-Steps 15+16 (D-05). |
| EDIT-09 | Toolbar-Buttons nutzen `execCommand` konsistent mit `styleWithCSS=false`; Grep-Gate analog EDIT-01/02. | `styleWithCSS=false`-Guard existiert in `wysiwyg_editor.rs:80` (`onmounted`), `evt.prevent_default()` in `onpaste`-Closure `wysiwyg_editor.rs:92`. Fehlt: mechanischer Grep-Gate (D-02) — Rust-`include_str!`-Test der beide Stellen substring-nachweist. |
| EDIT-10 | v1.4-Phase-24-UAT-Checklist wird im gleichen Zug abgehakt (Bold + Paste-Plain + Modal-Link-Dialog + neue Formatierungen als kombinierter Vorstand-Smoke-Test). | Kopiere `24-UAT-CHECKLIST.md` als `26-UAT-CHECKLIST.md`, aktualisiere Setup auf 2026-07 (Skill `run-rust-backend-and-frontend`), hänge Steps 13-16 für UL/OL/H2/H3 an. Ein Sign-Off-Termin (D-05). UAT ist Ship-Gate vor v1.5-Milestone-Close, nicht Merge-Gate innerhalb Phase (D-06). |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Rust-Backend + Dioxus-WASM-Frontend** — keine Sprachwechsel, keine DB-Wechsel im Scope.
- **Layered DAO/Service/REST** — für Phase 26 nicht relevant (reine Test-/UAT-Phase, kein neuer Layer-Code).
- **Component-First (Frontend)** — für Phase 26 nicht relevant (kein neues UI-Component; die Toolbar existiert bereits).
- **jj statt git** (Memory `feedback_use_jj_not_git`) — Commits via `jj commit -m …`, log via `jj log`, push via `jj git push`.
- **`r#type: "button"`** (Memory `feedback_dioxus_button_type`) — alle Toolbar-Buttons haben dies bereits.
- **Enum statt Boolean** (Memory `feedback_enum_not_boolean`) — keine bool-Toggles in Sanitize-Config; ammonia-Default bleibt (D-04).
- **Discuss/Feedback im Chat** (Memory `feedback_discuss_in_chat_form`) — keine AskUserQuestion-Popups im Downstream.
- **Component-First frontend / i18n zweisprachig** — die Keys `MailEditorUnorderedList`, `MailEditorOrderedList`, `MailEditorHeading1..3` existieren bereits in beiden Locales (`de.rs:224-228`) [VERIFIED].
- **Immer Tests für Änderungen** (globales `~/.claude/CLAUDE.md`) — deckt sich mit dem Phase-Ziel (Phase 26 IST eine Test-Phase).

## Summary

Phase 26 ist eine **reine Test-, Verifikations- und UAT-Phase** — kein Produktionscode wird bewusst geändert. Die Toolbar-Buttons für UL/OL/H1/H2/H3 sind bereits in `wysiwyg_toolbar.rs` seit Phase 24 verdrahtet, der `styleWithCSS=false`-Guard und `evt.prevent_default()` im `onpaste`-Handler stehen in `wysiwyg_editor.rs`, und `ammonia` erlaubt in seiner Default-Whitelist alle Ziel-Tags (`ul`, `ol`, `li`, `h1..h6`). Was fehlt sind (a) **Beweise** dass die Round-Trip-Kette Editor→POST→sanitize→SQLite→GET→Editor die neuen Elemente unbeschädigt durchreicht, (b) ein **mechanischer Grep-Gate** der die zwei kritischen Invarianten (`styleWithCSS=false` beim Mount, `prevent_default()` im Paste-Handler) vor stiller Regression schützt, und (c) der **nachgeholte Vorstand-Smoke-Test** aus Phase 24 (12 alte Punkte + 4 neue).

Kritische Fakten aus dem Scouting: `include_str!("wysiwyg_editor.rs")` wird im Projekt bisher NICHT für Source-Invariant-Tests verwendet — das Muster ist neu (D-02). Es funktioniert reliably nur, wenn die überwachten Substrings im Editor-Quelltext exakt in der geforderten Form vorliegen (mit Anführungszeichen und Klammern), sonst zerstört jedes `cargo fmt`-Whitespace-Reflow den Test. Das `genossi-frontend` crate ist ein **Binary** (`main.rs`, kein `[lib]`) — Tests laufen als `#[cfg(test)] mod tests` inline im überwachten Modul via `cargo test --bin genossi-frontend`, ein `tests/`-Integration-Test-Verzeichnis existiert nicht und würde zusätzliche Cargo-Konfiguration erfordern (also inline bleiben). Die Baseline-Zahlen sind: 252 Tests in `genossi_mail --lib`, 308 in `genossi_bin --test e2e_tests` (1 Pre-existing failure aus Phase 22 = `test_mail_preview_repayment_no_entries_does_not_default_to_one`, dokumentiert in STATE.md — NICHT Phase-26-Regression), 284+ Tests in `genossi-frontend`.

**Primary recommendation:** (1) 3 Unit-Tests in `genossi_mail/src/sanitize.rs` mod tests (UL / OL / H1+H2+H3 Round-Trip durch `sanitize_html`). (2) 1 E2E-Test in `genossi_bin/tests/e2e_tests.rs` (`create_template_body_html_lists_headings_round_trips`, Muster kopiert aus `create_template_body_html_sanitized` Zeile 14797 — der ist strukturell näher am Round-Trip als `bulk_mail_body_html_sanitized_and_persisted`, weil Templates POST/GET direkt spiegeln und keinen `MailJob`-Umweg brauchen). (3) 2 Source-Invariant-Tests inline in `wysiwyg_editor.rs::mod tests` (nicht separate Datei) — halbiert die Pfad-Fragilität von `include_str!`, weil der Test direkt in derselben Datei steht wie das überwachte File. (4) UAT-Copy von Phase 24 mit 4 Anhang-Steps.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Rendering von UL/OL/H1/H2/H3 im contenteditable | Browser / Client | — | `execCommand` läuft nativ im Browser; kein Server-Beitrag beim Editieren |
| Sanitization (ammonia-Whitelist) von UL/OL/Headings | API / Backend | — | Grenze zwischen User-HTML und Persistenz; sicherheitsrelevant |
| Round-Trip-Beweis (Save→GET) | API / Backend | Database / Storage | E2E-Test über `/api/mail/templates` — Backend + SQLite gemeinsam |
| Grep-Gate für Source-Invarianten | Build / Test | — | Läuft im Rust-Test-Runner nativ; kein Runtime-Beitrag |
| UAT-Vorstand-Smoke | Browser / Client | Human Verifier | Manuelle Browser-Session gegen Backend + Frontend Dev-Server |
| i18n-Keys UL/OL/H1/H2/H3 | Browser / Client | — | Bereits in `de.rs:224-228` + `en.rs` vorhanden [VERIFIED] |

## Standard Stack

Phase 26 fügt **keine neuen Dependencies** hinzu — alle benötigten Werkzeuge sind bereits im Workspace.

### Core (verwendet, nicht neu)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ammonia` | 4.x | HTML-Sanitization Backend | Bereits in `genossi_mail` (Cargo.toml Zeile 31 `ammonia = "4"`); Default-Whitelist erlaubt UL/OL/LI/H1-H6 [VERIFIED: docs.rs/ammonia via WebFetch 2026-07-17] |
| `reqwest` | 0.11/0.12 | HTTP-Client für E2E-Tests | Bereits im Muster `bulk_mail_body_html_sanitized_and_persisted` und `create_template_body_html_sanitized` in e2e_tests.rs verwendet |
| `serde_json` | 1.0 | JSON-Bau für Test-Payloads | Bereits im e2e_tests.rs-Muster (Zeile 14676-14686) |
| `tokio` | 1.35 | `#[tokio::test]`-Runner | Bereits Standard für alle E2E-Tests |
| `js-sys` / `wasm-bindgen` | 0.3 / 0.2.97 | `exec_command_*`-Helpers in `js.rs:174-237` | Bereits verwendet, `#[allow(dead_code)]` aufgehoben durch Toolbar-Calls |

### Supporting (verwendet, nicht neu)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `include_str!` (Rust std macro) | std | Source-Datei als String für Grep-Gate laden | Für D-02 im Grep-Gate-Test |
| `sqlx` | 0.8 | SQLite-Query im E2E-Test-Server | Bereits `test_server.rs` (setup()-Helper) |

**Installation:** Keine neuen Pakete zu installieren. `cargo test` und `dx serve` laufen mit dem existierenden Toolchain (Nix-Flake).

**Version verification skip-Rechtfertigung:** Alle relevanten Deps sind bereits in `Cargo.lock` gepinnt und Phase 24 hat sie in Produktion bewiesen. `ammonia = "4"` (Semver `^4`) im Workspace; laut docs.rs aktuell Version 4.x-Serie stabil.

## Package Legitimacy Audit

Nicht anwendbar — Phase 26 fügt **keine** neuen Dependencies hinzu (weder Backend noch Frontend). Alle Werkzeuge (`ammonia`, `reqwest`, `serde_json`, `tokio`, `js-sys`, `wasm-bindgen`, `sqlx`) sind bereits im Workspace verankert und wurden in Phase 22-25 in Produktion verwendet.

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ VORSTAND (Browser Session, UAT-Smoke)                               │
│                                                                     │
│   1. Klick auf UL/OL/H2/H3-Toolbar-Button                          │
│   2. execCommand mutates contenteditable innerHTML                  │
│   3. sync_from_dom() feuert on_change (plain, html)                │
│   4. Send/Save → POST /api/mail/templates {body_html: "…<ul>…"}    │
└─────────────────────────────┬───────────────────────────────────────┘
                              │ HTTPS JSON
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ BACKEND (genossi_bin Axum server)                                   │
│                                                                     │
│   POST /api/mail/templates                                          │
│      → MailTemplateService::create()                                │
│      → sanitize_html(body_html)  ← ammonia default whitelist        │
│      → INSERT INTO mail_template ... (SQLite)                       │
│                                                                     │
│   GET /api/mail/templates/{id}                                      │
│      → MailTemplateService::get()                                   │
│      → SELECT ... FROM mail_template WHERE id = ?                   │
│      → Return MailTemplateTO with body_html                         │
└─────────────────────────────┬───────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ TEST-EBENEN (parallel, decoupled)                                   │
│                                                                     │
│   Ebene 1: Unit-Test in genossi_mail/src/sanitize.rs                │
│      → sanitize_html("<ul><li>a</li></ul>") == "<ul><li>a</li></ul>"│
│      → Beweist ammonia-Whitelist bei UL/OL/H1-H3                    │
│                                                                     │
│   Ebene 2: E2E-Test in genossi_bin/tests/e2e_tests.rs               │
│      → POST /api/mail/templates {body_html: "<h2>…</h2><ul>…"}      │
│      → GET  /api/mail/templates/{id}                                │
│      → assert body_html enthält <h2>, <ul>, <li>, <ol>, <h3>        │
│      → Beweist: Backend + SQLite reicht unverändert durch           │
│                                                                     │
│   Ebene 3: Grep-Gate in genossi-frontend                            │
│      → include_str!("wysiwyg_editor.rs")                            │
│      → assert!(src.contains("styleWithCSS")) ...                    │
│      → Beweist: Source-Invariant überlebt Refactoring               │
│                                                                     │
│   Ebene 4: 26-UAT-CHECKLIST.md (manuell durch Vorstand)             │
│      → Browser gegen dx serve :8080 + backend :3000 mock_auth       │
│      → 16 Steps: 12 aus Phase 24 + 4 neue (13-16)                   │
└─────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

Keine neuen Dateien nötig. Alle Änderungen in existierenden Dateien:

```
genossi_mail/src/
└── sanitize.rs                    # +3 Tests (mod tests)

genossi_bin/tests/
└── e2e_tests.rs                   # +1 Test (append)

genossi-frontend/src/component/mail_compose/
└── wysiwyg_editor.rs              # +2 Source-Invariant-Tests (mod tests)

.planning/phases/26-editor-formatierung-vervollstaendigen/
└── 26-UAT-CHECKLIST.md            # NEU (Copy von 24 + 4 Steps)
```

### Pattern 1: Ammonia Round-Trip Unit-Test

**What:** Einfacher `#[test]`-Block, der `sanitize_html(input)` aufruft und die Ausgabe auf erwartete Tags prüft.

**When to use:** Für JEDE neue HTML-Struktur, die durch die Sanitize-Grenze soll (hier: UL/OL/H1/H2/H3).

**Example:** (Muster gespiegelt aus `sanitize_preserves_jinja_placeholder_in_text_content`, `sanitize.rs:94-108`)

```rust
// In genossi_mail/src/sanitize.rs::mod tests
#[test]
fn sanitize_preserves_unordered_list() {
    let input = "<ul><li>a</li><li>b</li></ul>";
    let output = sanitize_html(input);
    assert!(
        output.contains("<ul>"),
        "expected <ul> to survive, got: {output}"
    );
    assert!(
        output.contains("<li>a</li>") && output.contains("<li>b</li>"),
        "expected <li> items to survive with content, got: {output}"
    );
}

#[test]
fn sanitize_preserves_ordered_list() {
    let input = "<ol><li>1</li><li>2</li></ol>";
    let output = sanitize_html(input);
    assert!(output.contains("<ol>"), "expected <ol>, got: {output}");
    assert!(output.contains("<li>1</li>"), "expected content, got: {output}");
}

#[test]
fn sanitize_preserves_headings_h1_h2_h3() {
    let input = "<h1>A</h1><h2>B</h2><h3>C</h3>";
    let output = sanitize_html(input);
    for tag in ["<h1>", "</h1>", "<h2>", "</h2>", "<h3>", "</h3>"] {
        assert!(output.contains(tag), "expected {tag} to survive, got: {output}");
    }
}
```

**Toleranz-Regel:** `contains(<tag>)` statt `assert_eq!(input, output)`, weil ammonia die Ausgabe normalisiert (Attribute-Reihenfolge, Whitespace, Self-Closing-Style) — byte-exakter Vergleich wäre unnötig fragil.

### Pattern 2: E2E-Round-Trip via Template-Endpoint

**What:** POST ein Template mit `body_html`, GET es zurück, assert alle Tags überleben.

**When to use:** Für Grenzen-übergreifende Beweise (Frontend-JSON → Backend-sanitize → SQLite → Backend-serialize → Frontend-JSON).

**Vorbild:** `create_template_body_html_sanitized` in `e2e_tests.rs:14797-14842` — dieses Muster ist **näher am gewünschten Round-Trip** als `bulk_mail_body_html_sanitized_and_persisted` (Zeile 14655), weil Templates POST/GET direkt spiegeln, während der Bulk-Mail-Test einen `MailJob` zusätzlich anlegt.

**Example:**

```rust
// In genossi_bin/tests/e2e_tests.rs (append near existing template tests)
/// Phase 26 EDIT-06/07/08: POST + GET template with lists and headings —
/// prove ammonia-default preserves <ul>, <ol>, <li>, <h1>, <h2>, <h3>
/// through the Frontend→Backend→SQLite→Backend→Frontend round-trip.
#[tokio::test]
async fn create_template_body_html_lists_and_headings_round_trip() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let body_html = "<h1>Titel</h1><h2>Untertitel</h2>\
                     <ul><li>Punkt A</li><li>Punkt B</li></ul>\
                     <ol><li>Schritt 1</li><li>Schritt 2</li></ol>\
                     <h3>Sub</h3>";

    // POST
    let response = client
        .post(server.url("/api/mail/templates"))
        .json(&serde_json::json!({
            "name": "phase26-lists-headings-roundtrip",
            "subject": "Test",
            "body": "Plain fallback",
            "body_html": body_html,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: MailTemplateTO = response.json().await.unwrap();

    // GET
    let response = client
        .get(server.url(&format!("/api/mail/templates/{}", created.id)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: MailTemplateTO = response.json().await.unwrap();
    let stored = fetched.body_html.expect("body_html Some");

    // Assert each expected tag survives (ammonia may normalize attributes/order,
    // but the tag-token set must be intact — Pitfall: no byte-exact compare).
    for token in ["<h1>", "</h1>", "<h2>", "</h2>", "<h3>", "</h3>",
                  "<ul>", "</ul>", "<ol>", "</ol>",
                  "<li>", "</li>",
                  "Titel", "Untertitel", "Punkt A", "Punkt B",
                  "Schritt 1", "Schritt 2", "Sub"] {
        assert!(
            stored.contains(token),
            "round-trip lost token {token}, got: {stored}"
        );
    }
}
```

**Setup-Helper `setup()`, `sample_member()`, `server.url(...)`** existieren bereits in `e2e_tests.rs` und werden von den bestehenden Sanitize-Tests genutzt.

### Pattern 3: Source-Invariant Grep-Gate via `include_str!`

**What:** Ein Rust-Test, der die überwachte Datei via `include_str!` als String lädt und mit `contains`/Nachbarschaft-Suche kritische Substrings prüft.

**When to use:** Für Regressionsschutz auf Source-Ebene, wo (a) die Invariante nicht durch einen normalen Verhaltens-Test einfangbar ist (weil sie im Browser-Runtime läuft) UND (b) die Invariante als klar identifizierbarer Substring im Quelltext existiert.

**Empfohlener Ort:** **Inline in `wysiwyg_editor.rs::mod tests`** (nicht separate Datei) — weil:
1. `genossi-frontend` ist ein **Binary** (Cargo.toml: `name = "genossi-frontend"`, `src/main.rs`, kein `[lib]` oder `[[bin]]` Path-Override) — ein `tests/`-Integration-Test-Verzeichnis würde eine `[lib]`-Deklaration voraussetzen, die es nicht gibt. Inline `#[cfg(test)] mod tests` läuft direkt mit `cargo test --bin genossi-frontend` [VERIFIED: repo structure].
2. `include_str!(...)` löst Pfade relativ zur enthaltenden Datei auf — im selben Modul steht es also einfach `include_str!("wysiwyg_editor.rs")` — kein `../../..`-Pfad-Wildwuchs.
3. Wenn jemand `wysiwyg_editor.rs` refactored, sieht er den Grep-Gate im selben File und aktualisiert die Substrings mit.

**Example:**

```rust
// In genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
// (angehängt an bestehendes `mod tests`)

/// Phase 26 EDIT-09 — Source-Invariant Grep-Gate for the WYSIWYG editor.
///
/// These two tests protect against silent regression of the two invariants
/// that keep the ammonia sanitize gate working:
/// (1) styleWithCSS=false is set exactly once at mount, so bold/italic emit
///     semantic <b>/<i> and not <span style=…> (Pitfall 1 of 24-RESEARCH.md).
/// (2) The onpaste handler calls prevent_default() FIRST, so the browser
///     does not paste rich-text markup before our insertText override
///     (Pitfall 3 of 24-RESEARCH.md).
///
/// The tests load THIS FILE via include_str! and assert the invariants
/// are present verbatim. A cargo fmt reformat that changes whitespace or
/// argument quoting breaks these tests — that is the point.
#[cfg(test)]
mod grep_gate_tests {
    const EDITOR_SRC: &str = include_str!("wysiwyg_editor.rs");

    #[test]
    fn style_with_css_false_guard_present() {
        // Must appear literally somewhere in the file. Bind the assertion
        // to the exact call form the editor uses.
        assert!(
            EDITOR_SRC.contains(r#"exec_command_bool(&doc, "styleWithCSS", false)"#),
            "Grep gate FAILED: expected literal call \
             `exec_command_bool(&doc, \"styleWithCSS\", false)` in wysiwyg_editor.rs. \
             This guard is Pitfall 1 of 24-RESEARCH.md — removing it means Bold \
             emits <span style=…> instead of <b>, which ammonia strips silently."
        );
    }

    #[test]
    fn paste_handler_calls_prevent_default_before_read() {
        // Locate the onpaste closure and prove prevent_default() is called
        // within the same closure body. Heuristic: find "onpaste:", then
        // check prevent_default() appears within the next 400 chars (well
        // under the closure length in wysiwyg_editor.rs — currently ~20 lines).
        let idx = EDITOR_SRC
            .find("onpaste:")
            .expect("Grep gate FAILED: onpaste handler missing entirely in wysiwyg_editor.rs");
        let window = &EDITOR_SRC[idx..idx.saturating_add(400).min(EDITOR_SRC.len())];
        assert!(
            window.contains("evt.prevent_default()"),
            "Grep gate FAILED: expected `evt.prevent_default()` within 400 chars \
             of `onpaste:` in wysiwyg_editor.rs. This is Pitfall 3 of 24-RESEARCH.md \
             — without it, the browser pastes formatted HTML before our insertText \
             overrides it. Window around onpaste (first 400 chars):\n{window}"
        );
    }
}
```

**Nachbarschafts-Heuristik-Entscheidung (Claude's Discretion):** 400 Zeichen ist ein pragmatisches Fenster (`onpaste`-Closure in aktueller Datei ist ~ Zeile 89-108, also ~500 Zeichen inkl. Kommentaren — 400 fängt den `prevent_default()` in Zeile 92 sicher ein, aber nicht weit genug entfernte Referenzen). Wenn jemand die Closure später erheblich verlängert, kann das Fenster erweitert werden — der Test-Name macht klar warum.

### Pattern 4: UAT-Checklist als Copy + Anhang

**What:** `26-UAT-CHECKLIST.md` = `24-UAT-CHECKLIST.md` (Steps 1-12 unverändert übernehmen, Setup-Datum aktualisieren) + neue Steps 13-16 für UL/OL/H2/H3.

**When to use:** Wenn eine neue Phase die Verifikations-Semantik der Vorgänger-Phase erweitert und ein Vorstand-Sign-Off praktikabel ist (D-05, D-06).

**Vorschlag Steps 13-16** (Discretion beim Planner — Format aus Phase 24 spiegeln):

```markdown
- [ ] **13. Unordered List Toolbar-Button erzeugt <ul><li> [EDIT-06].** …
- [ ] **14. Ordered List Toolbar-Button erzeugt <ol><li> [EDIT-07].** …
- [ ] **15. H2 Toolbar-Button erzeugt <h2> und überlebt Reload [EDIT-08].** …
- [ ] **16. H3 Toolbar-Button erzeugt <h3> und überlebt Reload [EDIT-08].** …
```

Sign-Off-Zeile analog Phase 24 (Zeile 94-98):
```markdown
- **Vorstand smoke tester:** _______________
- **Date:** _______________
- **All 16 steps checked:** ☐ Yes  ☐ No — see notes
- **Hard-fail gates (3, 4, 5) passed:** ☐ Yes  ☐ No — MUST fix before v1.5 milestone close
```

### Anti-Patterns to Avoid

- **Byte-exakter Vergleich (`assert_eq!`) zwischen Input-HTML und ammonia-Output.** Ammonia normalisiert Attribute-Reihenfolge und kann Self-Closing-Style ändern (`<br>` vs `<br/>`); ein exakter Match ist fragil. Stattdessen `contains(<tag>)` pro erwartetem Token — deckt Semantik, ignoriert Formatierungs-Drift.
- **Grep-Gate über eine `tests/wysiwyg_source_invariants.rs`-Datei mit `include_str!("../../../src/component/mail_compose/wysiwyg_editor.rs")`.** Der lange Pfad ist fragil und `genossi-frontend` ist ein Binary — Integration-Test-Verzeichnis funktioniert ohne `[lib]`-Deklaration nicht sauber. Stattdessen inline im Ziel-Modul.
- **`sanitize.rs` in Phase 26 ändern.** D-04 sperrt das. Wenn die Unit-Tests aus D-03 failen (was nicht erwartet wird), MUSS erst der Planer via Ausstiegs-Klausel entscheiden ob ammonia-Config oder Snapshot-Test — nicht ad-hoc am `sanitize.rs`-Builder drehen.
- **UAT als harten Merge-Blocker in der Phase.** D-06: jj-WIP-Changes sind nicht wie Git-Branches gated; UAT ist Ship-Gate vor `/gsd-complete-milestone`, nicht innerhalb der Phase.
- **H1-Button aus der Toolbar entfernen** (D-01). Der Test für H1 überlebt sonst nicht und der UAT-Vorstand vermisst das Feature.
- **Neuen Toolbar-Button hinzufügen** (Out-of-scope, siehe Deferred). Text-Align/Font-Color/etc. ist explizit für spätere Phasen deferred.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTML-Sanitization | Custom regex tag stripper | `ammonia::clean()` via `sanitize_html()` in `genossi_mail/src/sanitize.rs` | Edge cases, XSS, Attribute-Escaping — bereits Phase 23 gelöst |
| E2E-HTTP-Client-Boilerplate | Manueller `hyper`-Stack | `reqwest::Client` + `setup()` aus `e2e_tests.rs` | Test-Server-Setup mit In-Memory-SQLite ist bereits in `genossi_rest/src/test_server.rs` gekapselt |
| Source-Datei einlesen im Test | `std::fs::read_to_string("...")` mit relativem Pfad | `include_str!("wysiwyg_editor.rs")` | `include_str!` löst Pfad zur Compile-Zeit relativ zur Datei auf, ist deterministisch und schlägt beim Bauen fehl (nicht erst beim Test), wenn die Ziel-Datei fehlt |
| UAT-Sign-Off-Modul | Neues UAT-Framework | Markdown-Checkbox-Liste (Copy 24-UAT) | Muster ist bewiesen (Phase 24), ein Vorstand-Signatur-Feld reicht |
| Neuer Toolbar-Button-Component-Baukasten | Neue TypeSafe-Toolbar-Config-Struktur | Die 13 bestehenden Buttons in `wysiwyg_toolbar.rs` unverändert | Refactoring ist out-of-scope; Phase-Ziel ist Test/Verifikation |

**Key insight:** Phase 26 ist bewusst eine **Zero-Production-Code-Phase**: alles Neue ist Test-Code (Unit + E2E + Grep-Gate) und Dokumentation (UAT-Checklist). Damit ist das Regressionsrisiko minimal — nichts, was in Produktion läuft, wird verändert. Der Wert entsteht durch mechanische Regressions-Detektion (Tests) + soziale Verifikation (Vorstand-Sign-Off).

## Runtime State Inventory

Nicht anwendbar — Phase 26 ist **keine Rename/Refactor/Migrations-Phase**. Es werden keine Strings umbenannt, keine gespeicherten IDs geändert, keine Env-Variablen umgetauft. Konkret pro Kategorie:

| Kategorie | Frage | Antwort |
|-----------|-------|---------|
| Stored data | Werden DB-Keys/Records verändert? | Nein — keine Migration, kein Rewrite bestehender `mail_template.body_html`-Werte. Der neue E2E-Test legt ein frisches Template an und liest es zurück; alte Templates bleiben unangefasst. |
| Live service config | Wird n8n/Datadog/etc. angefasst? | Nein — kein externer Service involviert. |
| OS-registered state | Werden Task-Scheduler-Namen / systemd-Units geändert? | Nein. |
| Secrets/env vars | Werden Secrets umbenannt? | Nein. |
| Build artifacts / installed packages | Werden Artefakte neu registriert? | Nein — keine neuen Deps, kein `cargo install`. |

## Common Pitfalls

### Pitfall 1: `include_str!`-Pfad zerbricht bei Datei-Umzug

**What goes wrong:** Wenn `wysiwyg_editor.rs` in ein anderes Verzeichnis verschoben wird (z. B. bei Refactoring in `component/editor/wysiwyg.rs`), zeigt `include_str!("wysiwyg_editor.rs")` in einer separaten Test-Datei ins Nirwana und kompiliert nicht mehr.

**Why it happens:** `include_str!` löst Pfade **zur Compile-Zeit** relativ zur enthaltenden Datei. Trennung von Test und Source multipliziert die Fragilität.

**How to avoid:** Test **inline im überwachten Modul** platzieren (D-02 Discretion, empfohlen: `wysiwyg_editor.rs::mod grep_gate_tests`). Umzug der Datei nimmt den Test automatisch mit; Pfad-String bleibt `"wysiwyg_editor.rs"` (Selbst-Referenz), verwirrenderweise korrekt, weil `include_str!` gegenüber dem enthaltenden File auflöst.

**Warning signs:** Cargo-Compile-Fehler `couldn't read wysiwyg_editor.rs: No such file or directory`.

### Pitfall 2: `cargo fmt` reflowt den überwachten Substring

**What goes wrong:** Wenn `wysiwyg_editor.rs:80` sich durch ein Format-Refactoring zu

```rust
let _ = crate::js::exec_command_bool(
    &doc,
    "styleWithCSS",
    false,
);
```

ändert, dann findet `contains(r#"exec_command_bool(&doc, "styleWithCSS", false)"#)` den Substring nicht mehr, weil Zeilenumbrüche eingefügt wurden.

**Why it happens:** `contains` ist byte-exakt; Whitespace-Änderungen zerstören die Übereinstimmung.

**How to avoid:** (a) Test-Fehlermeldung erklärt genau was falsch ist, sodass der Reformater den Test anpasst („Grep gate FAILED: expected literal call ..."). (b) Alternativ eine Regex mit Whitespace-Toleranz — aber D-02 fordert bewusst „exakt", also lieber die Fehlermeldung sprechen lassen. (c) Wenn Refactoring bevorsteht, den Grep-Substring in einer Konstante halten und im Kommentar über der Zielzeile spiegeln.

**Warning signs:** Grep-Gate schlägt fehl, obwohl der Guard semantisch vorhanden ist.

### Pitfall 3: E2E-Test-Pre-existing Failure aus Phase 22

**What goes wrong:** Wer `cargo test --test e2e_tests` ausführt, sieht `test_mail_preview_repayment_no_entries_does_not_default_to_one` fehlschlagen und denkt, Phase 26 hat etwas kaputt gemacht.

**Why it happens:** Dieser Test failed seit Phase 22 (dokumentiert in STATE.md Zeile 79, in `24-UAT-CHECKLIST.md` Zeile 79). Er ist NICHT Phase-26-Regression.

**How to avoid:** In Phase-26 Plan-Doku + im Verification-Report klar sagen: „308 baseline (307 grün + 1 pre-existing failure aus Phase 22) → 309 (308 grün + 1 unchanged failure)". Der Verifier zählt genau — 309 total, +1 vs. Baseline, alle neuen grün.

**Warning signs:** Reviewer notiert „Ein Test failed" — auf STATE.md verweisen.

### Pitfall 4: Ammonia normalisiert Whitespace zwischen Block-Elementen

**What goes wrong:** Input `<ul>\n  <li>a</li>\n</ul>` wird von ammonia zu `<ul><li>a</li></ul>` (oder umgekehrt) normalisiert. `assert_eq!(input, output)` fällt durch.

**Why it happens:** Ammonia entfernt/normalisiert Whitespace bei der HTML-Reparatur (`html5ever`-Parser-Verhalten).

**How to avoid:** `contains(<tag>)` pro erwartetem Token, nicht Byte-Vergleich. So auch die Empfehlung in Pattern 1 und Pattern 2.

**Warning signs:** Test schlägt fehl mit „expected X, got Y" wobei Y nur in Whitespace/Attribute-Reihenfolge abweicht.

### Pitfall 5: `execCommand`-Output-Variabilität zwischen Browsern

**What goes wrong:** Beim UAT-Smoke prüft der Vorstand in DevTools den innerHTML nach Klick auf UL-Button und sieht `<ul><li><br></li></ul>` (leerer neuer Listeneintrag) statt erwartetem `<ul><li></li></ul>`. Chromium fügt manchmal `<br>` in leere contenteditable-Elemente ein, Firefox nicht.

**Why it happens:** Browser-spezifische execCommand-Implementierung; MDN bestätigt dass execCommand's exakter Output nicht spezifiziert ist [CITED: developer.mozilla.org/en-US/docs/Web/API/Document/execCommand — „test to ensure cross-browser compatibility"].

**How to avoid:** UAT-Step-Text präzise formulieren („Der DOM enthält `<ul>` und `<li>` — leere `<br>`-Filler-Tags sind erlaubt"). Round-Trip-Test läuft trotzdem grün, weil ammonia `<br>` ebenfalls durchlässt.

**Warning signs:** UAT-Tester meldet „innerHTML sieht anders aus als erwartet" — Filler-`<br>` ist OK.

### Pitfall 6: Frontend-Test läuft nicht per Default in `cargo test` workspace-weit

**What goes wrong:** Reviewer läuft `cargo test` im Root, sieht 252+308 Tests grün, denkt alles ist gut — vergisst aber `cargo test --bin genossi-frontend`. Der Grep-Gate ist ein Frontend-Test und läuft nur mit dem `--bin`-Flag.

**Why it happens:** `genossi-frontend` ist als Binary konfiguriert (target `wasm32-unknown-unknown` für WASM-Builds, aber pure-Rust-Tests laufen native). Workspace-`cargo test` schließt Binaries mit ein, aber das Frontend-Bin hat weitere target-cfg-Anforderungen die je nach Aufruf-Weg divergieren können.

**How to avoid:** Im Verification-Report explizit alle 3 Test-Kommandos dokumentieren:
```bash
cargo test -p genossi_mail --lib          # 252 baseline → 255 (+3 Unit-Tests)
cargo test -p genossi_bin --test e2e_tests # 308 baseline (1 pre-existing fail) → 309
cargo test -p genossi-frontend            # 284+ baseline → 286 (+2 grep-gate)
```
Alle drei müssen laufen, sonst greift der Grep-Gate nicht.

**Warning signs:** CI oder Reviewer meldet nicht die 2 neuen Grep-Gate-Tests in der Zählung — Test-Kommando fehlt.

## Code Examples

Verified patterns from repo:

### Vorbild-Test in `e2e_tests.rs`

```rust
// Source: genossi_bin/tests/e2e_tests.rs:14797 — create_template_body_html_sanitized
#[tokio::test]
async fn create_template_body_html_sanitized() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/mail/templates"))
        .json(&serde_json::json!({
            "name": "html-sanitize-test",
            "subject": "Test",
            "body": "Hallo {{ first_name }}",
            "body_html": "<p>Hallo {{ first_name }}</p><script>alert(1)</script>",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: MailTemplateTO = response.json().await.unwrap();
    // ... GET + assert stored_html.contains("<p>"), !contains("<script>")
}
```

### Vorbild-Test in `sanitize.rs`

```rust
// Source: genossi_mail/src/sanitize.rs:94 — sanitize_preserves_jinja_placeholder
#[test]
fn sanitize_preserves_jinja_placeholder_in_text_content() {
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
```

### Verwendbare `exec_command_*`-Helpers

```rust
// Source: genossi-frontend/src/js.rs:174-237 — already used by wysiwyg_toolbar.rs
pub fn exec_command_bool(doc: &web_sys::Document, cmd: &str, arg: bool)
    -> Result<bool, wasm_bindgen::JsValue>;
pub fn exec_command_str(doc: &web_sys::Document, cmd: &str, arg: &str)
    -> Result<bool, wasm_bindgen::JsValue>;
pub fn exec_command_simple(doc: &web_sys::Document, cmd: &str)
    -> Result<bool, wasm_bindgen::JsValue>;
```

### Ammonia-Default-Whitelist (referenziert)

```
Source: docs.rs/ammonia (Builder::default, via WebFetch 2026-07-17)
a, abbr, acronym, area, article, aside, b, bdi, bdo, blockquote, br,
caption, center, cite, code, col, colgroup, data, dd, del, details, dfn,
div, dl, dt, em, figcaption, figure, footer, h1, h2, h3, h4, h5, h6,
header, hgroup, hr, i, img, ins, kbd, li, map, mark, nav, ol, p, pre, q,
rp, rt, rtc, ruby, s, samp, small, span, strike, strong, sub, summary,
sup, table, tbody, td, th, thead, time, tr, tt, u, ul, var, wbr

Default tag-attributes: <ol> has `start`. Others have only generic
attributes like `lang` and `title`.
```

Alle Ziel-Tags der Phase (`ul`, `ol`, `li`, `h1`, `h2`, `h3`) sind enthalten. D-04 (Nicht-Änderung von `sanitize.rs`) ist konsistent mit dieser Whitelist.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manuelle Vorstand-Kontrolle ohne UAT-Checklist | 16-Punkte-Sign-Off-Checkliste mit HARD FAIL GATES | Phase 24 (v1.4) | UAT strukturiert; deferred nach Phase 24; Phase 26 holt nach |
| Ad-hoc Substring-Checks in Shell-Scripts als „Grep-Gate" (EDIT-01/02-Formulierung) | Rust-Test mit `include_str!` + `contains` | Phase 26 (v1.5) — neu | Versioniert im Code, kein extra Tooling; Muster reusable für zukünftige Source-Invariant-Guards |
| Kein Test für Listen/Headings ammonia-Whitelist | 3 Unit-Tests + 1 E2E-Round-Trip | Phase 26 (v1.5) | Erweitert Phase-23-Test-Baseline (5 Tests) um 4 Tests |
| Merge-Gate (klassische Git-Konvention) | Ship-Gate vor Milestone-Close (jj-WIP-Kontext) | Phase 26 (v1.5) — D-06 | jj-Detachment akzeptiert; Milestone-Audit prüft UAT-Status |

**Deprecated/outdated:** —

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `execCommand`-Output für `insertUnorderedList` etc. ist konsistent genug, dass `<ul>`, `<li>`-Tags im innerHTML erscheinen (Filler-`<br>` erlaubt) | Pitfall 5, UAT-Steps 13-16 | LOW — MDN bestätigt die Command-Semantik; nur der genaue Filler-Content variiert. UAT-Step-Text muss Filler-`<br>` als OK markieren. |
| A2 | `cargo test --bin genossi-frontend` läuft die inline-Tests in `wysiwyg_editor.rs::mod grep_gate_tests` ohne WASM-target-Voraussetzung | Pattern 3, Pitfall 6 | LOW — `include_str!` und `contains` sind pure-Rust ohne Browser/wasm-bindgen-Bezug. Beim Plan-Task 1 kurz mit einem Dummy-Test bestätigen (`assert!(true)`) bevor die echten Grep-Tests reingeschrieben werden. |
| A3 | Der `setup()`-Helper im e2e_tests.rs verträgt zusätzliche Tests ohne Portkonflikte | Pattern 2 | LOW — 308 andere Tests im gleichen File tun das schon; `test_server.rs` gibt jedem Test seinen eigenen In-Memory-DB + random Port (siehe CLAUDE.md „Test Server Infrastructure"). |
| A4 | Ammonia 4.x lässt UL/OL/LI/H1-H6 unverändert durch, wenn keine Attribute drin sind | Pattern 1, D-04 | LOW — [VERIFIED: docs.rs/ammonia via WebFetch 2026-07-17], zusätzlich durch die drei neuen Unit-Tests bewiesen. Ausstiegs-Klausel in D-04 falls doch nicht. |
| A5 | 400 Zeichen Fenster im Nachbarschafts-Grep ist groß genug für die `onpaste`-Closure, aber klein genug um Fremd-`prevent_default()`-Calls auszuschließen | Pattern 3 | MEDIUM — aktuell ist die Closure ~20 Zeilen (~500 chars inkl. Kommentare); wenn sie über 400 chars vor dem `prevent_default()` wächst, muss der Test angepasst werden. Deutliche Fehlermeldung erklärt den Grund. |
| A6 | Der 24-UAT-CHECKLIST.md-Setup-Block ist inhaltlich für 2026-07 noch aktuell (Skill `run-rust-backend-and-frontend`, mock_auth Backend :3000, dx serve :8080) | D-05 | LOW — die referenzierten Ports und Skill-Namen sind unverändert seit Phase 24; falls Aktualisierung nötig (z. B. mock_auth-Env-Namen), der Planner beim Copy-Task nachziehen. |

**Any `[ASSUMED]` claim above must be gated by a discovery task or a manual smoke-check in the plan before it becomes a locked decision.**

## Open Questions

1. **Ort des Grep-Gate-Tests**
   - What we know: Frontend ist ein Binary; ein `tests/`-Verzeichnis würde `[lib]`-Deklaration erfordern.
   - What's unclear: Ob `wysiwyg_editor.rs::mod grep_gate_tests` (empfohlen), `mail_compose/mod.rs::mod tests`, oder eine neue `component/mail_compose/wysiwyg_source_invariants.rs` bevorzugt wird.
   - Recommendation: **Inline in `wysiwyg_editor.rs::mod grep_gate_tests`** (siehe Pattern 3). Halbiert Pfad-Fragilität von `include_str!` und macht den Grep-Gate für Refactorer sofort sichtbar.

2. **Toleranz-Breite im E2E-Test**
   - What we know: Byte-exakter Vergleich ist fragil (Pitfall 4).
   - What's unclear: Ob es reicht, jedes Tag-Token einzeln zu asserten (Pattern 2), oder ob eine strengere Format-Kontrolle (z. B. Reihenfolge der Tags im Body) sinnvoll ist.
   - Recommendation: Token-Set-Assertion reicht. Die Reihenfolge im HTML ist durch die POST-Payload determiniert, ammonia sortiert nicht um. Wenn später Bugs auftreten die Reihenfolge betreffen, dann gezielt ergänzen.

3. **Icon-Politur beim UAT — proaktiv oder reaktiv?**
   - What we know: UAT-Vorstand könnte „H2 sieht genauso aus wie H3" bemerken.
   - What's unclear: Ob wir schon jetzt Icons/Labels aufhübschen oder erst wenn es beim UAT auffällt.
   - Recommendation: **Reaktiv** — Deferred-Item ist explizit „nur wenn beim UAT als störend auffällt". Planer sollte einen Optional-Task „Cosmetic polish (only if UAT flags it)" reservieren, aber nicht in die Default-Task-Sequence hängen.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (Nix-Flake) | Alle Test-Builds | ✓ | flake-pinned | — |
| `cargo test` | Unit + E2E + Grep-Gate | ✓ | standard | — |
| `dx serve` | UAT-Smoke Frontend | ✓ | Nix-Flake | — |
| Backend `mock_auth` build | UAT-Smoke Backend | ✓ | `cargo run --features mock_auth --bin genossi` | — |
| SQLite (in-memory für E2E) | E2E-Round-Trip-Test | ✓ | via test_server.rs | — |
| ammonia 4.x | Backend sanitize | ✓ | im Workspace | — |
| Vorstand (Human) für UAT | 16-Punkte-Sign-Off | ✓ | verfügbar | UAT als Ship-Gate deferrable (D-06) |
| Chromium/Firefox Browser | UAT-Smoke DevTools | ✓ | User-Machine | — |

Keine fehlenden externen Abhängigkeiten. Alle Werkzeuge sind Teil der bestehenden Entwicklungs-/Test-Umgebung.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust unit tests via `#[test]`, `#[tokio::test]` für E2E) |
| Config file | none — standard cargo test discovery |
| Quick run command (pro Task) | `cargo test -p genossi_mail --lib` (Unit) · `cargo test -p genossi_bin --test e2e_tests <test_name>` (E2E einzeln) · `cargo test -p genossi-frontend grep_gate` (Source-Invariant) |
| Full suite command (Phase-Gate) | `cargo test` workspace-weit |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EDIT-06 | UL-Tags überleben ammonia + Round-Trip | Unit + E2E | `cargo test -p genossi_mail sanitize_preserves_unordered_list -- --exact` + `cargo test --test e2e_tests create_template_body_html_lists_and_headings_round_trip -- --exact` | ❌ Wave 1 (neu) |
| EDIT-07 | OL-Tags überleben ammonia + Round-Trip | Unit + E2E | `cargo test -p genossi_mail sanitize_preserves_ordered_list -- --exact` + selber E2E wie EDIT-06 | ❌ Wave 1 (neu) |
| EDIT-08 | H1/H2/H3-Tags überleben ammonia + Round-Trip | Unit + E2E | `cargo test -p genossi_mail sanitize_preserves_headings_h1_h2_h3 -- --exact` + selber E2E wie EDIT-06 | ❌ Wave 1 (neu) |
| EDIT-09 | Grep-Gate: styleWithCSS-Guard + onpaste-prevent_default vorhanden | Source-Invariant (Rust `include_str!`) | `cargo test -p genossi-frontend style_with_css_false_guard_present -- --exact` + `cargo test -p genossi-frontend paste_handler_calls_prevent_default_before_read -- --exact` | ❌ Wave 1 (neu) |
| EDIT-10 | UAT-Checklist (16 Steps) durch Vorstand abgehakt | Manual UAT | Vorstand-Smoke gegen `dx serve` — kein Automated Command | ❌ Wave 2 (26-UAT-CHECKLIST.md neu, Sign-Off nachgelagert vor Milestone-Close) |

### Sampling Rate

- **Per task commit:** `cargo test -p <package> <specific_test_name>` — der spezifische neue Test läuft direkt zur Verifikation der grade committeten Änderung (schnell, < 5 s pro Test).
- **Per wave merge:** `cargo test -p genossi_mail --lib && cargo test --test e2e_tests && cargo test -p genossi-frontend` — die drei Test-Ebenen komplett (dauert < 60 s, bewiesen durch Baseline-Läufe in Phase 24).
- **Phase gate:** Full workspace-Suite `cargo test` grün (mit dem einen bekannten Pre-existing failure aus Phase 22 — siehe Pitfall 3) VOR `/gsd-verify-work`.

### Wave 0 Gaps

Keine Infrastruktur-Gaps — alle Test-Frameworks, Test-Helper (`setup()`, `sample_member()`, `server.url()`), und die überwachten Source-Files existieren bereits. Kein „Wave 0" nötig; Wave 1 kann direkt mit den 3 Unit-Tests + 1 E2E + 2 Grep-Tests + UAT-Copy loslegen.

## Security Domain

`security_enforcement` ist im Projekt aktiv (kein explizites `false` in `.planning/config.json`), also folgt eine kompakte ASVS-Auswertung. Phase 26 ist Test-only, das Sicherheitsprofil ändert sich nicht — die Sanitize-Grenze bleibt Phase-23-Code.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Keine neuen Auth-Pfade; E2E-Test läuft mit demselben mock_auth-Setup wie 308 andere E2E-Tests. |
| V3 Session Management | no | Keine Session-Änderung. |
| V4 Access Control | no | Keine neuen Endpoints; alle betroffenen Endpoints (POST/GET `/api/mail/templates`) haben ihre Berechtigungen bereits in Phase 22-24 verdrahtet. |
| V5 Input Validation | yes (indirekt) | `ammonia::clean()` via `sanitize_html()` in `genossi_mail/src/sanitize.rs` (Phase 23 D-03) — Sanitize-on-Store-Grenze. Phase 26 erweitert nur die **Test-Abdeckung** dieser Grenze, ändert die Grenze nicht (D-04). |
| V6 Cryptography | no | Keine Krypto involviert; UAT-QR-Codes / SOPS-Keys / TLS unangetastet. |

### Known Threat Patterns for {Rust + Axum + Dioxus + ammonia + SQLx}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stored XSS via User-`body_html` (Vorstand malicious oder kompromittiert) | Tampering, Elevation of Privilege | `ammonia::clean()` an der Store-Grenze (bestehend seit Phase 23); Phase 26 fügt Test-Coverage für ammonia-Verhalten bei UL/OL/H1-H3 hinzu, verstärkt also indirekt das Vertrauen in die Grenze. |
| Bypass der Sanitize-Grenze durch fehlerhaftes Frontend (z. B. Bold als `<span style>` das ammonia stripped) | Tampering | Grep-Gate (D-02) macht die zwei kritischen Source-Invarianten regressions-fest: `styleWithCSS=false` (semantische Tags) und `onpaste prevent_default()` (kein Rich-Text-Paste). |
| SQL-Injection in `body_html`-Persistenz | Tampering | SQLx compile-time-verifizierte Queries; kein neuer SQL in Phase 26. |
| CSRF gegen `/api/mail/templates` | Spoofing | Bestehende Session-basierte Auth; Phase 26 fügt keine Endpoints hinzu. |

**Fazit:** Phase 26 verringert die Angriffsfläche minimal (nur Test-Code), erhöht die Test-Coverage der bestehenden Sanitize-Grenze aber messbar (5 neue Test-Assertions auf ammonia-Verhalten). Das ist netto **security-positiv** ohne neues Risiko.

## Sources

### Primary (HIGH confidence)

- Repo-Read: `.planning/phases/26-editor-formatierung-vervollstaendigen/26-CONTEXT.md` (User-Entscheidungen D-01 bis D-06)
- Repo-Read: `.planning/phases/26-editor-formatierung-vervollstaendigen/26-DISCUSSION-LOG.md` (Q&A-Historie)
- Repo-Read: `.planning/REQUIREMENTS.md` Zeilen 12-16 (EDIT-06..10)
- Repo-Read: `.planning/ROADMAP.md` Zeilen 106-116 (Phase 26 Goal + Success Criteria)
- Repo-Read: `.planning/STATE.md` (v1.5 Structure, Deferred Verification Phase 24, Pre-existing failure aus Phase 22)
- Repo-Read: `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` (styleWithCSS-Guard Zeile 80, onpaste-prevent_default Zeile 92, inline mod tests Zeile 184)
- Repo-Read: `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` (13 Buttons, UL/OL/H1/H2/H3-Wiring Zeilen 112-185)
- Repo-Read: `genossi-frontend/src/js.rs` (`exec_command_bool/str/simple` Zeilen 174-237)
- Repo-Read: `genossi_mail/src/sanitize.rs` (Wrapper + 4 bestehende Tests)
- Repo-Read: `genossi_bin/tests/e2e_tests.rs` (`bulk_mail_body_html_sanitized_and_persisted` Zeile 14655, `create_template_body_html_sanitized` Zeile 14797 als Vorbild)
- Repo-Read: `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md` (Copy-Vorlage, Setup, HARD FAIL GATES, Sign-Off-Muster)
- Repo-Read: `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-RESEARCH.md` (Pitfalls 1-8, execCommand-Tabelle)
- Repo-Read: `genossi-frontend/Cargo.toml` (Binary-only, kein `[lib]`)
- Repo-Read: `genossi-frontend/src/i18n/de.rs` Zeilen 224-228 (i18n-Keys bereits vorhanden)
- Repo-Read: `genossi_mail/Cargo.toml` (ammonia = "4")
- Grep: `genossi-frontend/src/` — `include_str!` bisher NICHT für Grep-Gates verwendet (Muster ist neu)
- WebFetch: docs.rs/ammonia (Default-Whitelist explizit für `ul`, `ol`, `li`, `h1`..`h6` bestätigt, `<ol>` erlaubt `start`-Attribut)

### Secondary (MEDIUM confidence)

- WebFetch: MDN `Document.execCommand` — bestätigt Existenz von `insertUnorderedList`, `insertOrderedList`, `formatBlock <h2>`; warnt vor Cross-Browser-Variabilität aber nennt keinen konkreten Ausgabe-Unterschied für unsere Ziel-Commands.

### Tertiary (LOW confidence)

- Keine — alle Behauptungen sind entweder repo-verifiziert oder von ammonia/MDN cited.

## Metadata

**Confidence breakdown:**
- Standard stack (keine neuen Deps): HIGH — alles bereits in Produktion (Phase 22-25).
- Architecture (Test-Ebenen 1-4): HIGH — Muster 1 und 2 sind direkte Kopien aus Phase 23/24, Muster 3 ist neu aber trivial.
- Pitfalls: HIGH — alle 6 Pitfalls sind entweder direkt beobachtet (Pitfall 3 dokumentiert in STATE.md, Pitfall 5 aus 24-RESEARCH Pitfall 5) oder trivial ableitbar (Pitfall 1, 2, 4, 6).
- Grep-Gate-Muster (D-02): MEDIUM — neues Muster im Projekt, aber technisch simpel und mit klarer Fehlermeldung ausgestattet. Pitfall 2 (cargo-fmt-Reflow) ist der einzige Reibungspunkt, dokumentiert.
- UAT-Copy (D-05): HIGH — Phase-24-Checklist ist stabil und produktions-erprobt.

**Research date:** 2026-07-17
**Valid until:** 2026-08-17 (30 Tage — Phase-Umfang stabil, keine fast-moving Dependencies)
