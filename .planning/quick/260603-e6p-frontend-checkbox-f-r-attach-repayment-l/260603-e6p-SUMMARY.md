---
quick_id: 260603-e6p
type: summary
wave: 1
status: completed
completed_at: 2026-06-03
duration_min: 14
tasks_total: 2
tasks_completed: 2
files_modified:
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/page/mail_page.rs
commits:
  - hash: 910f986
    message: "feat(quick-260603-e6p): frontend-checkbox for attach_repayment_letter"
requirements_satisfied:
  - QUICK-260603-e6p
key-decisions:
  - "Inline-RSX in mail_page.rs (kein neuer Component) — single-page UI ohne zweiten Konsumenten"
  - "Plain `#[serde(default)]` ohne skip_serializing_if für `attach_repayment_letter: bool` — false ist deliberater opt-out und muss im wire body sichtbar bleiben"
  - "Checkbox-Gate `if repayment_phase_id.read().is_some()` spiegelt backend-400-Validator (genossi_mail/src/rest.rs:478-481) — coupling-point dokumentiert"
---

# Quick 260603-e6p: Frontend-Checkbox attach_repayment_letter — Summary

## One-liner

Vorstand bekommt im Bulk-Mail-Compose-Flow eine Opt-in-Checkbox für die automatische Anhängung des per-Mitglied generierten RepaymentLetter-PDFs; backend-Feld `attach_repayment_letter` (Commit `62e62b7`) ist damit ohne Swagger-Roundtrip nutzbar.

## What changed

### `genossi-frontend/src/api.rs`

- `SendBulkMailRequest` gewinnt `attach_repayment_letter: bool` als letztes Feld mit `#[serde(default)]` (KEIN `skip_serializing_if`, weil bool immer wire-meaningful ist).
- `send_bulk_mail()` async-fn-Signatur erweitert um trailing `attach_repayment_letter: bool` Parameter; durchgereicht in den Request-Body.
- 2 neue Unit-Tests:
  - `test_send_bulk_mail_request_attach_repayment_letter_backward_compat` — Payloads ohne das Feld deserialisieren weiterhin (default = false).
  - `test_send_bulk_mail_request_attach_repayment_letter_roundtrip` — `true` serialisiert in `"attach_repayment_letter":true` und roundtrippt korrekt; wire-name matcht backend `genossi_mail/src/rest.rs:131-163` exakt.
- Bestehende Tests `test_send_bulk_mail_request_phase12_roundtrip` und `test_send_bulk_mail_request_skips_none_fields` um den neuen Pflicht-Init `attach_repayment_letter: false` erweitert.

### `genossi-frontend/src/i18n/mod.rs`

Zwei neue `Key`-Varianten in der Mail-Sektion (zwischen `MailTemplateError` und `// SMTP Settings`):
- `MailAttachRepaymentLetter`
- `MailAttachRepaymentLetterHint`

### `genossi-frontend/src/i18n/de.rs`

- `Key::MailAttachRepaymentLetter` → `"RepaymentLetter (Anschreiben) als persönliches PDF anhängen"`
- `Key::MailAttachRepaymentLetterHint` → `"Empfänger ohne generierten Brief in dieser Phase werden als fehlgeschlagen markiert."`

### `genossi-frontend/src/i18n/en.rs`

- `Key::MailAttachRepaymentLetter` → `"Attach RepaymentLetter (cover letter) as personal PDF"`
- `Key::MailAttachRepaymentLetterHint` → `"Recipients without a generated letter in this phase are marked as failed."`

### `genossi-frontend/src/page/mail_page.rs`

- Neuer Signal `let mut attach_repayment_letter = use_signal(|| false);` nach `selected_template_id`. Default ist immer `false` (kein URL-Param-Hydration; Vorstand opt-in pro Send).
- Conditional Checkbox-Block zwischen `TemplatePreview { ... }` und `// Attachment selector — visible only for single recipient`. Rendert NUR wenn `repayment_phase_id.read().is_some()` (Mirror des backend-400-Validators).
- Lokales `attach_letter_flag: bool` wird VOR dem `spawn(async move { ... })` aus dem Signal gelesen — closure capturiert nur den `bool`, nicht den Signal.
- `api::send_bulk_mail(...)` Aufrufstelle ergänzt um trailing `attach_letter_flag` Argument.
- Im `Ok(_job) => { ... }` Branch: `attach_repayment_letter.set(false)` neben den bestehenden Resets — fresh state für nächsten Compose.

## Placement decision

Wie geplant: zwischen `TemplatePreview` (~line 433-439) und `// Attachment selector` (~line 441). Begründung — Checkbox gehört semantisch zur Repayment-Mail-Konfiguration und sollte vor den Attachment-Selektoren stehen, damit der Vorstand die Repayment-spezifische Option klar als Teil des Repayment-Kontexts wahrnimmt.

## Component-First-Entscheidung

**Inline-RSX in `mail_page.rs` ist hier korrekt** und folgt der `feedback_component_first`-Regel:

- Checkbox + Hinweistext leben nur auf `mail_page.rs`. Aktuell und absehbar kein zweiter Konsument (keine andere Page sendet Bulk-Repayment-Mails).
- Component-First-Regel verhindert **Duplizierung**, nicht **Pre-Extraktion**. Single-Use-Components sind explizit out-of-scope der Regel.
- Sollte später eine zweite Page diese UI brauchen (z. B. eine dedizierte Repayment-Mail-Seite), ist die Extraktion nach `src/component/repayment_attach_checkbox.rs` trivial — das Pattern (RSX-Block + 2 i18n-Keys + Signal-Binding) ist isoliert und portabel.

## Existing call-sites of `send_bulk_mail` outside mail_page.rs

**Keine** — `grep -rn "send_bulk_mail(" genossi-frontend/src/` listet exakt zwei Treffer:
- `genossi-frontend/src/api.rs:873` — die Funktion-Definition selbst.
- `genossi-frontend/src/page/mail_page.rs:566` — die einzige call-site, im selben Commit aktualisiert.

Keine zusätzlichen Migrations notwendig.

## Backend coupling

Der frontend-seitige Conditional-Gate (`if repayment_phase_id.read().is_some()`) ist ein **direktes Mirror** des backend-400-Validators in `genossi_mail/src/rest.rs:478-481`:

```rust
if req.attach_repayment_letter && req.repayment_phase_id.is_none() {
    return Err(MailServiceError::BadRequest(
        "attach_repayment_letter requires repayment_phase_id".into(),
    ));
}
```

**Wenn dieser backend-Check jemals abgeschwächt oder umgebaut wird (z. B. um eine globale Phase-Auswahl-Logik), MUSS der frontend-Gate-Check hier ebenfalls aktualisiert werden**, sonst entsteht UI-vs-Validation-Drift. Dieser Coupling-Point ist auch in der Inline-Doc oberhalb des Signal-Initializers in `mail_page.rs` festgehalten.

## Deviations from plan

**Keine Plan-Deviations.** Der Plan wurde 1:1 ausgeführt; alle done-criteria sind erfüllt.

Eine kleine zusätzliche Test-Ergänzung (über Plan hinaus, conform CLAUDE.md "Always make sure you have tests for the changes"):
- 2 neue Unit-Tests in `api.rs` zur Backward-Compat + Roundtrip-Garantie für das neue Feld.

Diese Tests sind additiv und brechen keine bestehende API; sie zementieren die wire-format-Garantie gegen künftige `#[serde(...)]`-Drift.

## Build / test state

- `cargo check --manifest-path genossi-frontend/Cargo.toml` → exit 0 (nur pre-existing dead-code-warnings, out-of-scope).
- `rustfmt --check --edition 2021` auf allen 5 Dateien → exit 0.
- `cargo test --bin genossi-frontend send_bulk_mail` → **5/5 passed** (3 alte + 2 neue).

## Self-Check: PASSED

**Created files exist:**
- `.planning/quick/260603-e6p-frontend-checkbox-f-r-attach-repayment-l/260603-e6p-SUMMARY.md` (this file)

**Commit exists:**
- `910f986` → `feat(quick-260603-e6p): frontend-checkbox for attach_repayment_letter` (5 files, +99 insertions).

**Verification commands re-run after final commit:**
- `git log --oneline -1` → `910f986 feat(quick-260603-e6p): frontend-checkbox for attach_repayment_letter` ✓
- `grep -c "attach_repayment_letter" genossi-frontend/src/api.rs` → 15 (≥ 3) ✓
- `grep -c "MailAttachRepaymentLetter" genossi-frontend/src/i18n/{mod,de,en}.rs` → 2/2/2 ✓
- `grep -c "attach_repayment_letter" genossi-frontend/src/page/mail_page.rs` → 6 (≥ 5) ✓

## Notes for the next agent

- Die Backend-Coupling-Notiz oben (`genossi_mail/src/rest.rs:478-481`) ist wichtig: jeder Refactor am 400-Validator muss den UI-Gate mit-anpassen.
- `attach_repayment_letter` ist eine bool-Field; ein zukünftiger Phase-12+-Erweiterungspunkt könnte ein Enum daraus machen (z. B. `AttachMode::None | Personal | Bulk`), sollte dann aber backward-compat per Custom-Deserialize sicherstellen.
- Component-First-Extraktion nach `src/component/repayment_attach_checkbox.rs` ist trivial sobald eine zweite Page diese UI braucht; das Inline-RSX-Block ist bewusst isoliert gehalten (Signal-Param + 2 i18n-Keys, kein anderer State).
