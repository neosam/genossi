## Context

Das Template-System rendert aktuell Typst-Templates ausschließlich mit Member-Daten. Der Render-Endpoint (`POST /api/templates/render/{path}/{member_id}`) ruft `build_inputs()` auf, das ein JSON-Dict mit Key `member` und `today` erzeugt. Im Frontend-Template-Editor kann über die `MemberSearch`-Komponente ein Member für die Preview ausgewählt werden.

Applications (Eintrittserklärungen) haben ein ähnliches aber unterschiedliches Datenmodell — sie enthalten Adressdaten und beantragte Anteile, aber keine Member-spezifischen Felder wie `member_number`, `join_date` oder `current_balance`.

## Goals / Non-Goals

**Goals:**
- Eigener Render-Endpoint für Application-Daten, parallel zum bestehenden Member-Endpoint
- Application-Daten als eigener JSON-Key (`application`) an Typst übergeben
- Im Template-Editor zwischen Member- und Application-Preview umschalten können
- Nur offene Applications (Status "Offen") zur Auswahl anbieten

**Non-Goals:**
- Generischer Render-Endpoint für beliebige Entitäten
- Automatisches Versenden von Briefen
- Zusätzliche Stammdaten (Betrag pro Anteil, Bankverbindung der Genossenschaft)

## Decisions

### 1. Eigener Endpoint statt generischer Lösung

**Entscheidung:** Separater Endpoint `POST /api/templates/render-application/{path}/{application_id}` statt eines generischen `render/{entity_type}/{path}/{id}`.

**Begründung:** Member und Application sind unterschiedliche Datenmodelle mit unterschiedlichen JSON-Strukturen. Ein eigener Endpoint ist einfacher, typsicher und erfordert keine Runtime-Dispatch-Logik. Wenn weitere Entitäten dazukommen, kann man das Pattern wiederholen oder dann generalisieren.

### 2. JSON-Key `application` statt `member`

**Entscheidung:** Application-Daten werden unter dem Key `application` in `sys.inputs` bereitgestellt, nicht unter `member`.

**Begründung:** Templates sind entweder für Member oder für Applications geschrieben. Unterschiedliche Keys machen die Intention klar und verhindern Verwechslungen. Ein Template, das `sys.inputs.at("application")` verwendet, funktioniert nicht versehentlich mit dem Member-Endpoint — das ist gewollt.

### 3. ApplicationSearch als eigene Komponente

**Entscheidung:** Neue `ApplicationSearch`-Komponente analog zu `MemberSearch`, statt `MemberSearch` zu generalisieren.

**Begründung:** Die Suchlogik unterscheidet sich (Applications haben keine Mitgliedsnummer, dafür Status-Filter auf "Offen"). Das Display-Format ist anders. Eine eigene Komponente ist klarer als eine mit Flags überladene generische Komponente.

### 4. Toggle-UI im Template-Editor

**Entscheidung:** Zwei Tabs/Buttons ("Mitglied" / "Antrag") über dem Suchfeld, die zwischen `MemberSearch` und `ApplicationSearch` umschalten.

**Begründung:** Einfaches UI-Pattern. Beim Umschalten wird die aktuelle Auswahl zurückgesetzt. Der Render-Button ruft je nach aktivem Tab den passenden Endpoint auf.

### 5. Application-Daten für Typst

Die `build_inputs_application()` Funktion erzeugt folgende Struktur:

```json
{
  "application": "{\"first_name\":\"...\",\"last_name\":\"...\",\"salutation\":\"...\",\"email\":\"...\",\"street\":\"...\",\"house_number\":\"...\",\"postal_code\":\"...\",\"city\":\"...\",\"shares\":3,\"status\":\"Offen\",\"created\":\"14.04.2026\"}",
  "today": "14.04.2026"
}
```

## Risks / Trade-offs

- **Risiko: Code-Duplikation** zwischen Member- und Application-Render-Logik → Akzeptabel, da der gemeinsame Code minimal ist (Typst-Kompilierung wird wiederverwendet, nur `build_inputs` ist unterschiedlich)
- **Risiko: Leere Adressfelder** bei Applications, die vom Admin ohne Adresse angelegt wurden → Template muss damit umgehen (ist Verantwortung des Template-Autors)
- **Trade-off: Nur offene Applications** im Frontend-Dropdown → Reduziert Auswahlmöglichkeiten, aber Briefe an bereits bestätigte/abgelehnte Anträge sind kein Use-Case
