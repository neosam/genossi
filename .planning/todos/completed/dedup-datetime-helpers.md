---
title: "DRY: datetime-Helper (parse_datetime 7x, format_dt 6x, parse_date/format_date 2x) in DAO-Layer dedupen"
date: 2026-06-14
priority: medium
source: Code-Audit 2026-06-14 (Redundanz)
blocked_by: keiner
---

# datetime-Helper im DAO-Layer zentralisieren

## Was

Mehrere identische Datetime-Helper sind über die DAO-Dateien kopiert:

- **`parse_datetime()` — 7 Duplikate**: `member.rs:11-27`, `application.rs:12-28`, `user_preference.rs:11-27`, `member_action.rs:11-27`, `member_document.rs:11-27`, `audit_timestamp.rs:11-27`, `audit_log.rs:54-70`. Kanonische Version existiert bereits als `assembly.rs:14` (`pub(crate) fn parse_datetime`) — `repayment_phase.rs`, `repayment_entry.rs`, `helper_token.rs`, `assembly_member_snapshot.rs` importieren sie schon korrekt. Die 7 obigen haben den Import nie nachgezogen.
- **`format_dt()` — 6 Duplikate** (keine kanonische Version): `assembly.rs:83-88`, `helper_token.rs:12-17`, `repayment_phase.rs:66-71`, `repayment_entry.rs:60-65`, `assembly_member_snapshot.rs:55-60`, `attendance.rs:15-20`.
- **`parse_date()` / `format_date()` — 2 Duplikate**: `member.rs:29-36` und `member_action.rs:29-36`.

## Warum

Mechanische, risikoarme Konsolidierung. Die Codebasis ist bei `parse_datetime` sogar schon halb migriert und nur inkonsistent zurückgeblieben — ein Format-Bug müsste sonst an 7 Stellen gefixt werden.

## Fix

1. Ein `datetime_utils`-Modul in `genossi_dao_impl_sqlite/src/lib.rs` anlegen mit `pub(crate)` `parse_datetime`, `format_dt`, `parse_date`, `format_date`.
2. Die 7 + 6 + 2 lokalen Definitionen löschen, Imports umstellen.
3. Bestehende DAO-Tests müssen unverändert grün bleiben (Verhalten identisch).

## Akzeptanz

- Genau eine Definition pro Helper
- Workspace-Tests grün, clippy clean, keine Verhaltensänderung

## Routing

`/gsd-quick` — mechanischer Refactor, gut testabgedeckt.
