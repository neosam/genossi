## Why

Die App ist öffentlich im Internet erreichbar, hat aber keine Schutzmaßnahmen auf dem HTTP-Perimeter: `CorsLayer::permissive()` erlaubt Requests von jeder Origin, es gibt kein Rate-Limiting (weder für `/authenticate`, `/join`, noch für teure Endpoints wie PDF-Generierung), und Standard-Security-Header (HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy) fehlen komplett. Zusätzlich hat der einzige public Schreib-Endpunkt `/join` drei konkrete Schwächen: Timing-unsicherer API-Key-Vergleich, keine Input-Validierung über `is_empty()` hinaus, und bei jeder Request wird `send_mail=true` getriggert — d.h. jeder gültige Key-Hit löst eine E-Mail aus, was bei Leak zum Mail-DoS wird. All das sind klassische Perimeter-Themen, die vor einer ersten Lastspitze oder einem Missbrauchsversuch stehen sollten.

## What Changes

- `CorsLayer::permissive()` ersetzen durch eine konfigurierbare Origin-Allowlist, die standardmäßig nur die eigene `BASE_PATH`-Origin zulässt. Zusätzliche Origins via Config-Store administrierbar.
- Rate-Limiting via `tower-governor` (oder vergleichbar) einziehen:
  - Global per-IP Limit als sanfter Filter (z.B. 60 req/min)
  - Striktes Limit auf `/authenticate` (z.B. 10 req/min per IP) gegen OIDC-Loop-Abuse
  - Striktes Limit auf `/join` (z.B. 5 req/min per IP) gegen Spam-Anträge
  - Kein Limit auf `/api/public/member-count` (hat schon 5-Min-Cache)
- Security-Header als tower-http-Middleware:
  - `Strict-Transport-Security: max-age=63072000; includeSubDomains`
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy` minimal (keine Kamera/Mic/Geolocation/Payment)
- `/join` Hardening:
  - API-Key-Vergleich auf `constant_time_eq` umstellen
  - Input-Validierung: Email-Format (simple Regex oder crate), Längenlimits auf allen Text-Feldern (first_name/last_name/street/city 128, email 320, postal_code 16, house_number 32), `shares >= 1` explizit prüfen
  - Bei ungültigem Input: 422 mit Feld-spezifischen Fehlern statt 400 mit Free-Text
  - **Optional** (Design-Entscheidung): E-Mail-Versand beim Public-Submit drosseln (E-Mail an Admin bündeln oder per Config abschaltbar), damit ein Key-Leak nicht automatisch Mail-Spam triggert
- Request-Body-Limit explizit setzen (Axum default ist 2 MB — bleibt so, aber dokumentiert; Multipart-Uploads bis 50 MB sind über separate Route-Middleware zu handhaben)

## Capabilities

### New Capabilities

- `http-perimeter`: Definiert CORS-Politik, Security-Header, Rate-Limiting-Konfiguration und allgemeine HTTP-Härtung für den öffentlich erreichbaren Server.

### Modified Capabilities

- `membership-application`: `/join`-Endpoint bekommt konstant-Zeit-Vergleich und striktere Input-Validierung. Das ändert das beobachtbare Response-Verhalten bei ungültigen Requests (neue Fehlerschemata) und gehört daher in die Spec.

## Impact

**Code:**
- `genossi_rest/src/lib.rs` — CORS-Layer, Security-Header-Layer und Rate-Limit-Layer einziehen; Route-spezifische Rate-Limits für `/authenticate` und `/join`
- `genossi_rest/src/application.rs` — `/join`-Handler: constant-time Vergleich, Input-Validierung, bessere Error-Response
- `genossi_rest_types/src/lib.rs` — ggf. neuer Error-Response-Type für Validierung
- `genossi_bin/src/lib.rs` — Lesen von CORS-Allowed-Origins aus Config-Store

**Neue Dependencies:**
- `tower-governor` (Rate-Limiting) — aktiv gepflegt, axum-kompatibel
- `constant_time_eq` (kleine Crate, genau einer Funktion)
- `validator` oder einfache eigene Regex für Email-Format — Design-Entscheidung

**Config-Store:**
- Neuer Eintrag `cors_allowed_origins` (comma-separated, type `text`) — optional, default ist die `BASE_PATH`-Origin aus dem Env

**Datenbank:**
- Keine Migration nötig

**Benutzer:**
- Keine sichtbare Änderung für Admins. Der Vorstand kann bei Bedarf zusätzliche Origins eintragen (wenn z.B. WordPress auf anderer Domain läuft).
- 429-Responses bei Rate-Limit-Treffer sind normal und kommen nur bei Missbrauchs-Versuchen — nicht im Normalbetrieb.
- `/join`-Fehler werden jetzt pro Feld differenziert ausgespielt; das WordPress-Plugin muss diese Responses korrekt darstellen (ggf. Follow-up-Task auf Plugin-Seite).
