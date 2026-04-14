## Context

Das Application-Datenmodell speichert Eintrittserklärungen mit Personendaten (Name, Adresse, E-Mail) und beantragten Anteilen. Das Admin-Formular übergibt die Anrede (`salutation`) derzeit nicht, obwohl das Backend es unterstützt. Ein Titel-Feld ("Dr.", "Prof.") existiert im Application-Modell gar nicht, obwohl Members es haben. Bei Bestätigung einer Application wird ein Member erstellt — der Titel geht dabei verloren.

## Goals / Non-Goals

**Goals:**
- Anrede-Dropdown im Admin-Formular anzeigen und korrekt übergeben
- Titel-Feld (`title: Option<String>`) durch den gesamten Application-Stack hinzufügen
- Titel bei Application-Bestätigung in den neuen Member übernehmen
- Titel auch im öffentlichen Join-Request verfügbar machen
- Typst-Template-Inputs um `title` erweitern

**Non-Goals:**
- Änderungen am Member-Datenmodell (hat `title` bereits)
- Validierung des Titel-Inhalts (Freitext wie beim Member)
- Weitere neue Felder (Telefon, Geburtsdatum, etc.)

## Decisions

### 1. Titel als `Option<Arc<str>>` / `Option<String>`

**Entscheidung:** Gleicher Typ wie beim Member — `Option<Arc<str>>` im DAO/Service, `Option<String>` in REST-Types.

**Begründung:** Konsistenz mit dem bestehenden Member-Modell. Freitext erlaubt "Dr.", "Prof. Dr.", etc.

### 2. DB-Migration: einfaches ALTER TABLE

**Entscheidung:** `ALTER TABLE application ADD COLUMN title TEXT` — SQLite erlaubt nullable Spalten ohne Default.

**Begründung:** Keine bestehenden Daten müssen migriert werden, das Feld ist optional.

### 3. Anrede im Formular als einfaches Select

**Entscheidung:** `<select>` mit Optionen: (leer), Herr, Frau, Firma. Kein neues Feld im Backend nötig.

**Begründung:** `salutation` existiert bereits im `AdminCreateApplicationRequest` und wird aktuell nur als `None` gesendet. Es braucht nur die UI.

### 4. Bestätigung überträgt Titel

**Entscheidung:** `confirm()` in `application.rs` setzt `title` auf den Member, analog zu den anderen Feldern.

**Begründung:** Alle Personendaten sollten bei Bestätigung übernommen werden, damit nichts manuell nachgetragen werden muss.

## Risks / Trade-offs

- **Risiko: Öffentliches Join-Formular** — Falls es ein WordPress-Formular gibt, muss das ggf. auch um das Titel-Feld erweitert werden → Außerhalb des Scopes, die API akzeptiert das Feld einfach optional
- **Trade-off: Kein Detail-View** — Die Application-Detail-Ansicht zeigt den Titel noch nicht an → Kann separat ergänzt werden, ist aber kein Blocker
