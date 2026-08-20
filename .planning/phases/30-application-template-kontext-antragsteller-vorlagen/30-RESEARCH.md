# Phase 30: Application-Template-Kontext (Antragsteller-Vorlagen) - Research

**Researched:** 2026-08-20
**Domain:** Rust template rendering (minijinja) + SQLite additive migration + domain formatting
**Confidence:** HIGH (all claims verified against live code this session)

## Summary

This phase is a "verify-then-extend" job, not a discovery job. The CONTEXT.md (D-01..D-14)
is research-grade and names exact functions and line numbers. This research confirms every
load-bearing claim against the live codebase, and flags the **two places where CONTEXT.md
drifted from the current code** so the planner does not build on a false premise.

The work: (1) add an additive `template_type TEXT NOT NULL DEFAULT 'member'` column to
`mail_templates` and thread it through all 6 SQL sites + the DB-row struct + entity + TO;
(2) add a pure `application_to_template_context(...)` builder mirroring
`member_to_template_context`; (3) add `format_eur_de(cents: i64) -> String` in
`genossi_service` and retrofit `send_confirmation_mail`; (4) extract a
`validate_rendered(subject, body, &[Value])` core so `validate_template` keeps its exact
signature and a new `validate_application_template` reuses the dummy-probe pattern; (5) seed
a German "Zahlungserinnerung" template with fixed UUID `…0003`.

**Primary recommendation:** Follow CONTEXT D-01..D-14 verbatim, but resolve the two drift
items below before planning. All package/library needs are already in-tree — "add nothing"
holds; no new dependency.

**Two drift findings (read before planning):**
1. **D-10 is inaccurate about the current member flow.** `validate_template` is **NOT** called
   at template create/update. The member templates are validated at **send time** in
   `genossi_mail/src/rest.rs:534-547` (the bulk-send handler), and `mail_template_service.rs`
   create/update only sanitizes HTML and persists — no render probe. So "greift am selben
   Punkt wie beim Member-Flow: bei Create/Update" describes behavior that does not exist today.
   Adding application-template validation at create/update is a **new, additive** validation
   point (still fine, still additive, still leaves member tests green) — the planner must
   choose the injection site deliberately (recommendation below).
2. **D-02 "inkl. Test-DDL" does not apply to `mail_templates`.** The `dao_sqlite.rs` unit
   tests create manual DDL only for `mail_jobs`/`mail_recipients`/`static_documents`/etc. —
   there is **no** `CREATE TABLE mail_templates` in any test. E2E tests get the schema from
   real migrations (`sqlx::migrate!("../migrations/sqlite")`, `genossi_bin/tests/e2e_tests.rs:34`).
   So the "all SQL column lists incl. test DDL" checklist for `mail_templates` reduces to the
   6 production queries + INSERT/UPDATE + the `MailTemplateDb` row struct. No test DDL to touch.

All other CONTEXT line numbers and function names are **accurate**.

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Trennung via neuer Spalte `template_type TEXT NOT NULL DEFAULT 'member'` auf
  `mail_templates` (Werte `'member'`/`'application'`). Additive, forward-only Migration. Kein
  separates Tabellen-Set, kein Bool-Flag. Die 2 bestehenden Seeds bleiben `'member'`.
- **D-02:** PITFALL — ALLE `mail_templates`-SQL-Spaltenlisten (SELECT/INSERT/UPDATE, inkl.
  Test-DDL) um `template_type` erweitern. NULL-/Default-Legacy-Roundtrip absichern: bestehende
  Zeilen ohne expliziten Typ lesen als `'member'` zurück. *(Research note: no test DDL exists
  for mail_templates — see drift finding 2.)*
- **D-03:** Frontend-Member-Template-Selector filtert auf `template_type = 'member'`.
  Datengrundlage (Typ-Feld im Read-Pfad/TO) entsteht in Phase 30; UI-Filterung greift in Phase 32.
- **D-04:** Application-Kontext enthält Application-Felder unter member-kompatiblen Variablennamen:
  `first_name`, `last_name`, `salutation`, `title`. Anzahl Anteile als `shares` (Feld heißt
  `shares: i32`, NICHT `current_shares`).
- **D-05:** Kontext enthält zusätzlich Genossenschafts-Bankdaten aus der Config (dieselbe Quelle
  wie `send_confirmation_mail`): `bank_iban`, `bank_name`, `bank_bic`, `genossenschaft_name`.
- **D-06:** Offener Betrag als vorformatierter String unter `open_amount` (z. B. `"1.234,56 €"`).
  Berechnung `shares × share_value_cents`, formatiert via `format_eur_de`.
- **D-07:** `application_to_template_context` ist pure, synchron; nimmt aufgelöste Config-Werte
  als Parameter (KEIN eigener `config.get().await`). Service-Layer (Phase 31) löst Config auf.
- **D-08:** Probe-Render gegen einen Dummy-Application-Kontext (fester Sentinel, analog
  `validate_template_with_repayment`/`dummy_repayment_context`) — kein DB-Zugriff, deterministisch.
- **D-09:** Generischer `validate_rendered(subject, body, &[Value])`-Kern extrahiert; bestehendes
  `validate_template` UND neues `validate_application_template` rufen beide diesen Kern. Signatur
  von `validate_template` bleibt unverändert → ~40 bestehende Tests bleiben grün.
- **D-10:** Antragsteller-Vorlagen-Validierung greift bei Create/Update der Vorlage. Kein
  `strict`-Render-Crash beim späteren Versand. *(Research note: current member flow validates at
  send-time, not create/update — see drift finding 1.)*
- **D-11:** `format_eur_de(cents: i64) -> String` lebt in `genossi_service` (neben `iban::mask_iban`).
  Deutsches Format: Tausenderpunkt, Dezimalkomma, `€`-Suffix.
- **D-12:** `send_confirmation_mail` auf `format_eur_de` umstellen (ersetzt naives
  `format!("{},{:02} €")`).
- **D-13:** `format_eur_de` behandelt Null (`0,00 €`) und Negativ (`-1.234,56 €`) korrekt, direkt
  getestet.
- **D-14:** Seed-Vorlage „Zahlungserinnerung": formeller Ton, `template_type = 'application'`,
  fixe UUID `00000000-0000-0000-0000-000000000003`, `INSERT OR IGNORE` in eigener Seed-Migration.

### Claude's Discretion
- Exakter Wortlaut/Formatierung des Seed-Vorlagen-Textes (Betreff + Body), solange formell,
  deutsch, alle in D-14 genannten Platzhalter enthalten und strict-render-sicher.
- Genaue Sentinel-Werte des Dummy-Application-Kontexts (D-08), solange auffällig/deterministisch.
- Ob `body_html` für den Seed gesetzt wird oder text-only (NULL-Legacy = text-only ist ok).

### Deferred Ideas (OUT OF SCOPE)
- Mehrstufige Erinnerungs-Vorlagen (APTPL-FUT-01) — eigene Zukunfts-Phase.
- Versand + REST-Endpoints + Guardrails → Phase 31.
- Frontend-Compose-Dialog + Template-Selector-Filterung im UI → Phase 32.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| APTPL-01 | Vorlagen gegen Application-Kontext rendern; eigener Antragsteller-Typ (D1) | `member_to_template_context` (`template.rs:16`) is the exact template; `template_type` column (D-01) provides pool separation; all fields available on `Application` struct (`genossi_service/src/application.rs:11-27`) |
| APTPL-02 | Offener Betrag zur Laufzeit `shares × share_value_cents`, deutsche Euro-Formatierung | Config chain verified in `send_confirmation_mail` (`application.rs:55-97`); `app.shares: i32` confirmed; `format_eur_de` in `genossi_service/src/iban.rs` neighborhood |
| APTPL-03 | Deutsche Standard-Vorlage „Zahlungserinnerung" als Seed | Seed pattern verified (`20260416100001_seed_mail_templates.sql`); next UUID `…0003` |
| APTPL-04 | Validierung schlägt kontrolliert fehl; ~40 Member-Tests grün | `validate_template` (`template.rs:126`, signature `(&str, &str, &[MemberEntity])`), dummy-probe pattern (`dummy_repayment_context` `template.rs:258`, `validate_template_with_repayment` `template.rs:300`); strict env (`template.rs:65-69`) |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `template_type` column + roundtrip | DAO (impl) `genossi_mail/dao_sqlite.rs` | DAO trait/entity `dao.rs`, TO `rest_templates.rs` | Persistence + read-path exposure |
| `application_to_template_context` | Domain lib `genossi_mail/template.rs` | — | Pure function, unit-testable, no I/O (D-07) |
| `format_eur_de` | Domain lib `genossi_service/iban.rs` (or new `euro.rs`) | Consumed by `template.rs` + `application.rs` | Reusable domain formatting (D-11) |
| `validate_rendered` core + `validate_application_template` | Domain lib `genossi_mail/template.rs` | REST/service call-site | Render probe, deterministic (D-08/D-09) |
| Config resolution for context builder | Service `genossi_service_impl/application.rs` (Phase 31) | — | Async I/O stays out of the pure builder (D-07) |
| Seed „Zahlungserinnerung" | Migration `migrations/sqlite/` | — | Fixed-UUID `INSERT OR IGNORE` (D-14) |

## Verified Code Facts

### 1. `genossi_mail/src/template.rs` (all line numbers CONFIRMED)

| Symbol | Line | Signature / shape | Notes |
|--------|------|-------------------|-------|
| `member_to_template_context` | 16 | `pub fn member_to_template_context(entity: &MemberEntity) -> Value` | Uses `minijinja::context!{}`. Fields exposed: `member_number, first_name, last_name, email, company, comment, street, house_number, postal_code, city, join_date, shares_at_joining, current_shares, current_balance, exit_date, bank_account, masked_bank_account, migrated, salutation, title`. `salutation` mapped via `entity.salutation.as_ref().map(|s| s.as_str().to_string())`. |
| `strict_env` | 65 | `fn strict_env() -> minijinja::Environment<'static>` | `UndefinedBehavior::Strict` — undefined vars error at render. This is the mechanism behind APTPL-04. |
| `render_template` | 71 | `pub fn render_template(template_str: &str, context: &Value) -> Result<String, TemplateError>` | Text path. |
| `html_env` | 89 | `pub fn html_env() -> minijinja::Environment<'static>` | Strict + HTML autoescape. |
| `validate_template` | 126 | `pub fn validate_template(subject: &str, body: &str, members: &[MemberEntity]) -> Result<(), Vec<String>>` | **Signature MUST NOT change (D-09).** Builds a `member_to_template_context` per member internally, then probe-renders subject+body. |
| `merge_repayment_context` | 197 | `pub fn merge_repayment_context(base: Value, ...) -> Value` | Round-trips base `Value` → `serde_json` `BTreeMap` → `Value::from_serialize` because minijinja 2.19 has no `context!{ ..base }` spread. **Reuse this technique** if the application context needs to merge config-values into a base. |
| `dummy_repayment_context` | 258 | `pub fn dummy_repayment_context() -> (&'static str, i32, &'static str, i32)` | Returns sentinel `("99,99", 99, "99,99", 2099)`. **Direct model for the dummy application context (D-08).** |
| `validate_template_with_repayment` | 300 | `pub fn validate_template_with_repayment(subject, body, members: &[MemberEntity]) -> Result<(), Vec<String>>` | **Direct model for `validate_application_template`.** Probe-renders against a merged (dummy) context. |

**Test count:** 56 `#[test]`/`#[tokio::test]` in `template.rs` (the "~40 member-template tests"
in CONTEXT is an approximation — the exact figure is 56, of which the member-only render tests
plus footer/repayment/masked-bank tests all pass through `member_to_template_context`,
`validate_template`, or `render_template`). **None must break.** The `validate_template`
signature is exercised at `template.rs:491,498,507`.

**`validate_rendered` extraction shape (recommended, satisfies D-09):**
```rust
// Pure core: caller supplies already-built contexts.
fn validate_rendered(subject: &str, body: &str, contexts: &[Value]) -> Result<(), Vec<String>> {
    // syntax-check subject+body once, then probe-render each context (mirror current body of validate_template)
}
// Unchanged public signature — builds member contexts, delegates to core:
pub fn validate_template(subject: &str, body: &str, members: &[MemberEntity]) -> Result<(), Vec<String>> {
    let ctxs: Vec<Value> = members.iter().map(member_to_template_context).collect();
    validate_rendered(subject, body, &ctxs)
}
// New: builds ONE dummy application context, delegates to core.
pub fn validate_application_template(subject: &str, body: &str) -> Result<(), Vec<String>> {
    let ctx = dummy_application_context(); // sentinel values, see D-08
    validate_rendered(subject, body, std::slice::from_ref(&ctx))
}
```
Note the current per-member error messages embed `member.member_number`
(`template.rs:154,160`). The extracted core must keep that message shape for the member path
(or accept a label per context) so member-facing error strings don't regress the tests.

### 2. `genossi_service_impl/src/application.rs` §`send_confirmation_mail` (CONFIRMED)

- Function at **line 44**: `async fn send_confirmation_mail(&self, app: &Application)`.
- Config resolution chain (lines 55-97), all via `self.config_service.get("<key>").await`:
  - `share_value_cents` (55-67) — parsed `entry.value.parse::<i64>()`.
  - `bank_iban` (69-75), `bank_name` (77-83) — required, early-return on error.
  - `bank_bic` (85-89) — **optional**: `config.get("bank_bic").await.ok().map(|e| e.value.to_string())` → `Option<String>`.
  - `genossenschaft_name` (91-97) — required.
- **Naive euro-format to REPLACE (D-12), lines 99-102:**
  ```rust
  let total_cents = share_value_cents * app.shares as i64;
  let euros = total_cents / 100;
  let cents = total_cents % 100;
  let amount_str = format!("{},{:02} €", euros, cents);   // ← no thousands separator
  ```
  Replace with `let amount_str = format_eur_de(total_cents);`. There is **one** e2e/assertion
  surface that may check the confirmation-mail body wording — planner should grep the e2e tests
  for the amount format and update any expected string (blast radius is this single site).
- Application field accessors confirmed: `app.shares` (used lines 99, 125), `app.salutation`
  (104), `app.first_name`/`app.last_name` (105-106, 127), `app.email` (45). **`shares` is the
  field name — NOT `current_shares`** (verified in `Application` struct, next section).

### 3. `genossi_service/src/application.rs` — `Application` struct (CONFIRMED)

`pub struct Application` (line 11) fields: `id: Uuid`, `first_name: Arc<str>`,
`last_name: Arc<str>`, `salutation: Option<Salutation>`, `title: Option<Arc<str>>`,
`email: Option<Arc<str>>`, `street/house_number/postal_code/city: Option<Arc<str>>`,
**`shares: i32`**, `status: ApplicationStatus`, `created`, `deleted`, `version`.
`Salutation` (`genossi_dao/src/member.rs:9`) is `Herr`/`Frau` with `as_str()` → `"Herr"`/`"Frau"`
— matches the existing anrede logic `{% if salutation == "Herr" %}` (D-04 compatibility holds).

### 4. `mail_templates` — every site that must gain `template_type` (D-02)

**Migration schema today** (`20260416100000_create_mail_templates_table.sql`):
columns `id, created, deleted, version, name, subject, body`; plus `body_html` added later by
`20260702000000_mail_templates_add_body_html.sql`. So the live table has 8 columns.

**All `mail_templates` SQL/struct sites in `genossi_mail/src/dao_sqlite.rs`:**

| Site | Line | Current column list | Action |
|------|------|---------------------|--------|
| `MailTemplateDb` struct | 1178-1188 | `id, created, deleted, version, name, subject, body, body_html` | add `template_type: String` field |
| `TryFrom<&MailTemplateDb>` | 1190-1206 | maps 8 fields | map `template_type` into entity |
| `create` INSERT | 1226-1227 | `INSERT INTO mail_templates (id, created, deleted, version, name, subject, body, body_html) VALUES (?,?,NULL,?,?,?,?,?)` | add `template_type` column + bind |
| `update` UPDATE | 1249 | `UPDATE mail_templates SET name=?, subject=?, body=?, body_html=?, version=?, deleted=? WHERE id=?` | add `template_type=?` (or leave immutable — see note) |
| `dump_all` SELECT | 1267 | `SELECT id, created, deleted, version, name, subject, body, body_html FROM mail_templates` | add `template_type` |
| `find_by_id` SELECT | 1282 | `SELECT ... FROM mail_templates WHERE id=? AND deleted IS NULL` | add `template_type` |
| `all` SELECT | 1295 | `SELECT ... FROM mail_templates WHERE deleted IS NULL ORDER BY name ASC` | add `template_type` |
| `find_by_name` SELECT | 1310 | `SELECT ... FROM mail_templates WHERE name=? AND deleted IS NULL` | add `template_type` |

**Entity / trait / TO:**
- `MailTemplate` entity — `genossi_mail/src/dao.rs:238-249`: add `pub template_type: Arc<str>` (or a small enum). Trait `MailTemplateDao` (dao.rs:253-260) needs no signature change unless a `all_by_type`/filter method is added (defer the filter method to Phase 32 per D-03; Phase 30 only needs the field on the read path).
- `MailTemplateTO` — `genossi_mail/src/rest_templates.rs:20-32` + `From<&MailTemplate>` (60-72): add `template_type` so Phase-32 selector can filter (D-03). `CreateMailTemplateRequest`/`UpdateMailTemplateRequest` (34-58): decide whether create accepts a `template_type` (needed so the board can create application templates via the same CRUD) — recommend adding it with a `'member'` serde default to stay backward-compatible.
- `mail_template_service.rs` `create`/`update` (lines 42-60 trait, 78-162 impl): if `template_type` becomes settable, thread it through the `MailTemplate` construction (lines 98-108, 146-158).
- **Frontend** `genossi-frontend/src/api.rs:1494` has its own `MailTemplateTO` — Phase 30 backend exposes the field; the frontend struct + selector filter is Phase 32 (D-03). Not required to change in Phase 30, but adding the optional field to the FE TO now is harmless.

**Legacy roundtrip guarantee:** `template_type TEXT NOT NULL DEFAULT 'member'` means every
pre-existing row (and the 2 seeds) reads back as `'member'`. Because the column is `NOT NULL
DEFAULT`, the `MailTemplateDb.template_type` can be a plain `String` (no `Option`). If instead
the migration used a nullable column, the read path would need `unwrap_or("member")` — the
`NOT NULL DEFAULT` form is simpler and is the D-01 choice.

**Existing seeds (D-14 pattern):** `20260416100001_seed_mail_templates.sql` inserts two rows
with fixed UUIDs `X'00000000000000000000000000000001'` (version `…011`, "Formelle Anrede") and
`X'…0002'` (version `…022`, "Informelle Anrede") via `INSERT OR IGNORE`. New seed continues the
series: id `X'…0003'`, a fresh version blob (e.g. `X'…0033'`), `template_type = 'application'`.

### 5. `genossi_service/src/iban.rs` — home for `format_eur_de` (CONFIRMED)

- `mask_iban` at line 58, `group_iban` at line 26 — pure display helpers, no validation. Module
  is wired as `pub mod iban;` in `genossi_service/src/lib.rs:11`. `format_eur_de` can live either
  in `iban.rs` (D-11 says "neben iban::mask_iban") or a new sibling `pub mod euro;`. Both are
  re-exported the same way (`genossi_service::iban::format_eur_de` or `genossi_service::euro::format_eur_de`).
  Recommend a dedicated module `euro.rs` for clarity, but D-11's literal wording permits `iban.rs`.
- Callers import via full path, e.g. `member_to_template_context` already uses
  `genossi_service::iban::mask_iban` (`genossi_mail/src/template.rs:29`) — so `genossi_mail`
  already depends on `genossi_service`; `format_eur_de` is reachable from the context builder
  with no new dependency edge.

### 6. Migrations — additive forward-only pattern (CONFIRMED)

- Add-column model (`20260702000000_mail_templates_add_body_html.sql`): single statement
  `ALTER TABLE mail_templates ADD COLUMN body_html TEXT NULL;` with a forward-only comment
  (SQLite < 3.35 cannot drop columns; no down migration). For Phase 30:
  `ALTER TABLE mail_templates ADD COLUMN template_type TEXT NOT NULL DEFAULT 'member';`
- **Latest migration timestamp is `20260812000000`** (Phase 29 `mail_recipients_add_application_id`).
  New migrations must sort after it — recommend `20260813000000_mail_templates_add_template_type.sql`
  and `20260813000001_seed_zahlungserinnerung_template.sql`.
- Seed model (`20260416100001_seed_mail_templates.sql`): `INSERT OR IGNORE INTO mail_templates
  (...) VALUES (X'...', datetime('now'), X'...', 'Name', 'Subject', 'Body')`. The new seed must
  also set `template_type` explicitly (`'application'`) since it inserts a full row.

## Architecture Patterns

### Data flow (Phase 30 additions in **bold**)
```
Template CRUD (rest_templates.rs create/update)
        │  (D-10: NEW validate_application_template call-site — see Pitfall 1)
        ▼
MailTemplateService.create/update ── sanitize_html ──► MailTemplateDao.create/update
        │                                                        │
        │                                                    (SQL now carries template_type)
        ▼                                                        ▼
   MailTemplate entity  ◄──────────────────────────────  mail_templates row
   (**+ template_type**)

Phase 31 send (out of scope here):
   config.get(share_value_cents,bank_*,geno_name)  ──►  **application_to_template_context(app, resolved_config…)**  ──►  strict render
                                                              │
                                                    **open_amount = format_eur_de(shares × share_value_cents)**

send_confirmation_mail (application.rs:44)  ──►  **amount_str = format_eur_de(total_cents)** (replaces lines 99-102)
```

### Pattern 1: Pure context builder taking resolved config as params (D-07)
```rust
// Source pattern: genossi_mail/src/template.rs:16 (member_to_template_context)
// New — no async, no config.get(); Phase-31 service resolves config and passes values in.
pub fn application_to_template_context(
    app: &Application,
    share_value_cents: i64,
    bank_iban: &str,
    bank_name: &str,
    bank_bic: Option<&str>,
    genossenschaft_name: &str,
) -> minijinja::Value {
    let salutation_str = app.salutation.as_ref().map(|s| s.as_str().to_string());
    let open_amount = genossi_service::euro::format_eur_de(share_value_cents * app.shares as i64);
    minijinja::context! {
        first_name => app.first_name.as_ref(),
        last_name => app.last_name.as_ref(),
        salutation => salutation_str,
        title => app.title.as_deref(),
        shares => app.shares,
        open_amount => open_amount,
        bank_iban => bank_iban,
        bank_name => bank_name,
        bank_bic => bank_bic,
        genossenschaft_name => genossenschaft_name,
    }
}
```

### Pattern 2: Dummy-context probe for validation (D-08) — mirror `dummy_repayment_context`
```rust
// Source: genossi_mail/src/template.rs:258 (dummy_repayment_context) + :300 (validate_template_with_repayment)
fn dummy_application_context() -> minijinja::Value {
    // sentinel values, deterministic & conspicuous (Claude's discretion on exact values)
    minijinja::context! {
        first_name => "DUMMY-VORNAME", last_name => "DUMMY-NACHNAME",
        salutation => "Herr", title => "Dr.", shares => 99,
        open_amount => "9.999,99 €", bank_iban => "DE00 0000 0000 0000 0000 00",
        bank_name => "DUMMY-BANK", bank_bic => "DUMMYBIC", genossenschaft_name => "DUMMY-EG",
    }
}
```

### Anti-Patterns to Avoid
- **Adding `config.get().await` inside the context builder** — violates D-07, breaks unit-testability. Resolve config in the Phase-31 service and pass values.
- **Storing the open amount on `ApplicationEntity`** — explicitly forbidden; always compute at render.
- **Changing `validate_template`'s signature** — breaks the 56 template tests. Extract a core underneath instead (D-09).
- **Naive euro `format!("{},{:02}")`** — the exact bug D-12 removes (no thousands separator). Route all euro strings through `format_eur_de`.
- **Reusing `member_to_template_context` for applications** — the strict-render bomb (member-only vars like `member_number` undefined for applicants). Separate builder + separate type (D1) is the whole point.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Strict undefined-var detection | Custom placeholder scanner | `strict_env()` + probe render (`template.rs:65`) | Already the established APTPL-04 mechanism |
| Merging config values into a base minijinja context | `context!{ ..base }` (unsupported in minijinja 2.19) | `merge_repayment_context` round-trip technique (`template.rs:197-234`) if needed | Documented minijinja limitation |
| IBAN display / German formatting home | New util crate | `genossi_service::iban` module neighborhood (`iban.rs`) | Existing domain-format home, already a dependency of `genossi_mail` |
| Anrede branching | New salutation logic | Reuse `{% if salutation == "Herr" %}` templates (member-compatible var names, D-04) | `Salutation::as_str()` returns `"Herr"/"Frau"` |

## Common Pitfalls

### Pitfall 1: Assuming member templates are validated at create/update (they are NOT)
**What goes wrong:** Planning "mirror the member create/update validation" produces a task with no
existing code to mirror — `mail_template_service.rs` create/update only sanitizes HTML.
**Why it happens:** D-10's wording implies a create/update validation point that lives at
send-time instead (`rest.rs:534-547`).
**How to avoid:** Treat application-template create/update validation as a **new additive**
injection. Recommended site: `MailTemplateServiceImpl::create`/`update`
(`mail_template_service.rs:78,114`) — call `validate_application_template` when
`template_type == "application"`, returning `MailTemplateError::BadRequest` on failure. This keeps
the "broken template never persists" guarantee (D-10 intent) and never touches the member path
(member create/update stays validation-free, so member tests are unaffected).
**Warning signs:** A task that says "extend the existing template create/update validation."

### Pitfall 2: Missing one of the 8 `mail_templates` SQL/struct sites (the Phase-29 lesson, D-02)
**What goes wrong:** A `SELECT` that omits `template_type` while the `MailTemplateDb` struct
requires it → sqlx column-count/`RowNotFound`-style decode error at runtime, or a silent read of
the wrong column.
**How to avoid:** Update all 8 sites in the table above **and** the `MailTemplateDb` struct +
`TryFrom` in one pass. Then run the e2e template CRUD test (`e2e_tests.rs:8606+`) which exercises
create→list→get→update→delete against real migrations.
**Warning signs:** Any `SELECT ... FROM mail_templates` whose column list differs from the struct.

### Pitfall 3: `template_type` immutability vs. update
**What goes wrong:** If `UPDATE` omits `template_type` but the entity carries it, an edit silently
preserves the DB value (fine) — but if the TO/service reconstructs the `MailTemplate` with a
defaulted `'member'` and the UPDATE includes `template_type=?`, editing an application template
could flip it to `'member'`.
**How to avoid:** Either (a) make `template_type` immutable after create (do NOT include it in the
`UPDATE SET` list, `dao_sqlite.rs:1249`), or (b) always carry the existing value through
`MailTemplateServiceImpl::update` (it already loads `existing` at line 123). Recommend (a):
simplest, matches "type is fixed at creation."
**Warning signs:** `template_type` appearing in the UPDATE SET clause without the service reloading it from `existing`.

### Pitfall 4: Euro rounding / integer division on negatives (D-13)
**What goes wrong:** `-1234` cents via `euros = c/100; cents = c%100` gives `-12` and `-34`, and
naive `format!("{},{:02}")` yields `-12,-34`. The sign and the two-digit cents must be handled on
the absolute value.
**How to avoid:** Compute on `cents.abs()`, format the integer part with thousands separators
manually (Rust `std` has no locale formatter), prepend `-` if negative, append ` €`. Unit-test
`0 → "0,00 €"`, `1234 → "12,34 €"`, `123456 → "1.234,56 €"`, `-123456 → "-1.234,56 €"`,
`100000000 → "1.000.000,00 €"`.

## Runtime State Inventory

This phase is additive schema + code, not a rename/refactor. But a light inventory:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `mail_templates` rows (2 seeds + any board-created) all become `template_type='member'` via DEFAULT | None — the `NOT NULL DEFAULT 'member'` migration backfills automatically |
| Live service config | Config keys `share_value_cents`, `bank_iban`, `bank_name`, `bank_bic`, `genossenschaft_name` read at runtime by `send_confirmation_mail` — **not seeded by any migration** (set via config service/UI in prod) | None in Phase 30 (builder takes values as params); Phase 31 resolves them |
| OS-registered state | None | None |
| Secrets/env vars | None new | None |
| Build artifacts | None | None |

**Nothing found requiring data migration** beyond the additive column DEFAULT backfill.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `format!("{},{:02} €")` inline euro | `format_eur_de(cents)` single formatter | This phase (D-12) | Correct thousands separator everywhere |
| Member-only template pool | `template_type` column, `'member'`/`'application'` | This phase (D-01) | Enables applicant templates + future multi-stage reminders |

**Deprecated/outdated:** The naive euro format at `application.rs:99-102` is removed by D-12.

## Security Domain

`security_enforcement` not disabled in config → included. Phase 30 adds **no** new REST
endpoints or auth surface (send + endpoints are Phase 31), so the attack surface is unchanged.

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | yes | Strict minijinja env (`strict_env`) rejects unknown/member-only placeholders at validation time (APTPL-04). Author HTML already sanitized via `crate::sanitize::sanitize_html` at create/update. |
| V6 Cryptography | no | No crypto introduced |
| V2/V3/V4 Auth/Session/Access | no (unchanged) | Existing template CRUD auth path untouched |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Template injection via applicant/config values | Tampering | Strict env renders values as data, not template; no `{{ }}` re-evaluation of interpolated values |
| HTML/script in `body_html` seed | Tampering (XSS downstream) | ammonia sanitize at create/update (existing); seed may stay text-only (D allows NULL body_html) |
| Data over-exposure (DSGVO) | Information disclosure | Application context exposes only applicant-provided fields + the cooperative's own bank data (D-05); no member-pool leakage (separate builder) |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Recommended validation injection at `MailTemplateServiceImpl::create/update` (vs. rest_templates.rs handler) | Pitfall 1 | Low — both are valid; planner picks. Either keeps member tests green. |
| A2 | `template_type` best kept immutable after create (Pitfall 3 option a) | Pitfall 3 | Low — a design choice; option (b) also works |
| A3 | New migration timestamps `20260813000000/…001` | §6 | Low — must simply sort after `20260812000000`; exact value is cosmetic |
| A4 | Config keys (`share_value_cents`, `bank_*`) exist at runtime in prod (not migration-seeded) | Runtime Inventory | Low for Phase 30 (builder takes params); Phase 31 must handle missing-key errors as `send_confirmation_mail` already does |

**No `[ASSUMED]` package claims** — this phase installs nothing new.

## Open Questions

1. **Should `CreateMailTemplateRequest` accept `template_type`, or is the applicant seed the only `'application'` row until Phase 32?**
   - What we know: Phase 30 needs the field on the read path (D-03) and the seed (D-14). Board-authored application templates need a create path eventually.
   - What's unclear: whether Phase 30 exposes create-with-type or defers it to Phase 32.
   - Recommendation: Add `template_type: Option<String>` (serde default `'member'`) to the create request now — cheap, unblocks manual application-template creation, backward-compatible. Validation (`validate_application_template`) then keys off the resolved type.

2. **Does any e2e/integration assertion pin the exact confirmation-mail amount string?**
   - What we know: D-12 changes the format from `12345,67 €`-style to `1.234,56 €`-style.
   - Recommendation: grep `genossi_bin/tests/e2e_tests.rs` for the amount/`Bitte überweisen` wording before editing; update expected strings in the same commit as the `format_eur_de` retrofit.

## Sources

### Primary (HIGH confidence — read this session)
- `genossi_mail/src/template.rs` — context builder, validate_template, dummy/repayment probe, strict env (lines 16, 65, 126, 197, 258, 300)
- `genossi_service_impl/src/application.rs` — send_confirmation_mail config chain + euro format (lines 44, 55-102)
- `genossi_service/src/application.rs` — Application struct (`shares: i32`, line 22)
- `genossi_mail/src/dao_sqlite.rs` — 8 mail_templates SQL/struct sites (lines 1178-1319)
- `genossi_mail/src/dao.rs` — MailTemplate entity + trait (238-260)
- `genossi_mail/src/rest_templates.rs` — MailTemplateTO + CRUD handlers
- `genossi_mail/src/mail_template_service.rs` — create/update (no validation today)
- `genossi_mail/src/rest.rs` — actual validate_template call-site (534-547)
- `genossi_service/src/iban.rs` — mask_iban neighborhood + lib.rs:11 wiring
- `migrations/sqlite/` — create/seed/add-body_html patterns; latest ts `20260812000000`
- `.planning/REQUIREMENTS.md` (APTPL-01..04, D1/D3), `.planning/phases/30-.../30-CONTEXT.md`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all in-tree, nothing new to install
- Architecture: HIGH — every CONTEXT line number verified; 2 drifts flagged
- Pitfalls: HIGH — drawn from live code + the Phase-29 column lesson
- Security: HIGH — no new surface, existing controls identified

**Research date:** 2026-08-20
**Valid until:** 2026-09-19 (stable internal codebase; ~30 days)
