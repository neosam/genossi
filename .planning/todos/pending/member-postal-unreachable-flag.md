---
created: 2026-06-17T04:28:37.850Z
title: Genosse als postalisch nicht erreichbar markieren
area: general
files:
  - genossi_dao/src/member.rs
  - genossi_service_impl/src/member.rs
  - genossi-frontend/src/page (Member-Detail)
---

## Problem

Manche Genossen sind per Post nicht erreichbar — z.B. weil die hinterlegte
Adresse unzustellbar ist (Brief kommt als Rückläufer zurück) oder kein gültiger
Versandweg bekannt ist. Aktuell gibt es keine Möglichkeit, das pro Genosse zu
vermerken. Folge: Bei postalischem Versand / Anschreiben (z.B. RepaymentLetter,
Bulk-Mail mit Brief-Anhang, GV-Einladungen) werden diese Mitglieder weiterhin
einbezogen, was zu unnötigen Rückläufern, Kosten und manuellem Nacharbeiten
führt.

## Status (Update 2026-06-25 — Quick 260625-e14)

**Teilweise umgesetzt.** Das Datenmodell, die Audit-Integration und das Frontend
sind fertig (Quick-Task `260625-e14`):
- `PostalStatus`-Enum (`Erreichbar`/`Unzustellbar`, erweiterbar) auf Member durch
  alle Backend-Layer, Migration `20260625000000_add_postal_status_to_member.sql`.
- Statuswechsel läuft auditiert über `audited_update!` (`postal_status` als letztes
  `audit_fields()`-Element).
- Frontend: wiederverwendbarer `MemberPostalStatusSelect`-Component auf der
  Member-Detail-Seite + i18n (de/en).

**Noch offen (dieser Todo bleibt für den Folge-Task):** Berücksichtigung in den
postalischen Versand-Flows (RepaymentLetter, Brief-Bulk-Mail, GV-Einladungen).
Entschiedene Richtung: markierte Mitglieder **warnen/kennzeichnen** (Pre-Flight-Liste),
**nicht** hart herausfiltern.

## Solution

Flag/Status pro Member, der den postalischen Versand-Workflow beeinflusst.

Offene Entscheidungen (vor Implementierung klären):
- **Bool-Flag vs. Status-Enum**: Reicht ein `postal_unreachable: bool`, oder
  braucht es Gründe/Status (z.B. `unzustellbar`, `umgezogen`, `verstorben`,
  `kein Versand erwünscht`)? Status-Enum ist verbandskonform nachvollziehbarer.
- **Audit-Pflicht**: Member ist eine auditierte Entität → Änderung des Flags
  MUSS über die Audit-Macros (`audited_update!`) laufen, nicht via direktem
  `member_dao.update`. Neues Feld in `Member::audit_fields()` aufnehmen.
- **Migration**: Neue Spalte in `migrations/sqlite/` (nullable / default false),
  bestehende Daten bleiben kompatibel.
- **Versand-Berücksichtigung**: Postalische Send-/Bulk-Flows (RepaymentLetter,
  Brief-Anhang, ggf. GV) sollen markierte Mitglieder herausfiltern oder
  zumindest deutlich kennzeichnen. Pre-Flight-Liste analog zu
  [[backend-pre-flight-check-attach-repayment-letter]] denkbar.
- **Frontend**: Toggle/Auswahl auf der Member-Detail-Seite (Component-First —
  kein inline-RSX-Duplikat), sichtbare Kennzeichnung in Empfänger-Listen.

Tests: DAO-Roundtrip des neuen Felds, Audit-Eintrag bei Änderung, Filterung im
Versand-Flow.

## Routing

`/gsd-quick --discuss` empfohlen — Bool-vs-Status-Entscheidung und der genaue
Umfang der betroffenen Versand-Flows sollten vorab geklärt werden.
