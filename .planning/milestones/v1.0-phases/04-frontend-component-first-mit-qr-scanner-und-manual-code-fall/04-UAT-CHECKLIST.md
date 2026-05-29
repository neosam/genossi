# Phase 4 UAT Checklist (Manual Acceptance)

> **Zweck:** Manuelle Verifikation aller user-facing Acceptance-Kriterien (HLPR-03, SYNC-01, ATTN-06, Datenschutz, ROADMAP SC#1-6) auf realer Hardware/echtem Browser. Wird in Phase-5 Generalprobe ODER lokal durch den Entwickler abgehakt.
>
> **Voraussetzung:** automatische Verifikation laut `04-VERIFICATION.md` ist abgeschlossen (13 PASS / 1 FAIL — Pitfall 6 / 1 PENDING — wasm-bindgen-cli).

---

## Pre-Flight Setup

- [ ] **Backend:** `cargo run --bin genossi` läuft (Terminal 1, default `localhost:3000`)
- [ ] **Tailwind Watch:** `cd genossi-frontend && npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch` (Terminal 2)
- [ ] **Frontend:** `cd genossi-frontend && dx serve` läuft (Terminal 3, default `localhost:8080`)
- [ ] Browser: `http://localhost:8080` öffnet ohne Errors
- [ ] **Test-DB:** mindestens 3 Test-Mitglieder vorhanden (anders bleibt der Live-Counter trivial)
- [ ] **Vorstand-Login:** via OIDC (Nextcloud) ODER `--features mock_auth` für lokales Testing
- [ ] **Pre-Flight Build-Fix:** `dx build --release` läuft fehlerfrei durch (ggf. `cargo install wasm-bindgen-cli --version 0.2.104` vorher)
- [ ] **Pre-Flight Tailwind-Fix:** `grep "qr-card" target/dx/genossi-frontend/release/web/public/assets/tailwind.css` → ≥1 Treffer (Pitfall 6 acceptance — siehe 04-VERIFICATION.md Check 14)

---

## Block A — Vorstand-Workflow

### A1: GV anlegen
- [ ] Login als admin (Cookie/Session vorhanden)
- [ ] Navigate zu `http://localhost:8080/assemblies`
- [ ] Page lädt ohne Console-Errors
- [ ] Empty-State (oder bestehende Liste) sichtbar mit Button "Neue GV anlegen"
- [ ] Klick "Neue GV anlegen" → Modal öffnet sich
- [ ] Form ausfüllen: Name "UAT Test-GV 2026", Datum heute, Ort "Testraum"
- [ ] Klick "Speichern" → Modal schließt; neue GV erscheint in der Liste
- [ ] Status-Badge zeigt "Vorbereitung" (grau)

### A2: GV-Detail-Page öffnen
- [ ] Klick auf den Listen-Eintrag → Navigate zu `/assemblies/{id}`
- [ ] Header zeigt GV-Name "UAT Test-GV 2026" + Status-Badge "Vorbereitung"
- [ ] 3 Tabs sichtbar: **Stamm-Daten** / **Helfer-Tokens** / **Anwesenheit**
- [ ] Tab-Wechsel funktioniert (Tab-Indicator wechselt zu blau-600)

### A3: Helfer-Token erzeugen + drucken (D-09 + Pitfall 6)
- [ ] Tab "Helfer-Tokens" wählen
- [ ] Klick "Token erzeugen" → Modal öffnet sich
- [ ] Memo "Anna" eingeben → Klick "Erzeugen" → Modal schließt
- [ ] **QrCard erscheint inline** mit:
  - [ ] GV-Header "Helfer-Code für Anna" (oder analoger Text)
  - [ ] QR-SVG rendert sichtbar (kein Broken-Image)
  - [ ] 10-Char Klartext-Code in font-mono, leicht lesbar
  - [ ] "Drucken"-Button (blau, sichtbar) + "Schließen"-Button
- [ ] **WICHTIG:** Code-String **NOTIEREN** (für Block B+C unverzichtbar — nicht recoverable nach Schließen!)
- [ ] **Print-Test (Pitfall 6 acceptance):** Klick "Drucken" → Browser-Print-Dialog öffnet
  - [ ] Print-Preview zeigt **NUR die QrCard**: kein TopBar, kein Footer, A4 portrait, zentriert
  - [ ] QR und Code sind **klar lesbar** in der Print-Preview
  - [ ] **FAIL-Indikator:** Wenn die Preview komplett **leer/weiß** ist → Pitfall 6 noch nicht behoben → Issue eskalieren
- [ ] Print-Dialog schließen, Klick "Schließen" auf der Card
- [ ] Card verschwindet — Code/QR sind jetzt unrecoverable (D-21)
- [ ] Token-Liste zeigt einen Eintrag "Anna" mit Status-Badge "Offen" (gelb)
- [ ] **Optional:** Token "Bernd" erzeugen für Block B4 / C3 (Multi-Helfer-Test)

### A4: GV öffnen mit Confirmation
- [ ] Tab "Stamm-Daten" wählen
- [ ] Klick "GV öffnen" → Confirm-Dialog mit Text "Beim Öffnen wird... Snapshot..."
- [ ] Klick "Abbrechen" → Dialog schließt; Status bleibt **Vorbereitung**
- [ ] Erneut Klick "GV öffnen" → Confirm bestätigen
- [ ] Status-Badge wechselt auf "Offen" (grün)
- [ ] Button-Text ändert sich von "GV öffnen" zu "GV schließen"

---

## Block B — Helfer-Login (HLPR-03 + D-03 + ROADMAP SC#1, SC#2)

### B1: Auto-Redirect bei vorhandener Session (D-08)
- [ ] Vorstand-Cookie löschen (DevTools → Application → Cookies)
- [ ] **Wenn bereits Helfer-Session existiert** (von vorigem Lauf): Navigate zu `/helper`
  - [ ] Automatischer Redirect zu `/helper/attendance` (innerhalb <1s)
- [ ] **Wenn nicht:** Login-UI wird angezeigt (kein Redirect)

### B2: HLPR-03 Manual-Code-Path — **PFLICHT** (Hauptpfad iOS!)
- [ ] Navigate zu `http://localhost:8080/helper`
- [ ] Beide Pfade sichtbar nebeneinander/untereinander:
  - [ ] "QR-Code scannen"-Button (mit Camera-Icon)
  - [ ] Manual-Code-Input-Field (10-Zeichen-Slot)
- [ ] **Camera-Permission wurde NICHT angefragt** (Browser-URL-Bar zeigt KEIN Camera-Icon) — D-03
- [ ] In Manual-Input "abc" eintippen
  - [ ] Live-Filter macht "abc" → "ABC" (uppercase)
  - [ ] Submit-Button **disabled** (zu wenige Zeichen)
- [ ] In Manual-Input verbotene Zeichen testen: "ABCILU"
  - [ ] Frontend-Filter strippt I, L, U → bleibt "ABC"
  - [ ] Submit weiter disabled
- [ ] **10 Crockford-Zeichen** eintippen (z.B. zufälliger Falsch-Code "1234567890")
  - [ ] Submit-Button **enabled**
  - [ ] Klick Submit → Inline-Error unter Input: "Code nicht erkannt..." (404)
- [ ] **Korrekten Code aus Block A3 (Anna) eintippen**
  - [ ] Submit klicken
  - [ ] Inline-Error verschwindet
  - [ ] Page navigate auf `/helper/attendance`
- [ ] HelperShell-Header zeigt GV-Name "UAT Test-GV 2026"
- [ ] **KEIN TopBar mit Vorstand-Links sichtbar** (D-07 + Datenschutz):
  - [ ] kein Mitglieder-Link
  - [ ] kein Audit-Link
  - [ ] kein Mail/Inbox-Link
  - [ ] kein Backup-Link
  - [ ] kein Permissions-Link
- [ ] Sichtbar: NUR "Helfer-Modus" (oder analoger Indikator) + GV-Name + Logout-Button

### B3: Helfer-Logout (D-08)
- [ ] In HelperShell-Header: Klick "Abmelden"
- [ ] Page navigate zu `/helper`
- [ ] Login-UI wieder sichtbar (kein Auto-Redirect zu `/helper/attendance`)
- [ ] DevTools Cookies: Helfer-Cookie ist gelöscht/expired

### B4: QR-Scan-Path (ROADMAP SC#1) — Optional bei Camera-Verfügbarkeit
- [ ] Block A3 wiederholen mit neuem Memo "Bernd" (alter Code aus B2 ist invalid nach Redeem!)
- [ ] Auf Helfer-Page (`/helper`) Klick "QR-Code scannen"
- [ ] **Browser fragt Camera-Permission** (Permission-Dialog erscheint)
- [ ] Permission **GRANT** → schwarzes Camera-Frame mit Live-Stream wird angezeigt
- [ ] QR-Code (z.B. von zweitem Telefon-Display oder ausgedruckt aus A3) ins Frame halten
- [ ] Automatischer Scan + Navigate zu `/helper/attendance` (innerhalb <2s)
- [ ] **Optional Permission-DENY-Test** (separater Browser-Profil/Inkognito):
  - [ ] Permission verweigern → Inline-Error "Kamera-Zugriff verweigert. Bitte Code manuell eingeben."
  - [ ] Manual-Code-Input bleibt sichtbar (HLPR-03 Fallback funktioniert)

### B5: Camera-Lifecycle (RESEARCH Pitfall 2 / T-04-19) — Optional bei Camera
- [ ] Block B4 erneut starten mit aktivem Scan-Modal
- [ ] **Browser-Camera-Indicator** (URL-Bar-Symbol oder OS-Statusleiste) zeigt "Camera in use" (rot/Punkt)
- [ ] Klick X (Schließen) im Scan-Modal **OHNE** zu scannen
- [ ] **Browser-Camera-Indicator verschwindet sofort** — `use_drop` hat `track.stop()` aufgerufen
- [ ] **FAIL-Indikator:** Indicator bleibt grün/rot nach Schließen → Camera-Leak → Issue eskalieren

---

## Block C — Anwesenheits-Erfassung (SYNC-01 + ATTN-06 + ROADMAP SC#3-6)

### C1: Live-Counter (ROADMAP SC#3)
- [ ] Helfer-Login (Block B2 oder B4)
- [ ] `/helper/attendance` zeigt LiveCounter mit literalem Text:
  - [ ] **"X von Y anwesend"** (z.B. "0 von 3 anwesend") — NICHT "X/Y", NICHT "X anwesend"
  - [ ] **Y entspricht der Anzahl der Test-Mitglieder** beim Öffnen der GV (Member-Universe-Snapshot aus A4)
- [ ] AttendanceList unten rendert alle Y Mitglieder

### C2: No-Optimistic-UI (ROADMAP SC#6 + D-17)
- [ ] Klick auf Anwesend-Toggle (Häkchen-Button) eines Mitglieds
- [ ] Button zeigt **Loading-Spinner** (KEIN sofortiges Häkchen!)
- [ ] Innerhalb <1s: Häkchen erscheint nach 200-OK Response (gate-by-server)
- [ ] LiveCounter aktualisiert auf "1 von 3 anwesend"
- [ ] Erneut klicken (gleiches Mitglied) → Loading-Spinner erscheint
- [ ] Nach <1s: Häkchen verschwindet (mark_absent erfolgreich)
- [ ] LiveCounter aktualisiert auf "0 von 3 anwesend"

### C3: SYNC-01 — Multi-Helfer-Refresh (Race-Test)
- [ ] **Voraussetzung:** Zweiter Helfer-Token "Bernd" aus Block A3/B4 verfügbar
- [ ] **Browser-Tab 1:** Helfer A — `/helper/attendance` mit Code "Anna"
- [ ] **Browser-Tab 2:** Helfer B — `/helper/attendance` mit Code "Bernd"
  - **ALT:** Vorstand `/assemblies/{id}` Tab "Anwesenheit" (mit anderer Browser-Identity oder Inkognito)
- [ ] In Tab 1: Mitglied "Test 1" als anwesend markieren (Häkchen erscheint nach 200-OK)
- [ ] **Erwartung:** Tab 2 zeigt nach max ~5s (1 Polling-Tick) den neuen LiveCounter ("1 von 3 anwesend")
- [ ] **Erwartung:** AttendanceList in Tab 2 — Mitglied "Test 1" zeigt nach Refresh ebenfalls Häkchen
- [ ] **Schnelle-Refresh-Test:** in Tab 2 Such-Vorgang triggern (z.B. Search-Field fokussieren + Eingabe machen + clearen) → forced Refresh, Mitglied "Test 1" zeigt sofort Häkchen

### C4: ATTN-06 — Component-Reuse Visueller Diff
- [ ] **Tab 1 (Helfer):** `/helper/attendance` öffnen
  - [ ] AttendanceList rendert Rows mit **5 sichtbaren Feldern** pro Row:
    1. Mitglieds-Nummer (#XX)
    2. Anrede (z.B. "Herr"/"Frau"/"Divers" — optional, leer wenn nicht gesetzt)
    3. Titel (z.B. "Dr." — optional)
    4. Vorname
    5. Nachname
  - [ ] Toggle-Button rechts in jeder Row
- [ ] **Tab 2 (Vorstand):** `/assemblies/{id}` → Tab "Anwesenheit"
  - [ ] AttendanceList rendert mit **identischer Row-Struktur** (5 Felder)
  - [ ] Toggle-Button rechts in jeder Row
- [ ] **Visueller Diff (CRITICAL):**
  - [ ] **Beide Listen sehen identisch aus** (gleiches Row-Padding, gleicher Toggle-Style, gleicher Counter-Style, gleiche AttendanceSearch-Box)
  - [ ] **Einziger Unterschied** ist die Top-Bar:
    - Tab 1 (Helfer): HelperShell-Header (GV-Name + Logout)
    - Tab 2 (Vorstand): Vorstand-TopBar (vollständige Navigation)
  - [ ] **FAIL-Indikator:** Wenn Row-Layouts/Spacing/Farben divergieren → ATTN-06 Component-Reuse-Verletzung → Issue eskalieren

### C5: ConnectionBanner (D-16 + ROADMAP SC#6)
- [ ] Helfer-Page geöffnet halten (`/helper/attendance`)
- [ ] Backend stoppen (`Ctrl+C` im genossi-bin Terminal 1)
- [ ] Nach 2 Polling-Ticks (~10-15s):
  - [ ] **Amber sticky-banner** erscheint oben am Page-Rand
  - [ ] Text: "Verbindung instabil — letzte Aktualisierung vor mehr als 10 Sekunden." (oder analog)
- [ ] LiveCounter zeigt "— von 3 anwesend" (X dashed/leer; Y bleibt = Snapshot)
- [ ] Backend wieder starten (`cargo run --bin genossi`)
- [ ] Bei nächstem Poll-Tick (~5s):
  - [ ] Banner verschwindet
  - [ ] Counter zeigt wieder "X von Y anwesend" (X = aktueller Server-Stand)

---

## Block D — Datenschutz (CLAUDE.md §Datenschutz)

### D1: AttendanceList nur 5 Felder (PII-Whitelist)
- [ ] Helfer-Page Anwesenheits-Liste, beliebige Row
- [ ] DevTools → Inspect Element auf einer Row
- [ ] **Innerer Text der Row enthält NUR**: Mitglieds-Nummer, Anrede (optional), Titel (optional), Vorname, Nachname
- [ ] **NICHT enthalten** (DOM-Inspect bestätigt):
  - [ ] keine Email
  - [ ] keine IBAN
  - [ ] keine Adresse (Straße, PLZ, Ort)
  - [ ] kein Geburtsdatum
  - [ ] keine Telefonnummer
  - [ ] keine sonstige PII
- [ ] **Network-Tab Check:** API-Response von `GET /api/helper/attendance` (oder analog) enthält ebenfalls KEINE PII über die 5 Felder hinaus

### D2: HelperShell ohne Vorstand-Navigation
- [ ] `/helper` (Login-Page): KEIN Mitglieder-Link, KEIN Audit-Log-Link, KEIN Mail-Link, KEIN Permissions-Link, KEIN Backup-Link sichtbar
- [ ] `/helper/attendance` (nach Login): NUR GV-Name + Logout-Button im Header — keine sonstige Navigation

### D3: Helfer-API-Endpoint-Coverage
- [ ] DevTools Network-Tab: alle Requests vom Helfer-Browser gehen NUR an `/api/helper/*`-Routen
  - [ ] keine Calls auf `/api/members`, `/api/applications`, `/api/audit`, `/api/mail`, etc.
- [ ] **FAIL-Indikator:** Helfer-Browser ruft Vorstand-Endpoints → Permission-Layer-Bug → Issue eskalieren

---

## Block E — GV-Lifecycle abschließen

### E1: GV schließen + Helper-Session-Cascade
- [ ] Vorstand-Browser zurück zu `/assemblies/{id}` → Tab "Stamm-Daten"
- [ ] Klick "GV schließen" → Confirm-Dialog mit Text "Nach dem Schließen werden alle aktiven Helfer-Sessions ungültig..."
- [ ] Bestätigen → Status-Badge wechselt auf "Geschlossen" (blau)
- [ ] **Cascade-Test:** im Helfer-Tab (Tab 1 aus B2):
  - [ ] Nächster Toggle-Klick liefert 401 Unauthorized
  - [ ] Toast/Inline-Error erscheint: "Session abgelaufen" (oder analog)
  - [ ] Page navigate automatisch zu `/helper`

### E2: Vorstand-Edit nach GV-Schluss (Phase 3 ASSY-06 — bereits Phase 3 verifiziert; Smoke-Test)
- [ ] Vorstand `/assemblies/{id}` → Tab "Anwesenheit"
- [ ] AttendanceList ist noch editierbar (`read_only=false` für Vorstand auch nach GV-Schluss)
- [ ] Toggle eines Mitglieds funktioniert auch nach GV-Schluss → 200 OK
- [ ] LiveCounter aktualisiert sich

---

## Acceptance Sign-Off

### Requirement-Coverage
- [ ] **HLPR-03 acceptance:** Block B2 (Manual-Code-Path) **PASS**
- [ ] **SYNC-01 acceptance:** Block C1 + C3 **PASS**
- [ ] **ATTN-06 acceptance:** Block C4 (visueller Diff) **PASS**

### ROADMAP Phase 4 Success Criteria
- [ ] **SC#1 (QR-Scan-Login):** Block B4 **PASS** (oder N/A wenn keine Camera-Hardware)
- [ ] **SC#2 (Manual-Code-Login):** Block B2 **PASS**
- [ ] **SC#3 (Live-Counter "X von Y"):** Block C1 **PASS**
- [ ] **SC#4 (Multi-Helfer-Refresh):** Block C3 **PASS**
- [ ] **SC#5 (Component-Reuse):** Block C4 **PASS**
- [ ] **SC#6 (No-Optimistic + ConnectionBanner):** Block C2 + C5 **PASS**

### Datenschutz / DSGVO
- [ ] **AttendanceList ohne PII:** Block D1 **PASS**
- [ ] **HelperShell ohne Vorstand-Navigation:** Block D2 **PASS**
- [ ] **Helfer-API-Coverage:** Block D3 **PASS**

### Build-Pipeline (vor Generalprobe)
- [ ] **wasm-bindgen-cli@0.2.104** verfügbar; `dx build --release` erfolgreich
- [ ] **Pitfall 6:** `qr-card`-Print-Rules in finaler `tailwind.css` vorhanden → Block A3 Print-Test bestätigt visuell

### Auth/401-Handling
- [ ] **Cascade-401:** Block E1 **PASS** (Helfer-Session wird beim GV-Schluss invalidiert)
- [ ] **401-Toast/Redirect:** Block C5 ODER E1 **PASS** (Backend-Down ODER Session-Invalid liefert klares User-Feedback)

---

## FAIL-Reporting

Falls einer der obigen Blöcke FAIL ergibt, hier dokumentieren und nach Phase-5-Plans/Issues eskalieren:

| Block-ID | Beschreibung | Schweregrad | Phase-5-Issue/Plan |
|----------|--------------|-------------|---------------------|
| z.B. FAIL-A3-Print | qr-card-Print-Rules fehlen → leere Print-Preview | hoch (Pitfall 6 nicht behoben) | bekannt — siehe 04-VERIFICATION.md Check 14 |
| z.B. FAIL-C4-Diff | AttendanceList-Row-Padding zwischen Helfer und Vorstand divergiert | mittel | neu eskalieren |
| z.B. FAIL-D1-PII | API liefert IBAN auf Helfer-Endpoint | **kritisch (DSGVO)** | sofort blockend |

---

**Tester:** ________________

**Datum:** ________________

**Final-Status:** [ ] PASS  /  [ ] FAIL (mit notierten Issues)

**Signatur:** ________________
