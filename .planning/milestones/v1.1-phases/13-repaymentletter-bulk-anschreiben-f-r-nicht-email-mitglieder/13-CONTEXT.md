# Phase 13: RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder - Context

**Gathered:** 2026-06-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 13 ergänzt die v1.1-Auszahlungsphase um einen **Brief-Kanal** als Komplement zur Phase-10-Mail-Pipeline. Vorstand wählt auf der `RepaymentPhase`-Detail-Page (Phase 12 UI-02) `RepaymentEntry`-Datensätze multi-select aus und triggert eine Bulk-PDF-Generierung. Server rendert pro betroffenem Mitglied einen Info-Brief als auditiertes `MemberDocument` (echtes PDF-File im `document_storage`) und liefert zusätzlich ein gebündeltes Druck-PDF als Direct-Download für den einfachen Drucker-Workflow.

Das Schreiben informiert das Mitglied über die anstehende Auszahlung (Höhe, Anteile, hinterlegte IBAN); die tatsächliche Überweisung erfolgt unabhängig durch den Vorstand auf Basis der Phase-11-Auszahlungsliste. Der Brief ist **kein** Bank-Beleg.

**In scope:**
- Neuer Service-Trait `RepaymentLetterService` in `genossi_service/src/repayment_letter.rs`
- Neue Impl `RepaymentLetterServiceImpl` in `genossi_service_impl/src/repayment_letter.rs`:
  - Permission-Funnel: Vorstand-only via `check_permission("admin", ...)` (Phase 11 D-11 Pattern)
  - Status-Gate: Phase MUSS `Offen` ODER `Abgeschlossen` sein, sonst `ServiceError::Conflict("phase_not_active")`
  - Synchroner Render-Pfad (kein async Worker)
- Neuer Shared-Helper `RepaymentContextResolver` in `genossi_service_impl/src/repayment_context.rs` (Trait + Impl) — zentralisiert die Aggregations-Logik aus Phase-10-Worker (Open + Contacted Entries filtern, `share_count` summieren, `payout_amount` als deutscher Euro-String formatieren, `fiscal_year` aus Phase ziehen). Erster Caller: Letter-Service. Phase-10-Mail-Worker bleibt **zunächst unberührt** — der Refactor läuft als separates `/gsd-quick` nach Phase 13 (siehe `<deferred>`).
- `PdfGenerator::render_repayment_letter(...)` analog `render_attendance_list`; nutzt `sys.inputs`-JSON-Kontext (D-LETT-01)
- Bundle-PDF-Pfad: Service rendert N Einzel-PDFs (1 pro Member via Multi-Entry-Aggregation), persistiert sie als N `MemberDocument`-Einträge, und rendert zusätzlich ein gebündeltes Druck-PDF (`#pagebreak` zwischen Briefen) in-memory — wird im REST-Handler direkt als `application/pdf` zurückgegeben, nicht persistiert
- Neuer `DocumentType::RepaymentLetter`-Variante in `genossi_service/src/member_document.rs:48` mit `as_str = "repayment_letter"`, `is_singleton() = false`, `template_path() = None` (Template lebt unter eigenem Slug, nicht über das DocumentType-Mapping erreichbar)
- `audited_create!` pro Brief → `MemberDocument` mit:
  - `document_type = "repayment_letter"`
  - `relative_path` = echter Storage-Pfad zum Einzel-PDF (anders als Phase-10 `RepaymentMail`, das `""` setzt)
  - `template_id = NULL`, `mail_recipient_id = NULL`, `status = NULL` (Brief-Pfad nutzt diese Phase-10-Felder nicht)
  - `description` = `"Anschreiben Auszahlung GJ {fiscal_year}"`
- Neuer REST-Handler `genossi_rest/src/repayment_letter.rs`:
  - Route: `POST /api/repayment-phase/{phase_id}/letters/generate`
  - Body: `{ entry_ids: ["uuid", ...] }`
  - Response: `application/pdf` + `Content-Disposition: attachment; filename="auszahlungs_anschreiben_GJ_{fiscal_year}.pdf"` (Direct-Download; Frontend triggert Browser-Save)
  - Validation: alle `entry_ids` MÜSSEN zur `phase_id` im Pfad gehören, sonst 400 `BadRequest("entry_phase_mismatch")`
- Multi-Entry-Aggregation: Server gruppiert `entry_ids` per `member_id` und rendert **einen** Brief pro Member mit aggregierter `share_count` + `payout_amount` (analog Phase 10 D-04 Mail-Aggregation). Konsistent mit Resolver-Pattern.
- Neues Default-Template `templates/defaults/auszahlungs_anschreiben.typ`, registriert in `genossi_service_impl/src/template_storage.rs::DEFAULT_TEMPLATES` per `include_bytes!(...)`. Vorbild: `templates/zahlungsanfrage.typ` (letter-simple, Falzmarken, Logo, hardcoded Vorstands-Signatur). Brief-Body enthält:
  - Mitgliedsnummer + Vor-/Nachname + Anteils-Aufstellung (`share_count_to_pay_out`)
  - Auszahlungsbetrag als deutscher Euro-String (`X,YZ €`)
  - IBAN-Block mit Typst-`#if`-Switch: wenn vorhanden → "Wir überweisen auf deine hinterlegte IBAN {iban}"; wenn NULL → "Wir haben keine IBAN von dir hinterlegt — bitte teile sie uns mit unter mv@nebenan-unverpackt.de"
  - Hardcoded Vorstands-Signatur-Block ("Herzliche Grüße, Carolin Weidmann, Dina Beier und Simon Goller") — analog `zahlungsanfrage.typ:68`
- Template ist über den existierenden `/templates`-UI-Editor (`genossi-frontend/src/page/templates.rs`, REST `read_template`/`write_template`) **vom Vorstand anpassbar** — der `DEFAULT_TEMPLATES`-Eintrag liefert nur den Initial-Wert (Revision von D-LETT-02 aus den Architektur-Notes)
- DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()`: `RepaymentLetterServiceImpl` + `RepaymentContextResolver` mit DAO-Dependencies (`RepaymentPhaseDao`, `RepaymentEntryDao`, `MemberDao`, `MemberDocumentDao`, `AuditLogDao`, `TransactionDao`, `UuidService`) und `PdfGenerator` (existierend, shared mit Phase 6/11)
- 6+ E2E-Tests in `genossi_bin/tests/`:
  - Happy Path: 3 entry_ids für 2 Member → 2 MemberDocuments + 1 Bundle-PDF Response
  - Multi-Entry-Aggregation: 2 entry_ids für 1 Member → 1 MemberDocument mit aggregierter Summe
  - Permission-Denied (Helfer-Auth, kein Vorstand) → 403
  - Status-Gate: Phase im `Vorbereitung`-Status → 409 `phase_not_active`
  - entry_phase_mismatch: entry_ids gehören zu anderer Phase → 400
  - IBAN-NULL: Member ohne `bank_account` → PDF rendert ohne 4xx, NULL-Hinweisblock sichtbar
  - Audit-Hashchain bleibt valide nach Bulk-Letter-Run (`GET /api/audit/verify`)
  - Idempotenz: zweiter Bulk-Letter-Call für denselben Member erzeugt zweites MemberDocument (`is_singleton = false`)
- Frontend (Phase-12-Komplement, im selben Plan-Set):
  - Auf `RepaymentPhase`-Detail-Page Einträge-Tab: zweiter Bulk-Action-Button neben "Massenmail" → "Anschreiben erzeugen" mit Count-Badge
  - Button-Pattern Phase 12 D-01 zwingend: `r#type: "button"` + `onclick`-Handler, KEIN `<form onsubmit>` (Memory `feedback_dioxus_button_type.md`)
  - Click → POST mit `entry_ids` der aktuellen Multi-Selection → Browser-Download des Bundle-PDFs → Toast "N Briefe erzeugt"
  - Bestehendes Multi-Select-Pattern aus Phase 12 D-11 wiederverwenden
- Grep-Gate analog Phase 12 D-02: `rg 'button\s*\{' genossi-frontend/src/page/repayment_phase_detail.rs` darf KEINEN Treffer ohne `r#type:` haben

**Out of scope (deferred / explizit nicht):**
- **Status-Cascade Open → Contacted nach Brief-Erzeugung** — Backend toucht den Entry-Status NICHT. Vorstand triggert separat den existing Phase-8-Batch-Endpoint ("Als angeschrieben markieren"). Analog zur Phase-10-Mail-Pipeline.
- **PDF-Attachment an Mails** — explizit out-of-scope (war auch in Phase 10 ausgeschlossen). Brief und Mail sind komplementäre Kanäle.
- **Re-Generierungs-Block / Idempotenz-Check** — keine 409-Logik bei wiederholtem Klick auf "Anschreiben erzeugen". Jeder Aufruf erzeugt frische MemberDocuments mit aktuellen Daten (`is_singleton = false`).
- **Phase-10-Mail-Worker auf `RepaymentContextResolver` migrieren** — bleibt pending Todo `.planning/todos/pending/phase-10-worker-refactor-resolver.md`, wird nach Abschluss von Phase 13 als `/gsd-quick` abgearbeitet (siehe `<deferred>`).
- **Persistiertes Bundle-PDF an der RepaymentPhase** — das Bundle-PDF ist transient (Direct-Download), nicht persistiert. Re-Download via Erneut-Erzeugen.
- **SEPA-Verwendungszweck im Brief** — Verwendungszweck steht auf der Phase-11-Auszahlungsliste (PDF für die Bank), nicht im Info-Schreiben ans Mitglied.
- **Vorstandsnamen aus Config-Tabelle ziehen** — Signatur ist hardcoded im Default-Template, Vorstand passt bei Personenwechsel via existing Template-Editor an.
- **SEPA pain.001 XML / CSV-Export** — SEPA-01 deferred zu v2, CSV-Export deferred per Phase-11 D-12.
- **Brief-Status-Tracking pro Member** — kein eigenes Statusfeld "Brief generiert". Audit-Spur über `MemberDocument`-Einträge mit `description = "Anschreiben Auszahlung GJ {fiscal_year}"` reicht.

</domain>

<decisions>
## Implementation Decisions

### Bundle-Format + Endpoint-Vertrag

- **D-13-01:** **Hybrid Bundle-Strategie.** Server rendert N Einzel-PDFs (1 pro Member nach Aggregation) und persistiert sie pro `MemberDocument` im `document_storage`. Zusätzlich rendert der Server ein gebündeltes Druck-PDF in-memory (`#pagebreak` zwischen Briefen), das **nicht** persistiert wird und direkt im HTTP-Response ausgeliefert wird. **Why:** Klare 1:1-Audit-Spur pro Mitglied (MemberDocument zeigt auf eigenes File) + einfacher Druck-Workflow (Vorstand klickt einmal Drucken). Kostet 1× extra Typst-Compile pro Bulk-Request, was bei N=20 vernachlässigbar ist. **How to apply:** REST-Handler ruft `RepaymentLetterServiceImpl::generate(...)`, das die Persistenz + das gebündelte Bytes zurückliefert; Handler setzt `Content-Type: application/pdf` und `Content-Disposition: attachment`.
- **D-13-02:** **Direct-Download in Response, kein Persist-then-Fetch.** `POST /api/repayment-phase/{id}/letters/generate` antwortet sofort mit dem Bundle-PDF als binärem Body. Kein zweites GET zum Download, keine Phase-Document-Relation. **Why:** Konsistent mit Phase-11 PDF-Export-Pattern (`attendance_export.rs:122`, `repayment_export.rs`). **How to apply:** Frontend triggert Browser-Save via `Blob`/`createObjectURL` analog zum existierenden Export-Pattern in `genossi-frontend/src/page/repayment_phase_detail.rs` Export-Tab.
- **D-13-03:** **Selektions-Body = `{ entry_ids: [...] }`.** Endpoint nimmt eine flache Liste von `repayment_entry_id`s. Server validiert, dass alle entries zur `phase_id` im Pfad gehören (sonst 400 `entry_phase_mismatch`). **Why:** Konsistent mit Phase-12-Multi-Select-Pattern (Checkbox pro Row, D-11 Phase 12) — jede Selektion ist ein Entry. **How to apply:** Frontend liest die ausgewählten Entry-IDs aus dem Multi-Select-State und schickt sie 1:1. Resolver gruppiert dann per `member_id`.
- **D-13-04:** **Multi-Entry-Aggregation: 1 Brief pro Member mit Summe.** Wenn die `entry_ids`-Liste mehrere Entries für denselben Member enthält (z.B. Teil-Abtretung + Voll-Austritt in einer Phase), rendert der Server EINEN Brief mit aggregiertem `share_count = SUM(...)` und `payout_amount = share_count × phase.share_value`. **Why:** Analog Phase 10 D-04 (Mail-Worker aggregiert ebenfalls) — Single Source of Truth über den `RepaymentContextResolver`. Vermeidet redundante Briefe ans selbe Mitglied. **How to apply:** Resolver wird mit `(phase_id, member_id)` aufgerufen und liefert den aggregierten Kontext zurück. Letter-Service iteriert über `unique members in entry_ids`, nicht über die Roh-`entry_ids`.

### Template + Brief-Inhalt

- **D-13-05:** **Template = `templates/defaults/auszahlungs_anschreiben.typ`, in `DEFAULT_TEMPLATES` registriert, UI-editierbar.** Initial-Wert lebt unter `templates/defaults/` und wird via `include_bytes!` in `DEFAULT_TEMPLATES` registriert (analog `auszahlungsliste.typ`, `join_confirmation.typ`). Vorstand kann das Template über den existierenden `/templates`-Editor (`genossi-frontend/src/page/templates.rs` + REST `read_template`/`write_template`) anpassen. **Why:** Revision der Architektur-Note D-LETT-02 — der User wies darauf hin, dass der existierende Template-Editor genau für diesen Use-Case existiert. Layout-Änderungen (Falzmarken, Adressfenster) brauchen Typst-Kenntnisse, aber Wortlaut-Updates sind UI-fähig. **How to apply:** Keine neue `MailTemplate`-Variante, keine LetterTemplate-DAO — das existierende `template_storage`-System reicht.
- **D-13-06:** **Brief-Body enthält 4 Bausteine** (im Default-Wortlaut, anpassbar via Editor):
  1. **Reference-Block** (oben, analog `zahlungsanfrage.typ:48`): Mitgliedsnummer, Vor-/Nachname, `share_count_to_pay_out`
  2. **Auszahlungsbetrag-Absatz**: "Du erhältst eine Auszahlung in Höhe von `{payout_amount}` €."
  3. **IBAN-Block** mit Typst-`#if`-Switch:
     - vorhanden: "Wir überweisen auf deine hinterlegte IBAN: `{member.bank_account}`."
     - NULL: "Wir haben keine IBAN von dir hinterlegt — bitte teile sie uns unter mv@nebenan-unverpackt.de mit."
  4. **Vorstands-Signatur-Block** (hardcoded): "Herzliche Grüße, Carolin Weidmann, Dina Beier und Simon Goller"
- **D-13-07:** **KEIN SEPA-Verwendungszweck im Brief.** Das Schreiben ist ein **Info-Brief ans Mitglied**, kein Bank-Beleg. Der SEPA-Verwendungszweck steht auf der Phase-11-Auszahlungsliste (PDF für die Banking-Software). **Why:** User-Decision — das Mitglied erhält die Überweisung mit dem Verwendungszweck auf dem Kontoauszug, der Brief muss ihn nicht duplizieren. Reduziert die Verwirrung "wozu der gleiche String zweimal?".

### Idempotenz + Status-Cascade

- **D-13-08:** **Wiederholte Brief-Erzeugung = beide Aufrufe je ein MemberDocument.** Kein Server-Check, kein 409, kein Confirm-Dialog. Jeder Bulk-Run rendert frisch mit aktuellen Daten. **Why:** Konsistent mit `DocumentType::RepaymentLetter.is_singleton() = false` (D-LETT-04). Legitimer Use-Case: Anteils-Korrektur, dann erneutes Anschreiben mit korrigiertem Betrag. Beide Briefe sind im Member-Dokumente-Tab sichtbar und auditiert. **How to apply:** Keine UNIQUE-Constraint auf `(member_id, repayment_phase_id, document_type)`. Audit-Hashchain protokolliert beide Erzeugungs-Events chronologisch.
- **D-13-09:** **Backend macht KEIN Auto-Status-Toggle Open → Contacted.** Brief-Generierung lässt `RepaymentEntry.status` unverändert. Vorstand triggert separat den existing Phase-8-Batch-Endpoint ("Als angeschrieben markieren") nach erfolgreicher Brief-Erzeugung. **Why:** Symmetrie zur Phase-10-Mail-Pipeline (siehe Phase 10 D-Out-of-Scope: Mail-Worker macht auch keinen Auto-Toggle). Falls der Druck-Workflow später fehlschlägt, wäre ein Auto-Status-Wechsel faktisch falsch. Klare Audit-Trennung: "PDF erzeugt" ≠ "angeschrieben". **How to apply:** Frontend zeigt nach Bulk-Letter-Erfolg einen Toast-Hinweis "N Briefe erzeugt — vergiss nicht, die Einträge auf 'Angeschrieben' zu setzen" (Plan-Discretion).

### Resolver + Refactor-Scope

- **D-13-10:** **`RepaymentContextResolver` wird in Phase 13 gebaut; Phase-10-Worker-Refactor bleibt separat.** Letter-Service ist erster Caller des Resolvers. Phase-10-Mail-Worker behält seine Inline-Aggregation **vorerst unverändert**. Nach Abschluss von Phase 13 ist der Resolver stabil und getestet — dann wird der Worker via `/gsd-quick` migriert. **Why:** Minimiert Risiko am produktiv-stabilen Phase-10-Code. Kleinere PR, klare Verantwortung pro Phase. Konsistent mit Architektur-Note D-LETT-05. **How to apply:** Resolver-Trait-Signatur muss so designed werden, dass beide Caller (Letter + Worker) dieselben Eingaben (`phase_id`, `member_id`) und Ausgaben (`share_count`, `payout_amount_string`, `fiscal_year`) verwenden — Stabilität für den späteren Worker-Refactor.
- **D-13-11:** **Pending-Todo `phase-10-worker-refactor-resolver.md` als referenzierter Folge-Quick.** Das Todo bleibt in `.planning/todos/pending/` und wird in `<deferred>` explizit als nächster Quick-Task nach Phase 13 markiert.

### Claude's Discretion

- **Resolver-API-Design:** Trait-vs-Free-Function ist Planner-Discretion. Empfehlung: Trait `RepaymentContextResolver` mit Methode `resolve(phase_id, member_id) -> Result<RepaymentContext, ServiceError>` + struct `RepaymentContext { share_count: i32, payout_amount: String, fiscal_year: i32 }`. Mockable für Unit-Tests (analog UuidService-Mock-Pattern).
- **Euro-Format-Konvention:** Wiederverwenden was Phase 10 D-04 etabliert hat — deutsche Lokalisierung `"X,YZ"`, KEIN Tausenderpunkt, KEIN Euro-Symbol (Template rendert `{{ payout_amount }} €`).
- **Bundle-PDF-Filename-Konvention:** `auszahlungs_anschreiben_GJ_{fiscal_year}.pdf` — Planner-Discretion ob `phase_id` oder Datum mitkodieren.
- **MemberDocument-Filename-Konvention:** `auszahlungs_anschreiben_{member_number}_GJ_{fiscal_year}.pdf` pro Einzel-PDF im Storage — Planner-Discretion (analog `join_confirmation`-Pattern).
- **Order der `audited_create!`-Calls vs. Bundle-Render:** Empfehlung — erst alle N MemberDocuments innerhalb einer Transaction persistieren, dann Bundle-PDF in-memory rendern und committen. Bei Render-Fehler eines einzelnen Briefs → Transaction-Rollback, kein partieller Audit-Trail. Planner darf alternativen Pfad wählen, wenn Performance leidet (Transaction-per-Letter), aber dann All-or-Nothing-Semantik im Bundle bewahren.
- **Frontend-Toast-Wortlaut nach Erfolg:** Planner darf finalisieren. Empfehlung: "N Briefe erzeugt. Vergiss nicht, die Einträge anschließend als angeschrieben zu markieren." (verweist auf D-13-09).
- **Multi-Entry-Aggregation und Brief-Display:** Wenn ein Member 2 Entries (z.B. share_count_to_pay_out = 3 + 2) hat, zeigt der Brief `share_count = 5` aggregiert oder die Aufteilung `3+2`? Empfehlung: aggregiert (= Summe), keine Aufteilung im Template — entspricht Phase 10 D-04. Planner darf einen `share_count_breakdown`-Helper im Template einführen, falls Vorstand das später wünscht.
- **OpenAPI-Doku:** Utoipa-Schema für `POST .../letters/generate` mit Status-Codes 200 (PDF binary), 400 (entry_phase_mismatch / unknown entry_ids), 401 (no auth), 403 (helper auth), 404 (phase nicht gefunden), 409 (phase_not_active). Body-Type = `application/octet-stream` oder explizit `application/pdf`.

### Reviewed Todos (nicht gefoldet)

- **`phase-10-worker-refactor-resolver.md`** — als Folge-Quick nach Phase 13 referenziert, **nicht** in Phase-13-Scope eingefoldet (siehe D-13-10/D-13-11). Voraussetzung für das Todo (stabiler Resolver) entsteht in Phase 13.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architektur & Scope
- `.planning/notes/repayment-letter-architecture.md` — 5 Architektur-Entscheidungen (D-LETT-01..05), Hintergrund, Vorbild-Templates, offene Punkte. D-LETT-02 ist hier zu D-13-05 revidiert (UI-editierbar via existing Template-Editor).
- `.planning/seeds/repayment-letter-bulk-versand.md` — Seed mit grobem Scope, Aufwandsschätzung (5–7 Plans), Routing.
- `.planning/research/questions.md` — Bundle-Format-Trade-offs (Hybrid-Empfehlung), per D-13-01 entschieden.

### Vorbild-Phasen (Pattern-Quelle)
- `.planning/phases/10-massenmail-anbindung-template-variablen/10-CONTEXT.md` — Mail-Pipeline-Pattern, Aggregations-Filter D-04, MemberDocument-Persistenz-Pattern D-07..D-11, Status-Cascade-Out-of-Scope.
- `.planning/phases/11-export-pdf-csv/11-CONTEXT.md` — PDF-Export-Pattern (Permission-Funnel, Status-Gate, Typst-Render, DEFAULT_TEMPLATES-Registrierung, REST-Direct-Download-Response), Verwendungszweck-Schema D-04 (gehört auf Auszahlungsliste, nicht in Brief — siehe D-13-07).
- `.planning/phases/12-frontend-component-first/12-CONTEXT.md` — Frontend Button-Pattern D-01/D-02 (`r#type: "button"` + Grep-Gate, MANDATORY), Multi-Select-Pattern D-11, Massenmail-Action-Pattern D-18.
- `.planning/phases/06-uat-final-anwesenheits-app-und-export/` — `AttendanceExportServiceImpl` als Service-Layer-Vorlage (Permission-Funnel + Typst-Render).

### Projektkontext
- `.planning/PROJECT.md` — v1.1-Milestone-Status, Constraints, Brief-Anschreiben-Out-of-v1.1-Scope BRIEF-01.
- `.planning/REQUIREMENTS.md` §Brief-Anschreiben-Automatik — BRIEF-01 als deferred dokumentiert; Phase 13 hebt diesen Defer auf.

### Code-Referenzen (Files, die berührt werden)
- `genossi_service/src/member_document.rs:48-101` — `DocumentType`-Enum: neue `RepaymentLetter`-Variante.
- `genossi_service_impl/src/template_storage.rs:10` — `DEFAULT_TEMPLATES`-Liste: neuer Eintrag für `auszahlungs_anschreiben.typ`.
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!` Macro für `MemberDocument`-Erzeugung.
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — DI-Wiring für `RepaymentLetterServiceImpl` + `RepaymentContextResolver`.
- `templates/zahlungsanfrage.typ` — Layout-Vorbild (letter-simple, Falzmarken, Logo, Signatur-Block).
- `templates/join_confirmation.typ`, `templates/testbrief.typ` — weitere Brief-Vorbilder.
- `templates/defaults/auszahlungsliste.typ` — Vorbild für `sys.inputs`-JSON-Kontext-Pattern (Phase 11).
- `genossi_mail/src/worker.rs` — Phase-10-Inline-Aggregation, **nicht touched** in Phase 13 (siehe Todo).
- `genossi-frontend/src/page/templates.rs` — Existing Template-Editor-UI (Read/Write von Typst-Files), nutzt der Vorstand zum Anpassen des Brief-Wortlauts.
- `genossi-frontend/src/page/repayment_phase_detail.rs` — Detail-Page mit Einträge-Tab, neuer Bulk-Button "Anschreiben erzeugen".

### Pending Todos
- `.planning/todos/pending/phase-10-worker-refactor-resolver.md` — Folge-Quick nach Phase 13 (siehe D-13-10/D-13-11).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`PdfGenerator`** (geteilt mit Phase 6 + 11): Typst-Render-Engine, `sys.inputs`-JSON-Kontext-Pattern. Neue Methode `render_repayment_letter(...)` analog `render_attendance_list(...)`.
- **`audited_create!`-Macro** (`genossi_service_impl/src/audit_macros.rs`): atomare Persistenz + Hash-Chain-Audit-Eintrag pro Brief. Voraussetzung: `RepaymentLetterServiceImpl` hat `audit_log_dao` und `uuid_service` als Felder.
- **`check_permission("admin", ...)`**-Pattern (Phase 11 D-11): Vorstand-Auth-Funnel via `ClaimContext`.
- **`DEFAULT_TEMPLATES`-Mechanik** (`template_storage.rs:10`): Initial-Werte via `include_bytes!` registrieren; Existing Template-Editor erlaubt UI-Edits.
- **Existing Template-Editor** (`genossi-frontend/src/page/templates.rs`): Vorstand passt Typst-Templates über bestehende UI an — keine Mail-Template-Erweiterung nötig.
- **Multi-Select-Pattern** (`RepaymentEntryList` aus Phase 12 D-11): Checkbox pro Row + Header-Action-Leiste mit Count-Badge. Wiederverwenden für "Anschreiben erzeugen"-Button neben "Massenmail".
- **Direct-Download-Pattern** aus Phase 11 (`repayment_export.rs`): `Response::builder().header(CONTENT_TYPE, "application/pdf").header(CONTENT_DISPOSITION, ...)`. 1:1 wiederverwendbar.
- **`MEMBERS`-Global-Signal** im Frontend: Member-Daten für Client-Side-Join verfügbar (Phase 12 D-10).

### Established Patterns
- **Layered DAO/Service/REST mit Trait-Boundaries**: `RepaymentLetterService` als Trait in `genossi_service/`, Impl in `genossi_service_impl/`, REST-Handler in `genossi_rest/`. DI-Wiring im Binary-Layer (`gen_service_impl!`-Macro).
- **Audit-First für MemberDocument**: jeder neue Brief geht durch `audited_create!`, niemals direkter DAO-Call. Auditable-Trait-Impl auf `MemberDocument` existiert bereits (Phase 10 D-08 inkl. `template_id`/`mail_recipient_id`/`status`).
- **Soft-Delete-Konvention**: `entry.deleted IS NULL AND member.deleted IS NULL` immer als Filter (siehe Phase 11 D-02, Genossi-übergreifend).
- **Component-First-Frontend**: KEIN inline-RSX-Duplikat. "Anschreiben erzeugen"-Button reuses existing Bulk-Action-Pattern; neue Komponente nur falls echter Mehrwert.
- **Button-Pattern Phase 12 D-01/D-02**: `r#type: "button"` + `onclick`, KEIN `<form onsubmit>` — Grep-Gate als Pre-Merge-Check.
- **Resolver-Pattern für DRY-Aggregation**: zentrale Domain-Logik (Filter + Format) in einem Service, nicht in den Callers dupliziert.

### Integration Points
- **REST-Mount**: neuer Router montiert auf `/api/repayment-phase/{phase_id}/letters/...` in `genossi_rest/src/lib.rs::create_app`.
- **MemberDocument-Schema**: keine neue Migration nötig — die Phase-10-Felder `template_id`/`mail_recipient_id`/`status` bleiben NULL für RepaymentLetter; `document_type` und `relative_path` werden gefüllt.
- **Audit-Hashchain**: jede `audited_create!`-Call verlängert die Chain — `GET /api/audit/verify` muss nach Bulk-Letter-Run grün bleiben (E2E-Test SC).
- **Template-Storage-Filesystem**: neues File unter dem `TEMPLATE_PATH`-Root, identische Behandlung wie alle anderen Typst-Templates.
- **Frontend-Wiring**: API-Client-Methode `generate_repayment_letters(phase_id, entry_ids) -> Result<Vec<u8>>` in `genossi-frontend/src/api/mod.rs`, Browser-Save analog Phase-11-Export-Pattern.

</code_context>

<specifics>
## Specific Ideas

- **Layout-Vorbild für Default-Template:** `templates/zahlungsanfrage.typ` (letter-pro/letter-simple, Falzmarken, Logo top-left, Reference-Block-Tabelle, hardcoded Vorstands-Signatur). User-Hinweis bestätigt im Discuss-Schritt.
- **Vorstandsnamen im Default-Template:** "Carolin Weidmann, Dina Beier und Simon Goller" — exakt wie in `zahlungsanfrage.typ:68`. Anpassbar via existing UI-Template-Editor bei Personenwechsel, keine Config-Tabelle nötig.
- **IBAN-Hinweis-Mail-Adresse bei NULL:** `mv@nebenan-unverpackt.de` — exakt wie in `zahlungsanfrage.typ:23`-Footer.
- **Brief ist Info-Schreiben, kein Bank-Beleg:** User klarstellung — Vorstand überweist auf Basis der Phase-11-Auszahlungsliste; das Mitglied bekommt den Brief als Ankündigung mit Hinweis auf die hinterlegte IBAN.
- **Re-Generierung erlaubt:** User-Decision — Vorstand soll wiederholt Briefe erzeugen können (z.B. nach Anteils-Korrektur), beide bleiben als auditierter MemberDocument-Eintrag erhalten.

</specifics>

<deferred>
## Deferred Ideas

- **Status-Cascade Auto-Toggle (Backend-seitig)** — würde nach Brief-Erzeugung automatisch `Open → Contacted` setzen; bewusst out-of-scope für Symmetrie mit Phase-10-Mail-Pipeline. Vorstand toggelt selbst über existing Phase-8-Batch-Endpoint.
- **PDF-Attachment an Mails** — Brief und Mail bleiben komplementäre Kanäle; PDFs werden nicht in Mails eingebettet.
- **Persistiertes Bundle-PDF pro Phase** — das geteilte Druck-PDF ist transient (Direct-Download); Re-Download via erneutes "Anschreiben erzeugen". Eine künftige Phase könnte ein Phase-Document einführen, falls der Vorstand das Bundle archivieren möchte.
- **Vorstandsnamen aus Config-Tabelle** — heute hardcoded im Template. Eine künftige Phase könnte eine Vorstands-Member-Tabelle einführen, wenn der Personalwechsel häufig wird.
- **Brief-Status-Tracking pro Member** ("Brief generiert ja/nein"-Indikator in der Entry-Tabelle) — Audit-Spur über `MemberDocument` reicht aktuell; bei wachsender Komplexität ggf. Phase 14+.
- **SEPA pain.001 XML-Export** — explizit deferred zu v2 (SEPA-01).
- **CSV-Export der Auszahlungsliste** — deferred per Phase-11 D-12.

### Reviewed Todos (not folded)

- **`.planning/todos/pending/phase-10-worker-refactor-resolver.md`** — Phase-10-Mail-Worker auf den neuen `RepaymentContextResolver` migrieren. **Reason für Deferral:** Phase 13 baut den Resolver erst (Letter-Service ist erster Caller); Worker bleibt zunächst unberührt, um Risiko am stabilen Phase-10-Code zu minimieren (D-13-10). **Routing:** nach Phase 13 als `/gsd-quick` abarbeiten, sobald der Resolver-Service stabil und getestet ist.

</deferred>

---

*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Context gathered: 2026-06-01*
