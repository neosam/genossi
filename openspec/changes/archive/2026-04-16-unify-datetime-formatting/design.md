## Context

Im Frontend gibt es bereits eine i18n-basierte Datumsformatierung für reine `time::Date`-Werte (`i18n.format_date`, genutzt z. B. in `columns.rs`, `validation.rs`, `member_details.rs:942`). Für Zeitstempel mit Uhrzeit fehlt das Pendant. Dort hat sich entweder lokal etwas gebildet (drei Stellen mit eigenem `format_datetime`) oder es wurde gar nichts formatiert (rohe ISO-Strings inklusive Nanosekunden in der UI).

Backend liefert ISO8601 mit Nanosekunden (siehe `genossi_rest_types/src/lib.rs`). Das ist gut für die maschinenlesbare API; für Anzeigen ist es Lärm.

## Goals / Non-Goals

**Goals:**
- Eine zentrale Quelle für Zeitstempelformatierung im Frontend.
- Lokalisiertes Verhalten konsistent über alle drei Sprachen.
- Beseitigung der drei lokalen Hilfsfunktionen.
- Beseitigung roher ISO-Anzeigen.

**Non-Goals:**
- Keine Änderung am Backend-API-Format.
- Keine Zeitzonen-Konvertierung in diesem Change (Anzeige weiterhin in der Zeitzone, die das Backend liefert; ggf. ein Folge-Change).
- Keine relativen Zeitangaben („vor 5 Minuten") — eigener Folge-Change möglich.
- Keine Änderung an `format_date` für reine Daten.

## Decisions

### Zwei Funktionen statt einer

Wir führen zwei Methoden ein, weil sie unterschiedlichen Anwendungsfällen dienen:

- `format_datetime`: Datum + Stunden:Minuten — für Listen, Inline-Anzeigen, Mail-Historie, Antragsdaten. „Wann ungefähr".
- `format_datetime_long`: zusätzlich Sekunden — für Audit-Log, Timestamp-Verifikation, technische Diagnosen. „Wann genau".

Eine einzelne Funktion mit einem `precision`-Parameter wäre denkbar, aber zwei klar benannte Methoden sind aufrufseitig besser zu lesen und schwerer zu verwechseln.

*Alternative:* Eine Methode mit `Precision::ShortMinutes | Long::Seconds` als Argument. Verworfen wegen schlechterer Lesbarkeit am Aufrufort.

### Backend-Format unverändert lassen

Das Backend liefert weiter ISO8601 mit Nanosekunden. Die Aufgabe der Anzeige ist Frontend-Sache. Vorteil: keine Migration der API-Verträge, keine Auswirkung auf andere Konsumenten der API (z. B. WordPress-Plugin).

### Eingabetyp: ISO-String und time::OffsetDateTime

Manche Stellen halten den Wert bereits als `OffsetDateTime` (typisierte API-Felder), andere als `String` (z. B. in `application_list.rs` und `communication_timeline.rs`). Beide Eingabearten unterstützen wir, um die Migration der Aufrufstellen ohne Typänderung zu ermöglichen.

*Alternative:* Nur `OffsetDateTime` akzeptieren und alle Aufrufstellen typisieren. Sauberer, aber dieser Change soll fokussiert auf Anzeige bleiben — Typumstellungen sind ein eigener Schritt.

### Locale-spezifische Formate

| Locale | format_datetime | format_datetime_long |
|---|---|---|
| de | `16.04.2026 16:03` | `16.04.2026 16:03:34` |
| en | `Apr 16, 2026 04:03 PM` (oder `2026-04-16 16:03`) | … mit Sekunden |
| cs | `16. 04. 2026 16:03` | … mit Sekunden |

Die genauen englischen/tschechischen Formate werden bei der Implementierung mit den Konventionen aus `format_date` abgestimmt, damit Datum und Datetime sich konsistent anfühlen.

### Robustheit bei Parse-Fehlern

Falls ein ISO-String nicht parsebar ist (defensive Programmierung), gibt die Methode den ursprünglichen String zurück, statt das Rendering zu unterbrechen. So kann eine korrupte Backend-Antwort die UI nicht abreißen.

## Risks / Trade-offs

- [Migration aller Aufrufstellen kann übersehen werden] → Mitigation: Im Tasks-Block ist eine `grep`-Suche nach `format_datetime`, rohen Date-Strings und ISO-Patterns vorgesehen.
- [Englisches/tschechisches Format könnte von Nutzern abweichend gewohnt sein] → Akzeptiert; Anpassbarkeit über die jeweilige Locale-Implementierung bleibt erhalten.
- [String-Eingabe ist schwächer typisiert als `OffsetDateTime`] → Bewusst akzeptiert für sanften Migrationspfad. Folge-Change kann typisieren.
- [Tests für Locale-Formate sind aufwändig — drei Sprachen × zwei Methoden] → Wir testen pro Sprache mindestens ein Beispiel je Methode, kein vollständiger Locale-Test pro Aufrufstelle.
