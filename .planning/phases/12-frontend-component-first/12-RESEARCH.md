# Phase 12: Frontend (Component-First) - Research

**Researched:** 2026-06-01
**Domain:** Dioxus 0.6.3 WASM Frontend — RepaymentPhase/Entry Verwaltung
**Confidence:** HIGH (Reuse-Assets sind im Code verifiziert, Backend-Surface ist gelocked)

## Summary

Diese Phase ist ein REINER Frontend-Aufbau auf einem bereits voll verdrahteten Backend (Phasen 7-11). Alle in CONTEXT.md als "reusable" markierten Assets existieren tatsächlich an den dokumentierten Stellen und sind verifiziert (Datei-Lesen + grep). Das Reuse-Potenzial ist hoch:

- **`tab_strip.rs` existiert bereits** als generische Component (`component/tab_strip.rs`) — wurde in Phase-4-Plan-06 aus inline-RSX extrahiert und in `assembly_details.rs` produktiv eingesetzt. D-28-Fallback ("sonst neuer tab_strip.rs aus assembly_details.rs extrahieren") entfällt — direkt reused.
- **`MemberSearch`** hat die in CONTEXT.md beschriebenen Props (`on_select`/`selected_id`/`exclude_id`) — verifiziert in `component/member_search.rs:41-46`. Add-Entry-Modal kann es 1:1 reusen.
- **`Modal`** ist ein simpler Wrapper (`component/modal.rs`, 32 Zeilen) mit nur `children`-Prop — Add-Entry-/Confirm-Modals nutzen ihn als äußerer Container.
- **Toast-System** existiert in zwei Varianten: `ErrorAlert` (single, inline, dismiss-bar) und `ToastContainer + show_toast` (multi, auto-dismiss nach 5s). Phase 12 sollte für Mass-Operations-Fehler (D-04, D-17) `ToastContainer` reusen.
- **Backend-Surface ist exakt wie in CONTEXT.md beschrieben** — alle Routes, Query-Param-Konventionen, TO-Shapes und 409-Body-Schemas verifiziert in den genannten Dateien.

**Primary recommendation:** Direkter Reuse-First-Ansatz mit klarem Plan-Decomposition entlang UI-01..UI-06. Drei kritische Risiken: (1) Query-Param-Parsing in der `/mail`-Page muss neu gebaut werden (kein existierendes Pattern dafür im Frontend), (2) `editable_cell.rs` ist ein NEUER Component-Baustein ohne direkten 1:1-Anker im Code (member_details.rs nutzt Full-Page-Edit-Toggle, nicht Inline-Cell-Edit), (3) Kein WASM-Test-Infra → reine Logik-Tests via `cargo test -p genossi-frontend` sind das Maximum (member_search.rs zeigt das Pattern: pure `filter_members`-Funktion + `#[cfg(test)] mod tests`).

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Cross-Cutting — Button-Pattern (D-01/D-02):**
- Jeder neue Action-Button MUSS `r#type: "button"` explizit setzen
- `onclick` mit `MouseEvent`-Handler verwenden — NIE `<form onsubmit={...}>` mit `<button type="submit">`
- `<form>` nur für echte Form-Semantik (Enter-Submit auf Text-Input); sonst `<div>`
- Bei legitimen Forms: Handler synchron, `prevent_default()` zuerst, async `spawn` danach
- Grep-Gate (D-02): `rg 'button\s*\{' genossi-frontend/src/component/repayment_* genossi-frontend/src/page/repayment_*` darf KEINEN Treffer ohne `r#type:` haben

**Detail-Page Lifecycle-UX (D-03..D-09):**
- D-03: Lifecycle-Buttons im Stamm-Daten-Tab als Action-Kachel, NICHT im Page-Header
- D-04: 409 `CloseConflictResponse` → Toast (keine Auto-Tab-Switch); Pattern `error_alert.rs` reusen
- D-05: `share_value`-Korrektur (PHAS-04) inline im Stamm-Tab; 3 Render-Modi (Vorbereitung/Offen/Abgeschlossen); Pattern aus `member_details.rs`
- D-06: 'Vorbereitung'-Status — Tabs immer sichtbar, Einträge-/Export-Tab zeigen Hinweis-Box „Phase noch nicht geöffnet"
- D-07: 'Schließen' hat Confirm-Modal vor POST; 'Öffnen' KEIN Confirm
- D-08: Nach `Abgeschlossen` alle Felder read-only; Einträge-Tab ohne Edit/Delete/Toggle; Export-Tab voll aktiv
- D-09: Nach `Öffnen`-POST Page-Reload (neuer Status + version); KEIN Auto-Tab-Switch

**RepaymentEntryList Component (D-10..D-14):**
- D-10: 7 Spalten (Mitgliedsnummer, Name, Anteile, Betrag, IBAN, Status, Actions); Betrag = `share_count_to_pay_out × share_value` (Frontend rechnet, „60,00 €" deutsche Formatierung); IBAN-Spalte zeigt fehlende IBAN als „—"; Member-Join via `MEMBERS`-Global-Signal
- D-11: Multi-Select-Pattern — Per-Row-Checkbox links + Header-Checkbox; immer sichtbar (Tablet-tauglich); Header-Action-Leiste mit Count-Badges; Bulk-Buttons disabled bei 0 Selection
- D-12: Status-Filter als Tab-Strip-im-Tab: „Alle | Offen | Angeschrieben | Ausbezahlt" mit Count-Badges; client-side Filter
- D-13: `share_count_to_pay_out`-Inline-Cell-Edit; nur bei Status ∈ {offen, angeschrieben}; neuer Component `editable_cell.rs`
- D-14: Default-Sort Mitgliedsnummer ASC, Sekundär created ASC; Empty-State-Texte definiert; Soft-Delete via Trash-Icon mit Confirm; Status-Badge-Farben (Offen=grau, Angeschrieben=blau, Ausbezahlt=grün)

**ausbezahlt-Confirm + PaidOut-Flow (D-15..D-17):**
- D-15: Single-Endpoint im Backend (`POST /api/repayment-entry/{id}/mark-paid-out`); Frontend implementiert Bulk-Toggle als Sequential-Loop; ein Sammel-Confirm-Modal am Anfang; bei Fehler-in-der-Mitte: Toast „X von N erfolgreich, Y fehlgeschlagen"
- D-16: Confirm-Modal-Inhalt (Listentabelle, Gesamtsumme, 3-Punkt-Warnliste, roter „danger"-Style „Endgültig markieren")
- D-17: Backend-Validation-Fehler (PAYO-03) → Toast pro Entry, deutsche Mapping via `status_to_message`-Pattern

**Massenmail-Flow (D-18..D-20):**
- D-18: Trigger via Redirect zu `/mail?from=repayment&phase_id={uuid}&members={uuid,uuid,...}`; bestehende `/mail`-Page erweitern
- D-19: Repayment-Var-Buttons (`{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}`) erscheinen im `template_var_buttons.rs` nur bei `repayment_phase_id`-Kontext
- D-20: Status-Übergang `offen → angeschrieben` als separate manuelle Aktion via `/api/repayment-entry/batch-status` (target_status=Contacted); halbautomatisches Verbinden NICHT in Phase 12 (deferred)

**Add-Entry-Modal + Member-Picker (D-21..D-24):**
- D-21: `MemberSearch`-Component unverändert reused
- D-22: `share_count_to_pay_out`-Feld beim Member-Select mit `member.current_shares` vorbefüllt
- D-23: Client-Side-Validation minimal (`> 0`, Member ausgewählt); Submit-Button disabled bei Verletzung
- D-24: Add (`repayment_entry_add_modal.rs` mit Member-Picker) und Edit (Inline-Cell-Edit, D-13) sind zwei distinkte UI-Patterns

**Frontend-Routing & API-Client (D-25..D-27):**
- D-25: Neue Routes `/repayment-phases` (UI-01), `/repayment-phases/:id` (UI-02); Vorstand-only via `RequirePrivilege { privilege: "admin" }`
- D-26: Neue API-Funktionen in `api.rs` (Liste explizit); PDF-Export via `<a href="...">` (keine api.rs-Funktion); `AppError`/`status_to_message`-Pattern
- D-27: Neuer Menüpunkt „Anteils-Rückzahlung" in Vorstand-Nav-Group, zwischen „Anwesenheit" und „Mail" (oder Plan-Discretion)

**Tab-Component-Reuse (D-28):**
- Tab-Strip via existierende `tab_strip.rs`-Component — VERIFIZIERT als existierend (siehe Risiken-Sektion); Fallback-Klausel entfällt

### Claude's Discretion

- `repayment_phase_status_badge.rs`: analog `assembly_status_badge.rs`. Farben: Vorbereitung=grau, Offen=blau, Abgeschlossen=grün
- Listen-Page Default-Sort: `fiscal_year DESC, created DESC` (Phase-7 D-08); Sekundär-Sort Plan-Discretion
- Listen-Page Filter: vorerst keine; bei Schmerzgrenze nachziehen
- Modal-Component-Reuse: alle Add-/Confirm-Modals reusen `component/modal.rs`
- Toast-Pattern für API-Fehler: Phase 4 D-17 etabliert; reusen
- Anzeige Auto-Befüllung nach `Öffnen`: Empty-State erklärt N=0; Plan-Discretion auf Wortlaut
- i18n-Keys: Plan-Phase finalisiert die exakte Key-Liste; beide Locales (de/en) MÜSSEN gepflegt sein; UI-Default `Locale::De`

### Deferred Ideas (OUT OF SCOPE)

- Halbautomatische Status-Übergänge (`/mail?sent=true`-Banner) → nach v1.2
- `share_value`-Korrektur als separater Modal → falls UAT-Schmerzen zeigt, in Phase 13+
- Audit-Log-Verlinkung in Detail-Page nach `Abgeschlossen` → manuell über Top-Bar
- Listen-Page-Filter und Sort-Spalten → nach v1.2
- Bulk-`ausbezahlt` als Backend-atomar → out-of-scope (eigene Backend-Phase)
- Re-Open einer abgeschlossenen Phase → v2-Diskussion (PHAS-03 ist final)
- CSV-Export-Tab → EXPO-04 ist v2-deferred (Phase 11 D-12)
- WASM-Test-Suite (wasm-bindgen-test / Playwright) → out-of-scope v1.1
- Mobile-Layout-Optimierung → eigene Phase, falls je nötig

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UI-01 | Page `/repayment-phases` mit Liste aller Phasen | Backend-Liste verifiziert (`GET /api/repayment-phase`); Pattern-Anker: `assemblies.rs` für Listen-Layout + Create-Modal |
| UI-02 | Page `/repayment-phases/{id}` mit 3-Tab-Layout, Lifecycle-Aktionen | `tab_strip.rs` existiert; `assembly_details.rs` ist 1:1-Vorbild (3 Tabs + dynamischer 4. Tab) |
| UI-03 | Shared `RepaymentEntryList`-Component | Backend-Liste verifiziert (`GET /api/repayment-entry?phase_id=`); Multi-Select-Pattern aus `mail_page.rs` (`selected_member_ids`-Signal) reused |
| UI-04 | Add-Entry-Modal | `MemberSearch` direkt reuseable (Props in `component/member_search.rs:41-46`); Modal-Pattern in `modal.rs` |
| UI-05 | `ausbezahlt`-Confirm-Dialog | Backend-Endpoint `POST /api/repayment-entry/{id}/mark-paid-out` verifiziert; Modal-Pattern + Toast-Pattern beide etabliert |
| UI-06 | Massenmail-Aktion | Bestehender `/mail`-Page-Code (`page/mail_page.rs:189-541`) muss um Query-Param-Parsing + Repayment-Var-Buttons-Show-Condition erweitert werden |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Listen-/Detail-Rendering | Browser/Client (Dioxus WASM) | — | Reines UI; Backend liefert nur JSON-TOs |
| Daten-Aggregation Phase ↔ Entries ↔ Members | Browser (Client-Side-Join) | Backend (separate REST-Calls) | `MEMBERS`-Global-Signal ist bereits vorhanden; D-10 beschreibt Client-Side-Join explizit; keine neuen Backend-Aggregat-Endpoints nötig |
| Status-Filter (UI-03) | Browser (client-side, D-12) | — | Backend `GET /api/repayment-entry?phase_id=` liefert immer alle; Filter im Frontend |
| Multi-Select State | Browser (Signals) | — | Selection ist Page-Local; kein Global-State nötig (UI-03 ist Component, bekommt Selection via Props) |
| Lifecycle-Aktionen (open/close/PaidOut) | Backend (Atomare Cascade) | Browser (Confirm + POST) | Phase 9 D-12 + D-15: Backend macht atomic Cascade; Frontend triggert per Endpoint |
| `share_value`-Inline-Edit | Backend (Audit-Trail über `audited_update!`) | Browser (Edit-UI + PUT) | PHAS-04 ist auditpflichtig; Frontend ist nur Eingabe-Schicht |
| PDF-Download | Browser (`<a href=...>`) | Backend (Streaming) | Browser-native Content-Disposition; keine api.rs-Funktion (D-26) |
| Auth-Gate | Browser (`RequirePrivilege`) | Backend (OIDC + Permission-Funnel) | Defense-in-Depth: Frontend zeigt nicht; Backend würde sowieso 401/403 |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Dioxus | 0.6.3 | Reactive WASM-UI-Framework | Verifiziert in `genossi-frontend/Cargo.toml`; identisch mit Phase-4-Stack |
| dioxus-router | 0.6 (features=["router"]) | SPA-Routing über Route-Enum | Verifiziert: `#[route(...)]`-Macros in `router.rs:24-69`; Link-Component für Navigation |
| Tailwind CSS | latest (npm watch) | Utility-CSS | Verifiziert: `tailwind.css` als Output, Watch-Mode in `genossi-frontend/CLAUDE.md` |
| gloo-timers | 0.3 | setTimeout/setInterval-Brücke | Verifiziert: `TimeoutFuture` in `member_search.rs:66`, `toast.rs:24` |
| uuid | 1.6 | Entity-IDs | Verifiziert: alle TOs nutzen `Uuid` |
| reqwest | 0.11/0.12 | HTTP-Client | Verifiziert in `api.rs:185-220` |
| serde / serde_json | 1.0 | (De-)Serialisierung | Verifiziert in allen TOs |
| time | 0.3 | DateTime-Handling | Verifiziert in `MemberTO`, `RepaymentPhaseTO` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dioxus-logger | 0.6.2 | Browser-Console-Logging | Bei Phase-12-Debug-Output |
| wasm-bindgen / web-sys | 0.2/0.3 | JS-Interop | Für `window.location.search`-Parsing in `/mail`-Page-Erweiterung (D-18) |
| `urlencoding` (Plan-Discretion) | optional | URL-Param-Parsing | Für Query-Param-Split in `/mail`-Page; alternativ manuell via `web_sys::UrlSearchParams` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom Inline-Cell-Edit | dioxus-form-Library | Keine etablierte; Inline-Edit ist 30 Zeilen Component, kein Bedarf |
| Custom Multi-Select | dioxus-table-Library | Keine im Codebase; Pattern aus `mail_page.rs` reuseable als handgemachtes Pattern |
| Manuelle Query-Param-Parsing | `dioxus-router` Query-Param-Support | `dioxus-router` 0.6 hat Query-Param-Support; aber `/mail`-Page existiert bereits ohne — Pragmatic: `web_sys::window().location().search()` plus String-Split. **Plan-Discretion** für die Wahl. |

**Installation:** Keine neuen Dependencies nötig. Phase 12 lebt voll mit dem existierenden Stack.

**Version verification:**
```bash
grep -E "^dioxus|^tailwind|^gloo-timers" genossi-frontend/Cargo.toml
# dioxus = { version = "0.6.3", features = ["web", "router"] }
# dioxus-logger = "0.6.2"
```

[VERIFIED: genossi-frontend/Cargo.toml]

## Architecture Patterns

### System Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│ Browser (WASM)                                                   │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ Router (router.rs) - Route-Enum                        │     │
│  │ + RepaymentPhases, RepaymentPhaseDetails { id }        │     │
│  └────────────────┬────────────────────────────────────────┘     │
│                   │                                              │
│    ┌──────────────┴───────────────┐                              │
│    │                              │                              │
│  ┌─▼────────────┐         ┌──────▼────────────┐                  │
│  │ Liste-Page   │         │ Detail-Page       │                  │
│  │ (UI-01)      │         │ (UI-02)           │                  │
│  │ + Create-    │         │ + TabStrip        │                  │
│  │   Modal      │         │   (existing)      │                  │
│  └──────────────┘         └─┬─────────────────┘                  │
│                             │                                    │
│              ┌──────────────┼──────────────┬─────────────┐       │
│              │              │              │             │       │
│           ┌──▼──┐      ┌────▼──────┐   ┌───▼────┐   ┌────▼───┐  │
│           │Stamm│      │Einträge   │   │Export  │   │Confirm-│  │
│           │Tab  │      │Tab        │   │Tab     │   │Modals  │  │
│           │+    │      │+          │   │PDF-DL  │   │(D-07,  │  │
│           │Life-│      │Repayment- │   │Anker   │   │D-15)   │  │
│           │cycle│      │EntryList  │   │        │   │        │  │
│           │     │      │(UI-03)    │   │        │   │        │  │
│           │+    │      │+ Inline-  │   │        │   │        │  │
│           │share│      │  Cell-Edit│   │        │   │        │  │
│           │value│      │  (D-13)   │   │        │   │        │  │
│           │edit │      │+ Add-     │   │        │   │        │  │
│           │D-05 │      │  Modal    │   │        │   │        │  │
│           └─────┘      │  (UI-04)  │   │        │   │        │  │
│                        │+ Multi-   │   │        │   │        │  │
│                        │  Select   │   │        │   │        │  │
│                        │  → Mail-  │   │        │   │        │  │
│                        │  Redirect │   │        │   │        │  │
│                        │  (UI-06)  │   │        │   │        │  │
│                        │+ Paid-Out │   │        │   │        │  │
│                        │  Confirm  │   │        │   │        │  │
│                        │  (UI-05)  │   │        │   │        │  │
│                        └───────────┘   └────────┘   └────────┘  │
│                                                                  │
│  Globale Signals:                                                │
│    MEMBERS (state/member.rs) — read für Client-Side-Join D-10    │
│    AUTH    (service/auth.rs)  — RequirePrivilege "admin"         │
│    CONFIG  (service/config.rs) — backend URL                     │
│    I18N    (i18n/mod.rs) — Locale::De default                    │
│                                                                  │
└────────────────┬─────────────────────────────────────────────────┘
                 │ reqwest (api.rs) - check_response + map_response_error
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ Backend REST (genossi_rest) — gelocked, keine Änderungen         │
│                                                                  │
│  /api/repayment-phase          (genossi_rest/src/repayment_phase.rs:337)  │
│    GET /, POST /, GET /{id}, PUT /{id}, DELETE /{id}             │
│    POST /{id}/open, POST /{id}/close                             │
│                                                                  │
│  /api/repayment-entry          (genossi_rest/src/repayment_entry.rs:361) │
│    POST /batch-status                                            │
│    GET /?phase_id=<uuid>, POST /                                 │
│    GET /{id}, PUT /{id}, DELETE /{id}                            │
│    POST /{id}/mark-paid-out                                      │
│                                                                  │
│  /api/repayment-phase/{phase_id}/export/{format}?include=open|all|paid  │
│                                (genossi_rest/src/repayment_export.rs:163) │
│                                                                  │
│  /api/mail/send-bulk           (genossi_mail/src/rest.rs - Phase 10) │
│    + Optional template_id + repayment_phase_id Body-Felder       │
│                                                                  │
│  /api/members                  (Phase-4-Bestand)                 │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
genossi-frontend/src/
├── page/
│   ├── repayment_phases.rs              # UI-01 Liste + Create-Modal
│   └── repayment_phase_details.rs       # UI-02 3-Tab-Layout
├── component/
│   ├── repayment_phase_status_badge.rs  # Status-Badge (Vorbereitung/Offen/Abgeschlossen)
│   ├── repayment_entry_status_badge.rs  # Status-Badge (Offen/Angeschrieben/Ausbezahlt)
│   ├── repayment_entry_list.rs          # UI-03 Kern-Component
│   ├── repayment_entry_add_modal.rs     # UI-04 Modal mit Member-Picker
│   ├── repayment_entry_paidout_confirm.rs # UI-05 Bulk-Confirm-Modal
│   └── editable_cell.rs                  # D-13 (oder spezialisiert editable_share_count_cell.rs)
├── api.rs                                # +12 API-Funktionen (D-26)
├── router.rs                             # +2 Routes (D-25)
├── i18n/
│   ├── mod.rs                            # +Keys (Phase-12-Strings)
│   ├── de.rs                             # +Keys deutsche Übersetzungen
│   └── en.rs                             # +Keys englische Übersetzungen
└── component/
    ├── nav_group.rs                      # unverändert
    └── top_bar.rs                        # +NavItem für „Anteils-Rückzahlung" (D-27)

# Zu erweiternde existierende Dateien:
├── component/mail_compose/template_var_buttons.rs # +Repayment-Var-Buttons (D-19)
└── page/mail_page.rs                              # +Query-Param-Parsing (D-18)
```

### Pattern 1: Component-First / Reuse-Anker

**What:** Jede UI-Element-Familie, die mehr als einmal vorkommt, ist eine eigene `#[component]` in `src/component/`. Verletzung = Memory-Violation (`feedback_component_first.md`).

**When to use:** IMMER. Phase 4 Plan 06 hat das Pattern etabliert (TabStrip, ToastContainer, AssemblyStatusBadge wurden aus inline-RSX extrahiert).

**Example:**
```rust
// Source: genossi-frontend/src/component/assembly_status_badge.rs:33-39
#[component]
pub fn AssemblyStatusBadge(status: AssemblyStatusTO) -> Element {
    let i18n = use_i18n();
    let label = status_label(&i18n, &status);
    let class = status_badge_class(&status);
    rsx! { span { class: "{class}", "{label}" } }
}
```

[VERIFIED: genossi-frontend/src/component/assembly_status_badge.rs]

### Pattern 2: Tab-Layout via existierendes `TabStrip`

**What:** Generic Tab-Strip mit `TabDef { key, label }`-Liste und `EventHandler<String>`-Callback. Body wird als `children`-Prop übergeben.

**When to use:** Detail-Page (UI-02) — D-28 sagt explizit reusen.

**Example:**
```rust
// Source: genossi-frontend/src/component/tab_strip.rs:16-50
#[component]
pub fn TabStrip(
    tabs: Vec<TabDef>,
    active_key: String,
    on_change: EventHandler<String>,
    children: Element,
) -> Element { /* ... */ }

// Aufrufer-Pattern aus assembly_details.rs:83-150:
let mut active_tab = use_signal(|| "basics".to_string());
let tab_defs = vec![
    TabDef { key: "basics", label: i18n.t(Key::AssemblyTabBasics).to_string() },
    TabDef { key: "tokens", label: i18n.t(Key::AssemblyTabTokens).to_string() },
    TabDef { key: "attendance", label: i18n.t(Key::AssemblyTabAttendance).to_string() },
];
// Dynamic Tabs: Push 4. Tab nur wenn Status == Closed
if matches!(a.status, AssemblyStatusTO::Closed) {
    tab_defs.push(TabDef { key: "export", label: i18n.t(Key::AssemblyTabExport).to_string() });
}
let active_key = active_tab.read().clone();
rsx! {
    TabStrip {
        tabs: tab_defs,
        active_key: active_key.clone(),
        on_change: move |k: String| active_tab.set(k),
        match active_key.as_str() {
            "basics" => rsx! { BasicsTab { ... } },
            "tokens" => rsx! { TokensTab { ... } },
            // ...
            _ => rsx! { }
        }
    }
}
```

[VERIFIED: genossi-frontend/src/component/tab_strip.rs, genossi-frontend/src/page/assembly_details.rs:83-150]

**Phase-12-Anwendung:** Detail-Page (UI-02) hat IMMER 3 Tabs (Stamm/Einträge/Export, D-06 — kein dynamischer 4. Tab). Tab-Body branch via `match active_key.as_str()`.

### Pattern 3: Toast-Pattern für Massen-Operations-Fehler

**What:** Two-Layer-Toast-System.
- **Einzel-Fehler / Banner:** `ErrorAlert` (single, inline, mit „Details anzeigen"-Expand) — siehe `mail_page.rs:208-213`.
- **Multi-Toast / Auto-Dismiss:** `ToastContainer` + `show_toast`-Helper, Auto-Dismiss nach 5s — siehe `assembly_details.rs:158`.

**When to use:**
- Phase-12 D-04 (CloseConflictResponse) → ErrorAlert oder show_toast — beides funktioniert; Plan-Discretion.
- Phase-12 D-17 (PaidOut-Loop pro-Entry-Fehler) → show_toast (mehrere Fehler möglich, ToastContainer kann sie stapeln).

**Example:**
```rust
// Source: genossi-frontend/src/component/toast.rs:14-27
pub fn show_toast(
    toast_messages: &mut Signal<Vec<(u64, String)>>,
    toast_counter: &mut Signal<u64>,
    msg: String,
) {
    let id = *toast_counter.read();
    *toast_counter.write() += 1;
    toast_messages.write().push((id, msg));
    let mut toast_messages = toast_messages.clone();
    spawn(async move {
        TimeoutFuture::new(5_000).await;
        toast_messages.write().retain(|(tid, _)| *tid != id);
    });
}

// Aufrufer (assembly_details.rs:44-58):
let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
let mut toast_counter = use_signal(|| 0u64);
// ...
Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
// ...
ToastContainer { messages: toast_messages }
```

[VERIFIED: genossi-frontend/src/component/toast.rs:14-47, genossi-frontend/src/page/assembly_details.rs:44-58,158]

### Pattern 4: API-Call mit `AppError` + `status_to_message`

**What:** `async fn …(config: &Config, …) -> Result<T, AppError>`. Bei nicht-2xx wird `map_response_error` aufgerufen, das `status_to_message(status)` als deutsche Meldung mit optionalem Body-Detail nutzt.

**Example:**
```rust
// Source: genossi-frontend/src/api.rs:73-87 (status_to_message)
fn status_to_message(status: u16) -> &'static str {
    match status {
        400 => "Ungültige Anfrage",
        401 => "Keine Berechtigung — bitte erneut anmelden",
        403 => "Keine Berechtigung für diese Aktion",
        404 => "Nicht gefunden",
        409 => "Konflikt — das Element wurde zwischenzeitlich geändert",
        410 => "Bereits eingelöst",
        // ...
        _ => "Unbekannter Fehler",
    }
}

// Source: genossi-frontend/src/api.rs:199-209 (Standard-Pattern für POST)
pub async fn create_member(config: &Config, member: MemberTO) -> Result<MemberTO, AppError> {
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::Client::new().post(url).json(&member).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}
```

[VERIFIED: genossi-frontend/src/api.rs:73-87,199-209]

**Phase-12-Anwendung:** Neue 12 Funktionen in `api.rs` folgen genau diesem Pattern. Für strukturierte 409-Bodies (`CloseConflictResponse`, `BatchFailureResponse` — D-04, D-15): API-Function liefert `AppError`; Caller deserialisiert `error.detail` als JSON für die strukturierte Anzeige. Pattern-Note: `AppError.detail` ist `Option<String>` mit dem rohen Body; Plan-Discretion ob die API-Function einen typed Variant zurückgibt oder der Caller mit `serde_json::from_str::<CloseConflictResponse>(&detail)` arbeitet.

### Pattern 5: Globaler State via `GlobalSignal` + Refresh-Coroutine

**What:** Singleton-State (z.B. `MEMBERS`) wird via `GlobalSignal::new(Default::default)` deklariert und durch `async fn refresh_*()` befüllt.

**Example:**
```rust
// Source: genossi-frontend/src/service/member.rs:7-25
pub static MEMBERS: GlobalSignal<MemberState> = Signal::global(MemberState::default);

pub async fn refresh_members() {
    MEMBERS.write().loading = true;
    let config = CONFIG.read().clone();
    match get_members(&config).await {
        Ok(members) => {
            MEMBERS.write().items = members;
            MEMBERS.write().error = None;
        }
        Err(e) => {
            MEMBERS.write().error = Some(format!("Failed to load members: {}", e));
        }
    }
    MEMBERS.write().loading = false;
}
```

[VERIFIED: genossi-frontend/src/service/member.rs:7-25]

**Phase-12-Anwendung:** Plan-Discretion: Entweder neuer `REPAYMENT_PHASES`-Global-Signal (analog `MEMBERS`) ODER lokale `use_resource(...)`-Hooks pro Page. Vorschlag: lokale `use_resource` reicht für Phase 12 (Phasen sind nicht so „omnipresent" wie Members). Aber: `MEMBERS` MUSS nach jedem `mark-paid-out` invalidiert/refreshed werden, weil `Member.current_shares` sich ändert (Specifics, Punkt „current_shares Aktualität"). → einfacher Aufruf `refresh_members().await` nach Bulk-PaidOut-Loop.

### Pattern 6: Auth-Wrapper

**What:** `RequirePrivilege { privilege: "admin", fallback: ..., children: ... }` aus `auth.rs:35-48`. Lädt `AUTH`-Signal, prüft Privilege, rendert children oder fallback.

**Example:**
```rust
// Source: genossi-frontend/src/auth.rs:34-48
#[component]
pub fn RequirePrivilege(props: RequirePrivilegeProps) -> Element {
    let auth = AUTH.read().clone();
    match auth.auth_info {
        Some(auth_info) if auth_info.has_privilege(props.privilege) => props.children,
        _ => props.fallback.unwrap_or_else(|| { /* default Access Denied */ }),
    }
}

// Pattern aus mail_page.rs:190-193:
rsx! {
    RequirePrivilege {
        privilege: "admin",
        fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                // page content
            }
        }
    }
}
```

[VERIFIED: genossi-frontend/src/auth.rs:34-48, genossi-frontend/src/page/mail_page.rs:190-193]

### Anti-Patterns to Avoid

- **Inline-RSX-Duplikate über mehrere Pages** → Component-First-Verletzung (Memory). Extract-Trigger: Wenn die gleiche `rsx! { ... }`-Struktur in zwei Pages erscheint, wandert sie nach `src/component/`.
- **`<button type="submit">` mit `onclick` ohne `r#type:`** → triggert Page-Reload trotz `prevent_default` (D-01 / Hotfix e245013). Test: Grep-Gate D-02 in Plan-Acceptance.
- **`<form>` mit `onsubmit` + async `spawn`** → Hotfix c6f41fd lehrt: `<form>` → `<div>` umbauen, außer echte Form-Semantik ist nötig (Enter-Submit). Bei Forms: `prevent_default()` MUSS synchron VOR `spawn(async)` laufen.
- **Hard-Delete im Frontend** → Backend hat soft-delete via `PUT` mit `deleted`-Timestamp. Frontend ruft NUR `DELETE /api/repayment-entry/{id}` (das Backend mapped intern auf soft-delete). Pattern aus `delete_member` in api.rs:220-226.
- **Inline-Recompute statt `From<&Member>`-Impl** → Member-Daten sollten immer über das `MEMBERS`-Global-Signal kommen, nicht jedes Mal via API neu geladen.
- **Stale `version`-UUID** → bei jeder PUT/POST muss die aktuelle `version` aus dem letzten GET kommen. Bei 409 → Reload des Records, neue version verwenden.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Modal-Container | Custom `<div class="fixed">`-Wrapper inline | `component/modal.rs::Modal` | Existiert, 32 Zeilen, Styling konsistent |
| Tab-Strip | Inline-`<button>`-Liste | `component/tab_strip.rs::TabStrip` | D-28 sagt reusen; Phase-4-Anker |
| Member-Picker mit Substring-Suche | Custom-Search | `component/member_search.rs::MemberSearch` | D-21 sagt reusen; alle Props passen für UI-04 |
| Status-Badge-Styling | Inline Tailwind-Klassen | Neuer `repayment_*_status_badge.rs` analog `assembly_status_badge.rs` | Component-First; Vorlage da |
| Toast-System | Eigenes State-Management | `component/toast.rs::ToastContainer + show_toast` | Multi-Toast + 5s-Auto-Dismiss + key-Stable |
| Error-Banner | Custom-Red-Box | `component/error_alert.rs::ErrorAlert` | Mit Detail-Expand + Dismiss-Button |
| API-Error-Mapping | Custom Status-Code-Switch | `api.rs::map_response_error` + `status_to_message` | Deutsch-lokalisiert, mit `detail`-Body-Pass-Through |
| Auth-Gate | Custom Permission-Check | `auth.rs::RequirePrivilege { privilege: "admin" }` | D-25; etablierter Pattern |
| Member-Liste-State | Per-Page-Refetch | `state/member.rs::MEMBERS`-Global-Signal | Existiert; `refresh_members()` als zentraler Reload-Punkt |
| i18n-Translations | Hardcoded deutsche Strings im RSX | `i18n::Key`-Enum + `i18n.t(...)` | Beide Locales pflichtig; Pattern eingespielt |
| DateTime-Formatierung | Custom `format!`-Calls | `i18n.format_datetime(iso_str)` + `i18n.format_price(cents)` | ISO8601-Parser + DE/EN-Lokalisierung in `i18n/mod.rs:633-720` |
| URL-Search-Params-Parsing (D-18) | String-Split-by-Hand | `web_sys::UrlSearchParams::new_with_str(window.location().search().as_str())` | Browser-API; nullen Allocator-Overhead; Plan-Discretion zwischen `web_sys` vs. `dioxus-router` Query-Param-Support |

**Key insight:** Die Codebase hat in Phase 4 (Plan 06) eine konsequente Component-Extraction durchgeführt. Phase 12 muss diese Reuse-Disziplin fortsetzen — alle aufgelisteten Reuse-Targets sind verifiziert vorhanden. Verstoß = Memory-Violation `feedback_component_first.md`.

## Backend REST Surface — VERIFIED

[VERIFIED via direct file read]

### `/api/repayment-phase` — `genossi_rest/src/repayment_phase.rs:337-351`

| Method | Path | Body | Response |
|--------|------|------|----------|
| GET | `/` | — | `Vec<RepaymentPhaseTO>` |
| POST | `/` | `CreateRepaymentPhaseRequest { fiscal_year: i32, share_value: i64 }` | `RepaymentPhaseTO` |
| GET | `/{id}` | — | `RepaymentPhaseTO` |
| PUT | `/{id}` | `UpdateRepaymentPhaseRequest { fiscal_year, share_value, version: Uuid }` | `RepaymentPhaseTO` |
| DELETE | `/{id}` | — | `204 No Content` (soft-delete via PUT mit `deleted`) |
| POST | `/{id}/open` | — | `RepaymentPhaseTO` (status=Open, opened_at=now) |
| POST | `/{id}/close` | — | `RepaymentPhaseTO` (status=Closed) OR `409 + CloseConflictResponse` |

### `/api/repayment-entry` — `genossi_rest/src/repayment_entry.rs:361-384`

| Method | Path | Body / Query | Response |
|--------|------|--------------|----------|
| GET | `/?phase_id=<uuid>` | Query `phase_id` (Pflicht, D-10) | `Vec<RepaymentEntryTO>` |
| POST | `/` | `CreateRepaymentEntryRequest { phase_id, member_id, share_count_to_pay_out }` | `RepaymentEntryTO` |
| POST | `/batch-status` | `BatchStatusRequest { entry_ids: Vec<Uuid>, target_status: RepaymentEntryStatusTO }` (PaidOut → 400) | 200 OR 404 (missing) OR 409 + `BatchFailureResponse` |
| GET | `/{id}` | — | `RepaymentEntryTO` |
| PUT | `/{id}` | `UpdateRepaymentEntryRequest { share_count_to_pay_out: Option<i32>, status: Option<RepaymentEntryStatusTO>, version: Uuid }` | `RepaymentEntryTO` |
| DELETE | `/{id}` | — | `204 No Content` (soft-delete) |
| POST | `/{id}/mark-paid-out` | — | `RepaymentEntryTO` OR 400 (PAYO-03 ValidationError) OR 409 (PAYO-04 schon ausbezahlt) |

### `/api/repayment-phase/{phase_id}/export/{format}` — `genossi_rest/src/repayment_export.rs:163-169`

| Method | Path | Query | Response |
|--------|------|-------|----------|
| GET | `/{phase_id}/export/pdf` | `?include=open\|all\|paid` (Default: open) | PDF-Stream mit `Content-Disposition: attachment; filename="auszahlung-{fiscal_year}-{include}.pdf"` |

### TO-Schemas — `genossi_rest_types/src/lib.rs`

**`RepaymentPhaseTO`** (Z. 1186-1221):
```rust
pub struct RepaymentPhaseTO {
    pub id: Uuid,
    pub fiscal_year: i32,
    pub share_value: i64,  // cents
    pub status: RepaymentPhaseStatusTO,  // Preparation | Open | Closed
    pub opened_at: Option<PrimitiveDateTime>,
    pub closed_at: Option<PrimitiveDateTime>,
    pub created: Option<PrimitiveDateTime>,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Option<Uuid>,
}
```

**`RepaymentEntryTO`** (Z. 1309-1334):
```rust
pub struct RepaymentEntryTO {
    pub id: Uuid,
    pub member_id: Uuid,
    pub phase_id: Uuid,
    pub share_count_to_pay_out: i32,
    pub status: RepaymentEntryStatusTO,  // Open | Contacted | PaidOut
    pub created: Option<PrimitiveDateTime>,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Option<Uuid>,
}
```

**`CloseConflictResponse`** (Z. 1390-1396) — Backend liefert das im 409-Body bei `POST /{id}/close`:
```rust
pub struct CloseConflictResponse {
    pub error: String,
    pub pending_count: usize,
    pub pending_member_numbers: Vec<String>,  // up to 20, then "+N weitere"
}
```

**`BatchFailureResponse`** (Z. 1414-1422) — Backend liefert das im 409-Body bei `POST /batch-status` bei Domain-Conflict (nicht NotFound — das ist 404):
```rust
pub struct BatchFailureResponse {
    pub failure_index: usize,
    pub failure_id: String,
    pub failure_reason: String,
}
```

**Frontend muss diese Strukturen lokal deklarieren** (z.B. in `api.rs` als `CloseConflictResponse` und `BatchFailureResponse`) — `genossi_rest_types` ist nicht im Frontend-Path (Frontend hat sein eigenes `rest_types`-Crate). Plan-Phase muss klären: Sollen die Strukturen in `rest-types/` ergänzt werden (das ist das Frontend-shared Crate) oder lokal in `api.rs` als `#[derive(Deserialize)]`-Structs definiert werden? **Plan-Discretion** — Pragmatic: lokal in api.rs deklarieren (analog `MailJobTO` in `api.rs:819-829`).

## Common Pitfalls

### Pitfall 1: Button Page-Reload-Bug (D-01)

**What goes wrong:** `<button onclick={...}>` in einem `<form>`-Container ohne explizites `r#type: "button"` triggert beim Klick einen Browser-Form-Submit + Reload. Auch mit `e.prevent_default()` im handler.

**Why it happens:** HTML-Default-Button-Type ist `submit`. In Dioxus 0.6.3 wird der `prevent_default()` im async-Handler erst NACH dem Form-Submit ausgeführt.

**How to avoid:** Memory-Lock `feedback_dioxus_button_type.md` + D-01: ALLE `button { ... }`-Tags brauchen `r#type: "button"`. Bei legitimen Forms: synchroner Handler mit `e.prevent_default()` VOR `spawn(async ...)`.

**Warning signs:** Page reloaded nach Klick auf einen Action-Button im Formular. URL bekommt Query-Strings angehängt. Frontend-State geht verloren.

**Plan-Acceptance-Test (D-02 Grep-Gate):**
```bash
# Pre-Merge-Check: Suche Buttons OHNE r#type: in den neuen Phase-12-Dateien.
# Multi-Line: grep mit -A1, dann negativer match auf r#type:
rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' \
   genossi-frontend/src/component/repayment_*.rs \
   genossi-frontend/src/page/repayment_*.rs \
| grep -v 'r#type:' \
| grep 'button {'
# Erwartet: 0 Treffer (jeder button MUSS r#type: haben).
```

Pragmatische Variante (weniger präzise, aber im Plan einfacher zu prüfen):
```bash
# Anzahl button {-Vorkommen
COUNT_BUTTONS=$(rg -c 'button\s*\{' genossi-frontend/src/component/repayment_*.rs genossi-frontend/src/page/repayment_*.rs 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
COUNT_TYPED=$(rg -c 'r#type:\s*"button"' genossi-frontend/src/component/repayment_*.rs genossi-frontend/src/page/repayment_*.rs 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "buttons=$COUNT_BUTTONS typed=$COUNT_TYPED"
# Acceptance: COUNT_BUTTONS == COUNT_TYPED
```

**Negative-Control:** Phase-4-Dateien (z.B. `page/assembly_details.rs`) — Verify per Grep dass dort COUNT_BUTTONS == COUNT_TYPED ebenfalls hält. Falls nicht: Phase-12-Gate würde False-Positives produzieren; dann ist die Grep-Spec zu lockern oder zu präzisieren.

### Pitfall 2: Optimistic-Locking Stale Version (409)

**What goes wrong:** Frontend lädt RepaymentPhase, User editiert `share_value`, sendet `PUT` mit der alten `version`. Backend antwortet 409 `Version mismatch`.

**Why it happens:** Backend DAO bumpt `version` bei jedem UPDATE (atomare neue UUID). Frontend hat die alte Version aus dem GET im Speicher.

**How to avoid:** Bei 409 → automatischer Reload via `get_repayment_phase(id)`, neue `version` ins lokale State übernehmen, dem User Hinweis „Daten wurden zwischenzeitlich geändert — bitte erneut speichern". Pattern aus Phase 8 Plan 10 CR-01-Regression. Im Frontend: `error.status == Some(409)` → reload + status_to_message-Toast „Konflikt — das Element wurde zwischenzeitlich geändert" (`api.rs:79`).

**Warning signs:** Wiederholtes 409 ohne dass ein echter Konflikt vorliegt → vermutlich wird nach POST/PUT die Response-`version` NICHT zurück in den lokalen State geschrieben.

### Pitfall 3: Member-Daten-Konsistenz nach `mark-paid-out`

**What goes wrong:** Bulk-PaidOut-Loop ändert N `Member.current_shares` im Backend. `MEMBERS`-Global-Signal im Frontend ist stale; Folge-Inline-Edits nutzen alten `current_shares` als Default.

**Why it happens:** `MEMBERS` wird nur über `refresh_members()` neu geladen.

**How to avoid:** Nach der Bulk-PaidOut-Loop einmal `refresh_members().await` aufrufen (D-15 Loop + 1× Refresh). Plan-Discretion: per-Toggle-Refresh ist auch möglich, aber teurer.

**Warning signs:** Nach PaidOut zeigt die Member-Liste in der Sidebar/anderen Pages noch die alten Anteile.

### Pitfall 4: Query-Param-Parsing in `/mail`-Page (D-18)

**What goes wrong:** Frontend-Routing in `dioxus-router` 0.6 hat zwar Query-Param-Support, aber die existierende `MailPage`-Component nutzt das NICHT (Routes haben keine Query-Param-Felder in `router.rs:53` — nur `#[route("/mail")]`).

**Why it happens:** Phase 4 hat die /mail-Page ohne Query-Params entworfen.

**How to avoid:** Zwei Optionen:
1. **`web_sys::window().location().search()` + manuelle String-Split** im `use_effect` der MailPage. Pragmatic, kein dioxus-router-Refactor nötig.
2. **Route-Enum erweitern:** `#[route("/mail?:from&:phase_id&:members")]` + entsprechende `MailPage(props…)`-Signatur — dioxus-router 0.6 unterstützt das. Cleaner, aber breaking change auf MailPage.

**Plan-Discretion** — Vorschlag: Option 1 (web_sys) für minimal-invasiven Diff. Beispiel:
```rust
use_effect(move || {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            // search = "?from=repayment&phase_id=...&members=u1,u2"
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                if let Some(phase_id_str) = params.get("phase_id") {
                    // ... Pre-Selection setzen
                }
                if let Some(members_str) = params.get("members") {
                    let ids: Vec<Uuid> = members_str.split(',').filter_map(|s| Uuid::parse_str(s.trim()).ok()).collect();
                    selected_member_ids.set(ids);
                }
            }
        }
    }
});
```

**Warning signs:** Query-Param-Parsing in WASM ist tricky bei URL-Encoding (UUIDs mit Bindestrichen sind URL-safe, aber Komma-Listen brauchen ggf. URL-Encoding). Tests: e2e über echtes Browser-Klick.

### Pitfall 5: `editable_cell.rs` ist ein NEUER Pattern

**What goes wrong:** D-13 verlangt Inline-Cell-Edit. Aber `member_details.rs` nutzt KEIN Inline-Cell-Edit — die Page hat ein Page-Level `edit_mode`-Toggle, das ALLE Felder gleichzeitig editierbar macht (siehe `member_details.rs:495-548`: `input { oninput: move |e| { member.write().last_name = e.value().clone(); } }` — Direct-Binding im Edit-Modus).

**Why it happens:** CONTEXT.md D-05 sagt „Pattern-Vorlage: editable Felder in `member_details.rs`" — das gilt für das Stamm-Tab-`share_value`-Edit (single field), NICHT für Inline-Cell-Edit in einer Tabelle.

**How to avoid:** `editable_cell.rs` ist ein neuer Component-Baustein. API-Vorschlag:
```rust
#[component]
pub fn EditableCell(
    value: i32,                     // initial value
    on_save: EventHandler<i32>,     // commit handler
    disabled: bool,                 // read-only when status == PaidOut
) -> Element {
    let mut editing = use_signal(|| false);
    let mut local_value = use_signal(|| value);
    // Click → editing=true → input + ✓ ✗ Buttons
    // ✓ → on_save.call(*local_value.read()) → editing=false
    // ✗ → local_value.set(value) → editing=false
}
```

Alternativ spezialisiert: `EditableShareCountCell` mit hardcoded `i32`-Validator und Status-Awareness. **Plan-Discretion.**

**Warning signs:** Plan-Phase muss explizit den Pattern-Anker dokumentieren (NICHT „wie member_details" sagen), sonst implementiert der Developer ein Page-Level-Edit-Toggle statt eines Inline-Cell-Edit.

### Pitfall 6: Tab-Body-State bei Tab-Switch

**What goes wrong:** Inline-Edit-State in der Einträge-Tab geht beim Tab-Switch verloren (Tab-Body wird neu gemountet bei `match active_key`).

**Why it happens:** Dioxus mountet Children-Trees neu, wenn der Match-Branch wechselt.

**How to avoid:** Wenn Inline-Edit-State erhalten bleiben soll: Signal auf Detail-Page-Ebene halten und als Prop an `RepaymentEntryList` runter. Vorlage: `assembly_details.rs:48-50` (`search_query`, `refresh_signal` werden auf Page-Ebene gehalten).

**Warning signs:** User editiert Cell, klickt versehentlich auf Stamm-Tab, klickt zurück auf Einträge-Tab — Edit-Cell ist wieder geschlossen. Wenn das akzeptabel (UX-Defensiv-Approach), kein Fix nötig.

### Pitfall 7: Falsches `version` nach 200 OK

**What goes wrong:** Backend gibt nach `PUT/POST` die alte (lokale) version zurück, nicht die neue (DB-gebumped). Frontend nutzt die alte für Folge-Edits → 409. Lektion aus Phase 8 Plan 7+8 (CR-01).

**Why it happens:** Service-Layer-Konvention liefert die LOKAL übergebene Entity-Version, DAO bumpt aber atomar.

**How to avoid:** Nach jedem PUT/POST → erneutes GET aufrufen, NIE die Response-Body-Version direkt für Folge-Calls verwenden. Plan-Discretion: Ein Helper `reload_phase()` / `reload_entries()` als Standard-Post-Mutation-Hook.

**Warning signs:** Doppelter Save in Schnellfolge produziert 409.

### Pitfall 8: `MEMBERS`-Signal ist initial leer

**What goes wrong:** Detail-Page-Mount fires Client-Side-Join mit `MEMBERS` BEVOR `refresh_members()` durchlief → leere Tabelle / kein Match.

**Why it happens:** `MEMBERS` ist `GlobalSignal<MemberState>` mit `Default::default()` — leerer Vec.

**How to avoid:** Detail-Page muss in `use_effect` parallel `refresh_members()` + `get_repayment_phase()` + `list_repayment_entries()` triggern. Vorlage: `mail_page.rs:73-78`. Alternativ: `app.rs` lädt `MEMBERS` schon beim Auth-Erfolg pre-emptive — Plan-Discretion.

**Warning signs:** Beim ersten Aufruf der Detail-Page erscheint die Tabelle 1-2 Frames lang ohne Member-Namen, dann „blitzt" der Inhalt nach.

## Code Examples

Verified patterns from existing code:

### Reading an i18n key

```rust
// Source: genossi-frontend/src/i18n/mod.rs:735-737, de.rs:7
use crate::i18n::{use_i18n, Key};
let i18n = use_i18n();
let label = i18n.t(Key::Save);    // → "Speichern" in DE, "Save" in EN
```

[VERIFIED: genossi-frontend/src/i18n/mod.rs, de.rs, en.rs]

### Adding new i18n keys (concrete add-pattern)

```rust
// 1. Add to enum in src/i18n/mod.rs (e.g. after AssemblyTabExport block):
pub enum Key {
    // ... existing
    // ─── Phase 12 ─── RepaymentPhase ────────────────────────────
    RepaymentPhases,
    RepaymentPhaseCreate,
    RepaymentPhaseTabBasics,
    RepaymentPhaseTabEntries,
    RepaymentPhaseTabExport,
    RepaymentPhaseStatusPreparation,
    RepaymentPhaseStatusOpen,
    RepaymentPhaseStatusClosed,
    RepaymentEntryStatusOpen,
    RepaymentEntryStatusContacted,
    RepaymentEntryStatusPaidOut,
    RepaymentEntryMarkContacted,
    RepaymentEntryMarkPaidOut,
    RepaymentEntryPaidOutConfirmTitle,
    RepaymentEntryPaidOutConfirmWarn1,
    // ... etc.
}

// 2. Add to src/i18n/de.rs in the matching match arm:
pub fn translate(key: Key) -> Rc<str> {
    match key {
        // ... existing
        Key::RepaymentPhases => "Anteils-Rückzahlung".into(),
        Key::RepaymentPhaseCreate => "Neue Phase anlegen".into(),
        // ...
    }
}

// 3. Add to src/i18n/en.rs analog.
```

[VERIFIED: pattern from genossi-frontend/src/i18n/mod.rs:46-593, de.rs:4-13]

### Multi-Select-Pattern aus mail_page.rs

```rust
// Source: genossi-frontend/src/page/mail_page.rs:51-54, 250-258
let mut selected_member_ids = use_signal(|| Vec::<Uuid>::new());

// Add via button:
selected_member_ids.write().push(id);

// Remove:
selected_member_ids.write().retain(|id| *id != member_id);

// All:
selected_member_ids.set(all_ids);

// Clear:
selected_member_ids.set(Vec::new());

// Count:
let count = selected_member_ids.read().len();
```

[VERIFIED: genossi-frontend/src/page/mail_page.rs:51-54, 250-258, 318-322, 343-364]

### Format price in Cents → "60,00 €" (DE)

```rust
// Source: genossi-frontend/src/i18n/mod.rs:633-639
// i18n.format_price(6000) → "60,00 EUR" in DE, "60.00 EUR" in EN
pub fn format_price(&self, cents: i64) -> String {
    let euros = cents as f64 / 100.0;
    match self.locale {
        Locale::En => format!("{:.2} EUR", euros),
        Locale::De => format!("{:.2} EUR", euros).replace('.', ","),
    }
}
```

[VERIFIED: genossi-frontend/src/i18n/mod.rs:633-639]

**Note:** D-10 verlangt „60,00 €" mit Euro-Symbol; existing `format_price` liefert „60,00 EUR". Plan-Discretion: entweder neuer `format_payout(cents)`-Helper im RepaymentEntryList-Component oder bestehende Function nachschärfen. Vorschlag: lokaler Helper in `repayment_entry_list.rs::format_eur(cents: i64) -> String` mit „60,00 €".

### MemberSearch reuse (D-21 — direct copy-paste)

```rust
// Source-Pattern aus mail_page.rs Recipient-Picker + Component genossi-frontend/src/component/member_search.rs:42-46
let mut selected_member_id = use_signal(|| Option::<Uuid>::None);
rsx! {
    MemberSearch {
        on_select: move |id: Option<Uuid>| selected_member_id.set(id),
        selected_id: selected_member_id.read().clone(),
        exclude_id: None,  // oder eine bereits-selektierte ID, falls Duplikate verboten
    }
}
```

[VERIFIED: genossi-frontend/src/component/member_search.rs:41-46]

### Frontend-Test-Pattern (pure logic, kein WASM-Render)

```rust
// Source: genossi-frontend/src/component/member_search.rs:135-247
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_filter_by_last_name() {
        let members = test_members();
        let results = filter_members(&members, "müll", None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].member_number, 42);
    }
    // ...
}
```

**Phase-12-Anwendung:** Plan-Phase MUSS pure-Logik-Funktionen aus den Components extrahieren (z.B. `compute_payout_amount(share_count: i32, share_value: i64) -> i64`, `format_eur(cents: i64) -> String`, `filter_entries_by_status(entries: &[RepaymentEntryTO], filter: StatusFilter) -> Vec<&RepaymentEntryTO>`) — und für jede einen Unit-Test. RSX-Render-Tests sind out-of-scope (kein wasm-bindgen-test eingerichtet).

[VERIFIED: genossi-frontend/src/component/member_search.rs:135-247]

### Tab-Strip-Pattern für Detail-Page

```rust
// Source-Pattern aus assembly_details.rs:83-150 (Phase-4-Anker)
let mut active_tab = use_signal(|| "basics".to_string());
let tab_defs = vec![
    TabDef { key: "basics", label: i18n.t(Key::RepaymentPhaseTabBasics).to_string() },
    TabDef { key: "entries", label: i18n.t(Key::RepaymentPhaseTabEntries).to_string() },
    TabDef { key: "export", label: i18n.t(Key::RepaymentPhaseTabExport).to_string() },
];
// D-06: Alle 3 Tabs IMMER sichtbar (anders als assembly_details.rs, das den Export-Tab nur bei Closed pusht).
let active_key = active_tab.read().clone();
rsx! {
    TabStrip {
        tabs: tab_defs,
        active_key: active_key.clone(),
        on_change: move |k: String| active_tab.set(k),
        match active_key.as_str() {
            "basics" => rsx! { BasicsTab { phase: phase_for_basics, on_changed: move |_| load_phase() } },
            "entries" => match status_value {
                RepaymentPhaseStatusTO::Preparation => rsx! {
                    div { class: "text-center py-12 text-gray-500", "Phase noch nicht geöffnet — Einträge erscheinen nach dem Öffnen." }
                },
                _ => rsx! { RepaymentEntryList { phase, entries, on_changed: ..., on_error: ... } },
            },
            "export" => match status_value {
                RepaymentPhaseStatusTO::Preparation => rsx! {
                    div { class: "text-center py-12 text-gray-500", "Phase noch nicht geöffnet — Export verfügbar ab Status 'Offen'." }
                },
                _ => rsx! { ExportTab { phase } },
            },
            _ => rsx! {},
        }
    }
}
```

### NavGroup-Extension (D-27)

```rust
// Pattern aus top_bar.rs:46-66, 149-185
// In den existierenden Vorstand-Nav-Group (vermutlich "verwaltung" oder "mitglieder"):
verwaltung_items.push(NavItem {
    label: i18n.t(Key::RepaymentPhases).to_string(),  // "Anteils-Rückzahlung"
    route: Route::RepaymentPhases {},
});
```

[VERIFIED: genossi-frontend/src/component/top_bar.rs:46-112]

## Reuse Asset Inventory (concrete signatures from code)

| Asset | Path | Signature / Props |
|-------|------|-------------------|
| `MemberSearch` | `genossi-frontend/src/component/member_search.rs:42-46` | `on_select: EventHandler<Option<Uuid>>, selected_id: Option<Uuid>, exclude_id: Option<Uuid>` |
| `filter_members` (pure) | `genossi-frontend/src/component/member_search.rs:9-35` | `fn(members: &[MemberTO], query: &str, exclude_id: Option<Uuid>) -> Vec<&MemberTO>` |
| `Modal` | `genossi-frontend/src/component/modal.rs:8-16` | `children: Element` (only prop) |
| `ErrorAlert` | `genossi-frontend/src/component/error_alert.rs:6` | `error: AppError, on_dismiss: Option<EventHandler<()>>` |
| `ToastContainer` | `genossi-frontend/src/component/toast.rs:30` | `messages: ReadOnlySignal<Vec<(u64, String)>>` |
| `show_toast` (helper) | `genossi-frontend/src/component/toast.rs:14` | `fn(toast_messages: &mut Signal<...>, toast_counter: &mut Signal<u64>, msg: String)` |
| `TabStrip` | `genossi-frontend/src/component/tab_strip.rs:17-22` | `tabs: Vec<TabDef>, active_key: String, on_change: EventHandler<String>, children: Element` |
| `TabDef` | `genossi-frontend/src/component/tab_strip.rs:10-14` | `pub key: &'static str, pub label: String` |
| `AssemblyStatusBadge` | `genossi-frontend/src/component/assembly_status_badge.rs:33-39` | `status: AssemblyStatusTO` (Vorlage zum Klonen) |
| `NavGroup` | `genossi-frontend/src/component/nav_group.rs:12-46` | `label: String, items: Vec<NavItem>, is_open: bool, on_toggle: EventHandler<()>, on_navigate: EventHandler<()>` |
| `NavItem` | `genossi-frontend/src/component/nav_group.rs:5-9` | `label: String, route: Route` |
| `RequirePrivilege` | `genossi-frontend/src/auth.rs:34-48` | `privilege: &'static str, children: Element, fallback: Option<Element>` |
| `TemplateVarButtons` | `genossi-frontend/src/component/mail_compose/template_var_buttons.rs:28-30` | `on_insert: EventHandler<String>` (Hard-Coded Var-Listen PRIMARY_VARS + SECONDARY_VARS — D-19 erfordert Erweiterung um optionalen `extra_vars`-Prop oder bedingte Repayment-Var-Liste) |
| `MEMBERS` Global Signal | `genossi-frontend/src/service/member.rs:7` | `GlobalSignal<MemberState>` (`items: Vec<MemberTO>, loading: bool, error: Option<String>`) |
| `refresh_members()` | `genossi-frontend/src/service/member.rs:11-25` | `async fn refresh_members()` |
| `AUTH` Signal | `genossi-frontend/src/service/auth.rs` | `GlobalSignal<AuthState>` (read in `RequirePrivilege`) |
| `CONFIG` Signal | `genossi-frontend/src/service/config.rs` | `GlobalSignal<Config>` (`backend: String`) |
| `AppError` | `genossi-frontend/src/api.rs:14-19` | `status: Option<u16>, message: String, detail: Option<String>` |
| `status_to_message` | `genossi-frontend/src/api.rs:73-87` | `fn(status: u16) -> &'static str` (DE-lokalisiert) |
| `check_response` | `genossi-frontend/src/api.rs:119-125` | `async fn(response: reqwest::Response) -> Result<reqwest::Response, AppError>` |
| `I18n::format_price` | `genossi-frontend/src/i18n/mod.rs:633-639` | `fn(&self, cents: i64) -> String` (liefert „60,00 EUR" — nicht „60,00 €", siehe Pitfall D-10) |
| `I18n::format_datetime` | `genossi-frontend/src/i18n/mod.rs:643-657` | `fn(&self, iso: &str) -> String` |
| `I18n::format_date` | `genossi-frontend/src/i18n/mod.rs:612-631` | `fn(&self, date: &time::Date) -> String` |
| `RepaymentPhaseTO` | `genossi_rest_types/src/lib.rs:1186-1221` | siehe Backend-Surface oben |
| `RepaymentEntryTO` | `genossi_rest_types/src/lib.rs:1309-1334` | siehe Backend-Surface oben |
| `CloseConflictResponse` | `genossi_rest_types/src/lib.rs:1390-1396` | siehe Backend-Surface oben |
| `BatchFailureResponse` | `genossi_rest_types/src/lib.rs:1414-1422` | siehe Backend-Surface oben |

## Integration Touchpoints (line-level references)

| File | Lines | Touchpoint Type | Action |
|------|-------|-----------------|--------|
| `genossi-frontend/src/router.rs` | 24-69 | Route-Enum erweitern | +2 `#[route]`-Varianten (D-25) |
| `genossi-frontend/src/router.rs` | 1-22 | Page-Re-Exports | +2 `pub use crate::page::Repayment*` |
| `genossi-frontend/src/page/mod.rs` | (alle Re-Exports) | Page-Module-Re-Exports | +`pub mod repayment_phases; pub use repayment_phases::*;` etc. |
| `genossi-frontend/src/component/mod.rs` | (alle Re-Exports) | Component-Re-Exports | +`pub mod repayment_entry_list;` etc. |
| `genossi-frontend/src/api.rs` | ende-of-file | +12 API-Funktionen | Phase 7-11 REST-Endpoints (D-26) |
| `genossi-frontend/src/component/top_bar.rs` | 46-112 | NavItem-Liste in Vorstand-Nav-Group | +1 NavItem für `/repayment-phases` (D-27) |
| `genossi-frontend/src/component/mail_compose/template_var_buttons.rs` | 5-26, 28-30 | Var-Buttons-Liste + Props | +Bedingte Repayment-Vars (D-19); Plan-Discretion: `extra_vars: Option<Vec<(&str, &str)>>` Prop ODER `show_repayment: bool` Bool-Flag |
| `genossi-frontend/src/page/mail_page.rs` | 40-77 | MailPage-Component | +Query-Param-Parsing in `use_effect` (D-18); +Pre-Selection `selected_member_ids.set(parsed_members)` |
| `genossi-frontend/src/page/mail_page.rs` | 487-540 | Send-Button-onclick | +`repayment_phase_id` als Body-Feld, falls Query-Param vorhanden (Plan 10 D-03/D-12 Backend-Bereitschaft) |
| `genossi-frontend/src/api.rs::send_bulk_mail` | 872-892 | Body-Struktur | +`template_id`, `repayment_phase_id` Optional-Felder (Backend ist schon ready — Phase 10 Plan 04 ergänzte das im REST) |
| `genossi-frontend/src/i18n/mod.rs` | 46-593 | Key-Enum | +~25 Phase-12-Keys (RepaymentPhase…, RepaymentEntry…) |
| `genossi-frontend/src/i18n/de.rs` | 4-(EOF) | Translate-Match | +DE-Strings für alle neuen Keys |
| `genossi-frontend/src/i18n/en.rs` | analog | Translate-Match | +EN-Strings analog |

## Runtime State Inventory

Phase 12 ist greenfield-additiv (keine Renames, keine Migrations, keine Backend-Änderungen) — der Inventar-Audit aus der Standard-Researcher-Spec gilt überwiegend „nichts gefunden":

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — Phase 12 hat keinen eigenen Backend-State (Backend ist gelocked) | None |
| Live service config | None — keine externen Services außer Backend (gleicher Mount) | None |
| OS-registered state | None | None |
| Secrets/env vars | None — Frontend liest `config.backend` via `assets/config.json` (existierender Mechanismus) | None |
| Build artifacts | `genossi-frontend/dist/` wird durch `dx build` neu erzeugt; Tailwind generiert `assets/tailwind.css` neu via watch | Re-run `dx build` + `npx tailwindcss` nach Implementation (Standard-Workflow, kein Sonderfall) |

**Nothing found in stored data category:** Verifiziert — Phase 12 fügt nur Frontend-Source-Files hinzu; keine DB-Migrations (alle bereits in Phase 7+8 erfolgt); keine Mem0/ChromaDB; keine pm2/systemd-Registrations; keine SOPS-Secrets.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Backend + Frontend compile | ✓ (Standard-Dev-Env) | 2021 edition | — |
| `dx` CLI (Dioxus CLI) | `dx serve`, `dx build` | ? (im Nix-Env vermutlich vorhanden — siehe Lock-Memory `feedback_nix_toolchain.md`) | 0.6.x | manuelle `cargo build --target wasm32-unknown-unknown` als ungeprüfter Fallback |
| Node.js + npm | Tailwind watch | ✓ (Standard-Dev-Env) | — | — |
| `wasm-bindgen-cli` | Release-Build | ⚠ Phase-4-Lektion: in lokalem Nix-Profil fehlend (Version 0.2.104) | 0.2.104 | Phase 4 hatte das als PENDING flag bei der Verification; Plan 12 sollte das Tooling-Debt mitberücksichtigen (Build-Verifikation lokal). Aber: `dx serve` für Dev funktioniert ohne. |
| SQLite (Backend, für E2E) | E2E-Tests | ✓ (in-memory in Tests) | — | — |
| SMTP-Stub / echter SMTP-Account | UAT für UI-06 | — (UAT-Phase) | — | UAT-Checkliste dokumentiert manuelle Validierung mit Staging-SMTP (analog Phase 4) |

**Missing dependencies with no fallback:** Keine — alle für Dev nötigen Tools sind im Nix-Env.

**Missing dependencies with fallback:** `wasm-bindgen-cli@0.2.104` für `dx build --release` (Phase-4-Pending-Item übernommen). Plan-Phase entscheidet, ob das in Phase 12 closed wird oder weiter als Tooling-Debt separat erledigt wird.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in Rust test runner) |
| Config file | none — Cargo.toml metadata reicht |
| Quick run command | `cargo test -p genossi-frontend --lib` |
| Full suite command | `cargo test --workspace` (inkl. backend, ~927 tests + new Phase-12 unit tests) |

**Wichtig:** Phase 12 hat KEIN WASM-Test-Setup (`wasm-bindgen-test` ist nicht eingerichtet). Reine Logik-Tests (Funktionen ohne Dioxus-RSX-Context) sind das Maximum. Render-/Interaction-Tests laufen über UAT-Checkliste analog Phase 4.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UI-01 | Liste lädt + Sort + Create-Modal öffnet | unit (pure fns) + UAT | `cargo test -p genossi-frontend repayment_phases::tests` | ❌ Wave 0 |
| UI-01 | Default-Sort `fiscal_year DESC, created DESC` | unit | Pure-Func `fn sort_phases_default(phases: &[RepaymentPhaseTO]) -> Vec<&RepaymentPhaseTO>` mit Sort-Stabilität-Test | ❌ Wave 0 |
| UI-02 | 3-Tab-Layout + Lifecycle-Buttons je nach Status | UAT only | manual | UAT-Checkliste |
| UI-02 | `share_value`-Inline-Edit erlaubt (Vorbereitung+Offen) | UAT + unit | Pure-Func `fn is_share_value_editable(status: &RepaymentPhaseStatusTO) -> bool` | ❌ Wave 0 |
| UI-03 | 7-Spalten-Tabelle, Multi-Select, Status-Filter | UAT + unit | Pure-Func `fn filter_entries_by_status(entries: &[RepaymentEntryTO], filter: StatusFilter) -> Vec<&RepaymentEntryTO>` | ❌ Wave 0 |
| UI-03 | Default-Sort Mitgliedsnummer ASC, sekundär created ASC | unit | Pure-Func `fn sort_entries_default(entries: &[RepaymentEntryTO], members: &[MemberTO]) -> Vec<&RepaymentEntryTO>` | ❌ Wave 0 |
| UI-03 | Betrag-Berechnung in Cent → „60,00 €"-String | unit | Pure-Func `fn format_payout_eur(share_count: i32, share_value_cents: i64) -> String` mit Edge-Cases (0, große Werte, negative…) | ❌ Wave 0 |
| UI-04 | Add-Modal mit Member-Picker + Vorbelegung `current_shares` | UAT | manual | UAT-Checkliste |
| UI-04 | Validation `share_count > 0` und Member-Pflicht | unit | Pure-Func `fn validate_create_entry(member_id: Option<Uuid>, share_count: i32) -> Result<(), ValidationError>` | ❌ Wave 0 |
| UI-05 | Confirm-Modal-Inhalt + Summe-Berechnung | unit | Pure-Func `fn sum_payout_amounts(entries: &[RepaymentEntryTO], share_value: i64) -> i64` | ❌ Wave 0 |
| UI-05 | Bulk-Loop läuft auch nach Einzel-Fehler weiter | UAT | manual w/ staging SMTP+DB | UAT-Checkliste |
| UI-06 | Query-Param-Parsing in `/mail` Pre-Selection | unit | Pure-Func `fn parse_mail_query(search: &str) -> ParsedMailContext` | ❌ Wave 0 |
| UI-06 | Mail-Send mit `repayment_phase_id` im Body | UAT + e2e via `cargo test --test e2e_tests` (Backend) | manual | UAT-Checkliste (Backend-Side ist bereits getestet in Phase 10) |

### Sampling Rate

- **Per task commit:** `cargo test -p genossi-frontend --lib` (Frontend-Unit-Tests)
- **Per wave merge:** `cargo test --workspace` (gesamtes Workspace, ~927+ Tests, alle grün)
- **Phase gate:** Full suite green + UAT-Checkliste abgehakt vor `/gsd-verify-work`

### Wave 0 Gaps

Die Frontend-Crate hat eingespielte `#[cfg(test)] mod tests`-Pattern (siehe `member_search.rs:135-247`). Pro neue Component-Datei sollte ein `tests`-Modul für pure-Logik-Funktionen mitgepflanzt werden.

- [ ] `genossi-frontend/src/component/repayment_entry_list.rs` — `mod tests` mit Tests für `filter_entries_by_status`, `sort_entries_default`, `format_payout_eur`
- [ ] `genossi-frontend/src/page/repayment_phases.rs` — `mod tests` mit Test für `sort_phases_default`
- [ ] `genossi-frontend/src/page/repayment_phase_details.rs` — `mod tests` für Status-zu-Editability-Mapping (`is_share_value_editable`, `is_entry_editable`, `should_show_lifecycle_button`)
- [ ] `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` — `mod tests` für `sum_payout_amounts`
- [ ] `genossi-frontend/src/component/repayment_entry_add_modal.rs` — `mod tests` für `validate_create_entry`
- [ ] `genossi-frontend/src/page/mail_page.rs` — `mod tests` für `parse_mail_query` (zwei Helper-Funktionen, weil Page eh schon getestet ist via UAT, aber Pure-Func ist günstig)

**UAT-Checkliste-Anker (analog Phase-4 `04-UAT-CHECKLIST.md`, 173 Items):** Phase 12 produziert eigene UAT-Checkliste mit:
- Lifecycle-Klicks (Create / Open / Close / Edit `share_value`)
- 7-Spalten-Tabelle-Sort + Filter-Toggles
- Multi-Select-Aktionen (Mark Contacted, Mark Paid-Out, Massenmail)
- Inline-Cell-Edit (Anteile ändern)
- Add-Entry-Modal (Member-Picker + Vorbelegung)
- PaidOut-Confirm-Modal (Listen + Summe + Warnung + roter Button)
- PDF-Download (open/all/paid)
- Massenmail-Flow (Redirect zu /mail → Repayment-Vars sichtbar → Send → Zurück → Mark-Contacted)
- Button-Reload-Check (D-01/D-02 Grep + manueller Klick-Test)
- Empty-States (Phase ohne Einträge, Filter ohne Treffer)
- Auth-Gate (Helper-Login zeigt AccessDenied)

## Risks & Unknowns

### Risk 1: Tab-Strip existence — RESOLVED (HIGH confidence)

**Status:** `genossi-frontend/src/component/tab_strip.rs` existiert (78 Zeilen, mit Tests). D-28-Fallback („sonst neuen `tab_strip.rs` aus assembly_details.rs extrahieren") entfällt. Direct Reuse.

[VERIFIED: `ls genossi-frontend/src/component/tab_strip.rs` + Datei-Inhalt gelesen]

### Risk 2: Mail-Page Erweiterungs-Komplexität — MEDIUM

**Was:** `/mail`-Page (`mail_page.rs`, 800 Zeilen) ist eine umfangreiche Component mit eigenem Recipient-Picker (Lines 269-373), Compose-Form (375-541) und Job-Liste (546-700). D-18 verlangt Query-Param-Parsing + Pre-Selection. D-19 verlangt bedingte Repayment-Var-Buttons.

**Komplexität:**
- Query-Param-Parsing: 20-30 Zeilen `use_effect` (web_sys-API)
- Repayment-Var-Buttons-Show-Condition: `TemplateVarButtons` braucht neuen Prop (siehe Pitfall #4) — bedeutet API-Change, das wirkt sich auf eventuelle andere Aufrufer aus
- `repayment_phase_id` in `send_bulk_mail`-Body: api.rs:872-892 muss `SendBulkMailRequest` erweitern (Optional-Felder, Backward-Compat via `#[serde(default)]`)
- Send-Button-onclick (mail_page.rs:487-540) muss `repayment_phase_id` aus dem Page-State extrahieren und an api.rs übergeben

**Mitigation:** Plan-Phase teilt UI-06 in 3 Tasks: (1) `template_var_buttons.rs` Prop-Erweiterung + Test, (2) `mail_page.rs` Query-Param-Parsing + Pre-Selection + Body-Erweiterung, (3) `api.rs::send_bulk_mail` Body-Felder. Reihenfolge: Frontend-API zuerst, dann Page.

**Verification:** `TemplateVarButtons` aktuell genutzt nur in `mail_page.rs:375-379` (1 Aufrufer) — `rg "TemplateVarButtons" genossi-frontend/src/` bestätigt das. Prop-Erweiterung hat begrenzten Blast-Radius.

### Risk 3: i18n add-pattern overhead — LOW

**Was:** Phase 12 fügt ca. 25-30 neue i18n-Keys ein. Pflicht für beide Locales (de/en). Pattern ist Enum + Match-Arm in zwei Dateien.

**Komplexität:** Minimal — bekanntes Pattern, kein Risiko. Plan-Phase listet exakte Keys.

**Mitigation:** Plan-Phase finalisiert die Key-Liste; Wave 1 fügt alle Keys einmal an drei Stellen (mod.rs Enum, de.rs Match, en.rs Match) ein, bevor die Components sie nutzen.

### Risk 4: `editable_cell.rs` ist NEU — MEDIUM

**Was:** D-13 verlangt einen Inline-Cell-Edit-Component, den die Codebase NICHT hat. Member_details.rs nutzt Page-Level-Edit-Toggle, was ein anderes Pattern ist.

**Komplexität:** ~50-80 Zeilen Component. Pure-Logik-Test (`format_share_count_input(...)`, Validierung > 0) ist trivial. Render-Logik (click → input + ✓ ✗ Buttons) ist klar.

**Mitigation:** Plan-Phase spezifiziert die Component-API explizit (siehe Pitfall #5). Vorschlag: spezialisierte Version `EditableShareCountCell` (Hardcode i32-Type + Status-Awareness) statt generischer `EditableCell<T>` — weniger Generics, einfacher zu testen, gleicher Use-Case-Coverage. Falls später ein generischer Helper nötig wird, ist Refactoring günstig.

### Risk 5: `format_price` liefert "EUR" statt "€" — LOW

**Was:** D-10 fordert „60,00 €" (mit Euro-Symbol). `I18n::format_price` in `i18n/mod.rs:633-639` liefert „60,00 EUR".

**Mitigation:** Lokaler Helper in `repayment_entry_list.rs::format_eur(cents: i64) -> String` mit Replace „EUR" → „€" ODER direkt eigene Formatierung:
```rust
fn format_eur(cents: i64) -> String {
    let euros = cents / 100;
    let cents_rem = (cents.abs() % 100) as u32;
    format!("{},{:02} €", euros, cents_rem)
}
```
Plus Unit-Tests für 0, 100, 12345, -100, große Werte.

### Risk 6: Backend gibt stale `version` zurück (Phase-8-Lektion) — MEDIUM

**Was:** Phase 8 Plan 7+8 löste CR-01 — der Service-Layer gibt nach `audited_update!` die alte version zurück. Frontend muss nach jedem Mutation re-loaden (siehe Pitfall #7).

**Mitigation:** Plan-Phase fixiert ein Standard-Pattern „nach jedem PUT/POST/DELETE → re-fetch der Entity statt Verwendung der Response-Body-Version". Beispielsweise als Helper `async fn reload_phase_after_mutation(id: Uuid) -> Result<RepaymentPhaseTO, AppError>`. Alternativ: Plan-Discretion zur Diskussion ob das Phase-8-Re-Read-Pattern noch nötig ist (Phase 8 Plan 10 hat Regression-Tests dafür eingespielt — wenn die Re-Read schon im Service-Layer passiert, gibt der `RepaymentPhaseTO`-Response bereits die neue version). **Verify in Plan-Phase via Backend-Code-Read.**

### Unknown 1: Plan-Aufwand-Schätzung

**Was:** Phase 12 hat 6 Requirements und ist die größte Frontend-Phase. Vergleich zu Phase 4 (Frontend, Component-First) — 11 Plans. Phase 12 könnte ähnlich groß sein.

**Mitigation:** Plan-Decomposition-Vorschlag siehe nächste Sektion.

### Unknown 2: PDF-Download UX-Detail

**Was:** D-26 sagt PDF-Export via `<a href=".../export/pdf?include=open" target="_blank">`. Browser handelt Content-Disposition. Aber: Filter-Auswahl + Download in einem UI-Flow? Vorschlag: Radio-Buttons („open/all/paid") + großer „Download"-Button-Link, der die URL mit dem aktuellen Filter zusammenbaut.

**Mitigation:** Plan-Phase finalisiert Export-Tab-Layout. Reference: Phase-6 (`AttendanceExport*`-Keys in i18n/mod.rs:513-533) hat ein analoges Layout — gleicher Component-Stil.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Inline `<button onclick={}>`, default-type | `r#type: "button"` explizit + onclick | Phase 4 Hotfix e245013 (2026-05-06) | Memory-Lock `feedback_dioxus_button_type.md` |
| `<form onsubmit={async ...}>` | `<div>` + handler bei No-Form-Semantik; bei Form: prevent_default vor spawn | Hotfix c6f41fd (2026-05-06) | Forms NIE für simple Buttons-Container |
| Inline-Tabs in Pages | `TabStrip`-Component | Phase 4 Plan 06 | D-28 forced reuse |
| Inline-Toast in mehreren Pages | `ToastContainer + show_toast` | Phase 4 Plan 06 | D-04/D-17 forced reuse |
| Custom-Status-Badge inline | `*_status_badge.rs`-Component | Phase 4 Plan 06 | Vorlage für `repayment_*_status_badge.rs` |
| `wasm-bindgen-test` für Render-Tests | None — nur pure-Logik-Tests + UAT | Phase 4 D-110 | Phase 12 fortsetzen |

**Deprecated/outdated:**
- `applications_page.rs` hat noch inline-Tab-Pattern (siehe `tab_strip.rs:6` Doc-Kommentar „applications_page.rs has NOT been migrated") — Phase 12 NICHT der richtige Moment zum Migrieren (Scope-Disziplin)
- `members.rs` hat noch inline-Toast-Helper (siehe `toast.rs:7` Doc-Kommentar) — same as above

## Recommended Plan Decomposition

Vorschlag für Plan-Boundaries mit Wave-Grouping. Plan-Phase entscheidet final.

### Wave 1 (Foundation, parallel-bar): Standalone Reuse-Bausteine

- **12-01-PLAN.md** — `api.rs` 12 neue API-Funktionen + lokale 409-Response-Strukturen (`CloseConflictResponse`, `BatchFailureResponse`) + i18n-Keys (Enum + de/en Match-Arms). Foundation-Layer; ohne den läuft nichts.
- **12-02-PLAN.md** — Status-Badges (`repayment_phase_status_badge.rs` + `repayment_entry_status_badge.rs`) + Format-Helper (`format_payout_eur(share_count, share_value_cents)`) als Pure-Funktion + Unit-Tests. Pattern direkt aus `assembly_status_badge.rs`. Parallel zu 12-01 möglich (kein API-Bedarf).

### Wave 2 (Blocked on Wave 1): Routing + Nav + Listen-Page

- **12-03-PLAN.md** — `router.rs` Route-Enum + `top_bar.rs` NavItem (D-25, D-27). Klein, isoliert.
- **12-04-PLAN.md** — Listen-Page `repayment_phases.rs` (UI-01): Liste + Default-Sort + Create-Modal (mit `Modal`-Reuse). Verwendet 12-01-APIs und 12-02-Status-Badge.

### Wave 3 (Blocked on Wave 2): Detail-Page Skeleton + Stamm-Tab

- **12-05-PLAN.md** — Detail-Page `repayment_phase_details.rs` (UI-02-Skeleton): TabStrip-Mount + 3 Tab-Branches mit Platzhalter-Bodies + Lifecycle-Action-Tile im Stamm-Tab + Confirm-Modal für 'Schließen' (D-07). Reused: `TabStrip`, `Modal`, `RequirePrivilege`.
- **12-06-PLAN.md** — `share_value`-Inline-Edit im Stamm-Tab (D-05, PHAS-04): editable Feld mit „Speichern"-Button, Status-aware (Vorbereitung/Offen=editable, Abgeschlossen=read-only). Pure-Func `is_share_value_editable(status)` + Unit-Test.

### Wave 4 (Blocked on Wave 3): RepaymentEntryList Core

- **12-07-PLAN.md** — `editable_cell.rs` (oder `editable_share_count_cell.rs`) als Standalone-Component + Pure-Func-Tests (D-13).
- **12-08-PLAN.md** — `repayment_entry_list.rs` (UI-03): 7 Spalten + Client-Side-Join mit `MEMBERS` + Multi-Select-Pattern + Status-Filter-Tab-Strip-im-Tab (D-10/D-11/D-12) + Default-Sort + Empty-States (D-14) + Soft-Delete-Action. Verwendet `EditableCell` aus 12-07.

### Wave 5 (Blocked on Wave 4): Add + Confirm-Modals

- **12-09-PLAN.md** — `repayment_entry_add_modal.rs` (UI-04): Modal mit `MemberSearch` + `share_count_to_pay_out`-Input + Vorbelegung `current_shares` + Client-Validation (D-22, D-23, D-24).
- **12-10-PLAN.md** — `repayment_entry_paidout_confirm.rs` (UI-05): Bulk-Confirm-Modal mit Listentabelle + Gesamtsumme + 3-Punkt-Warnung + roter „Endgültig markieren"-Button (D-15, D-16) + Sequential-Loop-Logik mit pro-Entry-Toast (D-17). Pure-Func `sum_payout_amounts` + Unit-Test.

### Wave 6 (Blocked on Wave 5): Mail-Erweiterung (UI-06)

- **12-11-PLAN.md** — `template_var_buttons.rs` Prop-Erweiterung (D-19) + Unit-Test.
- **12-12-PLAN.md** — `mail_page.rs` Query-Param-Parsing + Pre-Selection + `repayment_phase_id`-Body-Erweiterung (D-18) + `api.rs::send_bulk_mail` Body-Felder.
- **12-13-PLAN.md** — Detail-Page-Verdrahtung: „Massenmail an N ausgewählte"-Button in `RepaymentEntryList` Header-Action-Leiste, baut Redirect-URL `/mail?from=repayment&phase_id=…&members=…` (D-18). Hier landet UI-06 final.

### Wave 7 (Blocked on Wave 6): Export-Tab + Cleanup

- **12-14-PLAN.md** — Export-Tab im `repayment_phase_details.rs`: Include-Filter-Radio + Download-Anker (D-26). Klein, isoliert.
- **12-15-PLAN.md** — D-02 Button-Pattern Grep-Gate Pre-Merge-Check + Component-First-Grep-Gate (siehe nächste Sektion) + Phase-12-UAT-Checkliste-Skelett (`12-UAT-CHECKLIST.md`).

### Optional Wave 8 (UAT + Verify)

- Echte UAT-Klick-Tour auf Staging mit echtem SMTP-Account.

**Geschätzter Plan-Count:** 14-15 Plans (Größenordnung Phase 4). Plan-Phase darf umgruppieren — die Wave-Boundaries sind Vorschläge, nicht Pflicht.

## Component-First-Grep-Gate Design (D-02 + Component-First-Check)

### Button-Type-Gate (D-02)

Plan-Phase-Acceptance:
```bash
# 1. Count buttons in Phase-12-Files
COUNT_BUTTONS=$(rg -c 'button\s*\{' \
    genossi-frontend/src/component/repayment_*.rs \
    genossi-frontend/src/page/repayment_*.rs 2>/dev/null \
    | awk -F: '{s+=$2} END {print s+0}')

# 2. Count typed buttons in same files
COUNT_TYPED=$(rg -c 'r#type:\s*"button"' \
    genossi-frontend/src/component/repayment_*.rs \
    genossi-frontend/src/page/repayment_*.rs 2>/dev/null \
    | awk -F: '{s+=$2} END {print s+0}')

# 3. Acceptance: COUNT_BUTTONS == COUNT_TYPED
test "$COUNT_BUTTONS" = "$COUNT_TYPED" || echo "FAIL: $COUNT_BUTTONS buttons but only $COUNT_TYPED typed"
```

**Negative-Control (Verify the gate works):**
```bash
# Phase-4-Dateien sollten ebenfalls passieren (kein False-Positive):
COUNT_BUTTONS=$(rg -c 'button\s*\{' genossi-frontend/src/page/assembly_details.rs genossi-frontend/src/page/assemblies.rs)
COUNT_TYPED=$(rg -c 'r#type:\s*"button"' genossi-frontend/src/page/assembly_details.rs genossi-frontend/src/page/assemblies.rs)
# Sollte gleich sein. (Plan-Phase: einmal lokal verifizieren.)
```

### Component-First-Grep-Gate

**Idee:** Inline-RSX in `page/repayment_*` darf nicht das gleiche Pattern enthalten wie ein extrahierter Component. Konkret: Wenn ein Page-File einen `<div class="...">`-Block mit Tab-Strip-Klassen hat, statt `TabStrip { ... }` zu nutzen, ist das eine Verletzung.

**Pragmatischer Check** (eine Approximation):
```bash
# Tab-Strip-Klassen-Pattern aus tab_strip.rs:24:
#   class: "flex border-b border-gray-200 mb-6 print:hidden"
# Wenn dieses Pattern in einer Page-Datei vorkommt OHNE TabStrip-Component-Use,
# ist es vermutlich inline-Duplikat.

if rg -q 'flex border-b border-gray-200 mb-6' genossi-frontend/src/page/repayment_*.rs; then
    if ! rg -q 'TabStrip\s*\{' genossi-frontend/src/page/repayment_*.rs; then
        echo "FAIL: tab-strip-classes found inline, but no TabStrip component used"
    fi
fi
```

**Alternative (cheaper):** Plan-Phase listet konkrete Component-Names als Pflicht-Reuse-Targets. Verify-Phase grepped:
```bash
# Pflicht-Reuse-Targets:
for COMP in TabStrip Modal ErrorAlert ToastContainer AssemblyStatusBadge RequirePrivilege MemberSearch; do
    rg "$COMP" genossi-frontend/src/page/repayment_*.rs genossi-frontend/src/component/repayment_*.rs 2>/dev/null > /dev/null \
        || echo "WARN: $COMP not used in Phase-12 files (might be intentional, manual check)"
done
```

**Plan-Phase finalisiert** die exakte Gate-Spec im Plan-Acceptance-Block.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `dx serve` und Tailwind-Watch sind im Standard-Dev-Env vorhanden | Environment Availability | Plan-Phase muss eventuell Tooling-Setup-Schritte ergänzen |
| A2 | `wasm-bindgen-cli@0.2.104` ist im Nix-Env vorhanden ODER Phase 12 baut auf Phase-4-Pending-Item auf (Tooling-Debt). | Environment Availability | Release-Build kann blocken; Dev-Server (`dx serve`) ist unabhängig |
| A3 | Backend liefert nach `mark-paid-out` die korrekte neue `Member.current_shares`-Version in `MEMBERS` nach Refresh (Phase 9 ist Cascade-atomar) | Pitfall #3 | `current_shares` kann stale bleiben → Bug-Symptom in UAT sichtbar |
| A4 | `dioxus-router` 0.6 Query-Param-Support ist verfügbar — aber Plan-Discretion empfiehlt `web_sys::UrlSearchParams` als minimal-invasive Lösung | Pitfall #4 | Falls Route-Macro-Erweiterung doch nötig: 2-3h zusätzlicher Refactor |
| A5 | `TemplateVarButtons` hat nur 1 Aufrufer (`mail_page.rs`) — Prop-Erweiterung ist blast-radius-arm. (VERIFIED via grep) | Risk 2 | — (verifiziert) |

**If this table is empty:** A1-A4 sind ASSUMED. Plan-Phase verifiziert A2 (Tooling) und A4 (Query-Param-Lösung) explizit.

## Open Questions

1. **Plan-Discretion: Lokal in `api.rs` deklarieren vs. `rest-types/` shared crate erweitern für `CloseConflictResponse`/`BatchFailureResponse`?**
   - What we know: Frontend hat sein eigenes `rest-types/`-Crate; Backend hat `genossi_rest_types`. Beide sind getrennt.
   - What's unclear: Welcher Reuse-Style ist Konvention?
   - Recommendation: Lokal in `api.rs` deklarieren (analog `MailJobTO` in api.rs:819-829). Pragmatic; falls Pattern wiederholt anfällt, später in `rest-types/` ziehen.

2. **Plan-Discretion: Bulk-PaidOut Toast pro Entry ODER ein konsolidierter Schluss-Toast?**
   - What we know: D-17 sagt „Toast pro Entry, deutsche Fehlermeldung". D-15 sagt „X von N erfolgreich, Y fehlgeschlagen — siehe Status-Spalte".
   - What's unclear: Beides koexistiert?
   - Recommendation: ToastContainer erlaubt Multi-Toast → 1 Toast pro Fehler-Entry (D-17) + 1 finaler Summary-Toast (D-15 „X von N erfolgreich"). Beide gleichzeitig kein Konflikt; ToastContainer stapelt sie.

3. **Plan-Discretion: Generischer `EditableCell<T>` oder spezialisierter `EditableShareCountCell`?**
   - What we know: D-13 schreibt „neuer Component-Baustein `editable_cell.rs` (vermutlich generischer Helper)".
   - What's unclear: Phase 12 hat nur einen Use-Case (share_count_to_pay_out, i32). Generics-Overhead lohnt nicht.
   - Recommendation: Spezialisiert. Falls in v1.2+ ein weiterer Inline-Cell-Edit-Case auftaucht, dann refactor zu generisch.

4. **Plan-Discretion: `repayment_phase_id` in `SendBulkMailRequest`-Body — wie Frontend-State an die Mail-Page durchreichen?**
   - What we know: D-18 sagt Query-Param. D-19 sagt Repayment-Var-Buttons abhängig vom Kontext.
   - What's unclear: Wenn `phase_id` als Query-Param kommt, muss MailPage es ZUSÄTZLICH als Body-Feld an `send_bulk_mail` weiterleiten (Phase-10 D-03/D-12).
   - Recommendation: `MailPage` parst Query-Param in `use_signal<Option<Uuid>>`-State, der nach Pre-Selection auch für den Send-Body verwendet wird. Pattern wie `selected_member_ids`.

5. **Plan-Discretion: Bei 409 `CloseConflictResponse` — Toast nur, oder Toast + Liste der pending member numbers anzeigen?**
   - What we know: D-04 sagt „Toast mit deutscher Fehlermeldung (`Schließen blockiert: N Einträge noch nicht ausbezahlt`)".
   - What's unclear: D-04 spezifiziert nicht, ob `pending_member_numbers` (bis zu 20 + „+N weitere") angezeigt wird.
   - Recommendation: ErrorAlert mit „Details anzeigen"-Expand (Pattern aus `error_alert.rs:24-44`) — kompakter Toast-Header + Click-to-Expand der Member-Liste. Plan-Discretion auf die exakte Wortlaut-Spec.

## Sources

### Primary (HIGH confidence) — verified via file reads

- `genossi-frontend/src/component/tab_strip.rs` — 78 lines, generic tab component, `#[component] TabStrip(tabs, active_key, on_change, children)`
- `genossi-frontend/src/component/member_search.rs` — `MemberSearch(on_select, selected_id, exclude_id)`, `filter_members(...)` pure helper
- `genossi-frontend/src/component/modal.rs` — `Modal { children }` only
- `genossi-frontend/src/component/error_alert.rs` — `ErrorAlert { error: AppError, on_dismiss: Option<EventHandler<()>> }`
- `genossi-frontend/src/component/toast.rs` — `ToastContainer { messages }` + `show_toast(...)` helper
- `genossi-frontend/src/component/assembly_status_badge.rs` — Vorlage für RepaymentPhase/Entry-Status-Badges
- `genossi-frontend/src/component/nav_group.rs` — `NavGroup(label, items, is_open, on_toggle, on_navigate)`
- `genossi-frontend/src/component/mail_compose/template_var_buttons.rs` — Hardcoded Var-Listen (PRIMARY_VARS, SECONDARY_VARS); erweitern für D-19
- `genossi-frontend/src/auth.rs` — `RequirePrivilege(privilege, children, fallback)`
- `genossi-frontend/src/api.rs` (2014 lines) — `AppError`, `status_to_message`, `check_response`, `send_bulk_mail`, Pattern-Vorlagen
- `genossi-frontend/src/router.rs` — Route-Enum mit `#[route(...)]`-Macros
- `genossi-frontend/src/service/member.rs` — `MEMBERS: GlobalSignal<MemberState>` + `refresh_members()`
- `genossi-frontend/src/i18n/mod.rs` — `Key`-Enum, `I18n`-Struct, `format_price`/`format_datetime`/`format_date`
- `genossi-frontend/src/i18n/de.rs` + `en.rs` — Match-Arm-Translate-Pattern
- `genossi-frontend/src/page/assembly_details.rs` — 3-Tab-Pattern Vorlage für `repayment_phase_details.rs`
- `genossi-frontend/src/page/mail_page.rs` (800 lines) — Multi-Select-Pattern + Send-Button-Body-Pattern
- `genossi-frontend/src/page/member_details.rs:495-562` — Page-Level-Edit-Toggle-Pattern (NICHT Inline-Cell)
- `genossi-frontend/src/component/top_bar.rs:46-185` — NavGroup-Befüllung
- `genossi-frontend/Cargo.toml` — Dioxus 0.6.3 verified
- `genossi_rest/src/repayment_phase.rs:337-371` — Route-Map verified
- `genossi_rest/src/repayment_entry.rs:361-401` — Route-Map verified (`/batch-status` VOR `/{id}` Reihenfolge wichtig)
- `genossi_rest/src/repayment_export.rs:163-178` — PDF-Export-Route verified
- `genossi_rest_types/src/lib.rs:1144-1422` — `RepaymentPhaseTO`, `RepaymentEntryTO`, `BatchStatusRequest`, `CloseConflictResponse`, `BatchFailureResponse` verified
- `.planning/phases/12-frontend-component-first/12-CONTEXT.md` — alle 28 Decisions

### Secondary (MEDIUM confidence)

- `genossi-frontend/CLAUDE.md` — Component-First-Principle (autoritativ); i18n-Patterns; bekannte Issues
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` — Globale Konventionen, Architecture Overview
- Memory `feedback_dioxus_button_type.md`, `feedback_component_first.md`, `feedback_verify_before_confirming.md`
- `.planning/STATE.md` Plan 04-10 Closure Notes — Phase-4-Frontend-Lektionen
- `.planning/ROADMAP.md` Phase 12 Section — Goal + 6 Success Criteria

### Tertiary (LOW confidence — needs Plan-Phase verification)

- A4 (Query-Param-Parsing-Strategie) — `web_sys::UrlSearchParams` vs. `dioxus-router` 0.6 Query-Param-Macros. Plan-Phase verifies.
- A2 (`wasm-bindgen-cli@0.2.104` Verfügbarkeit) — übernommen aus Phase-4-Pending. Plan-Phase verifies vor Release-Build.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — alle Versionen aus Cargo.toml + Code-Reads verifiziert
- Architecture: HIGH — Patterns aus Phase 4 dokumentiert; alle Reuse-Assets verifiziert vorhanden
- Reuse Asset Inventory: HIGH — alle Signatures direkt aus dem Code zitiert
- Integration Touchpoints: HIGH — Line-Number-Zitate aus aktuellem Code
- Pitfalls: HIGH — alle Pitfalls aus dokumentierten Hotfixes (e245013, c6f41fd, bb1be0b) und Phase-8-CR-01
- Backend-Surface: HIGH — Routes + TO-Schemas direkt aus Backend-Source verifiziert
- Validation Architecture: HIGH — Wave 0 Gap-Liste basiert auf existierendem Pattern in `member_search.rs::tests`
- Plan Decomposition: MEDIUM — 14-15 Plans ist Schätzung basierend auf Phase 4 (11 Plans); Plan-Phase kann umgruppieren

**Research date:** 2026-06-01
**Valid until:** 2026-06-30 (Frontend-Stack ist stabil; Dioxus 0.6 release ist > 6 Monate alt — keine Breaking-Changes erwartet)
