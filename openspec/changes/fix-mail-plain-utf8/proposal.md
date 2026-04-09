## Why

Empfänger mit GMX Android sehen in E-Mails ohne Anhang kaputte Umlaute (z.B. `Müller` → `MÃ¼ller`). Ursache: der Plain-Text-Pfad in `genossi_mail/src/worker.rs` baut die Nachricht über `MessageBuilder::body()`, wodurch lettre den Content-Type ohne `charset=utf-8` setzt. Tolerante Clients raten UTF-8, GMX Android rät Latin-1 — und zeigt Mojibake. Der Multipart-Pfad (mit Anhang) nutzt bereits `SinglePart::plain` und ist deshalb nicht betroffen.

## What Changes

- Der Plain-Text-Body wird in beiden Pfaden (mit und ohne Anhänge) über `SinglePart::plain` gebaut, sodass `Content-Type: text/plain; charset=utf-8` garantiert gesetzt wird.
- Ein Test sichert ab, dass die serialisierte Nachricht den UTF-8-Charset enthält und Umlaute korrekt encodiert sind.

## Capabilities

### New Capabilities
<!-- keine -->

### Modified Capabilities
- `mail-sending`: Anforderung ergänzen, dass ausgehende Plain-Text-Mails zwingend `charset=utf-8` im Content-Type tragen, unabhängig davon ob Anhänge vorhanden sind.

## Impact

- Code: `genossi_mail/src/worker.rs` (Funktion `send_mail_now` bzw. der Build-Pfad für Mails).
- Tests: neuer Unit-/Integrationstest in `genossi_mail`, der die formatierte Nachricht auf `charset=utf-8` prüft.
- APIs/DB/Config: keine Änderungen.
- Risiko: minimal — der Multipart-Pfad nutzt bereits denselben Mechanismus.
