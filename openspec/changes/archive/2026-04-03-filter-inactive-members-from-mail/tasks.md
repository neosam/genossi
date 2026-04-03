## 1. Gemeinsame Hilfsfunktionen extrahieren

- [x] 1.1 `is_active()` und `today()` aus `members.rs` in ein gemeinsames Modul verschieben (z.B. `src/member_utils.rs`)
- [x] 1.2 `members.rs` anpassen: Funktionen aus dem neuen Modul importieren statt lokal definieren
- [x] 1.3 Neues Modul in `src/page/mod.rs` oder `src/lib.rs` registrieren

## 2. Mail-Seite filtern

- [x] 2.1 In `mail_page.rs`: `is_active()` und `today()` importieren
- [x] 2.2 `members_with_email_count` (Zeile 65-68): Nur aktive, nicht-gelöschte Mitglieder mit E-Mail zählen
- [x] 2.3 "Alle"-Button `onclick` (Zeile 257-262): Nur IDs aktiver, nicht-gelöschter Mitglieder auswählen

## 3. Testen

- [x] 3.1 Bestehende Tests kompilieren und laufen lassen (`cargo test`)
- [x] 3.2 Unit-Test für `is_active()` im neuen Modul hinzufügen (falls noch nicht vorhanden)
