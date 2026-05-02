# Genossi

## What This Is

Mitgliederverwaltungs-Software für Genossenschaften. Ersetzt manuelle Excel-Listen durch eine REST-API mit Dioxus-WASM-Frontend, sodass Vorstände Mitgliederdaten verbandskonform pflegen, Anträge bearbeiten, Dokumente erzeugen und Audit-Spuren hinterlegen können. Ist heute produktiv im Einsatz; der nächste Meilenstein bringt papierlose Anwesenheits-Erfassung auf der Generalversammlung.

## Core Value

Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), und mit weniger manueller Arbeit bei wiederkehrenden Vorgängen wie Anträgen, Dokumenten und Generalversammlungen.

## Requirements

### Validated

<!-- Aus Codebase abgeleitet — bereits ausgeliefert und in Nutzung. -->

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

### Active

<!-- Aktueller Milestone: GV-Anwesenheits-Erfassung. Hypothesen bis ausgeliefert. -->

- [ ] Vorstand kann eine Generalversammlung als eigene Entität anlegen (Datum, Titel, Status)
- [ ] Vorstand kann pro Helfer einen einmalig nutzbaren QR-Code erzeugen — mit einem freien Text-Namen als Memo (z. B. „Anna", „Bernd"); beim ersten Scan wird der QR-Code verbraucht und eine Helfer-Session daran gebunden
- [ ] Helfer kann sich per QR-Code-Scan in eine zeitlich begrenzte Helfer-Session einloggen, gültig bis zum Schließen der GV
- [ ] Helfer-Ansicht zeigt eine reduzierte Mitgliederliste (nur Mitgliedsnummer, Name, Titel, Anrede)
- [ ] Helfer kann in der Liste suchen und Mitglieder als anwesend markieren oder austragen
- [ ] Vorstand kann die Helfer-Ansicht ohne QR-Code direkt aus seiner regulären Anmeldung heraus öffnen, um zu unterstützen
- [ ] Vorstand sieht einen Live-Counter „X von Y anwesend" während der GV
- [ ] GV-Ergebnis (Anzahl Anwesender + Anwesenheits-Liste) bleibt nach GV-Schluss persistent für Protokoll und Statistik
- [ ] Helfer-Sessions werden beim Schließen der GV automatisch ungültig

### Out of Scope

<!-- Bewusste Grenzen für diesen Milestone. -->

- Stimmrechte, Vollmachten, Beschlussfähigkeits-Berechnung — eigenständiges Feature mit deutlich höherer Komplexität (Vollmachts-Daten, Stimmgewichte, Quorum-Regeln); rein anwesend/nicht-anwesend reicht für Protokoll
- Audit-Hashchain-Eintrag pro Anwesenheits-Markierung — vom User explizit ausgeschlossen, weil das Anhakeln nicht verbandskonform protokolliert werden muss
- Offline-Modus — Helfer brauchen Netzwerk; Synchronisation/Conflict-Resolution würde den Scope sprengen
- Live-Push zwischen Helfern (SSE/WebSocket) — Synchronisation nur bei Refresh/Suche; kein Doppel-Abhaken-Schutz erforderlich
- Stimmgewichts-Anzeige oder Anteils-Daten in der Helfer-Ansicht — Helfer-View bleibt bewusst minimal
- Native Mobile-App — Web-First, Helfer nutzen Browser auf Tablet/Laptop/Handy

## Context

**Technische Umgebung:**
- Rust 2021 Workspace mit getrennten Crates für DAO/Service/REST/Binary plus Dioxus-WASM-Frontend
- SQLite via SQLx, Migrations in `migrations/sqlite/`, BLOB-UUIDs, ISO8601-Timestamps
- Axum 0.8 + Utoipa-OpenAPI, Tokio 1.35
- Auth: axum-oidc gegen Nextcloud, tower-sessions, tower-cookies
- Frontend: Dioxus 0.6.3, Tailwind, Component-First (`genossi-frontend/src/component/`)
- Nix-Toolchain via `flake.nix`

**Bestehende Muster, die das GV-Feature aufgreifen wird:**
- Entity-Struktur: `id` (UUID/BLOB), `created`, `deleted` (Option), `version` (UUID, optimistic locking)
- DAO-Pattern: Trait-Definition + SQLite-Impl, Minimal-Interface (`create`, `update`, `dump_all`, `find_by_id`)
- Service-Pattern: Trait + `*Impl`-Struct, Permission-Context-Enforcement
- Audit-Macros: `audited_create!`, `audited_update!`, `audited_delete!` — werden für die GV-Anwesenheit explizit NICHT benötigt
- REST-Pattern: Axum-Handler + Utoipa-Schemas, ISO8601 datetime serde

**Parallel-System:**
- `openspec/changes/` enthält ein eigenständiges Spec-Workflow-System für ältere Änderungen; GSD wird parallel für den GV-Milestone verwendet, nicht als Ersatz

**Bekannte Verbesserungs-Areas (aus `.planning/codebase/CONCERNS.md`):**
- Tech-Debt-Backlog wurde am 2026-05-02 frisch gemappt (516 Zeilen Findings) und sollte bei Phase-Planung berücksichtigt werden, ohne den GV-Milestone zu blockieren

## Constraints

- **Tech stack**: Rust + Axum + SQLx + SQLite Backend, Dioxus WASM Frontend — keine Sprachwechsel oder DB-Wechsel im Scope dieses Milestones
- **Architektur**: Layered DAO/Service/REST muss eingehalten werden; neue Entitäten implementieren bestehende Trait-Patterns — Why: Konsistenz mit gemappter Codebase, Testbarkeit
- **Frontend**: Component-First-Prinzip — keine inline-RSX-Duplikate; identische UI-Bausteine wandern in `genossi-frontend/src/component/` — Why: gelernte Lektion, in `CLAUDE.md` und Memory festgeschrieben
- **Security**: Helfer-QR-Codes sind One-Time-Use; nach Scan invalid — Why: kein unkontrollierter Zugriff auf Mitgliederdaten, auch wenn der QR-Code weitergegeben wird
- **Datenschutz**: Helfer sehen nur Mitgliedsnummer, Name, Titel, Anrede — Why: minimale Datenexposition, DSGVO-konforme Helfer-Funktion
- **Audit-Pflicht**: Bestehende auditierte Entitäten (Member, MemberAction, MemberDocument, Application) müssen weiterhin Audit-Macros verwenden; neue GV-Entitäten benötigen das **nicht**
- **Verbandskonformität**: Genossenschaftsverband akzeptiert Excel-Listen ungern — Software muss als Ersatz so funktionieren, dass das Protokoll der GV nachvollziehbar Anwesenheits-Zahlen ausweist

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| GV als eigene Entität (`Assembly`) statt globalem Zustand | Mehrere GVs pro Jahr, Historie für Protokoll-Export, klare Lifecycle-Grenzen | — Pending |
| Anwesenheit als Join-Tabelle (`AssemblyAttendance`) statt Member-Flag | Saubere Mehrfach-GV-Historie, kein Datenverlust bei nächster GV | — Pending |
| One-Time-Use-QR-Codes pro Helfer | Verhindert Weitergabe des Zugangs an Unbefugte, ohne Helfer-Identität fest zu binden | — Pending |
| Helfer-Name als Freitext-Memo am QR-Code | Vorstand kann beim Anlegen sehen „QR für Anna" / „QR für Bernd"; rein UI-Hilfe, kein Identitäts-Anker, kein Audit-Bezug | — Pending |
| Sync zwischen Helfern nur bei Refresh, kein Live-Push | Doppel-Abhaken ist akzeptables Risiko (idempotente Anwesend-Markierung); keine SSE/WebSocket-Komplexität nötig | — Pending |
| Anwesenheit ohne Audit-Hashchain | Genossenschaftsverband fordert nur Anwesenheits-Anzahl im Protokoll, nicht den Vorgang des Abhakens | — Pending |
| Helfer-View auch für Vorstand zugänglich (ohne QR) | Vorstand will im Notfall am gleichen UI mithelfen können; vermeidet UI-Duplikat | — Pending |

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
*Last updated: 2026-05-02 after initialization*
