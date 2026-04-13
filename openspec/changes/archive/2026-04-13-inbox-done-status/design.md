## Context

Das Inbox-System verwendet aktuell ein einzelnes `status`-Feld (String) auf `InboundMail` mit den Werten `new`, `assigned`, `replied`, `archived`, `ignored`. Diese Werte vermischen drei unabhängige Konzepte:

1. **Mitgliedszuordnung** (`assigned_member_id` existiert bereits separat)
2. **Wurde geantwortet?** (aktuell im Status-Feld kodiert)
3. **Ist die Aufgabe erledigt?** (kein Equivalent vorhanden)
4. **IMAP-Archivierung** (aktuell im Status-Feld kodiert)

Das DAO filtert `list_active()` nur `ignored` heraus — `replied` und `archived` Mails bleiben sichtbar.

## Goals / Non-Goals

**Goals:**
- `status`-Feld durch unabhängige Boolean-Felder `replied`, `done`, `archived` ersetzen
- `ignored`-Status und Endpoint entfernen
- Neuer `/api/inbox/{id}/done` Endpoint zum Markieren als erledigt
- `list_active()` filtert nach `done == false` statt `status != 'ignored'`
- DB-Migration: bestehende Daten korrekt überführen

**Non-Goals:**
- Undo/Reopen von erledigten Mails (kann später ergänzt werden)
- Filtermöglichkeit in der REST-API (z.B. Query-Param `?done=true`) — Frontend filtert vorerst client-seitig
- Änderungen am IMAP-Polling oder an der Reply-Logik

## Decisions

### 1. Drei neue Spalten statt eines Enum-Felds

Das `status TEXT`-Feld wird ersetzt durch:
- `replied INTEGER NOT NULL DEFAULT 0`
- `done INTEGER NOT NULL DEFAULT 0`
- `archived INTEGER NOT NULL DEFAULT 0`

**Warum nicht ein Enum?** Die Zustände sind orthogonal. Eine Mail kann gleichzeitig beantwortet, einem Mitglied zugeordnet und noch offen sein. Ein einzelner Status-Wert kann das nicht abbilden.

**Warum keine Timestamps statt Bool?** Timestamps wären semantisch reicher (wann wurde es erledigt?), aber aktuell gibt es keinen Use-Case dafür. YAGNI — falls nötig, kann man später ein `done_at` ergänzen.

### 2. `status`-Feld entfällt vollständig

Das alte `status`-Feld wird in der Migration gelöscht, nicht beibehalten. Es gibt keinen Bedarf für Rückwärtskompatibilität, da es keine externen API-Konsumenten gibt.

### 3. `InboundMail`-Struct wird angepasst

```rust
pub struct InboundMail {
    // ... bestehende Felder ...
    // status: Arc<str>,          // ENTFERNT
    pub replied: bool,            // NEU
    pub done: bool,               // NEU
    pub archived: bool,           // NEU
    pub assigned_member_id: Option<Uuid>,
}
```

### 4. Service-Methode `ignore()` wird zu `mark_done()`

- `ignore()` wird aus dem `InboxService`-Trait entfernt
- Neue Methode `mark_done(id: Uuid) -> Result<InboundMail, MailServiceError>`
- `reply()` setzt nur noch `mail.replied = true`, ändert keine anderen Felder
- `assign_member()` / `unassign()` setzen nur noch `assigned_member_id`, keine Status-Logik

### 5. REST-API-Änderungen

- **Entfernt:** `POST /api/inbox/{id}/ignore`
- **Neu:** `POST /api/inbox/{id}/done`
- **Geändert:** `InboundMailTO` und `InboundMailDetailTO`: `status: String` wird ersetzt durch `replied: bool`, `done: bool`, `archived: bool`

### 6. DAO-Änderung: `list_active()` Semantik

`list_active()` filtert künftig `WHERE done = 0` statt `WHERE status != 'ignored'`. Archivierte Mails werden weiterhin angezeigt (sie können trotzdem offen sein).

## Risks / Trade-offs

- **Breaking API Change** → Es gibt keine externen API-Konsumenten, daher vertretbar. Frontend wird gleichzeitig angepasst.
- **Migration bei laufendem System** → Die Migration ist additiv (neue Spalten + Datenmigration) gefolgt von einem Column-Drop. SQLite unterstützt kein `DROP COLUMN` in älteren Versionen, daher wird die Tabelle neu erstellt.
- **`replied` als separate Spalte bei einmaligem Setzen** → Aktuell wird `replied` nur einmal gesetzt und nie zurückgesetzt. Das ist korrekt — es ist ein Faktum, kein Status.
