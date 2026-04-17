# HTTP-Perimeter

## CORS

CORS ist auf eine Allowlist beschränkt. Erlaubte Origins:

1. **`BASE_PATH`** (Env-Variable) — wird automatisch als Default-Origin genutzt
2. **`cors_allowed_origins`** (Config-Store) — komma-separierte Liste zusätzlicher Origins

Nicht erlaubte Origins erhalten keinen `Access-Control-Allow-Origin`-Header.

**Wichtig:** Änderungen an `cors_allowed_origins` erfordern einen Server-Restart.

## Rate-Limiting

Rate-Limits sind per IP (Token Bucket via `tower-governor`):

| Route | Limit | Zweck |
|-------|-------|-------|
| `/authenticate` | 10 req/min | Schutz gegen OIDC-Loop-Abuse |
| `/api/public/join` | 5 req/min | Schutz gegen Spam-Anträge |
| `/api/*` (global) | 60 req/min | Allgemeiner Schutz |
| `/api/public/member-count` | kein Limit | Hat bereits 5-Min-Cache |
| Statische Assets | kein Limit | Frontend/Dioxus |

Bei Überschreitung: HTTP 429 mit `Retry-After`-Header.

**Hinweis:** Rate-Limit-State ist in-process. Bei Restart wird der State zurückgesetzt.

## Security-Header

Jede HTTP-Response enthält:

- `Strict-Transport-Security: max-age=63072000; includeSubDomains`
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()`

## /join Endpoint-Härtung

- API-Key-Vergleich erfolgt in konstanter Zeit (`constant_time_eq`)
- Input-Validierung mit Längenlimits und Email-Format-Prüfung
- Validierungsfehler werden als HTTP 422 mit `{"errors": [{"field": "...", "message": "..."}]}` zurückgegeben
