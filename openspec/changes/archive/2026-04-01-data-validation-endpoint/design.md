## Context

Genossi verwaltet Mitglieder mit fortlaufenden Mitgliedsnummern und Aktionen (Eintritt, Austritt, Übertragungen etc.). Nach einem Excel-Import wurde festgestellt, dass ein Mitglied fehlte — es gibt aktuell keinen Mechanismus, um die Datenintegrität systematisch zu prüfen.

Bestehende Infrastruktur:
- `MemberDao` liefert alle Mitglieder (inkl. soft-deleted via `dump_all`)
- `MemberActionDao` liefert alle Aktionen
- Übertragungen haben `UebertragungAbgabe` / `UebertragungEmpfang` mit `transfer_member_id`
- Das Layer-System (DAO → Service → REST) ist gut etabliert

## Goals / Non-Goals

**Goals:**
- Globaler Validierungs-Endpoint der alle Regeln auf einmal prüft
- Erkennung von Lücken in Mitgliedsnummern (Bereich min..max)
- Erkennung von Übertragungen ohne korrespondierenden Gegenpart
- Strukturierte, erweiterbare Ergebnis-Rückgabe
- Frontend-Seite zur Anzeige der Validierungsergebnisse

**Non-Goals:**
- Automatische Korrektur von Inkonsistenzen (nur Erkennung)
- Validierung bei Schreiboperationen (bleibt post-hoc)
- Einzelmitglied-Validierung (nur globale Prüfung)

## Decisions

### 1. Eigenständiger ValidationService statt Erweiterung bestehender Services

Der Validierungs-Service wird als neuer `ValidationService` Trait implementiert, nicht als Erweiterung von `MemberService` oder `MemberActionService`.

**Warum:** Die Validierung ist eine querschneidende Funktion, die sowohl Member- als auch Action-Daten benötigt. Ein eigener Service hält die bestehenden Services schlank und ermöglicht einfaches Hinzufügen neuer Regeln.

**Alternative:** Methode auf `MemberService` — abgelehnt, weil es MemberService mit Action-DAO-Dependency belasten würde.

### 2. Einzelner Endpoint statt mehrerer

Ein einziger `GET /api/validation` Endpoint führt alle Regeln aus und gibt ein aggregiertes Ergebnis zurück.

**Warum:** Einfacher zu konsumieren und die Datenmenge (alle Members + alle Actions) muss nur einmal geladen werden. Separate Endpoints würden die gleichen Daten redundant laden.

### 3. Mitgliedsnummern-Lücken: Bereich min..max

Die Prüfung ermittelt die minimale und maximale Mitgliedsnummer aller Mitglieder (inkl. soft-deleted) und meldet fehlende Nummern dazwischen.

**Warum:** Der Verein könnte nicht bei 1 anfangen. Der Bereich min..max ist die natürliche Erwartung.

### 4. Übertragungs-Matching: member_id + transfer_member_id + shares + date

Zwei Übertragungsaktionen gelten als Paar, wenn:
- Die member_id der einen Aktion der transfer_member_id der anderen entspricht (und umgekehrt)
- shares_change spiegelsymmetrisch ist (z.B. -3 und +3)
- Das Datum identisch ist
- Die Typen komplementär sind (Abgabe ↔ Empfang)

**Warum:** Alle vier Kriterien zusammen ergeben eine eindeutige Zuordnung. Nur member_id + transfer_member_id wäre zu ungenau bei mehreren Übertragungen zwischen denselben Mitgliedern.

### 5. Keine eigene DAO-Schicht nötig

Der ValidationService nutzt die bestehenden `MemberDao` und `MemberActionDao` direkt. Es werden keine neuen Tabellen oder Queries benötigt.

**Warum:** Die Validierung liest nur bestehende Daten und berechnet daraus Ergebnisse. Eine eigene DAO-Schicht wäre Overhead ohne Nutzen.

## Risks / Trade-offs

**[Performance bei großen Datenmengen]** → Bei sehr vielen Mitgliedern/Aktionen könnte der Endpoint langsam werden, da alle Daten geladen werden. Akzeptabel für die erwartete Größe (Genossenschaft). Bei Bedarf später optimierbar durch gezielte SQL-Queries.

**[Erweiterbarkeit vs. Einfachheit]** → Das Ergebnis-Struct hat dedizierte Felder pro Regel statt einer generischen Liste. Einfacher zu typisieren und zu konsumieren, aber erfordert Struct-Änderungen bei neuen Regeln. Akzeptabler Trade-off für die absehbare Anzahl an Regeln.
