---
title: Laufende Mitgliedschafts-Anpassungen während des Geschäftsjahres
trigger_condition: Start des nächsten Milestones nach v1.1; sobald /gsd-new-milestone ausgeführt wird
planted_date: 2026-06-04
---

# Laufende Mitgliedschafts-Anpassungen während des Geschäftsjahres

## Kurzbeschreibung

Ergänzt v1.1 (Auszahlungsphase mit Auto-Befüllung beim Jahres-Öffnen) um die
**laufende Pflege während des Geschäftsjahres**: Vorstand löst am Mitglied direkt
Kündigung, Anteils-Übertrag, Teilrückgabe oder Aufstockung aus — Software erzeugt
alle Folge-Datensätze (MemberAction, Austrittsdatum, RepaymentEntry) automatisch,
inklusive korrekter Stichtagsregel (1. Halbjahr → aktuelles GJ, 2. Halbjahr →
folgendes GJ).

## Warum jetzt nicht direkt umsetzen?

v1.1 wurde am 02.06.2026 fertig. Es gibt aktuell keinen offenen Milestone — das
hier ist ein klarer Kandidat für **v1.2** und sollte über `/gsd-new-milestone`
gestartet werden, damit Requirements und ROADMAP konsistent sind.

## Designkern (gekürzt — Details siehe [[membership-adjust-design]])

**UI:** Single-Button "Mitgliedschaft anpassen" auf Member-Detail-Seite (nicht
in der Liste — Audit-relevant). Dialog mit Sub-Choice + Vorschau vor Bestätigung.

**Vier Operationen:**
- **Kündigung** (Voll-Rückgabe an Genossenschaft) → H1/H2-Stichtag, Auszahlungsphase-Eintrag, Austrittsdatum
- **Teil-Rückgabe an Genossenschaft** → H1/H2-Stichtag, Auszahlungsphase-Eintrag, Mitglied bleibt aktiv
- **Übertragen an anderes Mitglied** (Teil oder voll) → sofort wirksam, zwei verlinkte MemberActions, kein Auszahlungseintrag, Austrittsdatum nur bei Voll-Übertrag
- **Aufstocken** → sofort wirksam, einfacher MemberAction-Eintrag

**Vereinheitlichende Regel:** H1/H2-Stichtag greift genau dann, wenn die Genossenschaft
Geld auszahlen muss.

**Datum:** Willensbekundung des Mitglieds, default `today()`, nur offenes GJ erlaubt.

**Constraints:** Empfänger beim Übertrag muss aktives Mitglied sein.

## Bezug zu bestehender Architektur

- Setzt auf **RepaymentPhase / RepaymentEntry** (v1.1) auf — schreibt in die passende
  offene Phase.
- Nutzt bestehende **MemberAction** mit Audit-Macros.
- Erweitert Member-Detail-Frontend (component-first laut CLAUDE.md).
- Audit-Hashchain bleibt durch Verwendung der `audited_*!`-Macros intakt.

## Out of Scope (siehe [[membership-adjust-design]])

- Rückwirkende Erfassung in abgeschlossene GJs
- Übertrag an Antragsteller (separater Seed: [[transfer-to-applicant]])
- Storno-Knopf für ausgelöste Kündigungen

## Offene Fragen (für /gsd-discuss-phase beim nächsten Milestone)

- Sub-Choice-Form im Dialog (4 flat vs. 3 mit Nesting vs. Quickpath)?
- Verhalten, wenn Ziel-Auszahlungsphase noch nicht existiert?
- Permissions und mehrstufiger Workflow?

## Related

- [[membership-adjust-design]] — Designentscheidungen aus der Explore-Session
- [[transfer-to-applicant]] — abgeleitete Out-of-Scope-Idee
- [[anteile-und-rueckzahlungsphase]] — Vorgänger-Seed (v1.1)
