# Roadmap: Genossi — GV-Anwesenheits-Erfassung

**Created:** 2026-05-02
**Granularity:** coarse (5 Phasen, 1–3 Plans pro Phase)
**Coverage:** 22/22 v1 Requirements mapped
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar, mit weniger manueller Arbeit. Dieser Milestone bringt papierlose Anwesenheits-Erfassung auf der Generalversammlung.

## Phases

- [ ] **Phase 1: Assembly-Aggregat + Audit-Hardening** — Vorstand kann Generalversammlungen anlegen, öffnen (mit Member-Universe-Snapshot) und schließen; alle Lifecycle-Operationen sind auditiert
- [ ] **Phase 2: Helfer-Token + Session + AuthContext::Helper** — Vorstand erzeugt One-Time-Use-QR-Codes; Helfer können diese atomar einlösen und erhalten eine GV-gebundene Session
- [ ] **Phase 3: Attendance-Aggregat + Cascade-Invalidation** — Backend liefert reduzierte Helfer-Member-View, idempotente Anwesenheits-Toggles und Live-Stats; GV-Schluss invalidiert Sessions
- [ ] **Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback** — Vorstand und Helfer bedienen GV, QR-Erzeugung, Anwesenheit und Live-Counter über Dioxus-WASM-UI
- [ ] **Phase 5: Pre-GV-Generalprobe und Operations-Plan** — System ist auf realer Hardware geprobt, Vorstand geschult, Backup-Plan dokumentiert

## Phase Details

### Phase 1: Assembly-Aggregat + Audit-Hardening
**Goal**: Vorstand kann Generalversammlungen über die API verwalten — anlegen, öffnen mit Member-Universe-Snapshot, schließen — und jede Lifecycle-Aktion ist über die bestehende Audit-Hashchain belegbar.
**Depends on**: Nothing (erste Phase)
**Requirements**: ASSY-01, ASSY-02, ASSY-03, ASSY-05, ASSY-07
**Success Criteria** (was muss WAHR sein):
  1. Vorstand kann eine GV mit Datum und Titel anlegen; sie startet im Status `Vorbereitung` (ASSY-01)
  2. Vorstand kann eine GV öffnen — beim Öffnen wird ein Member-Universe-Snapshot persistiert, der das stabile „Y" für den späteren Counter definiert (ASSY-02)
  3. Vorstand kann eine GV schließen; Status wechselt final auf `Geschlossen` und kann nicht zurückgesetzt werden (ASSY-03)
  4. GV-Daten (Member-Universe-Snapshot, Snapshot-Anzahl) bleiben nach Schluss persistent für Protokoll-Export und Statistik (ASSY-05)
  5. `GET /api/audit/verify` zeigt nach Lifecycle-Vorgängen (create, open, close) eine intakte Hash-Chain mit den entsprechenden Einträgen; CI-E2E-Test gegen den Verify-Endpoint ist grün (ASSY-07)
**Plans**: 5 plans
- [x] 01-01-PLAN.md — Migrationen + DAO-Traits + SQLite-DAO-Impls (assembly + assembly_member_snapshot)
- [x] 01-02-PLAN.md — REST-Types (AssemblyTO, AssemblyStatusTO, AssemblyDetailTO, CreateAssemblyRequest, UpdateAssemblyRequest)
- [x] 01-03-PLAN.md — AssemblyService Trait + AssemblyServiceImpl mit Lifecycle-Guards, atomarer Open-Tx, Audit-Macros
- [ ] 01-04-PLAN.md — Axum-Handler + Router-Registration + DI-Wiring in genossi_bin
- [ ] 01-05-PLAN.md — E2E-Test fuer Lifecycle + Audit-Hashchain-Verifikation (D-12, ASSY-07)

### Phase 2: Helfer-Token + Session + AuthContext::Helper
**Goal**: Vorstand kann pro Helfer einen einmalig nutzbaren QR-Token mit Memo-Namen erzeugen und vor GV-Beginn revoken; Helfer kann den Token atomar einlösen und erhält eine zeitlich an die GV gebundene Session — mit dafür typsicherer `AuthContext::Helper`-Variante, die Phase 3 für Permission-Checks braucht.
**Depends on**: Phase 1 (FK auf Assembly; close-Hook für Session-Lebensdauer)
**Requirements**: HLPR-01, HLPR-02, HLPR-04, HLPR-05, HLPR-06, HLPR-07
**Success Criteria** (was muss WAHR sein):
  1. Vorstand kann pro Helfer einen Token mit Freitext-Memo-Name erzeugen; das Backend liefert sowohl ein QR-SVG als auch einen 8–12-Zeichen-alphanumerischen Klartext-Code zurück (HLPR-01)
  2. Helfer kann einen gültigen Token via Redeem-Endpoint einlösen; Backend führt den Redeem in einem einzigen `UPDATE ... WHERE used_at IS NULL RETURNING ...` aus und bindet eine Session an die GV (HLPR-02)
  3. E2E-Race-Test mit zwei parallelen Redeem-Requests auf demselben Token zeigt exakt einen Erfolg und einen Fehler — kein Doppel-Login möglich (HLPR-04)
  4. Helfer-Session-Lebensdauer ist an `assembly.closed_at` gebunden; nach Schließen der GV ist sie ungültig, auch wenn das Cookie noch im Browser liegt (HLPR-05)
  5. Vorstand sieht in der GV-Detail-API die Liste aller Token mit Memo-Name und Status (offen/eingelöst); offene Token können vor GV-Beginn revoked werden (HLPR-06)
  6. Token-Erzeugung erscheint in der Audit-Hashchain mit Memo-Name, Erzeuger, Timestamp und GV-Bezug (HLPR-07)
  7. `AuthContext::Helper { session_id, assembly_id }` ist als typsichere Enum-Variante verfügbar und wird vom Session-Extract-Pfad korrekt aus den Session-Claims rekonstruiert
**Plans**: TBD

### Phase 3: Attendance-Aggregat + Cascade-Invalidation
**Goal**: Backend stellt reduzierte (DSGVO-konforme) Helfer-Mitgliederliste, idempotente Anwesenheits-Toggles, einen Live-Stats-Endpunkt und einen Vorstand-Post-Close-Edit-Endpoint bereit; das Schließen einer GV invalidiert kaskadierend alle zugehörigen Helfer-Sessions.
**Depends on**: Phase 2 (`AuthContext::Helper`-Variante; Helfer-Session-Mechanik)
**Requirements**: ASSY-04, ASSY-06, ATTN-01, ATTN-02, ATTN-03, ATTN-04, ATTN-05, ATTN-06, SYNC-02
**Success Criteria** (was muss WAHR sein):
  1. Helfer-API `GET /api/attendance/:assembly_id/members` liefert ausschließlich Mitgliedsnummer, Name, Titel und Anrede — Test verifiziert, dass das Response-JSON keine PII-Felder wie IBAN, Adresse, Geburtsdatum, Email enthält (ATTN-01)
  2. Helfer-API unterstützt Substring-Suche auf Name oder Mitgliedsnummer; ein Helfer findet ein Mitglied per Suchparameter (ATTN-02)
  3. Idempotenter `PUT /api/attendance/:aid/:mid` markiert ein Mitglied als anwesend; fünfmaliges Aufrufen liefert fünfmal 200 OK und genau einen Anwesenheits-Datensatz (ATTN-03)
  4. Idempotentes Austragen (anwesend → nicht-anwesend) funktioniert ebenfalls und ist in der Reverse-Richtung idempotent (ATTN-04)
  5. Anwesenheits-Markierungen werden bewusst NICHT in die Audit-Hashchain geschrieben; Test bestätigt, dass die Hash-Chain nach 100 Toggles unverändert bleibt (ATTN-05)
  6. Vorstand mit OIDC-Session ruft denselben Helfer-View ohne QR-Token erfolgreich auf; Permission-Check akzeptiert sowohl `AuthContext::Helper { assembly_id == X }` als auch admin-Permission (ATTN-06)
  7. `GET /api/assembly/:id/stats` liefert `{present, total}` für den Live-Counter (X von Y); concurrent-Doppel-Markierungs-Test durch zwei simulierte Helfer erzeugt keinen Fehler und keinen Duplikat-Eintrag (ASSY-04, SYNC-02)
  8. `close_assembly` invalidiert kaskadierend alle Helfer-Sessions dieser GV; nach Schließen schlägt jeder Helfer-Request mit 401 fehl
  9. Vorstand kann nach GV-Schluss Anwesenheits-Einträge ergänzen oder entfernen (Post-Close-Edit-Endpoint), ohne dass sich der GV-Status ändert; die Aktionen erscheinen weiterhin in der Audit-Hashchain (ASSY-06)
**Plans**: TBD

### Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback
**Goal**: Vorstand und Helfer bedienen das vollständige GV-Feature über das Dioxus-WASM-Frontend — GV anlegen/öffnen/schließen, QR-Codes erzeugen und drucken, Helfer-Login per QR-Scan ODER manuellem Code, reduzierte Mitgliederliste mit Suche, idempotenter Anwesenheits-Toggle, Live-Counter mit expliziter Y-Beschriftung. Components sind wiederverwendbar in `genossi-frontend/src/component/`, keine inline-RSX-Duplikate.
**Depends on**: Phase 3 (vollständige Backend-API; REST-Types-Schemas)
**Requirements**: HLPR-03, SYNC-01
**Success Criteria** (was muss WAHR sein):
  1. Helfer kann sich per QR-Scan über die Browser-Kamera (BarcodeDetector + Polyfill) in eine Session einloggen und gelangt direkt in die Anwesenheits-Ansicht
  2. Helfer kann alternativ den 8–12-Zeichen-Klartext-Code manuell in ein Eingabefeld tippen und damit dieselbe Session erzeugen — als Fallback bei Camera-Permission-Verweigerung oder Scanner-Fehlfunktion (HLPR-03)
  3. Vorstand sieht den Live-Counter in der Form „X von Y aktiven Mitgliedern" mit expliziter Y-Beschriftung; Polling alle ~5s aktualisiert die Zahl ohne Live-Push
  4. Helfer sehen aktualisierte Anwesenheits-Status beim nächsten Refresh oder beim nächsten Such-Vorgang (kein SSE/WebSocket nötig) (SYNC-01)
  5. Helfer-View und Vorstand-View teilen sich dieselben Components (`AttendanceRow`, `AttendanceSearch`, `LiveCounter`, `QrCard`, `QrScanner`); ein UI-Code-Diff zwischen den beiden Pages zeigt nur Auth-spezifische Top-Bar-Unterschiede, keine duplizierte Liste oder Suche
  6. Connection-Banner erscheint klar sichtbar bei Verbindungsverlust; Anwesenheits-Markierungen werden erst nach 200-OK-Response visuell bestätigt (kein Optimistic-UI-Phantom-Häkchen)
**Plans**: TBD
**UI hint**: yes

### Phase 5: Pre-GV-Generalprobe und Operations-Plan
**Goal**: Das fertige System ist mindestens eine Woche vor der echten GV unter realistischen Bedingungen geprobt — echtes Vereinsheim oder vergleichbare Umgebung, echte Hardware (iOS Safari + Android Chrome auf Tablet/Handy), echter Drucker für QR-Codes, mehrere Test-Helfer, eine Test-GV mit ≥10 Test-Mitgliedern in der DB. Vorstand bedient das System ohne Entwickler-Beistand. Backup-Plan (Mobile-Hotspot, gedruckte Mitgliederliste, Excel-Import-Pfad) ist schriftlich dokumentiert.
**Depends on**: Phase 4 (vollständiges System inkl. UI)
**Requirements**: (operativ — keine direkten REQ-IDs; verifiziert die Phasen 1–4 unter realen Bedingungen)
**Success Criteria** (was muss WAHR sein):
  1. `OPERATIONS.md` existiert mit dokumentierter Pre-Event-Checkliste, HTTPS-Setup-Anleitung für den GV-Tag (Caddy/mkcert/Cloudflare-Tunnel), Backup-Plan (Mobile-Hotspot + gedruckte Mitgliederliste + Excel-Import-Recovery-Pfad)
  2. Generalprobe wurde durchgeführt mit echtem Tablet (mindestens iOS Safari + Android Chrome), echtem Drucker, drei Test-Helfern und ≥10 Test-Mitgliedern; Vorstand hat selbständig durchgespielt: GV anlegen, QR-Codes erzeugen und drucken, Helfer einloggen lassen, Anwesenheit markieren, Counter beobachten, GV schließen, Anwesenheits-Liste exportieren oder einsehen
  3. iOS-Safari-Kamera-Permission-Pfad wurde manuell auf realem iPhone/iPad verifiziert (nicht nur DevTools-Emulation); Manual-Code-Fallback wurde aktiv getestet, indem die Kamera-Permission verweigert wurde
  4. Helfer-Briefing existiert schriftlich (1-Seiten-Anleitung); Vorstand-Schulung wurde ohne Entwickler-Anwesenheit erfolgreich abgeschlossen
  5. Generalprobe-Datum liegt mindestens 7 Kalendertage vor dem echten GV-Termin
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Assembly-Aggregat + Audit-Hardening | 0/5 | Not started | - |
| 2. Helfer-Token + Session + AuthContext::Helper | 0/0 | Not started | - |
| 3. Attendance-Aggregat + Cascade-Invalidation | 0/0 | Not started | - |
| 4. Frontend (Component-First) | 0/0 | Not started | - |
| 5. Pre-GV-Generalprobe und Operations-Plan | 0/0 | Not started | - |

## Coverage Summary

| Category | Requirements | Phase Mapping |
|----------|--------------|---------------|
| ASSY (Assembly-Lifecycle) | 7 | 5 in Phase 1, 2 in Phase 3 (ASSY-04 Stats-Endpoint, ASSY-06 Post-Close-Edit) |
| HLPR (Helfer-Token & Session) | 7 | 6 in Phase 2, 1 (HLPR-03 Manual-Code-UI) in Phase 4 |
| ATTN (Anwesenheit) | 6 | 6 in Phase 3 |
| SYNC (Multi-Helfer-Sync) | 2 | SYNC-02 in Phase 3 (Backend-Idempotenz), SYNC-01 in Phase 4 (Refresh-UX) |
| **Total v1** | **22** | **22 mapped, 0 orphans** |

## Phase Ordering Rationale

- **Phase 1 vor 2:** HelperPreToken hat FK auf Assembly; ohne Assembly-Entität gibt es keinen Anker für Token oder Sessions
- **Phase 2 vor 3:** Phase 3 (Attendance) braucht `AuthContext::Helper` für den Permission-Check; diese Variante landet zwingend in Phase 2, nicht später
- **Phase 3 vor 4:** Genossi-Konvention Backend-First; das Frontend konsumiert fertige API-Schemas aus `genossi_rest_types`
- **Phase 4 vor 5:** Generalprobe testet das fertige System; Phase 5 ist Verifikation und Operations, keine Entwicklung
- **Audit-CI-Hardening:** In Phase 1 gefaltet, weil die Hash-Chain bereits beim ersten Lifecycle-Vorgang Stress sieht — kein separater Vor-Phase-Aufwand nötig

## Hard Constraints (carry-over aus Research)

Diese Punkte sind nicht verhandelbar und müssen in den jeweiligen Phasen-Plans als Must-Have-Tasks landen:

- **Phase 1**: Member-Universe-Snapshot beim Öffnen der GV (definiert stabiles Y); Audit-Macros (`audited_create!`, `audited_update!`) für create/open/close/post-close-edit
- **Phase 2**: Atomarer Redeem via `UPDATE qr_token SET used_at = ?, session_id = ? WHERE id = ? AND used_at IS NULL RETURNING ...`; SHA256-Hash des Tokens in der DB, Klartext nur einmal ausgegeben; `AuthContext::Helper`-Enum-Variante
- **Phase 3**: Eigenes `AttendanceMemberTO` mit nur 4 Feldern (NICHT `MemberTO` mit serde-skip); idempotenter PUT (kein POST/INSERT-Pattern); UNIQUE(assembly_id, member_id) WHERE deleted IS NULL; Cascade-Invalidation in `close_assembly`
- **Phase 4**: Component-First (keine inline-RSX-Duplikate); Manual-Code-Eingabe muss alongside QR-Scanner landen; Y im Live-Counter explizit beschriftet („X von Y aktiven Mitgliedern"); BarcodeDetector + Polyfill; HTTPS für `getUserMedia`
- **Phase 5**: Generalprobe ist gleichrangig mit Code-Phasen; ohne sie sind die Phasen 1–4 nicht „done done"

---
*Roadmap created: 2026-05-02*
*Last updated: 2026-05-02 after initial roadmap*
