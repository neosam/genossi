---
title: RepaymentLetter-Anschreiben — Architektur-Entscheidungen
date: 2026-06-01
context: Exploration via /gsd-explore — Bulk-Brief-Versand für Nicht-Email-Mitglieder in Auszahlungsphasen
---

# RepaymentLetter-Anschreiben — Architektur-Entscheidungen

## Hintergrund

Ergänzt die Phase-10-Mail-Pipeline um einen **Brief-Kanal**: Mitglieder ohne
erreichbare Email-Adresse müssen in einer `RepaymentPhase` per Post angeschrieben
werden. Der Vorstand wählt die betreffenden `RepaymentEntry`-Datensätze
multi-select an und triggert eine Bulk-PDF-Generierung; pro gewähltem Mitglied
entsteht ein `MemberDocument`-Eintrag mit dem gerendeten Anschreiben als
echtem PDF-File.

Vorbild für Mechanik: [[10-CONTEXT]] (Mail-Worker mit Repayment-Kontext-Aggregation)
und Phase 6 `AttendanceExportServiceImpl` (PDF-Generierung via Typst + `sys.inputs`).
Vorbild für Template-Form: existierende Briefe (`templates/join_confirmation.typ`,
`zahlungsanfrage.typ`, `testbrief.typ`).

## Entscheidungen

### D-LETT-01: Engine = `sys.inputs` mit JSON-Kontext

**Entscheidung:** Das neue Anschreiben-Template `auszahlungs_anschreiben.typ`
(Name finalisiert beim Planning) nutzt **nativ-Typst** mit `sys.inputs.at("...")`
als Daten-Übergabe — kein minijinja-Preprocess.

**Why:** Status quo aller fünf bestehenden Typst-Templates
(`auszahlungsliste.typ`, `join_confirmation.typ`, `teilnehmerliste.typ`,
`testbrief.typ`, `zahlungsanfrage.typ`) ist `sys.inputs` + `json.decode`.
minijinja-Preprocess würde `{{ payout_amount }}`-Syntax in den Typst-Source
mischen und damit typst-lsp / tinymist (Syntax-Highlighting, Live-Preview,
Auto-Completion) brechen. Da das Template Dev-gepflegt ist (siehe D-LETT-02),
bringt minijinja keinen UX-Gewinn — Typst-Logik (`#if`, `#let`) reicht.

**Wie anwenden:** Service baut ein JSON-Objekt (member + repayment-context +
today + ggf. phase-meta) und übergibt es als `sys.inputs`-Map an
`PdfGenerator::render_repayment_letter(...)`. Template liest pro Variable
via `#let member = json.decode(sys.inputs.at("member"))`.

### D-LETT-02: Template-Pflege = Dev-Sache

**Entscheidung:** Template lebt in `templates/defaults/` und ist registriert
in `genossi_service_impl/src/template_storage.rs::DEFAULT_TEMPLATES` —
**nicht** im UI vom Vorstand editierbar (anders als Mail-Templates).

**Why:** Brief-Layout (Adressfenster, Falzmarken, Logo-Position, Vorstands-
Unterschrift) ist Layout-Logik, nicht Text-Logik; Änderungen brauchen Typst-
Kenntnisse. Konsistent mit `join_confirmation.typ` und `testbrief.typ`.

**Wie anwenden:** Neuer `DefaultTemplate`-Eintrag plus `include_bytes!(...)`-Verweis.
Wortlaut-Updates laufen über PR-Workflow.

### D-LETT-03: Trigger = separater Bulk-Endpoint, kein Mail-Worker

**Entscheidung:** Neuer REST-Endpoint, analog zu `POST /api/mail/send-bulk`
nimmt eine Liste von `repayment_entry_id`s (oder Phase + Member-IDs) und
liefert ein PDF. **Kein PDF-Attachment an Mails** (das war in 10-CONTEXT
explizit out-of-scope für v1.1).

**Why:** User-Use-Case ist genau die Komplementär-Menge: Mitglieder, die per
Mail NICHT erreicht werden. Im Mail-Worker ist das fehl am Platz.
Multi-Select-Bulk-Pattern ist vom Vorstand schon vom Mail-Versand vertraut.

**Wie anwenden:** Vorläufige Route — wird beim Planning finalisiert:
`POST /api/repayment-phase/{phase_id}/letters/generate` mit Body
`{ entry_ids: [...] }`; Response = ein gebündeltes PDF
(siehe offene Frage Bundle-Format in [[questions]]).

### D-LETT-04: Persistenz = `MemberDocument` mit neuem `DocumentType::RepaymentLetter`

**Entscheidung:** Pro generiertem Brief entsteht **ein** auditierter
`MemberDocument`-Eintrag (`audited_create!`) mit:
- `document_type = "repayment_letter"` (neue `DocumentType`-Variante; `is_singleton() = false` — mehrere Briefe pro Mitglied möglich, z.B. zwei Phasen)
- `relative_path` = echtes PDF im `document_storage` (anders als Phase-10
  `RepaymentMail`, das `relative_path = ""` setzt, weil dort kein File existiert)
- `template_id` und `mail_recipient_id` = `NULL` (Brief-Pfad nutzt diese
  Felder nicht; sie sind aus Phase 10 für Mail-spezifischen Kontext)
- `description` = z.B. `"Anschreiben Auszahlung GJ {fiscal_year}"`

**Why:** Konsistent mit Phase-10-Mail-MemberDocument-Pattern und mit
existierenden PDF-Documents (`join_confirmation.typ`-Output liegt schon
als File im document_storage). Vorstand kann später am Member sehen,
welche Briefe wann erzeugt wurden.

**Wie anwenden:** `DocumentType`-Enum in `genossi_service/src/member_document.rs:55`
erweitern; Audit-Macro im neuen `RepaymentLetterServiceImpl` aufrufen —
**nicht** im Worker (kein async Worker für Briefe nötig, Endpoint rendert
synchron und gibt PDF zurück).

### D-LETT-05: Aggregation = Shared `RepaymentContextResolver`-Service

**Entscheidung:** Die Repayment-Kontext-Aggregation (Entries mit
`status IN ('Open', 'Contacted')` laden, `share_count` summieren,
`payout_amount = share_count × phase.share_value` als deutschen Euro-String
formatieren, `fiscal_year` aus Phase ziehen) wird in einen neuen
**Service-Helper** ausgelagert — als Trait oder Free-Function in
`genossi_service_impl/src/repayment_context.rs` o.ä.

**Why:** Heute liegt diese Logik **inline im Mail-Worker** (Phase 10,
`genossi_mail/src/worker.rs`). Mit dem Letter-Service kommt ein zweiter
Caller — Code-Duplikation wäre möglich (B-Variante, ~20 LOC), aber die
Aggregations-Regel ist zentrale Domain-Logik (Filter-Set, Euro-Format-
Konvention aus Phase-10 D-04) und sollte einmal gepflegt werden.

**Wie anwenden:** Resolver-Service zuerst für Letter-Service bauen, danach
Mail-Worker per separatem Refactor-Todo darauf migrieren (siehe
[[phase-10-worker-refactor-resolver]]). Reihenfolge minimiert Risiko für
den stabilen Phase-10-Code.

## Bekannte offene Punkte (im Plan zu klären)

- **Bundle-Format** — Single gebündeltes PDF (mit `#pagebreak` zwischen
  Briefen) vs. ZIP mit N Einzel-PDFs? Siehe [[questions]].
- **Anschreiben-Wortlaut** — Standard-Template-Wortlaut für den Brief-Body
  (Anrede, Erklärung der Auszahlung, IBAN-Block, Vorstands-Signatur).
  Vermutlich `letter-simple` aus dem `letter-pro`-Package nutzen wie
  `zahlungsanfrage.typ` und `testbrief.typ`.
- **Status-Toggle auf RepaymentEntry** — Analog zur Mail (Phase 10 D-Out-of-
  Scope): Frontend triggert separat das `Open → Contacted`-Toggle nach
  Bulk-Brief-Generierung; nicht im Server-Cascade.
- **Re-Generierung idempotent?** — Was passiert, wenn der Vorstand "Anschreiben
  erzeugen" zweimal für denselben Member klickt? Zwei MemberDocuments
  (`is_singleton = false` erlaubt das), oder Check + Hinweis? Im Plan klären.
- **Permission-Modell** — Vermutlich Vorstand-only via `check_permission("admin", ...)`,
  analog zu Phase 11 D-11.

## Routing zur Umsetzung

Nicht in den jetzt abgeschlossenen v1.1-Milestone. Geht als forward-looking
Phase in den nächsten Milestone (vermutlich v1.2 oder als angehängte Phase 13):
siehe [[repayment-letter-bulk-versand]].
