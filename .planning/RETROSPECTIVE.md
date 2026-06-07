# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — GV-Anwesenheits-Erfassung

**Shipped:** 2026-05-29
**Phases:** 5 active + 1 SKIPPED | **Plans:** 34 | **Timeline:** 2026-05-02 → 2026-05-17 (≈15 Tage)

### What Was Built

- **Assembly-Lifecycle (Phase 1):** GV als eigene Entität mit Preparation→Open→Closed-Lifecycle, atomarem Member-Universe-Snapshot beim Öffnen, Audit-Hashchain-Integration
- **Helfer-Token-System (Phase 2):** Race-safe One-Time-Use-Tokens via `UPDATE ... WHERE used_at IS NULL RETURNING`; Crockford-Codes + SHA256-Hash + QR-SVG; typsichere `AuthContext::Helper`-Variante
- **Attendance-Aggregat (Phase 3):** DSGVO-konforme Helfer-Mitgliederliste (4 Felder strenge Whitelist), idempotenter UPSERT-Toggle, Live-Counter X von Y, kaskadierende Session-Invalidation bei GV-Schluss
- **Frontend Component-First (Phase 4):** Dioxus-WASM mit 4 shared Components (AttendanceList/Search/LiveCounter/ConnectionBanner) zwischen Helfer- und Vorstand-View; QR-Scanner + Manual-Code-Fallback (HLPR-03); Polling-Sync (SYNC-01) ohne SSE/WebSocket
- **Teilnehmerlisten-Export (Phase 6):** Drei Formate parallel (PDF via Typst, CSV mit UTF-8-BOM und Semikolon, XLSX via rust_xlsxwriter); read-only ohne neue DAO-Methode; Vorstand-only mit Closed-Gate
- **Produktiver Einsatz:** Echte Generalversammlung im Mai 2026 mit Genossi durchgeführt — stärkster möglicher E2E-Beweis

### What Worked

- **Audit-Pattern wiederverwendet:** Phase-1-Audit-Integration (Assembly-Lifecycle) konnte 1:1 das bestehende `audited_create!`/`audited_update!`-Pattern nutzen — Null-Aufwand-Integration in die existierende Hash-Chain
- **Backend-First-Reihenfolge:** Phasen 1→2→3 (Backend) vor Phase 4 (Frontend) hat sich bezahlt gemacht — `genossi-frontend/src/api.rs` konnte gegen fertige REST-Types entwickelt werden, kein „Frontend wartet auf API"-Block
- **Atomic-Redeem-Pattern via RETURNING-Clause:** SQLite-spezifisches `UPDATE ... WHERE ... RETURNING` löste die Race-Condition zwischen parallelen Token-Scans elegant ohne Locking auf Application-Ebene
- **Component-First (ATTN-06):** Shared Components zwischen Helfer- und Vorstand-Pages reduzierten die Frontend-Komplexität dramatisch und machten den Vorstand-Notfall-Fallback (Vorstand kann ohne QR aushelfen) zum Component-Reuse-Pattern statt zu einer Code-Duplikation
- **Phase 5 SKIPPED:** Statt die Pre-Generalprobe formell durchzuziehen, ging die Echte direkt — Hotfixes (live-counter, button-type, sort, magic-link) lieferten realistischeres Feedback als jede Probe
- **Hotfix-Pattern aus produktivem Betrieb:** Während der echten GV aufgetretene Bugs (Dioxus-Button-Reload, ✓-Glyph-Render-Problem) wurden live behoben und sind als Memory-Einträge (`feedback_dioxus_button_type`) festgehalten
- **ADR-2026-05-06 als Reverse-Engineering aus Production-Feedback:** „One-time-display"-Entscheidung von Phase 2 wurde zugunsten persistierter Code-Spalte revidiert, weil die GV-Praxis zeigte, dass Vorstand-Reprint nötig ist — saubere ADR-Dokumentation des Reversals

### What Was Inefficient

- **REQUIREMENTS.md Traceability-Drift:** 13/22 Checkboxes waren am Milestone-Close veraltet (`[ ]` obwohl die Phase verifiziert war) — der `/gsd-execute-phase`-Schritt synchronisiert die Checkboxes nicht automatisch zurück; sollte zukünftig automatisiert sein
- **Phase 02 als „human_needed" hängengeblieben:** Die SC#4 + SC#7 Deferred-to-Phase-3 Items wurden in der VERIFICATION.md als unvollständig markiert, obwohl Phase 3 sie genau das geschlossen hat — ohne manuelle Querreferenz zum Phase-3-Audit bleibt der Status irreführend
- **UAT-Checklisten formell offen, faktisch durch Live-GV geschlossen:** `02-HUMAN-UAT.md` partial und `04-UAT-CHECKLIST.md` unknown blieben nach der realen GV-Durchführung nicht systematisch updated; bei Milestone-Close mussten sie acknowledged werden
- **Phase 04 Tooling-Debt (`wasm-bindgen-cli@0.2.104`):** Wurde bereits in Phase-4-VERIFICATION als PENDING markiert, hat sich nicht aufgelöst, weil die produktive GV via `deploy-binaries.sh` lief — Toolchain wurde nie auf Nix-Profil hochgezogen
- **Pitfall 6 (Tailwind Purge .qr-card):** Kostete nach Phase-4-Verify einen zusätzlichen Fix-Commit (`bfffbe2`) weil Tailwind die `.qr-card`-Print-Rules ohne Top-level-Selektor wegpurged hat — Lehre für Custom-CSS-Print-Rules

### Patterns Established

- **DSGVO-Whitelist-Pattern mit 3 Verteidigungslinien:** Struct-Whitelist + Doc-Verbot der Konversion aus generischen TOs + Konversion exklusiv aus dedizierter DAO-Row (siehe AttendanceMemberTO)
- **Cascade-Invalidation in Lifecycle-Operations:** `close_assembly` → `list_session_ids_for_assembly` (in tx) → `tx.commit()` → pool-loop `delete_session` mit `tracing::warn!` bei Fehlern (Continue-on-Error mit Defense-in-Depth via D-18 Status-Check)
- **`AuthContext::Helper`-Enum-Variante** statt Sub-Type — typsicher, ohne cfg-Gate für mock_auth + oidc; abrufbar via `ClaimContext::as_helper(&self) -> Option<Uuid>` mit Default-Impl `None` (failure-closed)
- **Component-First-Trigger:** Identische UI auf zwei Pages → sofort in `genossi-frontend/src/component/`; bei nur einer Seite (Phase-6-ExportTab) bewusst inline (D-20)
- **No-Audit für hochfrequente Toggles:** Anwesenheits-Markierungen explizit OHNE `audited_*!`-Macros + Grep-Gate als Test
- **Snapshot-Pattern für stabile historische Zustände:** Member-Universe-Snapshot beim GV-Öffnen — Composite-PK-Tabelle ohne `deleted` (permanent)
- **Read-only Aggregate ohne neue DAO-Methoden:** Phase 6 nutzte existierendes `AttendanceDao::list_members_for_assembly` ohne Modifikation — Pattern für reine Konsumenten-Features
- **Hotfix-Memory-Pattern:** Wiederkehrende Pitfalls (Dioxus-Button-Reload-Bug) als `feedback_*.md` in Memory eintragen — zukünftige Sessions vermeiden den gleichen Fix

### Key Lessons

1. **REQUIREMENTS.md Checkbox-Sync sollte automatisch sein.** Bei Plan-Completion ist die Information da („dieser Plan completed REQ-IDs X, Y, Z"), aber der Sync zurück auf die Traceability-Table erfolgt nicht. Empfehlung: `gsd-sdk query` Hook beim Summary-Write.

2. **Phase-Deferred-Cross-Reference fehlt im Milestone-Audit.** Wenn Phase 2 etwas an Phase 3 deferred, sollte der Milestone-Audit automatisch erkennen, dass Phase 3 es geschlossen hat — manuelle Querverweise sind fehleranfällig.

3. **Produktive Verifikation > formelle Generalprobe.** Phase 5 SKIPPED war die richtige Entscheidung — die echte GV lieferte echtes Feedback, die Pre-Probe wäre Excel-für-Genossi-Generalprobe gewesen, also formal, nicht real.

4. **Tooling-Debt klar von Code-Debt trennen.** Phase-4 PENDING-Status (`dx build --release`) blieb 23 Tage offen, weil keiner ihn als „nicht-blockierend für Production" markiert hat — `deploy-binaries.sh` lief ja. Empfehlung: Tooling-Debt explizit als separate Kategorie führen.

5. **`/gsd-discuss-phase` mit 20 D-Decisions (Phase 6) war zu detailliert.** D-01..D-20 hätten als 10 leitende Entscheidungen plus implementierungstechnische Details in CONTEXT.md auch funktioniert. Die Tiefe der Discussion bezahlt sich aber bei `/gsd-plan-phase`: 4 Plans mit klaren Truths, alle 25/25 must-haves verified beim ersten Run.

### Cost Observations

- Echte Generalversammlung als E2E-Verifikation (stärkster möglicher Test) reduzierte den Bedarf an formellen UAT-Runs
- Hotfix-Velocity während der GV war hoch — vier Bug-Fixes (`8e92cfd`, `e245013`, `ed754fc`, `3cdfbb6`) während des Live-Betriebs eingeflossen, ohne Phase-Tracking-Overhead
- Anteilig haben Phase 1 (Foundation) und Phase 6 (Read-only Aggregate) weniger Iterationen gebraucht als Phase 2 (Helfer-Token, viele Edge-Cases) und Phase 4 (Frontend, viele Komponenten)

---

## Milestone: v1.1 — Anteile-Rückzahlungsphase

**Shipped:** 2026-06-02
**Phases:** 7 (07-13) | **Plans:** 56 | **Tasks:** 91

### What Was Built

Vollständige Anteile-Rückzahlungs-Pipeline als Excel-Ersatz: RepaymentPhase-Aggregat mit Lifecycle Vorbereitung → Offen → Abgeschlossen (Phase 7), RepaymentEntry mit Auto-Befüllung aus Vorjahres-Austritten + manuellem Picker + Batch-Status-Toggle (Phase 8), atomare Auszahlungs-Buchung via 12-Schritt-Cascade `audited_create!(MemberAction::Verkauf) + audited_update!(Member, RepaymentEntry)` in einer SQLite-Tx (Phase 9), Massenmail mit `{{ payout_amount }}`/`{{ share_count }}`/`{{ fiscal_year }}`-Variablen über bestehenden `POST /api/mail/send-bulk` (Phase 10), Typst-basiertes PDF-Export mit 6-Spalten-Banking-Vorlage und Filter `?include=open|all|paid` (Phase 11), Component-First-Frontend mit `RepaymentEntryList` + 3-Tab-Detail-Page + Inline-Cell-Edit (Phase 12), und additive Bulk-PDF-Briefe für Nicht-Email-Mitglieder mit `RepaymentContextResolver`-Aggregation + Direct-Download + auditierten MemberDocuments (Phase 13).

### What Worked

- **Pattern-Replikation als Beschleuniger:** Phase 7 RepaymentPhaseDao + Auditable + Service-Impl waren 1:1-Replikat des v1.0-Assembly-Aggregats mit Domain-Substitutionen (`fiscal_year: i32`, `share_value: i64 Cent`). Phase 8 RepaymentEntry war 1:1-Replikat von Phase 7. Das senkte die Cognitive-Load auf "Domain-Mapping" statt "Architektur erfinden".
- **Atomare Cascades + Audit-Hashchain als Sicherheitsnetz:** PAYO-Cascade (`audited_create!` + 2× `audited_update!` in einer Tx mit gemeinsamem Process-String) machte die Korrektheit per `/api/audit/verify` end-to-end überprüfbar. Audit-Chain-Multi-Endpoint-Assertion-Pattern (Phase 9 D-Pattern) wurde Vorlage für jede zukünftige Cross-Entity-Cascade.
- **Defensive Programmierung wo nötig:** `ausbezahlt`-Toggle final (PAYO-04), kein Composite-PK auf (member_id, phase_id) (ENTR-03), Pre-Exists-Check vor UPDATE (trennt NotFound von ConflictError), Edit-Matrix vor Version-Check (klarere Fehlermeldung), Frozen-Order-Audit-Felder mit Index-Tests — alle in den Phasen 7+8 etabliert und in Phase 9 wiederverwendet.
- **Inline Hotfixes statt separater Bug-Phasen:** 6 UAT-Defekte in Phase 12 (Dioxus button-reload, toast z-index, mail_page Query-Param-Race, entry-vs-member-ID-Mismatch, Phase-10 pure-member-probe blockt `{{ payout_amount }}`, /preview-Endpoint mit `repayment_phase_id`) wurden inline in den Plan-Commits resolved, ohne Bug-Brief-Overhead.
- **`RepaymentContextResolver` in Phase 13 als Pattern-Konsolidierung:** Trait + Impl mit `resolve()` (1 Entry) und `aggregate()` (N Entries für Bundle, eine DB-Read-Round) eliminierte N+1-Pattern für die Bulk-Brief-Pipeline. Phase-10-Worker bleibt per D-13-10 unverändert (Inline-Aggregation bewährt), aber für künftige Bulk-Operationen ist der Resolver der Anker.

### What Was Inefficient

- **Spät erkannter `format_dt`-Duplikat:** Phase-7-Helper war `fn` privat, Phase 8 musste duplizieren. Hätte mit `pub(crate)` 1 Zeile gespart werden können; Refactor in `crate::dt_helpers` wäre Rule-4-Change und blieb out-of-scope.
- **Plan-Text-Drift gegenüber Codebase:** Phase 10 Plan 07 behauptete, `transaction_dao` existiere bereits als RestStateImpl-Feld — tatsächlich war es das `DbAssemblyStatusProbe.transaction_dao`-Hilfsstruct (mock_auth-only). Rule-3-Auto-Fix musste das Feld nachreichen. Lesson: vor Move-vs-Clone-Refactors immer per `grep -nE '^\s*field:\s*Arc<...>'` auf der ECHTEN Struct-Definition verifizieren.
- **`{% if X is defined %}` vs `{% if X %}` Falle:** Plan 10-05 Test-Spec verwendete bare `{% if payout_amount %}` — minijinja-strict erwartete `{% if payout_amount is defined %}`. Hat einen ganzen RED-GREEN-Zyklus gekostet, bis das Pattern korrekt etabliert war. In v1.2 als known-pattern-Pitfall mitnehmen.
- **Quick-Tasks-Index out-of-sync:** Zwei Quick-Tasks (`260601-pfy`, `260602-c19`) wurden committet, aber ihre Index-Status blieb `missing`. Workflow-Issue, nicht Code-Issue, aber pollutet den Pre-Close-Audit.
- **Phase-12-Helper-OIDC-Session-Limit:** UI-06 konnte nicht voll-validiert werden, weil lokal keine Non-Admin-OIDC-Session verfügbar war. Tech-Debt für künftige Vorstand-vs-Helper-View-Tests.

### Patterns Established

- **i64-Cent-Konvention:** Alle Money-Werte als i64 Cent (kein Decimal/Float), SQLite INTEGER 8-Byte. Validierung `> 0` ohne Obergrenze (D-12). Vorlage für künftige Auszahlungs-/Einzahlungs-Felder.
- **Frozen-Order-Audit-Felder mit Index-Tests:** `test_auditable_audit_fields_member_id_first_phase_id_second` (Index-basiert) zusätzlich zu Names-Vec-Test verhindert Refactorings die nur Reihenfolge tauschen. Hash-Chain-Konsistenz garantiert.
- **404-vs-409-Trennung im Aggregat:** Missing/soft-deleted Entry → 404 `EntityNotFound`; Status-Konflikt (Edit-Matrix-Verletzung) → 409 `Conflict`. Pattern für künftige strukturierte 409-Body-Endpunkte: TO-Schema-Doc dokumentiert explizit welche Status-Codes NICHT abgedeckt sind.
- **Edit-Matrix-Check vor Version-Check:** Atomare D-07-Ablehnung liefert semantisch klarere Fehlermeldung als generisches `"Version mismatch"`. Mock-update().times(0) verifiziert no-write on violation.
- **0 direkte DAO-Calls außerhalb `audited_*!`-Macros:** Grep-Gate als Pre-Merge-Check für Repudiation-Defense. Filter `grep -v '^//' | grep -v '^*'` verhindert Self-Invalidation durch Anti-Pattern-Erwähnung in Kommentaren.
- **Direct-Download-Pattern für Bulk-PDFs:** Bundle-PDF transient als HTTP-Response, Persisted MemberDocuments parallel; `X-Document-Count`-Header für Frontend-Toast-Pluralisierung. Wiederverwendbar für künftige Multi-Document-Export-Endpunkte.
- **Resolver-Trait für N+1-Elimination:** `RepaymentContextResolver::aggregate` lädt N Entries in einer DB-Round. Pattern-Anker für künftige Bulk-Service-Operationen mit gleichem Aggregations-Pfad.

### Key Lessons

1. **Pattern-Replikation > Pattern-Erfindung:** Phase 7+8+9 waren systematische 1:1-Mirror des v1.0-Assembly-Aggregats. Wo es funktioniert, schneller Replicate als Re-Design.
2. **Audit-Hashchain als End-to-End-Korrektheits-Beweis:** `/api/audit/verify` + Multi-Endpoint-Field-Assertion macht atomare Cascades verifizierbar ohne externe Tools.
3. **Minijinja-strict + `is defined`-Guard ist Pflicht:** Bare `{% if X %}` errort auf truly-missing Variablen unter `UndefinedBehavior::Strict`. Templates-Authoring-Konvention für künftige Phase-12+-Frontend-Template-Editoren.
4. **Workspace-Toolchain via Nix-Store-Suche:** `rustfmt`/`clippy` fehlen oft am direkten PATH, sind aber unter `/nix/store/...` verfügbar. Nicht vorschnell "not installed" reporten — erst nix-store durchsuchen.
5. **Quick-Task-Tracking parallel zu Phase-Plänen ist Risiko:** Out-of-sync Index pollutet Audit. v1.2: Quick-Tasks atomisch via CLI (`gsd-sdk query quick.commit`) statt manuelle Frontmatter.

### Cost Observations

- **Pattern-Replikation senkt Pro-Phasen-Cost:** Phasen 7+8+9 waren systematisch schneller als Phase 10 (Massenmail mit minijinja-strict, viele Edge-Cases) und Phase 12 (Frontend mit 11 Waves, 15 Plans, 6 inline UAT-Fixes).
- **Phase 11 (PDF-Export) war single-Phase-Slot mit hoher Iteration:** Typst-Template-Drift (`Anteilsrückzahlung` Umlaut, Verwendungszweck-Strings, B3 Euro-Format ohne `.abs()`) brauchte mehrere REVISION-Fixes.
- **Phase 13 Bulk-Letter als additive Folge-Phase nach Milestone-Goal:** Reagierte auf Live-Bedarf (Mitglieder ohne E-Mail-Adresse), nicht im ursprünglichen v1.1-Scope. 7 Pläne in 6 Waves, ~2 Wochen. Beweis dass additiv eingeschobene Phasen mit klar geschnittenem Goal funktionieren.

---

## Milestone: v1.2 — Mitgliedschaft-Anpassungen während des Geschäftsjahres

**Shipped:** 2026-06-07
**Phases:** 5 | **Plans:** 24 | **Timeline:** 2026-06-04 → 2026-06-07 (4 Tage, 127 commits)
**Audit-Status:** `tech_debt` (31/31 REQs satisfied, kein BLOCKER für Phase-Goal)

### What Was Built

- **DAO/Domain Foundation (Phase 14):** `compute_effective_date` Pure-Function für H1/H2-Stichtag-Berechnung (6 Edge-Case-Tests), `RepaymentEntryDao::find_by_member_and_phase` als Foundation für Phase-16-Sum-Check + Auto-Fill-Skip-Pattern, `MemberSlimTO` mit 6-Feld-PII-Guard und sub-route-ordered `/api/members/transfer-recipients`-Endpoint
- **Cancel + Increase (Phase 15):** `MembershipAdjustService`-Trait + Service-Impl mit ADMIN_PRIVILEGE-Funnel, `cancel_membership` (audited_create Austritt + recalc_dates), `increase_shares` (audited_create Aufstockung + audited_update Member.current_shares atomar); 9 aktive E2E + 2 #[ignore] (mock_auth always-admin); v1.2-Audit-Pattern für Phase 16+17 etabliert
- **Partial Repayment + Auto-Anlegen-Phase (Phase 16):** `partial_repayment` mit 14-Schritt-Pipeline inkl. Closed-Phase-Status-Guard (HTTP 409 vor jedem audited_create), Sum-Check via `find_by_member_and_phase`, Auto-Create-Branch (Variante B: Status=Open, `DEFAULT_SHARE_VALUE_CENT=10000`-Fallback). Plan 16-05 als Gap-Closure für CR-01 nachträglich; 18 E2E-Tests + 11 Service-Unit-Tests; Auto-Fill-Skip-Pattern in `open_repayment_phase`
- **Transfer Atomic Cascade (Phase 17):** `transfer_shares` als 15-Schritt-Single-Tx mit Pre-Write-Detection des Voll-Übertrags; 5 audited_*!-Calls teilen `TRANSFER_PROCESS="member-adjust.transfer"`; Voll-Übertrag-Branch erzeugt zusätzlich `MemberAction::Austritt` mit `effective_date=transfer_date`; 8 E2E + 7 Pure-Function-Tests + 10 Mock-Service-Tests + 2 Race-Pattern (Same-Direction + Cross-Direction)
- **Frontend MembershipAdjustModal (Phase 18):** Single-file Dioxus modal (1078 LOC) mit `ModalStep`-Enum + 4 Sub-Views Cancel/PartialRepayment/Transfer/Upgrade; Live-Preview-Section pro Sub-View; `FiscalYearDateInput` mit GJ-Bounds; `ToastVariant{Success,Error}` + `show_success_toast` + `SuccessToastContainer` (additive Erweiterung, Zero Blast Radius); `MemberSearch.members_override`-Prop für Transfer-Empfänger-Decoupling; 8 neue DTOs + 5 API-Funcs + 46+ i18n-Keys DE/EN; Vorstand-UAT-Sign-Off durch Browser-Walk-Through aller 6 Szenarien am 2026-06-07
- **Audit-Integrität:** Audit-Hashchain bleibt valid über alle 4 v1.2-Operationen; AUDT-02 (TRANSFER_PROCESS shared by all 5 audited_*!-writes) explizit E2E-asserted via `assert_transfer_audit_trail`-Helper + `/api/audit/verify`

### What Worked

- **Cross-Phase-Foundation-Reuse:** Phase 14 (compute_effective_date + DAO-Methode) als pure Foundation hat sich bezahlt — Phasen 15+16 konnten parallel ohne Friction implementieren, kein „warte auf Foundation"-Block
- **Service-Trait incremental extension Phase 15→16→17:** Eine `MembershipAdjustService`-Trait mit 4 Methoden statt 4 separate Services — eine Dependency-Liste, gemeinsamer Audit-Pattern, kein DI-Explosion. `gen_service_impl!`-Macro skaliert sauber
- **`recalc_dates` Free-Function-Refactor:** Cross-Service-Reuse (cancel + transfer_shares Voll-Übertrag) ohne Trait-Bound-Hell; `pub(crate) async fn recalc_dates<Md, Mad, Tx>` als idiomatisches Rust-Pattern
- **Closed-Phase-Status-Guard Re-Verification-Pattern (Plan 16-05):** Gap-Closure als eigener Plan mit 3 Commits (Service-Guard + Unit-Test + E2E-Test); Re-Verification flipped Status von `gaps_found` auf `passed`. Beweis: GSD-Workflow tolleriert nachträgliche Gap-Closure ohne Phase-Re-Open
- **Pre-Write-Detection für Voll-Übertrag (D-17-01):** Statt via recalc_dates-Trigger zu detecten, wird die Voll-Übertrag-Logik vor dem audited_create durchgeführt — klare Sequenz, `recalc_dates(from)` läuft genau einmal nach der Cascade
- **Single-File-Modal mit ModalStep-Enum:** `MembershipAdjustModal` als 1078-LOC-Component mit State-Machine statt 4 separate Modals — Sub-Choice-UX braucht gemeinsame State, separates wäre Code-Duplikat
- **Frontend Wave-Parallelisierung:** Phase 18 in 3 Wellen — Wave 1 mit 5 parallelen Foundation-Plans (DTOs, Toast-Extension, FiscalYearDateInput, MemberSearch-Override, API-Funcs), Wave 2 Modal-Composition, Wave 3 Page-Integration. Backend-First-Logik analog
- **Component-First konsequent:** Memory `feedback_component_first.md` hat sich bezahlt — keine inline-RSX, keine Phase-18-Page-spezifischen Komponenten; alle wiederverwendbaren Bausteine in `genossi-frontend/src/component/`
- **Vorstand-UAT-Sign-Off als Mensch-Verifikations-Anchor:** `18-MANUAL-UAT.md` mit 6 Szenarien als persistentes Artefakt — Browser-WASM-Verhalten programmatisch nicht prüfbar, Manual-UAT füllt diese Lücke

### What Was Inefficient

- **REQUIREMENTS.md-Checkbox-Drift wiederholt:** 7/31 REQs in der Live-Datei waren `[ ]` und Traceability="Pending", obwohl die Phase-VERIFICATION sie als SATISFIED markierte. Gleicher Drift wie in v1.0 (13/22 unchecked) und v1.1 — der `gsd-execute-phase`-Schritt synchronisiert die Checkboxes nicht zurück. **Recurring lesson** seit v1.0; sollte endlich automatisiert werden (Hook beim Summary-Write).
- **CR-02 Permission-Check-Ordering als blinder Fleck:** Wurde in v1.1 nicht entdeckt (bestehender Pattern in `repayment_phase.rs`), Phase 15+16 hat das Pattern für v1.2 1:1 kopiert. Erst beim Plan-16-04 Code-Review aufgefallen, dann explicitly out-of-scope geparkt. Lehre: Code-Review sollte projektweit identische Anti-Patterns als „pre-existing carry-forward" erkennen, nicht nur in der aktuell betrachteten Phase
- **ROADMAP.md-Hygiene drift:** Phase 16 listete „5/5 plans complete" aber nur 16-01..04 in der Plans-Liste — Plan 16-05 (Gap-Closure) wurde im Service committet, aber die Roadmap-Übersicht nicht aktualisiert. Plus Top-Tabelle mit verrutschten Spalten. Beim Archive aufgefangen, aber zeigt: Roadmap-Updates per Plan-Completion sollten automatisiert sein
- **STATE.md-Zähler kaputt:** `completed_phases: 6 / total_phases: 5 / percent: 120` — entstand vermutlich durch `gsd-execute-phase`, der bei Gap-Closure-Plans (Plan 16-05) den completed_phases-Counter weiter bumpt statt unter total_phases zu bleiben. Vom Workflow im update_state-Step überschrieben
- **Phase 18 CR-01 + CR-02 als UX-Critical aber non-blocking Findings:** Zwei substanzielle UX-Findings (date_signal-leak, Signal::set in Render-Pfad) wurden im Code-Review als „critical" markiert, aber als nicht-blockierend eingestuft weil Submit-`is_valid` die Konsequenz blockt. Vorstand-UAT lief trotzdem grün durch. Frage: Hätten wir vor Plan-18-07 ein Refactor einschieben sollen oder ist „UX-Polish als v1.3-Cleanup" akzeptabel? Pro v1.3-Cleanup: kein Bug, kein Datenverlust. Contra: erkannte Findings sollten gefixt werden, sonst Tech-Debt-Akkumulation
- **Wire-Asymmetrie `serde_json::Value` im Frontend:** Bewusst-doku'd-Entscheidung („Zero-Coupling") wurde nicht in Frage gestellt. Tatsächlich verwirft das Modal den Response-Body sowieso (`Ok(_resp) => …`), also kein Runtime-Defekt. Aber zukünftiges `entry.id`-Read braucht Value::get()-Workarounds — späte Coupling-Schuld
- **`REPAYMENT_PHASE_CREATE_PROCESS`-String-Duplikat zu v1.1:** WR-05 dokumentiert, dass v1.2-Auto-Create-Phase und v1.1-Phase-Create den gleichen Audit-Process-String nutzen — forensische Queries können sie nicht unterscheiden. War keine bewusste Entscheidung, sondern Copy-Paste aus v1.1. Lehre: Audit-Process-Strings sollten in REVIEW als Cross-Cutting-Concern explizit überprüft werden

### Patterns Established

- **Pure-Function vor DAO-Trait für Berechnungs-Logik:** `compute_effective_date` als pure function in `membership_adjust.rs` statt in DateTime-Service — testbar ohne Clock-Mocking, 2-Branch-Berechnung benötigt kein Trait-Boilerplate. Pattern: wenn die Berechnung deterministisch ist und keine I/O macht, ist ein pub(crate) fn schlanker als ein Trait
- **Service-Trait incremental extension Pattern:** Eine Trait mit N Methoden, die incremental über N Phasen extended wird — D-15-13 explizit angewendet (Phase 15: trait+2 Methoden, Phase 16: +1 Methode, Phase 17: +1 Methode). Pro: Eine Dependency-Liste, gemeinsamer Audit-Pattern. Contra: Trait wächst, irgendwann splitten
- **15-Schritt-Single-Tx-Cascade als Service-Methode:** `transfer_shares` als deklarative 15-Schritt-Sequenz mit shared `tx.clone()` — Pattern bewährt sich für komplexe atomic operations. Cf. v1.1 Phase-9 `mark_paid_out` 12-Schritt-Cascade
- **Pre-Write-Detection-Pattern (D-17-01):** Statt Voll-Übertrag via recalc_dates-Trigger zu detecten, vor dem audited_create durchführen → klarere Sequenz, recalc_dates läuft exakt einmal nach der Cascade
- **Closed-Phase-Status-Guard vor audited_create (PITFALLS-Kat-Lifecycle):** Wenn Service auf existierende Entity reagiert (Phase im fiscal_year), MUSS Status-Guard VOR jedem audited_create stehen — kein Orphan-Audit-Eintrag durch rejected Request. Pattern: Status-Check zwischen Phase-Lookup und Match-Branch
- **Pre-Write-Voll-Detection + recalc_dates(once) Pattern für Aggregat-Cascades:** Generisches Pattern für Operations, die optional zusätzliche Action triggern (Voll-Übertrag erzeugt Austritt zusätzlich zur Übertragung): erst die Konsequenz detecten, dann alle audited_*! in passende Reihenfolge sequenzieren, danach recalc_dates exakt einmal
- **Sub-Route-Ordering Pitfall mitigierende Inline-Doc-Comments:** `member.rs` Sub-Route-Block hat explizite Doc-Comments „Sub-routes BEFORE `/{id}` catch-all" als Defense gegen zukünftige Drift — Pattern aus Phase 14
- **`MemberSlimTO` mit 6-Feld-PII-Guard:** DSGVO-Pattern aus v1.0 `AttendanceMemberTO` fortgesetzt — neue identifizierende DTOs werden mit minimaler Feld-Anzahl + explizitem Doc-Comment-Verbot der From<&MemberTO>-Konversion definiert
- **Race-Pattern E2E-Tests mit NIE-`[200, 200]`-Klausel:** Cross-Direction-Race + Same-Direction-Race jeweils mit `tokio::join!` + 1ms-Sleep + Konsistenz-Assertions; akzeptiert sortiert `[200, 409|500]`, NIE `[200, 200]`. Pattern aus v1.1 Phase-9-PaidOut-Race fortgesetzt
- **Modal-Wave-Parallelisierung im Frontend:** Wave 1 (5 parallele Foundation-Plans für DTOs, Components, API-Funcs) → Wave 2 (1 Modal-Composition-Plan, der alles aus Wave 1 wired) → Wave 3 (1 Page-Integration-Plan). Pro: maximale Parallelität. Contra: Wave 2 muss wirklich alle Wave-1-Outputs konsumieren — keine späteren API-Funcs
- **Manual-UAT als persistentes Artefakt:** `18-MANUAL-UAT.md` mit 6 Szenarien + Sign-Off-Frontmatter als Brücke zwischen automatisierten Tests und Vorstand-Browser-Verifikation; Pattern für Frontend-Phasen, wo WASM-Verhalten programmatisch nicht prüfbar ist

### Key Lessons

1. **REQUIREMENTS.md-Checkbox-Drift ist jetzt 3× verifiziert (v1.0, v1.1, v1.2).** Trotz expliziter Lesson nach v1.0 ist v1.2 wieder mit 7/31 Drift-Items geschlossen worden. Mensch-im-Loop-Reminder reicht nicht — der `gsd-execute-phase` braucht einen Sync-Hook beim Summary-Write, der REQ-IDs aus dem `requirements-completed`-Frontmatter automatisch in der REQUIREMENTS.md-Traceability-Table als Complete + Checkbox `[x]` markiert. Empfehlung: in v1.3-Cleanup-Phase implementieren.

2. **Pre-existing Anti-Patterns vererben sich projektweit ohne Code-Review-Cross-Cutting-Check.** CR-02 (Permission-Check-Ordering) existierte schon in v1.1's `repayment_phase.rs`, v1.2 hat es 1:1 in 4 neue Methoden kopiert. Code-Review-Agent sollte beim Audit nicht nur die diff-Files prüfen, sondern bekannte Anti-Patterns projektweit suchen und als „pre-existing carry-forward" flaggen. Cleanup-Phase für v1.3 sollte projektweit refactoren (extrahierbar in `gen_auth_admin!`-Helper).

3. **„UX-Polish als Tech-Debt akzeptieren" ist ein Akt der Reife, kein Versagen.** Phase 18 CR-01 + CR-02 wurden bewusst nicht gefixt (Submit-`is_valid` blockt Datenfehler, kein Korrektheits-Bug), und der Vorstand-UAT lief trotzdem grün. Lehre: nicht jedes Code-Review-Finding muss vor Milestone-Close gefixt werden — wenn die Konsequenz durch andere Verteidigungslinien (hier: Frontend-Validation) abgefangen ist, ist „in nächste Phase carry-forward" eine legitime Workflow-Option. Aber: tatsächlich tracken, nicht vergessen.

4. **Pure-Function-Foundation als eigene Phase rentiert sich.** Phase 14 (`compute_effective_date` + DAO-Methode + REST-Endpoint) war 4 Plans, aber Phasen 15+16 konnten danach parallel ohne Friction laufen. Hätten wir die Pure-Function in Phase 15 eingebaut, wäre Phase 16 von Phase 15 blockiert gewesen. Backend-First gilt nicht nur zwischen Backend/Frontend, sondern auch innerhalb des Backends: Domain-Logik vor Service-Use.

5. **Audit-Process-Strings als Cross-Cutting-Concern explicit prüfen.** WR-05 (`REPAYMENT_PHASE_CREATE_PROCESS`-Duplikat zu v1.1) ist kein funktionaler Bug, aber forensische Queries leiden. Code-Review sollte für jeden neuen `audited_*!`-Call den Process-String aktiv prüfen: ist er unique? Falls nicht, ist das absichtlich? Constants statt String-Literals helfen, aber das alleine reicht nicht — Cross-Crate-Audit muss explizit sein.

6. **Re-Verification nach Gap-Closure ist ein etabliertes Pattern (jetzt 2× angewendet).** Plan 16-05 als nachträglicher Plan im laufenden Milestone, der CR-01 aus dem 16-REVIEW schließt — VERIFICATION flipped von `gaps_found` auf `passed`. Beweis: GSD-Workflow toleriert nachträgliche Gap-Closure ohne Phase-Re-Open. Wichtig: das Re-Verification-Frontmatter (`previous_status`, `gaps_closed`, `new_truths_added`) als first-class-Datenstruktur halten — das ist der Forensik-Anker.

7. **127 Commits in 4 Tagen ist eine andere Velocity-Klasse als v1.1's 651 in 62 Tagen.** Pattern-Replikation + bereits etablierte Audit-/Service-/Frontend-Patterns aus v1.1 haben die Phase-Setup-Zeit dramatisch reduziert. Cross-Milestone-Lesson: je mehr stabile Patterns existieren, desto schneller können Folge-Milestones liefern. Aber: Tech-Debt-Akkumulation passiert auch schneller (CR-02 in 4 Tagen 4× kopiert).

### Cost Observations

- **Model mix:** ~ähnliche Verteilung wie v1.1 (überwiegend Opus für Discuss/Plan/Verify, Sonnet für Execute) — Annahme, exakte Telemetrie nicht erfasst
- **Sessions:** Aus der Commit-Verteilung über 4 Tage geschätzt ~8-12 Sessions (Phase-Boundaries als natürliche Session-Splits)
- **Notable:** Phase 18 als komplexester Frontend-Block mit 7 Plans wurde in einem Tag durch-implementiert (Wave-Parallelisierung), aber Vorstand-UAT-Sign-Off als separater Mensch-Loop am Ende des Tages. Wave-Pattern erlaubt höhere Velocity ohne Quality-Loss
- **Re-Run-Pattern:** Plan 16-05 als nachträglicher Gap-Closure-Plan war 3 Commits + 1 Stunden-Slot — kostengünstig im Vergleich zu Phase-Re-Open. Empfohlene Default-Strategie für post-VERIFICATION-Gap-Findings

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Skipped/Added | Key Change |
|-----------|--------|-------|---------------|------------|
| v1.0 | 5 + 1 SKIPPED | 34 | Phase 5 (Pre-Generalprobe → echte GV) | Erstmaliger Einsatz von GSD im Projekt; Phase 5 SKIP statt formellen Operations-Plan |
| v1.1 | 7 | 56 | Phase 13 ADDED (Brief-Bulk additiv nach Phase 12) | Pattern-Replikation als dominantes Velocity-Pattern; additive Folge-Phase als legitime Workflow-Option |
| v1.2 | 5 | 24 | Plan 16-05 ADDED (Closed-Phase-Status-Guard Gap-Closure) | Re-Verification-Pattern (gaps_found → passed via nachträglichen Plan); Wave-Parallelisierung im Frontend (Phase 18 in 3 Wellen) |

### Cumulative Quality

| Milestone | Tests (Workspace) | E2E-Tests | Notable |
|-----------|------------------|-----------|---------|
| v1.0 | ~927 (Backend 819 + Frontend 108) | 239 (E2E) | Race-Test deterministisch über 5 Runs grün; PII-Guard via Whitelist+Blacklist-E2E-Test |
| v1.1 | ~600 Workspace-Lib | 292 (E2E, +53 ggü. v1.0-Close) | Audit-Hashchain bleibt valid über alle Phase-9-Cascades; Atomic-Tx + Re-Read-Defense in jeder Lifecycle-Methode |
| v1.2 | ~67 neue Tests (Workspace+E2E) | + ~28 E2E (membership_adjust_e2e + transfer_recipients_e2e) | Audit-Hashchain bleibt valid über 4 neue Operations; Race-Tests (Same+Cross-Direction) deterministisch; Cross-Phase-Integration 6/6 OK; E2E-Flows 4/4 working |

### Velocity (Timeline)

| Milestone | Days | Commits | Commits/Day |
|-----------|------|---------|-------------|
| v1.0 | ~15 | (nicht erfasst) | — |
| v1.1 | 62 | 651 | ~10.5 |
| v1.2 | 4 | 127 | ~32 |

v1.2-Velocity-Sprung erklärt sich durch: (1) etablierte Audit-/Service-/Frontend-Patterns aus v1.0+v1.1 als Foundation, (2) Pattern-Replikation statt Erfindung (Service-Trait-Pattern, Audit-Macros, Component-First-Patterns existierten alle), (3) Wave-Parallelisierung im Frontend, (4) klar geschnittenes Goal mit nur 4 Operations.

### Top Lessons (Verified Across Milestones)

1. **(v1.0)** Produktive Verifikation schlägt formelle Generalprobe, wenn das Feature ohnehin live einsetzbar ist. SKIPPED-Phasen sind eine legitime Workflow-Option, wenn ihre Voraussetzung durch die Realität bereits erfüllt ist.
2. **(v1.1)** Pattern-Replikation gegenüber Pattern-Erfindung: wo ein etabliertes Aggregat-Pattern existiert, ist systematisches 1:1-Mirror schneller als Re-Design. Domain-Substitutionen sind reine Mapping-Arbeit.
3. **(v1.0+v1.1+v1.2)** Audit-Hashchain als E2E-Korrektheits-Beweis funktioniert über atomare Cascades hinweg. Multi-Endpoint-Field-Assertion ist Pflicht-Pattern für Cross-Entity-Operationen. v1.2 erweitert: shared `*_PROCESS`-String für verlinkte Action-Pairs (TRANSFER_PROCESS für UebertragungAbgabe + Empfang) macht das Audit-Aggregat forensisch nachvollziehbar.
4. **(v1.1+v1.2)** Additive Phasen nach VERIFICATION sind legitim, wenn sie auf konkretes Gap-Finding reagieren und klar geschnittenes Goal haben. v1.1: Phase 13 Brief-Bulk (Live-Bedarf). v1.2: Plan 16-05 Closed-Phase-Guard (Code-Review-Finding). Re-Verification-Pattern flipped Status von `gaps_found` auf `passed`.
5. **(v1.0+v1.1+v1.2) RECURRING:** REQUIREMENTS.md-Checkbox-Drift bei Milestone-Close ist 3× verifiziert. Trotz expliziter Lesson nach v1.0 ist v1.2 wieder mit 7/31 Drift-Items geschlossen worden. Mensch-im-Loop-Reminder reicht nicht — der `gsd-execute-phase` braucht einen Sync-Hook beim Summary-Write. Empfehlung: in v1.3-Cleanup implementieren.
6. **(v1.2)** Pre-existing Anti-Patterns vererben sich projektweit ohne Cross-Cutting-Code-Review-Check. CR-02 (Permission-Check-Ordering) existierte schon in v1.1, v1.2 hat es 1:1 in 4 neue Methoden kopiert. Code-Review sollte beim Audit projektweit nach bekannten Anti-Patterns suchen, nicht nur in der aktuell betrachteten Phase.
7. **(v1.2)** „UX-Polish als Tech-Debt akzeptieren" ist legitim, wenn die Konsequenz durch andere Verteidigungslinien abgefangen ist. Phase 18 CR-01/CR-02 (date_signal-leak, Signal::set in Render) wurden bewusst nicht gefixt, weil Submit-`is_valid` Datenfehler blockt. Wichtig: tatsächlich tracken, nicht vergessen.

---

*Last updated: 2026-06-07 after v1.2 milestone close*
