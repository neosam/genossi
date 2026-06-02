---
status: partial
phase: 12-frontend-component-first
source: [12-VERIFICATION.md, 12-UAT-CHECKLIST.md Section L]
started: 2026-06-01T12:30:00Z
updated: 2026-06-01T12:30:00Z
---

## Current Test

[awaiting human testing on staging or with non-admin OIDC account]

## Tests

### 1. Auth-Gate `/repayment-phases` ohne admin-Privilege
**expected:** AccessDeniedPage statt Listen-Page
**result:** pending — Helper-Login lokal nicht verfügbar während UAT 2026-06-01
**notes:** Code (`RequirePrivilege`) ist identisch mit Phase-4-Produktions-Pattern (assemblies.rs etc.). Risiko gering.

### 2. Auth-Gate `/repayment-phases/{id}` ohne admin-Privilege
**expected:** AccessDeniedPage statt Detail-Page
**result:** pending — siehe Test 1
**notes:** gleicher RequirePrivilege-Wrap wie Test 1.

### 3. NavItem-Sichtbarkeit ohne admin-Privilege
**expected:** TopBar zeigt KEINEN "Anteils-Rückzahlung"-NavItem (show_admin-Gate)
**result:** pending — siehe Test 1
**notes:** Code-Coverage via `show_admin: true` auf NavItem in mitglieder_items.

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps

(noch keine Defekte gefunden — Tests sind nur deferred, nicht failed)

## Resolution Path

Tests können verifiziert werden via:
- `/gsd-verify-work 12` (interaktiv) wenn Helper-Login wieder funktioniert
- ODER auf Staging-Instanz mit echtem OIDC-Account
- ODER manuell mit `--features mock_auth` und einem nicht-admin-`Context`-Mock
