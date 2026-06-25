---
title: "DRY: \"admin\"-Privilege-Literal in 13 Page-Dateien als const zentralisieren"
date: 2026-06-14
priority: medium
source: Code-Audit 2026-06-14 (Redundanz)
blocked_by: keiner
---

# admin-Privilege-Literal zentralisieren

## Was

Das String-Literal `"admin"` (jeweils `privilege: "admin"` + `AccessDeniedPage { required_privilege: "admin".to_string() }`) ist über 13 Page-Dateien im Frontend verstreut — 25 Vorkommen: `applications_page.rs`, `audit_log.rs`, `inbox_page.rs`, `mail_page.rs`, `mail_jobs_page.rs`, `config_page.rs`, `assemblies.rs`, `assembly_details.rs`, `mail_templates.rs`, `permissions.rs`, `repayment_phases.rs`, `repayment_phase_details.rs`, `member_details.rs`.

## Warum

Ein Tippfehler (`"admni"`) in einer Datei fällt nicht auf und öffnet/sperrt eine Seite falsch. Magic-String über die Codebasis verteilt.

## Fix

1. `const PRIVILEGE_ADMIN: &str = "admin";` zentral definieren (z.B. in `genossi-frontend/src/permission` oder einem `constants`-Modul).
2. Alle 25 Vorkommen darauf umstellen.

## Akzeptanz

- Kein rohes `"admin"`-Literal mehr in Pages (grep clean)
- WASM-Build grün

## Routing

`/gsd-quick` — mechanisch.
