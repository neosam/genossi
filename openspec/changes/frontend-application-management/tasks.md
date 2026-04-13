## 1. Frontend-API

- [x] 1.1 API-Funktionen in `genossi-frontend/src/api.rs`: `get_applications(status_filter)`, `get_application(id)`, `confirm_application(id)`, `reject_application(id)` — nutzt `ApplicationTO` und `ApplicationStatusTO` aus `rest-types`

## 2. i18n-Keys

- [x] 2.1 Neue i18n-Keys in `mod.rs` definieren (Applications, ApplicationsDesc, ApplicationStatus, StatusOffen, StatusBestaetigt, StatusAbgelehnt, StatusAll, NoApplications, ConfirmApplication, RejectApplication, ConfirmApplicationHint, RejectApplicationHint, ApplicationDetails, Shares, SubmittedAt, ApplicationConfirmed, ApplicationRejected)
- [x] 2.2 Deutsche Übersetzungen in `de.rs`
- [x] 2.3 Englische Übersetzungen in `en.rs`

## 3. Komponenten

- [x] 3.1 Komponente `ApplicationList` in `genossi-frontend/src/component/` erstellen: Tabelle mit Name, E-Mail, Anteile, Status, Datum; klickbare Zeilen
- [x] 3.2 Komponente `ApplicationDetail` in `genossi-frontend/src/component/` erstellen: Modal mit allen Antragsdaten + Bestätigen/Ablehnen-Buttons (nur bei Status Offen) mit Bestätigungsdialog
- [x] 3.3 Komponenten in `component/mod.rs` exportieren

## 4. Seite und Routing

- [x] 4.1 Neue Seite `ApplicationsPage` in `genossi-frontend/src/page/applications_page.rs`: Status-Filter-Tabs, lädt Anträge, zeigt `ApplicationList`, öffnet `ApplicationDetail` bei Klick
- [x] 4.2 Seite in `page/mod.rs` exportieren
- [x] 4.3 Route `/applications` in `router.rs` hinzufügen
- [x] 4.4 Navigation-Link in `top_bar.rs` für Admin-Benutzer hinzufügen

## 5. Tests

- [x] 5.1 E2E-Test: Anträge auflisten (Antrag einreichen, Liste abrufen, prüfen dass er vorhanden ist)
- [x] 5.2 E2E-Test: Antrag bestätigen und ablehnen (Status-Wechsel verifizieren)
