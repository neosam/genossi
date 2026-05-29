# Phase 2: Helfer-Token + Session + AuthContext::Helper - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-03
**Phase:** 2-helfer-token-session-authcontext-helper
**Areas discussed:** Token-Schema & Lifecycle, QR-Encoding & Klartext-Alphabet, AuthContext::Helper-Wiring & Session-Lebensdauer, REST-Endpoint-Vertrag

---

## Token-Schema & Lifecycle

### Q1 — Status-Modellierung

| Option | Description | Selected |
|--------|-------------|----------|
| Abgeleitet aus Spalten | Aus `used_at` + `revoked_at` ableiten; atomarer Redeem-UPDATE bleibt minimal | ✓ |
| Explizite Status-Enum-Spalte | Eigene `status TEXT`-Spalte wie `AssemblyStatus`; konsistent mit Phase 1, aber zwei Quellen-of-truth | |
| Du entscheidest | Claude wählt | |

**User's choice:** Abgeleitet aus Spalten
**Notes:** Reduziert Sync-Risiko zwischen Status-Spalte und State-Spalten; hält Atomic-UPDATE-WHERE-Clause klein.

### Q2 — Revoke nach Redeem

| Option | Description | Selected |
|--------|-------------|----------|
| Verboten — 409 Conflict | Revoke nur erlaubt solange `used_at IS NULL` | ✓ |
| Soft — setzt `revoked_at`, Session bleibt | `revoked_at` als Audit-Marker, Session läuft bis `closed_at` | |
| Cascade-Revoke | `revoked_at` + Session-DELETE | |

**User's choice:** Verboten — 409 Conflict
**Notes:** Cascade-Komplexität fällt weg; Session-Invalidation bleibt eine Phase-3-Frage (Cascade in `close_assembly`).

### Q3 — Audit-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Revoke | `audited_update!` mit `helper_token.revoke` | |
| Redeem | `audited_update!` mit `helper_token.redeem` | |
| Keine zusätzlichen Operationen | Nur Token-Erzeugung — strikt nach HLPR-07 | ✓ |

**User's choice:** Keine zusätzlichen Operationen
**Notes:** HLPR-07 fordert explizit nur Token-Erzeugung. Redeem ist Helfer-Aktion ohne Vorstands-Initiative; Revoke nicht in Audit-Hashchain — falls später gewünscht, ist die Erweiterung trivial.

### Q4 — Soft-Delete-Konvention

| Option | Description | Selected |
|--------|-------------|----------|
| Spalte reserviert ohne Delete-Pfad | Wie bei `assembly` (Phase 1) | ✓ |
| Nein, Token sind kurzlebig | Hard-Delete via späterem Cleanup-Job | |
| Du entscheidest | Claude wählt | |

**User's choice:** Spalte reserviert ohne Delete-Pfad
**Notes:** Genossi-Konvention; vermeidet spätere Migration. Konsistent mit Phase-1-D-04-Pattern.

---

## QR-Encoding & Klartext-Alphabet

### Q1 — QR-Inhalt

| Option | Description | Selected |
|--------|-------------|----------|
| URL mit Code als Query-Param | z.B. `${APP_URL}/helper?code=ABC...`; Kamera-App öffnet Helfer-Page direkt | ✓ |
| Nur der Klartext-Code | Helfer muss Frontend zuerst öffnen, dann scannen | |
| URL + Code im Pfad | `${APP_URL}/helper/redeem/ABC...`; gleiche UX, REST-Optik | |

**User's choice:** URL mit Code als Query-Param
**Notes:** Beste UX (ein Scan reicht); deckt häufigsten Workflow ab.

### Q2 — Alphabet

| Option | Description | Selected |
|--------|-------------|----------|
| Crockford Base32 | 32 Zeichen, ohne 0/O/1/I/L; ~5 bit/Zeichen | ✓ |
| Voll-Alphanumerisch | A-Z+a-z+0-9; 6 bit/Zeichen, aber Verwechslungsgefahr | |
| Nur Ziffern | Numpad-friendly, aber zu wenig Entropie ohne Rate-Limit | |

**User's choice:** Crockford Base32
**Notes:** 50 bit Entropie bei 10 Zeichen — weit über Brute-Force-Schwelle bei vorhandenem `tower_governor`-Rate-Limiter.

### Q3 — Längen-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Fix 10 Zeichen | Konstante Länge, einfache Frontend-Validation | ✓ |
| Range 8–12 wie REQUIREMENT formuliert | Strikte Lesart von HLPR-01 | |
| Du entscheidest | Claude wählt | |

**User's choice:** Fix 10 Zeichen
**Notes:** „8–12 alphanumerisch" in HLPR-01 ist erfüllt (10 ∈ [8,12]); fix vereinfacht Backend und Frontend.

### Q4 — QR-Crate

| Option | Description | Selected |
|--------|-------------|----------|
| `qrcode` 0.14 → SVG | De-facto-Standard im Rust-Ökosystem; pure-Rust | ✓ |
| `fast_qr` 0.13 | Neuere Alternative, kleinere Community | |
| Du entscheidest | Claude wählt | |

**User's choice:** `qrcode` 0.14
**Notes:** Stabil, pure-Rust ohne C-Dependency; SVG-Output direkt mit `EcLevel::Q` für gute Druckbarkeit.

---

## AuthContext::Helper-Wiring & Session-Lebensdauer

### Q1 — Cookie-Pfad

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse `app_session` mit Claims-Marker | Bestehender Cookie + JSON-Claims `{"kind":"helper", "assembly_id":...}` | ✓ |
| Eigener Cookie `helper_session` | Klare Trennung, aber mehr Auth-Middleware-Verzweigung | |
| Du entscheidest | Claude wählt | |

**User's choice:** Reuse `app_session` mit Claims-Marker
**Notes:** Auth-Middleware-Pfad bleibt kompakt; Claims-Schema (D-16) erlaubt Future-Erweiterung.

### Q2 — Expires-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Long-lived (24h) + Status-Check beim Verify | Defense-in-Depth; HLPR-05 schon in Phase 2 testbar | ✓ |
| Pure Cascade-Erwartung | Kein Status-Check; HLPR-05 in Phase 2 nicht E2E-testbar | |
| Hard-Bind: `expires = closed_at`, sonst +30 Tage | `expires` veraltet bei nachträglichem `close_assembly` | |

**User's choice:** Long-lived (24h) + Status-Check beim Verify
**Notes:** Erlaubt vollständigen E2E-Test von HLPR-05 in Phase 2; Phase-3-Cascade wird Optimierung statt Pflicht.

### Q3 — PermissionService-Verhalten

| Option | Description | Selected |
|--------|-------------|----------|
| Stub: explizit `PermissionDenied` | Helper-Variante immer denied; Phase 3 ergänzt positive Branch | ✓ |
| No-op: wie `None`/Unauthorized | Identisch im Effekt, aber semantisch unklar | |
| Du entscheidest | Claude wählt | |

**User's choice:** Stub: explizit `PermissionDenied`
**Notes:** Defensiv; verhindert versehentlichen Helfer-Zugriff auf Member-CRUD oder Assembly-Lifecycle.

### Q4 — `user_id`-Strategie für Helfer-Sessions

| Option | Description | Selected |
|--------|-------------|----------|
| Synthetischer User pro Token: `helper:<token_id>` | Eindeutige Auto-Registrierung, `revoke_all_for_user` pro Helfer | ✓ |
| Geteilter Helfer-User | Kompakter, aber kein Revoke-pro-Helfer, kein eindeutiger Audit | |
| Memo-Name: `helper:<memo>` | Memo ist nicht-eindeutig; UNIQUE-Constraint-Konflikte | |

**User's choice:** Synthetischer User pro Token: `helper:<token_id>`
**Notes:** Reused Pattern aus `ensure_user_and_create_session_with_claims` (bereits für „inventur token" da); ermöglicht eindeutige Audit-Spuren in Phase-3-Attendance-Aktionen.

---

## REST-Endpoint-Vertrag

### Q1 — Routing für Create/List

| Option | Description | Selected |
|--------|-------------|----------|
| Nested unter Assembly | `POST/GET /api/assembly/{id}/helper-tokens` | ✓ |
| Flat mit assembly_id im Body/Filter | `POST /api/helper-token`, bricht Aggregat-Pattern | |
| List eingebettet in `GET /api/assembly/{id}` | Detail-Response wird groß | |

**User's choice:** Nested unter Assembly
**Notes:** Konsistent mit Phase-1-`/api/assembly/{id}/open` und Aggregat-Konvention.

### Q2 — Redeem-Endpoint

| Option | Description | Selected |
|--------|-------------|----------|
| Öffentlich `POST /api/helper/redeem` | Helfer hat noch keine Session; klare Helfer-Domain | ✓ |
| Öffentlich `POST /api/assembly/{id}/helper-tokens/redeem` | Helfer kennt assembly_id beim ersten Call nicht | |
| Du entscheidest | Claude wählt | |

**User's choice:** Öffentlich `POST /api/helper/redeem`
**Notes:** Helfer kennt nur den Klartext-Code; assembly_id wird vom Backend aus dem Token-Hash abgeleitet.

### Q3 — HTTP-Status-Codes für Redeem-Fehler

| Option | Description | Selected |
|--------|-------------|----------|
| 404 / 410 / 403 | Differenziert: not-found / used / revoked-or-wrong-status | ✓ |
| 409 Conflict für alle Fehler | Anti-Probing, aber schlechte UX | |
| 401 für alle Fehler | Maximaler Anti-Probing, „Token ungültig"-Generic | |

**User's choice:** 404 / 410 / 403
**Notes:** Frontend kann differenziert reagieren („Code unbekannt — bitte prüfen" vs. „Code wurde bereits eingelöst" vs. „GV nicht offen / Code wurde widerrufen"). Anti-Probing wird über Rate-Limiting (Claude's Discretion) abgedeckt, nicht über Status-Code-Verschleierung.

### Q4 — Revoke-Erlaubnis

| Option | Description | Selected |
|--------|-------------|----------|
| Auch in `Open` erlaubt | Real-Welt: verlorenes Helfer-Tablet während GV | ✓ |
| Nur in `Preparation` (strikt nach HLPR-06) | Verhindert versehentliches Revoke während laufender GV | |
| Auch in `Closed` erlaubt | Nachträgliche Aufräumarbeit, geringer Wert | |

**User's choice:** Auch in `Open` erlaubt
**Notes:** HLPR-06 „vor GV-Beginn" gelesen als Default-UX; Real-Welt-Edge-Case (Tablet-Verlust) braucht Vorstands-Reaktion ohne Re-Open. Im Status `Closed`: 409 (Token-Liste eingefroren).

---

## Claude's Discretion

- Wo der Assembly-Status-Check beim Verify verdrahtet wird (SessionService erweitern, neuer HelperSessionService-Wrapper, oder im `extract_auth_context`).
- `TokenGenerator`-Service vs. freie Funktion mit `OsRng`.
- Hex-vs.-Base64-Encoding für `token_hash` in DB.
- `APP_URL`-Default-Verhalten beim Token-Create, falls Env nicht gesetzt (fail-fast am Start vs. fail beim ersten Create).
- Index-Strategie für `helper_token` (`(assembly_id)`, UNIQUE auf `(token_hash)`).
- `session_id`-FK-ON-DELETE-Verhalten (`SET NULL` empfohlen vs. `RESTRICT`).
- Pro-IP-Rate-Limiting auf `/api/helper/redeem` via `tower_governor`.

## Deferred Ideas

### Phase 3
- Cascade-Invalidation in `close_assembly` als Optimierung.
- Positive PermissionService-Branch für `AuthContext::Helper`.
- `AttendanceMemberTO`-Erzeugung (4 Felder).
- Live-Stats-Endpoint `GET /api/assembly/{id}/stats` (ASSY-04).

### Phase 4
- Manual-Code-Eingabe-UI (HLPR-03).
- QR-Scanner-Integration (BarcodeDetector + Polyfill).

### Spätere Phasen / Out of Scope
- Bulk-QR-Erzeugung (BULK-01/BULK-02) — v2 in REQUIREMENTS.md.
- Audit-Log für Redeem/Revoke — falls Vorstand später nachfragt, One-Liner-Erweiterung möglich.
- `tower-sessions` 0.14 → 0.15 Upgrade (STATE.md-TODO) — nicht in Phase 2 erforderlich.
- Differenzierte `manage_helper_tokens`-Permission — Phase 2 nutzt `admin`.
