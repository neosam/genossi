## Context

Auf der Member-Details-Seite gibt es ein Aktions-Formular für Übertragungs-Aktionen (Empfang/Abgabe). Das Gegenmitglied wird aktuell über ein UUID-Textfeld referenziert. Die Mitgliederliste ist bereits über das `MEMBERS` GlobalSignal im Frontend geladen (~1000 Einträge).

## Goals / Non-Goals

**Goals:**
- Benutzerfreundliche Auswahl eines Gegenmitglieds über Name oder Mitgliedsnummer
- Wiederverwendbare Komponente für zukünftige Mitglieder-Auswahl-Szenarien
- Clientseitiges Filtern ohne zusätzliche Backend-Anfragen

**Non-Goals:**
- Serverseitige Such-API (bei ~1000 Mitgliedern nicht nötig)
- Generische Autocomplete-Komponente (nur für Mitglieder)
- Änderungen am Backend oder an der REST-API

## Decisions

### 1. Autocomplete-Dropdown statt Modal

Inline-Autocomplete direkt im Formular, kein separates Modal.

**Rationale:** Weniger Klicks, sofortiges Feedback beim Tippen, kompaktere UX. Ein Modal wäre Overkill für die Auswahl eines einzelnen Eintrags.

**Alternative:** Modal mit durchsuchbarer Tabelle — zu schwergewichtig für diesen Anwendungsfall.

### 2. Clientseitiges Filtern auf MEMBERS GlobalSignal

Die Komponente liest `MEMBERS.read().items` und filtert lokal per Teilstring-Match (case-insensitive) auf `first_name`, `last_name` und `member_number`.

**Rationale:** Die Daten sind bereits geladen. Bei ~1000 Einträgen ist clientseitiges Filtern performant genug.

### 3. Komponenten-API

```rust
#[component]
fn MemberSearch(
    on_select: EventHandler<Option<Uuid>>,
    selected_id: Option<Uuid>,
    exclude_id: Option<Uuid>,
) -> Element
```

- `on_select`: Wird aufgerufen wenn ein Mitglied gewählt oder die Auswahl gelöscht wird
- `selected_id`: Zeigt das aktuell gewählte Mitglied an
- `exclude_id`: Filtert das aktuelle Mitglied heraus (kein Übertrag an sich selbst)

### 4. Ergebnisliste begrenzen

Maximal 10 Ergebnisse im Dropdown anzeigen, um die Darstellung übersichtlich zu halten. Ergebnisse nach Mitgliedsnummer sortiert.

### 5. Anzeige des ausgewählten Mitglieds

Nach Auswahl wird statt des Suchfelds der Name und die Nummer angezeigt mit einem ✕-Button zum Zurücksetzen.

## Risks / Trade-offs

- **[Dropdown überlappt andere Formularelemente]** → `z-index` und absolute Positionierung verwenden. Dropdown unter dem Input-Feld.
- **[MEMBERS noch nicht geladen]** → Komponente zeigt "Laden..." wenn `MEMBERS.loading` true ist. Sollte in der Praxis selten auftreten, da die Mitgliederliste beim App-Start geladen wird.
- **[Klick außerhalb schließt Dropdown nicht]** → `onfocusout`-Event auf dem Container verwenden, um das Dropdown zu schließen.
