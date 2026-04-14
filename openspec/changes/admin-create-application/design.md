## Context

Genossi hat bereits eine vollständige Application-Verwaltung: öffentlicher Endpunkt (`POST /api/public/join`) zum Anlegen, Admin-Endpunkte zum Auflisten, Bestätigen und Ablehnen, und eine Frontend-Seite zur Verwaltung. Die `submit()`-Methode im Service erstellt die Application und verschickt immer eine Bestätigungs-Mail. Es gibt aktuell keinen Weg, eine Application ohne Mail-Versand anzulegen.

Relevante Dateien:
- `genossi_service/src/application.rs` — Trait `ApplicationService` mit `submit()`
- `genossi_service_impl/src/application.rs` — Implementierung mit `send_confirmation_mail()`
- `genossi_rest/src/application.rs` — REST-Handler inkl. `public_join`
- `genossi_rest_types/src/lib.rs` — `PublicJoinRequest`, `ApplicationTO`
- `genossi-frontend/src/page/applications_page.rs` — Frontend-Seite

## Goals / Non-Goals

**Goals:**
- Admin-Endpunkt `POST /api/applications` zum manuellen Anlegen von Applications
- Optionaler `send_mail`-Parameter (default: `false`)
- Frontend-Formular mit Toggle "Bestätigungs-Mail senden" (default: aus)

**Non-Goals:**
- Änderung am Verhalten des öffentlichen Endpunkts (`POST /api/public/join`)
- Bearbeiten existierender Applications
- Bulk-Anlegen mehrerer Applications

## Decisions

### 1. `submit()` um `send_mail: bool` erweitern

**Entscheidung:** Die Signatur von `ApplicationService::submit()` wird um einen `send_mail: bool`-Parameter erweitert. Der öffentliche Endpunkt ruft `submit(data, true)` auf, der neue Admin-Endpunkt `submit(data, false)` (bzw. den Wert aus dem Request).

**Alternativen:**
- *Separate `create()`-Methode*: Würde die Validierung und DB-Logik duplizieren
- *Options-Struct statt Bool*: Overengineering für einen einzigen Parameter

**Begründung:** Minimale Änderung, keine Code-Duplizierung. Ein Bool ist klar und direkt.

### 2. Neuer Admin-Endpunkt `POST /api/applications`

**Entscheidung:** Der Endpunkt nutzt die bestehende Auth-Middleware und erfordert `manage_members`. Der Request-Body ist ein neuer Type `AdminCreateApplicationRequest` mit denselben Feldern wie `PublicJoinRequest` plus `send_mail: Option<bool>` (default `false` bei `None`).

**Alternativen:**
- *`PublicJoinRequest` wiederverwenden + Query-Param für send_mail*: Unüblich, Body ist besser
- *Gleicher Request-Type mit optionalem Feld*: Möglich, aber eigener Type ist expliziter

**Begründung:** Ein eigener Request-Type dokumentiert klar, dass dies ein Admin-Endpunkt ist, und erlaubt künftige Admin-spezifische Felder ohne den Public-Type zu verändern.

### 3. Service-Methode weiterhin ohne Auth-Context

**Entscheidung:** `submit()` bleibt ohne Auth-Context (wird vom öffentlichen Endpunkt ohne Login aufgerufen). Die Autorisierung (`manage_members`) wird im REST-Handler des Admin-Endpunkts geprüft, bevor `submit()` aufgerufen wird.

**Begründung:** Der öffentliche Endpunkt hat keinen Auth-Context. Die Berechtigung am REST-Layer zu prüfen ist konsistent mit dem bestehenden Pattern, wo der Public-Endpunkt über API-Key authentifiziert.

### 4. Frontend: Modal-Formular

**Entscheidung:** Button "Antrag anlegen" auf der Applications-Seite öffnet ein Modal mit den Feldern. Der Toggle "Bestätigungs-Mail senden" ist standardmäßig deaktiviert.

**Begründung:** Konsistent mit dem bestehenden UI-Pattern (Bestätigen/Ablehnen nutzen bereits Modals). Kein Seitenwechsel nötig.

## Risks / Trade-offs

- **[Signaturänderung]** → `submit()` bekommt einen neuen Parameter. Bestehende Tests und der öffentliche Endpunkt müssen angepasst werden. Überschaubar, da es nur eine Aufrufstelle gibt + Tests.
- **[Kein Auth-Check im Service]** → Der Admin-Endpunkt prüft Berechtigungen im REST-Handler. Wenn jemand `submit()` von einem anderen Ort aufruft, gibt es keinen Permission-Check. Akzeptabel, da `submit()` auch für den öffentlichen Endpunkt gedacht ist.
