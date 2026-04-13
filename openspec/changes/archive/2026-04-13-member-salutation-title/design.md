## Context

Das Member-Entity hat aktuell 24 Felder, aber keine Möglichkeit, Mitglieder korrekt anzuschreiben. Für formelle Korrespondenz (Briefe, E-Mails) werden Anrede und akademischer Titel benötigt.

Das Projekt folgt einer strikten Layer-Architektur: Migration → DAO → Service → REST → Frontend. Neue Felder müssen durch alle Layer durchgereicht werden. Es gibt bestehende Enum-Patterns (z.B. `ActionType`) die als Vorlage dienen.

Das Frontend nutzt Dioxus (Rust/WASM) mit einem Spalten-System (`columns.rs`) für die Mitgliederliste und einer Detail-Seite (`member_details.rs`).

## Goals / Non-Goals

**Goals:**
- Anrede-Feld (`salutation`) als Enum mit Werten `Herr`, `Frau`, `Firma` (optional, nullable)
- Titel-Feld (`title`) als Freitext (optional, nullable)
- Beide Felder in allen Layern: DB, DAO, Service, REST API, Frontend
- Editierbar in Mitgliederliste (inline) und auf Detail-Seite
- Keine Breaking Changes für bestehende Daten

**Non-Goals:**
- Template-Engine für automatische Anrede-Generierung (kommt separat)
- Excel-Import-Anpassung für diese Felder (kann später ergänzt werden)
- Validierung von Titel-Formaten

## Decisions

### 1. Salutation als Enum mit String-Serialisierung in DB

**Entscheidung**: `salutation` wird als `TEXT` in SQLite gespeichert mit den Werten `"Herr"`, `"Frau"`, `"Firma"`. Im Rust-Code ist es ein Enum mit `as_str()`/`from_str()` Konvertierung.

**Rationale**: Folgt dem bestehenden `ActionType`-Pattern. String-Werte in der DB sind lesbar und erweiterbar. Ein Enum im Code gibt Typsicherheit.

**Alternativen**:
- Integer-Codes (0/1/2): Weniger lesbar in der DB, schwerer erweiterbar
- Freitext: Keine Typsicherheit, Tippfehler möglich

### 2. Title als Freitext

**Entscheidung**: `title` wird als `TEXT` in SQLite gespeichert, im Rust-Code als `Option<Arc<str>>`.

**Rationale**: Akademische Titel-Kombinationen sind zu vielfältig für ein Enum ("Dr.", "Prof.", "Prof. Dr.", "Dr. med.", "Prof. Dr. Dr. h.c."). Freitext gibt maximale Flexibilität.

### 3. Beide Felder optional (nullable)

**Entscheidung**: Beide Felder sind `Option`-Typen und haben keinen Default-Wert in der Migration (`DEFAULT NULL`).

**Rationale**: Bestehende Mitglieder haben diese Daten nicht. Erzwingen würde eine Datenmigration erfordern die nicht sinnvoll automatisierbar ist.

### 4. Salutation-Dropdown im Frontend

**Entscheidung**: In der Mitgliederliste wird `salutation` als `<select>` gerendert (nicht als Freitext-Input). Auf der Detail-Seite ebenso.

**Rationale**: Bei einem Enum mit 3+1 Optionen (Herr/Frau/Firma/leer) ist ein Dropdown die natürliche UI-Wahl. Verhindert Tippfehler.

### 5. Enum-Definition in allen Layern

**Entscheidung**: Separate Enum-Typen pro Layer (`Salutation` in DAO, `SalutationTO` in REST-Types, `SalutationTO` in Frontend REST-Types) mit `From`-Implementierungen.

**Rationale**: Folgt dem bestehenden Pattern von `ActionType`/`ActionTypeTO`. Hält die Layer-Grenzen sauber.

## Risks / Trade-offs

- **[Enum-Erweiterung]** → Neue Anrede-Werte erfordern Code-Änderungen in allen Layern. Akzeptabel, da Änderungen selten und das Pattern etabliert ist.
- **[Freitext-Titel]** → Keine Konsistenz-Garantie (z.B. "Dr." vs "Dr" vs "Doktor"). → Akzeptabel, da die korrekte Schreibweise Aufgabe des Nutzers ist.
- **[Frontend-Spalten]** → Zwei neue Spalten in der ohnehin breiten Tabelle. → Nicht in den Default-Spalten aktivieren, nur auf Wunsch einblendbar.
