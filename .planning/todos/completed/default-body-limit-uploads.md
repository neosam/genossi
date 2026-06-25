---
title: "Upload-Härtung: kein DefaultBodyLimit gesetzt — Service-Limits (10/50 MB) sind toter Code"
date: 2026-06-14
priority: low
source: Code-Audit 2026-06-14 (Security)
blocked_by: keiner
---

# DefaultBodyLimit pro Upload-Route setzen

## Was

`genossi_rest/src/lib.rs` setzt nirgends `DefaultBodyLimit` → axum-Default von 2 MB greift global. Die Service-seitigen Limits (`member_document.rs:20` = 50 MB, `static_document_service.rs:25` = 10 MB) werden nie erreicht; der Excel-Import (`member_import.rs:325`) liest `field.bytes()` komplett in den Speicher ohne eigene Größenprüfung.

## Warum

Inkonsistenz (konfigurierte Limits sind toter Code) + leichte DoS-Härtung. Geringes Risiko: Upload-Routes erfordern `manage_members`-Privileg + Rate-Limit (60/min).

## Fix

Pro Upload-Route bewusstes `DefaultBodyLimit::max(...)` passend zum jeweiligen Service-Limit setzen (50 MB für MemberDocument, 10 MB für StaticDocument, sinnvolles Limit für Import).

## Akzeptanz

- Upload über Limit → 413, nicht stiller 2-MB-Abbruch
- Service-Limits und Route-Limits konsistent

## Routing

`/gsd-quick` — Axum-Layer-Konfiguration.
