## Context

Das Genossi-Frontend ist eine Dioxus-Anwendung (Rust/WASM) mit Tailwind CSS. Es nutzt GlobalSignals für State-Management und Coroutines für Service-Logik. Die Member-Detail-Seite (`member_details.rs`) zeigt aktuell nur Stammdaten in einem Formular an. Alle Backend-Endpoints für Member-Actions existieren bereits unter `/api/members/{member_id}/actions`.

## Goals / Non-Goals

**Goals:**
- Aktionen-Liste auf der Member-Detail-Seite mit allen Feldern
- Formular zum Erstellen und Bearbeiten von Aktionen
- Migrations-Status-Anzeige (migriert/ausstehend) mit Soll-Ist-Details
- Konsistente i18n für DE und EN

**Non-Goals:**
- Eigene Aktionen-Seite (bleibt auf Member-Detail-Seite)
- Automatische Paar-Erstellung bei Übertragungen (Caller-Verantwortung laut Backend-Design)
- Bulk-Aktionen oder Import von Aktionen
- Tschechisch-Übersetzung (nur DE/EN)

## Decisions

### 1. Aktionen als Sektion innerhalb der Member-Detail-Seite

Aktionen werden unterhalb der Stammdaten als eigene Sektion angezeigt, nicht als separate Seite. Dies ermöglicht schnelles Nachtragen bei der Migration ohne Seitenwechsel.

**Alternative**: Eigene Route `/members/:id/actions`. Verworfen, weil der Kontext (Mitglied-Info) beim Nachtragen sichtbar bleiben soll.

### 2. Inline-Formular für Aktions-Erstellung

Ein einklappbares Formular direkt über der Aktions-Liste. Beim Bearbeiten wird die selektierte Aktion ins Formular geladen.

**Alternative**: Modal-Dialog. Verworfen, weil bei der Migration viele Aktionen schnell nacheinander erfasst werden müssen — ein Inline-Formular ist dafür effizienter.

### 3. MemberActionTO im Frontend rest-types

`MemberActionTO`, `ActionTypeTO` und `MigrationStatusTO` werden in `genossi-frontend/rest-types/src/lib.rs` definiert — analog zum Backend-Pattern, aber als eigenständige Typen (kein Sharing der Crate, da Frontend WASM-Target hat).

### 4. Migrations-Status als Badge

Ein farbiger Badge am Seitenanfang zeigt den Migrations-Status:
- Grün: "Migriert" — alle Aktionen stimmen
- Orange: "Ausstehend" — mit Details (erwartet X Anteile, hat Y)

### 5. API-Funktionen folgen bestehendem Pattern

Neue async-Funktionen in `api.rs` analog zu den bestehenden Member-Funktionen (`get_member_actions`, `create_member_action`, etc.).

## Risks / Trade-offs

- **Seitenladung**: Member-Detail-Seite lädt jetzt Member UND Actions — zwei API-Calls. → Mitigation: Paralleles Laden via separate Signals.
- **Formular-Komplexität**: ActionType bestimmt, welche Felder sichtbar sind (z.B. transfer_member_id nur bei Übertragung). → Mitigation: Conditional rendering basierend auf ActionType.
