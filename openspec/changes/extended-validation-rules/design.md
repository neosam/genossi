## Context

Der Validierungs-Endpoint (`GET /api/validation`) existiert bereits mit zwei Regeln: Mitgliedsnummern-Lücken und Übertragungs-Gegenpart. Die Architektur ist erweiterbar angelegt — neue Regeln werden als zusätzliche Funktionen im `ValidationServiceImpl` implementiert und die Ergebnisse ins `ValidationResult` Struct aufgenommen.

Die bestehende `compute_migration_status`-Funktion in `member_action.rs` berechnet bereits Shares- und Action-Count-Vergleiche pro Mitglied, was für einige Regeln wiederverwendbar ist.

## Goals / Non-Goals

**Goals:**
- 7 neue Validierungsregeln im bestehenden Service und Endpoint
- Schweregrade (Fehler/Warnung/Info) pro Regel, damit das Frontend differenziert darstellen kann
- Frontend-Anzeige für alle neuen Regeln mit Schweregrad-Farben
- Wiederverwendung bestehender Logik wo möglich

**Non-Goals:**
- Automatische Korrekturen
- Konfigurierbare Regeln (ein/ausschaltbar)
- Export der Validierungsergebnisse

## Decisions

### 1. Flache Erweiterung des ValidationResult statt generischer Regel-Liste

Jede neue Regel bekommt ein eigenes Feld im `ValidationResult` Struct (z.B. `shares_mismatches: Arc<[SharesMismatch]>`), analog zu den bestehenden `member_number_gaps` und `unmatched_transfers`.

**Warum:** Typsicher, einfach zu konsumieren im Frontend, konsistent mit der bestehenden Struktur. Eine generische `Vec<ValidationIssue>` wäre flexibler, aber schwerer typisiert und würde die Frontend-Logik komplizieren.

### 2. Schweregrad als Darstellungslogik im Frontend

Der Backend-Response enthält keine expliziten Schweregrade. Das Frontend entscheidet basierend auf dem Feld-Typ, welche Farbe/Icon verwendet wird. Das ist einfacher als ein generisches Severity-Enum durchzureichen.

**Zuordnung:**
- Rot (Fehler): `shares_mismatches`, `missing_entry_actions`, `duplicate_member_numbers`
- Gelb (Warnung): `exit_date_mismatches`, `active_members_no_shares`, `migrated_flag_mismatches`
- Blau (Info): `exited_members_with_shares`

### 3. Wiederverwendung von compute_migration_status für migrated-Flag-Check

Die Funktion `compute_migration_status` ist bereits `pub(crate)` in `member_action.rs`. Für die Validierung importieren wir sie direkt im validation-Modul — sie ist im selben Crate.

**Warum:** Kein Code duplizieren. Die Funktion berechnet genau das, was wir brauchen.

### 4. Alle Regeln in einer Transaktion

Alle Prüfungen laufen in der bestehenden Transaktion des `validate()`-Aufrufs. Die Daten (Members + Actions) werden einmal geladen und für alle Regeln wiederverwendet.

## Risks / Trade-offs

**[Response-Größe]** → Bei vielen Mitgliedern mit Problemen könnte der Response groß werden. Akzeptabel für die erwartete Vereinsgröße.

**[Struct-Erweiterung ist Breaking für Frontend]** → Neue Felder im Response erfordern Frontend-Änderungen. Da beides im selben Repo ist und gleichzeitig deployed wird, kein Problem.
