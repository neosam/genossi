# Offene Recherche- und Klärungs-Fragen

## Auszahlungsphase (Anteile-Rückzahlung)

Erfasst am 2026-05-29 während `/gsd-explore`. Siehe [[auszahlungsphase-konzept]]
und [[anteile-und-rueckzahlungsphase]].

- [ ] **Tracking-Etappen pro Auszahlungs-Eintrag**: Reicht `offen → ausbezahlt`,
      oder werden Zwischenstufen gebraucht (z.B. `angeschrieben → IBAN erhalten →
      überwiesen`)? Bedarf vermutlich aus tatsächlicher Vorstand-Praxis ableiten.

- [ ] **Output-Dokument am Phasen-Ende**: Muss ein PDF / CSV / Excel für Buchhaltung
      oder Verband generiert werden? Welche Felder verlangt der Genossenschaftsverband
      bei Anteils-Auszahlungen? (Prüfung der Verbandsanforderungen analog zur
      Mitgliederliste.)

- [ ] **Batch-Anschreiben**: Aus der Auszahlungsgruppe heraus alle offenen Einträge
      mit einem Klick anschreiben (Template-System), oder bewusst einzeln? Wie
      tracken, wer schon angeschrieben wurde?

- [ ] **Mehrere Einträge pro Mitglied pro Phase**: Kann ein Mitglied in derselben
      Phase mehrere `RepaymentEntry`-Datensätze haben (z.B. Teil-Abtretung im April
      + Voll-Austritt im November), oder wird das pro Phase aggregiert?

- [ ] **Migration `share_count`**: Beim Einführen des Feldes brauchen alle
      Bestands-Mitglieder einen Wert. Default 1? Per Excel-Import aus der bestehenden
      Liste seeden? Vorstand selbst eintragen lassen?

- [ ] **Stornierung / Korrektur einer Auszahlung**: Was wenn eine Auszahlung
      irrtümlich als "ausbezahlt" markiert wurde und der Member-`share_count` bereits
      reduziert ist? Audit-konformer Rück-Weg?
