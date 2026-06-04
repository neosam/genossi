---
title: Anteilsübertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft
trigger_condition: Wenn Schenkungs-Aufnahmen häufiger werden ODER nach Abschluss von [[membership-adjust-during-fiscal-year]]
planted_date: 2026-06-04
---

# Anteilsübertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft

## Kurzbeschreibung

Sonderfall des Anteils-Übertrags aus [[membership-adjust-during-fiscal-year]]: Ein
bestehendes Mitglied überträgt Anteile nicht an ein anderes **aktives Mitglied**,
sondern an einen Mitgliedsantragsteller (Application), der seinen Beitrag bisher
nicht bezahlt hat. Durch die Schenkung des Anteils wird der Antragsteller
**automatisch zum Vollmitglied** — Aufnahme + Anteilszuschreibung in einem Schritt.

## Warum als separater Seed (nicht im Haupt-Feature)?

Diese Variante koppelt drei Systeme in einem atomaren Vorgang:

1. **Application** (Antrag) → Status wechselt auf "angenommen"
2. **Member** (neue Mitgliedschaft) → wird erzeugt mit Anteilszuschreibung
3. **MemberAction** beim schenkenden Mitglied → −n Anteile, verlinkt mit neuer
   Member-Aktion (+n Anteile)
4. **Audit-Hashchain** für alle drei Vorgänge konsistent halten

Das erhöht die Komplexität spürbar und ist in der Praxis selten — der Hauptpfad
(Übertrag zwischen zwei bestehenden Mitgliedern) deckt 99% der Fälle ab.

## Trigger zum Aufgreifen

- Schenkungs-Aufnahmen kommen real häufiger vor (z.B. >2× pro Jahr)
- Vorstand fragt aktiv nach dieser Funktion
- Verband stellt das als verbandskonforme Anforderung

## Lösungsskizze (grob)

- Erweiterung des "Anteile übertragen"-Dialogs um Empfänger-Typ-Wahl: bestehendes
  Mitglied **oder** offener Mitgliedsantrag (Suchfeld zeigt beides)
- Bei Wahl "Antrag" → zusätzlicher Bestätigungs-Schritt "Antragsteller wird durch
  diesen Vorgang Vollmitglied" mit Anzeige aller drei kaskadierenden Effekte
- Backend: neue Service-Methode `transfer_to_applicant(from_member, application,
  amount)` die atomar Application+Member+MemberAction handhabt

## Risiken / offene Punkte

- **Satzungskonformität:** Ist Anteilsschenkung als Aufnahmemechanismus überhaupt
  zulässig? Verband fragen.
- **Audit-Konsistenz:** Drei Audit-Vorgänge in einer Transaktion — Hashchain-Logik
  muss das tragen
- **Rückgängig-Pfad:** Falls Vorstand sich verklickt, ist der Antragsteller bereits
  Vollmitglied. Wie wird das rückabgewickelt?

## Related

- [[membership-adjust-during-fiscal-year]] — Haupt-Feature, aus dem dieser Sonderfall
  abgeleitet ist
- [[membership-adjust-design]] — Designentscheidungen, in denen diese Variante
  explizit als out-of-scope markiert wurde
