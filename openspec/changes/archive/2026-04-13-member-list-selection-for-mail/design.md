## Context

Die Mitgliederliste (`members.rs`) zeigt eine filterbare Tabelle aller Mitglieder. Klick auf eine Zeile navigiert zur Detailseite. Die Mail-Seite (`mail_page.rs`) hat eine eigene Empfänger-Auswahl per Autocomplete und einen "Alle"-Button. Es gibt keinen Weg, eine gefilterte Teilmenge aus der Mitgliederliste direkt als Mail-Empfänger zu übernehmen.

Die Mail-Seite stellt Empfänger als `flex-wrap` Chips dar, was bei >20 Empfängern unübersichtlich wird und bei >100 die Seite praktisch unbenutzbar macht.

## Goals / Non-Goals

**Goals:**
- Checkbox-Selektion in der Mitgliederliste mit mobilfreundlichem Touch-Target
- "Alle gefilterten auswählen/abwählen" via Checkbox im Tabellenkopf
- Übergabe der Selektion an die Mail-Seite via Navigation
- Skalierbare Empfänger-Darstellung auf der Mail-Seite (1–500+ Empfänger)
- Generischer Selektions-Mechanismus, der für zukünftige Bulk-Aktionen erweiterbar ist

**Non-Goals:**
- Weitere Bulk-Aktionen außer Mail (kommt später)
- Änderungen am Backend oder an der REST-API
- Pagination der Mitgliederliste
- Persistenz der Selektion über Browser-Reload hinaus

## Decisions

### 1. GlobalSignal für Selektions-Übergabe

**Entscheidung**: Neuer `GlobalSignal<Vec<Uuid>>` (`SELECTED_MEMBER_IDS`) in einem eigenen State-Modul.

**Alternativen:**
- *URL-Query-Parameter*: Würde bei vielen IDs extrem lange URLs erzeugen, Browser-Limits bei ~2000 Zeichen
- *Route-State*: Dioxus Router unterstützt keinen typisierten State-Transfer zwischen Routen

**Vorteil**: Konsistent mit bestehendem Pattern (`MEMBERS` GlobalSignal). Einfach, keine Serialisierung nötig.

### 2. Checkbox-Spalte statt Selektionsmodus

**Entscheidung**: Permanente Checkbox-Spalte als erste Spalte in der Tabelle. Checkbox-Klick selektiert, Rest der Zeile navigiert weiterhin zum Detail.

**Alternativen:**
- *Toggle-Selektionsmodus*: Komplexere UX, Nutzer muss Modus erst aktivieren
- *Long-Press auf Mobile*: Nicht intuitiv, kein visuelles Feedback

**Touch-Target**: Checkbox-Zelle bekommt mindestens 44x44px Klickfläche (Apple HIG / WCAG 2.5.8). `onclick` auf der `<td>` statt auf der `<input>`, mit `e.stop_propagation()` damit der Zeilen-Klick nicht triggert.

### 3. Aktionsleiste bei aktiver Selektion

**Entscheidung**: Sticky-Leiste am unteren Bildschirmrand (oder oberhalb der Tabelle), die nur erscheint wenn `selected_count > 0`. Zeigt Anzahl und "Mail senden"-Button.

**Rationale**: Erweiterbar für zukünftige Aktionen (Export, Dokument generieren). Sticky-Leiste bleibt sichtbar beim Scrollen.

### 4. Eingeklappte Empfänger-Darstellung auf Mail-Seite

**Entscheidung**: Standardmäßig eingeklappt mit Zusammenfassung ("187 Empfänger ausgewählt, 3 ohne E-Mail"). Aufklappbar zu einer scrollbaren Liste (`max-h-60 overflow-y-auto`) mit Name, E-Mail und Entfernen-Button pro Zeile.

**Alternativen:**
- *Chips beibehalten mit Scroll-Container*: Chips sind bei vielen Einträgen trotzdem unübersichtlich
- *Nur Zähler ohne Details*: Nutzer kann einzelne Empfänger nicht entfernen

### 5. Mail-Seite übernimmt GlobalSignal einmalig

**Entscheidung**: Die Mail-Seite liest `SELECTED_MEMBER_IDS` beim Mount als initialen Wert für `selected_member_ids` und leert dann das GlobalSignal. Danach arbeitet die Mail-Seite mit ihrem lokalen State.

**Rationale**: Verhindert, dass Änderungen auf der Mail-Seite (Empfänger entfernen) den GlobalSignal beeinflussen. Rückkehr zur Mitgliederliste zeigt keine "Geister-Selektion".

## Risks / Trade-offs

- **Kein Persistenz über Reload**: Selektion geht bei Page-Refresh verloren → Akzeptabel, da die Selektion ein kurzlebiger Workflow-Schritt ist
- **GlobalSignal-Kopplung**: Mitgliederliste und Mail-Seite teilen sich einen Signal → Minimal, da das Signal beim Mail-Seiten-Mount geleert wird
- **Große Selektionen**: 500+ Checkboxen in der Tabelle könnten bei schwacher Hardware laggen → Unwahrscheinlich, da Dioxus Virtual-DOM-Diffing nutzt und nur geänderte Checkboxen re-rendert
