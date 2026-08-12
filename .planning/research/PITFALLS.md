# Pitfalls Research

**Domain:** Applicant-email + reminder-templates + per-applicant communication history for a production German Genossenschaft member-mgmt app (Rust layered DAO/Service/REST + Dioxus, milestone v1.6)
**Researched:** 2026-08-12
**Confidence:** HIGH — every pitfall is grounded in the actual v1.6-relevant code paths (`genossi_service_impl/src/application.rs::send_confirmation_mail`, `genossi_mail/src/template.rs`, `genossi_mail/src/dao_sqlite.rs::get_member_communications`, `genossi_mail/src/service.rs::create_job`, `genossi_dao/src/application.rs`).

> **Phase labels below** are feature-area labels (roadmap not yet drawn):
> **P-TMPL** = Application template context & rendering · **P-AMT** = outstanding-amount computation · **P-HIST** = communication history / `application_id` linkage (DAO+migration) · **P-SEND** = mail send endpoint/service (dedup, status-guard, DSGVO) · **P-FE** = frontend compose dialog + timeline reuse.

---

## Critical Pitfalls

### Pitfall 1: `member_to_template_context` is `MemberEntity`-typed — there is no application path, and strict-env turns every missing variable into a hard render failure

**What goes wrong:**
`member_to_template_context(entity: &MemberEntity)` (template.rs:16) and `validate_template(subject, body, members: &[MemberEntity])` (template.rs:126) are hard-typed to `MemberEntity`. An `ApplicationEntity` cannot be passed. Worse, the render env is `UndefinedBehavior::Strict` (template.rs:65–69): any `{{ member_number }}`, `{{ current_shares }}`, `{{ join_date }}`, `{{ current_balance }}` in a template renders → **render error**, not empty string. If a shared template pool (member + application) is used, an application-context render of a member template dies on the first member-only variable. The worker path turns that into `mark_recipient_failed`; a direct-send path turns it into a 500 / silent non-send.

**Why it happens:**
The whole template subsystem was built for the bulk member/repayment flow. "Reuse the existing minijinja system" reads as free, but the context builder and the validator are the seam that is member-shaped. Developers copy `member_to_template_context`, delete a few fields, and forget the strict-env contract (`{% if X is defined %}` is required — plain `{% if X %}` on an undefined var still errors under strict; see the locked test `test_repayment_variable_missing_with_if_guard_renders_empty` at template.rs:693).

**How to avoid:**
- Add `application_to_template_context(&ApplicationEntity) -> minijinja::Value` next to the member one. Expose only fields Applications actually have: `first_name`, `last_name`, `salutation` (map via `Salutation::as_str()` so `{% if salutation == "Herr" %}` keeps working — same `Salutation` type is shared), `title`, `shares`, plus the computed `outstanding_amount`/`share_value` strings. Do **not** expose `member_number`/`join_date`/`current_shares`/`current_balance`.
- Add an `application`-specific `validate_template_for_application(subject, body, &[ApplicationEntity])` mirroring `validate_template`, so the template editor probe-renders against the *application* context, not members.
- **Decide the template-pool question explicitly** (it is already flagged as an open design question in PROJECT.md): a separate "Antragsteller" template type is far safer than a shared pool, because a shared pool means every member-only variable is a latent strict-render bomb for application sends. If a shared pool is chosen, restrict the variable-button UI (see Pitfall 12) and validate against *both* contexts.

**Warning signs:**
Template render errors mentioning `member_number`/`current_shares`/`join_date`; recipients stuck in `failed` with "undefined" errors; a copy of `member_to_template_context` that silently fills member fields with dummy/empty values.

**Phase to address:** P-TMPL

---

### Pitfall 2: Faking `member_id` (nil-UUID or `application_id`) to shoehorn applications through the member-shaped pipeline

**What goes wrong:**
`RecipientInput.member_id: Option<Uuid>` and `MailRecipient.member_id: Option<Uuid>` are the join key for the *member* communication timeline (`get_member_communications` JOINs `WHERE r.member_id = ?1`, dao_sqlite.rs:1082) and for `find_sent_member_ids_by_job_id` (used to attribute `MemberDocument`s). A developer wanting the timeline/attribution "to just work" is tempted to stuff the application's UUID into `member_id`. This poisons the member namespace: `find_sent_member_ids_by_job_id` (dao_sqlite.rs:357, filters `member_id IS NOT NULL`) starts returning application IDs; any future member with an id-collision assumption breaks; and the mail becomes forensically indistinguishable from a real member mail.

**Why it happens:**
`send_confirmation_mail` already sets the *correct* precedent — `RecipientInput { member_id: None }` (application.rs:132–135). But the moment you need a *timeline*, `member_id: None` produces an un-queryable orphan, so the pressure to reuse `member_id` is high.

**How to avoid:**
Keep `member_id: None` for all application sends (it is the existing, correct pattern). Add a **separate** `application_id: Option<Uuid>` column to `mail_recipients` (see Pitfall 6) and query the application timeline on that column. Never overload `member_id`.

**Warning signs:**
`RecipientInput { member_id: Some(app.id) }`; application UUIDs appearing in member communication timelines; `find_sent_member_ids_by_job_id` returning ids that are not in the `members` table.

**Phase to address:** P-HIST (must be settled before P-SEND wires the endpoint)

---

### Pitfall 3: Silent no-send — modelling the reminder on `send_confirmation_mail`, which returns `()` and swallows every failure

**What goes wrong:**
`send_confirmation_mail` returns nothing and on *every* failure branch (`no email`, `share_value_cents` missing/unparseable, `bank_iban`/`bank_name`/`genossenschaft_name` config missing, `create_job` error) it does `tracing::error!(...) ; return;` (application.rs:44–156). That is acceptable for a fire-and-forget confirmation, but if the Vorstand's "Zahlungserinnerung senden" button is wired the same way, the REST handler returns **200 OK while nothing was sent** — the Vorstand believes a reminder went out. For a payment reminder with legal/financial weight, that is the worst failure mode.

**Why it happens:**
`send_confirmation_mail` is explicitly called out in PROJECT.md as "bester Referenz-Code für den Service-Aufruf." Copying it wholesale copies the swallow-and-return error handling.

**How to avoid:**
The new mail-send service method must return `Result<MailJob, ServiceError>` and propagate: missing email → `ValidationError`/`BadRequest` (400), missing config → `InternalError` (500), `create_job` error → surfaced. The REST layer returns the created `MailJob` (id + status) so the frontend can show a real confirmation and link into the timeline. Do **not** reuse the `()`-returning helper for the user-triggered path.

**Warning signs:**
A send handler whose success path can't tell "queued" from "silently skipped"; frontend toast "E-Mail gesendet" with no job id; `tracing::error!` as the only signal that a reminder failed.

**Phase to address:** P-SEND

---

### Pitfall 4: Pre-membership history does NOT carry over — the member timeline shows nothing after `confirm()`

**What goes wrong:**
`confirm()` mints a **brand-new** member with `id = uuid_service.new_v4()` (application.rs:326), unrelated to `application.id`. Application mails were sent with `member_id: None` and (per Pitfall 2) keyed on `application_id`. `get_member_communications(new_member_id)` JOINs on `mail_recipients.member_id` — which is `None` for those rows — so **the newly-created member's timeline is empty**, even though the person received reminders as an applicant. The Vorstand loses the payment-reminder history exactly at the moment it matters (member just joined after a reminder).

**Why it happens:**
There is no link between `Application` and the `Member` it becomes — `confirm()` copies fields but stores no `application_id` on the member and no `member_id` back on the application. The two timelines are keyed on different columns with no bridge.

**How to avoid:** Pick one, decide it in P-HIST:
- **(a) Back-fill at confirm:** inside the `confirm()` transaction, `UPDATE mail_recipients SET member_id = ?new_member_id WHERE application_id = ?app_id` so the history flows into the member timeline. Clean, but mixes member+application keys on the row (both set).
- **(b) Union at read time:** the member timeline query also `UNION`s outbound rows whose `application_id` matches an application that converted into this member — requires storing `application_id` on the member (or a link table).
- **(c) Store `member_id` on the Application at confirm** (link column) and have the member timeline union on it.
Whichever is chosen, add an e2e test: applicant gets a reminder → confirm → member timeline shows the reminder.

**Warning signs:**
E2E gap: no test covering "reminder before membership → visible after confirm"; member detail timeline empty for members created via application-confirm.

**Phase to address:** P-HIST (linkage design) + verified in P-SEND/P-FE

---

### Pitfall 5: No status guard — reminding `Abgelehnt` / already-`Bestaetigt` (now-member) / withdrawn applicants

**What goes wrong:**
`confirm()` and `reject()` both guard `status == Offen` and return 409 otherwise (application.rs:314, 583). A naive mail endpoint has **no** such guard, so the Vorstand can fire a payment reminder at a rejected applicant (no legal basis — DSGVO problem, see Pitfall 8) or at a confirmed applicant who is now a Member and already gets member mail on a different channel (duplicate/confusing). Soft-deleted applications (`deleted.is_some()`) must also be unreachable — but `find_by_id` already filters `deleted.is_none()` (application.rs:126–136), so route the endpoint through the service `get`/`find_by_id`, not a raw dump.

**Why it happens:**
Reminders feel "always allowed"; the state machine lives on `confirm`/`reject` and is easy to omit on a new verb. Payment reminders only make sense for `Offen` (awaiting payment).

**How to avoid:**
In the send service method, load via `application_dao.find_by_id` (gets soft-delete filtering for free) and gate: allow send for `Offen`; for `Bestaetigt`/`Abgelehnt` either block (409) or require an explicit override flag with a warning in the UI. Add unit tests mirroring the existing `status != Offen → Conflict` tests.

**Warning signs:**
Send handler that never reads `application.status`; reminders landing on rejected applicants; QA can send to a soft-deleted application id.

**Phase to address:** P-SEND

---

### Pitfall 6: `mail_recipients` migration — the new `application_id` column must be nullable, forward-only, and every verbatim column list must be updated

**What goes wrong:**
The timeline linkage needs `application_id BLOB NULL` on `mail_recipients`. Three traps:
1. **NOT NULL** would break every legacy/member row (all pre-v1.6 recipients have no application) and the member send path.
2. SQLite here is **forward-only, no down-migration, no `DROP COLUMN`** (established policy, PROJECT.md ADR-2026-05-06). Get the column shape right the first time.
3. `dao_sqlite.rs` has the `mail_recipients` column list written out **verbatim in ~5 places** (INSERT at :272, SELECT by job at :295, next_pending at :311, backfill at :379, and the timeline UNION at :1067–1082). Miss one and you get a silent column-count/order mismatch or the new field never populated. `CommunicationEntryDb` / `CommunicationEntry` / `CommunicationEntryTO` (dao.rs:259, communication_rest.rs:29) must all learn the new field too if it is surfaced.

**Why it happens:**
No ORM; hand-written SQL with duplicated column lists is a known maintenance hazard in this codebase (v1.1 already logged `format_dt` duplication debt).

**How to avoid:**
Add `application_id BLOB NULL`. Grep every `mail_recipients` SQL string and update all of them in one commit. Extend the timeline query with an application branch (`WHERE r.application_id = ?1`) analogous to the member branch. Keep the existing `r.deleted IS NULL AND j.deleted IS NULL` soft-delete filters (dao_sqlite.rs:1083–1084) on the new branch.

**Warning signs:**
`SELECT`/`INSERT` column-count mismatch panics in tests; new column always NULL after send; a migration that tries `ALTER ... NOT NULL` or a down-migration file.

**Phase to address:** P-HIST

---

### Pitfall 7: Outstanding-amount arithmetic — negative/zero shares, cent modulo, and German formatting inconsistency

**What goes wrong:**
The reference computation is `total_cents = share_value_cents * app.shares as i64; euros = total_cents/100; cents = total_cents%100; format!("{},{:02} €", euros, cents)` (application.rs:99–102). Issues when reused broadly:
- **Negative shares:** `shares` is `i32`. The API validates `shares >= 1` on submit/update (application.rs:184, 625), but imported/legacy DB rows or a future path could carry `0` or negative. With a negative total, `total_cents % 100` is **negative**, producing garbage like `"0,-5 €"`. `{:02}` does not fix a negative value.
- **Zero shares:** yields `"0,00 €"` — a "please pay 0,00 €" reminder is nonsensical but not a crash; guard it.
- **Formatting inconsistency:** the confirmation path formats `"{},{:02} €"` (comma decimal, trailing €, **no** thousands separator → `1234,00 €`), while the repayment template path uses `format_payout_eur` / the `merge_repayment_context` strings `"X,YZ"` **without** the € sign (template.rs:180–202). If application templates borrow the repayment variables, authors get a different format than the confirmation mail. Pick one euro-formatter and reuse it.

**Why it happens:**
i64-cent math is correct for the happy path (the v1.1 decision to use i64 cents specifically to avoid float rounding is sound), so the edge cases (sign, zero, thousands separator, €-vs-no-€) get skipped.

**How to avoid:**
Extract a single `format_eur_de(cents: i64) -> String` helper (reuse the existing repayment formatter if the € placement is reconciled), compute `total = share_value_cents.checked_mul(shares.max(0) as i64)`, treat `shares <= 0` as an explicit validation/skip case, and unit-test negative, zero, and > 1000-euro inputs. Feed the formatted string into `application_to_template_context` as `outstanding_amount` so templates never do arithmetic.

**Warning signs:**
`"…,-N €"` in a preview; two different euro formats between confirmation and reminder mails; no test for `shares = 0`.

**Phase to address:** P-AMT (helper) consumed by P-TMPL

---

### Pitfall 8: DSGVO — emailing non-members without a lawful basis, and leaking member-only data into applicant templates

**What goes wrong:**
Applicants are **not** members. Two distinct risks:
1. **Lawful basis:** an applicant who submitted a Beitrittserklärung has a contract-initiation basis (Art. 6(1)(b) — Vertragsanbahnung) for a *payment reminder while `Offen`*. A **rejected** or withdrawn applicant has **no** such basis — reminding them is an unlawful send (ties directly to Pitfall 5's status guard). This is the DSGVO angle the Vorstand will be judged on.
2. **Data minimization / leakage:** if a shared template pool exposes member-only fields (bank account, balances) and those ever resolve for an application, or if the preview/test path echoes the applicant's real email, PII leaks. The codebase already has a hard rule for this in the member test-mail path: the test recipient is the **explicit** address from the request, *never* the resolved member email (template.rs docstring + `send_test_mail_with_template` note at service.rs:126–130). The same rule must bind the application preview/test path.

**Why it happens:**
"It's just a reminder" hides the consent question; the confirmation mail already exists so emailing applicants feels pre-blessed — but confirmation (transactional, at submit) and reminder (Vorstand-initiated, repeatable, to any status) are different legal animals.

**How to avoid:**
- Restrict sends to `Offen` (Pitfall 5); require explicit override for other states with a UI warning.
- Application template context exposes only `first_name/last_name/salutation/title/shares/outstanding_amount` — no member financial fields.
- Preview/test-send uses an explicit operator-supplied address, never `application.email` auto-resolved (mirror the member rule).
- Endpoint is Vorstand-only. Note: existing application methods gate on `MANAGE_MEMBERS_PRIVILEGE` (application.rs:24) — reuse that, not a new ad-hoc check.

**Warning signs:**
Send path with no status check; template var buttons offering `bank_account`/`current_balance` on an application; preview endpoint that emails the applicant's stored address.

**Phase to address:** P-SEND (basis + status) + P-TMPL (minimization)

---

### Pitfall 9: Accidental duplicate/spam sends — no idempotency on the compose button + the Dioxus form-reload footgun

**What goes wrong:**
`create_job` has no idempotency key; two clicks → two jobs → two reminder emails to the same applicant. On the frontend, the project has a **recorded, recurring** bug: `form` + `onsubmit` + `prevent_default` still reloads the page (MEMORY: `dioxus-form-onsubmit-page-reload`; hotfixes `c6f41fd`, `e245013`), which can re-fire a send or lose state. The Vorstand double-sending a payment reminder is both embarrassing and a (mild) DSGVO annoyance.

**Why it happens:**
The compose dialog reuses `component/mail_compose/`; if it's dropped into a `<form onsubmit>` the reload bug reappears, and without a disabled-while-pending button the send is trivially double-firable.

**How to avoid:**
Use the established pattern: `div` + `onclick` + `r#type: "button"` (never a submitting form); disable the send button while the request is in flight and after success; show the created job id as confirmation (ties to Pitfall 3). Optionally block re-send of an identical reminder within a short window at the service layer.

**Warning signs:**
`form { onsubmit: ... }` in the compose dialog; send button that stays enabled during the request; two identical mail jobs for one applicant in `get_jobs`.

**Phase to address:** P-FE (mostly) + P-SEND (optional dedup)

---

### Pitfall 10: Adding a field to the audited `ApplicationEntity` (e.g. `last_reminded_at`) ripples into the audit hash-chain and breaks locked tests

**What goes wrong:**
`ApplicationEntity` implements `Auditable` and `audit_fields()` returns exactly 11 fields; `test_auditable_fields_count` asserts `len == 11` (application.rs:69–92, 173). If someone tracks reminder state by adding a column to `Application` (e.g. `last_reminded_at`, `reminder_count`), they must (a) decide whether it belongs in `audit_fields()`, (b) update the locked count test, and (c) accept that every reminder now writes audit-log rows on a Member/Application-audited entity — noise in the verband-facing audit trail. The constraint is explicit: **new mail/communication entities are NOT audited; existing audited entities keep the macros.**

**Why it happens:**
"Just add a column to Application" is the path of least resistance for reminder bookkeeping, but Application is audited and mail entities deliberately are not.

**How to avoid:**
Keep reminder/communication state **out** of `ApplicationEntity`. Track it on the (non-audited) `mail_recipients`/`mail_jobs` rows via the new `application_id` linkage. Do not touch `audit_fields()` unless a genuinely audit-worthy application attribute is added — and then update the lock test deliberately.

**Warning signs:**
New nullable columns on `application`; `test_auditable_fields_count` edited to a new number; audit-verify output showing reminder events on `application` entities.

**Phase to address:** P-HIST (data-model decision)

---

### Pitfall 11: Optimistic-lock version handling on any application write path (documented footgun from Plan 25-05)

**What goes wrong:**
`update_application` requires `entity.version == update.version` (application.rs:654) and the DAO uses the **old** version in the optimistic-lock WHERE clause while generating a fresh v4 internally as the new version. The `confirm()` code carries an explicit warning comment (application.rs:496–502): passing a *new* UUID as the version to a DAO `update` makes every write blow up with `ConflictError("Version mismatch")` → 409. Any new application-side write introduced for v1.6 (unlikely if Pitfall 10 is heeded, but e.g. an "email edited before send" flow) that mishandles the version repeats this bug.

**Why it happens:**
The "pass old version, DAO bumps it" convention is codebase-wide but non-obvious; it already bit the team once in e2e Plan 25-05.

**How to avoid:**
If v1.6 adds no application write, this is moot — prefer that. If it does, follow the convention: pass the loaded entity's existing `version` to `update`; let the DAO mint the new one. Reuse the `update_application` version-check pattern verbatim.

**Warning signs:**
Spurious 409s on save; a hand-set `version = new_v4()` right before a DAO `update`.

**Phase to address:** P-SEND (only if an application write is added)

---

### Pitfall 12: Frontend — reusing member-bound `mail_compose` var-buttons and member API fns for applications

**What goes wrong:**
Two mismatches when maximizing reuse (a stated v1.6 goal):
1. `component/mail_compose/template_var_buttons.rs` / `template_selector` / `template_tester` present **member** variables. Dropped onto an application compose dialog, they let the Vorstand insert `{{ member_number }}`/`{{ current_shares }}` that are undefined in application context → strict-render failure at send (Pitfall 1).
2. `api.rs` is entirely `/api/members/{member_id}/...` shaped (fetch actions/documents/communications). There is no application-mail or application-communications client fn; reusing member fetchers silently hits the wrong URL. New endpoints are `POST /api/applications/{id}/mail` and `GET /api/applications/{id}/communications`.
Conversely, `CommunicationTimeline(entries: Vec<CommunicationEntryTO>)` (communication_timeline.rs:8) is **already decoupled** — it takes entries as a prop, so it *is* reusable as-is; the risk there is only that inbound rows can't exist for applications (assignment is member-only via `assigned_member_id`), so the application timeline is outbound-only — fine, but don't build inbound UI expecting data.

**Why it happens:**
"Maximum reuse of `mail_compose/` and `communication_timeline`" (PROJECT.md) is the plan; the timeline is genuinely reusable, which lulls one into assuming the var-buttons and API layer are too.

**How to avoid:**
- Give the compose dialog an **application variable set** (subset that exists in application context). If the template pool is shared, filter the var-buttons by context.
- Add dedicated `api.rs` fns for the two new application endpoints; do not reroute member fns.
- Reuse `CommunicationTimeline` directly (prop-based); expect outbound-only entries.
- Follow Pitfall 9's `div`+`onclick`+`r#type:"button"` and the Component-First rule (extract, don't inline-duplicate the compose block onto the application detail page — `component/application_detail.rs` already exists to host it).

**Warning signs:**
Member variable buttons on an application dialog; application compose calling `/api/members/...`; inline-copied compose RSX on the application page instead of a shared component.

**Phase to address:** P-FE

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Reuse `member_id` column for applications (skip new column) | No migration; timeline "just works" | Poisons member namespace; `find_sent_member_ids_by_job_id` returns non-members; forensically indistinguishable mails | **Never** |
| Copy `send_confirmation_mail` verbatim for the reminder | Fast; "reference code" | Swallows all errors → 200-OK-but-not-sent for a financially significant mail | **Never** for user-triggered send |
| Shared member+application template pool | One template list, less UI | Every member-only var is a strict-render bomb for application sends | Only with context-filtered var-buttons + dual-context validation |
| Track reminder state on `ApplicationEntity` | One place, familiar | Audit-chain noise + breaks locked field-count test | **Never** — use non-audited mail rows |
| Format euro inline per call site | Trivial | Confirmation mail (`… €`) and template vars (`X,YZ`, no €) diverge | Only if a single shared formatter is used everywhere |
| Inline the compose block on the application detail page | Skip a component | Violates Component-First; RSX duplication vs. member compose | **Never** (rule in CLAUDE.md) |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| minijinja strict-env | `{% if payout_amount %}` to guard an undefined var | Use `{% if X is defined %}` (strict errors on plain `{% if undefined %}`) |
| `config_service.get("share_value_cents")` | Assume present/parseable; use in a `()`-returning path | Propagate `ConfigMissing`/parse errors as `ServiceError` to the caller |
| `mail_recipients` SQL (hand-written, ~5 copies) | Update one column list, miss the others | Grep all `mail_recipients` strings; update in one commit; add a round-trip test |
| `application.email` | Treat as always-present `String` | It is `Option<Arc<str>>`; None → 400, never silent success |
| Salutation in templates | New enum mapping for applications | Reuse `Salutation::as_str()` (same type) so `== "Herr"/"Frau"` templates stay compatible |

## Security / Privacy Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Reminder to `Abgelehnt`/withdrawn applicant | Unlawful send (no Art. 6(1)(b) basis) | Status guard: default-allow only `Offen` |
| Shared pool exposes member financial fields to application render | PII leak / data-minimization breach | Application context excludes bank/balance fields |
| Preview/test emails the applicant's stored address | Unintended send to data subject | Explicit operator-supplied test address only (mirror member rule) |
| Endpoint not Vorstand-gated | Applicant PII exposed | Gate on `MANAGE_MEMBERS_PRIVILEGE` like other application methods |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No send confirmation (silent no-send) | Vorstand thinks reminder went out; applicant never paid-chased | Return + display the created `MailJob` id/status |
| Double-click / form-reload double send | Duplicate reminders to applicant | Disable-on-pending + `div`+`onclick`+`r#type:"button"` |
| Empty member timeline after confirm | "We never contacted them" despite reminders sent | Carry history over (Pitfall 4) with an e2e test |
| "Please pay 0,00 €" for 0-share edge | Nonsensical reminder | Guard `shares <= 0` before send |

## "Looks Done But Isn't" Checklist

- [ ] **Template rendering:** works for a normal applicant — verify it does NOT error on a template referencing a member-only var (strict-env), and that `{% if X is defined %}` guards are documented for authors.
- [ ] **Outstanding amount:** verify `shares = 0`, negative, and > 1000-euro inputs; verify € format matches the confirmation mail.
- [ ] **Timeline linkage:** verify an application reminder appears in the applicant timeline AND (post-`confirm`) in the resulting member's timeline.
- [ ] **Status guard:** verify send is blocked/warned for `Abgelehnt` and `Bestaetigt`, and impossible for soft-deleted applications.
- [ ] **Send confirmation:** verify the handler distinguishes "queued" from "config missing / no email" (no silent 200).
- [ ] **Idempotency:** verify double-click produces one job, not two; verify no `form onsubmit` reload.
- [ ] **Migration:** verify `application_id` is nullable, legacy member rows still send, and every `mail_recipients` SQL list is updated.
- [ ] **Audit untouched:** verify no new audit-log rows on `application` for a reminder; `test_auditable_fields_count` still 11.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| `member_id` overloaded with app ids (shipped) | HIGH | Migration to move ids to new `application_id`; scrub member timelines; audit `find_sent_member_ids_by_job_id` consumers |
| Silent no-send in production | MEDIUM | Add error propagation + return job status; re-send affected reminders once identified from logs |
| History doesn't carry over (shipped) | MEDIUM | Back-fill `mail_recipients.member_id` for converted applications by matching `application_id` → member link |
| Strict-render failures on shared pool | LOW | Split application template type OR add `is defined` guards + context-filtered var-buttons |
| Negative/zero-share formatting bug | LOW | Add `format_eur_de` with sign/zero handling + tests |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1 Member-typed context / strict-env | P-TMPL | Render test: application template with member-only var fails loudly; application context renders Anrede correctly |
| 2 Fake `member_id` | P-HIST | No application id in `member_id`; member timeline excludes application mails |
| 3 Silent no-send | P-SEND | Handler returns job id; config-missing → error, not 200 |
| 4 History carry-over | P-HIST + P-SEND | E2E: reminder → confirm → visible in member timeline |
| 5 Status guard | P-SEND | Unit: send to `Abgelehnt`/`Bestaetigt`/deleted rejected |
| 6 Migration/column | P-HIST | `application_id` nullable; all `mail_recipients` SQL updated; round-trip test |
| 7 Amount arithmetic | P-AMT | Unit: 0, negative, >1000€; format matches confirmation |
| 8 DSGVO basis/minimization | P-SEND + P-TMPL | Status-gated; context has no financial fields; test-send uses explicit address |
| 9 Duplicate send / reload | P-FE (+P-SEND) | Double-click → one job; no `form onsubmit` |
| 10 Audit ripple | P-HIST | No audit rows on `application`; field-count test intact |
| 11 Optimistic-lock version | P-SEND (if app write added) | No spurious 409; old-version convention followed |
| 12 Frontend member-bound reuse | P-FE | App var-button set; dedicated app API fns; timeline reused via prop |

## Sources

- `genossi_service_impl/src/application.rs` — `send_confirmation_mail` (amount/config/`member_id: None` reference), `confirm()` (new-member minting, optimistic-lock warning comment, status guards), `Auditable` field-count lock — HIGH
- `genossi_mail/src/template.rs` — `member_to_template_context`, `validate_template`, strict-env + `is defined` locked tests, `merge_repayment_context` euro formatting, test-send privacy note — HIGH
- `genossi_mail/src/dao_sqlite.rs` — `get_member_communications` UNION query (`WHERE r.member_id`, soft-delete filters), duplicated `mail_recipients` column lists — HIGH
- `genossi_mail/src/service.rs` — `create_job` signature, `RecipientInput.member_id`, explicit-address privacy rule — HIGH
- `genossi_dao/src/application.rs` — `ApplicationStatus` (Offen/Bestaetigt/Abgelehnt, no payment field), `Auditable` impl, soft-delete filtering in `all`/`find_by_id` — HIGH
- `genossi-frontend/src/component/{communication_timeline.rs, mail_compose/*}`, `api.rs` — prop-based timeline reuse, member-shaped var-buttons/API — HIGH
- Project memory: `dioxus-form-onsubmit-page-reload`; PROJECT.md constraints (audit rules, DSGVO whitelist, forward-only SQLite, open design questions) — HIGH

---
*Pitfalls research for: applicant-email + reminder-templates + per-applicant communication history (genossi v1.6)*
*Researched: 2026-08-12*
