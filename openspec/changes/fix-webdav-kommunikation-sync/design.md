## Context

Die Funktion `sync_communications` in `genossi_backup/src/sync.rs` erstellt Mitglieder-Verzeichnisse unter `{base_dir}/kommunikation/{NNN}_{Nachname}_{Vorname}` mit einem einfachen `mkcol()`-Aufruf. WebDAV MKCOL erfordert jedoch, dass alle Elternverzeichnisse bereits existieren. Da das `kommunikation/`-Verzeichnis nie explizit angelegt wird, schlägt jeder MKCOL-Aufruf mit HTTP 409 Conflict fehl.

Die Schwester-Funktion `sync_documents` zeigt das korrekte Pattern: Sie ruft `mkcol_recursive` für das Basisverzeichnis auf, bevor die Mitglieder-Unterverzeichnisse erstellt werden.

## Goals / Non-Goals

**Goals:**
- Kommunikations-Export über WebDAV funktioniert zuverlässig
- Konsistentes Pattern zwischen `sync_documents` und `sync_communications`

**Non-Goals:**
- Änderung am WebDAV-Client selbst
- Änderung der Verzeichnisstruktur oder Dateibenennung
- Retry-Logik bei WebDAV-Fehlern

## Decisions

### Vorab-Erstellung des kommunikation-Basisverzeichnisses

Vor der for-Schleife wird `mkcol_recursive` für `{base_dir}/kommunikation` aufgerufen, analog zu `sync_documents` Zeile 28-29.

**Alternativen:**
- `mkcol_recursive` statt `mkcol` für jedes `member_dir`: Funktioniert, erzeugt aber unnötige rekursive Aufrufe bei jeder Iteration
- `mkcol_recursive` am Anfang + `mkcol` in der Loop: Das bestehende `mkcol` in der Loop bleibt ausreichend, da nur noch eine Ebene (die Mitglieder-Ebene) fehlt

**Rationale:** Minimaler Eingriff, konsistent mit dem bestehenden Pattern.

## Risks / Trade-offs

- [Minimal] Zusätzlicher HTTP-Request beim Start der Kommunikations-Sync → Wird durch MKCOL auf existierendes Verzeichnis (405 → Ok) abgefangen, kein Performance-Problem
