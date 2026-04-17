## Context

Das Frontend zeigt Fehlermeldungen durchgehend als rohe technische Texte an: `error_for_status_ref()` produziert reqwest-Fehlertexte wie `HTTP status client error (422 Unprocessable Entity) for url ...`, Upload-Fehler zeigen rohen JSON, und manuell formatierte Fehler nutzen `format!("{}: {}", status, text)`. Alle 11+ Pages implementieren ihr eigenes lokales Error-Handling mit `use_signal(|| None::<String>)` und individuellen `div { class: "bg-red-100 ..." }` Blöcken.

Es existiert bereits eine `ErrorView`-Komponente (`component/error_view.rs`) mit globalem `ErrorStore` (`service/error.rs`), die aber von kaum einer Page genutzt wird. Fehler-i18n-Keys existieren vereinzelt (z.B. `UploadFailed`, `ErrorLoadingData`), werden aber nicht konsistent verwendet — die meisten Fehlertexte sind hardcodiert auf Englisch.

Das Proposal definiert: strukturierter Fehlertyp statt `Result<T, String>`, zentrales HTTP-Status-Mapping, wiederverwendbare Error-Komponente, und i18n-Integration.

## Goals / Non-Goals

**Goals:**
- Benutzerfreundliche, deutschsprachige Fehlermeldungen für alle HTTP-Fehlerfälle
- Strukturierter Fehlertyp `AppError` der HTTP-Status, User-Message und technisches Detail trennt
- Zentrales Mapping in `api.rs` — keine Fehler-Interpretation in Pages
- Wiederverwendbare `ErrorAlert`-Komponente mit optionalem Details-Aufklappfeld
- i18n-Integration für Fehlermeldungen (De + En)
- Technische Details bleiben zugänglich für Bug-Reports

**Non-Goals:**
- Backend-Änderungen — das Backend liefert bereits strukturierte Fehler
- Toast/Snackbar-Notifications — Fehler bleiben inline auf der jeweiligen Page
- Retry-Logik oder automatische Fehlerbehandlung
- Globaler Error-Boundary (Pages behalten lokale Fehler-Signals)
- Tschechische Übersetzungen (kein `cs.rs` vorhanden, trotz CLAUDE.md-Erwähnung)

## Decisions

### 1. Neuer `AppError`-Typ statt Wiederverwendung von `ErrorStore`

Der bestehende `ErrorStore` ist ein globaler Signal mit `Option<String>` — zu simpel für die Anforderungen (kein HTTP-Status, keine strukturierten Details). Statt ihn aufzublähen, wird ein neuer `AppError`-Struct in `api.rs` eingeführt:

```rust
pub struct AppError {
    pub status: Option<u16>,
    pub message: String,        // User-facing, i18n
    pub detail: Option<String>, // Technischer Text für Details-Aufklapper
}
```

**Warum nicht `ErrorStore` erweitern?** Der globale Store passt nicht zum bestehenden Page-Pattern (`use_signal` pro Page). Ein Struct-Typ ist flexibler und erfordert keine Architekturänderung. Der alte `ErrorStore` / `ErrorView` wird nicht gelöscht, aber nicht weiter genutzt — Pages migrieren auf `AppError` + `ErrorAlert`.

**Alternative verworfen:** `anyhow::Error` mit Kontext — zu generisch, kein HTTP-Status-Zugriff, keine i18n-Integration.

### 2. Zentrales Fehler-Mapping via `map_response_error()`

Eine Funktion `map_response_error(response: &Response) -> AppError` in `api.rs` extrahiert Status und Body und mappt auf benutzerfreundliche Meldungen. Das Mapping:

| Status | User-Message (i18n-Key) | Detail |
|--------|------------------------|--------|
| 400 | "Ungültige Anfrage" | Body-Text |
| 401 | "Keine Berechtigung — bitte erneut anmelden" | — |
| 403 | "Keine Berechtigung für diese Aktion" | — |
| 404 | "Nicht gefunden" | — |
| 409 | "Konflikt — das Element wurde zwischenzeitlich geändert" | Body-Text |
| 415 | Parsed: "Dateityp nicht erlaubt. Erlaubt: pdf, png, ..." | Roher JSON |
| 422 | "Validierungsfehler" | Body-Text (Feld-Details) |
| 429 | "Zu viele Anfragen — bitte warten" | — |
| 500+ | "Serverfehler — bitte später erneut versuchen" | Body-Text |
| Netzwerk | "Verbindungsfehler — bitte Internetverbindung prüfen" | reqwest-Error |

**Warum zentral statt pro Aufruf?** Eliminiert Duplikation. Spezial-Cases (z.B. 409 bei Dokumenten-Upload hat spezifischere Meldung) können über optionalen Context-Parameter gelöst werden.

**Alternative verworfen:** Error-Mapping in jeder API-Funktion einzeln — genau das existiert heute und ist das Problem.

### 3. Neue `ErrorAlert`-Komponente statt `ErrorView`-Erweiterung

Neue Komponente `ErrorAlert` in `component/error_alert.rs`:

```rust
#[component]
pub fn ErrorAlert(error: AppError, on_dismiss: Option<EventHandler<()>>) -> Element
```

- Zeigt `error.message` in rotem Alert-Banner (konsistentes Styling)
- Optional: Aufklappbarer "Details"-Bereich mit `error.detail`
- Optional: Dismiss-Button (X) wenn `on_dismiss` gesetzt
- Nutzt i18n für statische Texte ("Details anzeigen", "Schließen")

**Warum nicht `ErrorView` erweitern?** `ErrorView` ist an `ERROR_STORE` (global) gebunden. `ErrorAlert` nimmt Props und ist damit flexibel einsetzbar — passt zum bestehenden lokalen `use_signal`-Pattern der Pages.

### 4. API-Funktionen geben `Result<T, AppError>` zurück

Alle öffentlichen Funktionen in `api.rs` ändern ihren Error-Typ von `reqwest::Error` / `String` zu `AppError`. Die Migration passiert Funktion für Funktion:

- `error_for_status_ref()?` wird ersetzt durch explizite Status-Prüfung + `map_response_error()`
- Manuelle `format!`-Fehler werden durch `AppError::new()` ersetzt
- Pages ändern `error: Signal<Option<String>>` zu `error: Signal<Option<AppError>>`

### 5. i18n-Keys für Fehlermeldungen

Neue Keys im `Key`-Enum für jede Fehlerkategorie. Die `map_response_error()`-Funktion nutzt `i18n.t(Key::ErrorXxx)` für User-Messages. Bestehende Error-Keys (z.B. `UploadFailed`, `ErrorLoadingData`) werden wo sinnvoll weiterverwendet.

Da `i18n` ein Context-Signal ist und `map_response_error()` in async-Funktionen läuft (kein Component-Kontext), speichert `AppError` den i18n-Key als String. Die Übersetzung passiert in der `ErrorAlert`-Komponente beim Rendern — nicht beim Erstellen des Fehlers.

**Korrektur:** Da das i18n-System Keys zu Strings zur Compile-/Render-Zeit auflöst und wir in async-Kontexten ohne Component-Zugang arbeiten, wird `map_response_error()` die Messages direkt als statische Strings zurückgeben (deutsch als Default, da Hauptzielgruppe). Die i18n-Integration für Fehlermeldungen kann als Follow-up erfolgen, wenn das i18n-System async-fähig wird.

**Pragmatischer Ansatz:** Fehlermeldungen werden zunächst als deutsche Strings in `map_response_error()` hartcodiert. Englische Fallbacks über i18n-Keys können in einem Follow-up ergänzt werden.

## Risks / Trade-offs

**[Breite Migration]** Alle 11+ Pages müssen angepasst werden (Error-Signal-Typ + Rendering). → Mitigation: Schrittweise Migration, `AppError` implementiert `Display` als Fallback damit alte `format!("{}", e)` Patterns weiter kompilieren.

**[Async-Kontext ohne i18n]** `map_response_error()` läuft in async ohne Zugang zu Dioxus-Hooks. → Mitigation: Deutsche Strings direkt in der Mapping-Funktion, i18n als Follow-up.

**[Body-Parsing kann fehlschlagen]** `response.text().await` bei 415 kann unerwartetes Format haben. → Mitigation: Fallback auf generische Meldung wenn JSON-Parsing scheitert.

**[Bestehende Tests]** Einige Tests prüfen auf spezifische Error-Strings. → Mitigation: Tests auf `AppError`-Felder umstellen (Status-Code statt String-Match).
