---
id: 260729-9ld
type: quick
description: "OIDC-Login: sub-Claim statt preferred_username"
status: complete
date: 2026-07-29
---

# Quick Task 260729-9ld — Summary

## Was geaendert wurde

### `genossi_rest/src/session.rs`

`register_session()` leitet die `user_id` jetzt aus `oidc_claims.subject()`
(`sub`) ab statt aus `preferred_username()`. `sub` ist nach OIDC Core 1.0 in
jedem ID-Token Pflicht, entsprechend liefert `openidconnect 3.5.0` ihn als
`&SubjectIdentifier` und nicht als `Option` — der bisherige
`"NoUsername"`-Fallback ist damit ersatzlos entfallen.

Neu ist die Hilfsfunktion `normalize_username(&str) -> Option<String>`: sie
trimmt den Rohwert und liefert `None`, wenn nichts Verwertbares uebrig bleibt.
In diesem Fall bricht `register_session()` mit 500 ab und loggt, statt ein
Session-Cookie zu setzen. Das schliesst die eigentliche Schwachstelle des alten
Codes: Der Login schlug nie hart fehl, sondern schleuste alle Nutzer in einen
gemeinsamen, per Auto-Registrierung angelegten Account ohne Rollen und
Privilegien.

Die Funktion ist auf `#[cfg(any(feature = "oidc", test))]` gegated — im
Default-Build (`mock_auth`) bleibt sie testbar, ohne in einem Nicht-OIDC-Release
als toter Code zurueckzubleiben.

### `doc/AUTHENTICATION.md`

Das Code-Listing im Abschnitt "Session Registration" zeigte noch den alten
`preferred_username`-Pfad samt Fallback und ist an den Code angeglichen.

## Tests

Fuenf neue Unit-Tests in `genossi_rest/src/session.rs`:

| Test | Abdeckung |
|------|-----------|
| `keeps_a_plain_subject_unchanged` | normaler `sub` bleibt unveraendert |
| `keeps_an_opaque_provider_subject_unchanged` | UUID-formige Subjects werden nicht veraendert |
| `trims_surrounding_whitespace` | fuehrende/nachfolgende Whitespaces werden getrimmt |
| `rejects_an_empty_subject` | leerer String → `None` |
| `rejects_a_whitespace_only_subject` | whitespace-only → `None` |

Ergebnis:
- `cargo test -p genossi_rest --lib session` → 6 passed, 0 failed
- `cargo test -p genossi_rest --features oidc --lib session` → 6 passed, 0 failed
- `cargo clippy -p genossi_rest --features oidc --all-targets` → keine neuen Warnungen in `genossi_rest`

Die verbleibenden Clippy-Warnungen stammen aus `genossi_service_impl`
(`unnecessary_sort_by` in `repayment_letter.rs`) und bestanden vor diesem Patch.

## Offener Punkt vor dem Deploy — bitte pruefen

Bestehende Zeilen in der `users`-Tabelle stammen aus der
`preferred_username`-Aera. Sie behalten ihre Rollen nur, wenn der Provider fuer
dieselbe Person ein **identisches** `sub` liefert.

Weicht `sub` ab, legt `ensure_user_exists()`
(`genossi_dao/src/permission.rs:23`) beim ersten Login stillschweigend neue,
rechtlose Accounts an — der Login funktioniert dann zwar, aber die Nutzer haben
keine Berechtigungen mehr. Vor dem Rollout also einmal abgleichen:

```bash
sqlite3 genossi.db "SELECT name FROM users"
```

Stimmen die Werte nicht mit den `sub`-Werten des Providers ueberein, muessen die
Rollen einmalig umgehaengt werden. Das ist ein Datenschritt und bewusst nicht
Teil dieses Patches.

## Nicht geaendert

- Kein Fallback auf weitere Claims (`email`, `nickname`) — `sub` wurde als
  alleinige Identitaetsquelle festgelegt.
- Keine Migration bestehender Usernamen (siehe offener Punkt oben).
