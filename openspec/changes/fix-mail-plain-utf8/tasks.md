## 1. Implementation

- [x] 1.1 In `genossi_mail/src/worker.rs` den Build-Pfad für Mails so umstellen, dass der Body immer über `SinglePart::plain(body.to_string())` gebaut wird — auch im Pfad ohne Anhänge (via `.singlepart(text_part)` statt `.body(...)`).
- [x] 1.2 Sicherstellen, dass der Multipart-Pfad denselben `text_part` weiterverwendet, damit es nur noch einen Body-Bau-Pfad gibt.

## 2. Tests

- [x] 2.1 Unit-/Integrationstest in `genossi_mail` hinzufügen, der eine Nachricht ohne Anhang mit Umlauten (`äöüß`) baut, `Message::formatted()` aufruft und prüft, dass die serialisierten Bytes `charset=utf-8` enthalten.
- [x] 2.2 Analogen Test für den Pfad mit Anhang hinzufügen (sichert Regressionsschutz für den bereits funktionierenden Pfad).
- [x] 2.3 `cargo test -p genossi_mail` ausführen und grün sehen.

## 3. Qualität

- [ ] 3.1 `cargo fmt` ausführen. *(übersprungen: `cargo fmt` in diesem Environment nicht verfügbar)*
- [ ] 3.2 `cargo clippy -p genossi_mail` ohne neue Warnings. *(übersprungen: `cargo clippy` in diesem Environment nicht verfügbar)*
