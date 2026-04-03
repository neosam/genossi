## Context

Genossi ist eine REST-API mit Leptos-Frontend für die Verwaltung von Genossenschaftsmitgliedern. Dokumente wie Beitrittsbestätigungen werden aktuell manuell außerhalb der Software erstellt. Es existiert bereits ein Member-Documents-System für das Hochladen und Verwalten von Dateien im Dateisystem (`DOCUMENT_STORAGE_PATH`).

Das System läuft unter NixOS, wo Systemfonts nicht unter Standardpfaden liegen. Typst als Library umgeht dieses Problem, da Fonts in die Binary eingebettet oder im Repo mitgeliefert werden können.

## Goals / Non-Goals

**Goals:**
- Typst als Rust-Crate einbinden für PDF-Generierung ohne externe Dependencies
- Template-Verwaltung im Dateisystem mit vollständiger CRUD-API (Dateien und Ordner)
- Default-Templates via `include_bytes!` einbetten, pro Datei beim Start anlegen falls nicht vorhanden
- Frontend mit CodeMirror-Editor, Dateibaum-Navigation und PDF-Vorschau
- Typst `#import` zwischen Templates funktioniert natürlich über relative Pfade

**Non-Goals:**
- Sandboxing der Typst-Engine (z.B. `#read()` einschränken) — spätere Erweiterung
- Automatisches Speichern generierter PDFs als Member Document — spätere Erweiterung
- WYSIWYG-Editor — Vorstände arbeiten direkt mit Typst-Syntax
- Template-Versionierung — Dateisystem ist die einzige Quelle

## Decisions

### 1. Typst als Library statt CLI-Tool

**Entscheidung:** Typst wird als Rust-Crate (`typst`) direkt in die Anwendung eingebunden.

**Alternativen:**
- Typst CLI als externen Prozess aufrufen → Zusätzliche Dependency, Pfad-Probleme unter NixOS
- HTML-zu-PDF (z.B. headless Chromium) → Schwere Dependency, schlechtere Typografie
- LaTeX → Noch schwerere Dependency, komplexere Syntax

**Begründung:** Typst ist in Rust geschrieben, lässt sich als Library einbinden und erzeugt keine externen Dependencies. Unter NixOS besonders vorteilhaft, da keine Systempfade benötigt werden.

### 2. Dateisystem-basierte Template-Verwaltung

**Entscheidung:** Templates werden im Dateisystem unter `TEMPLATE_PATH` (Default: `./templates`) gespeichert. Die API liest und schreibt direkt Dateien.

**Alternativen:**
- Datenbank-basiert → `#import` zwischen Templates würde einen Custom FileSystem Provider für Typst erfordern. Deutlich mehr Komplexität.
- Fest im Code → Nicht editierbar durch Benutzer.

**Begründung:** Typst's `#import`-Mechanismus arbeitet mit Dateipfaden. Im Dateisystem funktioniert `#import "_layout.typ"` und relative Pfade in Unterordnern natürlich, ohne Custom-Code.

### 3. Default-Templates via `include_bytes!`

**Entscheidung:** Default-Templates (z.B. `join_confirmation.typ`, `_layout.typ`) werden via `include_bytes!` in die Binary eingebettet. Beim App-Start wird für jedes eingebettete Template geprüft, ob die Datei im `TEMPLATE_PATH` existiert. Nur fehlende Dateien werden angelegt.

**Alternativen:**
- Templates als externe Dateien mitliefern → Pfad-Probleme bei Deployment, besonders unter NixOS
- Keine Defaults → Benutzer startet mit leerem Editor

**Begründung:** Pro-Datei-Prüfung ermöglicht es, in späteren Versionen neue Default-Templates (z.B. `leave_declaration.typ`) nachzuliefern, ohne bestehende Benutzer-Anpassungen zu überschreiben.

### 4. REST-API mit Wildcard-Pfaden

**Entscheidung:** Template-Endpunkte verwenden Wildcard-Pfade (`{path..}`), um beliebige Verzeichnisstrukturen abzubilden.

```
GET    /api/templates                          → Dateibaum (rekursiv)
GET    /api/templates/{*path}                  → Dateiinhalt lesen
PUT    /api/templates/{*path}                  → Datei erstellen/aktualisieren
DELETE /api/templates/{*path}                  → Datei oder leeren Ordner löschen
POST   /api/templates/{*path}/render/{member_id} → PDF generieren
```

**Begründung:** Wildcard-Pfade bilden die Dateisystemstruktur natürlich ab. Ordner werden implizit durch Dateierstellung angelegt (wie `mkdir -p`).

### 5. Pfad-Traversal-Schutz

**Entscheidung:** Alle Pfade aus der API werden validiert — `..`-Segmente und absolute Pfade werden abgelehnt. Der resultierende Pfad muss innerhalb von `TEMPLATE_PATH` liegen.

**Begründung:** Ohne Validierung könnten Benutzer über `../../../etc/passwd` auf beliebige Dateien zugreifen.

### 6. Fonts im Repo einbetten

**Entscheidung:** Eine freie Schriftfamilie (z.B. Liberation Sans/Serif oder Noto Sans) wird im Repo unter `fonts/` mitgeliefert und der Typst-Engine beim Rendern übergeben. Zusätzlich werden Typst's eingebaute Fonts (New Computer Modern) verwendet.

**Begründung:** NixOS-Kompatibilität — kein Zugriff auf Systemfonts nötig. Die Binary ist autark.

### 7. Template-Variablen als Typst-Dictionary

**Entscheidung:** Mitgliedsdaten werden der Typst-Engine als Dictionary-Variable übergeben. Templates greifen darauf über `#member.first_name`, `#member.join_date` etc. zu.

**Begründung:** Saubere Trennung zwischen Template und Daten. Typst unterstützt Dictionaries nativ. Kein fehleranfälliges String-Replace nötig.

## Risks / Trade-offs

- **[Typst-Crate Stabilität]** Typst ist relativ jung, API-Änderungen möglich → Mitigation: Version pinnen, bei Updates testen
- **[Kompilierzeit]** Typst als Crate erhöht die Build-Zeit → Mitigation: Feature-Flag, nur bei Bedarf kompilieren
- **[Dateisystem-Zugriff]** Templates liegen im Dateisystem, kein Backup in DB → Mitigation: Reguläres Filesystem-Backup empfehlen
- **[Kein Sandboxing]** `#read()` in Typst kann Dateien lesen → Mitigation: Vorstände sind vertrauenswürdig, Sandboxing als spätere Erweiterung
- **[Binary-Größe]** Eingebettete Fonts und Templates erhöhen die Binary-Größe → Mitigation: Liberation-Familie ist ~2MB, akzeptabel
