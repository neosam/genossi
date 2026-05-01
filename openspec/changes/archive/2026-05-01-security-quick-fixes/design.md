## Context

Das Security-Audit vom 2026-04-18 hat drei unabhängige Kleinfunde aus den REST- und Config-Layern gemeldet. H3 (`eprintln!`) und M1 (CORS) sind triviale Einzelstellen. N2 (`unwrap()` in Serialisierungs-Pfaden) ist umfangreicher: **52 Call-Sites über 14 Code-Dateien in drei Crates**. Während der Erkundung wurde sichtbar, dass die betroffenen Handler zwei unterschiedliche Stil-Muster nutzen — der `unwrap()`-Austausch ist nur in einem davon trivial.

**Aktueller Stand:**

- `genossi_rest/src/auth_middleware.rs:26` loggt per `eprintln!("Auth context extraction error: {:?}", err)`. Der Rest der Codebase nutzt durchgängig `tracing` (siehe z. B. `genossi_rest/src/lib.rs:345`).
- `genossi_rest/src/lib.rs:366-367` baut den `CorsLayer` mit `AllowMethods::any()` / `AllowHeaders::any()`. Origins sind bereits via `AllowOrigin::list(origins)` eingeschränkt.
- 52 `serde_json::to_string(&x).unwrap()`-Aufrufe in REST-Handlern. Bei Serialisierungsfehlern paniket der Tokio-Worker. Zwei Handler-Muster sind im Einsatz:
  - **Muster A** (Mehrheit, 45 Sites): Handler wrappt seinen Body in `error_handler((async { ... }).await)` mit `Result<Response, LocalError>`-Rückgabe. Fehler propagieren via `?`.
  - **Muster B** (Minderheit, 7 Sites): Handler gibt direkt `Response` zurück, mit `match { Ok → Response::builder, Err → error_response(e) }`. Kein `Result`-Kontext, kein `?` möglich ohne Struktur-Umbau.

## Goals / Non-Goals

**Goals:**

- `eprintln!` im Auth-Pfad durch strukturiertes `tracing::warn!` ersetzen.
- CORS-Methoden und -Header auf explizite Whitelists einschränken.
- Alle 52 `.unwrap()`-Aufrufe in REST-Handlern eliminieren, so dass Serialisierungsfehler zu einer regulären HTTP-500-Response führen statt den Worker zu paniken lassen.
- Muster-B-Handler auf Muster A bringen, um einheitliche `?`-Propagation zu ermöglichen.

**Non-Goals:**

- Keine breitere `unwrap()`-Jagd in nicht-REST-Modulen.
- Kein Refactoring der Response-Builder-Bibliothek (z. B. Wechsel auf `axum::Json`).
- Keine Zusammenlegung der drei `error_handler`-Funktionen (genossi_rest, genossi_mail, genossi_config bleiben separat).
- Keine Migration auf `thiserror` — bleibt als separater Change.
- Keine Änderungen am existierenden Origin-Handling.

## Decisions

### Decision 1: `tracing::warn!` statt `tracing::error!` für Auth-Fehler

Der aktuelle Code gibt `Auth context extraction error` aus und setzt anschließend den Context auf `None` — der Request läuft als unauthentifiziert weiter. Das ist keine server-seitige Fehlsituation, sondern ein erwartbarer Fall (ungültiger Cookie, abgelaufene Session, manipulierter Bearer-Token). `warn!` signalisiert "ungewöhnlich, aber benign".

**Format:** `tracing::warn!(error = ?err, "auth context extraction failed")` — strukturiertes Feld, keine Debug-Formatierung der kompletten Message. So kann das Feld später gefiltert/maskiert werden, falls der Error sensitive Details enthält.

**Alternative:** `tracing::error!` + kompletter `{:?}`-Dump. Verworfen, weil das die Fehlerstufe überhöht und identische Probleme (fehlgeschlagener Login) hätte wie `eprintln!`.

### Decision 2: CORS-Whitelist explizit als Konstanten

`CorsLayer` wird umgebaut auf:

```rust
.allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
.allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
```

`PATCH` ist nicht in der Liste — eine projektweite Suche nach `.patch(` / `Method::PATCH` ergibt null Treffer. Sollte ein neuer Handler `PATCH` nutzen, schlägt der Preflight im Browser sichtbar fehl — erwünschtes Verhalten.

**Alternative:** Methoden/Header als Config-Keys. Verworfen — keiner Deployment-seitigen Flexibilität gegenüber dem Sicherheitsgewinn vorziehbar; OpenSpec-Requirement `http-perimeter` fordert ohnehin eine feste Whitelist.

### Decision 3: Einheitliches Handler-Muster + `?`-Propagation über `From<serde_json::Error>`-Impls

Alle REST-Handler werden auf Muster A gebracht, so dass durchgängig `?` nutzbar ist:

```rust
pub async fn handler(...) -> Response {
    error_handler(
        (async {
            let x = service.call().await?;
            let json = serde_json::to_string(&x)?;        // ← greift via From-Impl
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(json))
                .unwrap())                                // ← statisch, kann nicht fehlschlagen
        }).await,
    )
}
```

Pro lokalem Error-Type wird ein `From<serde_json::Error>`-Impl ergänzt, der nach der Serverfehler-Variante des jeweiligen Enums mappt:

| Error-Type           | Datei                          | Ziel-Variante            | Hinweis |
|----------------------|--------------------------------|--------------------------|---------|
| `RestError`          | `genossi_rest/src/lib.rs`      | `InternalError(String)`  | String direkt |
| `MailServiceError`   | `genossi_mail/src/service.rs`  | `DataAccess(Arc<str>)`   | `Arc::from(format!(...))` |
| `ConfigServiceError` | `genossi_config/src/service.rs`| `DataAccess(Arc<str>)`   | `Arc::from(format!(...))` |
| `MailTemplateError`  | `genossi_mail/src/mail_template_service.rs` | `DataAccess(Arc<str>)` | `Arc::from(format!(...))` |

Die Handler-Anpassungen in den sieben Muster-B-Stellen (siehe Decision 4) sorgen dafür, dass diese Sites einen `Result`-Kontext erhalten, in den die `?`-Operatoren propagieren können.

**Alternative A — lokales `map_err` pro Site:** Verworfen, weil es zu Stil-Inkonsistenz zwischen 45 und 7 Sites führt, die den Scope nicht rechtfertigt.

**Alternative B — zentraler `RestError` für alle Crates:** Würde die lokalen `error_handler` in `genossi_mail` und `genossi_config` abschaffen und alles nach `RestError` propagieren. Saubere Konsolidierung, aber substantiell größere Scope-Erweiterung — insbesondere würden die feingranularen Mappings in `MailTemplateError` (z. B. `DuplicateName → 409`, `VersionConflict → 409`) in einem zentralen Mapper verloren gehen oder müssten nachgebaut werden.

### Decision 4: Muster-B-Handler auf Muster A umbauen, fehlende Error-Varianten ergänzen

Sieben Handler geben `Response` direkt zurück, ohne `Result`-Wrapper. Sie werden mit dem existierenden lokalen `error_handler((async { ... }).await)`-Muster umwickelt, `match { Err(e) => error_response(e) }`-Pfade durch `?` ersetzt, Validierungsfehler (UUID-Parse, Member-Not-Found) über die passende Error-Variante propagiert.

Betroffene Handler:

| Datei                                  | Handler                | `.unwrap()`-Sites | Ziel-Error-Type      |
|----------------------------------------|------------------------|-------------------|----------------------|
| `genossi_mail/src/rest_templates.rs`   | `list_templates`       | 1                 | `MailTemplateError`  |
| `genossi_mail/src/rest_templates.rs`   | `create_template`      | 1                 | `MailTemplateError`  |
| `genossi_mail/src/rest_templates.rs`   | `get_template`         | 1                 | `MailTemplateError`  |
| `genossi_mail/src/rest_templates.rs`   | `update_template`      | 1                 | `MailTemplateError`  |
| `genossi_mail/src/rest_templates.rs`   | `delete_template`      | 0                 | `MailTemplateError`  |
| `genossi_mail/src/rest.rs`             | `preview_mail`         | 1 (Z. 464)        | `MailServiceError`   |
| `genossi_rest/src/mail_footer.rs`      | `get_footer`           | 2 (Z. 45, 74)     | `RestError`          |

`delete_template` hat keinen `.unwrap()`-Site, muss aber zwingend mit umgebaut werden, weil `error_response` in `rest_templates.rs` zu `error_handler` umgeformt wird (neue Signatur `Result<Response, MailTemplateError> -> Response`) — der bestehende `error_response(e)`-Aufruf in `delete_template` würde sonst nicht mehr kompilieren.

**Fehlende Error-Varianten für Validierungsfehler:**

Die Muster-B-Handler enthalten early-Returns für 400-Bad-Request-Fälle (UUID-Parse, Version-UUID-Parse), die auf keine existierende Variante der jeweiligen Error-Types passen:

- `MailTemplateError`: hat `NotFound`/`DuplicateName`/`VersionConflict`/`DataAccess` — keine 400-fähige Variante.
- `MailServiceError`: hat `TemplateValidation`, das auf 400 mappt — semantisch für Template-Rendering, passt nicht für "Invalid UUID".
- `RestError`: hat bereits `BadRequest(String)` — passt.

Beide Mail-Error-Types bekommen daher eine neue `BadRequest(Arc<str>)`-Variante, plus Mapping auf HTTP 400 in den jeweiligen `error_handler`-Funktionen. Das ist der kleinstmögliche Variantenzuwachs, um die semantische Eindeutigkeit der bestehenden Varianten (DataAccess → 500, TemplateValidation → 400 wegen Template-Syntax) zu erhalten.

**Folge-Effekt:** `impl From<MailServiceError> for RestError` in `genossi_rest/src/lib.rs:100-118` ist ein exhaustive Match ohne `_`-Arm und bricht beim Kompilieren, sobald die neue `BadRequest`-Variante existiert. Der Match wird um einen `BadRequest(msg) => RestError::BadRequest(msg.to_string())`-Arm ergänzt (semantisch konsistent: HTTP 400 auf beiden Seiten).

**Alternative — Validierungs-Fehler auf `TemplateValidation` mappen:** Verworfen; semantisch irreführend ("Template-Validierung" für einen UUID-Parse ist verwirrend, wenn später jemand Tracelogs liest).

**Alternative — Handler lassen wie sie sind und lokales `map_err` pro Site nutzen:** Ergibt funktional dasselbe (keine Panics), vermeidet den Refactor. Wurde in der Erkundung als Option C diskutiert und verworfen, weil der resultierende Stil-Bruch (45 Sites mit `?`, 7 Sites mit `map_err`) ohne klaren Scope-Gewinn entsteht.

### Decision 5: Scope auf REST-Handler beschränken

Angefasst werden nur Dateien, die HTTP-Handler enthalten: alle Handler-Dateien in `genossi_rest/src/` sowie `genossi_mail/src/rest.rs`, `genossi_mail/src/rest_templates.rs` und `genossi_config/src/rest.rs`. Andere `serde_json::to_string().unwrap()`-Aufrufe (z. B. in Migrations-Hilfen, Tests, Service-Impl-Layern) werden nicht angetastet. Kriterium: ist der Code-Pfad per HTTP erreichbar?

## Risks / Trade-offs

- **[Risiko] CORS-Whitelist bricht einen zukünftigen Handler mit neuer Methode (z. B. `PATCH`) stumm** → Mitigation: Requirement im Spec dokumentiert, E2E-Test in `genossi_bin/tests/e2e_tests.rs` mit einem Preflight für eine nicht erlaubte Methode.
- **[Risiko] `serde_json`-Fehler leaken im 500-Body Details** → Mitigation: Alle `error_handler` geben einen generischen `"Internal server error"`-Body aus und loggen den Detail-String nur via `tracing::error!`. Die neuen `From<serde_json::Error>`-Impls konstruieren lediglich die Log-Message.
- **[Risiko] 52 Call-Site-Änderungen + 7 Handler-Refactors → substanzieller Diff, Merge-Konflikt-Potenzial** → Mitigation: Commits pro Datei zerlegen; Handler-Refactors separat von reinen `.unwrap()→?`-Substitutionen commiten.
- **[Risiko] Handler-Refactor ändert Verhalten bei Error-Pfaden subtil** → Mitigation: Bestehende Tests für die 7 betroffenen Handler vorher auditieren, E2E-Regressionstest für je einen Error-Pfad pro umgebautem Handler.
- **[Risiko] Zwei neue `BadRequest`-Varianten in `MailServiceError` und `MailTemplateError` sind subtile API-Erweiterung** → Mitigation: Nur interne Crate-APIs betroffen (keine öffentliche Schnittstelle). Varianten folgen dem existierenden Muster (`DataAccess(Arc<str>)` etc.). Mapping in `error_handler` auf HTTP 400 dokumentiert.
- **[Trade-off] Scope wächst durch Handler-Refactor über reinen Quick-Fix hinaus** → Akzeptiert; Alternative (gemischter Stil) wurde bewusst verworfen.

## Open Questions

- **Telemetrie/Dashboards auf `stderr` aus dem Auth-Pfad?** **Annahme:** Nein, die Anwendung läuft unter `systemd`/Docker und alle Logs gehen via `tracing` in denselben Sink.
- **Unit-Test für Log-Call in `auth_middleware.rs`?** Pragmatisch: nein — `tracing`-Aufrufe werden nicht in Unit-Tests geprüft. Ausreichend: der Code kompiliert und existierende Tests bleiben grün.
