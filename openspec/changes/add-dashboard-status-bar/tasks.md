## 1. Routing & Home-Redirect

- [x] 1.1 Bestehende `home.rs`-Splash entfernen (oder auf reine Redirect-Komponente reduzieren)
- [x] 1.2 Route `/` so umbauen, dass sie auf `/members` umleitet (z. B. `use_effect` mit `navigator().replace(Route::Members {})`)
- [x] 1.3 Bookmark-Test: Aufruf von `/` führt ohne sichtbaren Zwischenschritt auf die Mitgliederliste

## 2. i18n-Keys

- [x] 2.1 Neue Keys in `src/i18n/mod.rs` ergänzen: `OpenApplicationsCount`, `OpenApplicationsNone`, `OpenInboxCount`, `OpenInboxNone`
- [x] 2.2 Übersetzungen in `de.rs` einpflegen ("3 offene Anträge", „Keine offenen Anträge", „12 offene Mails", „Keine offenen Mails")
- [x] 2.3 Übersetzungen in `en.rs` einpflegen
- [x] 2.4 Übersetzungen in `cs.rs` einpflegen (entfällt — kein cs.rs vorhanden, nur En/De)

## 3. Datenbeschaffung

- [x] 3.1 Hilfsfunktion in `api.rs` (oder `service/`): `get_open_applications_count` über bestehenden `get_applications(filter="Offen")`
- [x] 3.2 Hilfsfunktion `get_open_inbox_count` über bestehenden `get_inbox`-Endpoint mit Filter `open` (filtern, falls Endpoint keinen Status-Parameter unterstützt)
- [x] 3.3 Fehlerpfad: bei Fehlschlag wird `None` zurückgegeben, sodass die Komponente einen Platzhalter rendern kann

## 4. Wiederverwendbare Komponente `StatusBar`

- [x] 4.1 Neue Datei `src/component/status_bar.rs` anlegen
- [x] 4.2 Datentyp `StatusBarItem { label_with_count, label_none, count: Option<usize>, route: Route }` definieren
- [x] 4.3 Komponente `StatusBar` rendert eine horizontale Item-Reihe mit Trennzeichen (`•`) und passendem Tailwind-Stil (kompakt, eine Zeile)
- [x] 4.4 Jedes Item rendert als Link (`Link { to: ... }`); bei `count == None` wird neutraler Platzhalter „—" gezeigt
- [x] 4.5 Komponente exportiert in `src/component/mod.rs`

## 5. Einbau auf der Mitgliederseite

- [x] 5.1 In `src/page/members.rs` zwei Signals für die Counts (`open_applications_count`, `open_inbox_count`) ergänzen
- [x] 5.2 In `use_effect` beim Mount beide Counts laden
- [x] 5.3 `StatusBar` oberhalb der bestehenden Toolbar/Filter rendern, mit Items für Anträge und Mails
- [x] 5.4 Sicherstellen, dass das Layout der Mitgliederliste durch die zusätzliche Zeile nicht bricht (Print-Stile, mobile Breakpoints)

## 6. Tests

- [x] 6.1 Komponententest für `StatusBar`: Rendering bei `Some(3)`, `Some(0)`, `None`
- [x] 6.2 Komponententest für `StatusBar`: Linkziel stimmt für jedes Item
- [ ] 6.3 Manueller Smoke-Test im Browser: Aufruf von `/` leitet auf `/members`; Statusbalken erscheint mit beiden Items; Klicks landen auf `/applications` bzw. `/inbox`
- [ ] 6.4 Manueller Test mobil (schmaler Viewport): Statusbalken bleibt sichtbar und liest sich gut

## 7. Verifizierung

- [x] 7.1 `cargo fmt`
- [x] 7.2 `cargo clippy --all-targets` (clippy nicht installiert; cargo check --all-targets OK)
- [x] 7.3 `cargo test`
- [ ] 7.4 Spec-Scenarios aus `specs/dashboard-status-bar/spec.md` einzeln durchspielen und abhaken
