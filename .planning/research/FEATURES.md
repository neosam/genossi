# Feature Research

**Domain:** Applicant-facing reminder/dunning email + prospect communication in a small German-cooperative (Genossenschaft) member-management tool
**Milestone:** v1.6 Antragsteller-Kommunikation
**Researched:** 2026-08-12
**Confidence:** HIGH (domain + German legal grounding), MEDIUM on exact template-variable set (product decision for discuss-phase)

## Framing: what this actually is

This is **not** an email-marketing / CRM feature and must not be scoped as one. The target user is a Vorstand of a small cooperative sending a **one-off, human-triggered, transactional email** to a specific person who has **already signed a Beitrittserklärung** — most often a friendly *Zahlungserinnerung* ("you signed up, please pay your Geschäftsanteil"). The whole design center is: reuse the existing member-mail machinery, add an Application recipient path, and add guardrails so nobody accidentally spams anyone.

Two legal facts anchor everything below:

1. A Beitrittserklärung is a **binding written declaration** in which the applicant obligates themselves to pay their share (§§ 15, 15a GenG). So the applicant is a **person with a pre-contractual/contractual obligation at their own request**, not a cold prospect. A reminder about *their own declared payment* is transactional under **Art. 6(1)(b) DSGVO** (and defensibly 6(1)(f)) — it does **not** require separate consent and is **not** §7-UWG advertising.
2. That protection is **content-scoped**. It covers reminders and process communication about *this person's own application*. It does **not** cover newsletters, "become a member" promotion, or unrelated marketing — those would be advertising to a non-member and need consent. This line is the single most important anti-feature boundary in this milestone.

## Feature Landscape

### Table Stakes (Users Expect These)

Features the Vorstand assumes exist. Missing = feature feels broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Send a single email to one applicant, recipient = `application.email` | The literal milestone goal; "E-Mail senden" button on Application-Detail | LOW | Reuse `MailService::create_job` + `send_confirmation_mail` path (already does `member_id: None`). New `POST /api/applications/{id}/mail` |
| Compose dialog reusing member mail-compose UI (subject + WYSIWYG body) | Vorstand already knows this UI from member mailing; consistency expected | LOW | Reuse `component/mail_compose/` + v1.4 `WysiwygEditor`. Component-First: no forked compose |
| Reusable reminder templates with applicant placeholders | Vorstand won't retype the same Zahlungserinnerung each time; already have member templates | MEDIUM | Reuse minijinja system; add `application_to_template_context`. Placeholders: Anrede, Name, Titel, Anzahl Anteile, offener Betrag |
| "Offener Betrag" computed, not stored | Applications have no payment-status field; amount = shares × `share_value_cents` | LOW–MEDIUM | Pure computed value. Decide share-value source (config/current RepaymentPhase default `10000`?) in discuss-phase |
| Per-applicant communication history / timeline | "Have we already reminded them? when?" is the core question a reminder feature must answer | MEDIUM | Reuse `communication_timeline.rs`; entries need `application_id` (applications have no `member_id`). New `GET /api/applications/{id}/communications` |
| Visible "last email/reminder sent on …" | Prevents duplicate/annoying reminders; single most-cited expectation for dunning UX | LOW | Derived from the timeline; surface latest entry prominently on Application-Detail |
| Confirm-before-send | Emailing a real person is irreversible; Vorstand expects a review step | LOW | Follow the v1.x preview-confirm pattern already used across the app |
| Correct sender identity / Absender | Recipient must see it's from the cooperative, not a random address | LOW | Already handled by `genossi_mail` SMTP config; verify From/Reply-To sane |
| Handle "no email address" gracefully | Some applicants submit on paper; button must not silently fail | LOW | Disable/annotate send when `application.email` empty; do not fabricate. (No paper-letter generation this milestone) |
| Admin-only gate | Member PII + outbound mail must be Vorstand-only | LOW | `RequirePrivilege "admin"`. Note carry-forward CR-02 permission-ordering caveat when wiring |

### Differentiators (Competitive Advantage)

Small, cheap touches that make this feel purpose-built for a Genossenschaft — not a generic mailer. Align with Core Value (less manual work, nachvollziehbar).

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Prefilled "Zahlungserinnerung" template as first-class default | Turns the #1 use case into ~2 clicks; the actual reason this milestone exists | LOW | One good German template shipped as seed content beats a template editor nobody fills |
| Live template preview with resolved placeholders before send | Vorstand sees real Anrede/Name/Betrag, catches "{{ }}" mistakes and wrong amounts | MEDIUM | Reuse minijinja render; render into the compose preview. Guards against embarrassing mis-merges |
| "Offener Betrag" shown in the UI (not just in the mail) | Vorstand verifies the number before it goes out; ties reminder to the concrete obligation | LOW | Same computed value; display in compose header |
| Correct German salutation from `Salutation` + `title` | "Sehr geehrte Frau Dr. …" done right signals professionalism to a not-yet-member | LOW | Enum already exists on member side; mirror for application context |
| Timeline entry records template used + subject | "We sent the standard 1st reminder on 03.07." is exactly the audit story a Vorstand wants | LOW–MEDIUM | Store subject + template id on the communication entry |
| Deep-link from timeline entry back to the sent content | Answer "what exactly did we write?" without digging in the mail server | MEDIUM | Depends on how much of the body is persisted; may store body snapshot like member mail already does |

### Anti-Features (Commonly Requested, Often Problematic)

Document these explicitly to stop scope creep. Everything here is out of scope for v1.6.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Bulk / mass reminder to all "Offen" applicants | "Just remind everyone who hasn't paid" | Milestone is deliberately single-send; bulk = recipient-selection UI, throttling, partial-failure handling, per-recipient audit — a whole separate milestone. Also raises the spam risk this feature is trying to avoid | Send individually; explicitly a Future-Requirement (PROJECT.md already states this) |
| Automated dunning schedule / auto-escalation (reminder → 2nd → Mahnung) | "The system should chase them automatically" | Time-triggered outbound mail to individuals is high-risk (wrong-amount, already-paid, deceased, withdrawn). Needs payment-status tracking the data model doesn't have. One bad cron = many angry non-members | Keep it human-triggered. Vorstand decides when to remind, sees last-sent date |
| Tracking pixels / open + click tracking | "Did they read it?" | DSGVO-hostile (needs consent, Aufsichtsbehörden actively warn), pure overhead for a nonprofit tool, erodes trust | None. Out of scope. Rely on the fact that a payment either arrives or doesn't |
| Newsletter / "become a member" marketing to applicants | "While we have their email…" | This crosses from transactional into **advertising to a non-member** → §7 UWG consent territory. Exactly the case where the Warenkorb-Erinnerung rulings bite | Keep every applicant email tied to *their own application/payment*. No promotional content |
| Formal legal "Mahnung" with Verzug/interest/fees | "Make it a real dunning notice" | Legal formality (Verzug, §288 interest, fees) is inappropriate for a first friendly reminder and for a cooperative's relationship tone; risk of overstating claims | Ship a *friendly Zahlungserinnerung* tone. Formal Mahnung, if ever, is a manual Vorstand decision outside the tool |
| Stored payment-status field on Application (paid/unpaid) | "Track who paid so we know who to remind" | Real payment reconciliation (bank matching) is a large separate feature; a half-tracked flag rots and misleads reminders | Compute "offener Betrag" on the fly; Vorstand judges paid/unpaid from their own bank view. Defer real tracking |
| Attachments / generated PDF letters to applicants | "Attach the invoice / Beitrittsbestätigung" | v1.4 already added application-document carryover on confirm; adding attach-on-send here widens scope and file-lifecycle concerns | Plain formatted HTML mail (v1.4 pipeline). Attachments are Future-Requirement |
| Free-text arbitrary recipient (email to anyone) | "Let me just type an address" | Bypasses the applicant binding that makes this transactional-and-lawful; turns the feature into an open mailer | Recipient is always `application.email` of the specific Application |
| Reply/inbox threading for applicant replies | "See their reply here too" | Inbox/reply is the v1.3 subsystem scoped to the general inbox; wiring applicant-reply threading is separate | Applicant replies land in the existing inbox; not threaded into the applicant timeline this milestone |

## Feature Dependencies

```
Send single email to Application
    └──requires──> Application recipient path (RecipientInput member_id: None)   [exists: send_confirmation_mail]
    └──requires──> MailService::create_job                                       [exists]
    └──requires──> Admin permission gate                                         [exists: RequirePrivilege "admin"]

Reusable applicant templates
    └──requires──> application_to_template_context (Anrede/Name/Titel/Anteile/Betrag)   [NEW]
    └──requires──> minijinja template system                                     [exists, member-only today]
    └──enhances──> Send single email (prefilled body)

"Offener Betrag" placeholder
    └──requires──> share-value source decision (config vs RepaymentPhase default)  [DECISION]
    └──computed from──> application.shares × share_value_cents

Per-applicant communication history
    └──requires──> application_id on Mail-Job / communication entry              [NEW: model + migration]
    └──requires──> GET /api/applications/{id}/communications                     [NEW]
    └──enables──> "last reminder sent on …" display
    └──reuses──> communication_timeline.rs component                             [exists]

Compose dialog on Application-Detail
    └──requires──> component/mail_compose/ + WysiwygEditor                       [exists]
    └──requires──> all of the above wired to Application context

Bulk reminder  ──conflicts/deferred──> single-send scope (explicitly Future)
Auto-dunning   ──conflicts/deferred──> human-triggered design (explicitly out)
```

### Dependency Notes

- **The timeline is the load-bearing new data work.** Everything else is largely reuse. The one genuinely new persistence concern is attaching `application_id` (applications have no `member_id`) to mail-job / communication entries so the timeline and "last sent" can exist. Get the model/migration right early; the rest layers on cheaply.
- **Template context is the second real piece of new code.** `application_to_template_context` mirrors the member context but sources fields from Application. Keep the placeholder set a **subset** shared with member templates where names overlap (Anrede, Name, Titel) so a shared pool stays viable.
- **"Offener Betrag" has one open decision:** where `share_value_cents` comes from (a config value, or the latest RepaymentPhase default `DEFAULT_SHARE_VALUE_CENT=10000`, or a per-Genossenschaft setting). Flag for discuss-phase; it's a small decision but it's a *money number in an email*, so it must be deliberate.
- **Compose reuse depends on Component-First discipline.** Do not fork `mail_compose/`; extend it to accept an Application-context source. Forking would duplicate the WYSIWYG + preview and violate the project's hardest-learned lesson.

## MVP Definition

### Launch With (v1.6)

- [ ] Send single transactional email to an Application (recipient = `application.email`), admin-only — the core deliverable
- [ ] Compose dialog on Application-Detail reusing `mail_compose/` + `WysiwygEditor` — no forked UI
- [ ] Reusable templates with applicant placeholders (Anrede, Name, Titel, Anteile, offener Betrag) via `application_to_template_context`
- [ ] Live preview with resolved placeholders + confirm-before-send — prevents mis-merges and accidental sends
- [ ] Per-applicant communication timeline (`application_id`-linked) + `GET /api/applications/{id}/communications`
- [ ] Prominent "last email sent on …" on Application-Detail — the anti-spam guardrail users expect
- [ ] Graceful "no email address" handling (disabled/annotated button)
- [ ] One shipped default Zahlungserinnerung template (German, friendly tone)

### Add After Validation (v1.x)

- [ ] Template used + subject recorded on timeline entry (richer history) — add if Vorstand asks "which reminder did we send?"
- [ ] Body snapshot / deep-link to exact sent content — add if reconstructing sent mails becomes a real need
- [ ] Second/reminder-tier template variants (still manual send) — add once first-reminder flow is proven

### Future Consideration (v2+)

- [ ] Bulk reminder to all "Offen" applicants — explicitly deferred in PROJECT.md; needs selection UI + throttling + per-recipient audit
- [ ] Real applicant payment-status tracking (bank reconciliation) — large separate feature; only then does auto-dunning become defensible
- [ ] Attachments / generated PDF to applicants — depends on file-lifecycle work
- [ ] Applicant-reply threading into the timeline — depends on v1.3 inbox integration

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Single send to Application (recipient path + endpoint) | HIGH | LOW | P1 |
| Compose dialog reuse on Application-Detail | HIGH | LOW | P1 |
| Applicant template context + placeholders | HIGH | MEDIUM | P1 |
| "Offener Betrag" computed + shown | HIGH | LOW–MEDIUM | P1 |
| Communication timeline (`application_id`) + endpoint | HIGH | MEDIUM | P1 |
| "Last sent on …" display | HIGH | LOW | P1 |
| Confirm-before-send + live preview | HIGH | LOW–MEDIUM | P1 |
| Default Zahlungserinnerung seed template | MEDIUM | LOW | P1 |
| No-email graceful handling | MEDIUM | LOW | P1 |
| Template/subject recorded on timeline entry | MEDIUM | LOW–MEDIUM | P2 |
| Body snapshot / deep-link to sent content | MEDIUM | MEDIUM | P2 |
| Bulk reminder | MEDIUM | HIGH | P3 |
| Auto-dunning schedule | LOW (risky) | HIGH | P3 (anti) |
| Open/click tracking | LOW | MEDIUM | P3 (anti) |

## Compliance Notes (German / DSGVO — for the requirements author)

These are requirements, not background. Group them under a "Compliance / Guardrails" heading.

- **Legal basis is transactional, content-scoped.** A payment reminder about the applicant's *own* Beitrittserklärung is lawful under Art. 6(1)(b) DSGVO (pre-contractual/contractual at the data subject's request) without separate consent — because the join declaration binds them to pay (§§ 15/15a GenG). **Requirement:** keep all applicant email content tied to *their own application and payment*. No marketing, no newsletter, no cross-promotion.
- **Do not import the "abandoned cart" rule by mistake.** Case law (Aufsichtsbehörden, IT-Recht) that says reminder emails to a prospect need §7-UWG consent applies to *merchant-initiated marketing to someone with no obligation*. That is a different situation from a signed Beitrittserklärung. The distinguishing test for every email: **is this about fulfilling a duty the person already declared, or is it promotion?** Only the former is in scope.
- **Data minimization.** Template placeholders should stay limited to what a reminder needs (Anrede, Name, Titel, Anteile, offener Betrag). Don't expose more applicant PII than necessary — consistent with the project's DSGVO-whitelist pattern.
- **Transparency / sender identity.** Emails must show a clear cooperative Absender and a real Reply-To so the recipient can respond or object. (Absender already handled by `genossi_mail` SMTP config — verify, don't rebuild.)
- **Right to object / stop contacting.** A person can ask not to be emailed. For a small manual tool this is a **process guardrail, not automation**: the visible timeline + human-triggered send means the Vorstand can honor a "don't email me" request by simply not sending. No suppression-list engine needed at this scale — but the requirement to respect it should be stated.
- **Audit scope unchanged.** Application is already an audited entity (existing audit macros stay mandatory for Application writes). Pure mail/communication entities do **not** need audit macros (per PROJECT.md). Sending an email is not an Application mutation — don't accidentally pull it into the audit hashchain, but do record it in the (non-audited) communication timeline.

## Scope-Creep Flags (explicit — for the roadmap)

Anything below is **beyond single-send reminders** and should be pushed back if it appears in requirements:

1. Any recipient selection / multi-select / "send to all Offen" → bulk, deferred.
2. Any time-triggered or worker-driven outbound applicant mail → auto-dunning, out.
3. Any open/click/pixel tracking → DSGVO-hostile, out.
4. Any promotional/newsletter content to applicants → §7-UWG risk, out.
5. Any persisted payment-status flag or bank reconciliation → separate feature, out.
6. Any attachment/PDF-to-applicant generation → depends on file lifecycle, out.
7. Any formal Mahnung mechanics (Verzug/interest/fees) → wrong tone + legal complexity, out.
8. Any free-text arbitrary-recipient mailer → breaks the transactional/lawful binding, out.

## Sources

- [Beitrittserklärung/Beteiligungserklärung (§§ 7a, 15, 15a, 15b GenG) — GV Weser-Ems](https://www.gvweser-ems.de/DE/Beraten/gruendungsberatung/Praxistipps/BeitrittserklrungundbertragungvonGeschftsguthaben.pdf)
- [Genossenschaftsgesetz (GenG) — gesetze-im-internet.de](https://www.gesetze-im-internet.de/geng/GenG.pdf)
- [Begründung und Beendigung der Mitgliedschaft in der Genossenschaft — wohnungswirtschaft.online](https://wohnungswirtschaft.online/begruendung-und-beendigung-der-mitgliedschaft-in-der-genossenschaft/)
- [Zahlungserinnerung: Vorlagen, Tipps und rechtliche Grundlagen — acquisa](https://www.acquisa.de/magazin/zahlungserinnerung)
- [Warenkorberinnerungen per E-Mail: unzulässige Werbung (Aufsichtspraxis, DSGVO, UWG) — IT-Recht Kanzlei](https://www.it-recht-kanzlei.de/warenkorberinnerungen-per-email-unzulaessig.html)
- [Warenkorb-Erinnerungsmails zwischen Aufsichtspraxis, DSGVO und UWG — SLK Rechtsanwälte](https://www.slk-rechtsanwaelte.de/blog/warenkorb-erinnerungsmails-im-onlinehandel-zwischen-aufsichtspraxis-dsgvo-und-uwg/)
- [Können Mitglieder oder Kund*innen per Mail angeschrieben werden? — Datenschutzbeauftragter Hamburg](https://datenschutzbeauftragter-hamburg.de/2022/12/koennen-mitglieder-oder-kundinnen-per-mail-angeschrieben-werden/)
- Project context: `.planning/PROJECT.md` (v1.6 milestone + existing member-mail/template/communication subsystem), `CLAUDE.md` (audit + DSGVO + Component-First constraints)

---
*Feature research for: applicant-facing transactional reminder email in a small German-cooperative tool*
*Researched: 2026-08-12*
