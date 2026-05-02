# Project Research Summary

**Project:** Genossi — GV-Anwesenheits-Erfassung (QR-Code Helfer-Sessions)
**Domain:** Event-Anwesenheits-Tracking als Erweiterung einer bestehenden Rust/Axum/Dioxus-Plattform
**Researched:** 2026-05-02
**Confidence:** HIGH (Backend + Domainrecht); MEDIUM (Browser-QR-Scanner-Fragmentierung)

---

## Executive Summary

Das GV-Anwesenheits-Feature ersetzt eine jährliche Papier-/Excel-Anwesenheitsliste durch ein webbasiertes System mit One-Time-Use-QR-Codes für account-freie Helfer. Die rechtliche Basis (§ 47 GenG) ist klar: das System muss eine exportierbare Anwesenheitsliste mit Mitgliedszahl liefern, die in die Niederschrift eingeht und bei der Verbandsprüfung (§ 53 GenG) standhält. Der Stack ist bereits vorhanden; das Feature benötigt drei neue Aggregat-Entitäten (Assembly, HelperPreToken, Attendance), eine neue AuthContext-Variante und einen eigenen Session-Cookie neben dem bestehenden OIDC-Cookie.

Der empfohlene Bauplan folgt strikt der bestehenden Genossi-Layered-Architektur: erst alle drei Backend-Aggregate (A: Assembly-Lifecycle, B: QR-Token + Helfer-Session, C: Attendance + Cascade-Invalidation), dann das Frontend (D: Component-First). Ein kritisches Sicherheitsdetail: der QR-Token-Redeem muss atomar in einem einzigen SQL-UPDATE erfolgen, nicht als SELECT-then-UPDATE-Muster im Service-Layer. Assembly-create und Assembly-close bleiben auditiert (Verbands-Konformität); die einzelnen Anwesenheits-Markierungen hingegen nicht (explizite User-Entscheidung, da Verband nur die Endzahl fordert).

Das Hauptrisiko liegt nicht im Code, sondern im Live-Event-Betrieb: WLAN-Ausfall, iOS-Safari-Kamera-Permission-Bugs und fehlende Generalprobe können den GV-Tag empfindlich stören. Jede davon hat konkrete technische und operative Gegenmaßnahmen, die in der Planung als eigenständige Phase modelliert werden müssen. Ein Operations-Plan mit Generalprobe ist gleichrangig mit Code-Phasen.

---

## Key Findings

### Recommended Stack

Das Feature benötigt keine neuen Sprachen oder Datenbanken. Die additiven Bausteine sind minimal: `qrcode` 0.14.1 (Server-side QR-SVG-Generierung, 13M Downloads, `default-features = false, features = ["svg"]` reicht), `tower-sessions` 0.15.0 (Upgrade von 0.14, für zweite Session-Layer-Instanz mit eigenem Cookie-Namen via `SessionManagerLayer::with_name("genossi.helper")`), und der Browser-native `BarcodeDetector` mit dem `barcode-detector` npm-Polyfill (Pflicht für Safari/Firefox; deckt mit Polyfill faktisch 100 % der Helfer-Devices ab). Token-Erzeugung läuft über `rand::rngs::OsRng` (32 Byte, base64url-kodiert) — UUID v4 ist nicht ausreichend als Auth-Token. Der SHA256-Hash wird bereits via `sha2` im Workspace bedient.

**Core technologies (additiv):**
- `qrcode` 0.14.1: Server-side QR-SVG — größte Community, kein C-Binding, SVG out-of-the-box
- `tower-sessions` 0.15.0: zweite Session-Layer-Instanz neben OIDC — `with_name` per docs.rs verifiziert
- `BarcodeDetector` API + `barcode-detector`-Polyfill: Browser-Scanner für WASM — deckt Safari/Firefox
- `rand` 0.8 / `sha2` (bereits da): krypto-sicheres Token-Entropy + Hash — Standard im Workspace
- `base64` 0.22 (`URL_SAFE_NO_PAD`): kurze, scan-freundliche Token-Strings
- `subtle` 2.6 (optional): Konstantzeit-Vergleich gegen Timing-Attacks beim Token-Lookup

**Versionskompatibilität-Risiko:** `tower-sessions` 0.14 → 0.15 hat Breaking Changes im `Session::insert/get`-Pfad. Empfehlung: separater Vor-Phase-Task. Falls Risiko zu hoch, kann `with_name` auch auf 0.14 genutzt werden.

**HTTPS-Pflicht:** `getUserMedia` benötigt Secure Context. Am GV-Tag muss HTTPS stehen (Caddy + internal CA, mkcert, oder Cloudflare Tunnel). Im Operations-Plan dokumentieren und vor GV testen.

### Expected Features

**Must have (table stakes — ohne diese kein verbandskonformer Ersatz der Papierliste):**
- Assembly-Entität mit Status-Lifecycle (planned → open → closed) — Anker für alle GV-Objekte
- Member-Universe-Snapshot beim GV-Öffnen — stabilisiert Y im "X von Y"-Counter, verhindert, dass spätere Member-Updates Y nachträglich verändern
- Helper-Invite: One-Time-Use-QR mit Memo-Name (kein Identitäts-Anker, nur UX-Hilfe für Vorstand)
- Helper-Session, gebunden an genau eine Assembly, Auto-Invalidierung beim Schließen
- Reduzierte Helfer-Member-API (eigenes `AttendanceMemberTO` mit nur 4 Feldern: Mitgliedsnummer, Name, Titel, Anrede) — **nicht** `MemberTO` mit skip-Maske
- Suche + idempotenter Anwesend-Toggle (PUT statt POST/INSERT) im Helfer-UI
- Vorstand-Direkt-Zugriff ohne QR auf die Helfer-View
- Live-Counter "X von Y aktiven Mitgliedern" mit expliziter Y-Beschriftung
- Persistenz nach GV-Schluss (closed-Status friert Liste ein, kein Hard-Delete)
- CSV/JSON-Export der Anwesenheit (mindestens Zahl + Liste) — Pflicht für Niederschrift nach § 47 GenG
- Assembly-create und Assembly-close bleiben auditiert (Verbands-Konformität)
- Atomarer QR-Redeem: `UPDATE ... WHERE consumed_at IS NULL RETURNING *` — kein SELECT-then-UPDATE

**Should have (Differentiator, v1.x):**
- Anwesenheits-PDF-Anlage via Typst (nutzt bestehende Pipeline, druckbarer Protokoll-Anhang)
- Bulk-QR-Drucken (mehrere QR-Codes auf einer Seite)
- QR-Revoke-Endpoint (Vorstand kann einzelnen Token vor Verbrauch invalidieren)
- Pre-Activation-Window für QR-Tokens (nur am GV-Tag gültig, verhindert Test-Run-Verbrauch)
- Manual-Code-Eingabe als Fallback wenn Kamera-Permission fehlschlägt

**Defer (v2+):**
- Vollmachts-Erfassung — eigener Komplexitäts-Block mit Dokument-Upload und Vertretungs-Regeln
- Stimmrechte / Quorum-Berechnung — erfordert Satzungs-Modellierung
- Online-Voting — eigene Software-Domäne, ggf. Extern-Integration
- Self-Check-in durch Mitglied per persönlichem QR-Code — QR ist kein Identitäts-Beweis, nicht verbandskonform
- Re-Open einer geschlossenen GV — open question, in Requirements-Phase mit User klären

**Explizite Anti-Features (bleiben out-of-scope, keine Diskussion nötig):**
- Stimmrechte, Vollmachten, Quorum-Berechnung
- Audit-Hashchain pro Anwesenheits-Markierung (bewusste User-Entscheidung)
- Live-Push / SSE / WebSocket zwischen Helfern
- Native Mobile-App
- Identitäts-Verifikation per Mitglieds-QR-Code (Self-Check-in)
- Handschriftliche elektronische Unterschrift

### Architecture Approach

Drei neue Aggregate (Assembly, HelperPreToken, Attendance) als separate Trait+Impl-Crate-Files, exakt nach bestehendem Genossi-Pattern. Keines implementiert `Auditable` als Trait; Assembly-create und Assembly-close nutzen jedoch `audited_create!`/`audited_update!` direkt, da Lifecycle-Aktionen verbandskritisch sind. Die Helfer-Session wiederverwendet die bestehende `user_session`-Tabelle mit `claims = {"kind":"helper","assembly_id":"..."}` — kein eigenes Session-Schema. Der `AuthContext`-Enum erhält eine neue Variante `Helper { session_id, assembly_id }` neben `Oidc` und `Mock`. Die Attendance-View teilt eine einzige Page (`attendance_helper.rs`) für Helfer und Vorstand — ein UI, zwei Auth-Pfade.

**Major components:**
1. **AssemblyDao / AssemblyService** — Lifecycle create/open/close; `close_assembly` löst Cascade-Invalidation aller Helper-Sessions aus
2. **HelperPreTokenDao / HelperSessionService** — atomarer Redeem (SQL-Transaction), Token-Hash in DB, Klartext nur einmal ausgegeben; erzeugt `UserSession` mit Helper-Claims
3. **AttendanceDao / AttendanceService** — Join-Tabelle mit UNIQUE(assembly_id, member_id), idempotentes PUT, reduced-View nur via eigenem `AttendanceMemberTO`, Permission-Check via polymorpher AuthContext
4. **AuthContext::Helper-Variante** — `claims.kind == "helper"` im bestehenden Session-Extract-Pfad; bestehende OIDC-Flow unverändert
5. **Frontend: QrScanner-Component** — `BarcodeDetector` via `web-sys`, Polyfill-Loader in `index.html`, Manual-Code-Fallback als zweiter Pfad
6. **Frontend: attendance_helper.rs-Page** — gemeinsame View für Helfer + Vorstand, Auth-Differenz nur in Top-Bar

**Wichtige Boundary-Regeln:**
- Helfer-Endpoints sind ausschließlich read auf `Member` — keine Member-Mutations, kein Audit-Bypass
- `AssemblyService` kennt `HelperSessionService` (für Cascade) — Konstruktionsreihenfolge: HelperSessionService zuerst in `genossi_bin/src/lib.rs`
- `AttendanceService` kennt `AssemblyService` nur read-only (Status-Check)
- Eigenes `AttendanceMemberTO` mit 4 Feldern — niemals `MemberTO` mit skip-Attribut

### Critical Pitfalls

1. **Atomarer QR-Redeem fehlt (Race Condition)** — `UPDATE ... WHERE consumed_at IS NULL RETURNING *` in einer Transaktion; E2E-Test mit 2 parallelen Redeems muss exakt 1 Erfolg zeigen; kein SELECT-then-UPDATE im Service-Layer

2. **WLAN-Ausfall ohne Recovery-Plan (Live-Event-Risiko)** — Server-Deployment-Entscheidung VOR Phase 1 treffen (lokal vs. Cloud); Mobile-Hotspot als Backup vorbereiten; Frontend zeigt bei Verbindungsverlust expliziten Banner; gedruckte Backup-Mitgliederliste liegt am GV-Tag bereit

3. **PII-Leak an Helfer via DevTools** — eigenes `AttendanceMemberTO`-Struct (nie `MemberTO` mit skip-Maske); eigene Permission `assembly_helper` (nie `manage_members`); Test verifiziert Response-JSON-Inhalt (nicht nur UI-Sichtbarkeit)

4. **Helfer-Session stirbt bei iOS-Safari-Tab-Reload** — Persistent-Token in `localStorage` + Cookie kombinieren; `Expiry::AtDateTime(assembly.closed_at)` statt `OnSessionEnd`; Recovery-UI für Vorstand; manueller iOS-Safari-Reload-Test vor GV-Tag

5. **Camera-Permission-Fehler auf iOS Safari** — Manual-Code-Eingabe als Fallback immer anbieten; Permission-Request erst nach User-Klick; `playsinline` auf Video-Element; HTTPS ist Pflicht; Pre-GV-Test auf echtem iPhone und iPad

6. **Keine Generalprobe vor Live-Einsatz** — eigene Roadmap-Phase mit eigenem Erfolgskriterium; mindestens 1 Woche vor echter GV; echte Geräte, echter Drucker, 3 Test-Helfer; Vorstand-Schulung ohne Entwickler-Anwesenheit

7. **Audit-Pflicht für Assembly-Lifecycle wird übersehen** — `Assembly.create` und `Assembly.close` verwenden `audited_create!`/`audited_update!`; einzelne Anwesenheits-Markierungen nicht; Protokoll-Export-Endpoint liefert den Verbands-Beweis

---

## Implications for Roadmap

Suggested phase structure (5 Code-Phasen + 1 Operations-Phase):

### Phase A: CI-Hardening und Audit-Absicherung

**Rationale:** Verhindert Audit-Pipeline-Bruch für alle nachfolgenden GV-Code-Merges. Muss vor Phase B stehen, damit jeder PR gegen den CI-Test grünt.
**Delivers:** E2E-Test für `/api/audit/verify` in CI; PR-Checkliste für Audit-Macro-Verwendung; bestätigt, dass bestehende Hash-Chain-Tests intakt sind
**Addresses:** Pitfall 10 (Audit-Pipeline-Bruch durch GV-Code-Querverbindungen)
**Umfang:** Kein neuer Produkt-Code; nur Test + CI-Konfiguration

### Phase B: Assembly-Aggregat (DAO + Service + REST + Audit)

**Rationale:** Assembly ist die Wurzel-Abhängigkeit — ohne sie gibt es keinen FK-Anker für Tokens, Sessions oder Anwesenheits-Records. Assembly-create und Assembly-close müssen hier auditiert werden. Cascade-Invalidation kommt erst in Phase D (da sie HelperPreTokenDao kennt).
**Delivers:** Vorstand kann GVs anlegen, öffnen und schließen; Lifecycle-Audit-Entries für create/close; `Assembly`-Tabelle + Migration; REST-Endpoints `POST/PUT/GET /api/assembly`; DI-Wiring in `genossi_bin`
**Avoids:** Pitfall 7 (Audit-Verzicht-Lücke: Assembly-Lifecycle ist auditiert, Anwesenheits-Markierungen nicht)
**Stack:** Bestehende `audited_create!`/`audited_update!`-Macros; keine neuen Crates
**Research flag:** Standard-Pattern, kein Phase-Research nötig

### Phase C: QR-Token-Modell + Helfer-Session-Backend

**Rationale:** Baut auf Assembly (FK), nutzt bestehenden SessionService. `AuthContext::Helper`-Variante muss hier rein, weil Phase D sie benötigt. Atomarer Redeem und TTL sind Sicherheits-Pflicht dieser Phase.
**Delivers:** `helper_pre_token`-Tabelle + atomarer Redeem-Endpoint; `AuthContext::Helper { session_id, assembly_id }`; Helper-Cookie via bestehendem Session-Mechanismus; QR-SVG-Generierung server-side via `qrcode` 0.14.1; Vorstand kann QR-Codes erzeugen und sieht Memo-Name + Status; TTL + Revoke-Endpoint
**Avoids:** Pitfall 1 (Race Condition beim Redeem), Pitfall 11 (QR-Verbreitung ohne Revoke/TTL)
**Stack:** `qrcode` 0.14.1 (`default-features = false, features = ["svg"]`), `rand` 0.8 / `sha2` / `base64` 0.22; `tower-sessions` 0.15 (Upgrade-Risiko separat abwägen)
**Tests:** Concurrent-Redeem-Test (2 parallele Requests → genau 1 Erfolg); abgelaufene Assembly → 401; Helper-AuthContext-Extraction aus Session-Claims
**Research flag:** tower-sessions 0.14 → 0.15 Upgrade prüfen (Breaking Changes); kann als separater Pre-Phase-Task erfolgen

### Phase D: Attendance-Aggregat + Cascade-Invalidation

**Rationale:** Braucht Assembly (FK) und AuthContext::Helper (Phase C). Enthält den idempotenten PUT-Endpunkt und die reduzierte Member-View mit eigenem DTO. Cascade-Invalidation wird hier zu Phase B nachgezogen.
**Delivers:** `attendance`-Tabelle (UNIQUE(assembly_id, member_id) WHERE deleted IS NULL); `AttendanceMemberTO` mit 4 Feldern (DSGVO-Pflicht); eigene Permission `assembly_helper`; idempotentes `PUT /api/attendance/:aid/:mid`; `GET /api/assembly/:id/stats` für Live-Counter; Cascade-Invalidation in `close_assembly`; vollständiges Backend-API für alle 9 Active-Requirements aus PROJECT.md
**Avoids:** Pitfall 4 (PII-Leak via geteiltem MemberTO), Pitfall 6 (Doppel-Abhaken-Conflict)
**Tests:** Vorstand markiert ohne Helper-Token; Helfer nur in eigener Assembly; idempotent (5x PUT → 5x 200 OK); Member-Response-JSON enthält keine PII-Felder
**Research flag:** Standard-Pattern, kein Phase-Research nötig

### Phase E: Frontend (Component-First)

**Rationale:** Backend-first (Genossi-Konvention). Frontend konsumiert fertige API-Schemas aus `genossi_rest_types`. Erst Components, dann Pages — verhindert RSX-Duplikate.
**Delivers:** Components: `AttendanceRow`, `AttendanceSearch`, `AttendanceHeader`, `LiveCounter`, `QrCard`, `QrScanner`, `ConnectionBanner`; Pages: `assembly_list`, `assembly_detail`, `qr_redeem`, `attendance_helper` (gemeinsam); Manual-Code-Fallback; Persistent-Session-Token in `localStorage`; Connection-State-Banner; Bestätigungs-Modal für GV-Schließen
**Avoids:** Pitfall 2 (WLAN-Ausfall), Pitfall 3 (Session-Verlust nach iOS-Reload), Pitfall 5 (Counter-Y-Beschriftung), Pitfall 8 (Camera-Permission-iOS)
**Stack:** `BarcodeDetector` + `barcode-detector`-Polyfill (in `index.html` nachladen); bestehende `web-sys`/`wasm-bindgen`; `gloo-timers` 0.3 für Polling-Intervall
**Research flag:** iOS-Safari-Kamera-Test vor Merge zwingend auf echtem Gerät (nicht DevTools-Emulation)

### Phase F: Pre-GV-Generalprobe + Operations-Plan

**Rationale:** Keine Code-Phase, aber gleichrangig. Ohne Generalprobe schlagen mehrere Pitfalls am Live-Tag gleichzeitig zu. Dieser Schritt hat eigene Erfolgskriterien.
**Delivers:** `OPERATIONS.md` mit Pre-Event-Checkliste; dokumentiertes HTTPS-Setup für den Vereinsheim-Kontext; dokumentierter Backup-Plan (Mobile-Hotspot + Papierliste + Excel-Import-Pfad); Generalprobe-Durchführung: echtes Vereinsheim, echtes Tablet (iOS Safari + Android Chrome), echter Drucker, 3 Test-Helfer, 10+ Test-Mitglieder in DB; Vorstand-Schulung abgeschlossen; Helfer-Briefing schriftlich
**Avoids:** Pitfall 9 (Live-Demo ohne Generalprobe), Pitfall 2 (WLAN-Ausfall ohne vorbereiteten Plan)
**Erfolgskriterium:** Vorstand kann selbständig: Assembly anlegen, QR erzeugen, Counter lesen, GV schließen, Export herunterladen. Generalprobe-Datum: mindestens 7 Tage vor echter GV.

### Phase Ordering Rationale

- **A vor B-F:** CI-Hardening sichert die bestehende Audit-Hash-Chain ab, bevor neue Code-Pfade entstehen
- **B vor C:** HelperPreToken hat FK auf Assembly; kein Token ohne GV-Entität
- **C vor D:** AuthContext::Helper (Phase C) wird in AttendanceService Permission-Check (Phase D) benötigt
- **D vor E:** Genossi-Konvention Backend-First; Frontend nutzt REST-Types-Schemas
- **E vor F:** Generalprobe testet fertiges System; F ist Verifikation, nicht Entwicklung
- **tower-sessions-Upgrade:** Als separater Task vor Phase C; entkoppelt Upgrade-Risiko vom Feature-Code

### Research Flags

**Phases needing deeper research during planning:**
- **Phase C (tower-sessions 0.14 → 0.15 Upgrade):** Breaking Changes in `Session::insert/get`; `axum-oidc`-Kompatibilität prüfen. Option: bei 0.14 bleiben, da `with_name` dort auch verfügbar ist
- **Phase E (iOS-Safari-Scanner):** Browser-Fragmentierung ist real; WASM-Scanner + Polyfill sollte auf realem iPhone verifiziert werden bevor das Feature fertig gemeldet wird
- **Phase F (Open Question: Re-Open einer geschlossenen GV):** Requirements-Phase muss mit User klären ob der Genossenschaftsverband das erlaubt und ob Helfer-Session-Wiederherstellung machbar ist

**Standard patterns (skip research-phase):**
- **Phase A:** CI-Test-Pattern ist etabliert, Genossi-Audit-Verify-Endpoint existiert
- **Phase B:** Assembly folgt exakt dem bestehenden Entity-Pattern; `audited_create!`/`audited_update!` sind dokumentiert
- **Phase D:** DAO-Pattern, Join-Tabelle, idempotentes PUT — alles dokumentierte Genossi-Konventionen

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | crates.io-API am 2026-05-02 verifiziert; tower-sessions 0.15 per docs.rs bestätigt; `with_name` per Source verifiziert |
| Features | HIGH (Recht) / MEDIUM (Praxis) | § 47 GenG und § 53 GenG sind Originalquellen; Verbandspraxis aus Publikationen; Wettbewerber ohne direkten Test-Zugang |
| Architecture | HIGH | Basiert auf gemappter Bestands-Architektur aus CONCERNS.md und Codebase-Analyse; kein externer Unsicherheitsfaktor |
| Pitfalls | HIGH | CVE-Referenzen, WebKit-Bug-Tracker, Event-Domain-Praxisquellen; kritische Pitfalls mit mehreren Quellen verifiziert |

**Overall confidence:** HIGH

### Gaps to Address

- **Re-Open einer geschlossenen GV:** Requirements-Phase klären mit User; technisch lösbar, rechtlich unklar ob Verbands-konform; kein Blocker für v1
- **tower-sessions 0.14 vs. 0.15:** Vor Phase C entscheiden; Breaking-Change-Analyse nötig; kann als separater 1-Tag-Task modelliert werden
- **Member-Universe-Snapshot-Implementierung:** Requirements-Phase muss Speicherstrategie konkret festlegen (snapshot-Tabelle vs. computed Y zum GV-Datum)
- **QR-Format-Entscheidung:** SVG inline im JSON-Response vs. binärer Endpoint; für v1 reicht SVG-String im JSON
- **HTTPS-Setup am GV-Tag:** Im Operations-Plan konkret ausarbeiten; Entscheidung Cloud vs. Lokal beeinflusst Netzwerkrisiko-Profil

---

## Sources

### Primary (HIGH confidence)
- crates.io API — qrcode 0.14.1, tower-sessions 0.15.0, axum-oidc 0.6.0 (2026-05-02 verifiziert)
- docs.rs — tower-sessions 0.15.0, `SessionManagerLayer::with_name` + `with_expiry`
- § 47 GenG (dejure.org + gesetze-im-internet.de) — Niederschrift-Anforderungen
- § 53 GenG (dejure.org) — Pflichtprüfung
- GitHub WebKit 185448, 215884 — iOS-Safari-getUserMedia-Bugs
- GHSA-vh5j-5fhq-9xwg (GitHub) — One-Time-Token-Race-CVE
- `.planning/codebase/ARCHITECTURE.md`, `STACK.md`, `STRUCTURE.md`, `CONVENTIONS.md` — Bestands-Architektur
- `genossi_service/src/auth_types.rs`, `session.rs`, `genossi_rest/src/auth_middleware.rs` — Bestands-Code

### Secondary (MEDIUM confidence)
- Genoverband-Publikationen — Verbandspraxis GV-Niederschrift
- DGRV — Virtuelle Generalversammlungen
- MDN Barcode Detection API — Browser-Support-Matrix
- caniuse.com BarcodeDetector — 75.9 % native, Safari/Firefox via Polyfill
- Event-WiFi-Failure-Quellen (etechrentals, xpodigital) — WLAN-Ausfall-Strategien

### Tertiary (LOW-MEDIUM confidence)
- easyQuorum, SEWOBE, easyVerein, campai — Feature-Vergleich (kein direkter Test-Zugang)
- Scanbot Dioxus Tutorial — WASM-Scanner-Integrationsmuster (kommerziell, aber Muster übertragbar)

---
*Research completed: 2026-05-02*
*Ready for roadmap: yes*
