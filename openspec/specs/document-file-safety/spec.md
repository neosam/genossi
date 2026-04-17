## ADDED Requirements

### Requirement: Upload-Extension-Whitelist

Das System SHALL Datei-Uploads in `POST /api/members/{id}/documents` nur akzeptieren, wenn die Datei-Extension in einer fest definierten Whitelist enthalten ist. Die Whitelist umfasst mindestens: `pdf`, `png`, `jpg`, `jpeg`, `webp`, `txt`, `doc`, `docx`, `odt`, `xls`, `xlsx`, `ods`. Der Vergleich SHALL case-insensitive erfolgen.

Bei nicht erlaubter Extension SHALL das System mit HTTP 415 antworten und im Response-Body die Liste der erlaubten Extensions ausliefern (Schema `{"error": "<message>", "allowed_extensions": ["pdf", "png", ...]}`).

#### Scenario: Upload mit erlaubter Extension

- **WHEN** ein authentifizierter Admin eine Datei `beitrittserklaerung.pdf` hochlädt
- **THEN** das System akzeptiert den Upload und speichert das Dokument

#### Scenario: Upload mit verbotener Extension

- **WHEN** ein authentifizierter Admin eine Datei `malware.exe` hochlädt
- **THEN** das System antwortet mit HTTP 415 und listet die erlaubten Extensions in der Response

#### Scenario: Upload mit Uppercase-Extension

- **WHEN** ein authentifizierter Admin eine Datei `DOKUMENT.PDF` hochlädt
- **THEN** das System akzeptiert den Upload (Extension-Check ist case-insensitive)

#### Scenario: Upload mit zusammengesetzter Extension

- **WHEN** ein authentifizierter Admin eine Datei `archive.tar.gz` hochlädt
- **THEN** das System betrachtet nur die letzte Extension (`gz`), diese ist nicht whitelisted, und antwortet mit HTTP 415

### Requirement: Server-seitige MIME-Ableitung

Das System SHALL den MIME-Type eines hochgeladenen Dokuments ausschließlich aus einer server-seitigen Mapping-Tabelle ableiten, basierend auf der validierten Extension. Client-gelieferte MIME-Types (`Content-Type`-Feld im Multipart-Upload) SHALL ignoriert werden.

#### Scenario: Upload mit abweichendem client-MIME

- **WHEN** ein Client eine Datei `dokument.pdf` mit Multipart-`Content-Type: text/html` hochlädt
- **THEN** das System speichert das Dokument mit `mime_type = "application/pdf"` (aus der Mapping-Tabelle), nicht `text/html`

#### Scenario: Upload ohne client-MIME

- **WHEN** ein Client eine Datei `bild.png` ohne `Content-Type`-Header im Multipart-Feld hochlädt
- **THEN** das System speichert das Dokument mit `mime_type = "image/png"` (aus der Mapping-Tabelle)

### Requirement: Content-Disposition nach RFC 6266

Das System SHALL beim Download von Dokumenten einen `Content-Disposition`-Header setzen, der RFC 6266 entspricht. Der Header SHALL sowohl ein ASCII-sicheres `filename="..."`-Fallback als auch ein UTF-8-Percent-encoded `filename*=UTF-8''...` enthalten. Der Header SHALL niemals rohe `"`, `\r`, `\n` oder andere Steuerzeichen enthalten, auch wenn der DB-gespeicherte Dateiname diese enthält.

#### Scenario: Download eines Dokuments mit Umlauten

- **WHEN** ein authentifizierter User ein Dokument mit Dateinamen `Müller_Antrag.pdf` herunterlädt
- **THEN** der `Content-Disposition`-Header enthält `attachment; filename="Mueller_Antrag.pdf"; filename*=UTF-8''M%C3%BCller_Antrag.pdf` (oder semantisch äquivalent)

#### Scenario: Download eines Dokuments mit Anführungszeichen im Filename

- **WHEN** ein authentifizierter User ein Dokument herunterlädt, dessen Filename ein `"` enthält
- **THEN** der `Content-Disposition`-Header ist syntaktisch gültig; das `"` wird im ASCII-Fallback durch `_` ersetzt, im UTF-8-Teil als `%22` encoded

#### Scenario: Download eines Dokuments mit Newline im Filename

- **WHEN** ein authentifizierter User ein Dokument herunterlädt, dessen Filename ein `\r\n` enthält
- **THEN** der `Content-Disposition`-Header enthält weder `\r` noch `\n` im Klartext; im UTF-8-Teil sind sie als `%0D%0A` encoded, im ASCII-Fallback entfernt

### Requirement: Path-Traversal-Schutz im Storage

Das System SHALL in `DocumentStorage::save`, `load` und `delete` vor jeder Dateisystem-Operation prüfen, dass der normalisierte, absolute Pfad unter `base_path` bleibt. Bei Pfad-Traversal-Versuchen (z.B. `relative_path = "../../etc/passwd"`) SHALL das System `StorageError::ValidationError` zurückgeben, ohne auf das Filesystem zuzugreifen.

#### Scenario: Pfad bleibt im base_path

- **WHEN** eine Storage-Operation mit `relative_path = "abc-uuid.pdf"` aufgerufen wird
- **THEN** das System führt die Operation innerhalb von `base_path` aus

#### Scenario: Pfad enthält ..-Sequenz

- **WHEN** eine Storage-Operation mit `relative_path = "../etc/passwd"` aufgerufen wird
- **THEN** das System gibt `StorageError::ValidationError` zurück und greift nicht auf das Filesystem zu

#### Scenario: Pfad enthält absoluten Segment

- **WHEN** eine Storage-Operation mit `relative_path = "/etc/passwd"` aufgerufen wird
- **THEN** das System gibt `StorageError::ValidationError` zurück und greift nicht auf das Filesystem zu

### Requirement: Filename-Normalisierung bei generated Documents

Das System SHALL beim Generieren von Dokumenten (`POST /api/members/{id}/documents/generate/{document_type}`) den Filename so normalisieren, dass nur `[a-zA-Z0-9_-]`-Zeichen enthalten sind. Deutsche Umlaute (`ä`, `ö`, `ü`, `ß` sowie deren Großschreibung) SHALL transliteriert werden (`ä`→`ae` etc.). Andere Non-ASCII-Zeichen sowie Sonderzeichen SHALL durch `_` ersetzt werden.

#### Scenario: Generiertes Dokument für Mitglied mit Umlauten im Namen

- **WHEN** ein Admin ein Beitrittsbestätigungs-PDF für Mitglied "Müller, Max" (Nummer 1001) generiert
- **THEN** der Filename ist `join_confirmation_1001_mueller_max.pdf`

#### Scenario: Generiertes Dokument für Mitglied mit Bindestrich im Namen

- **WHEN** ein Admin ein Beitrittsbestätigungs-PDF für Mitglied "Anna-Lena Weber-Schmidt" generiert
- **THEN** der Filename enthält `weber-schmidt_anna-lena` (Bindestriche bleiben erhalten)

#### Scenario: Generiertes Dokument für Mitglied mit Sonderzeichen im Namen

- **WHEN** ein Admin ein Beitrittsbestätigungs-PDF für Mitglied mit Namen `"O'Brien"` generiert
- **THEN** der Filename enthält `o_brien` (Apostroph wird zu `_`)
