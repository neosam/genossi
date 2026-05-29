---
title: Anteile-Modell + Rückzahlungsphase (Auszahlungen an Genossen)
trigger_condition: Nach Abschluss des aktuellen GV-Milestones; spätestens vor GV 2027 (Anteilswert-Berechnung folgt der GV des Folgejahres)
planted_date: 2026-05-29
---

# Anteile-Modell + Rückzahlungsphase

## Kurzbeschreibung

Ersetzt manuelle Excel-Liste für Anteils-Auszahlungen an Genossen (Voll-Austritt
und Teil-Abtretung). Führt das **Anteile-Modell** in Genossi ein und ergänzt einen
**Rückzahlungsphasen-Workflow** mit Auto-Befüllung aus Vorjahres-Austritten.

## Scope (grob)

**Backend:**
- `Member.share_count: i32` (auditiert) + Migration mit sinnvollem Default/Import
- Neue Entitäten: `RepaymentPhase` (mit `fiscal_year`, `share_value`), `RepaymentEntry`
- DAO + Service + REST analog zu bestehenden Patterns
- Service-Logik: Auto-Befüllung Vorjahres-Austritte + Markieren ausbezahlt →
  `audited_update!` am Member

**Frontend:**
- Komponente "Auszahlungsgruppen-Liste" in `src/component/`
- Page: Phase anlegen / öffnen / abschließen
- Page: Eintrag bearbeiten (Anteile abtreten, ausbezahlt markieren)
- Integration in bestehendes Template-System für IBAN-Anschreiben

**Outputs (zu klären):**
- Auszahlungs-Liste als PDF/CSV für Buchhaltung/Verband (siehe offene Frage)

## Trigger

- **Hauptauslöser:** GV-Milestone fertig und archiviert → nächster Milestone geöffnet
- **Spätester Auslöser:** Vor GV 2027, da Anteilswerte für GJ 2026 dann berechnet
  werden müssen und Excel-Liste sonst wieder gebraucht wird
- **Sekundärer Auslöser:** Wenn bis dahin weitere Excel-getriebene Workflows
  priorisiert werden, kann das später rutschen — solange die Excel-Auszahlungs-Liste
  noch toleriert wird

## Verwandt

- Konzept-Notiz mit allen Detail-Entscheidungen: [[auszahlungsphase-konzept]]
- Aktueller GV-Milestone (muss zuerst fertig)

## Offene Fragen

Siehe `.planning/research/questions.md` — Sektion "Auszahlungsphase".

## Nicht-Ziele (jetzt)

- Steuerliche Berechnung der Auszahlungen (Kapitalertragsteuer etc.) — out of scope
- Anteils-Übertragung von Genosse zu Genosse (statt Rücknahme durch Genossenschaft)
  — bisher nicht angefragt, nur Rücknahme/Auszahlung
- Anteils-Klassen oder einzeln-erfasste Anteile mit Nummerierung — explizit verworfen
