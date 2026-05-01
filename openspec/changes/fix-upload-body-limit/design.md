## Context

Axum setzt ein Default-Body-Limit von 2 MB, sobald `Router` ohne expliziten `DefaultBodyLimit`-Layer benutzt wird. Ein Grep über das Repo zeigt: im aktuellen Code ist nirgends ein `DefaultBodyLimit` gesetzt (`genossi_rest/src/lib.rs`, `genossi_rest/src/member_document.rs`, `genossi_rest/src/static_document.rs`) — das Limit wirkt implizit.

Die Service-Layer-Limits liegen deutlich höher:
- `MAX_FILE_SIZE = 50 * 1024 * 1024` in `genossi_service_impl/src/member_document.rs:20` (aktuell privates `const`)
- `DEFAULT_MAX_SIZE_BYTES = 10 * 1024 * 1024` in `genossi_mail/src/static_document_service.rs:25`, zur Service-Init-Zeit überschreibbar via `STATIC_DOCUMENTS_MAX_BYTES` (gelesen in Zeile 55, im `max_size_bytes`-Feld gespeichert in Zeile 50).

Die Upload-Routen sind:
- `POST /api/members/{member_id}/documents` — zusammengesetzt in `genossi_rest/src/member_document.rs:37-47` (`generate_route`), genested in `genossi_rest/src/lib.rs:503-506`
- `POST /api/static-documents` — zusammengesetzt in `genossi_rest/src/static_document.rs:65-71`, genested in `genossi_rest/src/lib.rs:538-541`

Scope ist klein (zwei Routen, eine Crate), aber Defense-in-Depth gegen versehentliche, zu große Uploads auf allen anderen Endpoints soll erhalten bleiben. Gleichzeitig gibt es im Repo weitere potenziell upload-lastige Pfade (Excel-Import, Typst-Template-Binaries), die zu überprüfen sind, bevor ein hartes globales 2-MB-Limit scharfgeschaltet wird.

## Goals / Non-Goals

**Goals:**
- Uploads bis zum jeweiligen Service-Layer-Limit funktionieren End-to-End.
- Auf allen anderen Endpoints bleibt ein kleines Body-Limit aktiv (Defense-in-Depth gegen versehentlich/böswillig zu große Bodies).
- Numerische Limits haben **eine einzige Quelle der Wahrheit** — keine Konstante, die in REST- und Service-Layer divergieren kann.
- Überschreitet ein Client das Limit, bekommt er eine klare HTTP-413-Antwort.
- Tests decken je Route den 413-Fall und den Happy-Path knapp unter dem Limit ab.

**Non-Goals:**
- Keine Änderung an den Service-Layer-Validierungen (Größe, MIME-Type, Extensions) — die bleiben die Autorität.
- Keine neue Upload-Strategie (Streaming, Chunked, S3-ähnlich) — weiterhin klassisches Multipart.
- Kein allgemeines Body-Limit-Konfigurations-System — nur die zwei konkret betroffenen Upload-Routen plus ggf. Routen, die im Pre-Audit auffallen.
- Kein Client-seitiges Progress/Preview.
- Keine Migration der 50-MB-Member-Dokument-Konstante zu einer ENV-Variable (Open Questions).

## Decisions

### Entscheidung 1: Globales Limit explizit auf 2 MB setzen, route-lokal höher

**Gewählt:** Im Top-Level-Router (`genossi_rest/src/lib.rs`) ein explizites `DefaultBodyLimit::max(2 * 1024 * 1024)` setzen. Auf den Upload-Routen route-lokal ein `DefaultBodyLimit::max(<größerer Wert>)` hinzufügen.

**Layer-Ordering in Axum:** Die route-spezifischere (inneren) `DefaultBodyLimit`-Einstellung überschreibt die globale. Das ist dokumentiertes Axum-Verhalten und die Grundlage dafür, dass Global-2-MB und Route-50-MB koexistieren können. Die Tests (siehe Test-Plan) verifizieren dieses Verhalten pro Route.

**Alternativen:**
- *Nur route-lokale Layer, globalen Axum-Default (2 MB) implizit lassen.* Verworfen: der Axum-Default ist Versions-abhängig; ein expliziter Wert ist self-documenting und stabil gegen zukünftige Framework-Upgrades.
- *Globales Limit auf 50 MB hochziehen, statt pro-Route feintunen.* Verworfen: widerspricht dem Defense-in-Depth-Ziel und öffnet Angriffsfläche auf alle Endpoints.

### Entscheidung 2: Layer pro Upload-Route direkt beim `POST`-Handler

**Gewählt:** Der route-lokale `DefaultBodyLimit`-Layer wird direkt an der `post(upload_document)`-Route innerhalb der jeweiligen `generate_route()` angehängt (via `.route_layer(...)`). So liegt die Konfiguration neben der Route, die sie betrifft.

**Alternativen:**
- *Layer im Top-Level-`lib.rs` per `.nest(...).layer(...)` anhängen.* Verworfen: separiert Limit und Handler örtlich; Änderungen am Limit landen in einer Datei, die sonst keine Upload-Semantik kennt.
- *Layer auf dem gesamten Sub-Router `generate_route()` (also auch für GET/DELETE).* Verworfen: GET/DELETE brauchen kein hohes Limit.

### Entscheidung 3: Einheitliche Strategie — Limits als Parameter in `generate_route(max_bytes)`

**Gewählt:** Beide Upload-Routen werden nach demselben Muster gebaut:

1. Der jeweilige Service-Trait (`MemberDocumentService`, `StaticDocumentService`) bekommt eine Methode `max_upload_bytes() -> usize`.
2. `genossi_bin` liest den effektiven Wert einmalig beim Service-Setup aus (über die neue Methode) und reicht ihn an `generate_route(max_bytes: usize)` der jeweiligen REST-Route.
3. Die `generate_route`-Funktion setzt `DefaultBodyLimit::max(max_bytes)` auf die POST-Route.

**Warum eine einzige Strategie für beide Routen:**
- **Eine Quelle der Wahrheit:** Service kennt sein eigenes Limit. REST-Layer ruft nicht direkt auf ein Service-Impl-`const` zu — das Trait-Crate-Pattern bleibt intakt.
- **Konsistenz über Routen:** Kein Mix aus "Konstante importieren" und "Service-Methode aufrufen".
- **Funktioniert zur Layer-Bau-Zeit:** Der Service wird im Bin vor dem Router-Wiring instanziiert; sein `max_upload_bytes()` ist dort direkt aufrufbar — nicht über `State<RestState>`, der erst zur Request-Zeit existiert.
- **Testbar:** Fakes/Mocks können in Tests andere Limits setzen, ohne globale Konstanten zu ändern.

**Alternativen:**
- *`MAX_FILE_SIZE` im `genossi_service_impl` auf `pub const` umstellen und im REST-Layer importieren.* Verworfen: untergräbt das Trait-Crate-Pattern (REST soll nur gegen Traits kennen, nicht gegen Impls — auch wenn die Cargo-Dependency technisch existiert). Verschiedene Static-Route-Umgebungen (ENV-Override) lassen sich so nicht homogen behandeln.
- *Service-Methode direkt im Handler aufrufen statt als Layer.* Verworfen: Axum wirft den Body dann bereits mit 2 MB ab (Body-Limit wirkt *vor* dem Handler). Der Handler sieht den zu großen Body nie.

**Konsequenz für den Member-Document-Service:** Das aktuell private `MAX_FILE_SIZE = 50 * 1024 * 1024` wandert hinter `max_upload_bytes()`. Die Zahl bleibt als Konstante im Impl-Modul (hardcoded — keine ENV), aber exponiert ausschließlich über die Trait-Methode.

**Konsequenz für den Static-Document-Service:** `StaticDocumentService` hat den Wert bereits im Feld `max_size_bytes`. Die neue Trait-Methode liefert diesen Feldwert zurück (`usize`-Cast aus `u64`).

### Entscheidung 4: HTTP 413 ohne Custom-Handler

Axum liefert bei Überschreitung eine HTTP 413 Payload Too Large zurück. Das ist semantisch korrekt; kein Custom-Handler nötig. Der Error-Pfad wird in den E2E-Tests verifiziert.

### Entscheidung 5: Multipart-Semantik des Body-Limits explizit dokumentieren

`DefaultBodyLimit` begrenzt den **Gesamt-Body** einer Request (inkl. Multipart-Boundaries und Metadaten-Feldern), nicht einzelne Felder. Für Member-Dokumente heißt das: 50 MB ist der Multipart-Body, nicht die Datei-Netto-Größe. Der Overhead für Boundaries ist minimal (< 1 %), aber bei Dateien exakt an der Grenze relevant. Die Service-Layer-Prüfung bleibt die finale Autorität für die Datei-Größe selbst.

## Risks / Trade-offs

- **[Service-Layer bleibt die Autorität]** → Das Axum-Layer ist nur die äußere Schranke. Ändert sich der Service-Layer-Wert, greift das automatisch bei nächstem Service-Setup (die Trait-Methode liefert den aktuellen Wert).
- **[Static-Wert wird beim Startup einmal gelesen]** → Wird `STATIC_DOCUMENTS_MAX_BYTES` zur Laufzeit geändert, greift das nicht ohne Neustart. Status quo — keine Regression.
- **[Memory-Footprint bei großen Uploads]** → 50-MB-Multipart landet im Speicher (Axum puffert by default). Bei parallelen Uploads kumulativ. Das Risiko existiert bereits in der Service-Layer-Erwartung; nicht neu durch diesen Change. Mitigation (Streaming) ist ein separates Thema.
- **[Globales 2-MB-Limit könnte bestehende Endpoints kaputtmachen]** → Mitigation: siehe Pre-Implementation-Audit im Tasks-Dokument — Grep auf `Multipart`/Upload-Routen VOR der Implementation. Findings werden entweder in diesen Change aufgenommen oder als Follow-up dokumentiert.
- **[Falsches Limit an der falschen Route]** → Mitigation: Tests pro Route (siehe Test-Plan).
- **[Multipart-Overhead knapp an der Grenze]** → Clients, die 50-MB-Dateien hochladen, können minimal über dem Body-Limit landen. Ggf. auf 50 MB + 1 MB Reserve gehen; Entscheidung liegt beim Implementer.

## Test-Plan

Pro Upload-Route **zwei neue E2E-Tests** in `genossi_rest/tests/` (oder `genossi_bin/tests/e2e_tests.rs` — je nach bestehender Struktur):

1. **Happy-Path knapp unter Limit:** Upload mit Body ~95 % des route-spezifischen Limits → erwartet 200/201, Datei landet im Storage.
2. **413 über Limit:** Upload mit Body knapp über dem Limit → erwartet HTTP 413, keine Datei im Storage, kein Panik im Server.

**Zusätzlich ein globaler Test:** POST auf einen Nicht-Upload-Endpoint mit Body > 2 MB → erwartet 413 (bestätigt, dass das globale Limit greift).

Test-Infrastruktur: vorhandene `genossi_rest/src/test_server.rs` (Test-Server mit zufälligem Port, In-Memory-SQLite) wiederverwenden.

## Migration Plan

Reiner Code-Fix. Kein Datenbank-Migration, keine Config-Änderung, kein User-facing Breaking Change. Rollback: Layer-Calls entfernen, altes Verhalten wiederhergestellt.

Deployment läuft über das normale Release. Nach Deploy manuell ein >2-MB-PDF auf `/api/members/{id}/documents` hochladen, um den Fix an echter Infrastruktur zu verifizieren.

**Pre-Implementation-Audit (als erster Task):** Grep über den REST-Layer nach `Multipart`, `axum::body::Bytes`, `Bytes` und allen `post`-Routen mit potenziell großem Body (Excel-Import, Typst-Template-Binaries, Mail-Attachments). Jede gefundene Route wird kategorisiert: "< 2 MB ausreichend", "Limit nötig" oder "bereits abgedeckt". Ergebnis entscheidet, ob der Change über die zwei identifizierten Routen hinaus wachsen muss.

## Open Questions

- Soll das 50-MB-Member-Dokument-Limit ebenfalls per ENV konfigurierbar werden (analog zu `STATIC_DOCUMENTS_MAX_BYTES`)? Aktuell hardcoded — aus Scope hier bewusst ausgelassen, aber ein späterer Change könnte das vereinheitlichen.
- Soll der globale 2-MB-Wert selbst als Konstante / ENV konfigurierbar sein (z.B. `DEFAULT_BODY_LIMIT_BYTES`)? Nicht-Ziel in diesem Change, aber für Deployments mit sehr kleinen/sehr großen Upload-Anforderungen relevant.
