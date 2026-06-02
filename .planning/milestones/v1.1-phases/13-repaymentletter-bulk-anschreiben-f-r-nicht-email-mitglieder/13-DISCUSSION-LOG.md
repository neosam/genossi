# Phase 13: RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-01
**Phase:** 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
**Areas discussed:** Bundle-Format, Anschreiben-Wortlaut, Idempotenz & Status-Cascade, Resolver + Worker-Refactor-Scope

---

## Bundle-Format

### Bundle-Strategie (D-13-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid | Server rendert N Einzel-PDFs + speichert pro MemberDocument + zusätzlich gebündeltes Druck-PDF in-memory. Klare 1:1-Audit-Spur + einfacher Druck-Workflow. 1× extra Typst-Compile. | ✓ |
| Single PDF | Ein einzelnes PDF via #pagebreak. MemberDocument-Audit-Anker zeigt auf shared file → unklar. | |
| ZIP mit N Einzel-PDFs | 1:1-Storage-Mapping, aber Vorstand muss N Mal Drucken klicken oder ZIP entpacken. | |

**Notes:** Empfehlung aus `.planning/research/questions.md` direkt übernommen — Hybrid liefert beide Use-Cases (klare Audit-Spur + komfortabler Druck-Workflow) bei vernachlässigbaren Mehrkosten.

### Delivery (D-13-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Direct Download in Response | POST liefert direkt application/pdf + Content-Disposition. N MemberDocuments im selben Request persistiert. | ✓ |
| Persistiertes Phase-Document + JSON-Response | Bundle auch persistiert, JSON-Response mit Download-URL. Re-Download möglich, aber Doppel-Storage. | |
| JSON mit MemberDocument-IDs + Frontend-Bundling | Frontend bündelt clientseitig. Komplex im Frontend. | |

**Notes:** Konsistent mit Phase-11-PDF-Export-Pattern (`attendance_export.rs:122`).

### Selektions-Schnittstelle (D-13-03)

| Option | Description | Selected |
|--------|-------------|----------|
| Liste von repayment_entry_ids | Body `{ entry_ids: [...] }`. Konsistent mit Multi-Select-UI (Phase 12 D-11). Server validiert phase-Zugehörigkeit. | ✓ |
| Liste von member_ids | Body `{ member_ids: [...] }`. Server löst Entries auf. | |
| Filter-Predicate | Body `{ status: "open" }`. Schlechte UI-Kontrolle. | |

### Multi-Entry-Aggregation (D-13-04)

| Option | Description | Selected |
|--------|-------------|----------|
| 1 Brief pro Member mit Summe | Server gruppiert entry_ids per member_id, rendert pro Member einen Brief mit aggregiertem share_count + payout_amount. Analog Phase 10 D-04. | ✓ |
| Pro Entry ein eigener Brief | N entries → N Briefe → N MemberDocuments. Redundant. | |
| Fehler/400 bei Multi-Entry-Selektion | Endpoint lehnt ab, Vorstand muss vorher aufräumen. Zu strikt. | |

---

## Anschreiben-Wortlaut

### Pflicht-Felder (Initial-Frage)

| Option | Description | Selected |
|--------|-------------|----------|
| Member-Stamm: Name + Anschrift | letter-simple-Adressfenster | ✓ |
| Mitgliedsnummer + Anteils-Aufstellung | Reference-Block, nachvollziehbare Berechnung | ✓ |
| Auszahlungsbetrag + Verwendungszweck | payout_amount als deutscher Euro-String + SEPA-Verwendungszweck | ⚠ (nur Betrag; Verwendungszweck verworfen) |
| IBAN-Hinweis + Rückfrage-Aufforderung | IBAN bei NULL → expliziter Mail-Kontakt-Hinweis | ✓ |

**User's choice + Notes:** Custom-Antwort des Users: "Aber was bedeutet Pflichtfelder? Mach einfach ein Template und gib dem Benutzer die Möglichkeit das Template anzupassen." → Pivot zur Template-Pflege-Frage. Außerdem klar: "Wie kommst du auf Verwendungszweck? Das ist das Infoschreiben, das wir an das Mitglied schreiben. Und wir überweisen ja dann." → Verwendungszweck ist Bank-Beleg-Sache (Phase 11), nicht Info-Brief.

### Template-Pflege (D-13-05, Revision von D-LETT-02)

| Option | Description | Selected |
|--------|-------------|----------|
| User-editierbar wie Mail-Templates | Eigene Template-Variante mit UI-Editor | |
| Hybrid: Layout fix, Körper editierbar | Plain-Text-Body mit Var-Buttons | |
| Dev-only (ursprünglich D-LETT-02) | Customization via PR | |

**User's choice:** Custom — "Das soll einfach bei den typst Dokumentvorlagen liegen wie alles andere auch. Da ist ja extra ein Editor verbaut in dem Tool mit dem man diese Templates anpassen kann."

**Notes:** User wies auf existing `/templates`-Editor (`genossi-frontend/src/page/templates.rs` + REST `read_template`/`write_template`) hin. Verifiziert per Code-Search. D-LETT-02 wird durch D-13-05 ersetzt: Default in `DEFAULT_TEMPLATES` registriert (Initial-Wert), laufende Anpassungen via existierender UI.

### Default-Body-Bausteine (D-13-06)

| Option | Description | Selected |
|--------|-------------|----------|
| Mitgliedsnummer + Anteils-Aufstellung | Reference-Block oben, analog zahlungsanfrage.typ:48 | ✓ |
| Auszahlungsbetrag | payout_amount in deutscher Euro-Notation | ✓ |
| Verwendungszweck | SEPA-Schema aus Phase 11 D-04 | ✗ (gestrichen) |
| Vorstands-Signatur-Block | Hardcoded analog zahlungsanfrage.typ:68 | (impliziert via D-13-07-Folgefrage) |

**User's choice + Notes:** "Ja, schon Auszahlungsbetrag. Aber wie kommst du auf Verwendungszweck? Das ist das Infoschreiben, das wir an das Mitglied schreiben. Und wir überweisen ja dann." → Verwendungszweck als D-13-07 explizit out-of-scope dokumentiert.

### IBAN-Behandlung (D-13-06 Detail)

| Option | Description | Selected |
|--------|-------------|----------|
| IBAN ausweisen + bei NULL Hinweis | Vorhandene IBAN abdrucken; NULL → Mail-Aufforderung an mv@... | ✓ |
| IBAN immer ausweisen (auch bei NULL als 'fehlt') | Konsistentes Layout, aber 'fehlt'-Text wirkt nicht-info | |
| Keine IBAN im Brief | Mitglied prüft selbst bei der Bank | |

### Vorstands-Signatur (D-13-06 Detail)

| Option | Description | Selected |
|--------|-------------|----------|
| Hardcoded im Default-Template | "Herzliche Grüße, Carolin Weidmann, Dina Beier und Simon Goller" wie zahlungsanfrage.typ:68 | ✓ |
| Aus existierender Konfig-Tabelle | Auto-Resolve aus DB | |

**Notes:** Keine Vorstands-Tabelle existiert; Personenwechsel via existing Template-Editor.

---

## Idempotenz & Status-Cascade

### Wiederholte Brief-Erzeugung (D-13-08)

| Option | Description | Selected |
|--------|-------------|----------|
| Beide Aufrufe erzeugen je ein MemberDocument | Konsistent mit is_singleton=false; legitimer Use-Case "Anteils-Korrektur dann erneut anschreiben". | ✓ |
| Check + Confirm-Dialog im Frontend | Server liefert 409 bei existierendem Letter, Frontend zeigt Confirm. Extra Round-Trip. | |
| Hard-Block: 409 ohne Override | Vorstand muss erst altes Document löschen. Zu strikt. | |

### Status-Cascade nach Brief-Erzeugung (D-13-09)

| Option | Description | Selected |
|--------|-------------|----------|
| Nein — Status-Toggle bleibt separat | Backend toucht Entry-Status nicht; Vorstand triggert Phase-8-Batch separat. Symmetrie zur Mail. | ✓ |
| Ja — Auto-Toggle Open → Contacted | Bequemer, aber asymmetrisch und falsch bei Druck-Fehler. | |
| Frontend-Auto-Toggle nach Download | Komfort wie B, aber Server-Vertrag clean. | |

---

## Resolver + Worker-Refactor-Scope

### Refactor-Scope (D-13-10)

| Option | Description | Selected |
|--------|-------------|----------|
| Separat — Phase 13 baut Resolver, Refactor folgt als /gsd-quick | Minimiert Risiko am Phase-10-Code; kleinere PR. | ✓ |
| Einfolden — Resolver + Letter + Worker-Refactor in einer Phase | Atomar, aber größerer Blast-Radius. | |
| Worker-Refactor ohne Resolver (B-Variante) | Code-Duplikation; Aggregation könnte divergieren. | |

### Todo-Routing (D-13-11)

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — als reviewed in CONTEXT.md.deferred mit Folge-Quick-Verweis | Todo bleibt in pending/, CONTEXT.md verweist explizit. | ✓ |
| Nein — als 14. Plan in Phase 13 eingefoldet | Widerspruch zu Option A der vorigen Frage. | |

---

## Claude's Discretion

- Resolver-Trait-Signatur (Mockable für Unit-Tests)
- Euro-Format-Wiederverwendung aus Phase 10 D-04
- Bundle-PDF-Filename-Konvention (`auszahlungs_anschreiben_GJ_{fiscal_year}.pdf`)
- MemberDocument-Filename-Konvention pro Einzel-PDF
- Transaction-Granularität bei Bundle-Render (All-or-Nothing vs. per-Letter)
- Toast-Wortlaut nach Erfolg (verweist auf D-13-09 Manual-Status-Toggle)
- OpenAPI/Utoipa-Schema-Detail-Doku

---

## Deferred Ideas

- Status-Cascade Auto-Toggle (Backend)
- PDF-Attachment an Mails (komplementäre Kanäle bleiben getrennt)
- Persistiertes Bundle-PDF pro Phase
- Vorstandsnamen aus Config-Tabelle
- Brief-Status-Indikator in Entry-Tabelle
- SEPA pain.001 XML-Export (v2)
- CSV-Export der Auszahlungsliste (Phase 11 D-12 Defer)

### Reviewed Todos (not folded)

- `.planning/todos/pending/phase-10-worker-refactor-resolver.md` — Folge-Quick nach Phase 13 (siehe D-13-10/D-13-11).
