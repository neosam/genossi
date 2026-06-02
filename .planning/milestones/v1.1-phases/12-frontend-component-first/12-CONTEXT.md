# Phase 12: Frontend (Component-First) - Context

**Gathered:** 2026-06-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Dioxus-WASM-UI für die Vorstand-Verwaltung von `RepaymentPhase`/`RepaymentEntry`. Backend-REST ist komplett verdrahtet (Phasen 7–11) — Phase 12 konsumiert ausschließlich existierende Endpoints, fügt KEINE Backend-Routen, Migrations oder Service-Logik hinzu. Liefert sechs Frontend-Bausteine, alle Component-First in `genossi-frontend/src/`:

1. **Listen-Page `/repayment-phases`** (UI-01): Tabelle aller Phasen mit Status, fiscal_year, share_value, Anzahl-Einträge; sortierbar; Create-Phase-Modal.
2. **Detail-Page `/repayment-phases/{id}`** (UI-02): 3-Tab-Layout (Stamm-Daten / Einträge / Export) mit Lifecycle-Aktionen.
3. **Shared `RepaymentEntryList`-Component** (UI-03): multi-select, Status-Filter, sortierbar nach Mitgliedsnummer/Status, Inline-Edit der Anteile.
4. **Add-Entry-Modal** (UI-04): Member-Picker (Substring-Suche) + `share_count_to_pay_out`-Eingabe.
5. **`ausbezahlt`-Confirm-Dialog** (UI-05) mit Warnung „irreversibel / audit-pflichtig / reduziert current_shares" + Backend-Validation-Fehler-Toast.
6. **Massenmail-Aktion** (UI-06): Navigation in die bestehende `/mail`-Page mit Pre-Selection und `repayment_phase_id`-Kontext.

**Phase 12 liefert NICHT:**
- Backend-Änderungen (keine neuen Routen, keine Migrations, keine Service-Logik)
- SEPA pain.001 XML-Export (SEPA-01 deferred zu v2)
- CSV-Export (EXPO-04 deferred per Phase-11 D-12)
- WASM-Test-Setup (Genossi hat aktuell kein `wasm-bindgen-test` — Verify-Phase nutzt UAT-Checkliste analog Phase 4)
- Re-Open einer abgeschlossenen Phase (PHAS-03 ist final-only)
- Brief-Anschreiben-Automatik (BRIEF-01 deferred zu v2)

</domain>

<decisions>
## Implementation Decisions

### Cross-Cutting: Button-Pattern (verpflichtend, Grep-Gate)

- **D-01:** **Jeder neue Action-Button in Phase 12 MUSS `r#type: "button"` explizit setzen** und `onclick` mit `MouseEvent`-Handler verwenden — NIE `<form onsubmit={...}>` mit `<button type="submit">`. `<form>` nur, wenn echte Form-Semantik nötig (Enter-Submit auf Text-Input); sonst `<div>`. Bei legitimen Forms: Handler synchron, `prevent_default()` zuerst, async `spawn` danach. Rationale: HTML-Default ist `type="submit"`; trotz `prevent_default` triggert Klick in Forms (insbesondere mit `spawn(async ...)`-Handlern) verlässlich einen Page-Reload — siehe Hotfix-Commits `e245013` (16 GV-Buttons), `c6f41fd` (form→div), `bb1be0b` (PDF-Toggle). Memory: `feedback_dioxus_button_type.md`.
- **D-02:** **Grep-Gate-Test im Plan-Acceptance-Kriterium:** `rg 'button\s*\{' genossi-frontend/src/component/repayment_* genossi-frontend/src/page/repayment_*` darf KEINEN Treffer ohne `r#type:` haben. Plan-Phase fixiert das als Pre-Merge-Check; Verify-Phase verifiziert es nach Implementation. Pattern-Anker für künftige Frontend-Phasen.

### Detail-Page Lifecycle-UX

- **D-03:** **Lifecycle-Buttons („Öffnen" / „Schließen") sitzen im Stamm-Daten-Tab als große Action-Kachel** — NICHT im Page-Header und NICHT verstreut über mehrere Tabs. Vorstand muss aktiv den Stamm-Tab öffnen, um den Lifecycle-Schritt zu triggern (intentionale Reibung gegen versehentliche Aktion). Page-Header bleibt sauber: Titel + Status-Badge.
- **D-04:** **Schließen-409-Reaktion (Backend liefert `CloseConflictResponse { error, pending_count, pending_member_numbers }`):** Toast mit deutscher Fehlermeldung („Schließen blockiert: N Einträge noch nicht ausbezahlt"), KEIN Auto-Tab-Switch. Bestehendes `error_alert.rs`/Toast-Pattern reusen. Vorstand navigiert selbst zum Einträge-Tab. Rationale: simpel, kein Cross-Component-Tab-State-Trigger nötig.
- **D-05:** **`share_value`-Korrektur (PHAS-04) lebt inline im Stamm-Daten-Tab** als editierbares Feld mit „Speichern"-Button. Drei Render-Modi:
  - `Vorbereitung`: editierbar (initialer Wert).
  - `Offen`: editierbar mit Hinweistext „Korrektur wird auditiert" — POST `PUT /api/repayment-phase/{id}` mit neuem `share_value` + `version`.
  - `Abgeschlossen`: read-only.
  Pattern-Vorlage: editable Felder in `member_details.rs`.
- **D-06:** **'Vorbereitung'-Status-Anzeige der 3 Tabs:** Tabs immer sichtbar; Einträge-Tab und Export-Tab zeigen eine Hinweis-Box „Phase noch nicht geöffnet" (analog Phase-4 D-13: Assembly-Anwesenheits-Tab im `Preparation`-Status). Tab-Strip bleibt strukturell identisch über alle Status — Vorstand lernt das Layout einmal.
- **D-07:** **'Schließen'-Aktion hat einen Confirm-Modal vor POST:** „Phase final abschließen? Alle Einträge sind ausbezahlt. Diese Aktion ist nicht rückgängig machbar." Konsistent zum `ausbezahlt`-Confirm-Pattern (D-15). '`Öffnen`' hat KEIN Confirm (Effekt ist reversibel-im-Edit-Sinn — Einträge kann man bearbeiten/löschen).
- **D-08:** **Detail-Page nach Status `Abgeschlossen`:** alle Felder read-only. Einträge-Tab zeigt die Liste OHNE Edit-/Delete-/Toggle-Aktionen. Export-Tab bleibt voll aktiv (EXPO-01: PDF für Offen UND Abgeschlossen).
- **D-09:** **Nach erfolgreichem '`Öffnen`'-POST:** Page reloaded den Phase-State (neuer Status + neue `version`). KEIN Auto-Tab-Switch zum Einträge-Tab. Vorstand klickt selbst zum Einträge-Tab, um die Auto-Befüllung zu sehen. Rationale: spart Cross-Component-Tab-State-Glue; tab-strip ist intern state-getrieben. Bei `N=0` Auto-Befüllung zeigt der Einträge-Tab später eine Empty-State-Box mit Hinweis + Add-Entry-CTA (D-13).

### RepaymentEntryList Component (UI-03)

- **D-10:** **Spalten-Set (7 Spalten):** Mitgliedsnummer, Name, Anteile, Betrag, IBAN, Status, Actions. Betrag = `share_count_to_pay_out × phase.share_value` (Frontend rechnet, deutsche Formatierung „60,00 €" — Pattern aus Phase 10 D-04). IBAN-Spalte zeigt fehlende IBAN als „—". Member-Daten via Client-Side-Join aus `GET /api/members` (`MEMBERS`-Global-Signal existiert).
- **D-11:** **Multi-Select-Pattern:** Per-Row-Checkbox links + Header-Checkbox „Alle auswählen". Immer sichtbar (kein Hover-only — Tablet-tauglich). Action-Buttons („Massenmail", „Als angeschrieben markieren", „Als ausbezahlt markieren") in einer Header-Action-Leiste oberhalb der Tabelle, jeweils mit Count-Badge (z.B. „Mail an 3 ausgewählte"). Bei `0` Selection sind die Bulk-Buttons disabled.
- **D-12:** **Status-Filter als Tab-Strip-im-Tab:** „Alle | Offen | Angeschrieben | Ausbezahlt" mit Count-Badges. Filter ist client-side (Backend `GET /api/repayment-entry?phase_id=` liefert immer alle; Phase-8 D-10).
- **D-13:** **`share_count_to_pay_out`-Edit als Inline-Cell-Edit:** Klick auf die Anteile-Zelle wechselt die Zelle in einen Input + „✓ Speichern" / „✗ Abbrechen". Submit triggert `PUT /api/repayment-entry/{id}` mit neuem `share_count` + aktuelle `version`. Nur in Status `offen` oder `angeschrieben` (ENTR-04 Backend-Lock; UI rendert non-editable bei `ausbezahlt`). Neuer Component-Baustein `editable_cell.rs` (vermutlich generischer Helper).
- **D-14 (Defaults — Claude's Discretion):**
  - **Default-Sort:** Mitgliedsnummer ASC, Sekundär `created ASC` (konsistent mit PDF-Sort Phase 11 D-09).
  - **Empty-State:** zentrierte Box mit Hinweistext + Add-Entry-CTA. Texte:
    - Phase geöffnet + 0 Auto-Befüllt: „Keine Einträge — Vorjahres-Austritte fehlen. Eintrag manuell hinzufügen."
    - Status-Filter ergibt 0 Treffer: „Keine Einträge mit Status [X]."
  - **Soft-Delete:** Trash-Icon in Action-Spalte, nur sichtbar wenn Status ≠ `ausbezahlt` (ENTR-05). Klick → Confirm-Modal („Eintrag löschen?") → `PUT` mit `deleted`-Timestamp.
  - **Status-Badge-Farben:** Offen=grau, Angeschrieben=blau, Ausbezahlt=grün. Neuer `repayment_entry_status_badge.rs` analog zu `assembly_status_badge.rs`.

### `ausbezahlt`-Confirm + PaidOut-Flow (UI-05)

- **D-15:** **`ausbezahlt`-Toggle ist Single-Endpoint im Backend** (`POST /api/repayment-entry/{id}/mark-paid-out`; batch-status verbietet PaidOut, D-07 in `repayment_entry.rs`). Frontend implementiert **Bulk-Toggle als Sequential-Loop**: ein einziger Sammel-Confirm-Modal am Anfang, dann pro selektierten Entry seriell POST. Bei Fehler-in-der-Mitte: Toast „X von N erfolgreich, Y fehlgeschlagen — siehe Status-Spalte"; die bereits-erfolgreichen bleiben bestehen (Backend-Atomarität pro Entry, NICHT über die ganze Batch).
- **D-16:** **Confirm-Modal-Inhalt:**
  - Listentabelle der ausgewählten Einträge: Mitgliedsnummer | Name | Anteile | Betrag
  - Gesamtsumme unten („Summe: 4.500,00 €")
  - 3-Punkt-Warnliste:
    - „⚠ Diese Aktion ist final — kein Rückweg möglich."
    - „⚠ Erzeugt einen Verkauf-Audit-Eintrag pro Mitglied (`MemberAction::Verkauf` mit negativen `shares_change`)."
    - „⚠ Reduziert `current_shares` der betroffenen Mitglieder."
  - Bestätigungs-Button: rot/„danger"-Style, Text „Endgültig markieren".
- **D-17:** **Backend-Validation-Fehler (`PAYO-03`, z.B. `current_shares < share_count_to_pay_out`):** Toast pro Entry, ServiceError-Message als deutsche Fehlermeldung gemappt via `status_to_message`-Pattern. Liste der bereits-erfolgreichen Toggles bleibt sichtbar als Status-Updates in der Tabelle.

### Massenmail-Flow (UI-06)

- **D-18:** **Massenmail-Trigger via Redirect zur bestehenden `/mail`-Page mit Query-Param-Vorbelegung:** Klick auf „Mail an N ausgewählte" navigiert zu `/mail?from=repayment&phase_id={uuid}&members={uuid,uuid,...}`. Bestehende `/mail`-Page (`page/mail_page.rs`) hat schon Multi-Select-Recipient-UI + `mail_compose/`-Children. Phase-12-Erweiterung der `/mail`-Page: parsing der Query-Params + Pre-Selection im Recipient-Picker + Übergabe des `repayment_phase_id` an den `POST /api/mail/send-bulk`-Body.
- **D-19:** **Repayment-Var-Buttons (`{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}`) erscheinen im `template_var_buttons.rs` nur, wenn `repayment_phase_id` im Query-Param präsent ist** (oder als explizite `extra_vars: Vec<&str>`-Prop, Plan-Discretion). Bei normalem Mitglieder-Mailing bleibt die Var-Liste unverändert.
- **D-20:** **Status-Übergang `offen → angeschrieben` ist eine separate manuelle Aktion in der RepaymentEntryList:** „Als angeschrieben markieren"-Button in der Header-Action-Leiste (Multi-Select aktiv). POST `/api/repayment-entry/batch-status` mit `target_status=Contacted`. ENTR-06 wortwörtlich-manuell. Konsequenz: Vorstand-Workflow ist 2-stufig: (1) Mail senden → /mail-Redirect, (2) Zurück, Multi-Select + „Als angeschrieben markieren". Halbautomatisches Verbinden (z.B. via `?sent=true`-Banner-Vorschlag) NICHT in Phase 12 (deferred — siehe `<deferred>`).

### Add-Entry-Modal + Member-Picker (UI-04)

- **D-21:** **`MemberSearch`-Component aus `component/member_search.rs` wird unverändert reused** als Member-Picker im Add-Entry-Modal. API: `on_select(Option<Uuid>)`, `selected_id`, `exclude_id`. Datenquelle ist das bestehende `MEMBERS`-Global-Signal (lazy-loaded via `GET /api/members`).
- **D-22:** **`share_count_to_pay_out`-Feld wird beim Member-Select mit `member.current_shares` vorbefüllt** — Standard-Use-Case ist Voll-Auszahlung. Vorstand kann editieren für Teil-Abtretungen.
- **D-23:** **Client-Side-Validation im Add-Modal — minimal:**
  - `share_count_to_pay_out > 0` (matches CHECK-Constraint, Phase 8 D-11.3)
  - Member ausgewählt
  - Submit-Button disabled bei Verletzung.
  Backend-Validation (Service-Layer + DB-CHECK) bleibt Backstop. KEINE Hard-/Soft-Block-Limits gegen `current_shares` (ENTR-03 erlaubt mehrere Einträge pro Member+Phase).
- **D-24:** **Add und Edit sind zwei distinkte UI-Patterns:** Add = `repayment_entry_add_modal.rs` (Modal mit Member-Picker + Anteile-Eingabe). Edit = Inline-Cell-Edit (D-13). Trennung: Add braucht Member-Auswahl (Modal-Platz nötig); Edit ist nur Anteile (Inline reicht).

### Frontend-Routing & API-Client

- **D-25:** **Neue Routes in `genossi-frontend/src/router.rs`:**
  - `#[route("/repayment-phases")]` — Listen-Page (UI-01)
  - `#[route("/repayment-phases/:id")]` — Detail-Page (UI-02)
  Vorstand-only via bestehendem `RequirePrivilege { privilege: "admin" }`-Wrapper (Phase-4 D-05).
- **D-26:** **Neue API-Funktionen in `genossi-frontend/src/api.rs`:**
  - `list_repayment_phases()`, `get_repayment_phase(id)`, `create_repayment_phase(req)`, `update_repayment_phase(req)`, `open_repayment_phase(id)`, `close_repayment_phase(id)` — `/api/repayment-phase`
  - `list_repayment_entries(phase_id)`, `get_repayment_entry(id)`, `create_repayment_entry(req)`, `update_repayment_entry(req)`, `delete_repayment_entry(id)`, `batch_toggle_repayment_status(req)`, `mark_repayment_entry_paid_out(id)` — `/api/repayment-entry`
  - PDF-Export via `<a href="/api/repayment-phase/{id}/export/pdf?include={open|all|paid}" target="_blank">` oder programmatischer Download-Anker — keine api.rs-Funktion nötig (Browser handelt Content-Disposition).
  Alle nutzen bestehendes `AppError`/`status_to_message`-Pattern.
- **D-27:** **Top-Bar-Navigation:** Neuer Menüpunkt „Anteils-Rückzahlung" in der bestehenden Vorstand-Nav-Group (`nav_group.rs`), platziert zwischen „Anwesenheit" und „Mail" (oder Plan-Discretion). Link zu `/repayment-phases`.

### Tab-Component-Reuse

- **D-28:** **Tab-Strip via existierende `tab_strip.rs`-Component** (sofern in der Codebase vorhanden; sonst Phase-4-D-13-Anker: neuer `tab_strip.rs` aus assembly_details.rs extrahieren). Phase-12-Detail-Page ist die zweite Page mit Tab-Layout — Component-First-Anker.

### Claude's Discretion

- **`repayment_phase_status_badge.rs`:** analog `assembly_status_badge.rs`. Farben: `Vorbereitung`=grau, `Offen`=blau, `Abgeschlossen`=grün.
- **Listen-Page Default-Sort:** `fiscal_year DESC, created DESC` (Phase-7 D-08-Notiz: „Frontend (Phase 12) sortiert per `fiscal_year DESC, created DESC` zur Auffindbarkeit"). Plan-Discretion auf Sekundär-Sort.
- **Listen-Page Filter:** Vorerst keine Filter-Buttons; bei `N > 20` Phasen wäre ein Status-Filter nice — Plan-Discretion (lieber spät hinzufügen, wenn echte Schmerzgrenze sichtbar).
- **Modal-Component-Reuse:** Phase 4 D-08 hat das `Modal`-Component etabliert; alle Add-/Confirm-Modals in Phase 12 reusen `component/modal.rs`.
- **Toast-Pattern für API-Fehler:** Phase 4 D-17 etabliert; reusen für alle Phase-12-Fehler-Toasts.
- **Anzeige der Auto-Befüllung nach `Öffnen`:** Empty-State im Einträge-Tab erklärt N=0 mit „Keine Vorjahres-Austritte gefunden — manuelle Einträge sind möglich"; Plan-Discretion auf den genauen Wortlaut.
- **i18n-Keys:** Plan-Phase finalisiert die exakte Key-Liste; beide Locales (de/en) MÜSSEN gepflegt sein (Phase 4 D-19); UI-Default deutsch.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level Lock-Dokumente
- `.planning/PROJECT.md` — Core Value, Active v1.1-Requirements (Frontend Component-First UI-01..06), Constraints (Component-First-Prinzip, Datenschutz, Audit-Pflicht für bestehende Entitäten), Key Decisions (carry-over aus v1.0)
- `.planning/REQUIREMENTS.md` §"Frontend (Component-First)" — UI-01..UI-06 (jeweils Pending → Phase 12), Traceability-Tabelle
- `.planning/ROADMAP.md` §"Phase 12: Frontend (Component-First)" — Goal, 6 Success Criteria, Hard-Constraint Component-First-Grep-Gate
- `.planning/STATE.md` — Closure Snapshots Phase 7–11, Decision-Carry-over

### Phase 4 (Frontend-Vorbild aus v1.0)
- `.planning/milestones/v1.0-phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-CONTEXT.md` — Frontend-Architektur-Decisions (D-19 i18n, D-22 API-Client, D-08 Modal, D-13 Tab-Strip, D-17 Toast)
- Hotfixes aus Phase-4-Live-Betrieb mit Lektion: `e245013` (button type), `c6f41fd` (form→div), `bb1be0b` (PDF-Toggle), `8e92cfd` (live-counter), `ed754fc` (sort by member_number) — D-01/D-02

### Backend-Surface (Phasen 7–11) — Phase 12 konsumiert NUR
- `genossi_rest/src/repayment_phase.rs` — REST-Routes: `POST/GET/PUT /api/repayment-phase`, `/{id}/open`, `/{id}/close`
- `genossi_rest/src/repayment_entry.rs` — REST-Routes: `POST/GET/PUT/DELETE /api/repayment-entry`, `POST /batch-status` (D-07: target=PaidOut verboten), `POST /{id}/mark-paid-out`
- `genossi_rest/src/repayment_export.rs` — `GET /api/repayment-phase/{id}/export/pdf?include=open|all|paid`
- `genossi_rest_types/src/lib.rs` — `RepaymentEntryTO`, `RepaymentPhaseTO`, `RepaymentEntryStatusTO`, `CloseConflictResponse` (D-04), `BatchFailureResponse`
- `genossi_mail/src/rest.rs` — `POST /api/mail/send-bulk` mit `recipient_ids`, `template_id`, `repayment_phase_id` (Phase 10 D-12, D-03)
- `.planning/phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — Singular-Pfad-Konvention D-14, Audit-Disziplin
- `.planning/phases/08-repaymententry-auto-bef-llung/08-CONTEXT.md` — D-10 Listing-Query-API, ENTR-03 mehrere Entries pro Member+Phase, batch-status-Pattern
- `.planning/phases/09-auszahlungs-buchung-atomisch-auditiert/09-CONTEXT.md` — PAYO-Cascade-Atomarität (Frontend ruft Single-Endpoint pro Entry)
- `.planning/phases/10-massenmail-anbindung-template-variablen/10-CONTEXT.md` — D-12 template_id, D-03 repayment_phase_id, MAIL-02 Template-Variablen, D-09 RepaymentMail-DocumentType
- `.planning/phases/11-export-pdf-csv/11-CONTEXT.md` — D-01..03 include-Filter-Semantik, D-09 PDF-Sortierung (Mitgliedsnummer ASC), D-12 CSV deferred

### Frontend Codebase-Maps
- `.planning/codebase/ARCHITECTURE.md` §Component-First-Frontend, §Anti-Patterns (Inline RSX in Pages — kritisch für Phase 12)
- `.planning/codebase/STACK.md` — Dioxus 0.6.3, Tailwind, gloo-timers
- `.planning/codebase/CONVENTIONS.md` — snake_case-Files, Component-Service-State-Pattern

### Memory-Lock-Files (Cross-Cutting)
- `/home/neosam/.claude/projects/-home-neosam-programming-rust-projects-genossi3/memory/feedback_dioxus_button_type.md` — D-01 Grundlage (Button-Pattern)
- `/home/neosam/.claude/projects/-home-neosam-programming-rust-projects-genossi3/memory/feedback_component_first.md` — Component-First-Memory
- `/home/neosam/.claude/projects/-home-neosam-programming-rust-projects-genossi3/memory/feedback_verify_before_confirming.md` — Verify-Disziplin

### Bestehende Frontend-Patterns (zwingend wiederverwenden)
- `genossi-frontend/CLAUDE.md` §Component-First-Principle (autoritativ), §i18n (zwei Locales de/en)
- `genossi-frontend/src/api.rs` — `AppError`-Pattern, `status_to_message` (D-26)
- `genossi-frontend/src/component/member_search.rs` — Picker-Component (D-21 direkter Reuse)
- `genossi-frontend/src/component/modal.rs` — Modal-Pattern (Add-Entry, Confirm)
- `genossi-frontend/src/component/error_alert.rs` — Toast-Pattern (D-04, D-17)
- `genossi-frontend/src/component/mail_compose/` — Subject-Input, Body-Editor, Template-Selector, Template-Var-Buttons, Template-Preview (D-18/D-19 Reuse via /mail-Page)
- `genossi-frontend/src/component/assembly_status_badge.rs` — Vorlage für `repayment_phase_status_badge.rs` + `repayment_entry_status_badge.rs`
- `genossi-frontend/src/component/nav_group.rs` — Navigation-Extension (D-27)
- `genossi-frontend/src/page/assembly_details.rs` — Tab-Pattern-Vorbild (Phase 4 D-13)
- `genossi-frontend/src/page/mail_page.rs` — bestehende Mail-UI (D-18 Erweiterung)
- `genossi-frontend/src/page/member_details.rs` — Editable-Field-Pattern für `share_value`-Inline-Edit (D-05)
- `genossi-frontend/src/router.rs` — Route-Enum-Erweiterung (D-25)
- `genossi-frontend/src/auth.rs` — `RequirePrivilege { privilege: "admin" }` (D-25)
- `genossi-frontend/src/state/` — `MEMBERS`-Global-Signal (D-21 Datenquelle für Member-Picker)
- `genossi-frontend/src/i18n/{mod,de,en}.rs` — Neue i18n-Keys für Phase-12-Strings (UI-Default `Locale::De`)

### Projekt-Konventionen
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Component-First Frontend (autoritativ)
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Architecture Overview — Layered DAO/Service/REST (Backend; Phase 12 ändert daran NICHTS)
- `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/CLAUDE.md` §Component-First-Principle, §i18n

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`MemberSearch`** (`component/member_search.rs`) — Picker-Component mit `on_select`/`selected_id`/`exclude_id`-Props (D-21 direkt reuse)
- **`Modal`** (`component/modal.rs`) — Add-Entry-Modal, Confirm-Modals (D-07, D-15, D-24)
- **`error_alert.rs`** — Toast-Pattern für API-Fehler (D-04, D-17)
- **`mail_compose/`** (5 Komponenten) — Massenmail-Erweiterung via existierende /mail-Page (D-18/D-19)
- **`assembly_status_badge.rs`** — Vorlage für Status-Badges (D-14, D-26)
- **`nav_group.rs`** — Top-Bar-Erweiterung (D-27)
- **`MEMBERS`-Global-Signal** (`state/`) — Datenquelle für Client-Side-Join Member↔Entry (D-10)
- **i18n-System (de/en)** — zwei Locales pflegen (Phase 4 D-19)
- **`AppError`/`status_to_message`** (`api.rs`) — alle neuen API-Calls verwenden das (D-26)
- **`RequirePrivilege`** (`auth.rs`) — Admin-Wrapper für Phase-12-Routen (D-25)
- **`assembly_details.rs` Tab-Pattern** — Vorbild für `repayment_phase_details.rs` (D-28)
- **`member_details.rs` Editable-Field-Pattern** — Vorbild für `share_value`-Inline-Edit (D-05)

### Established Patterns
- **Component-First (autoritativ)** — `genossi-frontend/CLAUDE.md`, Memory `feedback_component_first.md`. Verletzung wäre Memory-Verletzung.
- **Button-Pattern** (D-01/D-02) — verpflichtend für jeden neuen Action-Button, Grep-Gate
- **Coroutine-Services** (`service/`) — globale State-Stores via `GlobalSignal`, fetched via Async-Coroutine
- **API-Calls** in `api.rs` als `async fn` mit `Result<T, AppError>`
- **Routing** via `dioxus-router` mit `Route`-Enum (neue Routes als zusätzliche Varianten)
- **i18n-Locale-Pflicht** — neue Keys MÜSSEN in beiden Locales (de/en) ergänzt werden
- **Tailwind-Utility-Klassen** im RSX-Inline — bei Phase-12-Components fortsetzen
- **Soft-Delete-Workflow** — `PUT` mit `deleted`-Timestamp (kein DELETE-Aufruf, ENTR-05)
- **Optimistic-Locking** — alle PUT-Calls senden `version`, bei 409 wird der lokale State neu geladen
- **Audit-Pflicht-Anker** — Phase 12 ändert NICHTS am Audit-Layer (Backend macht alles); Frontend zeigt Audit-Spur via existierender `/audit`-Page (Plan-Discretion: ggf. Verlinkung in Detail-Page nach `Abgeschlossen` — D-08 Variante 2 nicht gewählt, aber Plan kann hinzufügen)

### Integration Points
- `genossi-frontend/src/router.rs` — Route-Enum erweitern um `RepaymentPhases`, `RepaymentPhaseDetails { id: String }` (D-25)
- `genossi-frontend/src/app.rs` — Auth-Wrapper-Branch bleibt unverändert (Vorstand-Routen, kein Helper-Branch)
- `genossi-frontend/src/api.rs` — neue Funktionen (D-26)
- `genossi-frontend/src/component/mod.rs` — neue Components-Re-Exports
- `genossi-frontend/src/page/mod.rs` — neue Pages-Re-Exports (`repayment_phases.rs`, `repayment_phase_details.rs`)
- `genossi-frontend/src/component/mail_compose/template_var_buttons.rs` — Erweiterung um Repayment-Var-Buttons (D-19, bedingt durch `repayment_phase_id`-Kontext)
- `genossi-frontend/src/page/mail_page.rs` — Query-Param-Parsing für `from=repayment&phase_id=&members=` + Pre-Selection (D-18); kein neuer /repayment-spezifischer Composer
- `genossi-frontend/src/component/nav_group.rs` — Menüpunkt „Anteils-Rückzahlung" (D-27)
- `genossi-frontend/src/state/` — eventuell neuer State-Store für aktive Phase + zugehörige Einträge (Plan-Discretion: lokal-im-Page via use_resource vs Global)
- `genossi-frontend/src/i18n/{mod,de,en}.rs` — neue i18n-Keys

</code_context>

<specifics>
## Specific Ideas

- **Component-Naming (snake_case, Phase 4 D-23-Konvention):**
  - Pages: `repayment_phases.rs` (Liste), `repayment_phase_details.rs` (Detail)
  - Components (neu):
    - `repayment_phase_status_badge.rs` (Vorbereitung/Offen/Abgeschlossen)
    - `repayment_entry_status_badge.rs` (Offen/Angeschrieben/Ausbezahlt)
    - `repayment_entry_list.rs` (Kern-Component UI-03)
    - `repayment_entry_add_modal.rs` (Add UI-04)
    - `repayment_entry_paidout_confirm.rs` (Confirm-Modal UI-05)
    - `editable_cell.rs` (oder spezialisiert `editable_share_count_cell.rs` — Plan-Discretion)
    - Confirm-Modal-Generalisierung: ggf. existierendes Modal-Pattern wiederverwenden, kein eigener Component nötig
- **Detail-Page-Datenfluss:** Detail-Page lädt parallel via `use_resource`:
  - `get_repayment_phase(id)`
  - `list_repayment_entries(phase_id)` (für Einträge-Tab; Cache)
  - `MEMBERS`-Global-Signal ist bereits gefüllt
- **PDF-Export-Tab-Inhalt:** Format-Picker (verstecktes Default `pdf` weil nur ein Format) + Include-Filter-Radio (open/all/paid) + großer „Herunterladen"-Button → `<a href="..." target="_blank">`. Backend liefert Content-Disposition.
- **Banking-Workflow als Leitstern:** Vorstand öffnet die Detail-Page nach Sammel-Mail, exportiert das PDF (`?include=open`), tippt im Online-Banking Sammelüberweisung, kommt zurück zu Genossi, markiert die ausbezahlten Eintrage als `ausbezahlt` (Bulk-Loop D-15). Das ist DER Use-Case, alles andere ist sekundär.
- **`current_shares` Aktualität:** Nach `mark-paid-out`-Cascade ändert sich `Member.current_shares`. Die Frontend-`MEMBERS`-Global-Signal muss invalidiert/neu geladen werden — Plan-Discretion (entweder pro-Toggle ein partial-update oder ein Re-Fetch nach der Bulk-Loop).

</specifics>

<deferred>
## Deferred Ideas

### Halbautomatische Status-Übergänge (nach v1.2)
- **Was:** Nach Rückkehr aus `/mail?sent=true` → Vorschlag-Banner „Status der N Mitglieder auf angeschrieben setzen?" mit One-Click-Bestätigung. Bessere UX als manuelles Multi-Select + Toggle.
- **Warum deferred:** ENTR-06 sagt „manuell durch Vorstand" wortwörtlich; halbautomatischer Vorschlag wäre eine UX-Erweiterung, aber kein REQ-Pflicht. Phase 12 macht den manuellen Pfad zuerst; UAT zeigt, ob die zweistufige Reibung gefühlt-störend ist.

### `share_value`-Korrektur als separater Modal (statt inline)
- **Was:** D-05 Variante 2 (Modal-Button „share_value korrigieren" statt Inline-Edit). Defensiver, weil Auditeintrag pro Korrektur entsteht.
- **Warum deferred:** Inline-Edit (D-05 gewählt) ist konsistent mit `member_details.rs`-Pattern. Falls Vorstand-Feedback nach Phase-12-UAT zeigt, dass versehentliche Korrekturen ein Problem sind, kann Phase 13+ darauf umschwenken.

### Audit-Log-Verlinkung in der Detail-Page (nach Abgeschlossen)
- **Was:** D-08 Variante 2 — „Audit-Spur anzeigen"-Link im Stamm-Tab nach Phase-Abschluss; Tiefenklick zur `/audit`-Page.
- **Warum deferred:** Phase 12 hält die Detail-Page-UI minimal. Audit-Link ist eine Convenience; die `/audit`-Page existiert und ist über Top-Bar erreichbar.

### Listen-Page-Filter und Sort-Spalten (nach v1.2)
- **Was:** Status-Filter („Vorbereitung | Offen | Abgeschlossen") + sortierbare Tabellen-Header in der Liste der Phasen.
- **Warum deferred:** Solange Genossenschaft 1-4 Phasen pro Jahr hat, ist eine simple `fiscal_year DESC`-Sortierung ausreichend. Beim Hit auf den ersten Schmerz nachziehen.

### Bulk-`ausbezahlt` als Backend-atomar
- **Was:** Backend-Endpoint `POST /api/repayment-entry/batch-mark-paid-out` der die N Mark-Paid-Out-Cascaden atomar in einer Transaction ausführt.
- **Warum deferred:** Plan-Discretion-Diskussion zeigte, dass Frontend-Loop pragmatisch reicht (Vorstand hat selten >20 Einträge pro Phase). Atomar-Variante wäre eine eigene Backend-Phase (Service-Layer-Refactor, Cascade-Test-Suite, Race-Defense), für Phase 12 zu groß.

### Re-Open einer abgeschlossenen Phase
- Explizit nicht im v1.1-Scope (PHAS-03 ist final). Falls je gewünscht, wäre das eine v2-Diskussion mit Audit-Konsequenzen (`opened_at` revert? Neuer Audit-Eintrag?).

### CSV-Export-Tab
- EXPO-04 ist in REQUIREMENTS.md v2-deferred (Phase 11 D-12). Frontend zeigt nur PDF-Download. Re-Add ist additiv (Format-Picker-Erweiterung + Download-Anker pro Format).

### WASM-/Frontend-Test-Suite
- Phase 4 D-110-Discretion: WASM-Test-Setup nicht etabliert. Phase 12 macht Cargo-Tests für reine Logik (z.B. Betrag-Formatierung, Validation) + UAT-Checkliste analog Phase 4. Vollständige WASM-/E2E-Test-Pipeline (Playwright/wasm-bindgen-test) bleibt out-of-scope für v1.1.

### Mobile-Layout-Optimierung
- Phase 12 setzt Desktop-First; Tablet-Layout-Pass (Spalten reduzieren, Responsive-Breakpoints für 7-Spalten-Tabelle) wäre eine eigene Phase wenn das Genossi-Tool je auf Mobile getriggert wird. Vorstand-Tasks sind primär Desktop.

### Reviewed Todos (not folded)
None — keine offenen Todos in `.planning/todos/pending/`.

</deferred>

---

*Phase: 12-Frontend (Component-First)*
*Context gathered: 2026-06-01*
