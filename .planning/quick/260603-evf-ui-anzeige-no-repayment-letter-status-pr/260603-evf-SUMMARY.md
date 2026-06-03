---
quick_id: 260603-evf
date: 2026-06-03
description: UI-Anzeige no_repayment_letter-Status pro Empfänger im Bulk-Mail-Job-Detail
status: complete
---

# Quick Task 260603-evf — SUMMARY

## What changed

Failed-Empfaenger in Bulk-Mail-Jobs mit `error.starts_with("no_repayment_letter")` werden im Frontend jetzt visuell von generischem `failed` unterschieden (amber Badge "Kein Anschreiben generiert" statt rotem "Fehlgeschlagen") und bekommen einen Ein-Klick-Action-Button "Brief generieren + Retry" in der Empfaenger-Tabelle. Der Button laueft den 3-Schritt-Recovery-Flow `list_repayment_entries -> generate_repayment_letters -> retry_mail_job`, mit Loading-State, Success-Toast und Error-Toast.

Sichtbar in **beiden** Stellen, an denen die Empfaenger-Tabelle gerendert wird:
1. MailPage Expanded-Row (`genossi-frontend/src/page/mail_page.rs:MailPage`)
2. MailJobDetail Deeplink-Page (`genossi-frontend/src/page/mail_page.rs:MailJobDetail`)

Component-First strikt eingehalten: 2 neue reusable Components in `genossi-frontend/src/component/` — die Logik existiert genau einmal.

## Task Commits

1. **Task 1: Backend MailJobTO + Frontend Mirror — repayment_phase_id exponieren** — `a829433` (feat)
2. **Task 2: MailRecipientStatusBadge Component + i18n + Wiring** — `b0fc5e8` (feat)
3. **Task 3: NoRepaymentLetterAction Component + i18n + Wiring + Toast** — `db8694e` (feat)

## Architektur-Entscheidung: minimaler Backend-Touch

Der Quick-Task-Constraint sagte ursprünglich "Backend NICHT anfassen, beide Endpoints existieren". Die Planungsphase hat ein Backend-Loch entdeckt: `MailJobTO` exponiert `repayment_phase_id` nicht, obwohl es im DAO `MailJob` persistiert ist. Statt das als Blocker zurueckzugeben, hat der Plan einen sauberen Mittelweg gewaehlt:

- **Eine Zeile in struct + eine Zeile im From-Impl.** Keine neuen Endpoints, keine neue Service-Logik, keine Migration, keine Validation.
- Das Feld ist `Option<String>` mit `#[serde(default, skip_serializing_if = "Option::is_none")]` — additiv und backward-compatible.
- Begruendung: Constraint will Mini-Backend-Refactorings vermeiden. Das Hinzufuegen eines bereits persistierten Feldes zum Read-DTO ist KEIN Refactoring, sondern reine Daten-Exposition. Es ist die kleinste moegliche Backend-Aenderung im direkten Verhaeltnis zum UI-Ziel.
- Alternative (verworfen): Frontend muesste alle Phasen iterieren und fuer jede `list_repayment_entries(phase_id)` aufrufen, dann den Empfaenger-`member_id` quer-suchen. O(N*M) Roundtrips, ambig bei mehreren Phasen pro `fiscal_year`, operativ Wartungs-untauglich.

Diese Entscheidung war im PLAN expizit vorgesehen und genehmigt durch die executor-constraints.

## Files touched

| File | Change |
|------|--------|
| `genossi_mail/src/rest.rs` | `MailJobTO` +1 Feld `repayment_phase_id: Option<String>` + Doc-Kommentar + From-Impl-Update; +4 Tests + `make_mail_job` Helper |
| `genossi-frontend/src/api.rs` | `MailJobTO`-Mirror +1 Feld (identische Semantik); +1 Roundtrip/Backward-Compat-Test |
| `genossi-frontend/src/component/mail_recipient_status_badge.rs` | **NEU** — Badge-Component + 3 pure helpers (`is_no_repayment_letter_failure`, `status_label_key`, `status_badge_class`) + 9 Tests |
| `genossi-frontend/src/component/no_repayment_letter_action.rs` | **NEU** — Action-Button-Component + 2 pure helpers (`find_entry_for_member`, `button_label_for_state`) + `ButtonState`-Enum + 4 Tests |
| `genossi-frontend/src/component/mod.rs` | 2 neue `pub mod` + `pub use`-Exports |
| `genossi-frontend/src/i18n/mod.rs` | 5 neue Keys (1 Badge + 4 Action) |
| `genossi-frontend/src/i18n/de.rs` | 5 neue de-Uebersetzungen |
| `genossi-frontend/src/i18n/en.rs` | 5 neue en-Uebersetzungen |
| `genossi-frontend/src/page/mail_page.rs` | Imports erweitert; Inline-Status-Code in beiden Tabellen durch `MailRecipientStatusBadge` ersetzt; neue 4. Action-Spalte mit `NoRepaymentLetterAction`; ToastContainer + Signals in beiden Pages; MailJobDetail bekommt `id_signal`-Pattern fuer Refetch |

## Tests

| Suite | Result |
|-------|--------|
| `cargo test -p genossi_mail` (lib) | **149 passed, 0 failed** (vorher 145 → +4) |
| `cargo test --bin genossi-frontend` | **214 passed, 0 failed** (vorher 200 → +14: 1 api-Roundtrip + 9 Badge + 4 Action) |
| `cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown` | **clean** (nur pre-existing dead-code-Warnings auf `Key`-Variants) |

### Neue Tests im Detail

**Backend (`genossi_mail::rest::tests`, +4):**
- `test_mail_job_to_exposes_repayment_phase_id_when_present`
- `test_mail_job_to_repayment_phase_id_none_is_skipped_on_serialize`
- `test_mail_job_to_repayment_phase_id_serde_roundtrip`
- `test_mail_job_to_deserialize_backward_compat_without_repayment_phase_id`
- (+`make_mail_job` Helper)

**Frontend Mirror (`api::tests`, +1):**
- `test_mail_job_to_repayment_phase_id_roundtrip_and_backward_compat`

**MailRecipientStatusBadge (`component::mail_recipient_status_badge::tests`, +9):**
- `class_sent_is_green`, `class_failed_without_error_is_red`, `class_failed_with_no_repayment_letter_is_amber`
- `class_failed_with_no_repayment_letter_detail_suffix_still_amber` (future-safety: `.starts_with` Pattern)
- `class_failed_with_other_error_stays_red`, `class_pending_and_queued_are_gray`
- `label_key_matches_status_and_error`, `is_no_repayment_letter_failure_only_for_failed_status`, `all_share_pill_styling`

**NoRepaymentLetterAction (`component::no_repayment_letter_action::tests`, +4):**
- `find_entry_returns_matching_member`, `find_entry_returns_none_for_empty`, `find_entry_returns_first_match_when_duplicates`
- `button_label_for_state_maps_each_state`

## Decisions Made

| Aspekt | Entscheidung | Warum |
|--------|--------------|-------|
| Badge-Farbe für `no_repayment_letter` | amber (`bg-amber-100 text-amber-800`) | Konsistent mit dem amber Retry-Button in mail_page.rs:714, semantisch zwischen rot (final fail) und gruen (sent) |
| Error-String-Check | `.starts_with("no_repayment_letter")` statt `==` | Future-safety: falls der Worker irgendwann Suffix-Details haengt (`":<details>"`), bleibt der Marker korrekt |
| Blob-URL nach `generate_repayment_letters` | `web_sys::Url::revoke_object_url(&gen.blob_url)` ohne Click | Wir wollen NUR die serverseitige MemberDocument-Persist-Seite, kein Download. Memory-leak-safe |
| `r#type: "button"` am Action-Button | Explizit gesetzt | Dioxus Page-Reload-Bug-Workaround (Hotfix e245013-Pattern, Memory `feedback_dioxus_button_type`) |
| MailJobDetail-Reload nach Recovery | `id_signal`-Pattern (Signal<String> ist Copy via Reactive-Reference) | Erlaubt Refetch in mehreren on_done-Closures ohne FnOnce-Move-Problem |
| Toast statt Modal/Banner | `show_toast` + `ToastContainer` Pattern | 1:1-Konsistenz mit `repayment_phase_details.rs` |
| i18n strict | 5 Keys, alle in beiden Locales | Verifiziert via grep im Verify-Block des PLAN |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] MailJobDetail benötigte `id_signal`-Pattern für Refetch**
- **Found during:** Task 3 (NoRepaymentLetterAction Wiring in MailJobDetail)
- **Issue:** Plan-Vorschlag sah eine `reload_detail`-Closure vor, die aus mehreren `on_done`-Handlern aufgerufen wird. Der Rust-Compiler beanstandet das (`E0507: cannot move out of value, a captured variable in an FnMut closure`), weil die Closure die `id: String` per-Move capturet und dann im FnMut-Closure-Kontext nicht erneut gemoved werden kann.
- **Fix:** `id_signal = use_signal(|| id.clone())` erzeugt einen Copy-Signal. on_done liest den Signal-Wert und spawned die Refetch-Logik direkt inline.
- **Files modified:** `genossi-frontend/src/page/mail_page.rs` (MailJobDetail-Block)
- **Verification:** `cargo check --target wasm32-unknown-unknown` clean.
- **Committed in:** `db8694e` (Task 3 commit)

**2. [Rule 1 - Bug] `show_toast` Signatur ist `Signal<Vec<(u64, String)>>`, nicht `Vec<(usize, String)>`**
- **Found during:** Task 3 (Toast-Integration)
- **Issue:** Plan schrieb `usize`, der echte Code (`genossi-frontend/src/component/toast.rs`) nutzt `u64`.
- **Fix:** Alle drei Signals (in MailPage + MailJobDetail) als `Vec<(u64, String)>` + `u64` Counter typisiert.
- **Files modified:** `genossi-frontend/src/page/mail_page.rs`
- **Verification:** `cargo test --bin genossi-frontend` 214 passed.
- **Committed in:** `db8694e` (Task 3 commit)

**3. [Rule 1 - Bug] `use_i18n()` ist nicht `Send`, kann nicht im `spawn`-Block aufgerufen werden**
- **Found during:** Task 3 (NoRepaymentLetterAction onclick spawn)
- **Issue:** Plan-Vorschlag rief `i18n_clone.t(...)` im Inneren des `spawn(async move { ... })` auf — `I18n` ist nicht Send.
- **Fix:** Pre-resolve den `MailGenerateLetterAndRetryNoEntry`-String VOR dem `spawn` (`let no_entry_msg = use_i18n().t(...)`), dann move ihn in den async-Block.
- **Files modified:** `genossi-frontend/src/component/no_repayment_letter_action.rs`
- **Verification:** `cargo check --target wasm32-unknown-unknown` clean.
- **Committed in:** `db8694e` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (Rule 1 - Bug, alle 3 Build-blocking)
**Impact on plan:** Alle Auto-Fixes notwendig fuer Compile. Keine architektonische Aenderung, kein Scope-Creep — nur Anpassungen an die echte Signatur der bestehenden Helpers/Types.

## Issues Encountered

- Kein. Plan war praezise; nur die 3 oben genannten Type-/Signatur-Korrekturen waren noetig.

## Bulk-Action: Scope-Cut + Follow-up

Der Plan dokumentiert explizit "EXPLIZIT OUT OF SCOPE: Wir erweitern mail_page.rs NICHT um einen 'Alle no_repayment_letter-Briefe generieren + Retry'-Button". Follow-up-Todo wurde unter `.planning/todos/pending/frontend-bulk-no-repayment-letter-action.md` angelegt.

## Smoke-Test (vom Executor zu validieren)

Manueller Smoke-Test ist NICHT im Worktree moeglich (kein laufendes Backend + SMTP), wird stattdessen als Acceptance-Step beim naechsten Deploy/UAT durchgespielt:

1. `cargo run --bin genossi` starten
2. Bulk-Mail-Job mit `attach_repayment_letter=true` und einem Member ohne RepaymentLetter ausloesen
3. Frontend Mail-Page laden, Job aufklappen
4. Erwartung: amber Badge "Kein Anschreiben generiert" + Button "Brief generieren + Retry"
5. Klick triggert die Aktion, Loading-State erscheint, danach Toast "Brief generiert, Retry läuft"
6. Auf `/mail/job/{id}`-Deeplink dasselbe Verhalten

## Known Stubs

Keine. Alle UI-Elemente sind voll funktional verdrahtet — Badge, Action-Button, Toast, Refetch-Logik.

## Self-Check: PASSED

- [x] `genossi_mail/src/rest.rs` — MailJobTO + repayment_phase_id Feld + From-Impl + 4 Tests + make_mail_job Helper (committed `a829433`)
- [x] `genossi-frontend/src/api.rs` — Mirror MailJobTO + 1 Test (committed `a829433`)
- [x] `genossi-frontend/src/component/mail_recipient_status_badge.rs` — Component + 3 Helpers + 9 Tests (committed `b0fc5e8`)
- [x] `genossi-frontend/src/component/no_repayment_letter_action.rs` — Component + 2 Helpers + ButtonState + 4 Tests (committed `db8694e`)
- [x] `genossi-frontend/src/component/mod.rs` — 2 neue pub mod + pub use (committed in b0fc5e8 + db8694e)
- [x] `genossi-frontend/src/i18n/{mod,de,en}.rs` — 5 neue Keys konsistent in beiden Locales (committed in b0fc5e8 + db8694e)
- [x] `genossi-frontend/src/page/mail_page.rs` — MailPage + MailJobDetail nutzen beide Components, ToastContainer in beiden, Action-Spalte in beiden (committed in b0fc5e8 + db8694e)
- [x] Commits: `a829433`, `b0fc5e8`, `db8694e` alle im git log vorhanden
- [x] Tests: `cargo test -p genossi_mail` 149 passed, `cargo test --bin genossi-frontend` 214 passed
- [x] `cargo check --target wasm32-unknown-unknown` clean
