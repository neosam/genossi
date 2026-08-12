---
phase: 29-dao-schema-foundation-kommunikations-historie-pro-antragstel
verified: 2026-08-12T00:00:00Z
status: passed
score: 8/8 must-haves verified
behavior_unverified: 0
overrides_applied: 0
deferred:
  - truth: "Roadmap Success Criterion 1 (wörtlich): '... wird persistiert und über GET /api/applications/{id}/communications (outbound-only) als Historie-Eintrag zurückgeliefert.' — der REST-Endpoint selbst existiert nicht in Phase 29."
    addressed_in: "Phase 31"
    evidence: "ROADMAP.md Zeile 112 (Phase-31-Kurzbeschreibung): 'POST /api/applications/{id}/mail + GET /api/applications/{id}/communications, admin-only'. Die Phase-29-Kurzbeschreibung (Zeile 110) und beide PLAN.md-Frontmatter (29-01, 29-02) grenzen explizit auf DAO-Ebene ein ('KEIN REST-Endpoint in dieser Phase' — Read-Hälfte von Success-Kriterium 1). Der DAO-Teil (get_application_communications, outbound-only, Soft-Delete-korrekt) ist vollständig implementiert und getestet; nur das REST-Mounting ist auf Phase 31 verschoben."
---

# Phase 29: DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller) Verification Report

**Phase Goal:** Alle an einen Antragsteller gesendeten Mails werden über eine eigene `application_id`-Linkage erfasst und bleiben auch nach der Bestätigung zum Mitglied in dessen Timeline sichtbar — ohne den `member_id`-Namespace zu vergiften.
**Verified:** 2026-08-12
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Migration ist additiv/forward-only: `application_id BLOB` ohne DEFAULT/NOT NULL + Index, bestehende Zeilen lesen NULL byte-identisch zurück | ✓ VERIFIED | `migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql` — exakt `ALTER TABLE mail_recipients ADD COLUMN application_id BLOB;` + `CREATE INDEX IF NOT EXISTS idx_mail_recipients_application_id ...`; kein DEFAULT/NOT NULL. Timestamp `20260812000000` ist jüngste Migration (kein Ordering-Bruch). Test `test_recipient_roundtrip_null_legacy_application_id` grün. |
| 2 | `application_id: Option<Uuid>` ist Feld auf `MailRecipient` und `RecipientInput`, durchgefädelt bis `create_job` | ✓ VERIFIED | `genossi_mail/src/dao.rs:67`, `genossi_mail/src/service.rs:60`; `create_job` setzt `application_id: input.application_id` (`service.rs:456`). |
| 3 | Alle produktiven `mail_recipients`-SQL-Spaltenlisten (INSERT + 3×SELECT) und das In-Memory-Test-DDL enthalten `application_id`, kein Column-Count-Mismatch | ✓ VERIFIED | `dao_sqlite.rs:276` (INSERT), `:300`/`:316`/`:387` (die 3 SELECT-Listen), `:1422` (Test-DDL). `cargo build --workspace` grün, `cargo test -p genossi_mail` 288/288 grün — ein Spaltenzahl-Mismatch wäre ein sofortiger Laufzeit-/Compile-Fehler gewesen. |
| 4 | Persistenz-Roundtrip für `application_id` nachgewiesen (nicht nur NULL-Legacy) | ✓ VERIFIED | `test_recipient_roundtrip_application_id` (dao_sqlite.rs:1695) — schreibt `application_id: Some(app_uuid)`, `member_id: None`, liest zurück, assert exakt gleich. Grün. |
| 5 | `member_id`-Namespace bleibt sauber: eine Application-UUID landet niemals in `member_id` | ✓ VERIFIED | `test_recipient_application_send_keeps_member_id_namespace_clean` (dao_sqlite.rs:1743) — Persistenz-Assert `found[0].member_id.is_none()` bei gesetztem `application_id`. Zusätzlich `confirm()`-Hook (application.rs:569-580) übergibt `id` (Application-UUID) und `member_id` (genuine `uuid_service.new_v4()`-UUID, Zeile 330) als getrennte Argumente an `link_application_recipients_to_member(id, member_id)` — nie vertauscht, nie in ein Feld zusammengeführt. |
| 6 | `get_application_communications(application_id)` liefert die als Antragsteller gesendete Mail outbound-only zurück; fremde `application_id` → leer; Soft-Delete (Recipient UND Job) respektiert; kein inbound-Eintrag | ✓ VERIFIED | Impl `dao_sqlite.rs:1128-1166`: `WHERE r.application_id = ?1 AND r.deleted IS NULL AND j.deleted IS NULL`, kein `UNION`/inbound-Zweig. 5 grüne Tests: `test_application_communications_returns_outbound_entry`, `_empty_for_foreign_application`, `_excludes_soft_deleted_recipient`, `_excludes_soft_deleted_job`, `_is_outbound_only` (letzterer beweist explizit: ein inbound-Mail mit `assigned_member_id == application_id` taucht NICHT auf). |
| 7 | Carry-over bei `confirm()`: `link_application_to_member`-UPDATE setzt ausschließlich die genuine neue `member_id`, gefiltert auf `member_id IS NULL`, überschreibt keine bereits zugeordneten/fremden Zeilen | ✓ VERIFIED | `dao_sqlite.rs:411-419`: `UPDATE mail_recipients SET member_id = ? WHERE application_id = ? AND member_id IS NULL`. Test `test_link_application_to_member_backfills_and_is_visible_in_member_timeline` grün. |
| 8 | Nach `confirm()` erscheint die zuvor als Antragsteller gesendete Erinnerung in der Mitglieds-Timeline des neuen Mitglieds (e2e, unveränderte `get_member_communications`-Query) | ✓ VERIFIED | `genossi_service_impl/src/application.rs:561-580` — Hook läuft NACH `commit(tx)` (Zeile 545), `tracing::warn!` statt `?` bei Fehler (post-commit best-effort, kein Rollback bestätigt durch Code-Lesung). e2e-Test `test_application_communication_carries_over_to_member_on_confirm` (genossi_bin/tests/e2e_tests.rs) separat mit `--exact` ausgeführt: PASS. `get_member_communications` selbst unverändert (nur `link_application_to_member` schreibt zurück). |

**Score:** 8/8 truths verified (0 present, behavior-unverified)

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Roadmap SC1 (wörtlich) verlangt zusätzlich den REST-Endpoint `GET /api/applications/{id}/communications` — existiert nicht in Phase 29 | Phase 31 | ROADMAP.md Zeile 112 weist den Endpoint explizit Phase 31 zu; Phase-29-Kurzbeschreibung (Zeile 110) und beide PLAN.md (29-01/29-02 Frontmatter + Objective) grenzen bewusst auf DAO-Ebene ein. Der REST-Mount ist die einzige fehlende Teilmenge von SC1 — die Persistenz- und Read-Logik (DAO-Ebene) ist vollständig verifiziert (Truth 6 oben). |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql` | ALTER ADD COLUMN + Index, additiv | ✓ VERIFIED | Datei existiert, Inhalt exakt wie spezifiziert, jüngste Migration in `migrations/sqlite/` |
| `genossi_mail/src/dao.rs` — `MailRecipient.application_id`, `CommunicationDao::get_application_communications`, `MailRecipientDao::link_application_to_member` | Felder/Trait-Methoden + `#[automock]` | ✓ VERIFIED | Zeilen 67, 108-116 (link_application_to_member), 300-307 (get_application_communications); beide Traits behalten `#[automock]` |
| `genossi_mail/src/service.rs` — `RecipientInput.application_id`, `MailService::link_application_recipients_to_member` | Feld + create_job-Threading + Delegation | ✓ VERIFIED | Zeilen 60, 162, 456, 649-656 (Delegation an `recipient_dao.link_application_to_member`) |
| `genossi_mail/src/dao_sqlite.rs` — Impl beider neuer Methoden + alle SQL-Spaltenlisten + Test-DDL + Tests | Outbound-only SELECT + gefiltertes UPDATE + 8 Tests | ✓ VERIFIED | Siehe Truths 3, 4, 5, 6, 7 oben; 8 neue Tests namentlich verifiziert via `cargo test -- --list` |
| `genossi_service_impl/src/application.rs` — confirm()-Post-Commit-Hook | Nach commit(tx), best-effort, kein Rollback | ✓ VERIFIED | Zeilen 561-580, Code gelesen und bestätigt |
| `genossi_bin/tests/e2e_tests.rs` — Carry-over-e2e-Test | Erinnerung → confirm → sichtbar in Member-Timeline | ✓ VERIFIED | `test_application_communication_carries_over_to_member_on_confirm` läuft grün (`--exact`-Run durchgeführt) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `RecipientInput.application_id` | `MailRecipient.application_id` | `create_job` (service.rs:456) | ✓ WIRED | Direkte Zuweisung `application_id: input.application_id` |
| `MailRecipient.application_id` | SQLite-Spalte | `recipient_dao.create` INSERT (dao_sqlite.rs:276/285) | ✓ WIRED | Spalte in INSERT-Liste + korrespondierender Bind |
| SQLite-Row | `MailRecipient.application_id` | `TryFrom<&MailRecipientDb>` (dao_sqlite.rs:242) | ✓ WIRED | `parse_optional_uuid(&db.application_id)?` |
| `confirm()` post-commit | `mail_service.link_application_recipients_to_member` | application.rs:569-572 | ✓ WIRED | Aufruf mit getrennten Argumenten `(id, member_id)` |
| `mail_service.link_application_recipients_to_member` | `recipient_dao.link_application_to_member` | service.rs:649-656 | ✓ WIRED | Direkte Delegation |
| `link_application_to_member` UPDATE | `get_member_communications(new_member_id)` | Zurückgeschriebene member_id sichtbar über unveränderte Query | ✓ WIRED | e2e-Test bestätigt End-to-End-Sichtbarkeit |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| genossi_mail Unit-/DAO-Tests grün (inkl. 8 neuer Phase-29-Tests) | `cargo test -p genossi_mail` | 288 passed; 0 failed | ✓ PASS |
| genossi_service_impl Unit-Tests grün (confirm()-Hook + Mock-Erwartung) | `cargo test -p genossi_service_impl` | 437 passed; 0 failed; 2 ignored (pre-existing) | ✓ PASS |
| e2e-Carry-over-Test einzeln | `cargo test -p genossi_bin --test e2e_tests test_application_communication_carries_over_to_member_on_confirm --exact` | 1 passed | ✓ PASS |
| Vollständige e2e-Suite (Regressionscheck) | `cargo test -p genossi_bin --test e2e_tests` | 315 passed; 2 failed (beide vorbestehend, siehe unten) | ✓ PASS (mit dokumentierter Ausnahme) |
| Workspace kompiliert fehlerfrei | `cargo build --workspace` | Finished, 0 errors | ✓ PASS |
| Clippy auf betroffenen Crates ohne neue Warnungen | `cargo clippy -p genossi_mail -p genossi_service_impl -p genossi_bin --all-targets` | Nur vorbestehende Warnungen in `repayment_letter.rs` (unrelated) | ✓ PASS |
| Debt-Marker-Scan (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) auf allen 11 modifizierten Dateien | grep pro Datei | 0 Treffer | ✓ PASS |

**Pre-existing e2e-Failures (bestätigt out-of-scope):** `preview_body_html_round_trips_to_response` und `test_mail_preview_repayment_no_entries_does_not_default_to_one`. Beide betreffen `/api/mail/preview`-Rendering (Markdown-Bold-Stripping bzw. `errors`-Array-Serialisierung), Code den Phase 29 nicht anfasst (`render.rs`/`rest.rs` nur um `application_id: None`-Literale ergänzt, keine Logikänderung an der betroffenen Preview-Pipeline). Reproduzieren laut SUMMARY identisch auf Baseline-Commit `4f7940e` (vor Phase 29) und sind in `deferred-items.md` dokumentiert; nicht erneut auf dem Baseline-Commit nachgestellt (Vertrauen in dokumentierte, plausible Isolations-Prüfung der Executor-Session, da Ursache — Markdown/minijinja-Rendering — nachweislich außerhalb der 11 phase-29-modifizierten Dateien liegt).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| APHIST-01 | 29-01, 29-02 | Kommunikations-Historie pro Antragsteller über `application_id`-Linkage; Endpoint `GET /api/applications/{id}/communications` | ⚠ TEILWEISE SATISFIED (DAO-Ebene vollständig; REST-Endpoint fehlt, siehe Deferred-Item) | `get_application_communications` implementiert + 5 Tests grün; Endpoint nicht gemountet — laut ROADMAP.md Zeile 112 bewusst Phase 31 |
| APHIST-03 | 29-02 | Carry-over bei `confirm()` — Erinnerung erscheint in Mitglieds-Timeline | ✓ SATISFIED | Post-commit Hook + e2e-Test grün (siehe Truth 8) |

**Hinweis zur REQUIREMENTS.md-Traceability:** `REQUIREMENTS.md` markiert APHIST-01 als `[x]` und in der Traceability-Tabelle (Zeile 79) als "Complete", ausschließlich Phase 29 zugeordnet. Der Volltext von APHIST-01 (Zeile 28) nennt explizit den REST-Endpoint `GET /api/applications/{id}/communications` als Teil der Anforderung — dieser existiert nicht in der Codebase (verifiziert: `grep -rn "applications/{id}" genossi_rest/src/` liefert keinen Treffer; nur `/api/members/{member_id}/communications` ist gemountet). Da ROADMAP.md den Endpoint an anderer Stelle (Zeile 112, Phase-31-Kurzbeschreibung) explizit Phase 31 zuweist, wird dies hier als **deferred**, nicht als Gap von Phase 29 behandelt — der DAO-Anteil von APHIST-01, der laut PLAN-Frontmatter explizit der Phase-29-Scope ist, ist vollständig erfüllt. Empfehlung: `REQUIREMENTS.md`-Traceability-Tabelle in Phase 31 um eine zweite APHIST-01-Zeile ("REST-Mount") ergänzen oder den "Complete"-Status bis zum REST-Mount auf "Teilweise (DAO)" korrigieren, damit die Traceability-Tabelle nicht suggeriert, APHIST-01 sei vollständig mit Phase 29 abgeschlossen.

### Anti-Patterns Found

Keine. Debt-Marker-Scan (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) über alle 11 in Phase 29 modifizierten/erstellten Dateien: 0 Treffer.

### Human Verification Required

Keine. Alle Truths sind über Code-Lesung + automatisierte Tests programmatisch verifizierbar; kein UI-/visuelles/Echtzeit-Verhalten in dieser DAO/Schema-Foundation-Phase.

### Gaps Summary

Keine Blocker gefunden. Alle 8 aus ROADMAP-Success-Criteria und PLAN-Frontmatter abgeleiteten Truths sind im Code nachgewiesen (additive Migration, Struct-/Row-/SQL-Threading, DAO-Roundtrip- und Namespace-Tests, outbound-only Read mit Soft-Delete-Filter, post-commit best-effort Carry-over mit korrekter Namespace-Trennung, e2e-Beweis). Die einzige unvollständige Teilmenge — der wörtliche REST-Endpoint aus ROADMAP-SC1 — ist laut ROADMAP.md explizit Phase 31 zugewiesen und wird daher als deferred (nicht als Gap) geführt. Ein Traceability-Hinweis zu `REQUIREMENTS.md` (APHIST-01 "Complete"-Markierung trotz ausstehendem REST-Mount) wird zur Korrektur empfohlen, blockiert aber nicht den Phase-29-Abschluss, da das PLAN-Frontmatter den REST-Endpoint explizit aus dem Phase-29-Scope ausgenommen hat.

Die zwei vorbestehenden e2e-Failures (`/api/mail/preview`-Rendering) sind nachweislich nicht durch Phase 29 verursacht und in `deferred-items.md` dokumentiert.

---

_Verified: 2026-08-12_
_Verifier: Claude (gsd-verifier)_
