## ADDED Requirements

### Requirement: CORS Method- und Header-Whitelist

Das System SHALL für CORS-Preflight-Responses nur eine explizite Whitelist an HTTP-Methoden und Request-Headern erlauben, nicht `Access-Control-Allow-Methods: *` oder `Access-Control-Allow-Headers: *`.

- Erlaubte Methoden: `GET`, `POST`, `PUT`, `DELETE`, `OPTIONS`
- Erlaubte Request-Header: `Content-Type`, `Authorization`, `Cookie`

#### Scenario: Preflight für erlaubte Methode

- **WHEN** ein Browser einen CORS-Preflight-`OPTIONS`-Request für einen `POST` mit `Access-Control-Request-Method: POST` sendet
- **THEN** die Response enthält `Access-Control-Allow-Methods` mit den fünf erlaubten Methoden und keinen `*`-Wildcard

#### Scenario: Preflight für erlaubten Request-Header

- **WHEN** ein Preflight-Request `Access-Control-Request-Headers: Content-Type, Authorization` ankündigt
- **THEN** die Response enthält `Access-Control-Allow-Headers` mit `Content-Type, Authorization, Cookie` und keinen `*`-Wildcard

#### Scenario: Preflight für nicht erlaubte Methode

- **WHEN** ein Preflight-Request eine Methode außerhalb der Whitelist anfordert (z.B. `PATCH`)
- **THEN** die Response enthält diese Methode nicht in `Access-Control-Allow-Methods`, der Browser blockiert den nachfolgenden Request
