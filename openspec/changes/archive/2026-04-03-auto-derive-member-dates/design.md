## Context

Member-Entities haben `join_date` und `exit_date` als direkte Felder. Gleichzeitig existieren MemberActions (Eintritt, Austritt, Todesfall), deren Datum dieselbe Information trägt. Aktuell werden die Member-Felder nicht automatisch aus den Aktionen synchronisiert. Die Aktionen sollen die einzige Source of Truth sein.

Bestehender Mechanismus als Vorbild: `recalc_migrated()` wird bereits nach jedem Action create/update/delete aufgerufen und aktualisiert das `migrated`-Flag am Member. Das gleiche Muster soll für `join_date`/`exit_date` angewendet werden.

## Goals / Non-Goals

**Goals:**
- `join_date` und `exit_date` automatisch aus Aktionen ableiten nach jedem Action create/update/delete
- `effective_date` bei Austritt-Aktionen als Pflichtfeld erzwingen
- `join_date`/`exit_date` im Frontend bei bestehenden Mitgliedern read-only anzeigen
- Robustes Verhalten bei importierten Mitgliedern ohne Eintritt-Aktion

**Non-Goals:**
- Entfernung von `join_date`/`exit_date` aus dem Member-Datenmodell (bleiben als abgeleitete Felder bestehen)
- Handling von Mehrfach-Eintritten (kommt praktisch nicht vor)
- Änderung der API-Response-Struktur

## Decisions

### 1. Neue Funktion `recalc_dates()` analog zu `recalc_migrated()`

Die Funktion wird in `member_action_service` implementiert und nach jedem Action create/update/delete aufgerufen — direkt neben dem bestehenden `recalc_migrated()`-Aufruf. Auch `member_service.create()` ruft sie auf, da dort automatisch eine Eintritt-Aktion angelegt wird.

**Logik:**
```
recalc_dates(member_id, tx):
  actions = find_by_member_id(member_id)
  member = find_by_id(member_id)

  eintritt = actions.find(type == Eintritt)
  if eintritt:
    member.join_date = eintritt.date

  austritt = actions.find(type == Austritt)
  todesfall = actions.find(type == Todesfall)

  if austritt:
    member.exit_date = austritt.effective_date
  else if todesfall:
    member.exit_date = todesfall.date
  else:
    member.exit_date = None

  member_dao.update(member)
```

**Alternative erwogen**: Eigene DAO-Methode `update_dates()` analog zu `update_migrated()`. Verworfen, da `member_dao.update()` bereits existiert und die Gesamtentität speichert. Eine zusätzliche Methode würde unnötige Komplexität einführen.

### 2. `effective_date` Pflicht bei Austritt

Die Validierung in `validate_action()` wird erweitert: Wenn `action_type == Austritt` und `effective_date` nicht gesetzt ist, wird ein Validierungsfehler zurückgegeben. Begründung: Laut Satzung wird der Austritt immer erst am Ende des Geschäftsjahres wirksam — `effective_date` ist daher immer bekannt.

### 3. Kein Überschreiben ohne Eintritt-Aktion

Wenn kein Eintritt vorhanden ist (z.B. importierte Mitglieder, deren Aktionen noch nicht vollständig migriert sind), bleibt `join_date` unverändert. Das schützt importierte Daten.

### 4. Frontend: Read-only bei bestehenden Mitgliedern

`join_date` und `exit_date` werden im Member-Detailformular bei bestehenden Mitgliedern als read-only Felder angezeigt. Beim Neuanlegen bleibt `join_date` editierbar, da es das Datum der automatisch angelegten Eintritt-Aktion bestimmt.

## Risks / Trade-offs

- **Risk**: Bestehende Mitglieder mit Austritt-Aktionen ohne `effective_date` werden durch die neue Validierung beim Update blockiert → Mitigation: Die neue Validierung greift nur bei create/update von Aktionen. Bestehende Daten in der DB werden nicht validiert. Die Validierungsdaten werden nur bei neuen API-Calls geprüft.
- **Risk**: `recalc_dates()` und `recalc_migrated()` laden beide Actions und Member separat → Mitigation: Akzeptabler Overhead, da immer innerhalb einer Transaktion und auf kleinen Datenmengen. Optimierung kann später erfolgen.
