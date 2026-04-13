## Context

Genossi ist eine REST-API für Genossenschaftsverwaltung mit Layered Architecture (DAO → Service → REST). Seit 01.01.2025 erlaubt § 15 GenG Beitrittserklärungen in Textform (BEG IV). Die Genossenschaft betreibt eine WordPress-Seite und möchte dort ein Beitrittsformular anbieten, das serverseitig (PHP → Genossi API) Anträge übermittelt.

Aktuell gibt es einen öffentlichen Endpunkt (`/api/public/member-count`) als Referenz-Pattern. Mitglieder werden über `POST /api/members` mit Authentifizierung angelegt. Beitrittserklärungen sind ein neues Konzept – sie sollen separat von Mitgliedern gespeichert werden, bis ein Admin nach Geldeingang die Mitgliedschaft bestätigt.

## Goals / Non-Goals

**Goals:**
- Öffentlicher API-Endpunkt für Beitrittserklärungen mit API-Key-Authentifizierung
- Eigene Application-Entität (DAO/Service/REST) getrennt von Members
- Automatische Bestätigungs-Mail mit Überweisungsdaten nach Eingang
- Admin-Workflow: Offene Anträge einsehen, bestätigen (→ Member anlegen), ablehnen
- Config-Store-Einträge für API-Key, Anteilswert, Bankdaten (API-Key auto-generierbar)

**Non-Goals:**
- WordPress-Plugin (separater Change)
- Admin-Frontend/UI für Antragsverwaltung (separater Change)
- Automatische Zahlungserkennung (manueller Prozess)
- Double-Opt-In E-Mail-Verifikation
- Willkommens-Mail nach Bestätigung (kann später ergänzt werden)

## Decisions

### 1. Eigene Application-Entität statt Member-Status

**Entscheidung:** Beitrittserklärungen werden in einer eigenen `applications`-Tabelle gespeichert, nicht als Member mit Status "Beantragt".

**Alternativen:**
- *Neuer MemberStatus "Beantragt"*: Würde Mitgliederliste verschmutzen, Mitgliedsnummer müsste reserviert werden, Ablehnung = "gelöschtes Mitglied"
- *Eigene Entität*: Saubere Trennung, Mitgliedsnummer erst bei Bestätigung, eigener Lifecycle

**Begründung:** Anträge sind konzeptionell keine Mitglieder. Eine Mitgliedsnummer wird erst bei Geldeingang vergeben. Abgelehnte Anträge sollen nicht als gelöschte Mitglieder erscheinen.

### 2. API-Key-Authentifizierung für öffentlichen Endpunkt

**Entscheidung:** Der Endpunkt `POST /api/public/join` wird durch einen API-Key im `X-Api-Key`-Header geschützt, nicht durch User-Authentifizierung.

**Alternativen:**
- *Kein Schutz*: Zu riskant, jeder könnte Anträge spammen
- *User-Login*: Nicht möglich, Antragsteller haben keinen Account
- *API-Key*: WordPress-Server kennt den Key, Browser nie

**Begründung:** Der WordPress-Server macht den Call serverseitig. Der API-Key ist nie im Browser sichtbar. Wird im Config-Store als `secret` gespeichert.

### 3. Application-Entität folgt bestehendem DAO-Pattern

**Entscheidung:** `ApplicationEntity` bekommt `id`, `created`, `deleted`, `version` wie alle anderen Entitäten. Dazu `status` (Enum: Offen, Bestätigt, Abgelehnt).

**Begründung:** Konsistenz mit bestehendem Code. Die Macros und Patterns sind erprobt.

### 4. Bestätigung erzeugt Member über bestehenden Service

**Entscheidung:** `POST /api/applications/{id}/confirm` ruft intern die bestehende Member-Erstellungslogik auf (nächste freie Mitgliedsnummer, Eintritt + Aufstockung Aktionen).

**Alternativen:**
- *Eigene Erstellungslogik*: Dupliziert Code
- *Bestehenden Service nutzen*: Wiederverwendung, konsistentes Verhalten

**Begründung:** Der `join_date` wird auf das Bestätigungsdatum gesetzt. `shares_at_joining` kommt aus der Application. Der Rest folgt dem normalen Flow.

### 5. Mail über bestehende Infrastruktur

**Entscheidung:** Die Bestätigungs-Mail nutzt die vorhandene Mail-Queue/Worker-Infrastruktur. Da der Empfänger kein Mitglied ist, wird die Mail ohne `member_id` in die Queue gestellt (Subject und Body werden vor dem Einstellen gerendert, nicht vom Worker).

**Begründung:** Keine Duplizierung der SMTP-Logik. Der Worker kann die Mail wie jede andere versenden. Template-Rendering findet im Service statt, da kein Member-Kontext für den Worker verfügbar ist.

### 6. Config-Einträge mit Auto-Generierung

**Entscheidung:** Neuer Admin-Endpunkt `POST /api/config/generate-api-key` erzeugt einen UUID v4 und speichert ihn als `public_api_key` (Typ `secret`) im Config-Store. Anteilswert und Bankdaten sind reguläre Config-Einträge.

Config-Keys:
| Key | Typ | Beispiel |
|---|---|---|
| `public_api_key` | secret | `a1b2c3d4-...` |
| `share_value_cents` | int | `5000` |
| `bank_iban` | string | `DE89 3704 0044 ...` |
| `bank_name` | string | `GLS Bank` |
| `bank_bic` | string | `GENODEM1GLS` |
| `genossenschaft_name` | string | `Muster eG` |

## Risks / Trade-offs

- **[Spam-Risiko]** → API-Key schützt den Endpunkt. Zusätzlich Rate-Limiting auf WordPress-Seite (Captcha). Kein serverseitiges Rate-Limiting in Genossi (Non-Goal für diesen Change).
- **[Mail-Fehler]** → Wenn die Bestätigungs-Mail fehlschlägt, wird der Antrag trotzdem gespeichert. Admin kann die Mail manuell nachsenden oder den Antragsteller kontaktieren.
- **[Konfiguration vergessen]** → Wenn Config-Einträge (SMTP, Bankdaten) fehlen, schlägt der Endpunkt mit 500 fehl. Der Endpunkt sollte die Konfiguration validieren und eine verständliche Fehlermeldung liefern.
- **[Doppelte Anträge]** → Keine Duplikatserkennung (gleicher Name/E-Mail könnte mehrfach einreichen). Akzeptabel, da Admin ohnehin manuell prüft.
