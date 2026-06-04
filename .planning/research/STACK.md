# Stack Research — v1.2 Mitgliedschaft-Anpassungen

**Domain:** Brownfield-Erweiterung des Genossi-Workspace
**Researched:** 2026-06-04
**Confidence:** HIGH

## Zusammenfassung

**v1.2 fügt keinerlei neue Dependencies hinzu.** Alle 4 Operationen (Kündigung / Teil-Rückgabe / Übertrag / Aufstocken) bauen auf existierenden Crates auf, die in v1.0 und v1.1 bereits validiert wurden.

## Eingesetzte Existierende Stack-Komponenten

| Schicht | Bibliothek | Existierende Nutzung | Erweiterung für v1.2 |
|---------|-----------|----------------------|----------------------|
| Backend Async-Runtime | `tokio` 1.35+ | gesamtes Workspace | keine |
| HTTP-Framework | `axum` 0.8.3 | REST-Layer | neue Handler in `genossi_rest/src/membership_adjust.rs` (oder Erweiterung von `member.rs`) |
| ORM/DB | `sqlx` 0.8 + SQLite | DAO-Layer | evtl. neue Migration für ActionType-Enum-String-Erweiterung; sonst nur Service-Code |
| API-Doc | `utoipa` 5.0 | REST-Schema | neue TOs für 4 Operationen |
| Audit-Hash | `sha2` 0.10 | Hash-Chain | keine — `audited_*!`-Macros decken alles ab |
| UUID | `uuid` 1.6 | Entity-IDs | keine |
| Datetime | `time` 0.3 | ISO8601-Serde | Pure-Function `compute_effective_date(willensbekundung) -> (fiscal_year, exit_date)` mit `time::Date`-Arithmetik |
| Mocking | `mockall` 0.13 | Unit-Tests | keine |
| Frontend | `dioxus` 0.6.3 + Tailwind | WASM-UI | neuer `MembershipAdjustModal`-Component in `genossi-frontend/src/component/`, Button auf Member-Detail-Page |
| Frontend-JS-Bridge | `web-sys`, `wasm-bindgen` | div. Components | keine |
| Permission | bestehender `PermissionService` mit `ADMIN_PRIVILEGE` | v1.0/v1.1 | keine |

## Was NICHT in den Stack kommt

- Keine neue Crate-Dependency (z.B. kein `chrono`, `time` reicht; kein neues Permission-Crate)
- Keine Schema-Migrations für komplett neue Tabellen (nur Enum-String-Erweiterung der `member_action.action_type`-Spalte, falls die ActionTypes als Strings persistiert sind)
- Keine neuen OIDC-Roles (admin-only via existing `ADMIN_PRIVILEGE`)
- Kein neuer SMTP/IMAP-Pfad (keine Mail-Outputs)
- Keine PDF-Generierung (keine Dokumente)

## Migration-Skizze

Falls die `ActionType`-Werte als TEXT in SQLite gespeichert werden (zu verifizieren in `migrations/sqlite/`), wird ggf. eine forward-only Migration nötig:

```sql
-- Bei TEXT-Spalte: keine Schema-Migration nötig; neue Enum-Varianten werden beim
-- Insert als String akzeptiert. Falls CHECK-Constraint existiert, muss er
-- erweitert werden.
ALTER TABLE member_action ADD CONSTRAINT ... CHECK (action_type IN (
    'Eintritt', 'Austritt', 'Todesfall', 'Verkauf', 'Migration',
    'UebertragungAus', 'UebertragungEin', 'Aufstockung'
));
```

Falls die `ActionType`-Werte als INTEGER (Enum-Discriminator) gespeichert sind, ist kein Schema-Change nötig — nur Rust-Enum-Erweiterung.

## Verifikationspunkte für Discuss-Phase

- [ ] ActionType-Persistenz: TEXT vs. INTEGER in `migrations/sqlite/`? (Wenn TEXT mit CHECK → Migration nötig)
- [ ] Datepicker im Frontend: `web-sys`-native vs. eigene Component?
- [ ] Confirm-Modal: existing `RequirePrivilege` + `Modal` reuse (Phase 12 Pattern)?

## Quellen

- `Cargo.toml` Workspace + `.planning/codebase/STACK.md`
- v1.1 Phase 9 PaidOut-Cascade als Pattern-Anker
- v1.1 Phase 12 RepaymentEntryList-Multi-Select als Component-Reuse-Anker

---
*Stack research for: Genossi v1.2 Mitgliedschaft-Anpassungen*
*Researched: 2026-06-04*
