## Context

Der aktuelle `PUT /api/templates/{path}`-Handler in `genossi_rest/src/template.rs` extrahiert den Body als `String` (`body: String`). Das funktioniert nur für Text. `TemplateStorage::write_file()` nimmt ebenfalls `&str`.

Für Bilder (Logos etc.) in Typst-Templates braucht es Binär-Support.

## Goals / Non-Goals

**Goals:**
- Binärdateien (PNG, JPG, SVG etc.) über denselben Endpoint hochladen wie Textdateien
- Bestehende Text-Uploads dürfen nicht brechen

**Non-Goals:**
- Kein Multipart-Upload oder spezieller Upload-Endpoint
- Keine Dateigröße-Limits (erstmal)
- Kein Frontend-Upload-Button (separates Feature)

## Decisions

### Body als `axum::body::Bytes` statt `String` annehmen

Der Handler nimmt `body: axum::body::Bytes` entgegen. `Bytes` funktioniert für Text und Binärdaten gleichermaßen.

**Warum:** Einfachste Lösung. Kein Content-Type-Sniffing nötig. `Bytes` ist der natürliche Typ für „rohe Daten" in Axum. Die Unterscheidung Text/Binär ist für das Schreiben auf das Dateisystem irrelevant — `tokio::fs::write` akzeptiert `&[u8]`.

**Alternative:** Multipart-Upload → unnötige Komplexität für einzelne Dateien.

### Eine neue Methode `write_file_bytes` in `TemplateStorage`

Statt `write_file(&str)` zu ändern, kommt eine neue `write_file_bytes(&[u8])` Methode hinzu. Die bestehende `write_file` bleibt unverändert.

**Warum:** Bestehende Aufrufer von `write_file` (z.B. in Tests) müssen nicht geändert werden.

## Risks / Trade-offs

- **Große Dateien könnten Speicher füllen** → Akzeptabel für den Use-Case (Logos sind typischerweise klein). Limits können später ergänzt werden.
