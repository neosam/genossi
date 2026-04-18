## ADDED Requirements

### Requirement: CORS Origin-Allowlist

Das System SHALL CORS-Requests nur von explizit erlaubten Origins zulassen. Die Allowlist SHALL beim Server-Start aus zwei Quellen gebaut werden:
1. Der Origin aus der Env-Variable `BASE_PATH` (Default).
2. Optional zusätzliche Origins aus dem Config-Store unter Key `cors_allowed_origins` (comma-separated).

Bei Requests von nicht erlaubten Origins SHALL der Server keine `Access-Control-Allow-Origin`-Header setzen, wodurch der Browser den Cross-Origin-Request blockiert.

#### Scenario: Request von der eigenen Origin

- **WHEN** ein Browser einen Request mit `Origin: <BASE_PATH>` sendet
- **THEN** die Response enthält den Header `Access-Control-Allow-Origin: <BASE_PATH>`

#### Scenario: Request von einer konfigurierten zusätzlichen Origin

- **WHEN** `cors_allowed_origins` im Config-Store den Wert `https://partner.example.org` enthält UND ein Request mit `Origin: https://partner.example.org` eintrifft
- **THEN** die Response enthält den Header `Access-Control-Allow-Origin: https://partner.example.org`

#### Scenario: Request von einer fremden Origin

- **WHEN** ein Request mit `Origin: https://evil.example.com` eintrifft, die weder in `BASE_PATH` noch in `cors_allowed_origins` steht
- **THEN** die Response enthält keinen `Access-Control-Allow-Origin`-Header für diese Origin

### Requirement: Security-Response-Header

Das System SHALL auf jede HTTP-Response die folgenden Header setzen (sofern nicht bereits vorhanden):

- `Strict-Transport-Security: max-age=63072000; includeSubDomains`
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()`

#### Scenario: Beliebiger API-Response

- **WHEN** ein beliebiger erfolgreicher API-Call beantwortet wird
- **THEN** die Response enthält alle fünf oben genannten Header mit den festgelegten Werten

#### Scenario: Fehler-Response

- **WHEN** ein API-Call mit 4xx oder 5xx beantwortet wird
- **THEN** die Response enthält ebenfalls alle fünf Security-Header

### Requirement: Rate-Limiting auf /authenticate

Das System SHALL Requests auf den Endpoint `/authenticate` pro Quell-IP auf maximal 10 pro Minute begrenzen. Überschreitungen SHALL mit HTTP 429 und einem `Retry-After`-Header beantwortet werden.

#### Scenario: Normaler Login-Flow

- **WHEN** eine IP innerhalb einer Minute bis zu 10 Requests auf `/authenticate` sendet
- **THEN** jeder Request wird normal verarbeitet

#### Scenario: Rate-Limit überschritten

- **WHEN** eine IP in einer Minute den 11. Request auf `/authenticate` sendet
- **THEN** der Server antwortet mit HTTP 429 und einem `Retry-After`-Header

### Requirement: Rate-Limiting auf /join

Das System SHALL Requests auf den Endpoint `POST /api/public/join` pro Quell-IP auf maximal 5 pro Minute begrenzen. Überschreitungen SHALL mit HTTP 429 und einem `Retry-After`-Header beantwortet werden.

#### Scenario: Normale WordPress-Submission

- **WHEN** WordPress in einer Minute bis zu 5 Beitrittsanträge an `/api/public/join` sendet
- **THEN** jeder Antrag wird normal verarbeitet

#### Scenario: Rate-Limit überschritten

- **WHEN** von einer IP in einer Minute der 6. Request auf `/join` eintrifft
- **THEN** der Server antwortet mit HTTP 429 und einem `Retry-After`-Header

### Requirement: Globales Rate-Limit

Das System SHALL alle API-Requests (Pfad-Präfix `/api/`) pro Quell-IP auf maximal 60 pro Minute begrenzen. Statische Frontend-Assets und der `/api/public/member-count`-Endpoint SHALL von diesem Limit ausgenommen sein.

#### Scenario: Normale Frontend-Nutzung

- **WHEN** ein authentifizierter Browser den initialen Seitenaufbau macht und 20 API-Calls in 10 Sekunden triggert
- **THEN** alle Calls werden normal bedient

#### Scenario: Burst-Angriff

- **WHEN** eine IP innerhalb einer Minute 61 Requests auf `/api/*`-Routen sendet
- **THEN** Requests ab dem 61. werden mit HTTP 429 abgelehnt

## MODIFIED Requirements

_(Keine — dies ist eine neue Capability. `/join`-spezifische Änderungen sind in der `membership-application`-Capability-Delta.)_
