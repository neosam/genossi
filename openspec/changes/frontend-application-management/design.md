## Context

Das Backend hat vollständige CRUD-Endpoints für Beitrittserklärungen (`ApplicationTO` mit Feldern: id, first_name, last_name, salutation, email, street, house_number, postal_code, city, shares, status, created, deleted, version). Status-Werte: `Offen`, `Bestaetigt`, `Abgelehnt`. Die REST-Types (`ApplicationTO`, `ApplicationStatusTO`) sind in `genossi_rest_types` definiert und können über das `rest-types` Crate im Frontend wiederverwendet werden.

Das Frontend folgt dem Pattern: Page → Komponenten, API-Aufrufe in `api.rs`, Routing in `router.rs`, Navigation in `top_bar.rs`. Bestehende Seiten wie `members.rs` und `inbox_page.rs` bieten gute Vorbilder.

## Goals / Non-Goals

**Goals:**
- Übersichtliche Listenseite mit Status-Filter (Tabs oder Dropdown)
- Detailansicht mit allen Antragsdaten
- Bestätigen/Ablehnen mit Bestätigungsdialog (existierendes `Modal`-Component)
- Admin-only Navigation

**Non-Goals:**
- Bearbeiten von Beitrittserklärungen (es gibt keinen Update-Endpoint)
- Bulk-Aktionen (mehrere Anträge gleichzeitig bestätigen/ablehnen)
- E-Mail-Vorschau der Bestätigungsmail

## Decisions

### 1. Eigene Seite statt Unterseite

**Entscheidung:** Neue Route `/applications` mit eigener Seite `applications_page.rs`, nicht als Tab in der Mitglieder- oder Config-Seite.

**Warum:** Beitrittserklärungen sind ein eigener Workflow mit eigenem Status-Lebenszyklus. Admins sollen direkt dorthin navigieren können.

### 2. Liste + Inline-Detail statt Master-Detail mit separater Route

**Entscheidung:** Die Listenseite zeigt alle Anträge als Tabelle/Karten. Beim Klick auf einen Antrag öffnet sich ein Detail-Panel (Modal oder expandierbare Zeile) — keine separate `/applications/:id`-Route nötig.

**Alternativen:**
- Separate Detail-Route: Mehr Overhead, für die wenigen Felder eines Antrags nicht nötig
- Nur Liste ohne Detail: Zu wenig Information auf einen Blick

### 3. Status-Filter als Tabs

**Entscheidung:** Tabs "Alle" / "Offen" / "Bestätigt" / "Abgelehnt" oben auf der Seite. Default-Tab: "Offen", da das der häufigste Anwendungsfall ist.

### 4. Component-First

**Entscheidung:** Die Seite wird aus Komponenten zusammengesetzt:
- `ApplicationList` — Tabelle/Karten der Anträge
- `ApplicationDetail` — Detailansicht eines Antrags (im Modal)
- Page `ApplicationsPage` orchestriert nur

### 5. REST-Types aus dem bestehenden Crate

**Entscheidung:** `ApplicationTO` und `ApplicationStatusTO` werden direkt aus `rest-types` im Frontend genutzt, kein Duplizieren von Structs.

## Risks / Trade-offs

- **Wenige Anträge erwartet** → Einfache Liste ohne Pagination reicht. Falls die Zahl wächst, kann serverseitige Pagination nachgerüstet werden.
- **Confirm erstellt automatisch ein Mitglied** → Der Bestätigungsdialog muss klar kommunizieren, dass dadurch ein neues Mitglied angelegt wird.
