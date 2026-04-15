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

**Entscheidung:** Der Endpunkt nutzt die bestehende Auth-Middleware und erfordert `manage_members`. Der Request-Body ist ein neuer Type `AdminCreateApplicationRequest` mit reduzierten Pflichtfeldern: nur `first_name`, `last_name`, `shares` sind Pflicht (wie bei Mitgliedern). Alle anderen Felder (email, street, house_number, postal_code, city, salutation) sind optional. Dazu `send_mail: Option<bool>` (default `false` bei `None`). Wenn `send_mail: true` aber keine E-Mail angegeben → 422.

**Alternativen:**
- *`PublicJoinRequest` wiederverwenden*: Hat zu viele Pflichtfelder für den Admin-Use-Case (Papieranträge haben oft keine E-Mail/Adresse)
- *Gleicher Request-Type mit optionalem Feld*: Würde den Public-Endpunkt aufweichen

**Begründung:** Der Admin-Endpunkt spiegelt die Realität wider: bei Papieranträgen hat man oft nur Name und Anteile. Die Application-Entity muss dafür ebenfalls optionale Felder unterstützen (email, Adressfelder werden `Option`).

### 3. Service-Methode weiterhin ohne Auth-Context

**Entscheidung:** `submit()` bleibt ohne Auth-Context (wird vom öffentlichen Endpunkt ohne Login aufgerufen). Die Autorisierung (`manage_members`) wird im REST-Handler des Admin-Endpunkts geprüft, bevor `submit()` aufgerufen wird.

**Begründung:** Der öffentliche Endpunkt hat keinen Auth-Context. Die Berechtigung am REST-Layer zu prüfen ist konsistent mit dem bestehenden Pattern, wo der Public-Endpunkt über API-Key authentifiziert.

### 4. Frontend: Modal-Formular

**Entscheidung:** Button "Antrag anlegen" auf der Applications-Seite öffnet ein Modal mit den Feldern. Der Toggle "Bestätigungs-Mail senden" ist standardmäßig deaktiviert.

**Begründung:** Konsistent mit dem bestehenden UI-Pattern (Bestätigen/Ablehnen nutzen bereits Modals). Kein Seitenwechsel nötig.

### 5. Application-Entity: Felder werden optional

**Entscheidung:** In `ApplicationEntity` und `ApplicationSubmission` werden `email`, `street`, `house_number`, `postal_code`, `city` von `Arc<str>` zu `Option<Arc<str>>` geändert. Die Datenbank-Migration ändert die Spalten-Constraints (NOT NULL → nullable). Der öffentliche Endpunkt validiert diese Felder weiterhin als Pflicht im REST-Handler.

**Alternativen:**
- *Zwei verschiedene Entity-Typen*: Zu viel Duplizierung
- *Nur im Request optional, in der DB Pflicht mit Leerstring*: Semantisch falsch, "kein Wert" ≠ ""

**Begründung:** Die Quelle der Wahrheit (Application-Entity) soll abbilden, dass diese Felder tatsächlich fehlen können. Die Pflichtfeld-Validierung bleibt endpunkt-spezifisch (Public = strenger, Admin = lockerer).

## Risks / Trade-offs

- **[Signaturänderung]** → `submit()` bekommt einen neuen Parameter. Bestehende Tests und der öffentliche Endpunkt müssen angepasst werden. Überschaubar, da es nur eine Aufrufstelle gibt + Tests.
- **[Entity-Änderung]** → Felder in `ApplicationEntity` werden optional. Bestehende Daten (alle über WordPress angelegt) haben alle Felder befüllt, daher ist die Migration unkritisch. Code, der auf diese Felder zugreift (z.B. `confirm` bei Member-Erstellung), muss mit `Option` umgehen.
- **[Kein Auth-Check im Service]** → Der Admin-Endpunkt prüft Berechtigungen im REST-Handler. Wenn jemand `submit()` von einem anderen Ort aufruft, gibt es keinen Permission-Check. Akzeptabel, da `submit()` auch für den öffentlichen Endpunkt gedacht ist.
