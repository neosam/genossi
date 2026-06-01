# Genossi

## What This Is

Mitgliederverwaltungs-Software für Genossenschaften, produktiv im Einsatz. Ersetzt manuelle Excel-Listen durch eine REST-API mit Dioxus-WASM-Frontend, sodass Vorstände Mitgliederdaten verbandskonform pflegen, Anträge bearbeiten, Dokumente erzeugen und Audit-Spuren hinterlegen können. Mit dem v1.0-Milestone (GV-Anwesenheits-Erfassung, shipped 2026-05-29) ist papierlose Anwesenheits-Erfassung auf der Generalversammlung implementiert und auf einer echten Generalversammlung produktiv erprobt.

## Core Value

Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), und mit weniger manueller Arbeit bei wiederkehrenden Vorgängen wie Anträgen, Dokumenten und Generalversammlungen.

## Current Milestone: v1.1 Anteile-Rückzahlungsphase

**Goal:** Ersetzt die Excel-Liste für Anteils-Auszahlungen — Vorstand verwaltet Rückzahlungsphasen direkt in Genossi, schreibt Mitglieder per Massenmail an und exportiert auszahlbare Beträge als PDF zur Online-Banking-Übernahme.

**Target features:**

- `RepaymentPhase`-Entität pro Geschäftsjahr (Lifecycle angelegt → offen → abgeschlossen; `fiscal_year`, `share_value`)
- `RepaymentEntry`-Entität pro Mitglied (Status offen → angeschrieben → ausbezahlt; mehrere Einträge pro Mitglied+Phase erlaubt)
- Auto-Befüllung der Phase aus Vorjahres-Austritten (`exit_date` im `fiscal_year`)
- Manuelles Hinzufügen für Teil-Abtretungen und verspätet gemeldete Austritte
- Status-Toggle "angeschrieben" manuell durch Vorstand
- Status-Toggle "ausbezahlt" erzeugt automatisch `MemberAction::Verkauf` mit negativem `shares_change` (über bestehende Audit-Pipeline)
- Massenmail-Anbindung an bestehende Mail-Pipeline (analog Mitgliederliste-Pattern); Template kann aktuellen Auszahlungs-Wert pro Mitglied referenzieren
- PDF-Export der Auszahlungsliste **vor** Phasen-Abschluss (Online-Banking-Übernahme)
- CSV-Export für Buchhaltung
- Frontend: shared `RepaymentEntryList`-Component, Phase-Lifecycle-Page, Eintrag-Bearbeiten-Page

**Key context:**

- Datenmodell teilweise schon vorhanden: `Member.current_shares`, `Member.shares_at_joining`, `MemberAction::Verkauf` mit `shares_change` existieren — keine Member-Migration nötig
- Brief-Anschreiben bleiben manuell (out of scope für diesen Milestone)
- SEPA pain.001 XML out of scope — PDF genügt für Banking-Vorlage
- Trigger: spätestens vor GV 2027, da Anteilswerte für GJ 2026 dann berechnet werden

## Requirements

### Validated

<!-- Aus Codebase abgeleitet — bereits ausgeliefert und in Nutzung. -->

**Bestehende Plattform (vor v1.0):**

- ✓ Mitglieder-CRUD mit Soft-Delete und Versions-basiertem optimistischem Locking — existing
- ✓ Mitglieder-Aktionen (`MemberAction`) für Lebenszyklus-Ereignisse am Mitglied — existing
- ✓ Mitglieder-Dokumente (`MemberDocument`) als Anhänge / generierte Dateien — existing
- ✓ Antragsverwaltung (`Application`) für Beitritts-/Änderungsanträge — existing
- ✓ Hash-Chain-Audit-Log mit SHA256-Verkettung, Verifikations-Endpunkt `/api/audit/verify` — existing
- ✓ OIDC-Authentifizierung via Nextcloud (axum-oidc, feature-gated) — existing
- ✓ Permission/Context-System für Authorisierung auf Service-Layer — existing
- ✓ E-Mail-Pipeline: SMTP-Versand (lettre), IMAP-Polling (async-imap), Template-Generierung — existing
- ✓ PDF-Generierung via Typst-Templates — existing
- ✓ Excel-Import via calamine — existing
- ✓ Dioxus-WASM-Frontend mit Component-First-Prinzip — existing
- ✓ Nextcloud-Export für Vorstands-Zugänglichkeit (kein Backup; Backup läuft separat über Restic) — existing
- ✓ OpenAPI/Swagger-UI für API-Dokumentation — existing
- ✓ Sicherheits-Quick-Fixes: panik-freies serde_json, restriktives CORS, strukturierte Auth-Logs — existing (commit `7600e3c`)

**v1.0 GV-Anwesenheits-Erfassung (shipped 2026-05-29):**

- ✓ Vorstand kann eine GV als eigene Entität anlegen (Datum, Titel, Status Vorbereitung→Offen→Geschlossen) inkl. atomarem Member-Universe-Snapshot beim Öffnen — v1.0 (ASSY-01, ASSY-02, ASSY-03, ASSY-05, ASSY-07)
- ✓ Vorstand kann pro Helfer einen einmalig nutzbaren QR-Token mit Memo-Namen erzeugen; QR + 8–12-Zeichen-Klartext-Code via Crockford-Alphabet + SHA256-Hash — v1.0 (HLPR-01)
- ✓ Helfer kann sich per QR-Scan via BarcodeDetector + ZXing-Polyfill einloggen — v1.0 (HLPR-02 atomic redeem mit UPDATE...RETURNING)
- ✓ Helfer kann alternativ Manual-Code-Eingabe (HLPR-03) verwenden — iOS-Safari-Fallback verifiziert in produktiver GV
- ✓ Token ist One-Time-Use — Race-Test mit `tokio::join!` zeigt exakt einen Erfolg + einen Fehler — v1.0 (HLPR-04)
- ✓ Helfer-Session ist an `assembly.closed_at` gebunden — Cascade-Invalidation beim GV-Schluss invalidiert alle Sessions — v1.0 (HLPR-05 via Phase-3 SC#8)
- ✓ Vorstand sieht Token-Liste mit Memo + Status (offen/eingelöst); offene Token revokebar — v1.0 (HLPR-06)
- ✓ Token-Erzeugung im Audit-Hashchain (Memo, Erzeuger, Timestamp, GV-Bezug); `token_hash` ausgeschlossen — v1.0 (HLPR-07)
- ✓ Helfer-Mitgliederliste DSGVO-konform reduziert (Mitgliedsnummer, Name, Titel, Anrede) — strenge 7-Feld-Whitelist auf REST + DAO + PII-Guard-Test — v1.0 (ATTN-01)
- ✓ Substring-Suche auf Name oder Mitgliedsnummer (LIKE COLLATE NOCASE) — v1.0 (ATTN-02)
- ✓ Anwesenheits-Toggle idempotent — UPSERT `ON CONFLICT DO UPDATE`, fünfmaliges PUT erzeugt genau 1 Row — v1.0 (ATTN-03, ATTN-04)
- ✓ Anwesenheits-Markierungen werden bewusst NICHT in der Audit-Hashchain protokolliert — Grep-Gate `audited_*!` ist `0` — v1.0 (ATTN-05)
- ✓ Vorstand-OIDC-Zugang zu Helfer-View ohne QR — `check_assembly_access` akzeptiert beide Auth-Pfade — v1.0 (ATTN-06)
- ✓ Live-Counter „X von Y anwesend" mit explizit beschriftetem Y (Member-Universe-Snapshot); Refresh-Polling alle 5s — v1.0 (SYNC-01)
- ✓ Doppel-Markierung idempotent durch atomarem SQLite-UPSERT — v1.0 (SYNC-02, ASSY-04)
- ✓ Vorstand kann nach GV-Schluss Anwesenheits-Einträge ergänzen/entfernen ohne Status-Wechsel — v1.0 (ASSY-06)
- ✓ Teilnehmerlisten-Export in PDF (Typst) / CSV (Semikolon + UTF-8-BOM) / XLSX (rust_xlsxwriter) für geschlossene GVs; Vorstand-only, read-only, kein Audit-Eintrag — v1.0 (Phase 6, D-01..D-20)
- ✓ Helfer-Magic-Link via persistierter Code-Spalte — Helfer mit ausgedruckter Karte tippt nichts mehr (ADR-2026-05-06)

### Active

<!-- v1.1 Anteile-Rückzahlungsphase — Detaillierte REQ-IDs siehe .planning/REQUIREMENTS.md -->

- [x] `RepaymentPhase`-Entität (DAO/Service/REST/Frontend) mit Lifecycle angelegt → offen → abgeschlossen, `fiscal_year` + `share_value` — validated in Phase 12 (UI-01, UI-02)
- [x] `RepaymentEntry`-Entität mit Status offen → angeschrieben → ausbezahlt; mehrere Einträge pro Mitglied+Phase — validated in Phase 12 (UI-03, UI-04, UI-05)
- [x] Auto-Befüllung der Phase aus Vorjahres-Austritten, manuelles Hinzufügen — validated in Phase 12 (UI-03, UI-04)
- [x] "ausbezahlt"-Toggle erzeugt automatisch `MemberAction::Verkauf` mit negativem `shares_change` (audited) — validated in Phase 9 (PAYO-01..04)
- [x] Massenmail-Anbindung mit Auszahlungs-Wert als Template-Variable — validated in Phase 10 (MAIL-01..04), Frontend-Anbindung in Phase 12 (UI-06)
- [x] PDF-Export der Auszahlungsliste (vor Phasen-Abschluss verfügbar) für Online-Banking — validated in Phase 11 (EXPO-01, EXPO-02, EXPO-03, EXPO-05), Frontend-Tab in Phase 12
- [ ] CSV-Export für Buchhaltung (deferred to v2 per D-12)
- [x] Frontend: shared `RepaymentEntryList`-Component, Phase-Lifecycle-Page, Eintrag-Bearbeiten-Page — validated in Phase 12 (UI-01..06, 15 Plans, Component-First-Validation bestanden, 6 UAT-Defekte inline RESOLVED)

### Out of Scope

<!-- Bewusste Grenzen, weiterhin gültig. -->

- Stimmrechte, Vollmachten, Beschlussfähigkeits-Berechnung — eigenständiges Feature mit deutlich höherer Komplexität (Vollmachts-Daten, Stimmgewichte, Quorum-Regeln); für v1.0 reichte rein anwesend/nicht-anwesend für das Protokoll
- Audit-Hashchain-Eintrag pro Anwesenheits-Markierung — vom User explizit ausgeschlossen, weil das Anhakeln nicht verbandskonform protokolliert werden muss (Assembly-Lifecycle und QR-Token-Erzeugung BLEIBEN auditiert)
- Offline-Modus — Helfer brauchen Netzwerk; in der realen GV bestätigt
- Live-Push zwischen Helfern (SSE/WebSocket) — Refresh-Sync hat sich auf der realen GV als ausreichend bewiesen
- Re-Open einer geschlossenen GV — Lifecycle final; Korrekturen via Vorstand-Edit ohne Status-Wechsel (ASSY-06)
- Stimmgewichts-Anzeige oder Anteils-Daten in der Helfer-Ansicht — Helfer-View bleibt bewusst minimal (DSGVO)
- Identitäts-Verifikation per QR-Code für Mitglieder (Self-Check-in) — verbandsrechtlich heikel
- Native Mobile-App — Web-First, in der realen GV mit Browser auf Tablet/Handy bestätigt
- Anteils-Übertragung Genosse → Genosse (statt Rücknahme durch Genossenschaft) — bisher nicht angefragt, nur Rücknahme/Auszahlung im v1.1-Scope
- Anteils-Klassen oder einzeln-erfasste Anteile mit Nummerierung — explizit verworfen, ganze Anteile reichen
- Brief-Anschreiben für Auszahlungen — Vorstand erzeugt manuell; Massenmail-Automatik nur für E-Mail (v1.1)
- SEPA pain.001 XML / direkter Banking-Sammelüberweisung-Upload — out of scope; PDF reicht für Online-Banking-Vorlage (v1.1)
- Steuerliche Berechnung der Auszahlungen (Kapitalertragsteuer etc.) — Buchhaltung verarbeitet das separat (v1.1)
- Member-`share_count`-Migration / Excel-Import der Anteile — bereits in `Member.current_shares` vorhanden, keine Migration nötig (v1.1)

## Context

**Produktiver Stand (Stand 2026-05-29):**
- Echte Generalversammlung im Mai 2026 mit Genossi durchgeführt
- Production-Deployment via `deploy-binaries.sh` auf `shifty.nebenan-unverpackt.de`
- Hotfixes aus dem Live-Betrieb sind im Repo: `8e92cfd` (live-counter), `e245013` (button type), `ed754fc` (sort by member_number), `3cdfbb6` (token-codes magic-link), `c6f41fd` (form→div Reload-Bug), `bb1be0b` (✓→ja/nein im PDF)

**Technische Umgebung:**
- Rust 2021 Workspace mit getrennten Crates für DAO/Service/REST/Binary plus Dioxus-WASM-Frontend
- ~150k LOC Rust (workspace, ohne vendored deps)
- SQLite via SQLx, Migrations in `migrations/sqlite/`, BLOB-UUIDs, ISO8601-Timestamps
- Axum 0.8 + Utoipa-OpenAPI, Tokio 1.35
- Auth: axum-oidc gegen Nextcloud, tower-sessions, tower-cookies
- Frontend: Dioxus 0.6.3, Tailwind, Component-First (`genossi-frontend/src/component/`)
- Nix-Toolchain via `flake.nix`

**Bekannte Tech-Debt-Posten aus v1.0:**

- Phase 02 `validate_code_format` Unicode-Lookalike (`c as u8` truncation) — bekannte Spec-Divergenz, kein Security-Bug, Decision pending
- Phase 02 FK-Constraints ohne `PRAGMA foreign_keys=ON` im Production-Pool — Fix beim Pool-Setup
- Phase 04 `dx build --release` Tooling-Debt (`wasm-bindgen-cli@0.2.104`) — Production deployt erfolgreich, Release-Build lokal nicht verifizierbar
- REQUIREMENTS.md-Checkbox-Drift wurde bei v1.0-Close synchronisiert

Details siehe `.planning/milestones/v1.0-MILESTONE-AUDIT.md`.

**Bestehende Muster (für nächste Milestones aufgreifbar):**
- Entity-Struktur: `id` (UUID/BLOB), `created`, `deleted` (Option), `version` (UUID, optimistic locking)
- DAO-Pattern: Trait-Definition + SQLite-Impl, Minimal-Interface (`create`, `update`, `dump_all`, `find_by_id`)
- Service-Pattern: Trait + `*Impl`-Struct, Permission-Context-Enforcement
- Audit-Macros: `audited_create!`, `audited_update!`, `audited_delete!` für auditpflichtige Entitäten (Member/MemberAction/MemberDocument/Application/Assembly/HelperToken)
- Assembly-Member-Snapshot als Composite-PK-Tabelle ohne Audit (Pattern für stabile historische Zustände)
- Anwesenheits-Toggle ohne Audit, ohne Optimistic-Locking (Pattern für hochfrequente Operationen)
- Cascade-Invalidation: `close_assembly` → `list_session_ids_for_assembly` → pool-loop `delete_session` mit `tracing::warn!` (Pattern für Lifecycle-Cleanup)
- Component-First Frontend: Shared Components (AttendanceList/Search/LiveCounter/ConnectionBanner) zwischen Helfer- und Vorstand-Pages — ATTN-06-Pattern
- DSGVO-Whitelist mit 3-Verteidigungslinien: Struct-Whitelist + Doc-Verbot + Konversions-Pfad-Kontrolle

**Parallel-System:**
- `openspec/changes/` enthält ein eigenständiges Spec-Workflow-System; GSD wird parallel verwendet, nicht als Ersatz

## Constraints

- **Tech stack:** Rust + Axum + SQLx + SQLite Backend, Dioxus WASM Frontend — keine Sprachwechsel oder DB-Wechsel
- **Architektur:** Layered DAO/Service/REST muss eingehalten werden; neue Entitäten implementieren bestehende Trait-Patterns — Konsistenz mit gemappter Codebase, Testbarkeit
- **Frontend:** Component-First-Prinzip — keine inline-RSX-Duplikate; identische UI-Bausteine wandern in `genossi-frontend/src/component/` — gelernte Lektion, in `CLAUDE.md` und Memory festgeschrieben
- **Security:** Bearer-Tokens (Helfer-QR/Code) sind One-Time-Use, kein Identitätsnachweis; Helfer prüfen Mitglied physisch
- **Datenschutz:** Helfer-View bleibt minimal (Mitgliedsnummer, Name, Titel, Anrede); strenger DSGVO-Whitelist-Pattern
- **Audit-Pflicht:** Bestehende auditierte Entitäten (Member/MemberAction/MemberDocument/Application/Assembly/HelperToken) müssen Audit-Macros verwenden; Anwesenheit ohne Audit
- **Verbandskonformität:** Software muss als Excel-Ersatz verbandskonformes Protokoll-Material erzeugen
- **Production-Deployment:** `shifty.nebenan-unverpackt.de` via `deploy-binaries.sh`; Backup-Strategie via Restic, NextCloud nur Zugänglichkeit

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| GV als eigene Entität (`Assembly`) statt globalem Zustand | Mehrere GVs pro Jahr, Historie für Protokoll-Export, klare Lifecycle-Grenzen | ✓ Good (v1.0) — bewährt in produktiver GV |
| Anwesenheit als Join-Tabelle (`Attendance`) statt Member-Flag | Saubere Mehrfach-GV-Historie, kein Datenverlust bei nächster GV | ✓ Good (v1.0) |
| One-Time-Use-QR-Tokens pro Helfer | Verhindert Weitergabe des Zugangs an Unbefugte | ✓ Good (v1.0) |
| Helfer-Name als Freitext-Memo am Token | Vorstand sieht beim Anlegen „QR für Anna"; rein UI-Hilfe, kein Identitäts-Anker | ✓ Good (v1.0) |
| GV-Status final nach Schluss; Vorstand-Korrekturen ohne Re-Open (ASSY-06) | Vermeidet Status-Pingpong, hält Audit-Story einfach | ✓ Good (v1.0) |
| Manual-Code-Fallback (8–12 alphanumerisch Crockford) zusätzlich zu QR | iOS-Safari Camera-Quirks bekannt; Worst-Case auf echter GV vermeiden | ✓ Good (v1.0) — Helfer-Magic-Link via ADR-2026-05-06 noch besser |
| Atomarer Token-Redeem via SQL `UPDATE ... WHERE used_at IS NULL RETURNING` | Verhindert Race-Condition zwischen parallelen Scans | ✓ Good (v1.0) — Race-Test im E2E deterministisch grün |
| Member-Universe-Snapshot beim GV-Öffnen | Stabiles Y im Live-Counter, Late-Joins/Austritte verfälschen Protokoll nicht | ✓ Good (v1.0) |
| Sync zwischen Helfern nur bei Refresh, kein Live-Push | Doppel-Abhaken über Idempotenz abgefangen; keine SSE/WebSocket-Komplexität | ✓ Good (v1.0) — auf realer GV bestätigt |
| Anwesenheit ohne Audit-Hashchain | Verband fordert nur Anzahl-Anwesende im Protokoll | ✓ Good (v1.0) |
| Helfer-View auch für Vorstand zugänglich (ohne QR) | Vorstand will im Notfall mithelfen, kein UI-Duplikat | ✓ Good (v1.0) — ATTN-06 Component-Reuse-Pattern |
| Helfer-Token-Code-Persistenz (Reversal von „one-time-display") | Vorstand-UX: Re-Display ohne Revoke+Anlegen; Helfer-Magic-Link via persistierter Spalte | ✓ Good (v1.0) — ADR-2026-05-06 |
| Phase 5 SKIPPED statt Pre-GV-Generalprobe | Echte GV bereits durchgeführt; Pre-Generalprobe obsolet, Hotfixes lieferten echte Erkenntnisse zurück | ✓ Good (v1.0) |
| Drei Export-Formate parallel (PDF/CSV/XLSX) statt einer | Buchhaltung und Verband konsumieren unterschiedlich; ein einziger DAO-Call genügt für alle drei | ✓ Good (v1.0) — Phase 6 D-01 |
| Inline ExportTab statt extrahierte Component | Nur eine Seite betroffen; Component-First gilt erst ab Duplikation | ✓ Good (v1.0) — Phase 6 D-20 |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---

## ADR-2026-05-06 — Helfer-Token-Code-Persistenz (Reversal von D-11 / D-21)

**Status:** Accepted — supersedes the "one-time-display" portion of Phase 2 D-11 / D-21.

### Entscheidung
Der Klartext-Code des Helper-Tokens wird ab dieser Migration in einer neuen
Spalte `helper_token.code TEXT NULL` persistiert, sodass der Vorstand die QR-
Karte und den manuellen Code jederzeit erneut anzeigen kann (über die
bestehende admin-only Listing-Route `GET /api/assembly/{id}/helper-tokens`).

### Begründung
- **Vorstand-UX:** „One-time" zwingt den Vorstand, beim Anlegen sofort zu
  drucken. Wenn ein Helfer kurzfristig hinzukommt, fehlt eine Reprint-
  Möglichkeit; die Lösung war bisher: Token revoken + neuen anlegen — was an
  der GV stresst.
- **Single-tenant self-hosted:** Genossi läuft als Single-Tenant-Instanz pro
  Genossenschaft. Die Bedrohungslage (geleakter DB-Dump) wird ohnehin auf
  Filesystem-Ebene durch verschlüsseltes Restic-Backup abgedeckt; eine
  zusätzliche App-Layer-Verschlüsselung des Codes brächte hier keinen echten
  Vorteil.
- **Atomic Redeem unverändert:** Der race-sichere Redeem-Pfad arbeitet
  weiterhin gegen `token_hash` (`UPDATE helper_token SET used_at=?,
  session_id=? WHERE token_hash = ? AND used_at IS NULL RETURNING …`).
  `code` ist nur Read-State für die Re-Display-Route.
- **Helper-Magic-Link:** Der QR-Inhalt ist seit Phase 2 ohnehin
  `{APP_URL}/helper?code={code}`. Mit der persistierten Spalte kann der
  Frontend-`/helper`-Mount diesen Magic-Link beim ersten Aufruf automatisch
  redeemen — ein Helfer mit ausgedruckter Karte tippt nichts mehr ab.

### Grenzen die WEITER gelten
- **Audit-Log-Exklusion:** `code` wird in `HelperTokenEntity::audit_fields()`
  EXPLIZIT ausgeschlossen (mit Inline-Kommentar). Der audit_log darf
  niemals einen zweiten persistenten Code-Speicher werden.
- **`token_hash`-Redeem-Path unverändert:** Atomic redeem matched weiterhin
  auf `token_hash = SHA256(code)`. Plain-`code`-Spalte ist reine
  Re-Display-State, nie input zum Redeem.
- **Klartext nur in der DB-Row, NIE in Logs:** Die Phase-2-Regel „kein
  `tracing::debug!(code)`" bleibt; das Backend logged den Code nirgendwo.
- **Permission-Gate unverändert:** Re-Display läuft über die existierende
  admin-only `list_for_assembly`-Route. Helfer und unauthenticated User
  haben weiterhin keinen Zugriff auf den Code.

### Migration & Rollback
- Migration `20260506000000_add_code_to_helper_token.sql` fügt die Spalte
  als `NULL`-able hinzu. Vorhandene (Phase-2-)Tokens haben `code = NULL`.
- Frontend behandelt NULL-Code-Tokens distinkt: Anstelle des „QR/Code
  anzeigen"-Buttons erscheint die Hint-Zeile „Code nicht verfügbar (vor
  Update erstellt) — bitte revoken und neu erstellen". Damit ist die
  Migration für die GV ein No-Op: alte Tokens müssen einmal regeneriert
  werden, neue funktionieren ab Anlegen mit Re-Display.
- Kein Down-Migration: SQLite < 3.35 hat kein `DROP COLUMN`, und das
  Projekt führt nur Forward-Migrationen.

---

*Last updated: 2026-06-01 after Phase 12 (Frontend Component-First) complete — Vorstand verwaltet RepaymentPhases end-to-end im Browser: Listen-Page mit Anzahl-Einträge-Per-Row, 3-Tab Detail-Page (Stamm/Einträge/Export), Lifecycle-Action-Tiles, RepaymentEntryList (7-Spalten, Multi-Select, Inline-Cell-Edit, Soft-Delete, Status-Filter), AddModal mit MemberSearch + current_shares-Prefill, PaidOut-Confirm-Modal (Sequential-Loop, Per-Entry-Toast, MEMBERS-Refresh), Massenmail-Wiring (3 Var-Buttons, URL-Param-Redirect, Issue #2 fix), Export-Tab (PDF, 3 Radio-Filter). Component-First eingehalten: alle relevanten UI-Blocks als Components in genossi-frontend/src/component/. 15/15 Plans, 11 Waves, 196 Frontend-Tests + alle Backend-Tests grün, 6 UAT-Defekte inline RESOLVED (Dioxus button-reload, toast z-index, mail_page Query-Param-Race, entry-vs-member-ID-Mismatch, Phase-10 pure-member-probe blockt {{ payout_amount }}, /preview-Endpoint mit repayment_phase_id). UI-01..06 validated; Section L Auth-Gate via 12-HUMAN-UAT.md deferred. Milestone v1.1 ist damit funktional komplett.*
