---
title: "Security: Klartext-Redeem-Code landet bei DEBUG-Log im Tracing-Span"
date: 2026-06-14
priority: medium
source: Code-Audit 2026-06-14 (Security)
blocked_by: keiner
---

# instrument(skip(body)) im redeem_helper_token-Handler

## Was

`genossi_rest/src/helper_token.rs:283` (`redeem_helper_token`) nutzt `#[instrument(skip(rest_state))]` — `body` wird NICHT geskippt. `Json(body): Json<RedeemRequest>` mit `#[derive(Debug)] pub code: String` (`genossi_rest_types/src/lib.rs:1685-1688`) landet bei DEBUG/TRACE-Logging im Klartext im Span.

## Warum

Der Service-Layer vermeidet das bewusst (`helper_token.rs:188`), der REST-Span hebelt es aus. Brisant v.a. bei *fehlgeschlagenen* Redeems mit noch gültigem Code (z.B. vor GV-Öffnung), die dann replay-bar wären. Voraussetzung: Log-Zugriff → Schwere mittel.

## Fix

`#[instrument(skip(rest_state, body))]` setzen. Einzeiler.

## Akzeptanz

- Redeem-Code erscheint nicht mehr im Tracing-Output (auch bei RUST_LOG=debug)
- clippy clean

## Routing

`/gsd-quick` — Einzeiler.
