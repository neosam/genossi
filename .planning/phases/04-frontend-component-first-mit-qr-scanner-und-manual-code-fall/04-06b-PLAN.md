---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 06b
type: execute
wave: 2.5
depends_on: [04, 05, 06]
files_modified:
  - genossi-frontend/src/component/mod.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "mod.rs deklariert pub mod ... für ALLE Component-Files die Plans 04, 05, 06 angelegt haben (12 neue Component-Files)"
    - "mod.rs re-exportiert ALLE in Plans 04+05+06 erwähnten public types (AttendanceList, AttendanceSearch, LiveCounter, ConnState, ConnectionBanner, ManualCodeInput, QrScanner, QrCard, HelperShell, AssemblyStatusBadge, AssemblyListRow, TabStrip, TabDef, ToastContainer, show_toast)"
    - "Plans 04, 05, 06 haben mod.rs NICHT geschrieben — alle drei können in Wave 2 parallel laufen ohne Merge-Konflikt; Plan 06b serialisiert das mod.rs-Update als einziger Wave-2.5-Plan"
  artifacts:
    - path: "genossi-frontend/src/component/mod.rs"
      provides: "Modul-Deklarationen + Re-Exports für alle 12 neuen Phase-4 Components"
      contains: "pub mod attendance_list, pub mod qr_scanner, pub mod helper_shell, pub mod assembly_status_badge"
  key_links:
    - from: "Plans 04+05+06 (Component-Files geschrieben ohne mod.rs-Touch)"
      to: "src/component/mod.rs (single-writer Plan 06b)"
      via: "append-only deklarations + re-exports"
      pattern: "pub mod (attendance_list|attendance_search|live_counter|connection_banner|manual_code_input|qr_scanner|qr_card|helper_shell|assembly_status_badge|assembly_list_row|tab_strip|toast)"
---

<objective>
Single-writer für `genossi-frontend/src/component/mod.rs`. Plans 04, 05, 06 schreiben ihre 12 neuen Component-Files OHNE mod.rs zu touchen — Plan 06b ist der einzige Plan, der mod.rs editiert. Damit laufen Plans 04, 05, 06 in Wave 2 ohne Merge-Konflikt parallel; Plan 06b läuft sequentiell danach in Wave 2.5.

Hintergrund: Der Plan-Checker hat (BLOCKER B-01) festgestellt, dass alle drei Wave-2-Plans dieselbe Datei `mod.rs` modifizieren würden — bei parallelen Executor-Branches eine garantierte Merge-Kollision. Lösung: Sequenzieller Append-Plan nach Wave 2.

Purpose: Saubere Wave-Serialisierung; Plans 07/08/09 können ihre Imports gegen die finalen Re-Exports schreiben.
Output: 1 File modifiziert (mod.rs).
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@/home/neosam/programming/rust/projects/genossi3/.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-04-PLAN.md
@/home/neosam/programming/rust/projects/genossi3/.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-05-PLAN.md
@/home/neosam/programming/rust/projects/genossi3/.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-06-PLAN.md
@/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mod.rs

<interfaces>
**Dependency-Files vorhanden** (Plan 06b verifiziert per `test -f` bevor mod.rs geschrieben wird):
- Plan 04: `attendance_list.rs`, `attendance_search.rs`, `live_counter.rs`, `connection_banner.rs`
- Plan 05: `manual_code_input.rs`, `qr_scanner.rs`, `qr_card.rs`, `helper_shell.rs`
- Plan 06: `assembly_status_badge.rs`, `assembly_list_row.rs`, `tab_strip.rs`, `toast.rs`

**Reuse-Erweiterung Plan 06 (W-04 Component-Extraction)** — Plan 06 wurde erweitert um drei zusätzliche Components, die Plan 08 nutzt: `token_row.rs`, `create_token_form.rs`, `basics_tab.rs`. Plan 06b deklariert auch diese in mod.rs falls die Files existieren.

**Existing mod.rs (Phase-3-Stand):** 26 `pub mod`-Deklarationen, 19 `pub use`-Re-Exports. Plan 06b ergänzt 12-15 (je nach W-04-Umfang) zusätzliche `pub mod`-Zeilen + entsprechende `pub use`.

**Kein Re-Order:** Plan 06b APPENDED neue Zeilen am Ende der existing-Listen. Kein Sortieren der existing-Einträge (würde unnötigen Diff erzeugen).
</interfaces>
</context>

<threat_model>
| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-04-43 | T (Tampering) | Race-Condition wenn 06b vor allen Wave-2-Plans gestartet wird | mitigate | depends_on: [04, 05, 06] in frontmatter; Executor-Wave-Logik garantiert Sequenz |
| T-04-44 | T (Tampering) | mod.rs-Diff zerstört existing Re-Exports | mitigate | append-only — kein sed über existing Zeilen; Plan-Discretion-Verifikation per `git diff genossi-frontend/src/component/mod.rs` post-Edit |
</threat_model>

<tasks>

<task type="auto">
  <name>Task 1: Append pub mod + pub use für alle Phase-4 Components</name>
  <files>
    genossi-frontend/src/component/mod.rs
  </files>
  <read_first>
    - /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mod.rs (existing 26 declarations + 19 re-exports — DO NOT TOUCH existing lines)
    - /home/neosam/programming/rust/projects/genossi3/.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-04-PLAN.md (Components: attendance_list, attendance_search, live_counter, connection_banner)
    - /home/neosam/programming/rust/projects/genossi3/.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-05-PLAN.md (Components: manual_code_input, qr_scanner, qr_card, helper_shell)
    - /home/neosam/programming/rust/projects/genossi3/.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-06-PLAN.md (Components: assembly_status_badge, assembly_list_row, tab_strip, toast — plus W-04 token_row, create_token_form, basics_tab)
  </read_first>
  <action>
    1. **Pre-flight: alle erwarteten Component-Files müssen existieren** — Plan 06b failed-fast wenn ein File aus Plans 04/05/06 fehlt (Wave-2-Plan inkomplett):
       ```bash
       set -e
       cd /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component
       for f in attendance_list.rs attendance_search.rs live_counter.rs connection_banner.rs \
                manual_code_input.rs qr_scanner.rs qr_card.rs helper_shell.rs \
                assembly_status_badge.rs assembly_list_row.rs tab_strip.rs toast.rs; do
           test -f "$f" || { echo "MISSING: $f"; exit 1; }
       done
       # W-04 Component-Extraction (Plan 06 erweitert) — optional, falls Plan 06 sie angelegt hat:
       for f in token_row.rs create_token_form.rs basics_tab.rs; do
           test -f "$f" && echo "W-04: $f exists" || echo "W-04: $f MISSING (Plan 06 must add)"
       done
       ```
       **Wenn ein W-04-File fehlt** und Plan 06 hat es laut frontmatter modifiziert: in den Verify-Block fail-fast einbauen.

    2. **Append `pub mod`-Deklarationen** am Ende des bestehenden `pub mod`-Blocks in `genossi-frontend/src/component/mod.rs`. Reihenfolge: erst Plan 04 dann 05 dann 06 (für Reviewer-Lesbarkeit):
       ```rust
       // ─── Phase 4 Plan 04 ─── shared attendance components ────────────
       pub mod attendance_list;
       pub mod attendance_search;
       pub mod live_counter;
       pub mod connection_banner;

       // ─── Phase 4 Plan 05 ─── helper login components ─────────────────
       pub mod manual_code_input;
       pub mod qr_scanner;
       pub mod qr_card;
       pub mod helper_shell;

       // ─── Phase 4 Plan 06 ─── vorstand layout components ──────────────
       pub mod assembly_status_badge;
       pub mod assembly_list_row;
       pub mod tab_strip;
       pub mod toast;
       ```
       **W-04 Conditionals (Plan 06 oder 06-Erweiterung):** Falls die Files existieren, ergänze:
       ```rust
       // ─── Phase 4 Plan 06 (W-04 extraction from assembly_details) ─────
       pub mod token_row;
       pub mod create_token_form;
       pub mod basics_tab;
       ```
       Verwende die Pre-flight-Checks aus Schritt 1 als Truth-Source.

    3. **Append `pub use`-Re-Exports** am Ende des bestehenden `pub use`-Blocks:
       ```rust
       // ─── Phase 4 Plan 04 ─── shared attendance components ────────────
       pub use attendance_list::AttendanceList;
       pub use attendance_search::AttendanceSearch;
       pub use live_counter::{LiveCounter, ConnState};
       pub use connection_banner::ConnectionBanner;

       // ─── Phase 4 Plan 05 ─── helper login components ─────────────────
       pub use manual_code_input::ManualCodeInput;
       pub use qr_scanner::QrScanner;
       pub use qr_card::QrCard;
       pub use helper_shell::HelperShell;

       // ─── Phase 4 Plan 06 ─── vorstand layout components ──────────────
       pub use assembly_status_badge::AssemblyStatusBadge;
       pub use assembly_list_row::AssemblyListRow;
       pub use tab_strip::{TabStrip, TabDef};
       pub use toast::{ToastContainer, show_toast};
       ```
       **W-04 Conditionals:** falls Files vorhanden, ergänze:
       ```rust
       pub use token_row::TokenRow;
       pub use create_token_form::CreateTokenForm;
       pub use basics_tab::BasicsTab;
       ```

    4. **Build verifizieren** — Frontend muss compilen sobald mod.rs aktualisiert ist:
       ```bash
       cd /home/neosam/programming/rust/projects/genossi3/genossi-frontend && cargo build 2>&1 | tail -3
       ```

    5. Commit: `feat(04-06b): wire Phase-4 components in mod.rs (single-writer post-Wave-2)`
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3/genossi-frontend && cargo build 2>&1 | tail -3 && grep -q "pub mod attendance_list" src/component/mod.rs && grep -q "pub mod attendance_search" src/component/mod.rs && grep -q "pub mod live_counter" src/component/mod.rs && grep -q "pub mod connection_banner" src/component/mod.rs && grep -q "pub mod manual_code_input" src/component/mod.rs && grep -q "pub mod qr_scanner" src/component/mod.rs && grep -q "pub mod qr_card" src/component/mod.rs && grep -q "pub mod helper_shell" src/component/mod.rs && grep -q "pub mod assembly_status_badge" src/component/mod.rs && grep -q "pub mod assembly_list_row" src/component/mod.rs && grep -q "pub mod tab_strip" src/component/mod.rs && grep -q "pub mod toast" src/component/mod.rs && grep -q "pub use attendance_list::AttendanceList" src/component/mod.rs && grep -q "pub use live_counter::{LiveCounter, ConnState}" src/component/mod.rs && grep -q "pub use tab_strip::{TabStrip, TabDef}" src/component/mod.rs && grep -q "pub use toast::{ToastContainer, show_toast}" src/component/mod.rs</automated>
  </verify>
  <done>mod.rs hat 12 neue pub-mod-Zeilen + entsprechende pub-use-Re-Exports; Frontend compiliert grün; W-04-Components (token_row/create_token_form/basics_tab) sind ergänzt falls Plan 06 sie angelegt hat (per Pre-flight-Check sicher).</done>
</task>

</tasks>

<verification>
```bash
cd /home/neosam/programming/rust/projects/genossi3/genossi-frontend && cargo build 2>&1 | tail -3

# Alle 12 Phase-4-Component-Module deklariert
for m in attendance_list attendance_search live_counter connection_banner \
         manual_code_input qr_scanner qr_card helper_shell \
         assembly_status_badge assembly_list_row tab_strip toast; do
    grep -q "pub mod $m;" /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mod.rs || echo "MISSING: $m"
done

# Kein .nest()-Konflikt im Frontend (Sanity, nicht Backend) — irrelevant hier; Plan 01 deckt Backend ab
```
</verification>

<success_criteria>
- mod.rs hat 12 neue `pub mod`-Deklarationen + 12 entsprechende `pub use`-Re-Exports
- Frontend `cargo build` grün
- W-04 token_row/create_token_form/basics_tab sind deklariert falls Plan 06 sie angelegt hat
- Plans 07/08/09 können `use crate::component::{AttendanceList, ...}` ungehindert verwenden
</success_criteria>

<output>
After completion, create `.planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-06b-SUMMARY.md`.
</output>
</content>
