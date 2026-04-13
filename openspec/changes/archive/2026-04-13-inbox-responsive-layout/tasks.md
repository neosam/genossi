## 1. Viewport-fixiertes Layout

- [x] 1.1 Aeusseren Container (`p-4 max-w-6xl mx-auto`) um `flex flex-col h-[calc(100vh-4rem)]` erweitern
- [x] 1.2 Titel und Fehler-/Info-Banner in einen `flex-none` Bereich verschieben
- [x] 1.3 Content-Container (`flex gap-4`) um `flex-1 min-h-0` erweitern

## 2. Responsive Spalten

- [x] 2.1 Liste-Container: `w-1/2` zu `w-full md:w-1/2` aendern, `flex flex-col overflow-hidden` hinzufuegen
- [x] 2.2 Detail-Container: `w-1/2` zu `w-full md:w-1/2` aendern, `flex flex-col overflow-hidden` hinzufuegen
- [x] 2.3 Bedingte Sichtbarkeit: Liste `hidden md:flex` wenn `selected_id.is_some()`, Detail `hidden md:flex` wenn `selected_id.is_none()`

## 3. Internes Scrollen

- [x] 3.1 Mail-Liste (`ul`): `overflow-y-auto flex-1` hinzufuegen
- [x] 3.2 Mail-Body (`pre`): `max-h-96` durch `flex-1 overflow-y-auto` ersetzen
- [x] 3.3 Detail-Inhalt in scrollbaren und fixen Bereich aufteilen: Header (flex-none), Body+Actions (flex-1 overflow-y-auto)

## 4. Mobile Navigation

- [x] 4.1 Zurueck-Button im Detail-View hinzufuegen (`md:hidden`), der `selected_id` und `detail` auf `None` setzt
- [x] 4.2 Sicherstellen, dass Deep-Link `/inbox/:id` auf Mobil direkt die Detail-Ansicht zeigt

## 5. Testen

- [x] 5.1 Kompilierung und grundlegende Funktionalitaet pruefen (`cargo build -p genossi-frontend` oder `dx build`)
- [x] 5.2 Bestehende Tests ausfuehren und sicherstellen, dass nichts bricht
