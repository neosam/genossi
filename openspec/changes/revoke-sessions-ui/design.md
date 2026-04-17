## Context

Der Backend-Endpoint `POST /api/session/revoke-all` existiert bereits (aus `harden-auth-surface`), ebenso die Frontend-API-Funktion `api::revoke_all_sessions()`. Es fehlt nur die UI — ein Button mit Bestätigungsdialog, der diese Funktion aufruft und danach zum Login weiterleitet.

Das Frontend nutzt Dioxus mit Tailwind CSS. Die TopBar (`component/top_bar.rs`) zeigt bereits den Benutzernamen und einen Logout-Link. Bestätigungsdialoge werden im Projekt über die `Modal`-Komponente (`component/modal.rs`) mit einem `use_signal`-Flag-Pattern realisiert (siehe `templates.rs`, `application_detail.rs`).

## Goals / Non-Goals

**Goals:**
- Self-Service "Alle Sessions beenden" für eingeloggte Nutzer
- Bestätigungsdialog vor der destruktiven Aktion
- Nach Revoke: Redirect zum Login-Endpoint
- Konsistenter Look mit bestehenden Bestätigungsdialogen

**Non-Goals:**
- Einzelne Sessions anzeigen oder selektiv beenden (mögliche Zukunft)
- Admin-Sicht auf Sessions anderer Benutzer
- Session-Zähler oder -Liste

## Decisions

### 1. Platzierung: TopBar neben Logout-Link

**Entscheidung:** Der Revoke-Button wird in der TopBar platziert, direkt neben dem bestehenden Logout-Link.

**Alternativen:**
- *Eigene Settings-Page:* Überdimensioniert für einen einzelnen Button, es gibt aktuell keine Settings-Page.
- *Dropdown-Menü am Benutzernamen:* Wäre eleganter, erfordert aber einen neuen UI-Pattern (Dropdown), der aktuell nicht in der TopBar existiert.
- *TopBar direkt:* Nutzt bestehende Struktur, kein neuer Pattern nötig, sofort sichtbar.

**Begründung:** Minimaler Aufwand, maximale Auffindbarkeit. Der Button gehört logisch zum Auth-Bereich der TopBar.

### 2. Bestätigungsdialog: Bestehende `Modal`-Komponente

**Entscheidung:** Wiederverwendung der `Modal`-Komponente mit dem gleichen `use_signal`-Flag-Pattern wie in `templates.rs` und `application_detail.rs`.

**Begründung:** Konsistenter Look, kein neuer Pattern. Das Muster ist im Projekt etabliert: Signal steuert Sichtbarkeit, Modal zeigt Warn-Text + Confirm/Cancel-Buttons.

### 3. Post-Revoke-Verhalten: Redirect zum Backend-Logout

**Entscheidung:** Nach erfolgreichem Revoke wird `window.location` auf den Backend-Logout-Endpoint (`{backend_url}/logout`) gesetzt, identisch zum bestehenden Logout-Link.

**Alternativen:**
- *Dioxus `navigator()`:* Würde nur die SPA-Route ändern, nicht die Session tatsächlich beenden.
- *Eigene "Abgemeldet"-Seite:* Unnötig — der Logout-Endpoint leitet bereits zum OIDC-Provider weiter.

**Begründung:** Da alle Sessions serverseitig revoked sind, muss der Browser-State ebenfalls bereinigt werden. Der Logout-Endpoint macht genau das.

### 4. Neue Komponente: `RevokeSessionsButton`

**Entscheidung:** Eigene Komponente `component/revoke_sessions_button.rs`, die Button + Modal kapselt. Die TopBar bindet sie als `RevokeSessionsButton {}` ein.

**Begründung:** Component-First-Principle. Die Logik (Signal, API-Call, Redirect, Error-Handling) gehört nicht inline in die TopBar.

### 5. Error-Handling: ErrorAlert-Komponente im Modal

**Entscheidung:** Falls der API-Call fehlschlägt (z.B. Netzwerkfehler), wird die `ErrorAlert`-Komponente innerhalb des Modals angezeigt. Der Dialog bleibt offen, damit der Nutzer es erneut versuchen oder abbrechen kann.

**Begründung:** Nutzt das frisch implementierte `friendly-error-messages`-Pattern. Ein gescheiterter Revoke sollte nicht zu einem unklaren Zustand führen.

## Risks / Trade-offs

**[TopBar wird breiter]** → Der neue Button/Link nimmt Platz ein. Auf Mobile ist die TopBar bereits ein Hamburger-Menü, dort wird der Eintrag einfach ein weiteres `li`-Element — kein Layout-Problem.

**[Revoke erfolgreich, aber Redirect schlägt fehl]** → Unwahrscheinlich, da `window.location`-Zuweisungen nicht fehlschlagen. Selbst wenn: beim nächsten Request wird der Nutzer ohnehin zum Login umgeleitet, weil die Session ungültig ist.

**[Doppelklick auf Confirm]** → Ein `loading`-Signal deaktiviert den Button während des API-Calls, verhindert doppelte Requests.
