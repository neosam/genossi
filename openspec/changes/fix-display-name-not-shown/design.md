## Context

Auf der Berechtigungsseite (`permissions.rs`) und der Konfigurationsseite (`config_page.rs`) wird der Anzeigename (`sender_name`) per API geladen, aber nie im Input-Feld dargestellt.

**Berechtigungsseite**: `UserRowComponent` nutzt `use_signal(move || sender_name.clone())` zur Initialisierung des Input-Feldes. In Dioxus wird der Initializer von `use_signal` nur beim ersten Mount ausgeführt. Wenn sich die Props ändern oder die Komponente re-rendert, bleibt der alte Wert bestehen.

**Konfigurationsseite**: `sender_name` wird als Signal mit leerem String initialisiert und async per `sender_name.set(pref.value)` gesetzt. Das sollte eigentlich funktionieren — muss beim Fix verifiziert werden.

## Goals / Non-Goals

**Goals:**
- Der geladene `sender_name` wird korrekt im Input-Feld auf beiden Seiten angezeigt
- Benutzer können den Wert weiterhin editieren und speichern

**Non-Goals:**
- Keine Änderungen am Backend oder an der API
- Keine Änderung der Datenstruktur oder des Speicherverhaltens

## Decisions

### 1. Permissions-Seite: `use_effect` statt reinem `use_signal` Initializer

**Ansatz**: Einen `use_effect` in `UserRowComponent` hinzufügen, der `name_input` aktualisiert, wenn sich der `sender_name` im `users`-Signal ändert.

**Alternative**: Den `sender_name` direkt aus `users.read()[idx]` im RSX lesen, ohne separaten `name_input`-Signal. Das würde aber das Editieren des Feldes (vor dem Speichern) nicht erlauben, da jede Eingabe überschrieben würde.

**Entscheidung**: `use_effect` nutzen, um `name_input` zu synchronisieren wenn sich die Datenquelle ändert, aber dem Benutzer weiterhin erlauben, den Wert lokal zu editieren.

### 2. Config-Seite: Gleicher Fix-Ansatz prüfen

Falls das Problem auch auf der Config-Seite auftritt, den gleichen Ansatz anwenden. Möglicherweise funktioniert die Config-Seite aber schon korrekt, da dort `sender_name.set()` direkt auf dem angezeigten Signal aufgerufen wird (kein separater `name_input`-Signal).

## Risks / Trade-offs

- [Risk: use_effect überschreibt User-Eingaben] → Mitigation: use_effect nur bei initialem Laden triggern oder nur wenn sich der Quellwert tatsächlich ändert
- [Risk: Config-Seite hat ein anderes Problem] → Mitigation: Beide Seiten manuell testen
