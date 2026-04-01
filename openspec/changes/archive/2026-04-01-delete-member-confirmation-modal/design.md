## Context

Die Member-Detail-Seite (`member_details.rs`) hat einen Löschen-Button, der sofort `api::delete_member()` aufruft. Es existiert bereits ein `Modal`-Component in `component/modal.rs`, das aber nicht exportiert und nicht genutzt wird. Das Frontend nutzt Dioxus (Rust/WASM) mit Tailwind CSS und einem i18n-System mit drei Sprachen (En, De, Cs).

## Goals / Non-Goals

**Goals:**
- Bestätigungs-Modal vor jeder Member-Löschung anzeigen
- Das bestehende `Modal`-Component wiederverwenden und exportieren
- i18n-Unterstützung für alle Modal-Texte (En, De, Cs)

**Non-Goals:**
- Generisches Bestätigungs-Modal für andere Entitäten (nur Member)
- Änderungen am Backend oder der Delete-API
- Undo-Funktionalität nach Löschung

## Decisions

### 1. State-Signal für Modal-Sichtbarkeit

Ein `use_signal(|| false)` steuert, ob das Modal angezeigt wird. Der Delete-Button setzt es auf `true`, die Modal-Buttons setzen es auf `false` bzw. führen die Löschung aus.

**Rationale**: Einfachster Ansatz in Dioxus, konsistent mit dem bestehenden State-Management (andere Signals wie `loading`, `error` werden bereits so verwendet).

### 2. Bestehende Modal-Component wiederverwenden

Das existierende `Modal`-Component wird in `component/mod.rs` exportiert und direkt genutzt, anstatt ein neues zu erstellen.

**Rationale**: Vermeidet Code-Duplizierung. Das Component bietet bereits Overlay, Zentrierung und Scrolling.

### 3. Inline-Confirmation statt separater Component

Die Bestätigungslogik wird direkt in `member_details.rs` implementiert, nicht als eigene `ConfirmationModal`-Component.

**Rationale**: Es gibt nur einen Anwendungsfall. Eine Abstraktion wäre verfrüht. Falls zukünftig weitere Bestätigungsdialoge gebraucht werden, kann man dann refactoren.

### 4. i18n-Keys für Modal-Texte

Neue Keys im i18n-System: `DeleteMemberConfirmTitle`, `DeleteMemberConfirmMessage`, `Confirm`. Der bestehende `Cancel`-Key wird wiederverwendet.

**Rationale**: Konsistent mit dem bestehenden i18n-Muster. Alle drei Sprachen müssen bedient werden.

## Risks / Trade-offs

- **[Risk] Modal-Component ungetestet** -> Das Modal wurde bisher nicht aktiv genutzt. Es könnte bei Edge Cases (Mobile, kleine Screens) suboptimal sein. Mitigation: Manuelles Testen auf verschiedenen Bildschirmgrößen.
- **[Trade-off] Inline vs. generisch** -> Die Inline-Lösung ist schneller, aber bei einem zweiten Bestätigungsdialog wäre Refactoring nötig. Akzeptabel bei nur einem Anwendungsfall.
