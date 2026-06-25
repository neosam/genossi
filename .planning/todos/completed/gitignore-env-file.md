---
title: "Hygiene: .env aus Git-Tracking entfernen + in .gitignore aufnehmen"
date: 2026-06-14
priority: low
source: Code-Audit 2026-06-14 (Security)
blocked_by: keiner
---

# .env untracken

## Was

`.env` (nicht `.env.example`) liegt eingecheckt im Tree (`.env:1-2`). Inhaltlich aktuell harmlos (nur `DATABASE_URL=sqlite:genossi.db` und `RUST_LOG=...`), aber `.gitignore` ignoriert nur `.env.local*`.

## Warum

Präventiv: verhindert künftiges versehentliches Committen echter OIDC-/SMTP-Secrets in dieselbe Datei.

## Fix

1. `git rm --cached .env`
2. `.env` zu `.gitignore` hinzufügen
3. Sicherstellen, dass `.env.example`/`.env.oidc.example` als Vorlage existieren (tun sie)

## Akzeptanz

- `.env` nicht mehr in `git ls-files`
- lokale Entwicklung funktioniert weiter (Datei bleibt auf Disk)

## Routing

`/gsd-quick` — git-Hygiene.
