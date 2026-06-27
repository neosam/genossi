---
created: 2026-06-27T15:52:30.497Z
title: Originalen Mitgliedsantrag als Datei-Attachment an Application hinterlegen
area: general
files:
  - genossi_dao/src/ (Application-Entity)
  - genossi_service_impl/src/application.rs
  - genossi_rest/src/application.rs
---

## Problem

Bei Mitgliedsanträgen (Application) gibt es aktuell keine Möglichkeit, den
originalen Mitgliedsantrag als Datei (z. B. eingescanntes PDF) am Antrag zu
hinterlegen. Beim Aktivieren eines Antrags (Application → Member) soll dieses
Original-Dokument automatisch übernommen werden, damit es ohne erneutes
manuelles Hochladen direkt am Mitglied verfügbar ist.

## Solution

TBD — grobe Richtung:

- Datei-Upload für Application (analog zu bestehenden Document-Uploads, via
  `DocumentStorage` auf dem Filesystem speichern, nicht in der DB).
- Beim Aktivieren der Application das hinterlegte Original automatisch als
  `MemberDocument` an das neu angelegte Mitglied übernehmen.
- Audit-Pflicht beachten: Application und MemberDocument sind auditierte
  Entitäten → `audited_*!`-Macros verwenden.
- Frontend: Upload-/Anzeige-UI Component-First (kein Inline-RSX-Duplikat),
  ggf. bestehende Attachment-Komponente aus Phase 19 wiederverwenden.
