---
status: partial
phase: 02-helfer-token-session-authcontext-helper
source: [02-VERIFICATION.md, 02-REVIEW.md]
started: 2026-05-03T16:50:00Z
updated: 2026-05-03T16:50:00Z
---

## Current Test

[awaiting human decision]

## Tests

### 1. BLOCKER-04: validate_code_format Unicode-Lookalike-Bug
expected: Code mit Unicode-Lookalikes (z.B. `Ā=U+0100`, low-byte `0x00 = '0'`) sollte mit `400 ValidationError` abgewiesen werden, nicht mit `404 NotFound`. Aktuelle Implementierung in `genossi_service_impl/src/helper_token.rs:113` macht `(c as u8).contains(...)` — das truncated U+0100 zu 0x00 und akzeptiert es als gültiges Crockford-Zeichen.
result: [pending]

### 2. BLOCKER-01: AuthContext::Helper im Request-Lifecycle nie konstruiert
expected: Helper-Cookie löst beim Request-Roundtrip `AuthContext::Helper`-Konstruktion aus.
actual: `genossi_rest/src/session.rs:73-129` (`context_extractor`) ruft `verify_user_session` auf, NICHT `extract_auth_context`. Phase-2-Service-Layer-Code ist korrekt + unit-getestet, aber Pipeline ist nicht verdrahtet.
result: [pending — VERIFIER deferred to Phase 3 SC#6+SC#8]

### 3. BLOCKER-02: PRAGMA foreign_keys = ON fehlt global
expected: SQLite erzwingt FK-Constraints zur Laufzeit (`helper_token.assembly_id REFERENCES assembly(id)`).
actual: Migration deklariert FKs, aber Connection-Setup setzt `PRAGMA foreign_keys = ON` nicht — silently ignoriert. Pre-existing über alle Tabellen, nicht Phase-2-spezifisch.
result: [pending]

### 4. BLOCKER-03: OIDC-Build-Public-Route + doppelte CookieManagerLayer
expected: OIDC-Build ist E2E-verifiziert; Redeem-Endpoint funktioniert ohne CookieManagerLayer-Doppelung.
actual: E2E-Tests laufen ausschließlich im mock_auth-Build. OIDC-Build kompiliert, aber keine Verifikation.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
