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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Skipped | Key Change |
|-----------|--------|-------|---------|------------|
| v1.0 | 5 + 1 SKIPPED | 34 | Phase 5 (Pre-Generalprobe → echte GV) | Erstmaliger Einsatz von GSD im Projekt; Phase 5 SKIP statt formellen Operations-Plan |

### Cumulative Quality

| Milestone | Tests (Workspace) | E2E-Tests | Notable |
|-----------|------------------|-----------|---------|
| v1.0 | ~927 (Backend 819 + Frontend 108) | 239 (E2E) | Race-Test deterministisch über 5 Runs grün; PII-Guard via Whitelist+Blacklist-E2E-Test |

### Top Lessons (Verified Across Milestones)

*Wird mit weiteren Milestones aufgebaut.*

1. **(v1.0)** Produktive Verifikation schlägt formelle Generalprobe, wenn das Feature ohnehin live einsetzbar ist. SKIPPED-Phasen sind eine legitime Workflow-Option, wenn ihre Voraussetzung durch die Realität bereits erfüllt ist.

---

*Last updated: 2026-05-29 after v1.0 milestone close*
