## Why

Aus der Mitglieds-Detailseite gibt es heute keinen direkten Weg, dem Mitglied eine Mail zu schreiben. Der Admin muss zur Mitgliederliste zurück, das Mitglied dort markieren und dann auf „Mail senden" klicken — drei Schritte für eine Aktion, die im Kontext „ich schaue mir gerade dieses Mitglied an" naheliegend ist. Das nötige Pattern existiert bereits: Der globale Signal `SELECTED_MEMBER_IDS` und die Navigation auf `/mail` werden auf der Mitgliederliste schon genau so genutzt.

## What Changes

- Auf der Mitglieds-Detailseite (`member_details.rs`) erscheint ein Knopf „Mail senden" im Bereich der Aktionen.
- Beim Klick werden die `SELECTED_MEMBER_IDS` auf genau dieses eine Mitglied gesetzt und auf `/mail` navigiert.
- Der Knopf ist nur klickbar, wenn das Mitglied eine E-Mail-Adresse hinterlegt hat. Andernfalls ist er disabled mit einem Hinweis (Tooltip oder Text).

## Capabilities

### New Capabilities
- `mail-from-member-detail`: Direkter Sprung von der Mitglieds-Detailseite in den Mail-Versand mit dem aktuellen Mitglied als Empfänger.

### Modified Capabilities
<!-- keine - bestehende Mail-Capabilities (mail-compose-components, mail-sending) bleiben unverändert; sie werden nur über einen weiteren Eingangspunkt genutzt -->

## Impact

- **Frontend**:
  - `genossi-frontend/src/page/member_details.rs` — neuer Knopf im bestehenden Aktionsbereich.
  - Nutzt vorhandenes `SELECTED_MEMBER_IDS`-Pattern aus `genossi-frontend/src/service/member.rs`.
  - i18n-Keys: ein neuer Key für „Mail senden" (falls nicht bereits vorhanden) und einer für den Disabled-Hinweis.
- **Backend**: Keine Änderung.
- **Berechtigungen**: Keine Änderung — der Knopf ist sichtbar, wenn die Detailseite überhaupt sichtbar ist (alle aktiven Nutzer sind heute Admin).
