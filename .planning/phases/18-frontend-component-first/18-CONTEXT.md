# Phase 18: frontend-component-first - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 18 ist die **Frontend-Integration der vier v1.2-Membership-Adjust-Operationen** (Kündigung, Teil-Rückgabe, Übertrag, Aufstockung). Backend-Endpoints aus Phase 15/16/17 sind produktiv-fertig — Phase 18 bindet sie als shared `MembershipAdjustModal`-Component im Dioxus-WASM-Frontend an. Single-Button auf der Member-Detail-Page (Admin-only) öffnet eine Modal mit Sub-Choice (4 flat Buttons) und vier Operation-Sub-Views. Jede Sub-View hat eine Live-Vorschau im Form-Footer und ruft genau einen REST-Endpoint. i18n DE/EN.

**In scope:**

- **Neue Component `MembershipAdjustModal`** in `genossi-frontend/src/component/membership_adjust_modal.rs` (Component-First; PROJECT.md + CLAUDE.md hartes Constraint). Single File mit Enum-State (`ModalStep::SubChoice | Cancel | PartialRepayment | Transfer | Upgrade`) und `match`-rsx für die Sub-Views.
- **Sub-Choice-View** mit 4 flat Buttons (Kündigung / Teil-Rückgabe / Übertrag / Aufstocken) nebeneinander (2×2 oder 1×4 — Planner-Discretion via Tailwind grid). Klick auf einen Button setzt `ModalStep` und switcht zur Sub-View.
- **Vier Operation-Sub-Views** mit jeweils:
  - Header mit Operations-Name + Back-Pfeil (`←`) zur Sub-Choice
  - Form-Felder (FiscalYearDateInput, optional shares-Input, optional MemberSearch für Übertrag-Empfänger)
  - Live-Vorschau-Box im Footer mit Operations-spezifischem Text (siehe Roadmap-SC-3-Format)
  - Submit-Button (`type="button"` + onclick — Pattern aus memory [[feedback_dioxus_button_type]] gegen Page-Reload-Bug)
  - Abbrechen-Button (schließt Modal)
- **Neue Component `FiscalYearDateInput`** in `genossi-frontend/src/component/fiscal_year_date_input.rs` als wiederverwendbarer Datepicker mit GJ-Bounds. Native `<input type="date">` mit `min=today.year()-01-01` und `max=(today.year()+1)-12-31`. Bei Out-of-Range: Border-rot + Error-Text. Component-Schnittstelle: `(value: Signal<Option<Date>>, on_change: EventHandler<Date>, today: Date) -> Element`.
- **Button "Mitgliedschaft anpassen"** auf `genossi-frontend/src/page/member_details.rs` als Admin-only via `RequirePrivilege { privilege: "admin", ... }`. Click setzt lokales `show_adjust_modal`-Signal auf `true`. Modal wird conditional gemountet im `Modal { MembershipAdjustModal { ... } }`-Wrapper.
- **MemberSearch-Reuse für Übertrag-Empfänger** via Adapter-Pattern: TransferSubView ruft `use_resource(get_transfer_recipients(from_id))`, mappt `Vec<MemberSlimTO>` → `Vec<MemberTO>` (PII-Felder = None) und reicht an existing `MemberSearch`-Component weiter. Server-side `exit_date IS NULL`-Filter via Endpoint `GET /api/members/transfer-recipients?exclude_self={from_id}` (existing aus Phase 14).
- **Vier neue API-Client-Funktionen** in `genossi-frontend/src/api.rs`:
  - `cancel_membership(config, member_id, willensbekundung_date) -> Result<CancelResponse, AppError>` (Phase 15 D-15-11 Response-Shape `{action, member}`)
  - `partial_repayment(config, member_id, shares, willensbekundung_date) -> Result<PartialRepaymentResponse, AppError>` (Phase 16 D-16-16 Response-Shape `{entry, member, phase: Option}`)
  - `transfer_shares(config, from_id, to_id, shares, transfer_date) -> Result<TransferSharesResponse, AppError>` (Phase 17 D-15-15 + D-17-CF-07 Response-Shape `{actions, from, to}`)
  - `increase_shares(config, member_id, shares, willensbekundung_date) -> Result<IncreaseSharesResponse, AppError>` (Phase 15 Response-Shape `{action, member}`)
  - `get_transfer_recipients(config, exclude_self) -> Result<Vec<MemberSlimTO>, AppError>` (Phase 14)
- **Frontend-Live-Detection für Voll-Übertrag** in der TransferSubView: Sobald `shares == from.current_shares`, zeigt die Vorschau in Orange/Rot: `⚠ Voll-Übertrag — {first_name} {last_name} tritt am {transfer_date} aus` (i18n-Key). Mock-frei, Pure-Function-Logik im Component.
- **After-Success-Behavior** (alle 4 Operationen identisch): Auf 200-Response: `show_adjust_modal.set(false)` → `refresh_members().await` → grüner Toast (`show_toast(i18n.t(Key::MembershipAdjustSuccess{Op}))`). Member-Detail-Page rerendert automatisch mit neuen current_shares/exit_date.
- **Error-Handling** (alle 4 Operationen): API-Error → `ErrorAlert`-Component innerhalb der Modal anzeigen (NICHT Toast, da Modal-State erhalten bleibt). Submit-Button bleibt enabled. Vorstand kann Eingabe korrigieren und erneut submitten.
- **i18n DE/EN** mit ≥20 neuen Keys: Modal-Titles, Sub-Choice-Button-Labels, Form-Feld-Labels, Vorschau-Texte (mit Format-Args), Submit-Button-Labels, Voll-Übertrag-Warnung, Success-Toast-Messages, Validation-Errors. NUR DE/EN — keine Cs (genossi-frontend/CLAUDE.md harter Constraint).
- **ManualUAT-Sektion** in PLAN-Dokumenten mit Browser-Test-Anleitung pro Operation: (1) Aufruf member-detail/{id}, (2) Klick "Mitgliedschaft anpassen", (3) Sub-Choice klicken, (4) Datum/Anteile/Empfänger eingeben, (5) Vorschau verifizieren, (6) Submit, (7) Toast-Success + Member-Detail-Aktualisierung verifizieren.

**Out of scope (deferred / explizit nicht):**

- **Keine neuen Backend-Endpoints** — Phase 18 nutzt 15/16/17/14 unverändert.
- **Keine Bulk-Operationen** ("alle Mitglieder kündigen" aus PROJECT.md Out-of-Scope-Tabelle).
- **Kein Storno-Knopf für Übertrag/Kündigung** — bleibt manuelle MemberAction (FEATURES.md Out-of-Scope).
- **Keine Mitgliederliste-Integration** — Modal nur auf Member-Detail-Page (Roadmap explizit: "nicht in Liste — Audit-Bewusstsein").
- **Keine Anteilswert-Editierung** im Auto-Anlegen-Phase-Branch (Phase 16 D-16-05/06 — Vorstand korrigiert über existing v1.1 RepaymentPhase-UI nach Auto-Anlegen).
- **Keine Component-Aufteilung in 4 Sub-Form-Files** (Enum-State + match-rsx im selben File, ~400-600 LOC erwartet).
- **Keine globale Cache/Pre-Fetch für transfer-recipients** — on-mount via `use_resource`.
- **Keine Mehrstufigen Confirm-Dialoge** (CANC-06 wird durch Live-Vorschau im Form-Footer erfüllt — kein extra Dialog).
- **Keine Frontend-H1/H2-Berechnung** — Backend (Phase 14 `compute_effective_date`) ist Single-Source-of-Truth. Vorschau zeigt geplante Phase via separater Backend-Call ODER via Pure-Frontend-Mirror-Function (Planner-Discretion).
- **Keine A11y-Audit-Level über Standard hinaus** — Tailwind+Native-Inputs reichen für v1.2.
- **Keine Keyboard-Shortcuts** für Sub-Choice (Tab-Order ist Standard).

</domain>

<decisions>
## Implementation Decisions

### Sub-Choice-Form (Area 1)

- **D-18-01:** **4 flat Buttons** für Sub-Choice (Kündigung / Teil-Rückgabe / Übertrag / Aufstocken). **Why:** Klar, kein Nesting, jeder Workflow ist ein Klick zu erreichen. Vorstand sieht alle 4 Optionen sofort — maximale Discoverability. Roadmap "Discuss-Phase-Decision Phase 18: 4 flat vs. 3 mit Nesting vs. Quickpath" explizit als Discuss-Item markiert; Variante "4 flat" gewinnt aus FEATURES.md-Pro-Tabelle ("Klar, kein Nesting"). Vorstand-User-Base ist klein und trainiert — keine Quickpath-Optimierung nötig. **How to apply:** TailwindCSS `grid grid-cols-2 gap-4` oder `flex flex-row flex-wrap`, jeder Button mit Operations-Icon + Label + kurzer Beschreibung. Buttons sind gleich groß (auch der Kündigen-Button, kein Sonderstatus).

- **D-18-02:** **Single Modal mit Step-State + Back-Button (`←`).** Modal hat `let mut step = use_signal(|| ModalStep::SubChoice)`. Sub-Choice-Button-Klick: `step.set(ModalStep::Cancel/...)`. Header in Sub-View zeigt `← Mitgliedschaft anpassen · Kündigung` mit Back-Pfeil zur Sub-Choice. **Why:** Vorstand kann Operation wechseln ohne Modal zu schließen ("doch lieber Aufstockung als Kündigung"). Sub-Choice-Modal flackert nicht. Konsistent mit existing Multi-Step-Pattern (z.B. helper_login.rs nutzt einen Modal mit show_scanner-Toggle, aber ohne Step-State). **How to apply:** `#[derive(Clone, Copy, PartialEq)] enum ModalStep { SubChoice, Cancel, PartialRepayment, Transfer, Upgrade }`. Match-rsx im Component-Body. Back-Pfeil-Button setzt `step.set(ModalStep::SubChoice)` und reset Form-Felder (oder behält sie — Planner-Discretion).

- **D-18-03:** **Enum-State + match-rsx im selben File** (`component/membership_adjust_modal.rs`). Ein File mit erwartet ~400-600 LOC. **Why:** Component-First-Prinzip greift für die Component-Außenschnittstelle (ein wiederverwendbarer `MembershipAdjustModal`), nicht für interne Sub-Views (die nur intern relevant sind). State-Sharing zwischen Sub-Views (z.B. erhaltener `willensbekundung_date` falls Vorstand zwischen Cancel und Upgrade wechselt — Planner-Discretion) ist mit Enum-State trivial; separate Components würden Prop-Drilling oder Context erfordern. Roadmap-Phase-18-Constraint sagt "keine inline-RSX" — gemeint ist NICHT "keine match-Statements im Component", sondern "keine duplizierten RSX-Blöcke in Pages". Modal als shared Component ist Compliance. **How to apply:** Single `#[component] pub fn MembershipAdjustModal(props) -> Element`. Innere `match step.read()` switcht zwischen 5 rsx!-Blöcken. Pure helper-Funktionen (z.B. `fn render_cancel_subview()` returning Element) sind erlaubt um Lesbarkeit zu wahren.

- **D-18-04:** **Modal-Mount via `use_signal<bool>` Toggle auf Member-Detail-Page.** `let mut show_adjust_modal = use_signal(|| false)`. Button-onclick: `show_adjust_modal.set(true)`. Im rsx!: `if show_adjust_modal() { Modal { MembershipAdjustModal { member: member.clone(), on_close: move |_| show_adjust_modal.set(false), on_success: move |_| { /* refresh */ } } } }`. **Why:** Konsistent mit existing Pattern auf der `repayment_phase_details.rs:RepaymentEntryPaidOutConfirm`. Lokales State, kein globaler Context nötig (Roadmap explizit: nur Member-Detail-Page). `Modal { ... }`-Wrapper aus `component/modal.rs` re-used. **How to apply:** Im member_details.rs vor dem `rsx!`: `let mut show_adjust_modal = use_signal(|| false);`. RequirePrivilege-Wrap um den Button: `RequirePrivilege { privilege: "admin", Button { onclick: move |_| show_adjust_modal.set(true), "Mitgliedschaft anpassen" } }`.

### Vorschau-Flow (Area 2)

- **D-18-05:** **Einstufige Vorschau im Form-Footer (Live-Update).** Form-Felder oben (Datum, optional Anteile, optional Empfänger). Vorschau-Box unten, vom Form-State abgeleitet (`use_memo` oder direkt im rsx!). EIN roter Submit-Button am Ende der Modal (Operations-spezifisch: "Kündigung auslösen", "Teil-Rückgabe einreichen", "Übertrag ausführen", "Anteile aufstocken"). **Why:** Roadmap CANC-06 ("Vorschau-Confirm-Dialog zeigt Willensbekundungs-Datum, berechneten Stichtag, ...") wird durch Live-Vorschau erfüllt — der Vorstand sieht die Zahlen permanent, nicht erst nach einem zweiten Klick. Spart eine UI-Stufe und einen Klick. Vorstand klickt sowieso nicht-versehentlich auf "Kündigen" (Modal muss bewusst geöffnet werden, Sub-Choice gewählt, Form ausgefüllt). **How to apply:** Vorschau-Box mit Tailwind `bg-blue-50 border border-blue-200 rounded p-4`; Submit-Button `bg-red-600 hover:bg-red-700 text-white` (rot signalisiert "wirksamer Eingriff"). Submit ist nur enabled wenn alle Validierungen passen (Datum in Bounds, shares > 0, Empfänger selected).

- **D-18-06:** **Vorschau-Inhalt: Operations-spezifischer Text (Roadmap-SC-3-Format).**
  - Kündigung: `"{member_name}: {current_shares} Anteile (unverändert) · Stichtag: {effective_date} ({H1/H2}) · Auszahlung in Phase FY{fiscal_year}"`
  - Teil-Rückgabe: `"{member_name}: {current_shares} → {current_shares - n} Anteile (nach Auszahlung) · Stichtag: {effective_date} · Phase FY{fiscal_year}"` + Hinweis falls Auto-Create: `"⚠ Auszahlungsphase FY{fiscal_year} wird automatisch angelegt"`
  - Übertrag: `"{from_name}: {from_shares} → {from_shares - n} Anteile · {to_name}: {to_shares} → {to_shares + n} Anteile · Datum: {transfer_date}"`
  - Aufstockung: `"{member_name}: {current_shares} → {current_shares + n} Anteile · Datum: {willensbekundung_date}"`
  **Why:** Minimal, fokussiert auf das was sich ändert; matched Roadmap-SC-3 wortgenau. Kein generisches Tabellen-Layout — operations-spezifisch ist klarer. **How to apply:** Vorschau-Text in i18n-Keys mit `{}`-Placeholders. Frontend füllt via `i18n.t_format!(...)` oder Format-String (Planner-Discretion). Datum-Display als `format_date_german(date)` (DD.MM.YYYY) — neue Helper-Funktion in i18n.rs oder inline.

- **D-18-07:** **Voll-Übertrag-Warnung: Live-Detection im Form (TransferSubView).** Sobald `shares_input == from.current_shares`: Vorschau-Box zeigt zusätzlichen Hinweis in Orange/Rot: `⚠ Voll-Übertrag — {from_name} tritt am {transfer_date} aus`. Submit-Button bleibt erlaubt (Vorstand kann es bewusst auslösen), aber Warnung ist unübersehbar. **Why:** Vorstand sollte vor Submit wissen, dass Voll-Übertrag = Austritt (TRSF-05-Cascade). Kein Backend-Roundtrip nötig (frontend kann das `from.current_shares - shares == 0` lokal prüfen). Nach Submit zeigt Toast zusätzlich die erzeugten Actions (success message kann generisch "Übertrag ausgeführt" sein). **How to apply:** In TransferSubView-RSX nach Vorschau-Text: `if shares_input == from.current_shares { div { class: "mt-2 text-orange-700 font-bold", "⚠ {i18n.t_with_args(Key::FullTransferExitWarning, ...)}" } }`.

- **D-18-08:** **After-Success: Modal-Close + refresh_members() + grüner Toast.** Auf 200-Response von der API: `on_success.call(())` → Page setzt `show_adjust_modal.set(false)` UND `refresh_members().await` UND zeigt Toast `i18n.t(Key::MembershipAdjustSuccess{Op})`. **Why:** Konsistent mit existing v1.1-Pattern (RepaymentEntryPaidOutConfirm → Modal close + refresh + Toast). Member-Detail-Page rerendert via Signal-Cascade aus refresh_members(). Kein Workflow-Bruch durch Redirect. **How to apply:** `MembershipAdjustModal`-Props: `on_close: EventHandler<()>`, `on_success: EventHandler<()>`. Page bindet beide. Im Component nach `await api::cancel_membership(...)` → `match result { Ok(_) => on_success.call(()), Err(e) => error_signal.set(Some(e.to_string())) }`.

### Datepicker (Area 3)

- **D-18-09:** **Native HTML `<input type="date">` mit min/max-Attributen.** Browser-native Date-Picker, Mobile zeigt Touch-Picker, Browser macht Range-Validation. **Why:** Codebase nutzt das Pattern bereits (member_details.rs:31-79 — `format_date_input(d)` + `parse_date_input(s)` Helper). Kein neuer Code für Picker-Logik. Mobile-friendly (relevant da Vorstand auch mit Tablet/Smartphone arbeitet). Browser-Validation als first-line-defense. **How to apply:** `<input type="date" min={format_date_input(min)} max={format_date_input(max)} value={current_value} oninput={on_change}>`. Wert wird über `parse_date_input(event.value())` zurück in `time::Date` konvertiert. Helper-Funktionen entweder `pub`-machen oder duplizieren (kleine Funktionen, Planner-Discretion).

- **D-18-10:** **Neue `FiscalYearDateInput`-Component** in `genossi-frontend/src/component/fiscal_year_date_input.rs`. Props: `value: Signal<Option<Date>>, on_change: EventHandler<Date>, today: Date, error: Option<String>`. Component berechnet `min = Date::from_calendar_date(today.year(), Month::January, 1)` und `max = Date::from_calendar_date(today.year() + 1, Month::December, 31)` inline. Zeigt Native-Input + Helper-Text "Erlaubt: GJ {YYYY} oder {YYYY+1}" und bei Out-of-Range: rote Border + Error-Text. **Why:** Component-First-Prinzip — Datepicker wird in 4 Sub-Views (alle außer Sub-Choice) verwendet, also 4x Wiederverwendung sofort. Eigene Component ist testbar (Pure-Logic für min/max-Berechnung, Edge-Cases am Jahreswechsel). Vorbereitung für v1.3+ (z.B. wenn RepaymentPhase-Eingabe-Form auch GJ-Bounds braucht). **How to apply:** Datei + `#[component] pub fn FiscalYearDateInput(...)`. Re-export in `component/mod.rs`. Member-i18n-Key `Key::FiscalYearDateInputHelper` mit `{}`-Placeholder für Jahres-Range.

- **D-18-11:** **Validierung: Browser min/max (first-line) + Component-Border-rot bei Out-of-Range (UX-Feedback) + Submit-Button-Disabled.** Backend (Phase 15 D-15-05..08) lehnt redundant ab mit HTTP 400 (Defense-in-Depth). Frontend duplicate der Bounds-Logik ist hier OK (Range = aktuelles + nächstes Kalenderjahr, einfache Pure-Function), da UX-Feedback ohne Server-Roundtrip nötig ist. **Why:** Vorstand sieht sofort ob Datum valid ist, ohne Submit-Klick + Loading-Spinner + Error-Toast warten zu müssen. **How to apply:** Component prüft `is_valid_fiscal_year_date(date, today)` Pure-Function (Frontend-Mirror der Backend-Logik). Bei `false`: Border-rot via Tailwind `border-red-500`, Error-Text `i18n.t(Key::FiscalYearDateOutOfRange)`. Submit-Button-Disabled-State wird via Modal-Component aus dem Date-Signal abgeleitet (`use_memo` oder direkt im rsx!).

### MemberSearch für Übertrag (Area 4)

- **D-18-12:** **Server-side Daten via `GET /api/members/transfer-recipients?exclude_self={from_id}` (existing Phase 14) + Existing `MemberSearch`-Component (unverändert).** TransferSubView ruft `use_resource(move || api::get_transfer_recipients(config, from_id))`. Bei Loading: Spinner. Bei Erfolg: Liste an MemberSearch via Adapter-Pattern (D-18-13). **Why:** Server filtert `exit_date IS NULL`-Members (PERM-03 Backend-Side), liefert DSGVO-konformen `MemberSlimTO` (nur Member-Number, Name, Titel, Anrede — keine Email/PII). Frontend braucht keine eigene exit_date-Filter-Logik (Backend ist Single-Source-of-Truth). MemberSearch bleibt unverändert (kein Refactor-Risiko an existing v1.1 Phase 12 Usage). **How to apply:** Neue Funktion `pub async fn get_transfer_recipients(config: &Config, exclude_self: Uuid) -> Result<Vec<MemberSlimTO>, AppError>` in `api.rs`. Sub-View: `let recipients = use_resource(move || api::get_transfer_recipients(config, from_id))`. Match-loading/error/data.

- **D-18-13:** **MemberSlimTO → MemberTO Adapter-Pattern (Frontend-Mapping vor MemberSearch-Übergabe).** Pure Funktion `to_member_to(slim: &MemberSlimTO) -> MemberTO` in `service/member.rs` oder `component/member_search.rs`. Felder: `id = Some(slim.id), member_number, first_name, last_name, salutation, title — alle PII-Felder (email, phone, ...) = None/default`. **Why:** MemberSearch ist existing aus v1.1 Phase 12 mit MemberTO-Signatur. Refactor zu generischem Trait wäre teurer (Risiko an existing 12-Phase-Code, Trait-Bounds in Dioxus-Components nicht trivial). Adapter ist 5 Zeilen Code, klar isolierbar. **How to apply:** `let recipient_member_to_list: Vec<MemberTO> = slim_list.iter().map(to_member_to).collect()`. MemberSearch mit `members: recipient_member_to_list, on_select, exclude_id: None` (Backend hat schon exclude_self gefiltert).

- **D-18-14:** **Daten-Loading: On-Sub-View-Mount via `use_resource`.** Recipients werden erst beim Klick auf "Übertrag"-Sub-Choice-Button geladen — NICHT pre-fetched bei Modal-Öffnung. **Why:** Vorstand wählt häufig Kündigung/Teil-Rückgabe/Aufstockung — pre-fetched recipients wären verschwendet. `use_resource` cancelt automatisch bei Component-Unmount (z.B. Back-Button zur Sub-Choice). Konsistent mit existing Dioxus-Patterns. **How to apply:** Im TransferSubView-Component-Body: `let recipients = use_resource(move || async move { api::get_transfer_recipients(config.read(), from_id).await });`. Loading-Spinner: `match recipients.read().as_ref() { Some(Ok(list)) => MemberSearch::render(list), Some(Err(e)) => ErrorAlert {...}, None => Spinner {...} }`.

### Carry-Forward (locked aus PROJECT.md / CLAUDE.md / Phase 14-17)

- **C-18-CF-01 (PROJECT.md + genossi-frontend/CLAUDE.md + memory `feedback_component_first`):** **Component-First-Prinzip** — `MembershipAdjustModal` und `FiscalYearDateInput` MÜSSEN in `genossi-frontend/src/component/` extrahiert werden. Keine inline-RSX-Duplikate über mehrere Sub-Views ohne Helper-Funktionen.
- **C-18-CF-02 (genossi-frontend/CLAUDE.md):** **i18n nur DE/EN** — Phase 18 fügt KEINE Cs-Keys hinzu. Nur `de.rs` und `en.rs` editieren.
- **C-18-CF-03 (memory `feedback_dioxus_button_type`):** **Submit-Buttons im Modal verwenden `r#type: "button"` + `onclick`, NICHT `<form onsubmit>`** — Pattern aus Hotfix e245013 gegen Dioxus-Page-Reload-Bug. Alle 4 Submit-Buttons (Cancel/Partial/Transfer/Upgrade) folgen diesem Pattern.
- **C-18-CF-04 (Phase 14 D-14-12):** `GET /api/members/transfer-recipients?exclude_self={uuid}` existiert mit Sub-Route VOR `/{id}`-catch-all, liefert `MemberSlimTO` (id, member_number, first_name, last_name, salutation, title — 7-Feld-Whitelist).
- **C-18-CF-05 (Phase 15 D-15-11):** REST-Endpoint `POST /api/members/{id}/cancel` Response-Body: `{ action: MemberActionTO, member: MemberTO }`. Single-Round-Trip, kein POST→GET-Refresh nötig.
- **C-18-CF-06 (Phase 15 D-15-15):** REST-Endpoint `POST /api/members/{id}/increase-shares` Response-Body: `{ action: MemberActionTO, member: MemberTO }`.
- **C-18-CF-07 (Phase 16 D-16-16):** REST-Endpoint `POST /api/members/{id}/partial-repayment` Response-Body: `{ entry: RepaymentEntryTO, member: MemberTO, phase: Option<RepaymentPhaseTO> }`. `phase` nur befüllt wenn Auto-Anlegen passierte — Frontend zeigt Hinweis-Toast "Phase FY{YYYY} wurde automatisch angelegt".
- **C-18-CF-08 (Phase 17 D-15-15 + Phase 17):** REST-Endpoint `POST /api/members/{from_id}/transfer-shares` Response-Body: `{ actions: Vec<MemberActionTO>, from: MemberTO, to: MemberTO }`. 2 actions bei Teil-Übertrag, 3 actions bei Voll-Übertrag (Abgabe, Empfang, optional Austritt).
- **C-18-CF-09 (Phase 15 D-15-09 + D-14-08):** REST-Endpoints sind Sub-Routes registriert VOR `/{id}`-catch-all — relevant für API-Client (URLs sind korrekt aus Phase 15-17). Frontend muss nur die Pfade kennen.
- **C-18-CF-10 (Phase 15 D-15-12 + Phase 17 D-17-10):** HTTP-Status-Codes-Mapping: 200 (Success), 400 (Validation), 401 (no Auth), 403 (no admin Privilege), 404 (Member not found), 409 (Conflict: bereits-cancelled, optimistic-lock, recipient-cancelled-bei-Transfer). Frontend zeigt Response-Body-Error-Text in ErrorAlert.
- **C-18-CF-11 (existing `auth.rs:RequirePrivilege`):** Admin-Gate via `RequirePrivilege { privilege: "admin", children: { ... } }`. Privilege-Konstante ist `"admin"` (NICHT `"manage_members"` — Phase 15 D-15-01 etabliert ADMIN_PRIVILEGE für alle v1.2-Operationen).
- **C-18-CF-12 (existing `component/modal.rs`):** Modal-Wrapper-Component `Modal { children: Element }` ist wiederverwendbar. `MembershipAdjustModal` wird darin gewrappt: `Modal { MembershipAdjustModal { ... } }`.

### Claude's Discretion

- **Sub-Choice-Button-Layout (2×2 vs. 1×4)**: Planner entscheidet basierend auf erwarteter Modal-Breite. Empfehlung: 2×2 für Desktop (kompakter), 1×4 für Mobile via Tailwind responsive grid `grid grid-cols-2 sm:grid-cols-4`.
- **State-Reset beim Back-Pfeil**: Form-Felder bei Switch zwischen Sub-Views beibehalten oder reset — Planner-Discretion. Empfehlung: shared Felder (Datum, Member-ID) bleiben, operations-spezifische (shares-Input, recipient) werden reset.
- **Vorschau-Phase-Calculation Frontend vs. Backend**: Vorschau zeigt "Stichtag" und "Phase FY{YYYY}" für Cancel/PartialRepayment. Frontend kann:
  - (a) Pure-Mirror der Phase-14 `compute_effective_date`-Logik in Rust-WASM (5-10 LOC),
  - (b) Backend-Preview-Endpoint rufen (z.B. `GET /api/members/preview-effective-date?date=YYYY-MM-DD`),
  - (c) Vorschau zeigt nur das eingegebene Datum + Backend füllt Stichtag in der finalen Response (Vorschau ohne Stichtag-Anzeige).
  Empfehlung: (a) Pure-Mirror — zero-Roundtrip-UX, Logik ist deterministisch und stabil. Mit Unit-Test (Browser-side oder via wasm-bindgen-test) gegen Phase-14-Backend-Logic verifiziert.
- **Auto-Anlegen-Phase-Hinweis-Format**: Bei Partial-Repayment-Vorschau falls `target_year != latest_existing_phase.fiscal_year` → "⚠ Auszahlungsphase wird automatisch angelegt" anzeigen. Planner-Discretion ob Frontend das via Pure-Mirror erkennt oder erst nach Submit aus Response (Phase 16 Response hat `phase: Option`).
- **Toast-Component-Wahl**: Existing `component/toast.rs` (`show_toast`-API). Planner verifiziert Interface und nutzt.
- **i18n-Key-Naming-Convention**: Hierarchisch `MembershipAdjust{Sub}{Element}` (z.B. `MembershipAdjustCancelTitle`, `MembershipAdjustCancelButton`, `MembershipAdjustTransferRecipientLabel`, `MembershipAdjustFiscalYearHelper`, `MembershipAdjustSuccessCancel`). Planner-Discretion über genaue Liste. Mindestens 20 Keys (Roadmap-SC-4).
- **Pure-Frontend-Helpers**: `compute_effective_date_mirror`, `to_member_to`, `format_date_german`, `is_voll_uebertrag(shares, from_shares)` — alle als `pub(crate) fn` in den jeweiligen Files, mit `#[cfg(test)] mod tests` Edge-Cases.
- **Loading-Spinner-Component**: Planner nutzt existing oder erstellt `Spinner`-Component falls noch keiner existiert.
- **Error-Variant für "Empfänger-cancelled" (409 von Phase 17)**: Frontend könnte das in ErrorAlert pretty-print zeigen. Planner verifiziert dass Backend-Error-Body Text liest und passend in i18n übersetzt.
- **Modal-Höhe/Breite**: Tailwind `max-w-3/4 max-h-[90vh]` aus existing modal.rs ist OK. Planner kann pro Sub-View die innere Höhe anpassen.
- **Test-Strategie**: Unit-Tests für Pure-Helper-Funktionen (compute_effective_date_mirror, is_voll_uebertrag, format_date_german). ManualUAT-Sektion mit Browser-Test-Schritten. Keine Browser-Automation-Tests in dieser Phase (zu teuer für eine 4-Operation-UI).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Projekt-Foundation

- `.planning/PROJECT.md` — v1.2-Milestone, Constraints (Layered-Architektur, Component-First-Frontend, i18n DE/EN, ADMIN_PRIVILEGE für v1.2-Ops, Übertrag-Konsistenz-Story mit Voll-Übertrag-Austritt).
- `.planning/REQUIREMENTS.md` §UI-01..04 (Phase-18-Requirements: Button, Modal, Datepicker, Vorschau), §CANC-06 (Vorschau-Confirm-Dialog für Kündigung — durch Live-Vorschau im Form erfüllt).
- `.planning/ROADMAP.md` §Phase 18 — Phase-Goal, 4 Success-Criteria (MembershipAdjustModal, Datepicker+MemberSearch, Vorschau-Section pro Sub-View, Button + i18n + ManualUAT).
- `.planning/ROADMAP.md` §Constraints §Phase 18 — "Component-First: keine inline-RSX; alle Operationen-UI in MembershipAdjustModal".
- `.planning/ROADMAP.md` §Discuss-Phase-Decisions §Phase 18 — "Sub-Choice-Form (4 flat vs. 3 mit Nesting vs. Kündigung-Quickpath); Component-Reuse mit Phase-12-Pattern" (hier in D-18-01 als "4 flat" geklärt).

### Domain & Design

- `.planning/notes/membership-adjust-design.md` — Master-Design-Doc, insbesondere §UI ("Single-Button auf Member-Detail", "Vorschau-Bestätigungsdialog empfohlen", "Sub-Choice-Form offen für Discuss-Phase"), §Datums-Logik ("Datepicker default today(), nur offenes GJ + nächstes GJ erlaubt"), §Constraints ("Empfänger beim Übertrag muss aktives Mitglied sein").
- `.planning/research/FEATURES.md` §UI + §Sub-Choice-Form (Pro/Contra-Tabelle der 3 Varianten — Phase 18 wählt Variante 1 "4 flat Buttons" basierend auf D-18-01).
- `.planning/research/SUMMARY.md` — v1.2-Research-Synthesis (Frontend-Pattern-Quelle: v1.1 Phase 12 `RepaymentEntryPaidOutConfirm`).
- `.planning/codebase/STRUCTURE.md` §genossi-frontend (Component-Verzeichnis, Page-Verzeichnis, Pages müssen Components komponieren).

### Backend-API-Contract (Phase 14-17 Response-Shapes)

- `.planning/phases/14-dao-domain-foundation/14-CONTEXT.md` D-14-12 — `MemberSlimTO`-Felder (Whitelist: id, member_number, first_name, last_name, salutation, title — keine PII), `GET /api/members/transfer-recipients?exclude_self={uuid}`-Endpoint.
- `.planning/phases/15-service-rest-kuendigung-aufstockung/15-CONTEXT.md` D-15-09..12, D-15-15 — Sub-Routes `POST /api/members/{id}/cancel` + `POST /api/members/{id}/increase-shares`, Response-Shape `{ action, member }`, HTTP-Status-Codes 200/400/401/403/404/409.
- `.planning/phases/16-service-rest-teil-rueckgabe-auto-anlegen-phase/16-CONTEXT.md` D-16-14..16 — `POST /api/members/{id}/partial-repayment`, Response-Shape `{ entry, member, phase: Option }`, Auto-Anlegen-Hinweis im Frontend.
- `.planning/phases/17-service-rest-uebertrag-cascade/17-CONTEXT.md` D-17-10 (Error-Mapping-Tabelle), Carry-Forward C-17-CF-07 — `POST /api/members/{from_id}/transfer-shares`, Response-Shape `{ actions, from, to }`.

### Frontend-Pattern-Quelle (v1.0/v1.1)

- `genossi-frontend/CLAUDE.md` — Component-First-Prinzip (HARTER Constraint), i18n nur DE/EN (Cs existiert NICHT — korrigiert 2026-05-04), WeekView-Zoom-Pattern (irrelevant für Phase 18).
- `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` — Vorbild für Modal mit Submit-Loop, Loading-State, Error-Toast, on_close/on_complete-EventHandler-Pattern (v1.1 Phase 12 D-15..D-17). **Pattern für Phase 18 Modal.**
- `genossi-frontend/src/component/member_search.rs` — Existing MemberSearch (`filter_members(members, query, exclude_id)` Pure-Function + `MemberSearch`-Component mit `on_select: EventHandler<Option<Uuid>>`, `selected_id`, `exclude_id`). Wird für Übertrag-Empfänger via Adapter wiederverwendet (D-18-12, D-18-13).
- `genossi-frontend/src/component/modal.rs` — `Modal { children: Element }`-Wrapper mit fixed-overlay-Style. Wiederverwendet in D-18-04.
- `genossi-frontend/src/component/error_alert.rs` — Existing ErrorAlert für API-Errors im Modal (D-18-08).
- `genossi-frontend/src/component/toast.rs` — Existing show_toast-API für After-Success-Toast (D-18-08).
- `genossi-frontend/src/auth.rs` — `RequirePrivilege { privilege: "admin", children }` für Button-Gate (C-18-CF-11).
- `genossi-frontend/src/page/member_details.rs` — Member-Detail-Page (1560 LOC), wird um Button + Modal-Mount erweitert. **Datepicker-Helper-Funktionen** `format_date_input(d)` (Z.31) + `parse_date_input(s)` (Z.65-79) für ISO8601-Date-String-Conversion — Phase 18 macht sie `pub(crate)` für FiscalYearDateInput-Wiederverwendung ODER dupliziert minimal.
- `genossi-frontend/src/page/member_details.rs:1357` — Existing Admin-Check-Pattern (`has_privilege("manage_members") || has_privilege("admin")`) als Reference.
- `genossi-frontend/src/page/repayment_phases.rs:65` — Existing `RequirePrivilege`-Usage als Reference.
- `genossi-frontend/src/i18n/{de,en}.rs` — i18n-Translation-Files. Mindestens 20 neue Keys werden hier eingetragen (DE + EN, KEINE Cs).
- `genossi-frontend/src/i18n/mod.rs` — `Key`-Enum mit neuen Varianten + `Locale`-Enum (NUR `En`+`De`, kein `Cs`).
- `genossi-frontend/src/api.rs` — Existing API-Client mit `pub async fn get_member(...)`, `pub async fn update_member(...)` etc. Phase 18 fügt 5 neue Funktionen hinzu (cancel, increase_shares, partial_repayment, transfer_shares, get_transfer_recipients).
- `genossi-frontend/src/service/member.rs:11` — `refresh_members()`-Funktion für After-Success-Behavior.
- `genossi-frontend/rest-types/` — Symlink/Copy auf `genossi_rest_types`. Phase 18 importiert MemberTO, MemberSlimTO, MemberActionTO, RepaymentEntryTO, RepaymentPhaseTO, neue Request-DTOs (`CancelMembershipRequestTO`, `IncreaseSharesRequestTO`, `PartialRepaymentRequestTO`, `TransferSharesRequestTO`) und ggf. Response-DTOs falls benannt (oder nutzt serde_json::Value).

### Backend-Service-Codepaths (für Frontend-Mirror falls Pure-Function nachgebaut wird)

- `genossi_service_impl/src/membership_adjust.rs` — `compute_effective_date(willensbekundung)` Pure-Function (Phase 14). Frontend-Mirror in Rust-WASM ist 5-10 LOC (Planner-Discretion D-18 Claude's-Discretion).
- `genossi_service_impl/src/membership_adjust.rs` — `validate_willensbekundung_date(date, today)` Pure-Function (Phase 15 D-15-05). Frontend-Mirror identische Logik.

### Memory & Persistente Lessons

- Memory `feedback_component_first` — Component-First-Prinzip ist HART (Lesson: identische UI auf zwei Seiten → extract in `src/component/`).
- Memory `feedback_dioxus_button_type` — `r#type: "button"` + onclick statt form-onsubmit (Hotfix e245013). MUSS für ALLE 4 Submit-Buttons im Modal angewendet werden.
- Memory `feedback_use_jj_not_git` — `jj commit -m ...` statt `git commit -m ...` für alle Commits in Phase 18.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`Modal { children: Element }`** (`component/modal.rs`): Wrapper-Modal mit fixed-overlay, `bg-white max-w-3/4 max-h-[90vh] p-8 overflow-y-auto rounded-lg shadow-lg`. Wird in D-18-04 als Outer-Wrapper für `MembershipAdjustModal` genutzt.
- **`MemberSearch`** (`component/member_search.rs`): Pure-Function `filter_members(members, query, exclude_id) -> Vec<&MemberTO>` + Dioxus-Component `MemberSearch { on_select, selected_id, exclude_id }`. **MAX_RESULTS = 10**. Wird unverändert wiederverwendet (D-18-12) via Adapter-Pattern (D-18-13).
- **`RequirePrivilege`** (`auth.rs:35-48`): `RequirePrivilege { privilege: &'static str, children, fallback }`. Privilege-Check via `AUTH.read().auth_info.has_privilege(...)`. Default-Fallback ist eine Access-Denied-Div. Wird für Button-Gate in member_details.rs verwendet (C-18-CF-11).
- **`ErrorAlert`** (`component/error_alert.rs`): Wird in der Modal für API-Errors verwendet (D-18-08).
- **`show_toast`** (`component/toast.rs` API): Wird für Success-Toast nach Modal-Close verwendet (D-18-08).
- **`format_date_input(d)` + `parse_date_input(s)`** (`page/member_details.rs:31-79`): ISO8601-Date↔String-Helper. Werden vom neuen `FiscalYearDateInput` wiederverwendet (D-18-09 + D-18-10).
- **`refresh_members()`** (`service/member.rs:11`): Async-Funktion die MEMBERS-Signal aus Backend neu lädt. Wird in D-18-08 After-Success aufgerufen.
- **`api::get_member`, `api::update_member`** (`api.rs:188-225`): Pattern für neue API-Client-Funktionen (gleich aufgebaut: `format!("{}/api/...", config.backend)`, POST/PUT mit `body(serde_json::to_string(&payload)?)`).
- **i18n-System** (`i18n/mod.rs`): `use_i18n()`-Hook, `Key`-Enum, `i18n.t(Key::...)`-Lookup. Format-Args via i18n.t_format!-Macro (existing-Pattern verifizieren).
- **`v1.1 Phase 12 RepaymentEntryPaidOutConfirm`** (`component/repayment_entry_paidout_confirm.rs`): Vorbild für Modal mit on_close/on_complete-EventHandler-Pattern, Listentabelle-Render, Loop-Submit-State. **Strukturelles Vorbild** für `MembershipAdjustModal` (D-15..D-17 Pattern).

### Established Patterns

- **Component-First** (CLAUDE.md HART): Komponenten in `src/component/`, Pages komponieren. KEINE inline-RSX-Duplikate über Pages. `MembershipAdjustModal` MUSS Component sein.
- **i18n DE/EN only** (CLAUDE.md HART): Nur `de.rs` und `en.rs` editieren. NICHT `cs.rs` (existiert nicht).
- **Dioxus Button-Reload-Bug-Pattern** (memory): `r#type: "button"` + onclick statt form-onsubmit.
- **`use_signal<T>` Toggle-Pattern**: Lokales State für Modal-Open/Close auf Page (D-18-04). Existing Beispiel: `repayment_phase_details.rs` (paidout-confirm Toggle).
- **`use_resource` Async-Loading-Pattern**: Für API-Calls die beim Mount geladen werden (D-18-14). Standard-Dioxus-Pattern.
- **Page komponiert Component mit on_close/on_success-Handlers**: D-18-04 + D-18-08 folgen Phase-12-Pattern.
- **API-Client-Funktionen in `api.rs`** mit `format!("{}/api/...", config.backend)` + `reqwest`-Client. JSON-Body via `serde_json::to_string` + `body(...)`-Method.
- **Browser-Native-Date-Input mit min/max** (D-18-09): Codebase-Pattern aus `member_details.rs:Eintrittsdatum`-Input.
- **Error-Display in Modal via `ErrorAlert`** (NICHT Toast — Modal-State erhalten bleibt) (D-18-08).
- **Submit-Button-State**: `disabled` Attribut via `use_memo`/Signal abgeleitet. Loading-Spinner während Submit (existing Pattern aus v1.1-Modals).

### Integration Points

- **`page/member_details.rs`**: Button + Modal-Mount, etwa zwischen den existing Tabs/Sections. Konkrete Platzierung Planner-Discretion (z.B. im "Aktionen"-Tab oder im Header neben "Bearbeiten"-Button).
- **`component/mod.rs`**: Re-Export für `MembershipAdjustModal` und `FiscalYearDateInput`.
- **`api.rs`**: Neue async fn cancel_membership, increase_shares, partial_repayment, transfer_shares, get_transfer_recipients.
- **`i18n/mod.rs`**: Neue `Key`-Enum-Varianten (mind. 20).
- **`i18n/de.rs` + `i18n/en.rs`**: Translations für alle neuen Keys.
- **`router.rs`**: Keine Änderung — Modal wird auf bestehender `/members/{id}`-Route gemountet.
- **`rest-types/`**: Re-Exports prüfen — `MemberSlimTO` muss exportiert sein, falls Phase 14 das übersehen hat. CancelMembershipRequestTO, IncreaseSharesRequestTO, PartialRepaymentRequestTO, TransferSharesRequestTO müssen exportiert sein.

</code_context>

<specifics>
## Specific Ideas

- **i18n-Key-Naming-Convention** (Vorschlag): `MembershipAdjust{Sub}{Element}` hierarchisch.
  - Sub-Choice: `MembershipAdjustModalTitle`, `MembershipAdjustSubChoiceQuestion`, `MembershipAdjustSubChoiceCancel/PartialRepayment/Transfer/Upgrade`
  - Cancel: `MembershipAdjustCancelTitle`, `MembershipAdjustCancelDateLabel`, `MembershipAdjustCancelPreview` (mit Format-Args), `MembershipAdjustCancelSubmit`, `MembershipAdjustCancelSuccess`
  - PartialRepayment: `MembershipAdjustPartialRepaymentTitle`, `...DateLabel`, `...SharesLabel`, `...Preview`, `...AutoCreatePhaseHint`, `...Submit`, `...Success`
  - Transfer: `MembershipAdjustTransferTitle`, `...DateLabel`, `...SharesLabel`, `...RecipientLabel`, `...RecipientLoadingError`, `...Preview`, `...FullTransferExitWarning`, `...Submit`, `...Success`
  - Upgrade: `MembershipAdjustUpgradeTitle`, `...DateLabel`, `...SharesLabel`, `...Preview`, `...Submit`, `...Success`
  - FiscalYearDateInput: `FiscalYearDateInputHelper`, `FiscalYearDateOutOfRange`
  - Button auf Page: `MembershipAdjustButtonLabel`
  - **Mindest-Count:** ~30 neue Keys (übersteigt Roadmap-SC-4 Mindest-20 deutlich, aber sinnvoll für Vollständigkeit).
- **Sub-Choice-Button-Styling-Vorschlag** (Tailwind):
  ```rust
  div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
      button {
          r#type: "button",
          class: "flex flex-col items-start p-6 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition",
          onclick: move |_| step.set(ModalStep::Cancel),
          span { class: "text-lg font-semibold", "{i18n.t(Key::MembershipAdjustSubChoiceCancel)}" }
          span { class: "text-sm text-gray-600", "{i18n.t(Key::MembershipAdjustSubChoiceCancelDesc)}" }
      }
      // ... 3 weitere
  }
  ```
- **Vorschau-Box-Styling-Vorschlag**:
  ```rust
  div { class: "mt-4 p-4 bg-blue-50 border border-blue-200 rounded",
      h4 { class: "font-semibold text-sm uppercase text-blue-900 mb-2", "Vorschau" }
      p { class: "text-sm", "{vorschau_text}" }
      if is_voll_uebertrag {
          p { class: "mt-2 text-orange-700 font-bold", "⚠ {voll_uebertrag_warning_text}" }
      }
  }
  ```
- **Submit-Button-Styling-Vorschlag** (rote Farbe signalisiert "wirksamer Eingriff"):
  ```rust
  button {
      r#type: "button",
      class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-semibold disabled:bg-gray-300 disabled:cursor-not-allowed",
      disabled: !is_valid || submitting(),
      onclick: move |_| spawn(async move { /* call API */ }),
      if submitting() { "..." } else { "{i18n.t(Key::MembershipAdjustCancelSubmit)}" }
  }
  ```
- **FiscalYearDateInput-Skelett-Vorschlag** (~50 LOC):
  ```rust
  #[component]
  pub fn FiscalYearDateInput(
      value: Signal<Option<time::Date>>,
      on_change: EventHandler<time::Date>,
      today: time::Date,
  ) -> Element {
      let i18n = use_i18n();
      let min_year = today.year();
      let max_year = today.year() + 1;
      let min_str = format!("{:04}-01-01", min_year);
      let max_str = format!("{:04}-12-31", max_year);
      let value_str = value.read().as_ref().map(format_date_input).unwrap_or_default();
      let is_oor = value.read().as_ref().map_or(false, |d| d.year() < min_year || d.year() > max_year);

      rsx! {
          div { class: "flex flex-col",
              input {
                  r#type: "date",
                  min: "{min_str}",
                  max: "{max_str}",
                  value: "{value_str}",
                  class: if is_oor { "border-red-500" } else { "border-gray-300" },
                  oninput: move |e| {
                      if let Some(d) = parse_date_input(&e.value()) {
                          value.set(Some(d));
                          on_change.call(d);
                      }
                  }
              }
              if is_oor {
                  span { class: "text-red-600 text-sm", "{i18n.t(Key::FiscalYearDateOutOfRange)}" }
              }
              span { class: "text-gray-500 text-xs",
                  "{i18n.t_with_args(Key::FiscalYearDateInputHelper, [min_year, max_year])}"
              }
          }
      }
  }
  ```
- **Pure-Frontend-Mirror `compute_effective_date_mirror`** (Phase 14 Logic):
  ```rust
  pub(crate) fn compute_effective_date_mirror(willensbekundung: time::Date) -> (i32, time::Date) {
      let year = willensbekundung.year();
      if willensbekundung.month() <= time::Month::June {
          (year, time::Date::from_calendar_date(year, time::Month::December, 31).unwrap())
      } else {
          (year + 1, time::Date::from_calendar_date(year + 1, time::Month::December, 31).unwrap())
      }
  }
  // Unit-Test gegen Phase-14-Backend-Edge-Cases (6 Tests: 30.06, 01.07, 31.12, 01.01, Schaltjahr-Februar, GJ-Boundaries).
  ```
- **`is_voll_uebertrag(shares, from_current_shares)` Pure-Helper**:
  ```rust
  pub(crate) fn is_voll_uebertrag(shares: i64, from_current_shares: i64) -> bool {
      shares >= 1 && from_current_shares - shares == 0
  }
  ```
- **ManualUAT-Browser-Test-Anleitung-Format-Vorschlag** (pro Operation):
  ```markdown
  ### UAT: Kündigung
  1. Login als Vorstand (Admin).
  2. Navigiere zu /members/{id} eines aktiven Members.
  3. Klick "Mitgliedschaft anpassen" → Modal öffnet.
  4. Klick "Kündigung" → Form öffnet.
  5. Datum-Input: 2026-06-15. Vorschau zeigt: "Member X: 5 Anteile (unverändert) · Stichtag: 31.12.2026 · Auszahlung in Phase FY2026".
  6. Klick roter "Kündigung auslösen"-Button.
  7. Verifizieren: Modal schließt, grüner Toast "Kündigung ausgelöst", Member-Detail zeigt neue exit_date.
  ```

</specifics>

<deferred>
## Deferred Ideas

- **Mehrstufiger Workflow (Antrag → Genehmigung → Wirksamkeit)** — Vier-Augen-Prinzip ist Future (PROJECT.md Out-of-Scope für v1.2).
- **Bulk-Operationen ("alle Mitglieder kündigen")** — FEATURES.md Out-of-Scope.
- **Storno-Knopf für Übertrag/Kündigung im Modal** — bleibt manuelle MemberAction-UI (Phase 17 deferred ideas).
- **Mitgliederliste-Integration (Button auch in der Liste)** — Roadmap explizit: "nicht in Liste — Audit-Bewusstsein durch extra Klick" (CLAUDE.md + PROJECT.md).
- **Anteilswert-Editierung im Auto-Anlegen-Phase-Branch** — Vorstand nutzt existing v1.1 RepaymentPhase-UI nach Auto-Anlegen (D-16-05/06 Carry-Forward).
- **Globaler Cache + Pre-Fetch für transfer-recipients** — On-Mount via `use_resource` ist ausreichend (D-18-14).
- **Backend-Preview-Endpoint** `GET /api/members/preview-effective-date` — kein Backend-Endpoint hinzufügen; Frontend macht Pure-Mirror (Claude's Discretion).
- **Keyboard-Shortcuts für Sub-Choice** (z.B. Cmd+1 = Kündigung) — Tab-Order ist Standard, Vorstand klickt.
- **Browser-Automation-Tests (Playwright/Selenium)** — zu teuer für eine 4-Operation-UI in v1.2. ManualUAT reicht.
- **Generischer MemberSearch-Trait-Refactor** — Adapter-Pattern (D-18-13) ist 5 Zeilen Code, Trait-Refactor wäre Phase-12-Regression-Risk. Deferred bis Trait-basierte Member-Search in v1.3+ kommt.
- **`MembershipAdjustModal` in v1.3 als Top-Bar-Quick-Action** — z.B. global verfügbar mit Member-Picker, nicht nur Member-Detail. Bleibt v1.3+.

</deferred>

---

*Phase: 18-frontend-component-first*
*Context gathered: 2026-06-06*
