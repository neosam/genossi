---
title: Auszahlungsphase / Rückzahlungsphase — Konzept und Entscheidungen
date: 2026-05-29
context: Exploration via /gsd-explore — Vorbereitung für späteren Milestone
---

# Auszahlungsphase — Konzept (Exploration)

## Hintergrund

Wenn Genossen Anteile abtreten (Voll-Austritt oder Teil-Verkauf), muss der Vorstand
sie anschreiben (IBAN-Abfrage, Auszahlungs-Mitteilung) und die Auszahlung verbuchen.

**Status quo:** Manuelle Mails / Briefe + Excel-Liste außerhalb von Genossi.

**Ziel:** Innerhalb Genossi modellieren — analog zur Ablösung anderer Excel-Workflows.

## Modellierungs-Entscheidungen

### Anteile am Mitglied

- Anteile sind **homogen** — kein Bedarf, einzelne Anteile als eigene Einheit zu
  führen (keine Anteils-Nummerierung, keine Anteils-Klassen).
- Nur **ganze Anteile** werden erfasst und ausbezahlt — keine Bruchteile.
- → **Neues Feld** `Member.share_count: i32`, **auditiert** (Member ist bereits
  `Auditable`; Änderungen laufen über `audited_update!`).

### Anteilswert

- Wird **pro Geschäftsjahr** festgelegt, **nach der GV des Folgejahres**
  (sobald der Jahresabschluss festgestellt ist).
- Beispiel: GJ 2025 → Wert wird im Frühjahr 2026 nach der GV 2026 berechnet.
- → Wert wird **an der Rückzahlungsphase** hinterlegt (nicht global), weil die Phase
  ohnehin an ein Geschäftsjahr gekoppelt ist.

### Rückzahlungsphase

- Eine `RepaymentPhase` pro Geschäftsjahr.
- Felder: `fiscal_year`, `share_value` (Cent oder Decimal), Status (offen/abgeschlossen).
- Lebenszyklus: angelegt nach GV → Mitglieder befüllt (auto + manuell) → angeschrieben
  → Auszahlungen verbucht → abgeschlossen.

### Auszahlungs-Einträge

- Pro Mitglied in einer Phase: `RepaymentEntry { member_id, share_count_to_pay_out,
  status }`.
- **Auto-Befüllung:** Filter alle Mitglieder mit `austritts_datum` im
  Phasen-Geschäftsjahr (Austritts-Datum existiert schon am Mitglied; der
  "ausgetreten"-Status wird dynamisch aus Datum + Stichtag berechnet).
- **Manuelles Hinzufügen:** Teil-Abtretungen ohne Voll-Austritt sowie verspätet
  gemeldete Austritte.

### Auszahlung markieren

- Markieren eines Eintrags als "ausbezahlt" **triggert automatisch**:
  - `Member.share_count -= entry.share_count_to_pay_out`
  - Auditierte Aktualisierung (über bestehendes `audited_update!`-Pattern)
- Der **Austritts-Status** des Mitglieds bleibt davon unberührt — der ist über
  `austritts_datum` ohnehin bereits gesetzt.

### Anschreiben

- Bestehendes **Template-System** nutzen (IBAN-Abfrage, Auszahlungs-Mitteilung).
- Vermutlich Batch-Anschreiben aus der Auszahlungsgruppe heraus — siehe offene Lücken.

## Offene Detail-Lücken (für CONTEXT/SPEC-Phase)

- **Tracking-Etappen:** Reicht `offen → ausbezahlt`, oder werden Zwischenstufen
  gebraucht (z.B. `angeschrieben → IBAN erhalten → überwiesen`)?
- **Output-Dokument:** Muss am Phasenende ein PDF/CSV für Buchhaltung oder Verband
  generiert werden? Welche Felder?
- **Batch-Anschreiben:** Aus der Auszahlungsgruppe heraus alle offenen Einträge auf
  einmal anschreiben, oder einzeln?
- **Mehrere Entries pro Mitglied pro Phase:** Z.B. Teil-Abtretung im April + späterer
  Voll-Austritt im November — ein Eintrag oder zwei?
- **Migration:** Beim Einführen von `share_count` brauchen alle Bestands-Mitglieder
  einen Wert. Default 1? Oder per Excel-Import seed-en?

## Constraints / Was zu beachten ist

- **Audit-Pflicht:** `Member` ist auditiert; Änderungen an `share_count` müssen den
  vorhandenen Audit-Pfad nutzen, nicht direkt DAO-Calls.
- **Layer-Architektur** einhalten: neue Entitäten brauchen DAO-Trait, SQLite-Impl,
  Service-Trait + Impl, REST-Handler + Utoipa-Schema. Migrations in
  `migrations/sqlite/`.
- **Component-First Frontend:** Auszahlungs-Liste als wiederverwendbare Komponente
  in `genossi-frontend/src/component/`, nicht als inline RSX in der Page.
- **Verbandskonformität:** Diese Funktion ersetzt Excel-Liste — Output und
  Nachvollziehbarkeit müssen Verbandsprüfung standhalten.

## Bezug zum aktuellen Roadmap

Liegt **außerhalb** des aktuellen GV-Milestones. Kandidat für den nächsten Milestone
oder Backlog-Phase. Trigger siehe Seed [[anteile-und-rueckzahlungsphase]].
