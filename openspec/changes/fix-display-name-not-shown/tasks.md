## 1. Berechtigungsseite Fix

- [x] 1.1 In `UserRowComponent` (`permissions.rs`) einen `use_effect` hinzufügen, der `name_input` mit dem aktuellen `sender_name` aus dem `users`-Signal synchronisiert, wenn sich die Daten ändern
- [x] 1.2 Verifizieren, dass das Editieren des Feldes vor dem Speichern weiterhin funktioniert (use_effect darf User-Eingaben nicht überschreiben)

## 2. Konfigurationsseite prüfen und fixen

- [x] 2.1 Prüfen ob der `sender_name` auf der Config-Seite korrekt geladen und angezeigt wird (gleiches Problem oder nicht?)
- [x] 2.2 Falls nötig: gleichen Fix anwenden

## 3. Tests

- [x] 3.1 Bestehende Tests anpassen/erweitern, um sicherzustellen, dass der Anzeigename nach dem Laden korrekt dargestellt wird
