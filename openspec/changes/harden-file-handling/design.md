## Context

Drei Schwächen in drei unterschiedlichen Layern, die sich gegenseitig nicht auffangen:

**Upload** (`genossi_rest/src/member_document.rs:139-171`):
```rust
"file" => {
    file_name = field.file_name().map(|s| s.to_string());
    mime_type = field.content_type().map(|s| s.to_string());   // client-controlled
    file_data = Some(field.bytes().await.…);
}
…
let mtype = mime_type.unwrap_or_else(|| "application/octet-stream".to_string());
```

**Download** (`genossi_rest/src/member_document.rs:236-244`):
```rust
let content_disposition = format!("attachment; filename=\"{}\"", doc.file_name);
Response::builder()
    .header("Content-Type", doc.mime_type.as_ref())  // persisted from upload → client-controlled origin
    .header("Content-Disposition", content_disposition)
```

**Filesystem** (`genossi_service_impl/src/document_storage.rs:20-22`):
```rust
fn full_path(&self, relative_path: &str) -> PathBuf {
    self.base_path.join(relative_path)
}
```

Zusätzlich baut `generate_document` (`genossi_rest/src/member_document.rs:351-357`) den Filename aus `member.last_name` + `member.first_name` — User-editierbaren DB-Werten.

**Bestehende Schutzmaßnahmen**, die das Risiko abmildern (aber nicht wegnehmen):
- `Content-Disposition: attachment` zwingt den Browser zum Download statt Inline-Rendering.
- Die Extension-Extraktion cappt auf Länge ≤ 10 (schwach, aber existent).
- Der Upload-Handler prüft `MAX_FILE_SIZE = 50 MB`.
- `Relative_path` wird aus UUID + Extension gebaut, kein User-Input darin.

**Constraints:**
- Keine Breaking-Changes für bestehende Dokumente in der DB (Migration vermeiden).
- Keine neuen Crates, wenn es nicht wirklich nötig ist.
- `Content-Disposition` nach RFC 6266 — der aktuelle Industriestandard, von allen Mainstream-Browsern unterstützt (Chrome, Firefox, Safari, Edge).

## Goals / Non-Goals

**Goals:**
- Hochgeladene Dateien werden vor dem Persistieren gegen eine server-seitige Whitelist von Extensions und abgeleiteten MIME-Types geprüft.
- Gespeicherte MIME-Types stammen ausschließlich aus der server-seitigen Mapping-Tabelle, nicht aus Client-Input.
- `Content-Disposition`-Filenames sind header-safe unter beliebigen UTF-8-Eingaben (Umlaute, Sonderzeichen, Leerzeichen, `"`, `\r\n`).
- Filesystem-Operationen scheitern explizit, wenn ein Pfad aus dem `base_path` herausführen würde.

**Non-Goals:**
- Keine Magic-Byte-Validierung (also nicht "in der Datei drin auch wirklich ein PDF prüfen"). Das wäre eine neue Dep (`infer`/`tree_magic`) und bringt für unseren Use-Case (interne Mitgliederdokumente) wenig Zusatznutzen gegenüber Extension-Whitelist.
- Keine Antivirus-Integration (ClamAV o.ä.). Out of scope für Genossi.
- Kein vollständiges MIME-Sniff-Ersatz auf Server-Seite. Wir vertrauen der Extension, weil die Benutzer Vorstand/Admins sind.
- Keine rückwirkende Korrektur gespeicherter MIME-Types in der DB. Das Download-Verhalten bleibt für Altdaten wie es ist; nur neue Uploads werden strenger.

## Decisions

### Whitelist: Extension-basiert mit MIME-Mapping

**Wahl:** Zentrale Konstante in `genossi_service/src/member_document.rs`:

```
pub const ALLOWED_FILE_TYPES: &[(&str, &str)] = &[
    ("pdf",  "application/pdf"),
    ("png",  "image/png"),
    ("jpg",  "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("txt",  "text/plain"),
    ("doc",  "application/msword"),
    ("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    ("odt",  "application/vnd.oasis.opendocument.text"),
    ("xls",  "application/vnd.ms-excel"),
    ("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    ("ods",  "application/vnd.oasis.opendocument.spreadsheet"),
];
```

Upload-Pfad: Extension aus Filename extrahieren → in der Tabelle nachschlagen → bei Miss HTTP 415. Gespeicherter MIME-Type ist der Wert aus der Tabelle.

**Alternativen:**
- *MIME-basierte Whitelist:* Client-MIME ist untrustworthy. Extension + server-side Mapping ist robuster.
- *Magic-Byte-Check:* siehe Non-Goals. Zu viel Aufwand, zu wenig Mehrwert.
- *Konfigurierbare Whitelist via Config-Store:* Flexibilität vs. Komplexität. Eine harte Konstante ist pragmatisch; wenn später echter Bedarf entsteht (z.B. `.csv` für Imports), kleines Follow-up.

**Edge-Case `file.tar.gz`:** rsplit('.').next() → "gz". Wenn `gz` nicht auf der Whitelist ist (so geplant), Upload wird abgelehnt. Das ist das gewünschte Verhalten.

### Server-seitige MIME-Ableitung

**Wahl:** Der `mime_type`-Wert im `UploadDocument` wird nicht mehr vom Handler aus dem Multipart gelesen, sondern im Service anhand der validierten Extension gesetzt. Das Handler-Field `mime_type` im Multipart wird ignoriert (aber nicht als Fehler geworfen — einfach overridden).

**Alternative:** Client-MIME akzeptieren, nur gegen Whitelist prüfen, bei Mismatch ablehnen. Das ist strenger, aber bringt nichts, weil wir eh die Extension als Source-of-Truth wollen. Einfach ignorieren ist klarer.

### Content-Disposition nach RFC 6266

**Wahl:** Small Helper-Funktion `content_disposition_header(filename: &str) -> String`, liefert:

```
attachment; filename="safe-ascii-fallback.pdf"; filename*=UTF-8''%E4%F6%FC-file.pdf
```

- `filename*=UTF-8''...` mit UTF-8-Percent-Encoding — moderner Browser-Standard, unterstützt Umlaute und Sonderzeichen vollständig.
- `filename="..."` mit reinem ASCII-Fallback (Non-ASCII → `_`; `"` und `\` → `_`) als Fallback für ganz alte Clients.
- Kein Line-Break-Escaping nötig, weil Percent-Encoding bereits `\r` (`%0D`), `\n` (`%0A`) etc. erfasst.

**Alternativen:**
- *Crate `urlencoding`:* genau das was wir brauchen, aber so klein, dass eigene Impl in <30 LOC passt. Kein echter Vorteil für die Dep.
- *Nur `filename=` mit aggressivem Stripping:* verliert Umlaute aus User-Sicht, Download heißt dann `Mller_Max.pdf` statt `Müller_Max.pdf`. Unschön.

### Path-Traversal-Defense im Storage

**Wahl:** `full_path` wird zu `fn full_path(&self, relative_path: &str) -> Result<PathBuf, StorageError>`:

```
let joined = self.base_path.join(relative_path);
let canonical_base = std::fs::canonicalize(&self.base_path)?;  // base muss existieren
let canonical_joined = …;   // joined existiert bei save evtl. noch nicht → manuelle Normalisierung
if !normalized.starts_with(&canonical_base) {
    return Err(StorageError::ValidationError(…));
}
Ok(joined)
```

**Problem:** `canonicalize` erfordert, dass der Pfad existiert. Beim `save` existiert die Zieldatei noch nicht. Lösung: Normalisieren manuell (Komponenten-iteration mit Ablehnung von `..` und absoluten Segmenten), dann `starts_with` als Plausi-Check.

**Alternativen:**
- *Crate `path-clean`:* kleine Crate, macht exakt das, gut getestet. → Aufnehmen als Dep, wenn eigene Impl fehleranfällig wirkt. Design-Entscheidung: **wir nutzen `path-clean`**, weil die Normalisierungs-Logik leicht subtile Bugs hat.
- *Jeden `..`-Character im relative_path ablehnen:* einfacher, aber fragiler (was bei Windows-Separatoren?).

### Filename-Normalisierung bei `generate_document`

**Wahl:** Helper `sanitize_filename_component(s: &str) -> String`:
- Transliteriert Umlaute: `ä`→`ae`, `ö`→`oe`, `ü`→`ue`, `ß`→`ss`, Großschreibung entsprechend
- Alles außer `[a-zA-Z0-9_-]` → `_`
- Trimmt Leading/Trailing `_`
- Falls leer: `"_"` als Fallback

Filename wird also `join_confirmation_1001_mueller_max.pdf` — deterministisch, URL-safe, header-safe.

**Alternative:** Nichts verändern, nur den Content-Disposition-Helper das regeln lassen. Funktioniert für den HTTP-Header, aber der Filename ist auch `doc.file_name` in der DB — der bleibt so erhalten. Wir wollen ihn sauber, nicht nur safe im Moment der HTTP-Response. Konsistenz gewinnt.

## Risks / Trade-offs

- [Risk] Admins laden `.csv` hoch (z.B. aus Buchhaltung) → nicht in der Whitelist, Upload schlägt fehl → Mitigation: Whitelist leicht erweiterbar, ein kleiner Code-Change für einen neuen Typ. Bei Bedarf wird `.csv` schnell ergänzt.
- [Risk] Bestehende Dokumente in der DB haben einen anderen MIME-Type als die neue Mapping-Tabelle vorsieht → Mitigation: Read-Pfad liest weiter den gespeicherten MIME. Nur Write-Pfad setzt die neue Ableitung. Keine Inkonsistenz für Endnutzer sichtbar.
- [Risk] Filename-Normalisierung verändert das angezeigte Download-Filename bei `generate_document` → Mitigation: ist gewollt. Deterministische Filenames sind in Archiv-Workflows ein Plus, nicht Minus. Doku: "generate_document produziert normalisierte Filenames".
- [Risk] Path-Normalisierung via `path-clean` als neue Dep → Mitigation: Single-Purpose Crate, seit Jahren stabil, kein Transit-Footprint. Akzeptabel.
- [Risk] `canonicalize(base_path)` benötigt, dass `base_path` existiert → Mitigation: Storage erstellt `base_path` beim Start, wenn nicht vorhanden. Check beim Service-Init.
- [Risk] Whitelist ist strenger als vorher — Admin-UX kann Verwirrung stiften bei Ablehnung → Mitigation: 415-Response-Body enthält die erlaubten Extensions als Liste, Frontend zeigt das dem Nutzer an.

## Migration Plan

1. **Code mergen.** Keine DB-Migration.
2. **Frontend-Anpassung** (kann in gleicher PR oder Follow-up): Upload-Form zeigt die erlaubten Extensions im `accept`-Attribut des `<input type="file">` und rendert die 415-Response mit Whitelist-Angabe.
3. **Deploy.** Neue Uploads werden strenger validiert. Bestehende Dokumente sind unberührt.
4. **Verifikation:**
   - Upload einer `.pdf` → akzeptiert, MIME in DB ist `application/pdf` (aus Mapping, nicht aus Multipart)
   - Upload einer `.html` → 415 mit erlaubten Typen
   - Upload mit Filename `Müller.pdf` → Download-Content-Disposition enthält `filename*=UTF-8''M%C3%BCller.pdf`
   - Generierter Antrag für Mitglied "Müller, Max" → Filename ist `…_mueller_max.pdf`
   - Bestehendes Dokument wird weiter korrekt angezeigt
5. **Rollback:** Code zurück, keine Daten-Änderungen rückabzuwickeln.

## Open Questions

- Soll es eine Admin-UI für die Whitelist geben? → **Nein**, Konstante im Code. Änderung braucht Deploy, das ist akzeptabel.
- Soll das Upload-Response-Feld im 415-Fall die erwarteten Extensions enthalten? → **Ja**, als Array in der JSON-Response, damit das Frontend sie rendern kann.
- Werden `.eml`-Uploads gebraucht (Mail-Archivierung)? → Bitte beim Verfassen der Whitelist im Kopf behalten, aber im Zweifel **nicht** aufnehmen — wenn der Bedarf kommt, separater kleiner Change.
