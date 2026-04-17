## ADDED Requirements

### Requirement: Structured error type for API calls

All public functions in `api.rs` SHALL return `Result<T, AppError>` where `AppError` contains:
- `status`: Optional HTTP status code (`Option<u16>`)
- `message`: User-facing error message in German
- `detail`: Optional technical detail text for debugging

`AppError` SHALL implement `Display` (returning `message`) and `std::error::Error`.

#### Scenario: API function returns AppError on HTTP error
- **WHEN** an API function receives an HTTP error response (e.g. 422)
- **THEN** it SHALL return an `AppError` with the HTTP status code, a user-facing message, and the response body as detail

#### Scenario: API function returns AppError on network error
- **WHEN** an API function fails due to a network error (no response received)
- **THEN** it SHALL return an `AppError` with `status: None`, message "Verbindungsfehler", and the reqwest error text as detail

#### Scenario: AppError Display implementation
- **WHEN** `AppError` is formatted via `Display`
- **THEN** it SHALL output the `message` field

### Requirement: Central HTTP status-to-message mapping

A function `map_response_error(response: Response) -> AppError` in `api.rs` SHALL map HTTP status codes to user-facing German messages:

| Status | Message |
|--------|---------|
| 400 | "Ungültige Anfrage" |
| 401 | "Keine Berechtigung — bitte erneut anmelden" |
| 403 | "Keine Berechtigung für diese Aktion" |
| 404 | "Nicht gefunden" |
| 409 | "Konflikt — das Element wurde zwischenzeitlich geändert" |
| 415 | Parsed from response body (see special handling) |
| 422 | "Validierungsfehler" |
| 429 | "Zu viele Anfragen — bitte warten" |
| 500+ | "Serverfehler — bitte später erneut versuchen" |

The function SHALL read the response body and store it as `detail`.

#### Scenario: Known HTTP status maps to German message
- **WHEN** `map_response_error` receives a response with status 403
- **THEN** it SHALL return an `AppError` with status `Some(403)`, message "Keine Berechtigung für diese Aktion"

#### Scenario: Unknown client error status
- **WHEN** `map_response_error` receives a response with status 418
- **THEN** it SHALL return an `AppError` with status `Some(418)` and a generic message "Unbekannter Fehler"

#### Scenario: Server error status
- **WHEN** `map_response_error` receives a response with status 502
- **THEN** it SHALL return an `AppError` with message "Serverfehler — bitte später erneut versuchen"

### Requirement: Special handling for 415 file type errors

When `map_response_error` receives a 415 response, it SHALL attempt to parse the body as JSON with fields `error` and `allowed_extensions`. If parsing succeeds, the message SHALL read: "Dateityp nicht erlaubt. Erlaubte Typen: pdf, png, jpg, ..." (listing the extensions as a comma-separated readable list). If parsing fails, the message SHALL fall back to "Dateityp nicht erlaubt".

#### Scenario: 415 with parseable JSON body
- **WHEN** `map_response_error` receives status 415 with body `{"error":"File type 'exe' is not allowed","allowed_extensions":["pdf","png","jpg"]}`
- **THEN** the message SHALL be "Dateityp nicht erlaubt. Erlaubte Typen: pdf, png, jpg"

#### Scenario: 415 with unparseable body
- **WHEN** `map_response_error` receives status 415 with body "Unsupported Media Type"
- **THEN** the message SHALL be "Dateityp nicht erlaubt"

### Requirement: ErrorAlert component

An `ErrorAlert` component in `component/error_alert.rs` SHALL display error messages to the user. It SHALL accept an `AppError` and an optional dismiss callback as props.

The component SHALL:
- Display `error.message` in a red alert banner
- Show an expandable "Details" section when `error.detail` is present
- Show a dismiss button (X) when an `on_dismiss` handler is provided
- Use consistent Tailwind styling (red background/border/text)

#### Scenario: Error with message only
- **WHEN** `ErrorAlert` receives an `AppError` with message "Keine Berechtigung" and no detail
- **THEN** it SHALL display "Keine Berechtigung" in a red alert banner with no details section

#### Scenario: Error with detail
- **WHEN** `ErrorAlert` receives an `AppError` with message "Validierungsfehler" and detail "field 'email' is invalid"
- **THEN** it SHALL display "Validierungsfehler" with a collapsible "Details" section containing the technical text

#### Scenario: Dismissible error
- **WHEN** `ErrorAlert` receives an `on_dismiss` handler and the user clicks the dismiss button
- **THEN** the component SHALL invoke the `on_dismiss` callback

#### Scenario: Non-dismissible error
- **WHEN** `ErrorAlert` receives no `on_dismiss` handler
- **THEN** the dismiss button SHALL NOT be rendered

### Requirement: Pages use ErrorAlert with AppError

All pages that display errors SHALL use the `ErrorAlert` component with `Signal<Option<AppError>>` instead of inline error divs with `Signal<Option<String>>`. The error signal SHALL be set by API call results and cleared on dismiss or on successful retry.

#### Scenario: Page displays API error via ErrorAlert
- **WHEN** a page calls an API function that returns `Err(app_error)`
- **THEN** the page SHALL set its error signal to `Some(app_error)` and render `ErrorAlert`

#### Scenario: Page clears error on dismiss
- **WHEN** the user dismisses an error via the ErrorAlert dismiss button
- **THEN** the page SHALL set its error signal to `None`

#### Scenario: Page clears error on successful operation
- **WHEN** a page performs a successful API operation after a previous error
- **THEN** the page SHALL set its error signal to `None`
