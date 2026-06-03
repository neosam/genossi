---
phase: quick-260603-h0r
plan: 01
subsystem: mail-worker / repayment-aggregation
tags: [refactor, dry, single-source-of-truth, phase-10, phase-13]
requires:
  - genossi_service::repayment_context::RepaymentContextResolver (trait, Phase 13 D-13-04)
  - genossi_service_impl::repayment_context::RepaymentContextResolverImpl (impl, Phase 13 D-13-04)
  - genossi_mail::template::merge_repayment_context (unchanged 4-arg signature)
provides:
  - Mail-Worker delegiert die Phase-10 D-04 (Format) + D-06 (Filter) Aggregation
    an die geteilte RepaymentContextResolver-Trait-Methode `aggregate(...)`,
    die der Letter-Service bereits nutzt (Phase 13 D-13-10).
  - Single Arc<RepaymentContextResolver> pro Prozess wird vom selben Konstruktor
    (lib.rs:951) sowohl an den Letter-Service als auch an den Mail-Worker
    durchgereicht (.clone() in beide Pfade).
affects:
  - genossi_mail/src/worker.rs (Signatur: 14 -> 15 Generics + Args; Body: inline
    Filter/Sum/German-Format-Block durch resolver.aggregate(...) ersetzt)
  - genossi_bin/src/lib.rs (RestStateImpl::start_mail_worker reicht
    self.repayment_context_resolver.clone() durch; #[allow(dead_code)] entfernt)
tech-stack:
  added: []
  patterns:
    - "Resolver-Trait als Generic-Bound (RCR: RepaymentContextResolver<Transaction = MD::Transaction>) -
      konsistent mit den 14 bestehenden DAO-Generics; vermeidet Box<dyn>-Overhead im Hot-Loop"
    - "Aggregate-Result-3-Arm-Match: Ok -> merge, EntityNotFound -> skip (D-05
      Edge-Case unverändert), other-Err -> mark_recipient_failed + sleep + continue"
    - "share_value_str bleibt lokal aus phase.share_value abgeleitet (1 Zeile,
      doc-kommentiert) - RepaymentContext trägt share_value nicht (out-of-scope)"
key-files:
  created: []
  modified:
    - genossi_mail/src/worker.rs
    - genossi_bin/src/lib.rs
decisions:
  - "Approach (a) - direct Service-Trait-Import als 15. Generic + Bound -
    konsistent mit den existierenden 14 Generic-Bounds, vermeidet Box<dyn>-Overhead"
  - "Pure-fn-Variante `aggregate(...)` statt `resolve(...)` - Worker hält phase
    weiterhin lokal und kann share_value_str ohne API-Erweiterung am Resolver
    derivenSpiegelt das Letter-Service-Pattern (repayment_letter.rs:318-321)"
  - "share_value_str bleibt lokal im Worker (1 Zeile) - additive
    RepaymentContext.share_value wäre cleaner, ist aber out-of-scope für reines
    DRY-Refactor (siehe todo phase-10-worker-refactor-resolver.md)"
metrics:
  duration_minutes: ~12
  completed_date: "2026-06-03"
  tasks_total: 2
  tasks_completed: 2
  files_modified: 2
commits:
  - "369f174 refactor(quick-260603-h0r): Worker delegiert Repayment-Aggregation an Resolver"
  - "4dd6e7f refactor(quick-260603-h0r): start_mail_worker reicht Resolver-Arc weiter"
---

# Quick 260603-h0r: Phase-10 Mail-Worker auf RepaymentContextResolver — Summary

## One-liner

Mail-Worker delegiert die Phase-10-Aggregation (Filter Open/Contacted, SUM
share_count, deutsche Euro-Formatierung) an die Trait-Methode
`RepaymentContextResolver::aggregate(...)`, die der Letter-Service seit Phase 13
nutzt — Single Source of Truth, gespiegelte Algorithmen entfernt, gleicher Arc
geteilt zwischen Letter-Service und Mail-Worker.

## Was wurde gemacht

### Task 1 — genossi_mail/src/worker.rs (Commit `369f174`)

- `start_mail_worker` erhält 15. Generic `RCR` mit Bound
  `RepaymentContextResolver<Transaction = MD::Transaction> + Send + Sync + 'static`
  und 15. Arg `repayment_context_resolver: Arc<RCR>`.
- Neue Imports am Modul-Header:
  ```rust
  use genossi_service::repayment_context::RepaymentContextResolver;
  use genossi_service::ServiceError;
  ```
- Inline-Aggregations-Block (37 LOC, Filter+Sum+Format) durch 3-Arm-Match
  ersetzt:
  - `Ok(rc)` → merge in template-ctx (D-05 happy-path, identisches Verhalten)
  - `Err(EntityNotFound(_))` → skip merge (D-05 edge-case, identisches
    Verhalten — strict-env render fängt es downstream ab)
  - `Err(e)` → `mark_recipient_failed` + sleep + continue (Defense gegen
    T-h0r-04 DoS-Retry-Loop; identisches Failure-Pattern wie alle
    anderen Worker-Error-Branches).
- `share_value_str` bleibt lokal aus `phase.share_value` abgeleitet (1 Zeile,
  doc-kommentiert, warum). `phase` wird weiterhin via
  `phase_dao.find_by_id(...)` geladen (vor dem Resolver-Call), weil
  `RepaymentContext` `share_value` nicht trägt.
- `entries` (`Arc<[RepaymentEntryEntity]>` aus
  `find_by_phase_id`) wird per `&entries` an `aggregate(...)` weitergegeben;
  Deref-Coercion auf `&[RepaymentEntryEntity]` ist transparent.
- **Verifizierung Task 1:** `cargo build -p genossi_mail` clean,
  `cargo test -p genossi_mail --lib` 149/149 grün.

### Task 2 — genossi_bin/src/lib.rs (Commit `4dd6e7f`)

- `RestStateImpl::start_mail_worker` (Z. 1325) klont
  `self.repayment_context_resolver` in einen lokalen Arc neben den
  bestehenden 6 worker-Deps-Clones und reicht ihn als 15. positionellen Arg
  an `genossi_mail::worker::start_mail_worker(...)` weiter — mit Inline-
  Kommentar "Quick 260603-h0r: Shared aggregation resolver — same Arc as
  Letter-Service.".
- `#[allow(dead_code)]` am `repayment_context_resolver`-Feld (Z. 611)
  entfernt. Doc-Comment aktualisiert:
  - Alt: "kept on the state struct so a future Phase-10 worker-refactor (todo
    phase-10-worker-refactor-resolver) can also resolve via the same Arc."
  - Neu: "Same Arc passed to start_mail_worker (Quick 260603-h0r refactor);
    also cloned into the letter-service below. Single Arc per process."
- Single-Arc-Topologie verifiziert via Grep:
  - 1 Konstruktor (`Arc::new(RepaymentContextResolverImpl::<...> { ... })` an
    Z. 951)
  - 2 `.clone()`-Consumer: Letter-Service (Z. 977) + Mail-Worker (neue Z. im
    `start_mail_worker`-Block)
- **Verifizierung Task 2:**
  - `cargo build --workspace` clean.
  - `cargo clippy --workspace --all-targets -- -D warnings` clean (keine
    neuen Warnings, kein `dead_code` mehr am Feld).
  - `cargo test --workspace --lib` alle Suites grün (10 Crates).
  - `cargo test -p genossi_bin --test e2e_tests test_bulk_repayment_mail_creates_member_documents_per_recipient`
    grün — **byte-äquivalenter** `payout_amount` ("60,00" / "100,00" / "40,00")
    für die 3 Empfänger.

## Verhaltens-Äquivalenz — der zentrale Gate

Die `aggregate_for_member`-Pure-Fn (genossi_service_impl/src/repayment_context.rs:52)
spiegelt den vorher inline lebenden Algorithmus 1:1:

| Schritt | Pre-Refactor (inline) | Post-Refactor (resolver) |
|--------|----------------------|-------------------------|
| Filter | `e.deleted.is_none() && e.member_id == member.id && matches!(e.status, Open\|Contacted)` | identisch |
| Sum | `share_count_to_pay_out.sum()` | identisch |
| Format | `format!("{},{:02}", cents/100, cents%100)` | identisch |
| Empty-Case | `if !relevant.is_empty() { merge }` | `Err(EntityNotFound) → skip merge` |

Die 11 Unit-Tests des Resolvers (genossi_service_impl/src/repayment_context.rs::tests)
+ der E2E-Test `test_bulk_repayment_mail_creates_member_documents_per_recipient`
pinnen das Verhalten end-to-end auf die exakten deutschen Euro-Strings.

## Untouched Pathways (verifiziert kein Regress)

- `attach_repayment_letter`-Block (Quick-cz6, worker.rs:308-364) — unverändert.
- `find_repayment_letter_for_recipient`-Helper (Quick-cz6, worker.rs:81-115) —
  unverändert.
- `try_create_member_document_audited`-Audit-Pipeline (Phase 10 D-11) —
  unverändert. Single `audit_log_dao`-Arc pro Prozess; Hash-Chain bleibt valid
  (verifiziert implizit via E2E-Test, der MemberDocuments erzeugt).
- `masked_iban`-Template-Variable (Quick 260603-b43) — unverändert.

## Deviations from Plan

Keine. Plan exakt wie geschrieben ausgeführt (2 Tasks, 2 atomare Commits, alle
Grep-Gates und Verification-Commands aus dem Plan greifen).

## Threat Flags

Keine neue Threat-Surface — Refactor ändert nur Aggregations-Step
(pure read, kein neuer Endpoint, kein neuer Trust-Boundary, kein neuer
Auth-Pfad, keine neue Datei-Schreib-Operation). Resolver ist read-only,
inherited den existing Worker-tx-Borrow-Pattern.

## Follow-Up Todos

- Todo `.planning/todos/pending/phase-10-worker-refactor-resolver.md` kann
  jetzt nach `.planning/todos/done/` verschoben werden — der Refactor ist
  abgeschlossen. (Out-of-scope für diesen Plan: nur Implementierung; Todo-
  Move erfolgt separat.)
- Optionaler Aufräumschritt für eine künftige Quick-Task: `share_value` zu
  `RepaymentContext` hinzufügen → die 1-Zeile `share_value_str`-Ableitung im
  Worker entfällt. Aktuell bewusst out-of-scope ("reines DRY-Refactor", siehe
  Plan §crate_dependency_decision).

## Self-Check: PASSED

- Files modified exist and match expected diffs:
  - `genossi_mail/src/worker.rs` ✓ (FOUND)
  - `genossi_bin/src/lib.rs` ✓ (FOUND)
- Commits exist on main:
  - `369f174` ✓ (FOUND in git log)
  - `4dd6e7f` ✓ (FOUND in git log)
- Grep gates:
  - `phase.share_value * (...)` in worker.rs: 0 outside tests ✓
  - `repayment_context_resolver` in worker.rs: 2 (>= 2 expected) ✓
  - `repayment_context_resolver` in lib.rs: 6 (>= 5 expected) ✓
  - `RepaymentContextResolverImpl::<` in lib.rs: 1 (single ctor) ✓
  - `#[allow(dead_code)] ... repayment_context` in lib.rs: 0 ✓
- Verification gates:
  - `cargo build --workspace` clean ✓
  - `cargo clippy --workspace --all-targets -- -D warnings` clean ✓
  - `cargo test --workspace --lib` green ✓
  - `cargo test -p genossi_bin --test e2e_tests test_bulk_repayment_mail_creates_member_documents_per_recipient` green ✓
