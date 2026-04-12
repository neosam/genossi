## Why

Die Inbox-Seite ist auf mobilen Geraeten nicht benutzbar. Das Zwei-Spalten-Layout (`w-1/2`) wird auch auf kleinen Bildschirmen erzwungen, wodurch beide Spalten in ~180px gequetscht werden. Ausserdem hat die Seite keine feste Viewport-Hoehe — der gesamte Mail-Inhalt, die Reply-Form und die Action-Buttons druecken die Seite nach unten, sodass man die ganze Seite scrollen muss und dann wieder hochscrollen muss, um eine andere Mail auszuwaehlen.

## What Changes

- Viewport-fixiertes Layout: Der Inbox-Bereich wird auf `100vh` minus Header-Hoehe begrenzt. Mail-Liste und Mail-Detail scrollen intern statt die ganze Seite.
- Responsive Zwei-Spalten-Layout: Auf Desktop (`md:` Breakpoint, >=768px) bleibt das Zwei-Spalten-Layout. Auf Mobil wird ein List/Detail-Pattern verwendet — entweder Liste oder Detail sichtbar, nie beides gleichzeitig.
- Zurueck-Button auf Mobil: Im Detail-View wird ein Zurueck-Button angezeigt, der zur Liste zuruecknavigiert (setzt `selected_id` auf `None`).
- Scrollbare Bereiche: Sowohl die Mail-Liste als auch der Mail-Body werden intern scrollbar (`overflow-y-auto`), die Seite selbst scrollt nicht mehr.

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- `genossi-frontend/src/page/inbox_page.rs`: Hauptaenderungen am Layout (CSS-Klassen, bedingte Sichtbarkeit, Zurueck-Button)
- Keine API-Aenderungen, keine Backend-Aenderungen
- Keine neuen Abhaengigkeiten
- Tailwind-Klassen `md:`, `hidden`, `flex`, `h-[calc(...)]`, `overflow-y-auto`, `min-h-0` werden verwendet (alle bereits in Tailwind enthalten)
