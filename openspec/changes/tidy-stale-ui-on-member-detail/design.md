## Context

Beide Aufräumarbeiten betreffen `genossi-frontend/src/page/member_details.rs` (1373 Z.) — die größte Page der Anwendung. Sie verfolgen dasselbe Meta-Prinzip „nur anzeigen, wenn relevant", unterscheiden sich aber in der konkreten Bug-Klasse:

- Der Eintrittsbestätigungs-Knopf hat einen Code, der die Sichtbarkeit korrekt steuern wollte, aber der String-Vergleich greift nicht (vermutlich weil das Backend `document_type` anders serialisiert als der hardcoded String `"join_confirmation"`).
- Der Migrationsstatus-Block zeigt im Normalfall ein „grüner Badge ohne Information"-Symbol — UI-Lärm, kein Bug.

## Goals / Non-Goals

**Goals:**
- Bestehender Bug fixen (Eintrittsbestätigung-Knopf verschwindet wirklich, wenn Dokument vorhanden).
- Stillen Lärm aus dem Migrationsstatus-Block entfernen.
- Eine Quelle der Wahrheit für Dokumenttyp-Bezeichner herstellen.

**Non-Goals:**
- Keine umfassende Audit-Aktion über andere „immer sichtbarer Button"-Stellen (eigene Recherche/eigener Change).
- Keine Verallgemeinerung zu einer „auto-hide-when-irrelevant"-Komponente. Die zwei Stellen sind zu spezifisch und zu unterschiedlich.
- Keine Änderung am Migrationsstatus-Backend; nur die Frontend-Anzeige im `migrated`-Zustand entfällt.

## Decisions

### Vergleich über Enum-Wert statt Hardcode

Im Code lebt das Dokumenttyp-Enum bereits in `rest_types::DocumentTypeTO` (siehe `member_details.rs:1057-1069`). Der Vergleich `d.document_type == "join_confirmation"` ignoriert diese Quelle der Wahrheit und führt offensichtlich zu falschem Verhalten. Wir nutzen `DocumentTypeTO::JoinConfirmation.as_str()`. Damit bricht der Build, falls jemand den Enum-Variantennamen ändert — guter Schutz vor künftigen Divergenzen.

*Alternative:* Den Vergleich über `DocumentTypeTO::from_str(&d.document_type) == Some(DocumentTypeTO::JoinConfirmation)` führen. Sauberer, aber mehr Code für denselben Effekt. Der einfache `as_str()`-Vergleich reicht.

### Migrated-Block: ganz weglassen statt „dezenter machen"

Der `migrated`-Badge enthält null Information. „Dezenter darstellen" (kleineres Element, graue Farbe) wäre eine Option, aber genau dasselbe Problem. Klarer ist: ganz raus, der Pending-Block bleibt unverändert (er trägt echte Information).

*Alternative:* Badge in den Header der Detailseite verschieben (z. B. neben den Mitgliedsnamen). Verworfen, weil der Badge im 99 %-Fall reines Rauschen ist.

### Keine zusätzliche Spec-Modifikation an `member-migrated-flag`

Die existierende Capability `member-migrated-flag` ist auf das Backend-Verhalten ausgerichtet (Wann ist ein Mitglied migriert? Wer setzt den Flag?). Sie macht keine Aussage zur UI-Anzeige. Unsere neue Capability `member-detail-ui-tidy` ergänzt das im Frontend, ohne die Backend-Spec anzufassen.

## Risks / Trade-offs

- [Beim Vergleich gegen `as_str()` müssen die Enum-Variantennamen und ihre serialisierten Strings konsistent bleiben] → Akzeptiert. Falls jemand das Enum umbenennt, fällt das in Tests sofort auf, weil Frontend und Backend dasselbe Enum verwenden.
- [Wenn das Backend für `JoinConfirmation` einen abweichenden String liefert, fixt der Refactor das Symptom nicht] → Mitigation: Im Tasks-Block ist eine kurze Verifikation („was kommt im JSON vom Backend?") als erster Schritt vorgesehen, bevor der Refactor erfolgt.
- [Es gibt ggf. Kunden, die den `migrated`-Badge als positives Signal vermissen] → Akzeptiert; bei 99 % Normalfall überwiegt der Wegfall des Lärms.
