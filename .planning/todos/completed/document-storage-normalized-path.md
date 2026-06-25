---
title: "Defense-in-Depth: document_storage::full_path gibt unnormalisierten Pfad zurück"
date: 2026-06-14
priority: low
source: Code-Audit 2026-06-14 (Security)
blocked_by: keiner
---

# full_path normalisierten Pfad zurückgeben lassen

## Was

`genossi_service_impl/src/document_storage.rs:25-55`: `full_path` validiert die *normalisierte* Variante gegen das Base-Verzeichnis, gibt aber den *unnormalisierten* `joined` zurück.

## Warum

Aktuell **nicht** ausnutzbar — alle Aufrufer liefern server-generierte UUID-Pfade, kein User-Dateiname fließt ein, Extension auf `[a-z0-9]{1,10}` validiert. Wird aber fragil, sobald künftig ein Caller einen rohen Dateinamen durchreicht (Path-Traversal-Fläche). Härtung.

## Fix

`Ok(normalized)` statt `Ok(joined)` zurückgeben. Test: Pfad mit `..`-Segment wird abgelehnt bzw. normalisiert.

## Akzeptanz

- Rückgabe ist immer normalisiert
- bestehende Dokument-Tests grün

## Routing

`/gsd-quick` — Einzeiler + Test.
