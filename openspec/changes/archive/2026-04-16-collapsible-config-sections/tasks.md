## 1. CollapsibleSection-Komponente

- [x] 1.1 Neue Datei `genossi-frontend/src/component/collapsible_section.rs` anlegen
- [x] 1.2 Komponentensignatur: `CollapsibleSection { title: String, default_open: bool, children: Element }`
- [x] 1.3 Lokales `use_signal(default_open)` für den Aufklapp-Zustand
- [x] 1.4 Header rendern als `<button>` mit Titel + Pfeil-Icon (z. B. ▶ / ▼ via Unicode oder SVG); Klick schaltet den Zustand um
- [x] 1.5 Inhalt bedingt rendern: nur wenn Zustand „offen"
- [x] 1.6 Tailwind-Styling abstimmen mit dem bestehenden Sektion-Look (`bg-white rounded-lg shadow p-6 mb-6`)
- [x] 1.7 Re-Export in `genossi-frontend/src/component/mod.rs`

## 2. Config-Page umbauen

- [x] 2.1 SMTP-Sektion (Z. 220 ff.) in `CollapsibleSection { title: "SMTP" }` wickeln
- [x] 2.2 Mail-Footer & Sender-Name (Z. 458 ff.) wickeln
- [x] 2.3 IMAP-Posteingang (Z. 551 ff.) wickeln
- [x] 2.4 WebDAV-Backup (Z. 777 ff.) wickeln
- [x] 2.5 `TsaConfigSection` (Z. 991 ff.) in einen `CollapsibleSection`-Wrapper packen
- [x] 2.6 `WordPressIntegrationSection` (Z. 999 ff.) in einen `CollapsibleSection`-Wrapper packen
- [x] 2.7 Generische Config-Entries (Z. 1020 ff.) wickeln
- [x] 2.8 Doppelte `bg-white rounded-lg shadow p-6 mb-6`-Wrapper entfernen, wenn die `CollapsibleSection` schon den Container darstellt

## 3. Tests

- [x] 3.1 Komponententest für `CollapsibleSection`: Default eingeklappt, Klick öffnet, erneuter Klick schließt
- [x] 3.2 Komponententest: `default_open: true` startet aufgeklappt
- [ ] 3.3 Manueller Test: `/config` aufrufen → alle Sektionen zu, Klick öffnet einzelne, mehrere können gleichzeitig offen sein
- [ ] 3.4 Manueller Test: Inhalte und Verhalten jeder Sektion unverändert (Eingaben funktionieren, Speichern funktioniert)
- [ ] 3.5 Mobile Test: kompakte Darstellung bleibt benutzbar

## 4. Verifizierung

- [x] 4.1 `cargo fmt`
- [x] 4.2 `cargo clippy --all-targets` (keine neuen Warnings in `collapsible_section.rs` oder geänderten Bereichen)
- [x] 4.3 `cargo test` (33 Tests grün, inkl. 7 neue für `CollapsibleSection`)
- [x] 4.4 Spec-Scenarios aus `specs/collapsible-sections/spec.md` einzeln durchspielen und abhaken
