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

## RepaymentLetter (Brief-Kanal für Nicht-Email-Mitglieder)

Erfasst am 2026-06-01 während `/gsd-explore`. Siehe
[[repayment-letter-architecture]] und [[repayment-letter-bulk-versand]].

- [ ] **Bundle-Format des Bulk-Letter-Endpoints**: Liefert
      `POST /api/repayment-phase/{phase_id}/letters/generate` ein **einzelnes
      gebündeltes PDF** (Briefe via `#pagebreak` aneinandergehängt, Vorstand
      schickt das in einem Rutsch zum Drucker), ODER ein **ZIP mit N Einzel-
      PDFs** (jedes Member-Dokument einzeln, das ZIP ist nur
      Transport-Container)?

      Trade-offs:
      - Single PDF: einfacher Druck-Workflow, ein File-Download, ein
        Typst-Compile-Call mit N Sub-Dokumenten — aber `MemberDocument`-Storage
        braucht trotzdem N einzelne File-Saves (sonst hängt der Audit-Anker
        an einem geteilten File, was den `relative_path`-Pfad pro Document
        unklar macht).
      - ZIP: `MemberDocument`s zeigen 1:1 auf ihr eigenes PDF im Storage,
        klare Audit-Spur, aber Vorstand muss N Mal "Drucken" klicken
        (oder das ZIP entpacken + alle Files markieren).
      - Hybrid: Server rendert N einzelne PDFs + speichert sie + erzeugt
        ein zusätzliches gebündeltes Download-PDF (in-memory, nicht
        persistiert). Kostet 1× extra Typst-Compile pro Bulk-Request, aber
        beide Use-Cases werden abgedeckt.

      Empfehlung beim Planning: Hybrid, weil minimaler Cost für
      maximale UX.
