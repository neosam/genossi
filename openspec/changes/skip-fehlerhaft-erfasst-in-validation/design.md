## Context

Mitglieder mit Status `FehlerhaftErfasst` waren nie echte Mitglieder — sie entstanden durch Importfehler aus der alten Excel-Liste. Die Validierung behandelt sie aktuell wie normale Mitglieder und meldet False Positives (fehlende Eintritts-Aktionen, Anteile-Inkonsistenzen usw.). Beim Anlegen werden automatisch Eintritt- und Aufstockungsaktionen erstellt, die manuell gelöscht werden müssen.

## Goals / Non-Goals

**Goals:**
- Validierungschecks sollen `FehlerhaftErfasst`-Mitglieder bei mitgliedschaftsbezogenen Prüfungen überspringen
- Beim Erstellen eines `FehlerhaftErfasst`-Mitglieds sollen keine automatischen Actions angelegt werden
- `current_shares` soll bei `FehlerhaftErfasst`-Erstellung auf 0 gesetzt werden

**Non-Goals:**
- Kein automatischer Statuswechsel bei Update (Normal ↔ FehlerhaftErfasst)
- Keine automatische Bereinigung bestehender Actions bei Statuswechsel
- Keine neuen API-Endpunkte oder Datenmodell-Änderungen

## Decisions

### 1. Filter-Strategie: `is_normal()` in jeder Funktion

Jede betroffene Validierungsfunktion erhält einen zusätzlichen `.filter(|m| m.status.is_normal())` in der Member-Iteration. Das ist konsistent mit dem bestehenden Muster in `count_active()`.

**Alternative**: Zentrale Vorfiltierung aller Members vor den Checks. Verworfen, weil Nummern-Lücken und Duplikate weiterhin alle Mitglieder berücksichtigen sollen — eine zentrale Filtierung wäre irreführend.

### 2. Bedingte Action-Erstellung in `create`

Die Eintritt- und Aufstockungsaktionen werden mit einem `if item.status.is_normal()` umschlossen. Bei `FehlerhaftErfasst` wird `current_shares` auf 0 gesetzt, da ohne Aufstockungsaktion die Shares-Konsistenz sonst sofort verletzt wäre.

**Alternative**: Immer Actions erstellen und bei `FehlerhaftErfasst` sofort soft-deleten. Verworfen — unnötige Komplexität.

### 3. Keine Sonderlogik beim Status-Update

Wenn ein bestehendes Mitglied von `Normal` auf `FehlerhaftErfasst` geändert wird, passiert nichts mit den bestehenden Actions. Der Nutzer muss diese manuell bereinigen. Dies ist akzeptabel, weil der Fall in der Praxis selten vorkommt und eine automatische Bereinigung risikobehaftet wäre.

## Risks / Trade-offs

- **[Inkonsistente Daten bei Statuswechsel]** → Mitigation: Kommt in der Praxis kaum vor. Actions können manuell bereinigt werden. Validierung wird die Inkonsistenz nicht anzeigen, aber die Daten sind historisch korrekt.
- **[Vergessener Filter in neuem Check]** → Mitigation: Bestehende Tests als Vorlage, neues Muster ist konsistent und gut sichtbar.
