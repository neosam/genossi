## Context

Der aktuelle Zustand des HTTP-Perimeters ist in mehreren Files konzentriert, die für sich jeweils einfache Defaults haben, in Summe aber keine Verteidigungslinie bilden:

- `genossi_rest/src/lib.rs:415`: `.layer(CorsLayer::permissive())` — erlaubt jede Origin, jede Methode, jeden Header.
- Keine Middleware für Rate-Limiting — weder global noch route-spezifisch.
- Keine Response-Header-Middleware — weder in Axum (z.B. via `tower-http SetResponseHeaderLayer`) noch auf nginx-Ebene dokumentiert.
- `genossi_rest/src/application.rs:66-68`: `if api_key != stored_key { ... }` — Plain Rust-Equality auf Strings. Timing-Leak theoretisch möglich, über Public-Internet durch Noise schwerer, aber Lehrbuch-Antipattern.
- `genossi_rest/src/application.rs:70-92`: Input-Validierung besteht ausschließlich aus `is_empty()`-Checks. Keine Email-Format-Prüfung, keine Längenlimits, `shares` wird ungeprüft übernommen (`body.shares` ist Typ-sicher i32, aber 0 oder negativ wird akzeptiert).
- `genossi_rest/src/application.rs:109`: `submit(&submission, true)` — jeder erfolgreiche Public-Submit triggert eine Confirmation-E-Mail. Bei einem Key-Leak ist das ein automatischer Mail-DoS-Verstärker.

Die App läuft hinter nginx mit Let's Encrypt (modul.nix hat ACME-Integration). Theoretisch könnten Security-Header auf nginx-Ebene gesetzt werden — aber eine App-seitige Lösung ist portabler (auch für Non-NixOS-Deploys) und die Verantwortlichkeit bleibt beim Code.

**Constraints:**
- Kein Breaking-Change für den legitimen WordPress-Plugin-Call — die bestehende `X-Api-Key`-Konvention bleibt.
- Kein Migration-Aufwand für bestehende Daten.
- Minimal zusätzliche Dependencies — nur wenn mit Standard-Rust-Ökosystem nicht abgedeckt.
- Rate-Limits dürfen den Normalbetrieb (OIDC-Login-Rush am Morgen, WordPress sendet Application) nicht stören.

## Goals / Non-Goals

**Goals:**
- CORS akzeptiert nur explizit erlaubte Origins — Default: die eigene Origin aus `BASE_PATH`.
- Jeder Response trägt die Standard-Security-Header (HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy).
- Rate-Limits für `/authenticate` und `/join` machen Brute-Force und Key-Leak-Ausnutzung langsam genug, dass sie im Log auffallen und der Mail-Versand nicht gesprengt wird.
- `/join` vergleicht den API-Key in konstanter Zeit und lehnt offensichtlich malformed Eingaben (leere Strings, unplausible Länge, kein `@` in Email) mit klaren 422er-Fehlern ab.

**Non-Goals:**
- Keine WAF-artige Content-Inspection (CSP, DDoS-Schutz in der App) — das gehört auf Infrastruktur-Ebene.
- Keine IP-Blocklists / Fail2Ban-ähnliche Mechanismen — Rate-Limit reicht für den Scope.
- Kein CAPTCHA am `/join`-Endpunkt — das bringt WordPress-seitige Abhängigkeiten, ist aus Plugin-Sicht ein separates Thema.
- Keine neue Version des API-Keys (Rotation bleibt wie sie ist, Endpoint existiert bereits).
- Keine Content-Security-Policy — das ist wegen WASM/Dioxus tricky und gehört in einen eigenen, frontend-nahen Change.

## Decisions

### CORS: Allowlist aus Env + Config-Store

**Wahl:** Default-Origin wird aus `BASE_PATH` (Env-Variable) abgeleitet und beim Server-Start als einziger erlaubter Origin in den `CorsLayer` geschrieben. Zusätzliche Origins können im Config-Store unter `cors_allowed_origins` als comma-separated Liste eingetragen werden und werden beim Start gelesen.

**Alternativen:**
- *Config-Store als einzige Quelle:* zwingt Admin zum Nachkonfigurieren auch im Standard-Fall. Umständlich bei Neudeployments.
- *Wildcard `*`:* das ist genau der aktuelle Zustand und was wir loswerden wollen.
- *Dynamisches Rebuild der CORS-Layer zur Laufzeit:* Axum unterstützt das nicht ohne Workaround. Änderungen an `cors_allowed_origins` im Config-Store erfordern einen Service-Restart (dokumentieren).

**Rationale:** `BASE_PATH` ist bereits die dokumentierte Source of Truth für die Server-URL (Swagger-UI, OIDC-Callback). Dass die Standard-Origin daraus kommt, ist keine neue Information.

### Rate-Limiting: `tower-governor`

**Wahl:** `tower-governor` Crate als Middleware-Layer. Drei Policy-Stufen:
- **Global**: 60 req/min per IP, auf alle Routes angewendet
- **Auth-kritisch**: 10 req/min per IP, auf `/authenticate`
- **Public-Write**: 5 req/min per IP, auf `/join`

Rate-Limit-Exceeded → HTTP 429 mit `Retry-After`-Header.

**Alternativen:**
- *Eigener Rate-Limiter:* einfach, aber unnötiges NIH.
- *`tower_http::limit::ConcurrencyLimitLayer`:* begrenzt nur Parallelität, nicht Rate.
- *`tower-sessions-core` o.ä. mit eigenem Backing-Store:* zuviel Overhead für unsere Last.

**Zustand:** `tower-governor` nutzt in-process `governor`-State (token bucket). Kein Redis, keine externe Abhängigkeit. Bei Multi-Instance-Deploy würde das jede Instance getrennt limiten — da wir Single-Instance laufen, passt das.

**Ausnahmen:** Statische Assets (`/` Frontend-Serve via Dioxus) sollen nicht gelimited werden, damit Dioxus-Startup nicht unter dem 60er-Limit leidet. Lösung: Rate-Limit nur auf `/api/*` und `/authenticate`.

### Security-Header: `tower-http::set_header`

**Wahl:** Ein zentraler Middleware-Stack mit `SetResponseHeaderLayer::if_not_present(...)` für jeden Header. Werte:
- `Strict-Transport-Security`: `max-age=63072000; includeSubDomains` (2 Jahre, konservativ ohne `preload`)
- `X-Content-Type-Options`: `nosniff`
- `X-Frame-Options`: `DENY`
- `Referrer-Policy`: `strict-origin-when-cross-origin`
- `Permissions-Policy`: `camera=(), microphone=(), geolocation=(), payment=()`

**Alternativen:**
- *nginx-seitig setzen:* funktioniert, aber bindet an Deploy-Config. Nicht für Dev.
- *CSP mitnehmen:* WASM macht das kompliziert (`script-src 'wasm-unsafe-eval'`, Dioxus Hot-Reload, Swagger-UI), separater Change.

### `/join`-Härtung: `constant_time_eq` + `validator` Crate

**Wahl:**
- API-Key-Vergleich: `constant_time_eq::constant_time_eq(api_key.as_bytes(), stored_key.as_bytes())`. Eine kleine Crate (≈50 LOC), seit Jahren stabil, keine Deps.
- Input-Validierung: wir schreiben eine kleine eigene Validierung in einer Helper-Funktion (`validate_join_request`) statt neue Deps zu ziehen. Email-Check: `email.contains('@') && email.len() > 3 && email.len() <= 320`. Kein Full-RFC-Parser — pragmatisch für Spam-Schutz.

**Alternativen:**
- *`validator` Crate:* feature-reich, aber für unsere wenigen Regeln Overkill.
- *`lettre::Address::parse`:* Lettre ist eh schon dabei. Könnten wir nutzen für echten RFC-Check. **Entscheidung:** Wenn `lettre` bereits als Dep da ist (wegen SMTP), `Address::from_str` für Email-Check verwenden. Wenn nicht, die eigene Minimal-Prüfung.
- *JWT als Public-Key:* overkill für diesen Use-Case.

**Längenlimits:**
- first_name, last_name: max 128 chars
- email: max 320 (RFC 5321)
- street: max 128
- house_number: max 32
- postal_code: max 16
- city: max 128
- title: max 64
- shares: >= 1 (i32 kann auch negativ sein in Rust)

Bei Überschreitung: 422 mit `{ "errors": [{"field": "email", "message": "too long (max 320)"}, ...] }`.

### E-Mail-Versand bei Public-Submit

**Wahl:** `send_mail` bleibt `true`, aber das Mail-Queue-System schützt schon per Design gegen DoS (Mails sind gequeued, pro Minute ein Wurf — s. `mail-queue` Spec). Der eigentliche Engpass ist nicht der SMTP-Server, sondern der Log-Spam bei vielen Submissions. Der Rate-Limit auf `/join` (5/min/IP) entschärft das hinreichend.

Wir **verzichten** in diesem Change auf eine neue Config-Option zum Abschalten des Mails — das wäre ein anderer Change (`membership-application` Policy), nicht Perimeter-Security.

**Alternative:** neue Config `send_confirmation_on_public_submit: bool`. Nice to have, aber Scope-Expansion. Wenn der Bedarf auftritt, separater kleiner Change.

## Risks / Trade-offs

- [Risk] Rate-Limit auf `/authenticate` blockiert versehentlich legitime User, die durch einen OIDC-Loop geraten (z.B. wenn WordPress down geht und Browser redirects in Schleife läuft) → Mitigation: 10/min pro IP ist großzügig, ein normaler Login braucht 1-2 Requests. Wenn's doch klappert, Wert im Code erhöhen — keine DB-Config, da Reboot nötig wäre.
- [Risk] CORS-Allowlist zu eng — ein zweites Deploy der App (Staging, zweite Instanz) kommt nicht durch → Mitigation: Config-Store-Eintrag `cors_allowed_origins` dokumentieren. Operativ lösbar.
- [Risk] Rate-Limit-State ist in-process → bei Rolling-Deploy oder Prozess-Restart wird der State gelöscht, kurz nach Restart könnten Angreifer eine Burst-Window nutzen → Mitigation: vernachlässigbar. Das Rate-Limit ist kein Hardening gegen nation-state, sondern gegen Skript-Kiddies und versehentlichen Bot-Traffic.
- [Risk] `tower-governor` bringt eine zusätzliche Dep (plus `governor` als transitive) → Mitigation: beide sind kleine, aktiv gepflegte Crates. Alternative ist eigenbau = mehr Wartungslast.
- [Risk] HSTS mit 2 Jahren ist schwer zu widerrufen, wenn die Seite aus Versehen ohne HTTPS aufgesetzt wird → Mitigation: Let's Encrypt/ACME im NixOS-Modul ist schon konfiguriert (`forceSSL = true`), Downgrade ist kein realistischer Fall.
- [Risk] `X-Frame-Options: DENY` bricht eine eventuelle WordPress-Embed-Strategie → Mitigation: Das WordPress-Plugin sendet Anträge via serverseitiges POST, embedded kein UI. `DENY` ist sicher für uns.
- [Risk] Email-Validierung via `contains('@')` ist lax — wird `foo@bar` durchlassen → Mitigation: Das ist akzeptabel, der echte Filter ist der spätere SMTP-Versand (wenn die Mail bouncet, wird das behandelt). Wir verhindern hier nur *offensichtlichen* Junk.

## Migration Plan

1. **Code mergen** inkl. neuer Dependencies (`tower-governor`, `constant_time_eq`).
2. **Config-Store**: `cors_allowed_origins` ist optional — wenn nicht gesetzt, leitet sich alles aus `BASE_PATH` ab. Kein Admin-Eingriff nötig.
3. **Deploy.** Kein Migration-Step.
4. **Verifikation:**
   - `curl -H "Origin: https://evil.com" ...` → CORS-Header fehlt oder blockt.
   - `curl` auf `/join` mit 6 Requests in 1 Minute → letzte kriegt 429.
   - Browser DevTools → Response Headers zeigen HSTS, X-Frame-Options etc.
   - `/join` mit Email "foo" → 422 mit Field-spezifischem Error.
5. **Rollback:** Code zurück, keine Schema/Config-Änderungen zu rückabwickeln.

## Open Questions

- Soll die globale Rate-Limit-Schwelle (60/min/IP) im Config-Store konfigurierbar sein? → Empfehlung: erstmal nein, Konstante im Code. Vermeidet Komplexität. Wenn Ops das braucht, Follow-up.
- Wird `lettre::Address` aktuell schon verwendet oder muss es erst aktiviert werden? → beim Implementieren kurz prüfen; wenn nein, einfache `contains('@')` Prüfung reicht.
- WordPress-Plugin: wird die neue 422-Antwort korrekt angezeigt? → Ist eine Test-Aufgabe beim Rollout, keine Code-Aufgabe hier. In `tasks.md` als Smoke-Test aufnehmen.
