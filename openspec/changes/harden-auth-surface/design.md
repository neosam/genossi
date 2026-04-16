## Context

Die aktuelle Session-Architektur ergibt sich aus dem Zusammenspiel mehrerer Komponenten, die jeweils für sich sinnvoll sind, in Summe aber einen unerwünscht langen Hijack-Window öffnen:

- `axum-oidc` validiert OIDC-Claims gegen den WordPress-OIDC-Provider **nur beim Initial-Login**.
- `register_session` (`genossi_rest/src/session.rs:22-60`) erstellt danach eine Genossi-eigene DB-Session mit hartcodiertem 365-Tage-Timeout und setzt das Cookie `app_session`.
- `context_extractor` (`genossi_rest/src/session.rs:63-102`) liest bei jedem folgenden Request nur noch das Cookie und verifiziert die Session gegen die lokale DB — **OIDC wird nicht mehr konsultiert**.
- Zwischen Zeile 72 und 85 stehen vier `tracing::info!`-Aufrufe, die Session-ID, Cookie-Liste und Session-Entity mit allen Feldern ausgeben. Bei hartem `genossi=debug` im `EnvFilter` (`genossi_bin/src/main.rs:16`) landen diese Daten bei jedem authentifizierten Request in den Logs.

Die Session-Tabelle (`migrations/sqlite/20250129000000_create_auth_tables.sql:48-55`) hat heute die Spalten `id`, `user_id`, `expires`, `created`, `claims`. Ein Inaktivitäts-Feld existiert nicht.

`SessionEntity` in `genossi_dao/src/permission.rs:116-122` spiegelt die Tabelle 1:1 wider.

**Constraints:**
- Backwards-Compatibility zum Login-Flow über WordPress-OIDC ist Pflicht — wir ändern nicht, wie sich User einloggen.
- Keine neuen Crates (Scope-Kontrolle).
- Single-Server-Deployment: SQLite, keine verteilte Session-Invalidation nötig.
- App ist öffentlich im Internet → Defense-in-Depth zählt.

## Goals / Non-Goals

**Goals:**
- Session-ID erscheint nirgends mehr in Logs, Fehlermeldungen oder Responses.
- Sessions haben einen absoluten Lebenszeit-Deckel (14 Tage) und einen Inaktivitäts-Timeout (24 Stunden).
- User können ihre eigenen Sessions selbst revoken, ohne DB-Admin oder Support.
- Auth-Path wirft keine Panics mehr; DB-Fehler führen zu sauberem HTTP 500.
- Bestandsdaten (alte Sessions) werden safe migriert.

**Non-Goals:**
- Kein Re-Check gegen den OIDC-Provider bei jedem Request (zu viel Komplexität und Last für diesen Change). Wenn später gewünscht, ist das ein eigener Change.
- Kein WordPress → Genossi Push-Revoke (Account-Deaktivierung bleibt ein Ops-Prozess, der über den Inaktivitäts-Timeout innerhalb von 24h de facto wirksam wird).
- Kein Admin-Endpoint "fremde Sessions revoken" (nur Self-Service für diesen Change; Admin-Case später).
- Kein UI-Reskin des Login-Flows.
- Keine Rate-Limits (gehört ins separate Bundle `harden-public-perimeter`).

## Decisions

### Session-Lifetime: 14 Tage absolut + 24h Inaktivität

**Wahl:** Zwei-Timeout-Modell mit `created + 14d` (absolut) und `last_used_at + 24h` (Inaktivität). Der strengere von beiden greift.

**Alternativen:**
- *365 Tage behalten, nur Logs fixen:* löst das Log-Problem, aber ein geleaktes Cookie ist weiterhin 1 Jahr nutzbar. Im Public-Internet nicht vertretbar.
- *Nur Inaktivitäts-Timeout, kein Absolut-Timeout:* User der täglich eingeloggt ist bleibt theoretisch ewig drin. Kein harter Cap auf "worst case Hijack-Dauer".
- *OIDC-Re-Check bei jedem Request:* löst das Problem fundamentaler (inkl. WordPress-Revoke), erhöht aber Last (OIDC-Call pro Request oder Token-Caching nötig), bringt Komplexität (Refresh-Token-Handling) und hat längere Implementierungszeit. Eigener Change-Kandidat für später.

**Rationale für 14d + 24h:** Der Vorstand nutzt die App in der Regel alle paar Tage bis wöchentlich, die 14 Tage absolute Cap sind pragmatisch (Nach-Urlaub-Re-Login akzeptabel). Die 24h-Inaktivität macht das "Laptop-über-Nacht-gestohlen"-Szenario am nächsten Tag harmlos.

### Inactivity via `last_used_at` Update pro Request

**Wahl:** Neue Spalte `last_used_at INTEGER` in der `session`-Tabelle. `verify_user_session` aktualisiert sie nach erfolgreicher Verifikation via separates `UPDATE session SET last_used_at = ? WHERE id = ?`.

**Alternativen:**
- *Implizite Inaktivität via `expires` rollierend setzen:* verkompliziert den Unterschied zwischen Inaktivitäts- und Absolut-Timeout. Zwei explizite Felder sind klarer.
- *Timestamp nur stündlich updaten (Throttle):* weniger Write-Last auf SQLite, aber der Gewinn ist bei unserer Request-Frequenz vernachlässigbar. Bei Performance-Problemen später nachrüstbar.

**Schreib-Last:** Jeder authentifizierte Request triggert ein `UPDATE`. SQLite mit WAL-Modus verträgt das locker für die erwartete Last (Vorstand + WordPress-Public-Endpoint).

### Logging: User-ID statt Session-ID, Debug-Level

**Wahl:** Alle `{:?}`-Ausgaben von Cookies und Session-Entities entfernen. Statt dessen auf `debug`-Level knappe Messages wie `tracing::debug!(user_id = %session.user_id, "session verified")`. Die User-ID ist der WordPress-Username — nicht geheim, aber pseudonym genug.

**Alternativen:**
- *Session-ID als Hash loggen:* theoretisch möglich (SHA256 der Session-ID → unleserlich), aber für Debugging hat man dann auch nichts. User-ID ist nützlicher und sicherer.
- *Gar nichts loggen:* verschlechtert Debugging-Fähigkeit merklich. Die User-ID reicht, um nachvollziehen zu können, wer was aufgerufen hat.

### Panic-Entfernung: `.expect` → `match` mit 500er-Response

**Wahl:** `ensure_user_and_create_session` Fehler wird als `RestError::InternalError` zurückgegeben. Log-Message enthält die Error-Details, Response bleibt generisch.

**Alternativen:**
- *Retry-Loop:* könnte bei transienten DB-Errors helfen, aber fügt Komplexität hinzu und maskiert echte Probleme. Erstmal sauberes Fail-Fast.

### Self-Revoke: `POST /api/session/revoke-all`

**Wahl:** Authentifizierter Endpoint (braucht gültige Session). Löscht alle Sessions des aktuellen Users aus der DB (inklusive der, mit der der Request gekommen ist → nächster Request → 401 → Re-Login).

**Alternativen:**
- *Nur andere Sessions revoken, aktuelle behalten:* User-freundlicher, aber der typische Use-Case ist "ich vermute mein Account wurde kompromittiert — alles weg". Alles killen und Re-Login ist die sichere Default-Option.
- *DELETE-Methode statt POST:* HTTP-semantisch sauberer, aber unser Stil im Projekt verwendet POST für Aktionen (Konsistenz mit `/confirm`, `/reject`).

### Migration: `last_used_at` auf Bestandsdaten

**Wahl:** Neue Migration fügt Spalte hinzu mit `NOT NULL DEFAULT 0`, dann `UPDATE session SET last_used_at = created WHERE last_used_at = 0`. Nach dem Deployment: `verify_user_session` prüft `now - last_used_at < 24h` — da alle Bestandssessions `last_used_at = created` haben und `created` im Normalfall Wochen alt ist, werden sie beim nächsten Request gelöscht. Effektiv: alle User müssen sich neu einloggen. Das ist erwünscht (**BREAKING** laut Proposal).

## Risks / Trade-offs

- [Risk] 14-Tage-Cap ist zu kurz für User, die die App selten nutzen (z.B. reiner Monat-Ende-Login für Finanzen) → Mitigation: Re-Login über OIDC ist schnell (WordPress-SSO), gefühlt wie normales Weitersurfen. Wenn sich das als Problem erweist: Wert ist eine Konstante, leicht zu tunen.
- [Risk] Session-Revoke löscht auch die laufende Session → User bekommt sofort 401, das kann verwirrend wirken → Mitigation: Response des Endpoints enthält klaren Text "Alle Sessions beendet. Sie werden ausgeloggt." Frontend zeigt das vor dem Redirect an.
- [Risk] Schreib-Last durch `last_used_at`-Update bei jedem Request → Mitigation: Bei Last-Problemen Throttling auf 5-Min-Granularität nachrüsten. Aktuell unproblematisch.
- [Risk] Logging-Reduktion erschwert Debugging → Mitigation: User-ID bleibt, Request-Path bleibt (tower_http). Session-spezifische Probleme kann man via User-ID und Timestamp nachvollziehen. Falls wirklich nötig, kann pro Bedarf ein temporäres Feature-Flag gesetzt werden.
- [Risk] Breaking-Change: alle User müssen sich nach Deployment neu einloggen → Mitigation: Ankündigung im Vorstand-Chat vor Release. Login über WordPress ist ein Klick — Schmerz minimal.

## Migration Plan

1. **Code-Änderungen mergen** (Migration, Service-Logik, Routes, Logging-Fix) auf `main`.
2. **Release-Ankündigung** an Vorstand: "Nach dem nächsten Deploy müsst ihr euch einmal neu einloggen."
3. **Deploy** → SQLx führt die Migration automatisch aus (`last_used_at` Spalte wird angelegt, Bestandszeilen bekommen `created`-Wert).
4. **Verifikation**: Erster Login eines Test-Users → neue Session hat `last_used_at` gesetzt; nach >24h ohne Traffic verfällt sie; Revoke-Endpoint funktioniert.
5. **Rollback-Plan**: Code zurückrollen, Spalte `last_used_at` bleibt bestehen (harmlos, wird dann nicht mehr aktualisiert oder gelesen). Da wir keine Daten löschen, ist der Rollback datenschonend.

## Open Questions

- Soll der Revoke-Endpoint zusätzlich eine Admin-Variante bekommen ("revoke all sessions for user X")? → Aktuell Non-Goal, kann als Follow-up einzeln proposed werden. Bitte beim Verfassen der tasks.md im Kopf behalten, aber nicht implementieren.
- Frontend-Integration des Revoke-Endpoints: Soll ein Button im User-Menu gleich mit dazu, oder erstmal nur Backend-API? → Empfehlung: API jetzt, Frontend-Button als separate kleine Change-Proposal nachziehen. Das verhindert Scope-Creep in diesem Change.
