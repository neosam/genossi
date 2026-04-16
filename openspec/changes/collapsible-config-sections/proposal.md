## Why

Die Konfigurationsseite (`config_page.rs`, 1245 Zeilen) zeigt aktuell sechs bis sieben große Sektionen direkt untereinander: SMTP, Mail-Footer, IMAP-Posteingang, WebDAV-Backup, TSA, WordPress-Integration und generische Config-Entries. Beim Aufruf scrollt der Nutzer durch viel Inhalt, der für den momentanen Zweck irrelevant ist. Eine zusammenklappbare Akkordion-Darstellung würde die Übersicht massiv verbessern und folgt einem Pattern, das auch auf weiteren langen Seiten (Mitglieds-Detail, Mail-Page) Anwendung finden kann.

## What Changes

- Neue wiederverwendbare Komponente `CollapsibleSection` unter `genossi-frontend/src/component/`, die einen klickbaren Header mit Pfeil-Icon und einen ein-/ausklappbaren Inhaltsbereich kapselt.
- Alle bestehenden Sektionen der Konfigurationsseite werden in `CollapsibleSection` gewickelt — inkl. der bereits ausgelagerten `TsaConfigSection` und `WordPressIntegrationSection`.
- Standardverhalten beim Aufruf der Seite: alle Sektionen sind eingeklappt.
- Mehrere Sektionen können gleichzeitig geöffnet sein (kein striktes Akkordion).
- Klick auf den ganzen Header öffnet/schließt; ein Pfeil-Icon signalisiert den Zustand.

## Capabilities

### New Capabilities
- `collapsible-sections`: Wiederverwendbare ein- und ausklappbare Sektionskomponente plus deren Anwendung auf der Konfigurationsseite.

### Modified Capabilities
<!-- keine - die Inhalte der Sektionen (SMTP, IMAP, WebDAV, TSA, WordPress, generische Config-Entries) bleiben in ihren Anforderungen unverändert; nur ihre Darstellung wird einklappbar -->

## Impact

- **Frontend**:
  - Neue Datei `genossi-frontend/src/component/collapsible_section.rs`.
  - Re-Export in `genossi-frontend/src/component/mod.rs`.
  - `genossi-frontend/src/page/config_page.rs` — alle Sektion-`div`s in `CollapsibleSection` umbauen.
  - i18n-Keys nur, falls neue Texte (z. B. „Alle aufklappen") eingeführt werden — vorerst nicht geplant.
- **Backend**: Keine Änderung.
- **Berechtigungen**: Keine Änderung.
