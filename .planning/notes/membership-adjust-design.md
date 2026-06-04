---
title: Mitgliedschaft anpassen — Designentscheidungen aus /gsd-explore
date: 2026-06-04
context: Exploration nach v1.1 (Anteile-Rückzahlungsphase) — Vorbereitung für nächsten Milestone
---

# Mitgliedschaft anpassen — Designentscheidungen

## Ausgangslage

v1.1 hat die **Auszahlungsphase** mit Auto-Befüllung beim Öffnen implementiert: Beim
Start eines neuen Geschäftsjahres werden die Kündigungen aus dem Vorjahr automatisch
übernommen. Was fehlt, ist die **laufende Pflege während des Geschäftsjahres**:
Vorstand soll am Mitglied direkt "Kündigung", "Anteile übertragen" oder "Anteile
verkaufen/abtreten" auslösen können, und die Software erzeugt sämtliche Folge-Einträge
automatisch — inklusive korrekter Stichtagsregel.

## Vier Operationen

| Operation                          | Wirksam / Stichtag                                                       | Folge-Datensätze                                                       | Auszahlungsphase           | Austrittsdatum                |
| ---------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------- | -------------------------- | ----------------------------- |
| **Kündigung** (Voll-Rückgabe)      | H1 (1. Halbjahr) → 31.12. aktuelles GJ; H2 → 31.12. **folgendes** GJ     | MemberAction (alle Anteile −)                                          | ja, Eintrag mit Voll-Summe | ja, zum Wirksamkeitsdatum     |
| **Teil-Rückgabe an Genossenschaft** | H1/H2 wie oben                                                           | MemberAction (−n)                                                      | ja, Eintrag mit Teilbetrag | nein, Mitglied bleibt aktiv   |
| **Übertragen an anderes Mitglied** (Teil oder voll) | sofort wirksam                                                            | Zwei verlinkte MemberActions: A (−n) + B (+n)                          | nein                       | nur wenn A=0 Anteile übrig    |
| **Aufstocken**                     | sofort wirksam                                                           | MemberAction (+n)                                                      | nein                       | nein                          |

## Vereinheitlichende Logik

- **H1/H2-Regel gilt immer dann, wenn die Genossenschaft Geld auszahlen muss.**
  Bei Übertrag fließt nichts aus der Kasse → sofort wirksam.
- **Voll-Rückgabe an Genossenschaft = Kündigung.** Das Austrittsdatum am Mitglied
  trägt die "ich bin draußen"-Bedeutung — kein zusätzliches Flag im Modell.
- **Voll-Übertrag an Mitglied = sofortiger Austritt.** Auch hier: Austrittsdatum wird
  automatisch auf das Übertragsdatum gesetzt.

## Datums-Logik

- Maßgeblich ist das **Datum der Willensbekundung des Mitglieds** (z.B. Briefdatum),
  nicht das Erfassungsdatum.
- Frontend zeigt **Datepicker, default `today()`, vom Vorstand überschreibbar**.
- Datepicker **erlaubt nur Daten im aktuell offenen Geschäftsjahr**. Rückwirkende
  Erfassung in bereits abgeschlossene GJs ist explizit **out of scope** dieses
  Features (Vorstand nutzt dafür die bestehende manuelle Aktions-Erfassung).

## UI

- **Single-Button "Mitgliedschaft anpassen"** auf der Member-Detail-Seite.
- **Bewusst nur auf der Detail-Seite, nicht in der Mitgliederliste.** Grund: Aktion
  ist Audit-relevant und kaskadiert — extra Klicks erzwingen Bewusstsein.
- Dialog mit Sub-Choice der vier Operationen. Konkrete Form (4 separate Buttons vs.
  3 mit Sub-Sub-Choice "Reduzieren → Ziel: Genossenschaft oder Mitglied" vs.
  Kündigung-Quickpath) bewusst **offen gelassen für /gsd-discuss-phase**.
- **Vorschau-Bestätigungsdialog empfohlen** vor dem finalen Klick — drei verlinkte
  Audit-Einträge können entstehen, ein Klick kann einen Austritt auslösen.

## Constraints

- **Empfänger beim Übertrag muss aktives Mitglied sein** (existiert im System,
  nicht selbst bereits ausgetreten/gekündigt).
- Wird im UI als Auswahlliste/Suchfeld umgesetzt — kein Freitext.

## Bezug zu bestehenden Komponenten

- **MemberAction (auditiert)** — bestehende Entity, wird von allen vier Operationen
  benutzt. Audit-Macros (`audited_create!`) bleiben verpflichtend.
- **Member.austrittsdatum** — bestehendes Feld, wird in Kündigung/Voll-Übertrag
  automatisch gesetzt.
- **RepaymentPhase / RepaymentEntry** (v1.1) — die neuen Operationen schreiben in die
  passende offene Phase. Wenn keine Phase für das berechnete Ziel-GJ existiert, muss
  das im Plan-Stadium geklärt werden (Auto-Anlegen? Fehler? Vorstand öffnet Phase manuell?).

## Out of Scope (Begründung)

- **Rückwirkend in abgeschlossene GJs** — sehr individuell, Vorstand regelt das mit
  manueller Aktions-Erfassung (existiert bereits).
- **Übertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft** — koppelt
  Application + Member + Anteile + Aktion in einem Schritt; zu komplex für jetzt.
  Siehe [[transfer-to-applicant]].
- **Storno-Knopf für ausgelöste Kündigungen** — über bestehende manuelle Aktions-UI
  (negative Gegenbuchung).

## Offene Fragen für Discuss-Phase

- Welche Sub-Choice-Form im Dialog (4 flat vs. 3 mit Nesting vs. Quickpath)?
- Verhalten, wenn Ziel-Auszahlungsphase noch nicht existiert (z.B. Übertrag im
  Dezember mit Wirksamkeit 31.12. folgendes GJ und Phase fürs folgende GJ ist nicht
  angelegt)?
- Bestätigungsdialog-Layout: textuelle Vorschau, Tabelle, Diff-Style?
- Permissions: nur Vorstand oder auch andere Rollen?
- Mehrstufiger Workflow (Antrag → Genehmigung → Wirksamkeit) oder One-Click?

## Related

- [[membership-adjust-during-fiscal-year]] — Seed (Haupt-Feature für nächsten Milestone)
- [[transfer-to-applicant]] — Seed (abgeleitete Out-of-Scope-Idee)
- [[auszahlungsphase-konzept]] — Vorgänger-Note zur Repayment-Phase
- [[anteile-und-rueckzahlungsphase]] — Vorgänger-Seed
