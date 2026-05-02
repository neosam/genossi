# Pitfalls Research

**Domain:** GV-Anwesenheits-Erfassung mit One-Time-QR-Helfer-Sessions, Live-Erstrollout in Genossenschafts-Generalversammlung
**Researched:** 2026-05-01
**Confidence:** HIGH (Mischung aus etablierten Web-/Event-Domain-Erfahrungen, Genossi-Codebase-Kenntnis aus CONCERNS.md, GV-Recht aus dejure/Haufe; alle kritischen Pitfalls mit mehreren Quellen verifiziert)

> **Lese-Hinweis für Roadmap-Planer:** Pitfalls sind nach Live-Event-Risiko sortiert, nicht nach technischer Tiefe. Wenn ein Cluster verbessert werden muss, dann zuerst die `LIVE-EVENT-RISIKO`-markierten — die sehen die Genossen direkt.

---

## Critical Pitfalls

### Pitfall 1: One-Time-QR-Token nicht atomar redeemed (Race Condition)

**LIVE-EVENT-RISIKO: NEIN (Backstage)** — aber Sicherheits-/Trust-Risiko, falls je ausgenutzt.

**What goes wrong:**
Zwei (oder mehr) Helfer scannen denselben QR-Code in derselben Sekunde — z. B. weil Vorstand Anna den QR ausgedruckt hat, sie ihn in der Hektik zwei Helfern hinhält, beide gleichzeitig scannen. Beide Requests laufen parallel: SELECT (`token_used IS NULL` → true), beide bestehen, beide UPDATEs. Ergebnis: zwei Helfer-Sessions auf demselben Token. Eine sollte gar nicht existieren. Wenn der Vorstand annimmt "nur ein Helfer pro QR", ist das eine Sicherheitslücke.

**Why it happens:**
Das naive Vorgehen "SELECT token, prüfe `used_at IS NULL`, dann UPDATE auf used" hat ein Time-of-Check-vs-Time-of-Use-Fenster. SQLite serialisiert zwar Schreibvorgänge, aber wenn die Logik im Service-Layer steht (nicht in einem einzigen UPDATE-Statement), können beide Requests die Lese-Phase parallel durchlaufen. In Genossi liegt die Validierung typischerweise im Service-Layer (siehe `genossi_service_impl/src/application.rs:498` für Optimistic-Locking-Pattern); dasselbe Anti-Pattern droht beim QR-Redeem.

**How to avoid:**
- Redeem in **einem einzigen atomaren UPDATE**: `UPDATE qr_token SET used_at = ?, session_id = ? WHERE id = ? AND used_at IS NULL RETURNING *;`
- Wenn `RETURNING` keine Zeile liefert, war der Token schon verbraucht → 409/410 zurückgeben.
- **Nicht** das Optimistic-Locking-Pattern (Version-UUID) für QR-Tokens verwenden — bei One-Time-Use ist `used_at IS NULL` der korrekte Constraint, nicht ein Versionsabgleich.
- Zusätzlich: UNIQUE-Index auf `qr_token.session_id` verhindert, dass je zwei Sessions am selben Token hängen können (Defense-in-Depth).
- Für Tests: Reproduziere den Race (zwei `tokio::spawn`s, die gleichzeitig redeemen) im E2E-Test — analog zum Optimistic-Locking-Test, der laut CONCERNS.md fehlt.

**Warning signs:**
- Code-Review findet `find_token_by_id` gefolgt von `update_token` in Service-Code → Refactor zu atomarem SQL.
- E2E-Test "100 parallele Redeems desselben Tokens" zeigt > 1 erfolgreiche Session.
- Logs zeigen mehrere Helfer-Sessions mit identischer `token_id`.

**Phase to address:**
Phase „QR-Token-Modell + Redeem-Endpoint" (vor Helfer-Login-UI). Atomarität ist ein **Phase-1-Pflicht-Test**, nicht später.

---

### Pitfall 2: WLAN bricht weg während GV — kein Recovery, peinliche Stille vor 80 Genossen

**LIVE-EVENT-RISIKO: JA — der peinliche Albtraum.**

**What goes wrong:**
Die GV läuft, 30 Genossen sind eingecheckt, plötzlich verliert das Vereinsheim-WLAN den Uplink (Standard-Failure-Mode bei Veranstaltungen — Event-WiFi „doesn't fail because providers are incompetent—it fails because they didn't plan for the inevitable"). Helfer-Tablets zeigen rotierende Spinner oder „Verbindung verloren". Der Vorstands-Live-Counter zeigt nichts mehr. Wenn der Server selbst noch läuft (er hängt im Vereinsheim-LAN), könnte intern noch alles ok sein — aber Helfer-Browser können nicht mehr synchronisieren. Wenn der Server **außerhalb** des Vereinsheims gehostet ist und das Internet weg ist, ist alles tot. Out-of-Scope ist „Offline-Modus", nicht „Recovery-Strategie".

**Why it happens:**
Der Stack ist „synchron HTTP, Refresh-only, kein Service-Worker, kein localStorage-Cache". Sobald das Netz weg ist, hat der Browser keine Daten mehr. Die Architektur-Entscheidung „kein Offline-Modus" ist legitim, aber **kein Recovery-Plan** ist es nicht.

**How to avoid:**
- **Decision schon vor Phase 1:** Wo läuft der Server am GV-Tag? (a) Im Vereinsheim auf Laptop = LAN reicht, externes Internet egal; (b) Auf Hetzner/Cloud = Internet ist Single Point of Failure → Mobile-Hotspot-Backup auf einem Vorstandshandy als WLAN-Bridge muss vorbereitet sein.
- **Manueller Backup-Plan dokumentiert:** Vorstand druckt **VOR** der GV eine vollständige Mitgliederliste mit Mitgliedsnummer + Name + freie Spalte „anwesend ✓". Bei Komplettausfall: Papier weiter, später nachpflegen via Bulk-Import-Endpoint (oder Excel-Import — den gibt's bereits).
- **Read-Only-Erkennung im Frontend:** Wenn Refresh fehlschlägt → klare UI-Banner "Verbindung verloren. Letzte Aktualisierung HH:MM. Markierungen werden NICHT mehr gespeichert." statt stiller Spinner. Verhindert „phantom checkmarks" in der UI ohne DB-Persistenz.
- **Optimistic-UI vermeiden** für diesen Milestone: Anwesend-Markierung darf erst gefärbt werden, **nachdem** die API 200 zurückgibt — sonst sieht der Helfer „ist abgehakt", aber serverseitig fehlt es.

**Warning signs:**
- Pre-Event-Test im Vereinsheim wurde nicht gemacht (Wifi/LAN ungetestet) → Stop, das ist Pflicht.
- Es gibt keine gedruckte Backup-Liste am GV-Tag.
- Frontend zeigt bei Netzwerkfehlern keinen Banner, sondern nur einen Spinner.

**Phase to address:**
Phase „Operations-Plan / Pre-Event-Generalprobe" — separater Phasen-Schritt, nicht nur Code. Plus „Frontend Connection-State Banner-Komponente" (component-first laut CLAUDE.md) in der UI-Phase.

---

### Pitfall 3: Helfer-Session geht durch Tab-Reload / Browser-Crash verloren — kein Re-Login möglich

**LIVE-EVENT-RISIKO: JA — sehr realistisch im Live-Betrieb.**

**What goes wrong:**
Helfer Bernd hat sich um 18:00 mit seinem QR-Code eingeloggt. Um 18:45 reloaded sein Tablet (Akku, iOS-Memory-Pressure killt Safari-Tabs aggressiv, versehentlich Browser geschlossen). Sein QR-Code ist bereits **verbraucht** (One-Time-Use!). Wenn die Session in einem Session-Cookie ohne `Persistent` lebte, ist sie weg. Bernd kann sich nicht mehr einloggen, sein QR ist Müll. Lösung: neuer QR-Code drucken? Vorstand hat das Drucksystem nicht dabei. Bernd ist ausgesperrt.

**Why it happens:**
Konflikt zwischen zwei Anforderungen: (1) One-Time-Use für QR (Sicherheit) und (2) Session muss Tab-Reload überleben (Usability). Standard-Webtech: Session-Cookies sterben bei Browser-Close, localStorage überlebt — aber wenn die Session-ID im Cookie ist, hilft localStorage nicht. Mobile Browser (iOS Safari) [killen Tabs aggressiv bei Memory-Druck](https://abp.io/support/questions/8885/Session-Lost-on-Mobile-Browsers-iOSAndroid-After-Closing-Tab) und [verlieren Session-Cookies beim Schließen](https://learn.microsoft.com/en-us/answers/questions/1165940/auth-cookie-is-deleted-by-the-browser-when-it-clos).

**How to avoid:**
Drei Optionen, jede mit Tradeoff:
- **Option A (empfohlen): Persistent Session-Token**
  Beim QR-Redeem wird ein langlebiges Helfer-Session-Token (UUID) erzeugt, im Backend an die GV gebunden, im Frontend in `localStorage` gespeichert (nicht nur Cookie). Reload → Frontend liest Token aus `localStorage`, sendet als Bearer/Header. Token ist gültig bis `Assembly.closed_at`. **One-Time-Use bleibt erhalten** (der QR ist trotzdem verbraucht, aber das Session-Token überlebt den Tab-Crash).
- **Option B: Re-Authentifizierungs-Token im URL**
  Nach Redeem leitet das Backend auf eine URL mit `?session=<token>` um. Helfer kann diese URL bookmarken oder nach Reload erneut öffnen. Vorteil: kein localStorage nötig. Nachteil: Token in URL → Browser-History → Schulter-Surfen.
- **Option C: Vorstand kann manuell eine neue Helfer-Session ausstellen**
  Recovery-UI: Vorstand klickt „Helfer Bernd hat seine Session verloren → neue Session ausstellen", Backend erzeugt neuen QR oder direkten Login-Link. Ohne neuen Druck.

Empfehlung: **Option A + Option C als Fallback**. Nicht nur Cookies.

**Warning signs:**
- Session-Lebenszeit-Strategie nicht in Phase-1-Doku festgelegt → Risiko, dass Default-Cookie-Verhalten sich durchsetzt.
- Manueller Test fehlt: „Helfer scannt, Tablet macht Hard-Refresh — kommt er wieder rein?" — muss vor Live-Tag bestätigt sein.
- Code-Review: `tower_sessions` mit `Expiry::OnSessionEnd` (= Browser-Close) statt `Expiry::AtDateTime(assembly.closed_at)`.

**Phase to address:**
Phase „Helfer-Session-Lebenszyklus + Frontend-Auth-Persistenz" — getrennt von QR-Generierung, weil der Lifecycle eigene Tests braucht.

---

### Pitfall 4: Helfer-View leakt versehentlich PII über reduzierten Datenbestand hinaus

**LIVE-EVENT-RISIKO: NEIN (technisch nicht sichtbar)**, aber **schwerwiegend datenschutzrechtlich** — kann nach GV zur Beschwerde / Datenschutz-Audit führen.

**What goes wrong:**
Frontend zeigt nur Mitgliedsnummer/Name/Titel/Anrede in der Liste — wie in den Constraints festgelegt. Aber: das Backend liefert über `/api/member` die volle `MemberTO` (Bankdaten, IBAN, Adresse, Geburtsdatum). Helfer-Frontend filtert nur visuell. Über Browser-DevTools → Network-Tab kann ein neugieriger Helfer die kompletten Daten lesen. CONCERNS.md M3 listet das explizit: *„All authenticated users with `manage_members` permission can see all member fields including bank account information. No field-level access control exists."*

Zweiter Vektor: Helfer öffnet die Detail-Ansicht (falls Suche zur Detail-Page führt) statt der reduzierten Liste — und sieht alles.

Dritter Vektor: Suchfeld leitet die Query an einen generischen `/api/member?search=...`-Endpoint, der vollständige Member-Records zurückgibt.

**Why it happens:**
- „Filterung im Frontend" ist die schnellste Implementierung.
- Bestehende REST-Endpoints liefern volle PII-Records; den Helfer-View dranflanschen ist verlockend.
- Genossi hat **kein** Field-Level-Access-Control (CONCERNS.md M3 — known finding).
- Permission-System ist binär (`manage_members` ja/nein), keine zweite Stufe für „limitierte Helfer-Sicht".

**How to avoid:**
- **Eigener Endpoint** `/api/assembly/{id}/members` mit eigenem DTO `AttendanceMemberTO { member_id, member_number, name, title, salutation, attended }` — **nicht** `MemberTO` mit `#[serde(skip)]`-Maske. Skip kann durch Refactoring kaputt gehen; getrenntes Struct ist explizit.
- **Helfer-Permission als eigene Permission** (`assembly_helper`), nicht `manage_members`. Endpoint validiert: nur diese Permission darf den Endpoint sehen.
- **Keine Detail-Routes für Helfer.** Helfer-Frontend hat keinen Link zur regulären Member-Detail-Seite. Frontend-Routing muss das aktiv ausschließen.
- **Audit dieses Endpoints lokal:** Test verifiziert, dass `serde_json::to_value(response)` keine `iban`, `bank_account`, `email`, `address`, `birthday` enthält — als Regression-Schutz.
- **Bezug zu CONCERNS.md M3:** Das ist die Gelegenheit, eine zweite Permission-Stufe einzuführen, die später für andere Feature-Slices wiederverwendbar wird. Nicht als Workaround behandeln.

**Warning signs:**
- PR-Diff zeigt: Helfer-Endpoint nutzt bestehendes `MemberTO`-Struct → Stop.
- Frontend-Code filtert Felder visuell, aber API liefert sie → Stop.
- Es gibt keinen Test, der prüft, was im Helfer-Response-JSON drin ist (nur was im UI sichtbar ist).

**Phase to address:**
Phase „Helfer-View Backend (read-only Endpoints)" — bevor das Frontend gebaut wird. Der getrennte DTO ist Phase-Pflicht.

---

### Pitfall 5: Quorum-Counter zeigt missverständliche Zahlen

**LIVE-EVENT-RISIKO: JA — Vorstand zeigt Zahl auf Beamer, Genossen fragen, Vorstand stottert.**

**What goes wrong:**
Live-Counter zeigt „32 von 87 anwesend". Vorstand schaut, denkt „87 = unsere Mitglieder", verkündet „wir sind beschlussfähig" oder „leider nicht". Aber:
- Y = Alle Member ohne `deleted IS NOT NULL`? Inklusive ausgetretener?
- Y = Nur stimmberechtigte? Die Anwendung trackt Stimmrechte **nicht** (Out of Scope). Also kann Y nicht „stimmberechtigte" sein.
- Y = Member, die zur GV eingeladen wurden? Diese Zuordnung gibt's nicht.
- Beschlussfähigkeit braucht laut Genossenschaftsgesetz/Satzung definierte Quoren — die Software berechnet sie nicht und sollte nicht so tun, als ob.

Wenn der Vorstand in der GV den Counter falsch interpretiert und auf der Basis Beschlüsse fasst, kann der Verband das im Protokoll-Review beanstanden.

**Why it happens:**
„Counter X von Y" wirkt intuitiv vollständig. Entwickler nimmt naheliegende Y-Definition ohne Rücksprache. Vorstand interpretiert die Zahl mit eigener Vorerwartung. Beschlussfähigkeit ist ein **rechtliches** Konzept (Satzung, GenG), kein technisches.

**How to avoid:**
- **Y im UI explizit beschriften.** Statt „32 von 87" → „32 von 87 aktiven Mitgliedern (ohne ausgetretene)". Der Helper-Text steht **immer** dran, nicht erst im Tooltip.
- **Klarstellen, dass die Zahl KEINE Beschlussfähigkeit darstellt.** Footnote: „Beschlussfähigkeit gemäß Satzung wird vom Versammlungsleiter festgestellt."
- **In PROJECT.md / Frontend-Doku festhalten,** wie Y berechnet wird — damit Vorstand vorher Bescheid weiß, nicht beim Live-Einsatz.
- **Vor erstem Live-Einsatz: User-Test mit dem Vorstand** — „Was glauben Sie zeigt diese Zahl?" — wenn Antwort „Beschlussfähigkeit", dann ist die Beschriftung kaputt.
- Bezug zu Out-of-Scope: Stimmrechte sind ausdrücklich kein Scope, also darf der Counter sich nicht so verhalten, als wären sie es.

**Warning signs:**
- UI-Mock zeigt nur „X / Y" ohne Y-Label.
- Vorstand fragt im Review „Heißt Y, dass wir beschlussfähig sind, wenn 50 % davon da sind?" → Beschriftung ist unklar.
- Code: `let y = member_dao.count_active(); let counter = format!("{}/{}", x, y);` ohne Kontext.

**Phase to address:**
Phase „Live-Counter UI + Vorstands-Review-Mock" — Beschriftung muss vor Code-Implementierung mit Vorstand abgestimmt sein.

---

### Pitfall 6: Doppel-Abhaken-Fehlerton irritiert Helfer und stört GV-Atmosphäre

**LIVE-EVENT-RISIKO: JA — niedriger Schmerz, aber sichtbar.**

**What goes wrong:**
Helfer Anna markiert Frau Müller anwesend (200 OK). Anna scrollt weiter. Helfer Bernd, der parallel arbeitet (kein Live-Sync laut Out-of-Scope), sucht ebenfalls nach Müller (steht noch in seinem alten Listenstand auf „nicht anwesend"), klickt sie an. Backend bekommt zweiten POST. Wenn der Endpoint nicht idempotent ist → 409 Conflict, Frontend zeigt rote Fehlermeldung „Konflikt", Bernd ist verwirrt, fragt Anna laut quer durch den Saal — soziale Reibung.

**Why it happens:**
- Naive Implementierung: `INSERT INTO attendance ...` mit UNIQUE-Constraint → zweite Insertion crasht.
- Optimistic-Locking mit Version-UUID übertragen → Versionsabgleich schlägt fehl.
- Out-of-Scope sagt explizit „kein Doppel-Abhaken-Schutz erforderlich" — aber das heißt nicht „Doppel-Abhaken soll Fehler werfen", sondern „Doppel-Abhaken ist ok, weil idempotent".

**How to avoid:**
- **Idempotenter PUT/UPSERT statt POST/INSERT.** `PUT /api/assembly/{aid}/attendance/{mid}` mit Body `{"present": true}` — wenn schon present, no-op, 200 OK.
- **State-Machine, nicht Event-Stream.** Anwesenheit ist ein Zustand (boolean + Timestamp), nicht eine Sequenz von „check-in"-Events.
- **Frontend zeigt visuell deutlich** „bereits anwesend" — z. B. grünes Häkchen + grauer Text, nicht klickbar (oder klickbar = austragen, aber dann mit Bestätigung).
- Bei Refresh: vor jeder Mark-Aktion einmal den aktuellen Zustand laden, dann die Aktion senden.
- **Idempotenz-Test:** E2E-Test mit 5x demselben PUT — alle 200, Datenbankzustand identisch nach jedem.

**Warning signs:**
- Endpoint-Design ist `POST /attendance` mit body `{member_id, action: "check_in"}` → Event-Stream-Pattern, nicht idempotent.
- Tabelle hat `attendance_event_log` mit Audit-ähnlicher Struktur statt `attendance` als State-Tabelle.
- 409-Codes im Test-Output bei „Mehrfach-Markierung-Test".

**Phase to address:**
Phase „Attendance-Endpoints + Datenmodell" — idempotenter Designentscheid muss vor erster Implementierung stehen.

---

### Pitfall 7: Audit-Verzicht für Anwesenheit unterläuft Verbandsprotokoll-Anforderungen

**LIVE-EVENT-RISIKO: NEIN**, aber **Verbandsprüfung-Risiko nach GV** — und der Audit-Verzicht ist explizit eine User-Decision; wir müssen prüfen, ob er hält.

**What goes wrong:**
Vorstand entscheidet (laut PROJECT.md Key-Decision): Anwesenheit braucht keinen Audit-Hashchain-Eintrag. Ok für „Anhakeln". **Aber:**
- Das **GV-Ergebnis selbst** (Anzahl Anwesende, Liste der Anwesenden) muss laut [GenG § 47 / Haufe-Protokollanleitung](https://www.haufe.de/id/beitrag/generalversammlung-einer-wohnungsbau-eg-formen-nach-neu-32-protokollierung-der-generalversammlung-HI15517552.html) ins Protokoll und ist verbandsprüfungs-relevant.
- Das **Schließen der GV** (`Assembly.closed_at`-Setzen) ist ein Vorstands-Handlung mit Konsequenz (alle Helfer-Sessions invalidiert + Liste eingefroren) — sollte auditiert sein.
- Das **Anlegen einer Assembly** durch den Vorstand ist eine Lifecycle-Aktion mit Auswirkung — sollte auditiert sein.
- Wenn nur „Anhakeln" nicht audited ist, aber Assembly-Lifecycle (create/close) **schon**, dann ist die Entscheidung in sich konsistent. Wenn beides nicht audited ist, kann der Verband fragen: „Wer hat die GV geschlossen, wann?".

**Why it happens:**
„Audit ausgeschlossen" wird breit interpretiert als „kein Audit für gar nichts in dem Feature". Tatsächlich gemeint ist „kein Audit pro Anwesenheits-Markierung".

**How to avoid:**
- **Klar trennen:** 
  - `Assembly.create` → audited (`audited_create!`).
  - `Assembly.close` → audited (`audited_update!`).
  - `AssemblyAttendance.set_present` / `set_absent` → **nicht** audited.
  - `QrToken.create` (Vorstand erstellt Helfer-QRs) → audited.
  - `QrToken.redeem` (Helfer scannt) → **nicht** audited (sonst Audit-Spam).
- **Anwesenheits-Endbestand persistent** in `AssemblyAttendance`-Tabelle, ohne Soft-Delete-Flag-Wechsel — Datenbestand selbst ist „der Beweis".
- **Protokoll-Export-Endpoint** `/api/assembly/{id}/protocol-export` liefert PDF/CSV mit den Anwesenden inkl. Timestamp — das ist der Verbandsbeweis, ersetzt einzelne Audit-Entries.
- **CLAUDE.md-Vorgabe einhalten:** Bestehende auditierte Entitäten (Member, MemberAction, MemberDocument, Application) müssen weiterhin Audit-Macros verwenden — der Helfer-Code darf diese Macros nicht umgehen, falls er nebenbei auf Member-Daten schreibt (sollte er aber sowieso nicht).

**Warning signs:**
- Code-Review: `Assembly`-Entity hat keine `Auditable`-Implementierung → diese **muss** aber ran (für create/close).
- Es gibt keinen Protokoll-Export-Endpoint → Vorstand kann am GV-Ende keinen Beweis erzeugen.
- Anwesenheits-Tabelle wird via `DELETE` bereinigt statt persistiert → Datenverlust nach GV.

**Phase to address:**
Phase „Assembly-Lifecycle + Audit-Integration" (für create/close) und **eigene Phase „Protokoll-Export"** (für Verbandskonformität). Beide nicht in „Helfer-UI" mischen.

---

### Pitfall 8: Camera-Access-Permission stolpert beim Helfer-Login (iOS Safari)

**LIVE-EVENT-RISIKO: JA — Helfer kommt nicht rein, Vorstand muss auf manuelle Mitgliedsnummer-Eingabe ausweichen.**

**What goes wrong:**
Helfer öffnet die Helfer-Login-Seite auf seinem iPhone Safari, klickt „QR scannen". Browser will Kamera-Permission. Aber: 
- Wenn die Seite per `http://` (nicht `https://`) ausgeliefert wird → `getUserMedia` ist **undefined**, kein Prompt, keine Kamera. [HTTPS ist Pflicht](https://copyprogramming.com/howto/navigator-mediadevices-getusermedia-not-working-on-ios-12-safari).
- Wenn Genossi als PWA mit `display: standalone` läuft (aktuell unklar) → Safari [zeigt keinen Permission-Prompt im Standalone-Mode](https://bugs.webkit.org/show_bug.cgi?id=185448) bis iOS 14.x.
- Wenn `<video>`-Element kein `playsinline`-Attribut hat → Stream startet nicht.
- Wenn der Permission-Prompt auf einer Vorgänger-Seite ohne User-Geste angefragt wurde → Browser unterdrückt ihn.
- Repeated-Prompt-Bug: bei jedem SPA-Routenwechsel fragt Safari neu — Helfer ist genervt.

**Why it happens:**
- Browser-Spec sagt: `getUserMedia` braucht Secure Context **und** User-Geste.
- Dioxus-WASM-Frontend ruft typischerweise via `web_sys::window().navigator().media_devices()` an — wenn `media_devices()` `undefined` returned, gibt's einen `JsValue`-Fehler, der gerne `.unwrap()` wird (siehe `genossi-frontend/src/api.rs` Style — `window().unwrap().location().origin().unwrap()` laut CONCERNS.md).
- Niemand testet auf iOS, weil Entwickler Linux/Chrome haben.

**How to avoid:**
- **Server am GV-Tag MUSS HTTPS ausliefern.** Wenn Server lokal im Vereinsheim läuft → self-signed-cert + Vorstand trägt Cert auf Helfer-Geräten ein, oder mDNS-Local-Name + Caddy-mit-internal-CA, oder per ngrok/Cloudflare Tunnel mit echtem TLS. **Im Plan dokumentieren, vor GV testen.**
- **Manuelle Eingabe als Fallback**: Helfer-Login-Seite hat zwei Optionen: „QR scannen" oder „Code manuell eintippen" (kurzer 6-stelliger Token statt langer UUID). Wenn Kamera versagt, ist der Helfer nicht ausgesperrt.
- **Kein PWA-Standalone-Mode** für die Helfer-Seite — bleibt im Browser-Tab, dort funktioniert Kamera-Permission verlässlich.
- **`playsinline` setzen, wenn Video-Element verwendet** (`koder`, `wascan`, `rqrr-wasm` o. Ä.).
- **Permission-Request nur nach User-Klick** auf „Scannen starten"-Button, nicht beim Page-Load.
- **Fehler-UX:** Wenn `getUserMedia` failed → klare Meldung „Kamera nicht verfügbar. Bitte Code manuell eintippen." statt Spinner-Tod.
- **Cross-Device-Test im Plan:** Vor GV testen mit (a) iOS Safari, (b) Android Chrome, (c) iPad Safari.

**Warning signs:**
- HTTPS-Setup nicht in Pre-GV-Checkliste.
- Frontend-Code hat kein Try/Catch um `getUserMedia` — fällt durch zu Panic.
- Es gibt keinen Manual-Code-Fallback im UI-Mock.

**Phase to address:**
Phase „QR-Scanner-Komponente (Frontend)" — und **eigene Phase „GV-Tag-Operations-Plan"** für HTTPS-Setup, Pre-Event-Test, Backup-Eingabe.

---

### Pitfall 9: Live-Demo / erste Inbetriebnahme ohne Generalprobe

**LIVE-EVENT-RISIKO: JA — meta-Pitfall, der alle anderen verstärkt.**

**What goes wrong:**
„Auf meinem Laptop funktioniert's, deploy 30 Minuten vor GV-Start, was soll schon schiefgehen?" — und dann: WLAN unbekannt, HTTPS-Cert-Fehler, Helfer-Tablets nie getestet, Tokens nicht gedruckt, Vorstand kennt das UI nicht, niemand weiß was zu tun ist wenn ein Helfer fragt. Plus alle einzelnen Pitfalls 1-8 schlagen jetzt gleichzeitig zu.

> *„In demos, external services work perfectly, but in production, APIs slow down, tokens expire, networks drop—causing retry storms, timeouts, and partial failures."* — typischer Demo-vs-Prod-Bias.

**Why it happens:**
- Genossi ist bisher Backend-mit-Web-UI ohne Live-Event-Charakter; die Vorstand-Workflow war zeitunkritisch. GV ist die erste Eventisierung.
- Tests laufen mit `localhost`, e2e-Tests mit in-memory SQLite — alles ohne Netzlatenz, ohne reale Geräte, ohne reales Drucken.
- Kein Staging-Environment dokumentiert.

**How to avoid:**
- **Pflicht-Generalprobe mindestens 1 Woche vor GV.** Im echten Vereinsheim oder vergleichbarer Umgebung, mit echtem Tablet, echtem Drucker, echten ausgedruckten QRs, mindestens 3 Test-„Helfern", mindestens 10 Test-Mitgliedern in der DB.
- **Pre-Event-Checkliste in `.planning/`** — separate Datei, z. B. `OPERATIONS.md`, mit:
  - [ ] HTTPS funktioniert von außen?
  - [ ] WLAN/LAN getestet mit allen Helfer-Geräten?
  - [ ] Drucker-Backup für Mitgliederliste?
  - [ ] Mobile-Hotspot-Backup vorbereitet?
  - [ ] QR-Codes gedruckt + gescannt + verifiziert (nicht nur generiert)?
  - [ ] Vorstand kennt: Helfer-View, Counter, GV-Schließen, Protokoll-Export?
  - [ ] Helfer wissen: was tun bei Tablet-Crash, was tun bei nicht-gefundenem Mitglied, was tun bei Internet-Aussetzer?
- **Rollback-Plan:** Falls GV vor Beschlüssen feststellt „System unbenutzbar" → Papierliste aus dem Schrank, GV läuft analog weiter. Keine GV-Pause nötig.
- **Demo-Mode:** Eigener Endpoint / Konfiguration, der eine Test-Assembly erzeugt mit synthetischen Mitgliedern — für Generalprobe und Vorstand-Schulung, ohne echte Daten zu nutzen.

**Warning signs:**
- Es gibt keinen Generalproben-Termin.
- Keine `OPERATIONS.md` oder Äquivalent.
- Niemand außer dem Entwickler hat das System je benutzt.

**Phase to address:**
**Eigene Phase „Pre-GV-Generalprobe + Operations-Plan"** — getrennt von Code-Phasen, eigener Erfolgskriterien, eigener Schritt im Roadmap. Höchste Priorität-Phase nach Code-Komplettheit.

---

### Pitfall 10: Bestehender Audit-Pipeline-Bruch durch Assembly-Code-Querverbindungen

**LIVE-EVENT-RISIKO: NEIN**, aber **Codebase-Risiko** — kann Member/Application-Audit nach GV-Code-Merge brechen.

**What goes wrong:**
Helfer-Endpoint braucht Member-Daten. Entwickler ergänzt Member-DAO um eine neue Methode `find_by_id_minimal()` für die reduzierte Helfer-Sicht. Bei der Gelegenheit räumt er „aus Versehen" das Audit-Macro-Wrapping um, weil's „verwirrend" ist. Oder neuer Endpoint umgeht die `audited_update!`-Macro, weil er „nur lesend" ist — aber im Code-Pfad triggert er stillschweigend einen Member-Update (z. B. `last_active_at`-Feld). Audit-Hash-Chain bricht oder wird unvollständig.

CONCERNS.md weist explizit auf die Fragilität der Audit-Hash-Chain hin und auf den Mangel an Tests dafür.

**Why it happens:**
- Audit-Macros (`audited_create!`, `audited_update!`, `audited_delete!`) sind ein Konvention, nicht eine Sprach-Konstrukt-Erzwingung. Compiler erlaubt direkten DAO-Aufruf.
- Neue Entitäten (Assembly, AssemblyAttendance, QrToken) sind explizit **nicht** auditiert — der Mental-Switch „kein Audit hier" könnte versehentlich auf bestehende Entitäten überspringen.
- CLAUDE.md sagt klar: *„Bestehende auditierte Entitäten (Member, MemberAction, MemberDocument, Application) müssen weiterhin Audit-Macros verwenden"* — aber das wird nur in Code-Reviews durchgesetzt.

**How to avoid:**
- **Keine Member/Application-Mutationen aus dem GV-Feature.** Helfer-Endpoint ist ausschließlich Read auf Member; schreibende Aktion geht nur auf neue Tabellen.
- **PR-Review-Checkliste**: bei GV-Phase-PRs: hat sich an `genossi_service_impl/src/member.rs`, `application.rs`, `member_document.rs`, `member_action.rs` etwas geändert? Wenn ja, Audit-Macro-Verwendung bestätigen.
- **E2E-Test schreiben** (fehlt laut CONCERNS.md sowieso): „Member-Update via REST → Audit-Verify-Endpoint zeigt Hash-Chain-OK". Diesen Test in CI laufen lassen, schlägt aus, wenn Audit-Pipeline bricht.
- **`/api/audit/verify` regelmäßig in CI** ausführen gegen frisch-migrierte DB mit Test-Daten.
- **Aufräumen-Phase NACH dem Live-Einsatz:** Wenn CONCERNS.md M3 (Field-Level-Access) während GV-Phase angefasst wurde, klar trennen welcher Commit was tat.

**Warning signs:**
- PR-Diff zeigt Änderungen an `member.rs`-Service ohne Begründung.
- CI-Test für Audit-Hash-Chain fehlt — also kein Schutz.
- Helfer-Code importiert direkt aus `genossi_dao`-Crate, ohne über Service-Layer zu gehen → Audit-Bypass.

**Phase to address:**
Phase „CI-Hardening: Audit-Verify-Test" — sollte VOR den GV-Code-Phasen stehen, sodass jeder GV-PR gegen den Test grünt. Plus Phase-übergreifende PR-Checkliste.

---

### Pitfall 11: QR-Code-Verbreitung / -Verlust ist breiter als „weitergegeben"

**LIVE-EVENT-RISIKO: NIEDRIG (aber: One-Time-Use mitigiert es schon teilweise).**

**What goes wrong:**
Vorstand druckt 8 Helfer-QR-Codes. Bei der GV-Vorbereitung:
- Anna verlegt ihren Ausdruck — landet später im Müll, theoretisch könnte jemand ihn fotografieren.
- Bernd fotografiert seinen QR mit dem eigenen Handy und scannt vom Foto — das ist ok, aber er teilt das Foto in der Helfer-WhatsApp-Gruppe, weil „sicherheitshalber".
- Carl scannt seinen QR an einem Test-Tablet, das nicht an der GV beteiligt ist → QR ist verbraucht für die echte GV.
- Drucker spuckt zweimal aus, ein Ausdruck bleibt im Tray, Putzpersonal findet ihn.

One-Time-Use mitigiert das **nach erstem Scan**, aber nicht **vor erstem Scan**.

**Why it happens:**
Papier-QRs sind nicht wie Passwörter „vor Augen schützen", sondern wandern. Erwartung „Vorstand passt auf" hält selten.

**How to avoid:**
- **Pre-Activation-Window:** QR-Tokens sind nur in einem Zeitfenster gültig (z. B. nur am GV-Tag, ab 2 Stunden vor offiziellem Beginn). Vorher gescannte Tokens verbrauchen sich nicht, sie sind „noch nicht aktiv".
- **Sichtbarer Helfer-Memo-Text auf dem Ausdruck.** Wenn jemand „QR für Anna" findet, erkennt der Finder, das gehört Anna; Anna kann beim Vorstand melden „mein QR ist weg, neuen ausstellen".
- **QR-Revoke-Endpoint:** Vorstand kann einen einzelnen QR-Token vor Aktivierung invalidieren („Anna sagt, sie hat ihren verloren"). Nicht erst nach Verbrauch reagierbar.
- **Kein Foto-Versand sinnvoll:** Helfer-Onboarding ist explizit physisch (Vorstand übergibt QR persönlich). Im Helfer-Briefing erklären „bitte nicht fotografieren / weitersenden". Soziale Mitigation.
- **Drucker-Routine:** „Print → Fold → Distribute" als Vorgang; keine offen rumliegenden Ausdrucke.

**Warning signs:**
- QR-Tokens sind 30 Tage gültig statt 1 Tag.
- Kein Revoke-Endpoint im API-Plan.
- Helfer-Onboarding ist nicht im Operations-Plan beschrieben.

**Phase to address:**
Phase „QR-Token-Modell" für Pre-Activation/TTL und Revoke-Endpoint. Phase „Pre-GV-Operations-Plan" für die soziale Komponente.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `MemberTO` mit `#[serde(skip)]`-Maske statt eigenem `AttendanceMemberTO` | Schnell, 5 Zeilen Code | Refactor an Member fügt versehentlich PII-Feld ohne Skip ein → Helfer sieht es; M3 (CONCERNS.md) bleibt ungelöst | **Nie**: ist die DSGVO-Hauptcheckliste. |
| QR-Token als „use Member.id direkt als Login" statt eigene Token-Entität | Eine Tabelle weniger | Permission-System verkrampft, Lebenszyklus nicht trennbar, kein Revoke möglich | **Nie**: das ist die Sicherheitsarchitektur. |
| Optimistic-Locking-Pattern (Version-UUID) für QR-Redeem statt atomarem `WHERE used_at IS NULL` | Konsistenz mit bestehenden Entitäten | Race-Condition (Pitfall 1) bleibt offen | **Nie** für One-Time-Use-Tokens. |
| Cookie-Session ohne `Persistent`/`AtDateTime` für Helfer | Default `tower_sessions`-Verhalten | Helfer fliegt bei Tablet-Reload raus (Pitfall 3) | **Nie** für GV-Helfer; **OK** für Vorstands-Web-Login. |
| Counter-UI nur „X / Y" ohne Y-Beschriftung | Zwei Wörter sparen | Vorstand interpretiert falsch, GV-Pannen (Pitfall 5) | **Nie** vor Live-Einsatz; **OK** in einem Admin-Debug-Panel. |
| Direct DAO-Aufruf in Helfer-Endpoint statt Service-Layer | Einfacher Code-Pfad | Audit-Bypass, Permission-Check umgangen, CLAUDE.md-Architektur-Regel verletzt | **Nie**: layered architecture. |
| `unsafe impl Send/Sync` für neue GV-Service-Structs (analog zu CONCERNS.md N1) | Trait-Bound-Konflikte umgehen | Tech-Debt vergrößert sich, neue Stellen für die spätere Migration | **Nur** wenn als Tech-Debt-Item in CONCERNS.md aufgenommen. |
| Audit-Macro-Wrapping bei Member „aus Versehen" weglassen, weil GV-Code „nichts Wichtiges" schreibt | Etwas weniger Boilerplate | Audit-Hash-Chain bricht (Pitfall 10) | **Nie**: Audit-Macros sind Pflicht für auditierte Entitäten. |
| QR-Tokens unbegrenzt gültig | Vereinfacht Lifecycle | Pre-GV-Tokens werden Reststaub, Verbrauchszeitpunkt unklar | **Nie**: TTL ist Pflicht. |

---

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **OIDC (Nextcloud)** | Vorstand-Login-Flow nicht kompatibel mit Helfer-QR-Login (zwei Auth-Systeme parallel) | Vorstand bleibt in OIDC-Session; Helfer-Session ist eigener Pfad mit eigenem Cookie-Name (`assembly_helper_session`); beide koexistieren ohne Mix; Vorstand kann zusätzlich Helfer-View ohne QR sehen, indem das Vorstands-OIDC-Cookie als Helfer-Berechtigung gelesen wird. |
| **Browser `getUserMedia`** | Permission-Request beim Page-Load → Browser unterdrückt | Permission-Request nach User-Klick auf „Scannen starten"-Button. |
| **Browser `localStorage`** | Helfer-Session-Token in Cookie, das beim Browser-Close stirbt | Token in `localStorage` + zusätzlich Cookie für serverseitige Validation; bei Reload aus `localStorage` rekonstruieren. |
| **Drucker (Vorstands-Workflow)** | QR-PDF als A4-Vollbild → Drucker-Setting „Fit to page" verzerrt → QR unscannbar | Druck als Halbseite mit klar definierter QR-Größe (≥ 2x2 cm), dazu printer-friendly Schwarz-auf-Weiß, ausreichend Quiet Zone. |
| **SQLite Connection Pool** | Default-Pool-Konfig (CONCERNS.md): unter Helfer-Last keine explizite max_connections | Vor GV `max_connections=20` setzen; Live-Counter-Polling kann sonst Pool erschöpfen. |
| **Axum Body Limit** | Default 2 MB (CONCERNS.md HIGH-Bug) — Helfer-Endpoint nicht betroffen, aber Vorstand will am GV-Tag noch ein PDF hochladen → 413 | Bevor GV: `fix-upload-body-limit`-Proposal mergen, sonst Side-Quest am GV-Tag. |
| **Nextcloud-Export** (laufendes System) | Nach GV-Schluss exportiert irgendein Job die Mitgliederliste mit den neuen Anwesenheits-Daten zu Nextcloud → Vorstand sieht die Liste, aber jetzt enthält sie auch GV-Daten? | Klar definieren, ob Anwesenheits-Daten in Nextcloud-Export gehören. Wenn nein: Filter; wenn ja: Schema versionieren, sonst bricht Export-Pipeline. |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Helfer-Liste lädt **alle** Member auf einmal | Lange Initial-Latenz beim Helfer-Login auf Tablet | Pagination + Server-Side-Suche; nur Treffer rendern | Bei > 500 Mitgliedern auf Tablet-Browser mit WASM-Render-Overhead. Genossi-Skala ist heute eher 100-300, also nicht akut, aber für 1000+ relevant. |
| Live-Counter pollt jede Sekunde via `/api/assembly/{id}/counter` | DB-Read-Last + Tablet-Akku stirbt | Polling-Intervall 5-10 s; oder ETag/304-Antwort bei keiner Änderung; oder Counter-Wert clientseitig inkrementieren ohne Server-Roundtrip pro Mark | Mit 5+ Helfern parallel + 100 Mitgliedern, Polling 1s = 5+ req/s nur für Counter. |
| `dump_all()` für Helfer-Suche statt indizierter Query | Suche dauert Sekunden | SQL-LIKE mit Index auf `member_number`, `last_name`; oder FTS5 SQLite-Full-Text-Search-Tabelle | Bei > 200 Mitgliedern wird dump_all spürbar; bei 1000+ peinlich. |
| Audit-Verify (`/api/audit/verify`) auf der GV laufen lassen | Verify scant ganze Audit-Tabelle, blockt Pool | Verify nur außerhalb GV-Zeiten; oder paginierte Verify-Variante | Mit > 10000 Audit-Einträgen → Sekunden bis Minuten. |
| Soft-Delete-Filter (`WHERE deleted IS NULL`) ohne Index auf `deleted` | Liste-Endpoint langsam | Composite-Index `(deleted, ...)` oder Partial-Index `WHERE deleted IS NULL` | Bei > 1000 Mitgliedern mit vielen Soft-Deletes. CONCERNS.md erwähnt das implicit. |
| `unsafe impl Send/Sync` (CONCERNS.md N1) verbirgt echtes Sync-Problem | Race / Datarace die nur unter Last sichtbar | Tech-Debt-Cleanup vor neuer Service-Komposition; nicht neue `unsafe impl` für Assembly-Service hinzufügen | Schon heute latent; bei neuen GV-Services mit dem gleichen Pattern wird's mehr. |

---

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| QR-Token-ID ist sequenziell oder kurz (z. B. 6-stellig) | Brute-Force-Erraten von gültigen Tokens | UUID v4 (128 bit) als Token-ID; **oder** wenn Manual-Code-Fallback (Pitfall 8) gewünscht, dann zusätzlich kurze Codes mit Rate-Limit + TTL ≤ 1h |
| Helfer-Permission = `manage_members` recyceln | Helfer kann Member editieren, nicht nur lesen | Eigene Permission `assembly_helper`, scope-limited auf Read + Anwesenheit-Update |
| Helfer-Session-Token im URL-Pfad | Token in Browser-History, Server-Logs, Referer-Header | Token im Header (`Authorization`) oder Cookie, **nicht** im Pfad |
| Helfer-Endpoint nimmt `member_id` aus Frontend, ohne zu validieren, dass member zur Assembly gehört | Helfer kann theoretisch beliebige fremde Mitglieder als „anwesend" markieren | Validate: Member existiert, Member ist nicht soft-deleted, Member-Sichtbarkeitsregel okay. Auch bei Anwesenheit-Schreiben |
| QR-Code generiert Server-Side mit user-supplied Inhalten ohne Sanitization | XSS / Open-Redirect via QR-Inhalt | QR-Inhalt = nur Token-ID + Base-URL, beides server-controlled |
| HTTP statt HTTPS am GV-Tag | Helfer-Login via HTTP → Token im Klartext im LAN | HTTPS-Pflicht im Operations-Plan; Test vor GV |
| Pre-Activation-Window fehlt | QR-Codes können vor GV von Test-Lauf verbraucht werden | TTL + Activation-Window pro Token |
| Permission-Check für Helfer-View nur im Frontend | Direkter API-Call mit Vorstands-Cookie umgeht Helfer-Limits → sieht alles | Server-Side-Permission immer; Frontend-Filter ist nur UX, nicht Security |
| OIDC + Helfer-Session beide in Cookie-Jar → Verwechslung | Helfer-Cookie wird vom Vorstands-Browser mitgesendet, Permission-Mix | Cookie-Names disjunkt; Endpoints validieren genau einen Auth-Modus |
| Mitgliedsnummern in Helfer-Liste sind sequenziell und predictable | Helfer kann Mitgliedsnummer-Bereich komplett iterieren | OK in diesem Kontext (Helfer ist trusted), aber: Endpoint-Rate-Limit als Defense-in-Depth |

---

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| „Anwesend"-Toggle-Button ohne visuellen Bestätigungs-Zustand | Helfer klickt zwei-, dreimal weil unsicher → Doppelmarkierungen | Klarer Vor-/Nach-Zustand: grauer Kreis → grünes Häkchen mit Pulse-Animation; nach Server-OK |
| Suche ohne Auto-Fokus auf Suchfeld nach Page-Load | Helfer muss erst klicken, dann tippen — pro Mitglied 2x klicken | `autofocus` auf Suchfeld; nach Markierung Suchfeld leeren + zurückfokussieren |
| Suche case-sensitive oder ohne Umlaut-Normalisierung | „Müller" vs „mueller" findet nichts | Case-insensitive + Unicode-Normalisierung; suche in Mitgliedsnummer **und** Name parallel |
| Liste zeigt nur Mitgliedsnummer (1234), Helfer kennt nur Name | Helfer scrollt komplett, ineffizient | Name vollwertig anzeigen (Constraint erlaubt es) — Mitgliedsnummer als Sekundär-Info |
| Counter „X von Y" ohne Update bei Refresh | Vorstand fragt sich „warum stagniert der?" | Counter-Refresh sichtbar (Spinner / Timestamp „aktualisiert vor 3 s") |
| Kein „Letzte Markierung rückgängig"-Button | Helfer markiert falsche Person, weiß nicht wie korrigieren | „Anwesend"-Markierung anklickbar = austragen mit Bestätigung |
| GV-Schließen-Button leicht zu erreichen | Vorstand klickt versehentlich → Sessions weg, Helfer ausgesperrt | „GV schließen" + Bestätigungs-Modal mit Zähler („32 Anwesende werden eingefroren, Helfer-Sessions werden beendet. Wirklich schließen?") |
| Keine Anzeige „GV läuft" / „GV beendet" | Helfer verstehen nicht, warum sie ausgesperrt sind | Top-Banner mit GV-Status |
| Helfer-Name (Memo) nicht sichtbar im UI | Wenn Vorstand mehreren Helfern hilft, weiß er nicht wer angemeldet ist | Header zeigt „Eingeloggt als: Anna" |
| Fehler-Meldungen technisch („SQL constraint violation") | Helfer panisch, Vorstand muss weghelfen | Domänen-spezifische Meldungen: „Mitglied bereits abgehakt", „Verbindung verloren — bitte Seite neu laden" |
| Helfer-Liste sehr lange ohne Pagination/Scroll-to-Top | iPad-Helfer scrollt 500 Einträge auf Touch | Sticky Header + Suchfeld; Liste virtualisiert oder paginiert |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces. Verify each before declaring milestone done.

- [ ] **One-Time-QR-Redeem:** Tests umfassen **konkurrenten** Redeem (zwei parallele Requests), nicht nur sequenziellen. Atomares UPDATE im SQL, nicht SELECT-then-UPDATE im Service.
- [ ] **Helfer-Session-Persistenz:** Manueller Test „Tablet-Reload nach Login" wurde gemacht; Helfer ist immer noch eingeloggt. Auf realem mobilen Browser (nicht nur Desktop-Chrome).
- [ ] **HTTPS am GV-Tag:** Es gibt eine dokumentierte HTTPS-Setup-Anleitung im Operations-Plan, getestet im Vereinsheim, nicht erst am GV-Morgen.
- [ ] **Helfer-PII-Limitierung:** Test verifiziert Response-JSON-Inhalt (nicht nur UI-Sichtbarkeit). DTO ist eigenes Struct, nicht skip-maskiertes `MemberTO`.
- [ ] **Counter-Beschriftung:** Y wird im UI klar erläutert; Vorstand wurde befragt „Was glauben Sie zeigt diese Zahl?" und antwortet konsistent.
- [ ] **Idempotente Anwesenheits-Markierung:** E2E-Test mit 5x demselben Request → 5x 200 OK, DB-Zustand identisch.
- [ ] **Audit-Hash-Chain:** `/api/audit/verify` läuft grün am GV-Tag; CI-Test verifiziert das nach jedem GV-Code-Merge.
- [ ] **Camera-Permission auf iOS Safari:** Tatsächlich auf einem iPhone getestet (nicht nur DevTools-Mobile-Emulation).
- [ ] **Manuelle Code-Eingabe als Fallback:** Frontend bietet „Code eintippen" wenn Kamera nicht verfügbar; Backend akzeptiert beide Wege.
- [ ] **Pre-Activation-Window für QR:** Tokens sind nicht generierbar + sofort scanbar; gibt's TTL/Window?
- [ ] **QR-Revoke-Endpoint:** Vorstand kann einen einzelnen QR vor Verbrauch invalidieren.
- [ ] **Backup-Plan für Internet-Aussetzer:** Gedruckte Mitgliederliste + manueller Bulk-Import-Pfad nach GV existiert und ist getestet.
- [ ] **GV-Schließen-Bestätigung:** Vorstand kann nicht versehentlich klicken; Modal zeigt Konsequenzen.
- [ ] **Protokoll-Export-Endpoint:** GV-Ergebnisse können nach Schließen als PDF/CSV exportiert werden, mit Timestamp, signierbar.
- [ ] **Bestehende Audit-Pipeline intakt:** PR-Diff zeigt keine Änderungen an `member.rs`-Service-Audit; CI-Test bestätigt.
- [ ] **Generalprobe:** Ein vollständiger Trockenlauf hat im Vereinsheim mit echten Geräten und mindestens 3 Helfern stattgefunden. Mindestens eine Woche vor echter GV.
- [ ] **Vorstands-Schulung:** Vorstand kann selbständig: Assembly anlegen, QRs erzeugen, Counter sehen, Helfer-Memo zuordnen, GV schließen, Protokoll exportieren — ohne Entwickler-Hilfe.
- [ ] **Helfer-Briefing dokumentiert:** Es gibt schriftliche Anleitung „Was tun wenn Tablet abstürzt", „Was tun wenn Mitglied nicht in Liste", „Was tun wenn Internet weg".

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| WLAN ausgefallen mitten in GV | LOW (wenn vorbereitet), HIGH (wenn nicht) | (1) Helfer wechseln auf Mobile-Hotspot eines Vorstandshandys (vorher konfiguriert). (2) Falls auch das nicht: Papierliste rausholen, manuell weiter, später nachpflegen via Excel-Import (existiert in Genossi). (3) Vorstand kommuniziert „kurze Pause / weiter analog" — keine Panik. |
| Helfer-Session verloren nach Tablet-Reload | LOW | Wenn Persistent-Token implementiert: Reload reicht. Sonst: Vorstand stellt manuell neuen Helfer-QR aus (Recovery-UI), neuer Scan, weiter. |
| Doppel-Markierung führt zu 409 (statt idempotent zu sein) | LOW | Helfer ignoriert, Refresh, prüft ob anwesend → ja → fertig. Frontend zeigt explizit „bereits anwesend". |
| QR-Code verloren / weggeworfen | LOW | Vorstand stellt Revoke-Endpoint-Aufruf aus, druckt neuen QR; Helfer scannt frisch. Voraussetzung: Revoke-Endpoint existiert, Drucker da. |
| Camera-Permission auf iOS verweigert | LOW | Helfer wechselt auf Manual-Code-Eingabe. Voraussetzung: Manual-Fallback im UI. |
| PII-Leak an Helfer (z. B. via DevTools) | HIGH | Nach Entdeckung: GV-Schluss → Datenschutz-Beauftragter informieren → Helfer-Liste auf Vertraulichkeit verpflichtet → Code-Hotfix vor nächster GV. Kein Quick-Fix während laufender GV. |
| Audit-Hash-Chain bricht (durch GV-Code-Merge) | MEDIUM | (1) `/api/audit/verify` zeigt Bruchstelle. (2) Code-Bisect: welcher Commit hat Audit-Macro umgangen. (3) Hotfix + neue Audit-Migration, die Chain fortschreibt. (4) Verband informieren bei verbandskritischen Entitäten. |
| Vorstand schließt GV versehentlich vor Ende | MEDIUM | Wenn schließen audited ist: Vorgang sichtbar, Vorstand kann „GV wieder öffnen"-Endpoint nutzen (sollte existieren!). Helfer-Sessions müssen neu ausgestellt werden. Kommunikation an Helfer notwendig. |
| Counter zeigt falsche Y-Zahl, Beschluss auf falscher Basis gefasst | HIGH | Verband-Konsultation; Beschluss-Wiederholung in nächster GV; Protokoll-Korrektur. Verhindern (Pitfall 5)! |
| Live-Demo-Crash (Pitfall 9 manifestiert sich) | HIGH | Zurück auf Papier, GV-Pause minimieren, Demo-Ergebnis später nachpflegen, nächste GV mit getestetem System; Lessons-Learned-Doku. |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls. Phasen-Namen sind Vorschläge für die Roadmap; Reihenfolge orientiert an Abhängigkeit.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1 — One-Time-Token-Race | Phase „QR-Token-Modell + Redeem-Endpoint" | E2E-Test: 100 parallele Redeems → 1 erfolgreich |
| 2 — WLAN-Ausfall ohne Recovery | Phase „Operations-Plan / Pre-Event" + UI-Phase „Connection-State Banner" | Pre-Event-Checkliste abgehakt; Banner-Komponente existiert |
| 3 — Session-Verlust nach Reload | Phase „Helfer-Session-Lifecycle + Frontend-Auth-Persistenz" | Manueller iOS-Safari-Reload-Test besteht; Recovery-UI für Vorstand existiert |
| 4 — PII-Leak im Helfer-View | Phase „Helfer-View Backend (read-only Endpoints)" | Eigener DTO; Test prüft Response-JSON-Felder; eigene Permission `assembly_helper` |
| 5 — Counter-Y-Mehrdeutigkeit | Phase „Live-Counter UI + Vorstands-Mock-Review" | Vorstand-Test bestätigt korrekte Interpretation; Y-Beschriftung sichtbar |
| 6 — Doppel-Abhaken-Conflict | Phase „Attendance-Endpoints + Datenmodell" | E2E-Test 5x identischer PUT; State, nicht Event |
| 7 — Audit-Verzicht-Lücke | Phase „Assembly-Lifecycle (create/close) + Audit" + Phase „Protokoll-Export" | `Auditable` für Assembly; PDF/CSV-Export-Endpoint; Verband-Akzeptanz-Check |
| 8 — Camera-Permission-iOS | Phase „QR-Scanner-Komponente" + Phase „Operations-Plan" | iOS-Safari-Test besteht; Manual-Code-Fallback existiert; HTTPS-Setup-Doku |
| 9 — Live-Demo-Bias | **Eigene Phase „Pre-GV-Generalprobe"** | Generalprobe durchgeführt; Operations-Plan vollständig |
| 10 — Audit-Pipeline-Bruch | Phase „CI-Hardening: Audit-Verify-Test" (vor GV-Code-Phasen!) | CI-Test grün; PR-Checkliste eingeführt |
| 11 — QR-Verbreitung/-Verlust | Phase „QR-Token-Modell" (Pre-Activation, TTL, Revoke-Endpoint) | Tokens haben TTL; Revoke-Endpoint getestet |

**Roadmap-Empfehlung (Phase-Reihenfolge):**

1. **CI-Hardening: Audit-Verify-Test** (verhindert Pitfall 10 für alle nachfolgenden Phasen)
2. **Assembly + Auditable + DAO/Service-Skelett** (Lifecycle-Entitäten — adressiert Pitfall 7 teilweise)
3. **QR-Token-Modell + atomarer Redeem + TTL/Revoke** (adressiert Pitfalls 1, 11)
4. **Helfer-Session-Lifecycle + Backend** (adressiert Pitfall 3 backend-seitig)
5. **Helfer-View Backend (read-only, eigener DTO, eigene Permission)** (adressiert Pitfall 4)
6. **Attendance-Endpoints (idempotent)** (adressiert Pitfall 6)
7. **Live-Counter Endpoint + Frontend-Mock-Review** (adressiert Pitfall 5)
8. **Frontend: QR-Scanner-Komponente + Manual-Fallback** (adressiert Pitfall 8)
9. **Frontend: Helfer-View + Auth-Persistenz + Connection-Banner** (adressiert Pitfalls 2, 3 frontend-seitig)
10. **Frontend: Vorstands-View + GV-Schließen-Bestätigung** (UX-Pitfalls)
11. **Protokoll-Export-Endpoint + UI** (adressiert Pitfall 7 finalisierend)
12. **Pre-GV-Generalprobe + Operations-Plan + Vorstands-Schulung** (adressiert Pitfalls 2, 9; Cross-Cutting)

Phase 12 ist **gleichberechtigt mit Code-Phasen** und nicht „nice-to-have". Roadmap muss das explizit als Phase modellieren, mit eigenen Erfolgskriterien.

---

## Sources

### Domain-spezifische Quellen
- [Common Mistakes of Using QR Codes for Event Check-in (Dreamcast)](https://www.dreamcast.in/blog/qr-codes-for-event-check-in/) — Event-Check-in-Failure-Modes (MEDIUM)
- [QR code quiet zone and contrast: the print checklist (QRshuffle)](https://qrshuffle.com/blog/qr-code-quiet-zone-contrast) — Print/Scan-Failures (MEDIUM)
- [How to Handle a WiFi Outage at Your Event (etechrentals)](https://etechrentals.com/etech-blog/how-to-handle-wifi-outage-at-your-event/) — WLAN-Backup-Strategien (MEDIUM)
- [A Critical Point of Failure: Event Network Connectivity (Xpodigital)](https://www.xpodigital.com/blog/sales-kickoff-event-wifi) — „Event WiFi doesn't fail because providers are incompetent" (MEDIUM)

### Technische Quellen
- [Race Condition in /get-patch token replay (GHSA-vh5j-5fhq-9xwg)](https://github.com/tailot/taylored/security/advisories/GHSA-vh5j-5fhq-9xwg) — Konkretes One-Time-Token-Race-CVE (HIGH)
- [Race Conditions in Web Applications (Raijuna)](https://www.raijuna.com/knowledge/race-conditions) — TOCTOU-Pattern für Token-Redeem (HIGH)
- [getUserMedia in standalone PWA bug (WebKit 185448)](https://bugs.webkit.org/show_bug.cgi?id=185448) — iOS Safari getUserMedia-Quirks (HIGH)
- [Repeated Camera Permission Prompts in Web SPA (WebKit 215884)](https://bugs.webkit.org/show_bug.cgi?id=215884) — iOS-Permission-Repeat (HIGH)
- [Session Lost on Mobile Browsers After Closing Tab (ABP.IO)](https://abp.io/support/questions/8885/Session-Lost-on-Mobile-Browsers-iOSAndroid-After-Closing-Tab) — Mobile-Browser-Session-Loss (MEDIUM)
- [Idempotency in APIs (RestfulAPI.net)](https://restfulapi.net/idempotent-rest-apis/) — Idempotent-PUT für Anwesenheit (HIGH)

### Recht / Verbandskontext
- [Generalversammlung Wohnungsbau-Genossenschaft Protokollierung (Haufe)](https://www.haufe.de/id/beitrag/generalversammlung-einer-wohnungsbau-eg-formen-nach-neu-32-protokollierung-der-generalversammlung-HI15517552.html) — Protokoll-Pflichtinhalte (HIGH)
- [§ 47 GenG — Niederschrift über Beschluss](https://dejure.org/gesetze/GenG/47.html) — Gesetzliche Anforderungen (HIGH)
- [Anwesenheitsliste im Verein (campai)](https://www.campai.com/de/akademie/anwesenheitsliste-im-verein) — DSGVO-Datenminimierung (MEDIUM)
- [Datenschutz und Mitgliederversammlung (Vereinswelt)](https://www.meine.vereinswelt.de/artikel/so-bekommen-sie-das-thema-datenschutz-und-mitgliederversammlung-in-den-griff/) — DSGVO-Praxis-Hinweise (MEDIUM)

### Codebase-interne Quellen
- `.planning/PROJECT.md` (HIGH — Projekt-Constraints, Out-of-Scope, Key-Decisions)
- `.planning/codebase/CONCERNS.md` (HIGH — Tech-Debt: M3 Field-Level-Access, M5 No Hard Delete, N1 unsafe Send/Sync, audit-verify-test fehlt, default body limit, etc.)
- `CLAUDE.md` (HIGH — Architektur-Regeln, Audit-Macro-Pflicht für bestehende Entitäten, Component-First-Frontend)

---

*Pitfalls research for: GV-Anwesenheits-Erfassung mit One-Time-QR-Helfer-Sessions, Live-Erstrollout*
*Researched: 2026-05-01*
