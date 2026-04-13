## Context

Die Mitgliederliste (`members.rs`) ist eine feature-reiche Tabelle mit konfigurierbaren Spalten, Inline-Editing und Spalten-Picker. Spalten werden über ein `ColumnDef`-System definiert, das Funktionszeiger für `render`, `get_value`, `set_value` auf `MemberTO`-Feldern nutzt. Eine Upload-Spalte passt nicht in dieses System, da sie keine Dateneigenschaft ist, sondern eine Aktion mit eigenem State.

Dokumente werden bereits pro Mitglied verwaltet (`GET/POST /members/{id}/documents`). Singleton-Typen (JoinDeclaration, JoinConfirmation) erlauben nur ein aktives Dokument — der Upload wird blockiert wenn eines existiert (nach Umsetzung von `generate-and-store-documents`).

Es gibt keinen Endpunkt, der effizient für alle Mitglieder liefert, ob ein Dokument eines bestimmten Typs vorhanden ist.

## Goals / Non-Goals

**Goals:**
- Upload-Spalte in der Mitgliederliste als optionale Spezialspalte
- Globaler Dokumenttyp und Beschreibung für alle Uploads einer Session
- Effizienter Bulk-Endpunkt für Dokumenten-Existenz pro Typ
- Visueller Status pro Zeile (vorhanden / leer / lädt / fertig / fehler)
- Funktioniert unabhängig vom Bearbeitungsmodus

**Non-Goals:**
- Drag-and-Drop Upload (einfacher File-Input reicht)
- Batch-Upload mehrerer Dateien in einem Schritt
- Auto-Matching von Dateinamen zu Mitgliedern
- Änderung des bestehenden `ColumnDef`-Systems

## Decisions

### 1. Upload-Spalte als Spezialspalte neben dem ColumnDef-System

Die Upload-Spalte wird nicht als `ColumnDef` implementiert, sondern als eigenständige optionale Spalte mit eigener Render-Logik. Im Spalten-Picker erscheint sie als zusätzlicher Toggle (visuell abgetrennt), aber ihre Daten und ihr Rendering laufen komplett separat.

**Warum nicht ColumnDef erweitern?** Die `ColumnDef`-Struktur ist auf Datenfelder von `MemberTO` ausgelegt (Funktionszeiger `render`, `get_value`, `set_value`). Eine Upload-Spalte hat keinen Wert in `MemberTO`, braucht async-Operationen und eigenen State pro Zeile. Eine Erweiterung würde das bestehende System unnötig verkomplizieren.

### 2. Bulk-Count-Endpunkt statt Einzelabfragen

Neuer Endpunkt `GET /api/member-documents/counts?type={document_type}` liefert eine Map `{ member_id: count }` für alle Mitglieder, die mindestens ein aktives Dokument des Typs haben. Mitglieder ohne Dokument dieses Typs werden nicht aufgeführt (leere Map = keiner hat eins).

**Warum nicht alle Dokumente laden?** Bei 600 Mitgliedern mit je 1 Dokument wären das 600 vollständige Dokument-Objekte. Der Count-Endpunkt liefert nur IDs und Zahlen — deutlich schlanker.

**Warum nicht in den Member-Endpunkt integrieren?** Das würde einen Join im Standard-Request erzwingen. Lazy Loading per separatem Request hält den Standardfall schlank.

### 3. Lazy Loading der Counts

Die Counts werden erst geladen, wenn **beide** Bedingungen erfüllt sind: Upload-Spalte ist aktiv UND ein Dokumenttyp ist ausgewählt. Bei Typ-Wechsel werden die Counts neu geladen.

### 4. Upload-Status als lokaler Signal-State

Pro Zeile wird ein `HashMap<Uuid, UploadStatus>` geführt:
- `None` (nicht in Map) → File-Input anzeigen
- `Existing(count)` → "vorhanden" anzeigen, kein Upload
- `Uploading` → Spinner
- `Success` → Erfolgsmeldung, danach Count aktualisieren
- `Error(msg)` → Fehlermeldung

### 5. Globale Einstellungen über der Tabelle

Wenn die Upload-Spalte aktiv ist, erscheint ein Einstellungsbereich über der Tabelle mit:
- Dokumenttyp-Dropdown (Pflicht, kein Default)
- Beschreibungsfeld (optional, Freitext)

Beide Werte gelten für alle Uploads. Der Dokumenttyp muss ausgewählt sein, bevor Uploads möglich sind.

### 6. Spalten-Picker-Persistenz

Der Upload-Spalten-Toggle wird **nicht** in den User-Preferences persistiert. Es ist eine Session-Einstellung — beim Neuladen der Seite ist die Spalte wieder ausgeblendet. Die Upload-Spalte ist ein Arbeitsmodus, kein Dauerzustand.

## Risks / Trade-offs

**[Risk] Viele gleichzeitige Uploads** → Uploads laufen sequentiell pro User-Interaktion (ein File-Input pro Zeile). Kein Risiko für Backend-Überlastung, da der User pro Klick nur eine Datei hochlädt.

**[Risk] Stale Counts nach Upload** → Nach erfolgreichem Upload wird der lokale Count für dieses Mitglied inkrementiert, ohne die Counts neu zu laden. Bei Singleton-Typen wechselt der Status auf "vorhanden".

**[Trade-off] Keine Persistenz der Upload-Spalte** → Beim Neuladen muss man die Spalte erneut einblenden und den Typ wählen. Das ist akzeptabel, da es ein temporärer Arbeitsmodus für die Migration ist.

**[Trade-off] Globale Beschreibung für alle** → Individuelle Beschreibungen pro Mitglied sind nicht möglich. Für den Migrations-Use-Case reicht das, da alle Dokumente denselben Kontext haben.
