---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-07
subsystem: mail-inbox
tags: [security, dos-protection, rust, mail-parser, probe-read, attachments, gap-closure]

# Dependency graph
requires:
  - phase: 19-02
    provides: "extract_attachments + persist_attachment + ATTACHMENT_MAX_BYTES (10-MB-Cap), die bisher unbedingt VOR der Cap-Pruefung allokierten"
  - phase: 19-04
    provides: "run_attachment_backfill Callsite, die ebenfalls auf die neue persist_attachment-Signatur migriert werden musste"
provides:
  - "Probe-Read-Pattern in extract_attachments: D-02 (Memory-DoS-Schutz) greift VOR jeder Heap-Allokation"
  - "ParsedAttachment.declared_size: u64 fuer konsistente size_bytes-Persistierung auch bei oversized (Vec leer, declared_size > Cap)"
  - "persist_attachment(declared_size) Signatur — oversized + size_bytes werden aus declared_size berechnet, nicht aus bytes.len()"
  - "Neuer Unit-Test test_extract_attachments_oversized_skips_materialization als Regression-Guard fuer das Probe-Read-Verhalten"
affects: [phase-19 verifier, future mail-attachment-pipelines, future inbound-mail-features]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Probe-Read vor Heap-Allokation: bei externen Daten unbekannter Groesse zuerst die deklarierte Laenge pruefen (slice via &[u8]), erst dann to_vec()"
    - "Source-of-Truth-Trennung: declared_size (real, vom Sender) vs. bytes.len() (materialisiert, evtl. 0 bei DoS-Schutz)"

key-files:
  created:
    - ".planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-07-SUMMARY.md"
  modified:
    - "genossi_mail/src/inbox.rs"

key-decisions:
  - "Probe-Read statt Pre-Parse-Guard: ein vorgelagerter `MAX_MAIL_SIZE`-Check in poll_once (Defense-in-Depth-Empfehlung aus VERIFICATION) wurde NICHT mitgezogen. Begruendung: Reduziert Komplexitaet; der Probe-Read schliesst CR-01 vollstaendig. Pre-Parse-Guard bleibt fuer ein separates Follow-up gegen mail_parser-Heap-Verbrauch reserviert, falls konkreter Bedarf entsteht."
  - "declared_size in der Struct (nicht nur lokal in extract_attachments): persist_attachment muss die echte Groesse bis in die DB-Row durchreichen, damit `oversized=true` + `size_bytes=<echte_Groesse>` (statt 0) persistiert wird. Verhindert Inkonsistenz zwischen oversized-Marker und Groessenangabe im Frontend."
  - "to_vec()-Calls bleiben im Else-Zweig erlaubt: pattern `if oversized { Vec::new() } else { part.contents().to_vec() }` ist ueberpruefbar (grep + manuelle Inspektion); kein abstrahiertes Helper-Wrapping notwendig fuer 2 Callstellen."

patterns-established:
  - "Probe-Read vor Allokation: `let raw_len = part.contents().len()` — `part.contents()` liefert `&[u8]` ohne Alloc; erst `.to_vec()` kopiert in den Heap. Eignet sich fuer jedes Streaming-Parser-Interface, das Lazy-Slices liefert."
  - "Declared-size-Separation: bei DoS-geschuetzten Pipelines muss die wahre Groesse vom Materialisierungs-Status getrennt werden, damit Metadata (Audit-Log, UI-Anzeige) konsistent bleibt — `bytes.len()` allein reicht nicht."

requirements-completed:
  - D-02

# Metrics
duration: ~20min
completed: 2026-06-07
---

# Phase 19 Plan 07: Memory-DoS-Fix Summary

**Probe-Read-Pattern in `extract_attachments` schliesst CR-01 BLOCKER: 10-MB-Cap greift jetzt VOR Heap-Allokation, nicht erst beim Persist-Schritt.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-06-07T~15:25Z
- **Completed:** 2026-06-07T~15:45Z
- **Tasks:** 2 (1 Fix-Task + 1 Verifikations-Task)
- **Files modified:** 1 (`genossi_mail/src/inbox.rs`)

## Accomplishments

- **CR-01 BLOCKER geschlossen** — VERIFICATION Truth #3 ("Worker materialisiert Attachment-Bytes erst dann, wenn die 10-MB-Cap geprueft ist") ist nun erfuellbar.
- **`ParsedAttachment.declared_size: u64`** — echtes Senderangaben-Feld, unabhaengig von `bytes.len()`.
- **Probe-Read in `extract_attachments`** — `part.contents().len()` als Probe; bei `raw_len > ATTACHMENT_MAX_BYTES` wird `Vec::new()` allokiert statt `part.contents().to_vec()`.
- **`persist_attachment(declared_size)`** — erweiterte Signatur; `oversized` + `size_bytes` werden aus `declared_size` berechnet, nicht aus `bytes.len()`.
- **Beide Caller migriert** — `poll_once` (Z. 791) und `run_attachment_backfill` (Z. 940) reichen `att.declared_size` durch.
- **Neuer Regressions-Test** — `test_extract_attachments_oversized_skips_materialization` (multipart/mixed mit > 10 MB `text/plain`-Body-Part, `Content-Transfer-Encoding: 8bit`). Assertion: `att.declared_size > ATTACHMENT_MAX_BYTES` UND `att.bytes.is_empty() == true`.

## Task Commits

Jeder Task atomar committet:

1. **Task 1+2 zusammen (Fix + Regression Check):** `146875ea` (`fix(19-07): probe-read pattern in extract_attachments (D-02 / CR-01 memory-DoS guard)`)

   _Begruendung fuer den kombinierten Commit:_ Der RED-Test bleibt vor dem Schema-Refactor nicht standalone kompilierbar (`declared_size` existiert noch nicht). RED→GREEN als ein logischer Refactor-Schritt; TDD-Discipline ist in den Schritten dokumentiert (RED-Phase erst lokal verifiziert: `cargo test ... → no field declared_size`, danach GREEN). `workflow.tdd_mode` ist nicht aktiv — die `tdd_review_checkpoint`-Gate-Pruefung greift nicht.

## Files Created/Modified

- `genossi_mail/src/inbox.rs` — Probe-Read + declared_size field + persist_attachment-Signatur + 2 Caller-Migrationen + neuer Test + 2 bestehende Tests an neue Signatur angepasst + assert in `test_parse_raw_mail_extracts_attachments` (declared_size == bytes.len() unter Cap).

## Decisions Made

- **Probe-Read vor Heap-Alloc** (statt z.B. `mail_parser`-internal Cap oder `MAX_MAIL_SIZE`-Pre-Guard): minimaler Footprint, schliesst CR-01 vollstaendig, kein neuer dependency.
- **declared_size als Struct-Field** (nicht nur lokal): die DB-Row muss die echte Groesse persistieren, sonst zeigt das Frontend `0 bytes oversized` was inkonsistent ist (T-19-07-02).
- **Beide `to_vec()`-Stellen explizit guarded** (statt Helper-Function): nur 2 Aufrufe, beide im selben Func — Helper waere Overkill; `grep -nE 'part\.contents\(\)\.to_vec\(\)'` zeigt nun deterministisch, dass jede Stelle im else-Zweig liegt.

## Deviations from Plan

**None** — Plan exakt umgesetzt. Optional-Erweiterung `MAX_MAIL_SIZE`-pre-parse-Guard explizit als out-of-scope dokumentiert (per VERIFICATION-Empfehlung), nicht implementiert.

## Issues Encountered

**None** — TDD-Flow ohne Friktion:
- RED-Test scheiterte erwartungsgemaess mit Compile-Error `no field declared_size on type &ParsedAttachment`.
- Nach GREEN-Implementierung: `cargo test -p genossi_mail --lib` → 176 passed (175 vorher + 1 neu), 0 failed.
- `cargo check --workspace --exclude genossi-frontend` → clean.
- `cargo clippy -p genossi_mail --all-targets` → 0 neue Findings auf `inbox.rs`.

## User Setup Required

None — kein externes Service-Config noetig.

## Next Phase Readiness

- **Phase 19 BLOCKER-frei** — VERIFICATION kann jetzt 14/14 must-haves abnehmen (vorher 13/14).
- Out-of-scope-Follow-ups (alle Recommend / Info aus 19-REVIEW.md, nicht BLOCKER):
  - WR-01 (`short_mime` i18n), WR-02 (inline Toolbar-Refactor), WR-04 (Backfill-Happy-Path-Integration-Test), WR-05 (PII-Logging-Reduktion), WR-06 (Filename-Sanitization).
  - IN-01..IN-05 (Info-Level Refactor-Empfehlungen).
  - Optional `MAX_MAIL_SIZE`-Pre-Parse-Guard als Defense-in-Depth gegen `mail_parser`-Heap-Verbrauch (separater Follow-up bei Bedarf).

---
*Phase: 19-e-mail-anhaenge-anzeigen*
*Plan: 19-07 (gap-closure / BLOCKER fix)*
*Completed: 2026-06-07*
