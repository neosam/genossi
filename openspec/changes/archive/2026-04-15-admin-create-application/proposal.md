## Why

Beitrittserklärungen können aktuell nur über den öffentlichen Endpunkt (`POST /api/public/join`) angelegt werden, der für die WordPress-Anbindung gedacht ist und immer eine Bestätigungs-Mail verschickt. Für Anträge, die per Papier, telefonisch oder auf anderem Weg eingehen, fehlt eine Möglichkeit, sie manuell im System zu erfassen -- ohne dass automatisch eine Mail rausgeht.

## What Changes

- **Neuer Admin-Endpunkt** `POST /api/applications`: Erstellt eine Application. Pflichtfelder nur `first_name`, `last_name`, `shares` (analog zur Mitgliederliste). Alle anderen Felder (email, Adresse, salutation) sind optional. Erfordert `manage_members`-Berechtigung. Optionaler Parameter `send_mail` (default: `false`) steuert, ob die Bestätigungs-Mail verschickt wird (erfordert E-Mail).
- **Service-Erweiterung**: `submit()` erhält einen `send_mail: bool`-Parameter. Der öffentliche Endpunkt ruft `submit(data, true)` auf, der Admin-Endpunkt `submit(data, false)`.
- **Frontend-Formular**: Button "Antrag manuell anlegen" auf der Applications-Seite. Modal mit Eingabefeldern (Name, Adresse, E-Mail, Anteile) und einem Toggle "Bestätigungs-Mail senden" (standardmäßig aus).

## Capabilities

### New Capabilities
<!-- Keine neuen Capabilities -- die Änderung erweitert bestehende. -->

### Modified Capabilities
- `membership-application`: Neues Requirement für authentifizierten Admin-Endpunkt zum manuellen Anlegen von Applications mit optionaler Mail-Versendung.

## Impact

- **Backend**: Signaturänderung an `ApplicationService::submit()` (breaking für Trait-Implementierungen), neuer REST-Handler, neuer Request-Type
- **Frontend**: Neues Formular-Modal und API-Call auf der Applications-Seite
- **Bestehender öffentlicher Endpunkt**: Ruft `submit()` weiterhin mit `send_mail: true` auf -- Verhalten ändert sich nicht
