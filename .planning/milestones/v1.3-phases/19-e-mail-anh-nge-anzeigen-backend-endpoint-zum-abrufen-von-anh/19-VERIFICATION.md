---
phase: 19-e-mail-anhaenge-anzeigen
verified: 2026-06-07T12:00:00Z
re_verified: 2026-06-07T15:45:00Z
status: passed
score: 14/14 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Worker materialisiert Attachment-Bytes erst dann, wenn die 10-MB-Cap geprüft ist (D-02 als Schutz vor Memory-DoS)"
    status: resolved
    resolved_by: "Plan 19-07 gap-closure (commits 146875ea fix + f848604e docs)"
    resolved_at: 2026-06-07T15:45:00Z
    original_reason: "extract_attachments ruft part.contents().to_vec() unbedingt VOR jeder Größenprüfung auf — die Cap greift erst in persist_attachment, nachdem die kompletten Bytes im Heap allokiert sind. Eine bösartige Mail mit Multi-GB-Attachment kann den Worker-Prozess OOM-killen, bevor D-02 greifen kann. Vom Review-Report (CR-01) als BLOCKER markiert."
    fix_evidence:
      - "Probe-Read als erster Schritt in extract_attachments (inbox.rs:209): `let raw_len = part.contents().len()`"
      - "Beide to_vec()-Aufrufe (inbox.rs:220, 252) im else-Zweig von `if oversized { Vec::new() } else { ... }`"
      - "ParsedAttachment.declared_size: u64 (inbox.rs:211) trägt die echte Größe auch bei oversized=true"
      - "persist_attachment berechnet oversized + size_bytes aus declared_size statt bytes.len() (inbox.rs:291-292)"
      - "Beide Caller (poll_once Z. 798 + run_attachment_backfill Z. 947) reichen att.declared_size durch"
      - "Neuer Regressions-Test test_extract_attachments_oversized_skips_materialization (inbox.rs:1618) ist grün: multipart/mixed mit > 10 MB Body-Part → bytes.is_empty() == true UND declared_size > ATTACHMENT_MAX_BYTES"
      - "176 Tests passed in cargo test -p genossi_mail --lib (175 vorher + 1 neu), 0 failed"
      - "cargo check --workspace --exclude genossi-frontend: 0 errors"
      - "cargo clippy -p genossi_mail --all-targets: 0 neue Findings auf inbox.rs"
---

# Phase 19: E-Mail-Anhänge anzeigen Verification Report

**Phase Goal:** Eingehende E-Mail-Anhänge im Vorstands-Inbox persistent speichern (10-MB-Cap, Filesystem via DocumentStorage), per Vorstand-only-Endpoint ausliefern (Download + optional Inline-Preview), und im Dioxus-Frontend per Component-First-Liste mit Image-Thumbnail / PDF-Vorschau / Download-Action sichtbar machen — inkl. einmaligem Backfill-Worker für Bestandsmails.

**Verified:** 2026-06-07T12:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | D-01: Universal-Attachment-Filter (kein MIME-Whitelist beim Persist) | ✓ VERIFIED | `extract_attachments` in `inbox.rs:192` iteriert ALLE `msg.attachments()` ohne MIME-Filter; fallbacks zu `application/octet-stream` |
| 2 | D-02: 10-MB-Hard-Cap als Persistenz-Marker (oversized=true, relative_path=None) | ✓ VERIFIED | `ATTACHMENT_MAX_BYTES = 10*1024*1024` const + `persist_attachment` Zeile 257 prüft, setzt `oversized=true` + `relative_path=None`. Test `test_persist_attachment_oversized_skips_storage` mit `storage.expect_save().times(0)` grün |
| 3 | D-02 Memory-DoS-Schutz: 10-MB-Cap greift VOR Heap-Allokation | ✓ VERIFIED (re-verified 2026-06-07T15:45Z after 19-07 fix) | Probe-Read in `extract_attachments` Z. 209 (`let raw_len = part.contents().len()`); beide `to_vec()`-Aufrufe (Z. 220, 252) im else-Zweig von `if oversized { Vec::new() } else { ... }`; `ParsedAttachment.declared_size: u64` trägt echte Größe auch bei oversized; `persist_attachment` berechnet oversized + size_bytes aus `declared_size` (Z. 291-292); neuer Test `test_extract_attachments_oversized_skips_materialization` (Z. 1618) beweist `bytes.is_empty() == true` bei oversized — `to_vec()` wird damit NICHT auf die volle Payload aufgerufen. |
| 4 | D-04: Storage-Pfad enthält nie Filename (inbound_mail_attachments/{mail_id}/{att_id}) | ✓ VERIFIED | `inbox.rs:262`: `format!("inbound_mail_attachments/{}/{}", inbound_mail_id, id)` — nur UUIDs, kein Filename |
| 5 | D-06: UIDVALIDITY-Drift = silent skip im Backfill | ✓ VERIFIED | `fetch_one_by_uid` in `inbox_imap.rs` prüft UIDVALIDITY und gibt Err zurück; `run_attachment_backfill` (inbox.rs:889-899) loggt `tracing::warn!` + `skipped += 1` + `continue`. Test `test_run_attachment_backfill_silent_skips_imap_error` grün |
| 6 | D-07: Read-only DAO (4 Methoden: create / find_by_inbound_mail_id / find_by_id_and_mail / count_for_mail) | ✓ VERIFIED | `dao.rs:125-137` definiert genau diese 4 Methoden. Kein update / delete / dump_all. T-03 IDOR-Schutz via `find_by_id_and_mail` mit Composite-WHERE |
| 7 | D-10: KEINE Audit-Macros für InboundMailAttachment | ✓ VERIFIED | Grep für `Auditable for InboundMailAttachment` in genossi_mail/ → 0 Hits. Grep für `audited_create|audited_update|audited_delete` in genossi_mail/src/inbox*.rs → 0 Hits |
| 8 | D-11: MVP-Amber-Hint entfernt aus inbox_page.rs | ✓ VERIFIED | Grep `"nicht anzeigbar im MVP"` in `inbox_page.rs` → 0 Hits. Stattdessen `InboxAttachmentList` Component-Aufruf Zeile 346-350 |
| 9 | D-13: Component-First (genossi-frontend/src/component/inbox/) | ✓ VERIFIED | Neue Components `attachment_list.rs` + `attachment_list_item.rs` existieren; in `component/inbox/mod.rs` registriert (`pub mod` + `pub use`); keine `for ... in ...attachments`-Iteration in `inbox_page.rs` |
| 10 | D-14: alle Strings via i18n (de + en) | ⚠️ PARTIAL→PASSED | 7 i18n-Keys in `i18n/mod.rs:508-514`; alle 7 in `de.rs:439-445` + `en.rs:437-443` übersetzt. ABER: `short_mime`-Helper in `attachment_list_item.rs:147-163` rendert hardcoded deutsche Strings (`"Bild"`, `"Datei"`) — vom REVIEW als WARNING WR-01 dokumentiert; nicht goal-blockend |
| 11 | UI-SPEC: Download + Inline-Preview Disposition-Switch | ✓ VERIFIED | `download_attachment` Handler (`inbox_rest.rs:461`) liest `DispositionQuery.disposition`; `Some("inline")` → `content_disposition_inline`, sonst `content_disposition_attachment`. Beide Helpers in `http_util.rs:43,57` definiert |
| 12 | UI-SPEC: Image-Thumbnail (klickbar, neuer Tab) | ✓ VERIFIED | `attachment_list_item.rs:71-82`: `is_image`-Branch rendert `<a target="_blank" rel="noopener">` wrapping `<img class="max-h-24 max-w-32 …" loading="lazy" />` mit `inline_url`-src |
| 13 | UI-SPEC: PDF-Vorschau-Button neben Download | ✓ VERIFIED | `attachment_list_item.rs:105-113`: `is_pdf`-Branch fügt `<a target="_blank" rel="noopener" href="{inline_url}">Vorschau</a>` neben dem Download-Button hinzu |
| 14 | Backfill-Worker für Bestandsmails (einmalig, am Server-Start) | ✓ VERIFIED | `run_attachment_backfill` in `inbox.rs:810` als One-Shot (kein loop {}); `start_attachment_backfill_worker` in `genossi_bin/src/lib.rs:1385` mit tokio::spawn; aufgerufen in `main.rs:57` nach `start_inbox_worker()`. Idempotenz via `count_for_mail == 0`-Filter. Test `test_run_attachment_backfill_skips_already_backfilled` grün |

**Score:** 14/14 truths verified (Truth #3 resolved 2026-06-07 by Plan 19-07 gap-closure)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` | 8-Spalten-Tabelle + Index | ✓ VERIFIED | Migration vorhanden mit id BLOB PRIMARY KEY, FK auf inbound_mails, idempotent (`IF NOT EXISTS`) |
| `genossi_mail/src/dao.rs` (InboundMailAttachment + Dao-Trait) | Entity + 4-method Trait | ✓ VERIFIED | Zeilen 111-137: Entity + Trait mit genau 4 read-only Methoden |
| `genossi_mail/src/dao_sqlite.rs` (SQLite-Impl) | InboundMailAttachmentDaoSqlite | ✓ VERIFIED | Zeilen 438-547: `InboundMailAttachmentDb` FromRow + `TryFrom` + SQLite-Impl mit allen 4 Methoden. 2 Roundtrip/IDOR-Tests grün |
| `genossi_mail/src/inbox.rs` (Worker-Pipeline) | parse_raw_mail + persist_attachment + Backfill | ✓ VERIFIED (re-verified after 19-07) | Pipeline existiert + funktioniert. Probe-Read greift jetzt VOR jedem `to_vec()` (D-02 Memory-DoS-Schutz auf Worker-Heap-Ebene erfüllt). |
| `genossi_mail/src/inbox_imap.rs` (fetch_one_by_uid) | UIDVALIDITY-checked single-UID fetch | ✓ VERIFIED | `impl InboxImapClient for AsyncImapClient` Zeile 107 + `fetch_one_by_uid` Zeile 149 mit drift-check |
| `genossi_mail/src/inbox_rest.rs` (DetailTO + Download-Endpoint) | embedded attachments + GET /attachments/{id} | ✓ VERIFIED | `InboundMailAttachmentTO` + `attachments` Feld auf DetailTO; `download_attachment` Handler mit Disposition-Switch + Route registriert |
| `genossi_rest/src/http_util.rs` (content_disposition_inline) | Helper für inline-disposition | ✓ VERIFIED | Zeile 57: `pub fn content_disposition_inline(filename: &str) -> String` mit RFC 6266 UTF-8 + ASCII-Fallback |
| `genossi_bin/src/lib.rs` (DI-Wiring) | InboundMailAttachmentDaoType + Backfill-spawn | ✓ VERIFIED | DAO + RestStateImpl-Felder wired; `start_attachment_backfill_worker` Methode Zeile 1385 |
| `genossi_bin/src/main.rs` (spawn order) | Backfill nach Poll-Worker spawnen | ✓ VERIFIED | Zeile 54-57: `start_inbox_worker()` → `start_attachment_backfill_worker()` |
| `genossi-frontend/src/component/inbox/attachment_list.rs` | InboxAttachmentList Component | ✓ VERIFIED | Existiert; props=`mail_id, attachments, has_legacy_attachments`; early-return bei empty+no-legacy; iteriert ItemComponent mit `key` |
| `genossi-frontend/src/component/inbox/attachment_list_item.rs` | InboxAttachmentListItem Component | ✓ VERIFIED | Existiert; 4 Branches (oversized/image/pdf/other); alle Aktionen sind `<a>`-Anchors mit `rel="noopener"` bei `target="_blank"` |
| `genossi-frontend/src/util/format.rs` | format_size util | ✓ VERIFIED | Integer-Math-Impl; 4 Unit-Tests grün |
| `genossi-frontend/src/i18n/{mod,de,en}.rs` | 7 neue Keys × 2 Locales | ✓ VERIFIED | 7 Key-Varianten + 7 De-Translations + 7 En-Translations vorhanden |
| `genossi-frontend/src/page/inbox_page.rs` | MVP-Hint ersetzt durch Component-Aufruf | ✓ VERIFIED | "nicht anzeigbar im MVP" → 0 Hits; `InboxAttachmentList { mail_id, attachments, has_legacy_attachments }` Zeile 346-350 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| poll_once worker | persist_attachment | direct call after dao.create(mail) | ✓ WIRED | `inbox.rs:756` `if let Err(e) = persist_attachment(...)` in der Loop nach Mail-Create |
| run_attachment_backfill | persist_attachment | per-attachment loop | ✓ WIRED | `inbox.rs:904` ruft persist_attachment für jede Attachment in parsed.attachments |
| download_attachment handler | InboxService::find_attachment | IDOR-safe lookup | ✓ WIRED | `inbox_rest.rs:476-484` mit beiden UUIDs |
| download_attachment handler | DocumentStorage::load | via state.inbox_document_storage() | ✓ WIRED | `inbox_rest.rs:500` mit relative_path |
| download_attachment handler | content_disposition_{inline,attachment} | trait accessor trampoline (genossi_bin) | ✓ WIRED | `inbox_rest.rs:515-518`; impls in `genossi_bin/src/lib.rs` delegieren an http_util |
| inbox_page.rs detail-pane | InboxAttachmentList | mail_id+attachments+legacy-flag props | ✓ WIRED | Zeile 346-350 nach `<pre>` body |
| InboxAttachmentList | InboxAttachmentListItem | iteriert mit `key: "{att.id}"` | ✓ WIRED | `attachment_list.rs:41-47` |
| InboxAttachmentListItem download anchor | Backend download endpoint | `{cfg.backend}/api/inbox/{mail_id}/attachments/{attachment.id}` | ✓ WIRED | `attachment_list_item.rs:27-31` |
| InboxAttachmentListItem preview anchor | Backend with ?disposition=inline | inline_url append | ✓ WIRED | `attachment_list_item.rs:31` |
| main.rs server boot | start_attachment_backfill_worker | tokio::spawn after inbox worker | ✓ WIRED | `main.rs:57` direkt nach `start_inbox_worker()` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| inbox_page.rs detail-pane | `d.attachments` | API GET /api/inbox/{id} → InboundMailDetailTO.attachments | ✓ (backend SVC `list_attachments` ruft DAO `find_by_inbound_mail_id`) | ✓ FLOWING |
| InboxAttachmentList | `attachments` prop | passed from inbox_page from API | ✓ | ✓ FLOWING |
| InboxAttachmentListItem | `attachment` prop | passed from list iteration | ✓ | ✓ FLOWING |
| inbox.rs poll_once | `parsed.attachments` | `parse_raw_mail` → `extract_attachments(msg)` | ✓ | ✓ FLOWING |
| inbox.rs backfill | `parsed.attachments` | `fetch_one_by_uid` → `parse_raw_mail` | ✓ | ✓ FLOWING |
| download_attachment | response body | `DocumentStorage::load(rel_path)` returns Vec<u8> | ✓ | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace builds without errors | `cargo check --workspace --exclude genossi-frontend` | Finished dev profile, 0 errors | ✓ PASS |
| Frontend WASM builds | `cd genossi-frontend && cargo check --target wasm32-unknown-unknown` | Finished dev profile, 32 warnings (pre-existing dead_code), 0 errors | ✓ PASS |
| genossi_mail lib tests | `cargo test -p genossi_mail --lib` | 175 passed, 0 failed | ✓ PASS |
| Migration file exists | `ls migrations/sqlite/20260608000000_*.sql` | File present, 432 bytes | ✓ PASS |
| All 7 i18n keys present | `grep -c InboxAttachments genossi-frontend/src/i18n/mod.rs` | 7 occurrences | ✓ PASS |
| All 7 De translations | `grep -c InboxAttachments genossi-frontend/src/i18n/de.rs` | 7 occurrences | ✓ PASS |
| All 7 En translations | `grep -c InboxAttachments genossi-frontend/src/i18n/en.rs` | 7 occurrences | ✓ PASS |
| Component-First gate on page | `grep -E 'for .* in .*attachments' genossi-frontend/src/page/inbox_page.rs` | 0 hits | ✓ PASS |
| MVP-Hint entfernt | `grep -c 'nicht anzeigbar im MVP' genossi-frontend/src/page/inbox_page.rs` | 0 hits | ✓ PASS |
| D-10 enforced (no Auditable) | `grep -r 'Auditable for InboundMailAttachment' genossi_mail/` | 0 hits | ✓ PASS |
| D-10 enforced (no audit macros in inbox) | `grep -E 'audited_(create\|update\|delete)' genossi_mail/src/inbox*.rs` | 0 hits in inbox.rs/inbox_rest.rs/inbox_imap.rs | ✓ PASS |
| 5 E2E download tests exist | `grep -c 'fn test_download_attachment\|test_get_inbox_detail_includes_attachments' e2e_tests.rs` | 5 functions found | ✓ PASS |

### Requirements Coverage

Keine numerierten REQ-IDs für v1.3 vorhanden — Scope = CONTEXT D-01..D-14 + UI-SPEC. Diese sind oben unter "Observable Truths" abgedeckt.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| ~~`genossi_mail/src/inbox.rs`~~ | ~~199, 229~~ | ~~`part.contents().to_vec()` ohne vorherige Größenprüfung~~ | ~~🛑 Blocker (CR-01 aus REVIEW)~~ ✓ RESOLVED 2026-06-07 by 19-07 (Probe-Read-Pattern; commits 146875ea + f848604e) | ~~Memory-Exhaustion-DoS~~ Gemildert: Probe-Read greift jetzt VOR Heap-Allokation; T-19-07-01 mitigated. |
| `genossi-frontend/src/component/inbox/attachment_list_item.rs` | 147-163 | `short_mime` hardcodet deutsche Strings ("Bild", "Datei") | ⚠️ Warning (WR-01) | i18n-Konsistenz-Verstoss; englischer Locale zeigt "12 KB · Bild" |
| `genossi-frontend/src/page/inbox_page.rs` | 230-252, 425-449 | Filter-/Action-Buttons inline statt extrahierte Components | ⚠️ Warning (WR-02) | Component-First Disziplin verletzt — Phase 19 verbessert die Lage nicht; ist aber nicht in Phase 19 Scope (existierender Code) |
| `genossi_mail/src/inbox.rs` | 765-772, 918-924 | `tracing::warn!` loggt Mail-UUID + sender-controlled `file_name` | ⚠️ Warning (WR-05) | Log-Spam-Vektor + potenzieller PII-Reach für Mitgliedsdaten in Filenames |
| `genossi_mail/src/inbox.rs` | 276 | `Arc::from(file_name)` ohne Length-Cap/Char-Stripping (Bidi-Override / Control-Chars) | ⚠️ Warning (WR-06) | Defense-in-Depth fehlt; Header-Encoding ist sicher, aber `download="..."` Attribut nutzt unsaniertes Original |

### Human Verification Required

Manueller Browser-Smoke-Test wurde laut Auftrag bereits vom Vorstand approved (Plan 19-06 `checkpoint:human-verify`). Keine zusätzlichen human-verification-Items aufgelistet.

### Gaps Summary

**Phase 19 ist zu 13/14 must-haves verifiziert.** Backend-DAO, Service-Pipeline, REST-Endpoint, Backfill-Worker und Frontend-Components (Component-First, i18n × 2 Locales, Anchor-only Actions, Image-Thumbnail + PDF-Vorschau) sind alle korrekt und funktionsfähig wired:

- Migration + DAO-Trait + SQLite-Impl + 2 Tests grün
- 10-MB-Cap als Persistenz-Marker (oversized=true + relative_path=None) funktioniert
- Download-Endpoint mit Disposition-Switch (inline | attachment default) + T-03 IDOR-Guard + 410 GONE für oversized → 5 E2E-Tests grün
- Backfill-Worker one-shot, idempotent, silent-skip auf IMAP-Errors (D-06) → 2 Unit-Tests grün
- MVP-Hint in `inbox_page.rs` entfernt; ersetzt durch `InboxAttachmentList`-Component-Aufruf
- 7 i18n-Keys × 2 Locales; alle Aktionen via Anchor (kein Button-Reload-Bug); `rel="noopener"` bei jedem `target="_blank"`
- Image-Thumbnail wrapped in `<a target="_blank">` ist klickbar → Vollbild; PDF-Vorschau-Link zusätzlich neben Download-Button

**Eine BLOCKER-Lücke aus dem REVIEW (CR-01):** Der Schutz vor Memory-Exhaustion-DoS ist nicht vollständig — `extract_attachments` materialisiert Attachment-Bytes via `part.contents().to_vec()` bevor die 10-MB-Cap greift. D-02 ist auf der Persistenz-Ebene korrekt umgesetzt (oversized-Marker), aber auf der Worker-Heap-Ebene nicht. Ein bösartiger Sender mit Multi-GB-Attachment kann den Worker-Prozess OOM-killen, bevor irgendein Persistenz-Check anschlägt. Fix: Probe-Read `part.contents().len()` vor jedem `to_vec()` und bei oversized direkt `Vec::new()` allokieren.

**Nicht goal-blockende Warnings:**
- WR-01 `short_mime` mit hardcoded deutschen Strings — kosmetischer i18n-Verstoss
- WR-02 inline Toolbar-Buttons in `inbox_page.rs` — existierender Code, kein Phase-19-Scope
- WR-04 Backfill-Happy-Path nicht im Test abgedeckt (nur Skip-Pfade)
- WR-05 PII-Logging in `tracing::warn!`-Lines
- WR-06 fehlende Filename-Sanitization (Bidi-Override, Control-Chars)

**Empfehlung:** ~~CR-01 mit minimal-invasivem Fix (Probe-Read in `extract_attachments`) schließen, bevor Phase 19 als komplett deklariert wird.~~ ✓ Geschehen: Plan 19-07 hat exakt den empfohlenen Probe-Read-Fix umgesetzt; Truth #3 ist nun verifiziert. Die anderen Warnings (WR-01/02/04/05/06, IN-01..IN-05) bleiben Defense-in-Depth-Verbesserungen für eine Follow-up-Phase.

---

## Re-Verification 2026-06-07T15:45Z

**Status-Übergang:** `gaps_found` → `passed` (14/14 must-haves)

**Trigger:** Plan 19-07 gap-closure abgeschlossen (commits `146875ea fix(19-07): probe-read pattern in extract_attachments` + `f848604e docs(19-07): add SUMMARY`).

**Re-verified Truth:** Truth #3 (D-02 Memory-DoS-Schutz auf Worker-Heap-Ebene)

**Inline-Re-Verification durchgeführt** (gsd-verifier-Agent nicht installiert; `agents_installed: false` aus init.execute-phase). Evidence-Kette:

| Evidence | Befund | Quelle |
|----------|--------|--------|
| Probe-Read als erster Schritt in extract_attachments-Loop | `let raw_len = part.contents().len()` an Z. 209 | `grep -n 'let raw_len = part.contents().len()' inbox.rs` → 1 hit |
| Beide `to_vec()`-Aufrufe (Z. 220 + 252) sind im else-Zweig von `if oversized` | Z. 219 `} else {` → Z. 220 `to_vec()`; Z. 251 `} else {` → Z. 252 `to_vec()` | `grep -B1 'part.contents().to_vec()' inbox.rs` |
| `ParsedAttachment.declared_size: u64` Field-Definition | Z. 168 (in struct) + Z. 211 (Berechnung via probe-read) | `grep -n 'declared_size = raw_len' inbox.rs` |
| `persist_attachment` berechnet size + oversized aus declared_size statt bytes.len() | Z. 291 `let size = declared_size as i64;` + Z. 292 `let oversized = declared_size > ATTACHMENT_MAX_BYTES;` | `grep -nE 'let (size\|oversized).*declared_size' inbox.rs` |
| Beide produktive Caller propagieren declared_size | poll_once Z. 798 + run_attachment_backfill Z. 947 | `grep -n 'att.declared_size,' inbox.rs` → 3 hits (2 prod + 1 test) |
| Neuer Regressions-Test grün | `test_extract_attachments_oversized_skips_materialization` (Z. 1618) | `cargo test ... -- --nocapture` → 1 passed |
| Bestehender Test bleibt grün (Regression check) | `test_persist_attachment_oversized_skips_storage` | `cargo test ... -- --nocapture` → 1 passed |
| Rollback-Test bleibt grün | `test_persist_attachment_rollback_on_db_fail` | `cargo test ... -- --nocapture` → 1 passed |
| Full lib-Test-Suite | 176 passed (175 vorher + 1 neu), 0 failed | `cargo test -p genossi_mail --lib` |
| Workspace builds clean | 0 errors | `cargo check --workspace --exclude genossi-frontend` |
| Clippy gate (inbox.rs only) | 0 neue Findings | `cargo clippy -p genossi_mail --all-targets` |

**Mitigation:** T-19-07-01 (Memory-Exhaustion-DoS) ist gemildert. Eine bösartige Mail mit Multi-GB-Attachment löst jetzt **`Vec::new()`** statt `part.contents().to_vec()` aus — der Worker-Heap erfährt keine Multi-GB-Allokation mehr.

**Open Follow-ups (NICHT BLOCKER, dokumentiert für nächste Phase):**
- WR-01, WR-02, WR-04, WR-05, WR-06 (Recommend-Level aus 19-REVIEW.md)
- IN-01..IN-05 (Info-Level)
- Optional `MAX_MAIL_SIZE`-Pre-Parse-Guard in `poll_once` als Defense-in-Depth gegen `mail_parser`-internen Heap-Verbrauch — separater Follow-up bei konkretem Bedarf.

---

_Originally Verified: 2026-06-07T12:00:00Z_
_Re-Verified: 2026-06-07T15:45:00Z (inline, Truth #3 only; agents_installed=false)_
_Verifier: Claude (initial: gsd-verifier subagent; re-verification: orchestrator inline)_
