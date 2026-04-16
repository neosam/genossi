## Why

Das Security-Review hat drei zusammenhängende Schwächen in der Datei-Handhabung aufgedeckt, die isoliert jeweils niedrig bis mittel wirken, in Kombination aber einen realen Angriffsvektor öffnen:

1. **Upload akzeptiert jeden MIME-Type und jede Extension** — der Client-gelieferte MIME-Type wird 1:1 gespeichert (`genossi_rest/src/member_document.rs:141`), die Extension wird per naivem `rsplit('.')` extrahiert und lediglich auf Länge ≤ 10 gefiltert (`genossi_service_impl/src/member_document.rs:34-40`). Es gibt keine Whitelist.
2. **Download serviert den gespeicherten MIME-Type 1:1 zurück** (`genossi_rest/src/member_document.rs:241`), und das `Content-Disposition`-Feld `filename="..."` wird durch `format!()` ohne Escaping gebaut (`genossi_rest/src/member_document.rs:236-237`). Bei `generate_document` fließen `member.last_name` und `member.first_name` in den Filename ein — User-editierbare DB-Werte landen so direkt im HTTP-Header.
3. **`DocumentStorage::full_path` vertraut `relative_path` blind** (`genossi_service_impl/src/document_storage.rs:20-22`). `PathBuf::join` resolved keine `..`-Sequenzen. Aktuell ist der Pfad durch `{UUID}.{ext}` sicher, aber jede künftige Code-Änderung, die das Pfadschema ändert, kann das ungeschützt brechen.

Die App läuft öffentlich im Internet; über die OIDC-authentifizierte Admin-Oberfläche lassen sich Dokumente hochladen. Kombiniert mit möglichem Cookie-Hijack (anderer Change in Arbeit) will man nicht, dass ein beliebiger Upload als `text/html` mit scriptfähigem Inhalt später per Download an Admins ausgeliefert wird — auch wenn `Content-Disposition: attachment` viel abfängt, ist eine tiefergehende Verteidigung hier günstig umgesetzt.

## What Changes

- **Upload-Whitelist**: Akzeptierte Datei-Typen beschränken auf eine feste Liste: `pdf`, `png`, `jpg`/`jpeg`, `webp`, `txt`, `doc`, `docx`, `odt`, `xls`, `xlsx`, `ods`. Ablehnung erfolgt mit HTTP 415 (Unsupported Media Type), Fehlermeldung enthält die erlaubten Typen.
- **Server-seitiger MIME-Type**: Der gespeicherte MIME-Type wird nicht mehr aus dem Multipart-Feld übernommen, sondern server-seitig anhand der (validierten) Extension abgeleitet aus einer festen Mapping-Tabelle. Client-gelieferte MIME-Types werden ignoriert.
- **Filename-Escaping im Download**: `Content-Disposition` wird per RFC 6266 `filename*=UTF-8''<percent-encoded>` gesetzt. Zusätzlich gibt es ein ASCII-`filename=`-Fallback, das auf sichere Zeichen gestrippt wird. Keine rohen `"`, `\r`, `\n` mehr im Header.
- **Path-Traversal-Defense-in-Depth**: `DocumentStorage::save`, `load` und `delete` validieren, dass der resultierende absolute Pfad nach `join` unter `base_path` bleibt (Canonical-Check oder `starts_with` auf normalisierten Pfad). Bei Verletzung: `StorageError::ValidationError`, kein FS-Zugriff.
- **Extension-Extraktor-Härtung**: `extract_extension` akzeptiert nur `[a-zA-Z0-9]{1..=10}` und prüft gegen die Whitelist. Unklare Fälle wie `file.tar.gz` bekommen explizit definiertes Verhalten (nur letzte Extension).
- **Generated-Document-Filename**: Bei `generate_document` werden `first_name`/`last_name` vor Einsatz im Filename auf `[a-zA-Z0-9-_]` reduziert (Umlaute transliteriert oder ersetzt), damit Filenames deterministisch und header-safe bleiben.

## Capabilities

### New Capabilities

- `document-file-safety`: Definiert Upload-Whitelist, server-seitige MIME-Ableitung, sichere Download-Header (Content-Type, Content-Disposition mit RFC-6266-Escaping) und Path-Traversal-Schutz im Filesystem-Storage-Layer.

### Modified Capabilities

_(keine bestehenden Specs betroffen — `member-documents` beschreibt Audit-Verhalten, nicht File-Handling-Safety)_

## Impact

**Code:**
- `genossi_rest/src/member_document.rs` — Upload-Handler ignoriert client-MIME, validiert Extension/MIME gegen Whitelist, 415 bei Ablehnung. Download-Handler setzt Content-Disposition per RFC 6266. `generate_document` normalisiert den Filename.
- `genossi_service/src/member_document.rs` — neue Typen: `AllowedFileType` Enum oder Whitelist-Konstante mit `extension → mime_type`-Map
- `genossi_service_impl/src/member_document.rs` — `extract_extension` strenger (ASCII-alnum + Whitelist-Check)
- `genossi_service_impl/src/document_storage.rs` — `full_path` liefert `Result<PathBuf, StorageError>`, prüft `starts_with(base_path)` nach Canonicalize
- `genossi_rest_types/src/lib.rs` — ggf. neuer TO-Typ für die Whitelist-Response (falls im Frontend gebraucht)

**Keine neuen Dependencies** — RFC-6266-Encoding schreiben wir als kleine Helper-Funktion (UTF-8-Percent-Encoding ist ≈20 LOC). `urlencoding`-Crate wäre Option, aber nicht nötig.

**Datenbank:**
- Keine Migration. Existierende Dokumente behalten ihren gespeicherten MIME-Type (der Read-Pfad ist abwärtskompatibel). Neue Uploads bekommen server-seitig abgeleiteten MIME.

**Benutzer:**
- Admin kann Dateien mit exotischen Extensions (z.B. `.exe`, `.zip`, `.html`) nicht mehr hochladen. Das ist neu, aber erwartungskonform — für Mitgliederdokumente sind Office-Formate und Bilder/PDFs der reale Use-Case.
- Download-Verhalten bleibt äußerlich identisch (Browser speichert unter dem Originalnamen), nur der Header ist strenger UTF-8-fähig und escape-sicher.
- Keine Breaking-Changes für bestehende Dokumente.
