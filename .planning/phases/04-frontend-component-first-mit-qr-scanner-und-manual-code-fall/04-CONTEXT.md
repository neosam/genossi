# Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback - Context

**Gathered:** 2026-05-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Dioxus-WASM-Frontend für die GV-Anwesenheits-Erfassung. Phase 4 liefert (a) Helfer-Page (`/helper` + `/helper/attendance`) mit QR-Scanner-Login (BarcodeDetector + lazy-loaded ZXing-JS-Polyfill) und Manual-Code-Eingabe (HLPR-03), (b) Vorstand-Assembly-UI als Liste (`/assemblies`) + Detail-Page (`/assemblies/{id}`) mit Tabs (Stamm, Tokens, Anwesenheit), (c) geteilte Anwesenheits-Components zwischen Helfer-View und Vorstand-View (ATTN-06 erfüllt durch Component-Reuse, NICHT durch Route-Sharing), (d) Live-Counter mit ~5s-Polling und SYNC-01-Refresh-UX, (e) Connection-Banner bei wiederholten Polling-Fehlern, (f) 200-OK-verifiziertes Toggle-Feedback (kein Optimistic-UI). Alle Components landen in `genossi-frontend/src/component/`; identische UI zwischen Helfer- und Vorstand-Page existiert nur einmal.

**Phase 4 liefert NICHT:**
- Bulk-QR-Druck-Layout / mehrere QR pro A4-Seite (REQUIREMENTS BULK-01/02 explizit v2 — Phase 4 nutzt einzelne Browser-Prints pro QR-Card)
- PDF-Export der Anwesenheits-Liste (REQUIREMENTS EXPO-01 explizit v2)
- Backend-Änderungen (Phase 1–3 abgeschlossen; Phase 4 konsumiert nur die existierenden REST-Endpoints)
- Generalprobe / Operations-Plan / OPERATIONS.md (Phase 5)
- Live-Push (SSE/WebSocket) — explizit ausgeschlossen, Refresh-only-Sync (PROJECT.md Decision)
- Mehrsprachige Helfer-UI — Helfer-View ist deutsch only (Genossenschafts-Mitglieder im DACH-Raum)
- Erweiterte Vorstand-Rollen (z.B. Schriftführer ohne admin) — Phase 4 nutzt admin/Helper wie Phasen 1–3

</domain>

<decisions>
## Implementation Decisions

### QR-Scanner-Strategie
- **D-01:** **Primär: BarcodeDetector** (Browser-native API). Aufruf via `web_sys`/`js_sys` mit Feature-Detection `'BarcodeDetector' in window`. Unterstützt: Chrome ≥83, Edge, Android Chrome, iOS Safari ≥17. Kein zusätzlicher Bundle für die meisten User.
- **D-02:** **Polyfill: ZXing-JS** (`@zxing/library`), **lazy-loaded** nur wenn BarcodeDetector nicht verfügbar ist. Bundle-Größe ~200KB on-demand. Begründung: aktiv gepflegt (zxing-js org), Goldstandard für Browser-Barcode-Scanning, beste iOS-Quirks-Dokumentation. Loading-Mechanismus: dynamisches `<script>`-Tag oder ESM-Import beim ersten Klick auf „QR-Code scannen", wenn Native-API nicht verfügbar.
- **D-03:** **UX-Flow Helfer-Login:** Initial-View zeigt zwei gleichberechtigte Pfade nebeneinander/untereinander: (a) Button „QR-Code scannen" + (b) Manual-Code-Input mit Submit-Button. Camera-Permission wird **erst beim Klick** auf „QR-Code scannen" angefragt — konform mit iOS-Safari-User-Gesture-Anforderung, kein Surprise-Dialog. Bei Permission-Verweigerung oder Scanner-Fehler: Manual-Input bleibt sichtbar, kein Error-Wall. Manual-Code-Validation: Frontend prüft Länge=10 + Crockford-Base32-Alphabet (`0-9A-HJ-NP-Z`, kein I/O/L/U) bevor POST.
- **D-04:** Beim erfolgreichen Scan/Submit: POST an `/api/helper/redeem` mit `{code: "ABCXYZ1234"}`. Response: 200 + Set-Cookie + `{assembly_id, expires_at}`. Frontend speichert `assembly_id` im State und `Navigate::to("/helper/attendance")`. Bei Fehler: Error-Code-Mapping zu deutscher Message (404→„Code nicht erkannt", 410→„Code wurde bereits verwendet", 403→„GV nicht offen oder Code widerrufen", 400→„Ungültiges Code-Format").

### Routing & Layout-Trennung
- **D-05:** **Getrennte Routen, geteilte Components** für Helfer vs Vorstand:
  - Helfer-Login: `/helper` — Layout OHNE TopBar (eigenes minimales Layout), eigener `HelperShell`-Component-Wrapper.
  - Helfer-View: `/helper/attendance` — gleiches HelperShell-Layout, rendert die geteilten `AttendanceList` + `AttendanceSearch` + `LiveCounter` Components.
  - Vorstand-Liste: `/assemblies` — Standard-App-Layout MIT TopBar, RequirePrivilege „admin".
  - Vorstand-Detail: `/assemblies/{id}` — Standard-Layout MIT TopBar, RequirePrivilege „admin", Tab-Aufbau (Stamm-Daten, Helfer-Tokens, Anwesenheit). Anwesenheits-Tab rendert dieselben Components wie `/helper/attendance` (ATTN-06 durch Component-Reuse).
  - Vorstand-Anwesenheits-Direktlink: optional `/assemblies/{id}/attendance` als Deep-Link auf den Anwesenheits-Tab (Plan entscheidet ob nötig).
- **D-06:** Helfer-Page-State-Machine: `/helper` und `/helper/attendance` sind **zwei getrennte Routen** (User-Wahl, nicht Single-Page-State-Machine). Browser-Back-Edge-Case wird durch Auto-Redirect gehandhabt: beim Mount von `/helper` prüft die Page mittels API-Call (z.B. `GET /api/auth/whoami` oder ein dedizierter `/api/helper/session` mit 401 wenn keine Session) ob bereits eine gültige Helfer-Session existiert; wenn ja → Navigate zu `/helper/attendance`. Vermeidet, dass eingeloggter Helfer beim Browser-Back erneut die Login-UI sieht.
- **D-07:** **HelperShell-Layout** (neuer Layout-Wrapper-Component): minimal, mobile-first, ohne Top-Bar/Footer-Branding. Zeigt nur einen schmalen Header mit GV-Name (nach Redeem) + LogOut-Button. Hintergrund: Helfer arbeiten auf Tablet/Handy, Top-Bar mit Vorstand-Menü wäre verwirrend und würde Datenschutz-Probleme schaffen (Helfer dürfen keinen Zugriff auf Member-Liste/Audit-Log/Mail/etc. haben).

### Vorstand-Assembly-UI-Struktur
- **D-08:** **Pages-Struktur:**
  - `/assemblies` (neue Page): Liste aller GVs mit Status-Badges (Vorbereitung/Offen/Geschlossen), Anlegen-Button (Modal) → `POST /api/assembly`.
  - `/assemblies/{id}` (neue Page): Detail mit drei Tabs:
    1. **Stamm-Daten** — Name, Datum, Ort editieren (nur in Status `Preparation`); Buttons „GV öffnen" (Status `Preparation` → `Open` mit Snapshot-Erzeugung), „GV schließen" (Status `Open` → `Closed`).
    2. **Helfer-Tokens** — Liste aller Tokens (Memo-Name + Status `Open`/`Used`/`Revoked`), „Token erzeugen"-Button (Modal mit Memo-Input) → `POST /api/assembly/{id}/helper-tokens`. Jeder neu erzeugte Token wird als **Card mit QR-SVG + Klartext-Code + Memo-Name** dargestellt (das Backend liefert SVG nur einmalig im Create-Response — Frontend muss SVG sofort speichern/anzeigen, da nicht erneut abrufbar). Revoke-Button pro offenem Token. Drucken: pro Card individueller Browser-Print-Button (CSS `@media print` versteckt App-Chrome, druckt nur die Card).
    3. **Anwesenheit** — wenn Status `Open` oder `Closed`: rendert dieselben `AttendanceList` + `AttendanceSearch` + `LiveCounter` Components wie Helfer-View. Vor `Open`: zeigt Hinweis „GV noch nicht eröffnet".
- **D-09:** **QR-Druck-Pfad: kein dedizierter Print-View, nur einzelne Browser-Prints per Card.** Begründung: Bulk-QR-Druck ist v2 (BULK-01/02); Genossenschaften haben typisch 2–5 Helfer pro GV; einzelnes Drucken ist akzeptabel. Bei mehr Helfern später (Phase 5+) kann Bulk-Print nachgezogen werden. Phase 4 baut nur die einfachste funktionale Variante.
- **D-10:** Token-Liste zeigt für **bereits eingelöste Token** den Memo-Name + Status `Used` + eingelöst-am-Timestamp. Klartext-Code und QR-SVG sind nicht mehr verfügbar (Backend speichert beides nie persistent — Phase 2 D-11). Wenn Vorstand einen Token verloren hat: muss neuen erzeugen (alter bleibt als „eingelöst" sichtbar oder wird revoked falls noch unbenutzt).

### Geteilte Anwesenheits-Components
- **D-11:** Drei neue geteilte Components in `genossi-frontend/src/component/`:
  - `attendance_list.rs` — `AttendanceList` Props: `assembly_id: Uuid`, `read_only: bool` (false für aktive Toggles, true wenn GV `Closed` und Helfer-View — Vorstand sieht trotzdem editierbar). Rendert Liste der Mitglieder mit reduzierten Feldern (member_number, last_name, first_name, salutation, title) + Toggle-Button. Reused von Helfer-View und Vorstand-Tab.
  - `attendance_search.rs` — `AttendanceSearch` Props: `value: String`, `on_change: EventHandler<String>`. Debounced Substring-Search (500ms Debounce-Default; Plan finalisiert exakten Wert). Pattern-Vorlage: bestehende `member_search.rs`.
  - `live_counter.rs` — `LiveCounter` Props: `assembly_id: Uuid`, `polling_enabled: bool`. Holt `/api/assembly/{id}/stats` alle ~5s, zeigt „X von Y anwesend" mit expliziter Y-Beschriftung (nicht nur „X/Y" — ROADMAP-Hard-Constraint Phase 4 SC#3). Zeigt Loading-State („— von Y") während Polling-Failure.
- **D-12:** Zusätzliche Components für Helfer-Login + Token-Verwaltung:
  - `qr_scanner.rs` — `QrScanner` Props: `on_scan: EventHandler<String>`, `on_error: EventHandler<String>`. Kapselt BarcodeDetector + ZXing-JS-Fallback hinter einer einheitlichen API. Verwaltet Camera-Stream-Lifecycle (start/stop), Permission-State.
  - `manual_code_input.rs` — `ManualCodeInput` Props: `on_submit: EventHandler<String>`. 10-Zeichen-Input mit Crockford-Base32-Validation, Auto-Uppercase-on-Type, Submit-Button.
  - `qr_card.rs` — `QrCard` Props: `memo: String`, `code: String`, `qr_svg: String`. Print-fähige Card mit `@media print`-Styling.
  - `helper_shell.rs` — `HelperShell` Props: `children: Element`. Layout-Wrapper für Helfer-Routes (D-07).
- **D-13:** Components für Vorstand-Assembly-UI:
  - `assembly_list_row.rs` — Listen-Eintrag mit Status-Badge.
  - `assembly_status_badge.rs` — wiederverwendbare Status-Anzeige (`Preparation`/`Open`/`Closed` → deutsche Labels via i18n-System; siehe D-19).
  - Tab-Aufbau in `assembly_details.rs`-Page nutzt bestehendes `CollapsibleSection` oder ein neues `tab_strip.rs` (Plan entscheidet).

### Polling-Architektur
- **D-14:** `LiveCounter` nutzt `use_resource` mit `gloo_timers::future::TimeoutFuture` für ~5s-Intervall (Pattern existiert nicht in der Codebase, aber `gloo-timers` ist bereits Dep). Polling startet bei Component-Mount, stoppt bei Unmount. Re-Fetch bei Polling-Tick. **Kein globaler Polling-Service** — der Counter polled lokal solange er sichtbar ist.
- **D-15:** **Refresh-Trigger für AttendanceList:** Refresh nach jedem Toggle-Klick (auf 200-OK), nach jedem Such-Vorgang (Debounce-getriggert), und alle ~5s parallel zum Counter (gleicher Tick — Plan entscheidet ob ein gemeinsamer Polling-Hook für Counter+Liste oder zwei separate). SYNC-01 erfüllt: Helfer sieht aktualisierte Markierungen anderer Helfer beim nächsten Refresh oder Such-Vorgang.

### Connection-Banner & 200-OK-Feedback
- **D-16:** **Connection-Banner-Trigger:** Banner erscheint, wenn der Live-Counter zwei Polls in Folge fehlschlägt (network-error oder 5xx). Banner verschwindet, sobald ein Poll wieder Erfolg liefert. Toleriert kurze 4G-Wackler ohne Alarm, zeigt aber zuverlässig echte Verbindungsverluste nach ~10s. Pattern-Vorlage: bestehender `status_bar.rs`. Kann später (Phase 5) auf einen dedizierten `online_indicator.rs` (Status-Dot) umgestellt werden, wenn Banner zu aufdringlich wirkt — Plan/Discretion.
- **D-17:** **Toggle-Feedback-Pattern:** Klick auf Anwesend-Toggle setzt den Button **sofort** in einen Loading-State (Spinner-Icon, `disabled=true`); KEIN visuelles Anwesend-Häkchen. Erst nach 200-OK wird der Toggle in den neuen State geflippt (Häkchen erscheint). Bei 4xx/5xx: Toast-Notification mit Error-Message (deutsch via bestehendem `status_to_message`-Pattern in `api.rs:53`); Button kehrt in den Vor-Klick-State zurück. Vermeidet Phantom-Häkchen wenn der Request scheitert (ROADMAP Phase 4 SC#6).
- **D-18:** **Doppel-Klick-Schutz:** Während ein Toggle-Request läuft (`disabled=true`), sind alle Folge-Klicks auf demselben Button ignoriert. Nach Antwort (Erfolg oder Fehler): Button reaktiviert sich. Wenn der User in dieser Zeit bewusst mehrfach klickt, hat das keinen Effekt. Backend-Idempotenz (ATTN-03/04, SYNC-02) macht Race trotzdem sicher.

### i18n & Sprache
- **D-19:** **Bestehendes i18n-System nutzen** (`genossi-frontend/src/i18n/`). Neue Keys für Phase 4: GV-Status-Labels (`Preparation` → „Vorbereitung", `Open` → „Offen", `Closed` → „Geschlossen"), Helfer-Login-Strings, Counter-Beschriftung („X von Y anwesend"), Error-Messages für Redeem-Fehler. Alle drei Locales (de, en, cs) müssen die neuen Keys haben — bestehende Konvention (`genossi-frontend/CLAUDE.md` §i18n). **Helfer-View standardmäßig deutsch** (Genossenschafts-Mitglieder DACH); andere Locales bleiben verfügbar für Vorstand-UI, sind aber nicht auto-detected — Plan finalisiert ob Helfer-Page locale-Switch hat oder fix de.

### Frontend-Build & Dependencies
- **D-20:** Neue NPM-/Cargo-Deps für Phase 4:
  - **Cargo (`genossi-frontend/Cargo.toml`):** `web-sys` Features-Erweiterung um `BarcodeDetector`, `MediaDevices`, `MediaStream`, `MediaStreamTrack`, `MediaStreamConstraints`, `HtmlVideoElement`, `Navigator` (falls noch nicht aktiv). KEIN neuer Rust-Crate für QR — alles über web-sys + JS-Polyfill.
  - **JS-Polyfill ZXing-JS:** wird als Static Asset (z.B. `assets/zxing.js`) ausgeliefert oder via CDN-`<script>`-Tag dynamisch nachgeladen — Plan entscheidet (CDN ist einfacher, lokal ist offline-tauglich; für Phase 5 Generalprobe mit Mobile-Hotspot ggf. lokal besser).
  - **Tailwind:** keine neuen Klassen-Tools nötig, Standard-Utility-Setup reicht für Helfer-UI.
- **D-21:** **Print-Styling:** `@media print` CSS in `input.css` ergänzen — versteckt TopBar/Footer/Sidebar, druckt nur die zu druckende QrCard formatfüllend (z.B. zentriert auf A4-Seite). Plan entscheidet exakte CSS-Definitionen.

### REST-API-Konsumption
- **D-22:** Neue API-Client-Funktionen in `genossi-frontend/src/api.rs`:
  - `redeem_helper_token(code: &str) -> Result<RedeemResponseTO, AppError>` — POST `/api/helper/redeem`.
  - `list_assemblies() -> Result<Vec<AssemblyTO>, AppError>` — GET `/api/assembly`.
  - `get_assembly(id: Uuid) -> Result<AssemblyTO, AppError>` — GET `/api/assembly/{id}`.
  - `create_assembly(req: AssemblyCreateTO) -> Result<AssemblyTO, AppError>` — POST `/api/assembly`.
  - `update_assembly(id, req)` / `open_assembly(id)` / `close_assembly(id)` — analog.
  - `list_helper_tokens(assembly_id)` / `create_helper_token(assembly_id, memo)` / `revoke_helper_token(assembly_id, token_id)`.
  - `list_attendance_members(assembly_id, search)` / `mark_present(aid, mid)` / `mark_absent(aid, mid)` / `get_assembly_stats(assembly_id)`.
  - Alle nutzen bestehendes `AppError`/`status_to_message`-Pattern (`api.rs:53`).

### Naming
- **D-23:** Neue Frontend-Files englisch, snake_case (Genossi-Konvention): `helper_shell.rs`, `qr_scanner.rs`, `manual_code_input.rs`, `qr_card.rs`, `attendance_list.rs`, `attendance_search.rs`, `live_counter.rs`, `assembly_list_row.rs`, `assembly_status_badge.rs`. Pages: `helper_login.rs` (für `/helper`), `helper_attendance.rs` (für `/helper/attendance`), `assemblies.rs` (für `/assemblies`), `assembly_details.rs` (für `/assemblies/{id}`).

### Claude's Discretion
- **Connection-Banner-Defaults** (D-16): „Banner bei 2 fehlgeschlagenen Polls in Folge" wurde von Claude als sinnvoller Default gewählt (User hat Fragen-Set zu diesem Punkt nicht im Detail beantwortet). Plan kann auf Status-Dot oder andere Variante umstellen — die UX-Anforderung „Verbindungsverlust klar sichtbar" (ROADMAP SC#6) ist mit beiden Varianten erfüllbar.
- **Toggle-Feedback-Pattern** (D-17/D-18): Loading-Spinner + 200-OK-Verifizierung wurde von Claude als pragmatischer Default gewählt. Plan kann zusätzlich subtle visual feedback (z.B. zarte Hintergrund-Animation während Loading) ergänzen.
- **Polling-Hook-Sharing** (D-15): ein gemeinsamer Hook für Counter+Liste vs zwei separate. Performance-Argument für gemeinsamen Hook (ein Tick, zwei Endpoints parallel via `futures::join!`); Architektur-Argument für separate (Component-Isolation, einzeln testbar). Plan/Researcher entscheidet.
- **Debounce-Wert für AttendanceSearch** (D-11): 500ms Default; Plan kann mit echter Test-Last validieren.
- **JS-Polyfill-Bezug** (D-20): CDN vs lokal. Plan/Phase-5-Operations entscheidet — lokal ist offline-tauglich für Vereinsheim-WiFi-Probleme.
- **i18n-Helfer-Page-Locale-Switch** (D-19): fix de oder mit User-Locale-Detection. Plan entscheidet — fix de ist defensiv, Locale-Detection wäre konsistenter mit bestehender App.
- **Tab-Implementation** (D-13): bestehende `CollapsibleSection` reusen vs neuer `tab_strip.rs`. Plan/Researcher schaut existing Pattern in `member_details.rs`.
- **Helfer-Auto-Redirect-Endpoint** (D-06): Welcher Endpoint signalisiert „gültige Helfer-Session vorhanden"? Vermutlich ein neuer `GET /api/helper/session`-Endpoint oder ein bestehender wie `/api/auth/whoami` mit Helper-Context-Branch. Plan finalisiert Backend-Vertrag oder ergänzt mit minimalem Additional-Endpoint (Phase 4 darf Backend nicht groß ändern, aber ein READ-only-helper-Endpoint ist akzeptable Erweiterung).
- **Print-CSS-Layout-Details** (D-21): exakte `@media print`-Regeln für QR-Card-Zentrierung, Page-Break-Verhalten. Plan finalisiert.
- **Test-Strategie für Phase 4:** WASM-Tests sind in der Codebase nicht etabliert (Genossi hat keine `wasm-bindgen-test`-Setup). Plan entscheidet: (a) reine Cargo-Tests für reine Logik (Validation, Error-Mapping), (b) manuelle E2E auf Generalprobe (Phase 5), oder (c) Playwright/Cypress-Setup neu (out-of-scope-Risiko). Phase-5-Generalprobe ist die finale Verifikation — Phase 4 darf sich darauf verlassen.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level Locking-Dokumente
- `.planning/PROJECT.md` — Core Value, Active Requirements (Helfer-View reduziert, Live-Counter, Component-First, QR + Manual-Code), Constraints (Component-First-Prinzip, Datenschutz Helfer sieht nur 4 Felder, iOS-Safari-Quirks bekannt), Key Decisions (One-Time-Use-QR, Manual-Code-Fallback, Sync nur per Refresh, Helfer-View für Vorstand zugänglich).
- `.planning/REQUIREMENTS.md` §Anwesenheits-Erfassung (ATTN-06 Helfer-View für Vorstand), §Helfer-Token (HLPR-03 Manual-Code-UI in Phase 4), §Sync (SYNC-01 Refresh-Update).
- `.planning/ROADMAP.md` §Phase 4 — Goal, 6 Success Criteria, Hard Constraints (Component-First, kein Optimistic-UI, Connection-Banner, Counter mit „X von Y" expliziter Y-Beschriftung).
- `.planning/STATE.md` §Accumulated Context — Skills/Conventions to Apply (Component-First, Layered Architecture, ISO8601-Datetime).
- `.planning/phases/01-assembly-aggregat-audit-hardening/01-CONTEXT.md` — Assembly-Status-Werte englisch (D-06/D-17), Frontend i18n-Mapping (de Labels in Phase 4), Lifecycle-Übergänge linear (D-07).
- `.planning/phases/02-helfer-token-session-authcontext-helper/02-CONTEXT.md` — Token-Erzeugung-Response-Vertrag (D-21: `{token, code, qr_svg}` einmalig), QR-URL-Format `${APP_URL}/helper?code=ABC1234567` (D-12), Klartext-Code-Format 10 Zeichen Crockford Base32 (D-09), Redeem-HTTP-Status-Codes (D-24: 404/410/403/400/200), Endpoint `/api/helper/redeem` öffentlich ohne Auth (D-22).
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-CONTEXT.md` — `AttendanceMemberTO`-Schema mit 7-Feld-Whitelist (D-24), Endpoints `/api/attendance/{aid}/members?q=`, `PUT /api/attendance/{aid}/{mid}`, `DELETE /api/attendance/{aid}/{mid}`, `GET /api/assembly/{aid}/stats` (D-21), HTTP-Status-Codes für Toggle (D-26), Substring-Search im DAO (D-25), Cascade-Invalidation in close_assembly (D-11..D-15).

### Codebase-Maps (Bestands-Architektur)
- `.planning/codebase/ARCHITECTURE.md` §Component-First-Frontend, §Anti-Patterns (Inline RSX in Pages — Phase-4-relevant).
- `.planning/codebase/STACK.md` — Dioxus 0.6.3, web-sys, wasm-bindgen-Versionen.
- `.planning/codebase/CONVENTIONS.md` — snake_case-Files, Component-Service-State-Pattern.

### Bestehende Frontend-Patterns als Vorlage
- `genossi-frontend/CLAUDE.md` §Component-First-Principle (autoritativ); §i18n-System (Locale-Konvention); §Backend-Configuration (proxy auf localhost:3000).
- `genossi-frontend/src/api.rs:53` — `status_to_message` für deutsche Error-Messages (D-04, D-22).
- `genossi-frontend/src/api.rs:14-50` — `AppError`-Pattern für REST-Calls.
- `genossi-frontend/src/component/member_search.rs` — Pattern-Vorlage für `attendance_search.rs` (Debounced Substring-Search).
- `genossi-frontend/src/component/top_bar.rs` — wird **NICHT** in Helfer-Page verwendet (D-07); für Vorstand-Pages (Phase 4 D-08) reused.
- `genossi-frontend/src/component/status_bar.rs` — Pattern-Vorlage für Connection-Banner (D-16).
- `genossi-frontend/src/component/error_alert.rs` — Toast-Notification-Pattern für Toggle-Fehler (D-17).
- `genossi-frontend/src/component/modal.rs` — Modal für Token-Erzeugen, Assembly-Anlegen (D-08).
- `genossi-frontend/src/component/collapsible_section.rs` — Möglicher Tab-Pattern-Vorlage (D-13).
- `genossi-frontend/src/component/pagination_controls.rs` — falls Mitgliederliste in Phase 5 als Stress-Issue auftaucht; Phase 4 nutzt sie zunächst NICHT (REQUIREMENTS-Hard-Constraint: keine Pagination, Substring-Search reicht).
- `genossi-frontend/src/auth.rs:25-50` — `RequirePrivilege` für Vorstand-Pages („admin"-Privilege, D-05).
- `genossi-frontend/src/router.rs` — Route-Enum, neue Routes hier hinzufügen (D-05, D-23).
- `genossi-frontend/src/app.rs:36-54` — App-Layout mit `Auth`-Wrapper; Helfer-Routes brauchen separate Branch (kein TopBar/Footer für `/helper*`-Routes — D-07).
- `genossi-frontend/src/state/auth_info.rs` — `AuthInfo` mit Privileges; Helfer-Auth-Context muss erkennbar sein (Backend liefert `is_helper`/`assembly_id`).
- `genossi-frontend/src/i18n/mod.rs` + `de.rs`/`en.rs`/`cs.rs` — neue Keys für Phase 4 (D-19); alle drei Locales pflegen.
- `genossi-frontend/src/page/member_details.rs` — Pattern-Vorlage für Tabs (falls verwendet) und Detail-Page-Aufbau (D-08).
- `genossi-frontend/src/page/members.rs` — Pattern-Vorlage für Liste-Page mit Modal (Anlegen).
- `genossi-frontend/src/service/auth.rs` — Auth-Service-Pattern (Coroutine-Service-Pattern).
- `genossi-frontend/Dioxus.toml` — Backend-Proxy-Konfiguration für `localhost:3000`.

### CLAUDE.md (Projekt-Konventionen)
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Component-First Frontend (autoritativ — Memory-Eintrag verweist hier zurück).
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Architecture Overview — Layered DAO/Service/REST (Backend, Phase 4 ändert daran nichts).
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Datenschutz — Helfer sehen nur Mitgliedsnummer, Name, Titel, Anrede (Phase 4 muss diese Konvention auf UI-Ebene einhalten — kein „extra Info"-Tooltip o.ä.).

### Web-APIs / External Docs (Phase-4-Researcher liest)
- **BarcodeDetector** — MDN: https://developer.mozilla.org/en-US/docs/Web/API/Barcode_Detection_API
- **ZXing-JS** — Repo: https://github.com/zxing-js/library, npm: `@zxing/library`
- **Dioxus** — https://dioxuslabs.com/learn/0.6/ (Router, use_resource, EventHandler-Patterns)
- **web-sys MediaDevices** — docs.rs/web-sys für getUserMedia, MediaStreamConstraints, etc.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`AppError`-Pattern** (`api.rs:14-50`) mit `status_to_message` (deutsche Messages) — alle neuen API-Calls in D-22 nutzen das.
- **`MemberSearch`-Component** (`component/member_search.rs`) — direkte Vorlage für `attendance_search.rs` (Debounced Substring-Search, ähnliche UX).
- **`Modal`-Component** — für Token-Erzeugen + Assembly-Anlegen (D-08).
- **`status_bar.rs`** — Pattern-Vorlage für Connection-Banner-Stil (D-16).
- **`error_alert.rs`** — Pattern-Vorlage für Toast-Errors (D-17).
- **`gloo-timers` 0.3** (Cargo.toml) — `TimeoutFuture` für Polling-Tick und Search-Debounce (D-14).
- **`web-sys` + `wasm-bindgen`** (Cargo.toml) — bereits eingebunden; Phase 4 erweitert nur die `web-sys`-Features um Camera-/MediaDevices-/BarcodeDetector-spezifische Items (D-20).
- **i18n-System** mit `Locale::De` (`src/i18n/`) — alle UI-Strings landen dort, deutsche UI-Labels (D-19).
- **`Auth`/`RequirePrivilege`** (`auth.rs`) — Vorstand-Routen wrappen mit `RequirePrivilege { privilege: "admin" }`.
- **`use_coroutine`-Service-Pattern** (`service/auth.rs`) — für Helper-Session-Service falls nötig.
- **`Dioxus.toml` Proxy** — `/api/*` proxied auf `localhost:3000`; keine Konfiguration nötig.

### Established Patterns
- **Component-First** (autoritativ, `genossi-frontend/CLAUDE.md`) — alles wandert in `src/component/`, Pages komponieren nur.
- **Coroutine-Services** (`service/`) — globale State-Stores via `GlobalSignal`, fetched via Async-Coroutine.
- **API-Calls** sind alle `async fn` in `api.rs`, geben `Result<T, AppError>` zurück.
- **Routing** via `dioxus-router` mit `Route`-Enum (`router.rs`); neue Routes als zusätzliche Varianten.
- **i18n-Locale-Pflicht** — neue Keys MÜSSEN in allen drei Locales (de/en/cs) ergänzt werden, sonst broken UI.
- **Tailwind-Utility-Klassen** im RSX-Inline (kein BEM, kein CSS-Modules) — bei Phase-4-Components fortsetzen.

### Integration Points
- `genossi-frontend/src/router.rs` — `Route`-Enum erweitern um `Helper`, `HelperAttendance`, `Assemblies`, `AssemblyDetails { id: String }`. Auth-Branching im `app.rs` so anpassen, dass `/helper*` Routes NICHT durch `Auth`-Wrapper gehen (Helfer hat keine OIDC-Auth, sondern Cookie-Session).
- `genossi-frontend/src/app.rs:36-54` — Branch-Logik: wenn aktuelle Route mit `/helper` beginnt → `HelperShell`-Layout (kein TopBar/Footer); sonst Standard-Layout. Plan entscheidet ob router-side Sub-Layout via Outlet oder per-Page-Decision.
- `genossi-frontend/src/api.rs` — neue API-Funktionen (D-22). Reqwest-Client mit Cookie-Support — Cookie wird vom Browser auto-mitgeschickt (existing `app_session`-Cookie).
- `genossi-frontend/src/state/` — neuer State-Store für Helfer-Session-Info (assembly_id + expires_at) und ggf. Assembly-State-Cache.
- `genossi-frontend/src/component/mod.rs` — neue Components-Re-Exports.
- `genossi-frontend/src/page/mod.rs` — neue Pages-Re-Exports.
- `genossi-frontend/Cargo.toml` — `web-sys` Features erweitern (D-20); ggf. ZXing-JS als CDN-Script-Tag in `index.html` oder lazy-loaded JS-Modul.
- `genossi-frontend/input.css` — `@media print`-Regeln für QrCard-Druck (D-21).

</code_context>

<specifics>
## Specific Ideas

- **`HelperShell`-Layout-Pattern:** App-Layout in `app.rs` prüft beim Render `if route.starts_with("/helper")` → `HelperShell { children: Router::<Route> {} }` ohne TopBar/Footer; sonst Standard-Layout mit TopBar/Footer/Auth-Wrapper. Helfer-Pages bekommen einen schmalen Header mit GV-Name (nach Redeem) und LogOut-Button (löscht Cookie via `/api/auth/logout` oder dedizierten `/api/helper/logout`-Endpoint).
- **`QrScanner`-Component-Internals:** Ruft `getUserMedia({video: {facingMode: 'environment'}})` für Rückkamera; Stream wird in `<video>`-Element gepiped; bei jedem `requestVideoFrameCallback` wird das aktuelle Frame an BarcodeDetector (oder ZXing-JS-Polyfill) gegeben; bei Match: `on_scan(code)` und Stream stoppen. Permission-Verweigerung: `on_error("Kamera-Zugriff verweigert. Bitte Code manuell eingeben.")`.
- **Code-Format-Frontend-Validation:** Vor POST auf `/api/helper/redeem`: prüfen `len() == 10 && chars.all(|c| Crockford-Base32-Alphabet.contains(c))`. Falls invalid: kein Round-Trip zum Backend, sondern direkt deutsche Error-Message anzeigen. Backend-400 ist Backstop für unerwartete Fälle (z.B. URL-Parameter-Tampering).
- **Counter-„X von Y"-Beschriftung exakt** (ROADMAP-Hard-Constraint Phase 4 SC#3): Component-Output nicht „X/Y" oder „X anwesend", sondern exakt deutsch „X von Y anwesend". Bei Polling-Fehler: „— von Y anwesend" (Y bleibt wenn schon mal geladen, X wird Dash).
- **i18n-Helfer-Page-Strategie:** Helfer-View ist standardmäßig deutsch (assembly hat sowieso deutsches Datum). Bei späterer Mehrsprachigkeit (Genossenschaft mit nicht-deutschen Mitgliedern): Locale-Switch in HelperShell-Header. Phase 4 baut die Strings als i18n-Keys, aber das Frontend-Default ist Locale::De.
- **Polling-Stop-bei-Closed-Assembly:** Wenn `LiveCounter`/`AttendanceList` bemerkt dass Assembly-Status `Closed` ist (via Stats-Response oder via separatem Assembly-Fetch), Polling-Intervall erhöhen oder stoppen — kein Sinn alle 5s zu pollen wenn Daten eingefroren sind. Plan entscheidet (Polling kann auch einfach weiterlaufen — minimaler Server-Last).
- **Token-Card-Print-CSS-Sequenz:** Klick auf „Drucken" in QrCard → `window.print()`; CSS `@media print` versteckt `body > .app` und zeigt nur die druckende Card; nach Print-Dialog: zurück zum Normal-View. Pattern in vielen Apps etabliert.
- **Vorstand-Tab-Visibility-Logic:** Token-Tab ist immer sichtbar; Anwesenheits-Tab nur wenn Status `Open` oder `Closed`. Stamm-Tab ist immer sichtbar, aber Edit-Felder nur in `Preparation` enabled.

</specifics>

<deferred>
## Deferred Ideas

### Phase 5 (Generalprobe & Operations)
- **Realer iOS-Safari-Test:** ob BarcodeDetector + ZXing-JS-Polyfill auf echtem iPhone/iPad zuverlässig funktioniert (DevTools-Emulation reicht nicht). Phase 5 SC#3 explizit.
- **Connection-Banner-UX-Validation unter realer Vereinsheim-WiFi-Last:** ob 2-Polls-in-Folge das richtige Threshold ist oder ob 3 oder eine Time-Window-Variante besser passt.
- **Print-Layout-Polishing:** echtes Drucken auf Vorstand-Drucker vs CSS-Preview; ggf. zusätzliche Print-CSS-Tweaks.
- **Bulk-Print-Layout:** wenn Vorstand >5 Helfer hat, ist Einzeldruck mühsam — Phase 5 evaluiert ob Bulk-Print nachgezogen werden muss.
- **Stats-Polling-Last:** ob ~5s-Polling beim Vereinsheim-WiFi mit 5 parallelen Helfern okay ist; ggf. Rate-Limit oder Polling-Intervall-Anpassung.

### Spätere Phasen / Out of Scope (REQUIREMENTS §v2 + §Out of Scope)
- **Bulk-QR-Druck-Layout** (BULK-01/02 v2): Mehrere QR pro A4-Seite. Phase 4 baut nur Einzel-Print.
- **Mehrsprachige Helfer-UI** mit Auto-Detect: deferred bis tatsächlich gefordert.
- **Native-Mobile-App** (Out of Scope): Web-First bleibt.
- **PDF-Export der Anwesenheits-Liste** (EXPO-01 v2): Backend-Typst-Pipeline, Phase 4 nicht relevant.
- **CSV/Excel-Export** (EXPO-02 v2): Backend-Job, Phase 4 nicht relevant.
- **Vollmacht-/Stimmrechts-UI** (VOTE-01..04 v2): komplett separater Workflow.
- **Self-Check-in für Mitglieder per persönlichem QR-Code** (Out of Scope): verbandsrechtlich heikel.
- **WASM-Test-Setup** (`wasm-bindgen-test` oder Playwright): nicht in Genossi etabliert; Phase 4 verlässt sich auf Phase-5-Generalprobe und Cargo-Tests für reine Logik.

### Reviewed Todos (not folded)
None — keine TODOs für Phase 4 in `.planning/todos/`.

</deferred>

---

*Phase: 4-Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback*
*Context gathered: 2026-05-04*
