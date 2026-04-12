## Context

Der Mitgliederstatus wird aktuell implizit aus Actions (Eintritt/Austritt/Todesfall) und Datumsfeldern abgeleitet. Es gibt keinen Mechanismus, um Mitglieder zu kennzeichnen, die nie echte Mitglieder waren (z.B. fehlerhafte Erfassungen). Diese erscheinen faelschlicherweise als aktive Mitglieder.

## Goals / Non-Goals

**Goals:**
- Erweiterbares Enum-Feld `status` auf der Member-Entity
- Fehlerhaft erfasste Mitglieder korrekt aus Aktiv-Zaehlung ausschliessen
- Status beim Anlegen und nachtraeglich setzbar
- Rueckwaertskompatibilitaet: bestehende Mitglieder erhalten `Normal` als Default

**Non-Goals:**
- Kein Ersatz fuer das Action-basierte Statusmodell (Eintritt/Austritt/Todesfall)
- Keine Aenderung der Soft-Delete-Logik
- Keine automatische Erkennung fehlerhafter Erfassungen

## Decisions

### Decision 1: Enum als TEXT in SQLite

Das `MemberStatus`-Enum wird als TEXT-Spalte in SQLite gespeichert (`"Normal"`, `"FehlerhaftErfasst"`).

**Alternativen:**
- INTEGER-Mapping: Kompakter, aber schlechter lesbar in der Datenbank und fragil bei Reordering
- Separate Status-Tabelle: Ueberengineered fuer ein einfaches Enum

**Rationale:** TEXT ist konsistent mit dem bestehenden Muster (z.B. `Salutation`-Enum) und direkt lesbar.

### Decision 2: Status-Feld mit Default-Wert in Migration

Die Migration fuegt die Spalte mit `DEFAULT 'Normal'` hinzu. Bestehende Zeilen erhalten automatisch den Wert.

**Rationale:** Kein separater Daten-Migrationschritt noetig. Neue Spalte ist sofort konsistent.

### Decision 3: Filterlogik im DAO-Layer

`count_active` und aktive Mitglieder-Abfragen filtern auf `status = 'Normal'` zusaetzlich zu den bestehenden Bedingungen.

**Rationale:** Die Filterung gehoert in den DAO-Layer, da sie datenbanknah ist und alle Abfragen konsistent beeinflusst.

### Decision 4: Enum im REST-Layer als String

Das API serialisiert den Status als String (`"Normal"`, `"FehlerhaftErfasst"`). Kein separater Endpoint fuer Statusaenderungen — der Status wird ueber den normalen Update-Endpoint gesetzt.

**Rationale:** Einfach, konsistent mit bestehenden Enum-Feldern (Salutation). Kein Bedarf fuer einen spezialisierten Endpoint.

## Risks / Trade-offs

- **[Enum-Erweiterung]** Neue Werte erfordern Code-Aenderungen in allen Layern und eine ggf. angepasste Filterlogik. → Mitigation: Enum ist bewusst einfach gehalten, neue Werte folgen dem gleichen Muster.
- **[Unbekannte Status-Werte in DB]** Wenn manuell ein ungueltiger Wert in die DB geschrieben wird, schlaegt das Parsing fehl. → Mitigation: Validierung beim Lesen, serde-Deserialisierung faengt ungueltige Werte ab.
