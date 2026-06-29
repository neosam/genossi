# Genossi

## What This Is

Mitgliederverwaltungs-Software für Genossenschaften, produktiv im Einsatz. Ersetzt manuelle Excel-Listen durch eine REST-API mit Dioxus-WASM-Frontend, sodass Vorstände Mitgliederdaten verbandskonform pflegen, Anträge bearbeiten, Dokumente erzeugen und Audit-Spuren hinterlegen können. Mit v1.0 (GV-Anwesenheits-Erfassung, shipped 2026-05-29) ist papierlose Anwesenheits-Erfassung auf der Generalversammlung produktiv erprobt; mit v1.1 (Anteile-Rückzahlungsphase, shipped 2026-06-02) ersetzt Genossi die manuelle Excel-Liste für Auszahlungen — vom RepaymentPhase-Lifecycle über atomare Auszahlungs-Buchung bis zu Massenmail- und Bulk-PDF-Brief-Versand. Mit v1.2 (Mitgliedschaft-Anpassungen während des Geschäftsjahres, shipped 2026-06-07) deckt Genossi den vollen Mitgliedschafts-Lifecycle ab — Vorstand triggert Kündigung, Teil-Rückgabe, Übertrag und Aufstockung direkt am Mitglied (Single-Button auf Member-Detail), während v1.1's PaidOut-Cascade weiterhin die `current_shares`-Reduktion und `Verkauf`-Action übernimmt (kein Doppelbuchen). Mit v1.3 (Posteingang-Benachrichtigung & Reply-Komfort, shipped 2026-06-28) verpassen Vorstände keine eingehenden Mails mehr — Inbox-Anhänge sind sichtbar/herunterladbar, ein täglicher Digest-Worker mailt die offenen Posteingangs-Mails mit Deep-Link, und das Antworten läuft im vollflächigen Modal mit Entwurfs-Schutz.

## Core Value

Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), und mit weniger manueller Arbeit bei wiederkehrenden Vorgängen wie Anträgen, Dokumenten, Generalversammlungen und Anteils-Auszahlungen.

## Current State

**Shipped Milestones:**

- ✅ **v1.0 GV-Anwesenheits-Erfassung** (2026-05-29) — Helfer-QR-Tokens, Anwesenheits-Erfassung, Teilnehmerlisten-Export PDF/CSV/XLSX
- ✅ **v1.1 Anteile-Rückzahlungsphase** (2026-06-02) — RepaymentPhase-Lifecycle, atomare Auszahlungs-Buchung, Massenmail mit Auszahlungs-Variablen, PDF-Export für Banking, Bulk-Briefe für Nicht-Email-Mitglieder
- ✅ **v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres** (2026-06-07) — `MembershipAdjustModal` als shared Component (1078 LOC, 4 Sub-Views Kündigung/Teil-Rückgabe/Übertrag/Aufstockung mit Live-Preview-Confirmation), `compute_effective_date` Pure-Function für H1/H2-Stichtag, atomare Single-Tx-Cascades für alle 4 Operationen, 5 neue REST-Endpoints unter `/api/members/{id}/{cancel|increase-shares|partial-repayment|transfer-shares}` + `/api/members/transfer-recipients`. Vorstand-UAT signed-off; Audit-Status `tech_debt` (alle 31 REQs satisfied, dokumentierte Carry-forward-Posten zu CR-02 Permission-Ordering + Phase-18-UX-Polish).
- ✅ **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** (2026-06-28) — Inbox-Anhänge persistieren/anzeigen/herunterladen (Phase 19), täglicher Inbox-Digest-Worker (`genossi_mail/src/digest.rs`: konfigurierbare Empfänger + Uhrzeit, Ein-Versand-pro-Kalendertag, Deep-Link auf `/inbox`, kein Versand bei leerem Posteingang/ohne Empfänger), und Reply im vollflächigen `Modal` (X-Header + «Abbrechen» + Dirty-Check-Confirm, Body-Editor unverändert `h-40`). Audit `passed` (11/11 REQs, Integration 8/8 sauber, E2E-Flow Digest→Inbox→Anhänge→Reply, Live-Browser-Smoke-Test bestanden). Phase-21-Code-Review fand+fixte 1 Critical (Footer-Load überschrieb getippten Reply-Body → Datenverlust).

## Current Milestone: v1.4 Mail-Formatierung & Antrags-Dokumente

**Goal:** Vorstände versenden professionell formatierte HTML-Mails (statt nur Rohtext) und können den originalen Mitgliedsantrag als Datei am Antrag hinterlegen, die beim Aktivieren automatisch ans Mitglied übergeht.

**Target features:**
- **8bit-Kodierung** — Mail-Text als 8bit statt quoted-printable; keine `=`-Soft-Breaks mehr (geteilter Helfer in `genossi_mail`, betrifft `worker.rs` + `service.rs`)
- **HTML-Mail-Backend** — `multipart/alternative` (Text + HTML) in `genossi_mail`; Plain-Text-Fallback bleibt erhalten
- **WYSIWYG-Editor (Frontend)** — Rich-Text-Editor (fett/kursiv/Links/Listen) als Dioxus-Component, damit Vorstände ohne HTML-Kenntnisse formatieren
- **Original-Antrag als Attachment** — Datei-Upload an `Application` (Filesystem via `DocumentStorage`), automatische Übernahme als `MemberDocument` beim Aktivieren des Antrags (Audit-Macros, da `Application`/`MemberDocument` auditiert)

Definiert via `/gsd-new-milestone` am 2026-06-29. Research → Requirements → Roadmap folgen.

**Offene Backlog-Kandidaten** (siehe `.planning/ROADMAP.md` Backlog 999.x + `milestones/v1.2-MILESTONE-AUDIT.md`):
- 999.1 mock_auth-Deploy-Footgun absichern · 999.2 MailRecipientsTable-Komponente extrahieren · 999.3 Service-Layer für audit_log-/backup-Handler · 999.4 Daten-Lade-Boilerplate in Hook bündeln · 999.5 In-App-Hilfe für Vorstände
- Carry-forward v1.2-Tech-Debt: CR-02 Permission-Check-Ordering (`gen_auth_admin!`-Helper), Phase-18 UX-Polish, Mail-Subsystem-Triage, 16 deferred v1.1-Quick-Tasks
- Aus v1.3: Phase-21 IN-02 (trivial — `cached_quote`-Signal-Redundanz)

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

**v1.1 Anteile-Rückzahlungsphase (shipped 2026-06-02):**

- ✓ `RepaymentPhase`-Entität (DAO/Service/REST/Frontend) mit Lifecycle Vorbereitung → Offen → Abgeschlossen, `fiscal_year` + `share_value` (i64 Cent), auditpflichtig — v1.1 (PHAS-01..05, Phase 7+12)
- ✓ `RepaymentEntry`-Entität mit Status offen → angeschrieben → ausbezahlt; mehrere Einträge pro Mitglied+Phase erlaubt (ENTR-03 ohne Composite-PK-Constraint) — v1.1 (ENTR-01..06, Phase 8+12)
- ✓ Auto-Befüllung beim Phase-Öffnen atomar in Status-Übergangs-Tx, manuelles Hinzufügen mit MemberSearch + current_shares-Prefill — v1.1 (ENTR-01, ENTR-02, Phase 8+12)
- ✓ `ausbezahlt`-Toggle erzeugt atomar `MemberAction::Verkauf` + reduziert `Member.current_shares` in einer SQLite-Tx mit gemeinsamem Process-String; final-Semantik (PAYO-04 kein Rücksetzen) — v1.1 (PAYO-01..04, Phase 9)
- ✓ Massenmail-Anbindung mit `{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}` Template-Variablen über bestehenden `POST /api/mail/send-bulk`-Endpoint; pro Empfänger ein auditiertes `MemberDocument` — v1.1 (MAIL-01..04, Phase 10+12)
- ✓ PDF-Export Auszahlungsliste (6-Spalten mit Verwendungszweck, Filter `?include=open|all|paid`, Filename-Schema `auszahlung-{fy}-{include}.pdf`) für offene UND geschlossene Phasen — v1.1 (EXPO-01..03, EXPO-05, Phase 11+12)
- ✓ Frontend Component-First: `RepaymentEntryList`, `RepaymentPhaseStatusBadge`, `RepaymentEntryStatusBadge`, `format_payout_eur` Helper, 3-Tab-Detail-Page, PaidOut-Confirm-Modal — v1.1 (UI-01..05, Phase 12)
- ⚠ **UI-06 partial:** Massenmail-Aktion im Tabellen-Header — Code-Pfade grep-verifiziert, Service-Layer-403 unit-getestet, 3 HUMAN-UAT-Items pending Non-Admin-OIDC-Session (siehe Known Gaps)
- ✓ **(additiv) Bulk-PDF-Anschreiben für Nicht-Email-Mitglieder** — Multi-select auf RepaymentPhase-Detail → `POST /api/repayment-phase/{id}/letters/generate` → pro Member ein auditiertes `MemberDocument` + Bundle-PDF mit `#pagebreak()` als Direct-Download — v1.1 Phase 13 (BRIEF-01)

**v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres (shipped 2026-06-07):**

- ✓ Kündigung (Voll-Rückgabe) erzeugt `MemberAction::Austritt` mit H1/H2-Stichtag via `compute_effective_date`-Pure-Function; `Member.exit_date` via `recalc_dates`-Hook; KEIN `Verkauf`/`RepaymentEntry` direkt — v1.2 Phase 14+15 (CANC-01..06)
- ✓ Teil-Rückgabe erzeugt `RepaymentEntry` in Ziel-Phase (H1/H2-GJ) mit Auto-Anlegen-Phase (Variante B: Status=Open, `DEFAULT_SHARE_VALUE_CENT=10000`-Fallback), Sum-Check, Auto-Fill-Skip-Pattern, Closed-Phase-Status-Guard (HTTP 409) — v1.2 Phase 16 (PART-01..06)
- ✓ Anteils-Übertragung erzeugt 2 verlinkte MemberActions (`UebertragungAbgabe`/`UebertragungEmpfang`) + aktualisiert `current_shares` atomar in 15-Schritt-Single-Tx-Cascade; Voll-Übertrag triggert zusätzlich `MemberAction::Austritt` mit `transfer_date` als `effective_date`; alle 5 audited_*!-Calls teilen `TRANSFER_PROCESS` — v1.2 Phase 14+17 (TRSF-01..07, AUDT-02, PERM-03)
- ✓ Aufstockung erzeugt `MemberAction::Aufstockung` + aktualisiert `current_shares` atomar via `audited_create!` + `audited_update!`; Block für gekündigte Mitglieder via HTTP 400 — v1.2 Phase 15 (UPGD-01..04)
- ✓ Single-Button „Mitgliedschaft anpassen" + 4-Sub-View-Modal + Live-Preview-Confirm-Section auf Member-Detail-Frontend (Vorstand-only via `RequirePrivilege "admin"`); `FiscalYearDateInput` mit GJ-Bounds; `MembershipAdjustModal` als shared Component (1078 LOC); Vorstand-UAT signed-off — v1.2 Phase 18 (UI-01..04)

### Active

<!-- v1.3 noch nicht definiert. Wird mit `/gsd-new-milestone` initialisiert (questioning → research → requirements → roadmap). -->

v1.3 Posteingang-Benachrichtigung & Reply-Komfort (siehe `.planning/REQUIREMENTS.md` für REQ-IDs):

- [ ] Täglicher Inbox-Digest an konfigurierbare Empfänger zu konfigurierbarer Uhrzeit, nur bei nicht-leerem Posteingang, mit Mail-Zusammenfassung (Titel/Absender/Zeitpunkt) + Deep-Link auf `/inbox`
- [ ] Empfänger-Adressen und Versand-Uhrzeit über das bestehende Config-System (Config-Seite) pflegbar
- [ ] Antworten auf Mails öffnet in einem vollflächigen Modal statt im schmalen Inline-Feld

### Out of Scope

<!-- Bewusste Grenzen, weiterhin gültig. -->

- Rückwirkende Mitgliedschafts-Anpassung in bereits abgeschlossene Geschäftsjahre — sehr individuell, Vorstand regelt das mit der bestehenden manuellen MemberAction-UI (v1.2)
- Übertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft — koppelt Application + Member + Anteile + Aktion atomar; zu komplex für jetzt, abhängiger Seed `transfer-to-applicant` bleibt unaktiviert (v1.2)
- Storno-Knopf für ausgelöste Kündigungen — über bestehende manuelle MemberAction-UI als negative Gegenbuchung (v1.2)
- Zwei-Stufen-Workflow (Antrag → Genehmigung → Wirksamkeit) — One-Click mit Vorschau-Confirm ist Default; Vier-Augen-Prinzip ist Future-Requirement (v1.2)
- v1.2 darf NICHT MemberAction::Verkauf erzeugen und NICHT current_shares reduzieren — das macht v1.1's PaidOut-Cascade; v1.2 erzeugt nur Intent-Datensätze (kein Doppelbuchen) (v1.2)
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

**Produktiver Stand (Stand 2026-06-07):**
- Echte Generalversammlung im Mai 2026 mit Genossi durchgeführt (v1.0 produktiv erprobt)
- v1.1 shipped 2026-06-02 — Anteils-Rückzahlungs-Pipeline live; erste produktive RepaymentPhase noch nicht durchgeführt (kommt in der Q1/Q2-2026-GJ-Abrechnung)
- v1.2 shipped 2026-06-07 — Mitgliedschaft-Anpassungen während des Geschäftsjahres (4 Operationen Vorstand-getriggert, Audit-pflichtig, Vorstand-UAT signed-off); produktiver Einsatz steht noch bevor (kommt mit der nächsten realen Mitgliedschafts-Anpassung)
- Production-Deployment via `deploy-binaries.sh` auf `shifty.nebenan-unverpackt.de`; Release-Tags `v1.0.0`, `v1.0.1`, `v1.1.0`, `v1.2.0`, `v1.2.1` durch `/release-version`-Skript erzeugt
- Hotfixes aus v1.0-Live-Betrieb: `8e92cfd` (live-counter), `e245013` (button type), `ed754fc` (sort by member_number), `3cdfbb6` (token-codes magic-link), `c6f41fd` (form→div Reload-Bug), `bb1be0b` (✓→ja/nein im PDF)
- Hotfix-Pattern für v1.1: Dioxus-Button-Reload-Bug (`r#type: "button"` statt form-onsubmit) konsequent in allen RepaymentPhase-Pages befolgt; v1.2 setzt das Pattern fort

**Technische Umgebung:**
- Rust 2021 Workspace mit getrennten Crates für DAO/Service/REST/Binary plus Dioxus-WASM-Frontend
- ~338k LOC Rust (workspace, ohne vendored deps); v1.1 fügte ~10k LOC für RepaymentPhase/Entry/Letter hinzu, v1.2 ~3-4k LOC für MembershipAdjustService + Modal (24 Plans, 127 commits in 4 Tagen)
- SQLite via SQLx, Migrations in `migrations/sqlite/`, BLOB-UUIDs, ISO8601-Timestamps
- Axum 0.8 + Utoipa-OpenAPI, Tokio 1.35
- Auth: axum-oidc gegen Nextcloud, tower-sessions, tower-cookies
- Frontend: Dioxus 0.6.3, Tailwind, Component-First (`genossi-frontend/src/component/`)
- Mail: lettre (SMTP), async-imap, minijinja-strict mit `{% if X is defined %}`-Pattern
- PDF: Typst 0.14 + typst-pdf, fresh-install Default-Template-Provisioning
- Nix-Toolchain via `flake.nix`

**Bekannte Tech-Debt-Posten aus v1.2 (siehe `.planning/milestones/v1.2-MILESTONE-AUDIT.md`):**

- Phase 16+v1.1 projektweit — **CR-02 (Permission-Check-Ordering) BLOCKER carry-forward:** `current_user_id()` läuft VOR `check_permission()` in allen 4 v1.2-MembershipAdjustService-Methoden UND in allen 5 v1.1-RepaymentPhaseService-Methoden. Side-Channel-Risiko + `"SYSTEM"`-Audit-Fallback bei `Ok(None)`. Explicitly out-of-scope für v1.2; extrahierbar in `gen_auth_admin!`-Helper für v1.3.
- Phase 16 — WR-01..05: Inkonsistente Check-Reihenfolge (Date-Validation NACH Member-Load → SELECT-Spam), PII-Leak in 409-Conflict-Message (`exit_date={:?}`), `unwrap()` auf `Response::builder()`, pre-Read Member ohne re-read post-commit, `REPAYMENT_PHASE_CREATE_PROCESS`-String-Duplikat zu v1.1 (forensisch nicht unterscheidbar)
- Phase 18 — **CR-01 + CR-02 UX-Critical:** `date_signal` überlebt Sub-Choice-Wechsel im MembershipAdjustModal (User kann altes Datum unbemerkt übertragen; Submit-`is_valid` blockt aber); `Signal::set` im Render-Pfad von `render_sub_choice` (Re-Render-Loop-Risiko + User-Datenverlust beim Zurück-Navigieren). Side-Effects gehören in onclick.
- Phase 18 — Smell: Wire-Asymmetrie `PartialRepaymentResponseTO.entry/.phase` als `serde_json::Value` im Frontend vs. typisiert im Backend. Wire-kompatibel, Modal verwirft Body.
- Phase 18 — Warnings: Alle Submit-Buttons `bg-red-600` (visuell destruktiv für Aufstockung); 4 Operations-spezifische Success-Keys + 2 AutoCreate-Hint-Keys unused (generische statt kontextspezifische Toasts); `format_date_input`/`parse_date_input` doppelt in `fiscal_year_date_input.rs` und `member_details.rs`.

**Bekannte Tech-Debt-Posten aus v1.1:**

- Phase 7 — Optimistic-Locking Stale-Retry-Pattern: DAO bumpt DB-Version, propagiert sie nicht zurück (codebase-weite Service-Konvention)
- Phase 8 — `format_dt`-Helper lokal in `repayment_entry.rs` dupliziert (Refactor in `crate::dt_helpers` wäre Rule-4-Change)
- Phase 9 — SQLITE_BUSY Race-Path im E2E-Test akzeptiert sortierte Statuses `[200, 409|500]` statt strict `[200, 409]`
- Phase 10 — `DocumentType::is_singleton()` TODO: Idempotency-Storage-Growth bei Re-Generierung (3 deferred Strategien in WR-06)
- Phase 11 — `from_env()` defaults zu relativen Pfaden — unsafe unter parallelen Cargo-Tests (IN-04)
- Phase 12 — 3 Auth-Gate-UAT-Items pending (Helper-OIDC-Session lokal nicht verfügbar)
- Phase 13 — Bundle-Template Side-Effect via `#import`; `std::mem::forget(tempdir)` leakt `/tmp`-Dirs in Tests (IN-01)
- 16 deferred v1.1-Quick-Tasks ohne SUMMARY (Mail/Template/RepaymentLetter-Themen) — siehe STATE.md Deferred Items, beim v1.2-close acknowledged

**Bekannte Tech-Debt-Posten aus v1.0:**

- Phase 02 `validate_code_format` Unicode-Lookalike (`c as u8` truncation) — bekannte Spec-Divergenz, kein Security-Bug, Decision pending
- Phase 02 FK-Constraints ohne `PRAGMA foreign_keys=ON` im Production-Pool — Fix beim Pool-Setup
- Phase 04 `dx build --release` Tooling-Debt (`wasm-bindgen-cli@0.2.104`) — Production deployt erfolgreich, Release-Build lokal nicht verifizierbar

Details siehe `.planning/milestones/v1.0-MILESTONE-AUDIT.md`, `v1.1-MILESTONE-AUDIT.md` und `v1.2-MILESTONE-AUDIT.md`.

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
| i64-Cent für `share_value` statt Decimal/Float | SQLite INTEGER 8-Byte → Rust i64; Floats produzieren Rundungsfehler bei Cent-Multiplikation; Validierung `> 0` ohne Obergrenze | ✓ Good (v1.1) — Phase 7 D-01, durchgezogen bis Phase 11 PDF-Export |
| KEIN UNIQUE-Constraint auf (member_id, phase_id) im RepaymentEntry | Mehrere Einträge erlauben Teil-Abtretungen, Korrekturen, mehrstufige Auszahlungen; PAYO-Sum-Check via `Member.current_shares` reicht | ✓ Good (v1.1) — Phase 8 ENTR-03 |
| Atomare Cascade `ausbezahlt`-Toggle in einer SQLite-Tx | Verhindert Inkonsistenz zwischen `MemberAction::Verkauf` und `Member.current_shares`-Reduzierung; gemeinsamer Process-String macht Audit-Story einheitlich | ✓ Good (v1.1) — Phase 9 PAYO-01..04 |
| PAYO-04 final — kein `ausbezahlt → angeschrieben`-Rücksetzen | Verhindert Audit-Verzerrung und inkonsistente `current_shares`; Rückbuchung wäre zweite manuelle MemberAction | ✓ Good (v1.1) — Phase 9 PAYO-04 |
| minijinja-strict + `{% if X is defined %}`-Pattern statt `..base`-Spread | Workspace-minijinja 2.19 unterstützt Spread nicht; serde_json round-trip + BTreeMap-Merge ist 1:1-Mirror der Plan-10-Worker-Aggregation | ✓ Good (v1.1) — Phase 10 |
| Pdf-only Export (kein CSV/XLSX) | Buchhaltung kann Online-Banking-Vorlage direkt aus PDF kopieren; CSV-Export für Buchhaltung in v2 verschoben | ✓ Good (v1.1) — Phase 11 D-12 |
| Bulk-Brief-Service deduplicated Aggregation via `RepaymentContextResolver::aggregate` | Eliminiert N+1-DB-Read; Phase-10-Worker behält Inline-Aggregation per D-13-10 (nicht refactored, todo `phase-10-worker-refactor-resolver.md` low-prio) | ✓ Good (v1.1) — Phase 13 D-13-04 |
| Direct-Download Bundle-PDF + persisted MemberDocuments parallel | Vorstand bekommt sofort druckbares Bundle, Audit-Trail behält pro-Member-Dokumente; Selection-Preservation (D-13-09) erlaubt direkt "Als angeschrieben markieren" anschließend | ✓ Good (v1.1) — Phase 13 |
| `compute_effective_date` als Pure-Function in `membership_adjust.rs` statt im DateTime-Service | Testbar ohne Clock-Mocking, 2-Branch-Berechnung benötigt kein Trait-Boilerplate | ✓ Good (v1.2) — Phase 14, 6 edge-case tests grün, Phase 15+16 wiederverwenden |
| `MembershipAdjustService` als single trait mit 4 Methoden statt 4 separate Services | Eine Dependency-Liste, gemeinsamer Audit-Pattern, gemeinsame ADMIN_PRIVILEGE-Funnel | ✓ Good (v1.2) — Phase 15→16→17 incremental extension, kein DI-Explosion |
| `recalc_dates` zu `pub(crate)` Free-Function refactor | Cross-Service-Reuse (cancel + transfer_shares Voll-Übertrag) ohne Trait-Bound-Hell | ✓ Good (v1.2) — Phase 15 |
| v1.2 erzeugt NUR Intent-Datensätze; v1.1 PaidOut-Cascade bleibt Single-Source-of-Truth für `current_shares`-Reduktion + `MemberAction::Verkauf` | Verhindert Doppelbuchung; klare Verantwortlichkeits-Trennung zwischen "Intent" und "Geldfluss" | ✓ Good (v1.2) — Auto-Fill-Skip-Pattern fängt Edge-Case (v1.2-Entry vor /open) |
| Auto-Anlegen-Phase Variante B (Status=Open, share_value aus Vorgänger oder DEFAULT=10000) | A=Preparation hätte zusätzlichen /open-Schritt verlangt; B liefert direkt nutzbare Phase | ✓ Good (v1.2) — Phase 16, funktional korrekt aber forensisch nicht von v1.1-Phase-Create unterscheidbar (WR-05) |
| Closed-Phase-Status-Guard via Plan 16-05 Gap-Closure | CR-01 in 16-REVIEW fand fehlenden Guard; HTTP 409 vor jedem audited_create | ✓ Good (v1.2) — Re-Verifikation flipped von gaps_found auf passed |
| `transfer_shares` als 15-Schritt-Single-Tx-Cascade in einer Service-Methode mit shared `tx.clone()` | Atomarität, gemeinsamer Process-String für 5 audited_*!-Calls, Audit-Pair-Verlinkung verifizierbar | ✓ Good (v1.2) — Phase 17, Race-Tests grün, AUDT-02 satisfied |
| Voll-Übertrag-Detection pre-write statt via recalc_dates-Trigger | Klare Sequenz: detect → audited_create Austritt → audited_update → recalc_dates exakt einmal nach Cascade (D-17-02) | ✓ Good (v1.2) — Phase 17 |
| `MembershipAdjustModal` als single-file 1078-LOC-Component mit ModalStep-Enum statt 4 separate Modals | Sub-Choice-UX braucht gemeinsame State-Machine; 4 separate Modals wären Code-Duplikat | ⚠ Revisit (v1.2) — Vorstand-UAT bestätigt UX, aber 2 CR-Findings (date_signal-leak, Signal::set in Render) sind UX-Polish |
| `MemberSlimTO` mit 6-Feld-PII-Guard für `/transfer-recipients` statt MemberTO-Re-Use | DSGVO: Vorstand-Such-Endpoint exposed nur identifizierende Felder, keine IBAN/Email/Adresse | ✓ Good (v1.2) — Phase 14, Pattern aus v1.0 `feedback_component_first.md` fortgesetzt |
| Sub-Route-Ordering Pitfall: alle 5 v1.2-Sub-Routes BEFORE `/{id}` catch-all | Axum match by declaration order; ohne Ordering würde `/transfer-recipients` als UUID geparst → 400 | ✓ Good (v1.2) — Phase 14, explizite E2E-Asserts |

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

*Last updated: 2026-06-29 — Milestone v1.4 (Mail-Formatierung & Antrags-Dokumente) gestartet via `/gsd-new-milestone`: 8bit-Mail-Kodierung (`=`-Soft-Breaks weg), HTML-Mail-Backend (`multipart/alternative`), WYSIWYG-Rich-Text-Editor im Frontend, Original-Antrag als Datei-Attachment an Application mit Auto-Übernahme ans Mitglied. Research → Requirements → Roadmap folgen. Vorheriger Stand: 2026-06-28 nach v1.3-Milestone (Posteingang-Benachrichtigung & Reply-Komfort, 3 Phasen/11 Pläne, Audit `passed`, 11/11 REQs satisfied).*
