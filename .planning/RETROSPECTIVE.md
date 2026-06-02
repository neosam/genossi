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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Skipped/Added | Key Change |
|-----------|--------|-------|---------------|------------|
| v1.0 | 5 + 1 SKIPPED | 34 | Phase 5 (Pre-Generalprobe → echte GV) | Erstmaliger Einsatz von GSD im Projekt; Phase 5 SKIP statt formellen Operations-Plan |
| v1.1 | 7 | 56 | Phase 13 ADDED (Brief-Bulk additiv nach Phase 12) | Pattern-Replikation als dominantes Velocity-Pattern; additive Folge-Phase als legitime Workflow-Option |

### Cumulative Quality

| Milestone | Tests (Workspace) | E2E-Tests | Notable |
|-----------|------------------|-----------|---------|
| v1.0 | ~927 (Backend 819 + Frontend 108) | 239 (E2E) | Race-Test deterministisch über 5 Runs grün; PII-Guard via Whitelist+Blacklist-E2E-Test |
| v1.1 | ~600 Workspace-Lib | 292 (E2E, +53 ggü. v1.0-Close) | Audit-Hashchain bleibt valid über alle Phase-9-Cascades; Atomic-Tx + Re-Read-Defense in jeder Lifecycle-Methode |

### Top Lessons (Verified Across Milestones)

1. **(v1.0)** Produktive Verifikation schlägt formelle Generalprobe, wenn das Feature ohnehin live einsetzbar ist. SKIPPED-Phasen sind eine legitime Workflow-Option, wenn ihre Voraussetzung durch die Realität bereits erfüllt ist.
2. **(v1.1)** Pattern-Replikation gegenüber Pattern-Erfindung: wo ein etabliertes Aggregat-Pattern existiert, ist systematisches 1:1-Mirror schneller als Re-Design. Domain-Substitutionen sind reine Mapping-Arbeit.
3. **(v1.0+v1.1)** Audit-Hashchain als E2E-Korrektheits-Beweis funktioniert über atomare Cascades hinweg. Multi-Endpoint-Field-Assertion ist Pflicht-Pattern für Cross-Entity-Operationen.
4. **(v1.1)** Additive Folge-Phasen nach Milestone-Goal (z.B. Phase 13 Brief-Bulk) sind legitim, wenn sie auf konkreten Live-Bedarf reagieren und klar geschnittenes Goal haben. Nicht jeder Bug ist eine Quick-Task, nicht jede Erweiterung muss in den nächsten Milestone warten.

---

*Last updated: 2026-05-29 after v1.0 milestone close*
