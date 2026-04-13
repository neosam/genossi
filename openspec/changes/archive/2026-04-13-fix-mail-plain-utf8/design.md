## Context

`genossi_mail/src/worker.rs` baut ausgehende Mails mit der `lettre`-Crate. Es existieren zwei Pfade:

1. **Ohne Anhang**: `Message::builder().body(body.to_string())` — `MessageBuilder::body()` setzt einen Default-Content-Type `text/plain` **ohne** `charset`-Parameter. Clients müssen dann raten; GMX Android rät Latin-1/Windows-1252 und zeigt Umlaute als Mojibake (`ü` → `Ã¼`).
2. **Mit Anhang**: `SinglePart::plain(body.to_string())` innerhalb eines `MultiPart::mixed` — das setzt explizit `Content-Type: text/plain; charset=utf-8` und ein passendes `Content-Transfer-Encoding` (quoted-printable). Dieser Pfad funktioniert korrekt.

Das Feedback aus der Praxis bestätigt: Mails mit Anhang → OK, Mails ohne Anhang → Umlaute kaputt.

## Goals / Non-Goals

**Goals:**
- Plain-Text-Mails ohne Anhang tragen `charset=utf-8` im `Content-Type`-Header.
- Beide Pfade (mit/ohne Anhang) nutzen denselben Mechanismus zum Body-Bau, sodass keine zukünftige Divergenz entstehen kann.
- Automatischer Test, der Regressionen abfängt.

**Non-Goals:**
- Änderungen an Subject-Encoding, Anhang-Dateinamen-Encoding oder From-Header (funktioniert bereits).
- Unterstützung von HTML-Mails oder alternativen Text/HTML-Multiparts.
- Änderungen am Worker-Loop, an der Persistenz oder an der Template-Rendering-Logik.

## Decisions

### Decision: Beide Pfade über `SinglePart::plain` bauen

Der `text_part` wird einmal über `SinglePart::plain(body.to_string())` gebaut und dann entweder direkt via `.singlepart(text_part)` auf dem `Message::builder()` gesetzt (kein Anhang) oder in ein `MultiPart::mixed()` eingebettet (mit Anhängen).

**Alternativen erwogen:**
- **`ContentType::TEXT_PLAIN_UTF_8` manuell als Header setzen** und `.body()` weiter verwenden: funktioniert, verdoppelt aber die Wege, auf denen ein Body gebaut werden kann. Höhere Divergenzgefahr.
- **Custom Header `Content-Type: text/plain; charset=utf-8` direkt injizieren**: fragil, umgeht lettre's interne Transfer-Encoding-Auswahl.

Die gewählte Lösung ist die kleinste, konsistenteste Änderung.

### Decision: Test über serialisierte Message-Bytes

Der Test baut eine Nachricht mit Umlauten (ohne Anhang) und prüft via `Message::formatted()` (liefert `Vec<u8>`), dass die Bytes den String `charset=utf-8` enthalten. Optional auch, dass der Body quoted-printable-kodiert ist (`=C3=BC` für `ü`). Das testet das beobachtbare On-the-Wire-Format, nicht Implementierungsdetails von lettre.

## Risks / Trade-offs

- **Risiko**: `SinglePart::plain` wählt automatisch Transfer-Encoding (7bit/quoted-printable/base64). Theoretisch könnte ein Client quoted-printable nicht dekodieren. → **Mitigation**: Der Multipart-Pfad nutzt dasselbe seit längerem ohne Beschwerden; GMX Android unterstützt quoted-printable.
- **Risiko**: Test könnte auf lettre-Interna prüfen, die sich in zukünftigen Versionen ändern. → **Mitigation**: Nur auf den `charset=utf-8`-String prüfen; das ist eine stabile RFC-Garantie.
