## Context

Mitgliedsanträge können aktuell erstellt, angesehen und bestätigt/abgelehnt werden. Die DAO-Schicht hat bereits eine `update()`-Methode, die aber nicht über Service- und REST-Layer exponiert wird. Im Frontend existiert ein `ApplicationCreateForm`-Komponente mit allen Formularfeldern, die als Basis für ein wiederverwendbares Formular dienen kann.

## Goals / Non-Goals

**Goals:**
- Admins können Antragsfelder (Name, Adresse, Anteile etc.) nachträglich bearbeiten
- Wiederverwendung des bestehenden Create-Formulars für den Edit-Modus
- Optimistic Locking über das `version`-Feld beim Update

**Non-Goals:**
- Änderungshistorie / Audit-Log für Anträge
- Bearbeitung durch Nicht-Admins
- Status-Änderungen über den Edit-Workflow (Confirm/Reject bleiben separate Aktionen)

## Decisions

### 1. Wiederverwendbares ApplicationForm-Komponente

Das bestehende `ApplicationCreateForm` wird zu einem generischen `ApplicationForm` refactored, das über Props zwischen Create- und Edit-Modus unterscheidet.

- **Create-Modus**: Leeres Formular, Submit ruft `create_application()` auf, zeigt "Send Mail"-Checkbox
- **Edit-Modus**: Vorbefüllt mit bestehenden Daten, Submit ruft `update_application()` auf, keine Mail-Checkbox

**Warum nicht zwei getrennte Formulare?** Die Felder sind identisch. Duplizierung würde bei neuen Feldern zu Inkonsistenzen führen. Ein enum-basierter Modus (z.B. `ApplicationFormMode::Create` / `ApplicationFormMode::Edit { application }`) hält die Komponente sauber.

### 2. PUT-Endpoint statt PATCH

`PUT /api/applications/{id}` erwartet alle editierbaren Felder. Das vereinfacht Validierung und ist konsistent mit dem bestehenden API-Stil der Anwendung.

**Alternative PATCH**: Würde partielle Updates erlauben, ist aber komplexer und im Admin-Kontext nicht nötig, da das Formular immer alle Felder sendet.

### 3. Edit-Button in ApplicationDetail

Der "Bearbeiten"-Button wird in der bestehenden `ApplicationDetail`-Modal angezeigt. Klick öffnet das Formular im Edit-Modus als eigenes Modal (Detail-Modal wird geschlossen). Nach erfolgreichem Update wird die Liste aktualisiert.

**Alternative**: Inline-Editing in der Detail-Ansicht. Zu komplex für den Mehrwert.

### 4. Service-Layer Update-Methode

Neue Methode `update_application(id, request)` im `ApplicationService`-Trait. Die Methode:
- Lädt den bestehenden Antrag
- Prüft Version (Optimistic Locking)
- Aktualisiert die Felder
- Ruft `dao.update()` auf

## Risks / Trade-offs

- **[Concurrent edits]** → Optimistic Locking über `version`-Feld. Bei Versionskonflikt gibt der Service einen Fehler zurück, den das Frontend als Hinweis anzeigt.
- **[Formular-Komplexität]** → Der Mode-Enum hält die Unterschiede überschaubar. Die meisten Felder verhalten sich identisch in beiden Modi.
