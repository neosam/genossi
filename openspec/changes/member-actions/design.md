## Context

Genossi verwaltet Genossenschafts-Mitglieder mit Stammdaten und Anteils-Informationen. Aktuell werden Anteile als Snapshot-Werte (`shares_at_joining`, `current_shares`) am Member gespeichert. Es gibt keine Historie, wie sich Anteile über die Zeit verändert haben.

Die Daten werden initial per Excel-Import eingelesen. Die Excel-Liste enthält Spalten "Anteile Beitritt", "Anteile aktuell" und "Anzahl Aktionen", aber keine detaillierte Aktions-Historie. Genossi soll diese Historie vollständig abbilden und beim Migrationsweg unterstützen.

Es gibt ~600 Mitglieder. Etwa zwei Drittel haben `action_count == 0` und `shares_at_joining == current_shares` — diese können automatisch migriert werden.

## Goals / Non-Goals

**Goals:**
- Vollständige Anteils-Historie pro Mitglied als eigene Entity
- Unterscheidung zwischen Status-Aktionen (Eintritt, Austritt, Todesfall) und Anteils-Aktionen (Aufstockung, Verkauf, Übertragung)
- Migrations-Validierung: Abgleich nachgetragener Aktionen gegen Excel-Import-Werte
- Automatische Eintritts-Aktion-Erzeugung für auto-migrierbare Mitglieder beim Import
- Übertragungen bilden Paare: Abgabe bei einem Mitglied, Empfang beim anderen

**Non-Goals:**
- Automatische Berechnung von `current_shares` aus Aktionen (bleibt vorerst manuell/Import-Wert)
- Workflow-Engine für Genehmigungen (Aktionen werden direkt erfasst)
- Frontend für Migrations-Dashboard (Validierung erstmal nur über API)

## Decisions

### 1. MemberAction als eigenständige Entity im bestehenden Schicht-Modell

MemberAction folgt dem gleichen Muster wie Member: DAO-Entity → Service-Typ → REST-TO. Das DAO hat `dump_all`, `create`, `update` mit Default-Implementierungen für `all` und `find_by_id`.

**Zusätzlich**: `find_by_member_id` als Default-Implementierung auf dem DAO-Trait (filtert `dump_all` nach `member_id`).

**Alternative**: Aktionen als Teil des Member-DAO. Verworfen, weil es die Trennung der Concerns verletzt und das Member-DAO unnötig komplex macht.

### 2. Aktionstypen als Enum mit zwei Kategorien

```
StatusAktionen (shares_change = 0):
  Eintritt, Austritt, Todesfall

AnteilsAktionen (shares_change ≠ 0):
  Aufstockung (+N), Verkauf (-N),
  ÜbertragungEmpfang (+N), ÜbertragungAbgabe (-N)
```

In SQLite als TEXT gespeichert (wie bestehende Enum-Patterns).

**Alternative**: Zwei getrennte Tabellen für Status- und Anteils-Aktionen. Verworfen, weil eine einheitliche Timeline pro Mitglied wertvoller ist.

### 3. Übertragung als zwei verknüpfte Aktionen

Eine Übertragung erzeugt immer ein Paar:
- `ÜbertragungAbgabe` bei Mitglied A (shares_change: -N, transfer_member_id: B)
- `ÜbertragungEmpfang` oder `Aufstockung` bei Mitglied B (shares_change: +N, transfer_member_id: A)

Beide Aktionen referenzieren das jeweils andere Mitglied über `transfer_member_id`. Die Konsistenz ist Caller-Verantwortung — der Caller muss beide Aktionen nacheinander über die gleiche API erstellen. Eine dedizierte `create_transfer_pair()`-Methode kann bei Bedarf später ergänzt werden.

**Alternative**: Eine Übertragung-Entity mit von/an Feldern. Verworfen, weil es die pro-Mitglied-Timeline bricht.

### 4. Austritt hat zusätzliches effective_date

Der Austritt hat neben dem `date` (Willensbekundung) ein `effective_date` (Wirksamkeit laut Satzung). Dieses Feld ist `Option<Date>` auf der MemberAction und nur für Austritt relevant. Todesfall ist sofort wirksam und braucht kein `effective_date`.

### 5. action_count als permanentes Feld am Member

Das Feld `action_count` wird aus der Excel-Spalte "Anzahl Aktionen" importiert und dauerhaft am Member gespeichert. Es zählt nur Anteils-Aktionen (Aufstockung, Verkauf, Übertragung) — nicht den Eintritt.

### 6. Auto-Migration beim Import

Beim Excel-Import wird für Mitglieder mit `action_count == 0` UND `shares_at_joining == current_shares` automatisch eine Eintritts-Aktion erzeugt (Datum = `join_date`, shares_change = 0). Eine Aufstockung mit `shares_change = shares_at_joining` wird ebenfalls erzeugt.

Mitglieder, die diese Bedingung nicht erfüllen, erhalten keine automatischen Aktionen — sie müssen manuell nachgetragen werden.

### 7. REST-Endpoints als Sub-Resource von Member

Aktionen werden unter `/api/members/{member_id}/actions` exponiert. Das entspricht der fachlichen Zuordnung und hält die API-Struktur sauber.

## Risks / Trade-offs

- **Doppelte Wahrheit während Migration**: `current_shares` am Member und Σ Aktionen können divergieren. → Mitigation: Validierungs-Endpoint zeigt Differenzen. Langfristig wird `current_shares` aus Aktionen berechnet.
- **Übertragung-Konsistenz**: Beide Seiten einer Übertragung müssen vom Caller nacheinander erzeugt werden. → Mitigation: Validierung prüft, dass transfer_member_id auf ein existierendes Mitglied zeigt. Eine dedizierte `create_transfer_pair()`-Methode kann bei Bedarf ergänzt werden.
- **action_count-Semantik**: Die Excel-Spalte "Anzahl Aktionen" zählt möglicherweise anders als erwartet. → Mitigation: Validierung zeigt Abweichungen, Nutzer kann korrigieren.
