# Requirements: Genossi — Milestone v1.3

**Defined:** 2026-06-26
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), und mit weniger manueller Arbeit bei wiederkehrenden Vorgängen.

**Milestone-Goal:** Vorstände verpassen keine eingehenden Mails mehr und können bequemer auf sie antworten.

## v1 Requirements

Requirements für Milestone v1.3. Jede Anforderung mappt auf genau eine Roadmap-Phase.

### Inbox-Digest

- [ ] **DIGEST-01**: Vorstand kann eine oder mehrere Empfänger-E-Mail-Adressen für die tägliche Posteingangs-Benachrichtigung über die Config-Seite pflegen (speichern und ändern)
- [ ] **DIGEST-02**: Vorstand kann die tägliche Versand-Uhrzeit für die Benachrichtigung über die Config-Seite konfigurieren
- [ ] **DIGEST-03**: Das System verschickt einmal pro Kalendertag zur konfigurierten Uhrzeit eine Digest-Mail an alle konfigurierten Empfänger
- [ ] **DIGEST-04**: Das System verschickt keine Digest-Mail, wenn der Posteingang leer ist (keine nicht-archivierten Mails vorhanden)
- [ ] **DIGEST-05**: Die Digest-Mail listet alle offenen (nicht-archivierten) Mails mit Titel, Absender und Eingangszeitpunkt auf
- [ ] **DIGEST-06**: Die Digest-Mail enthält einen Deep-Link, der direkt die Inbox-Seite (`/inbox`) öffnet
- [ ] **DIGEST-07**: Sind keine Empfänger konfiguriert, unterbleibt der Versand ohne Fehler (Feature ist faktisch deaktiviert)

### Reply-Komfort

- [ ] **REPLY-01**: Vorstand öffnet das Antwort-Formular für eine eingegangene Mail in einem vollflächigen Modal statt im Inline-Feld
- [ ] **REPLY-02**: Das Antwort-Modal bietet deutlich mehr Schreibfläche (größeres Textfeld) als das bisherige schmale Inline-Feld
- [ ] **REPLY-03**: Vorstand kann das Antwort-Modal abbrechen/schließen, ohne zu senden, und kehrt zur Mail-Ansicht zurück
- [ ] **REPLY-04**: Das Absenden der Antwort aus dem Modal nutzt die bestehende Sende-Logik und zeigt Erfolg/Fehler-Feedback wie bisher

## v2 Requirements

Anerkannt, aber für v1.3 zurückgestellt.

### Inbox-Digest

- **DIGEST-F1**: Digest-Inhalt nur über neu eingegangene Mails seit letztem Versand (statt aller offenen) — bewusst verworfen zugunsten der Workqueue-Erinnerung
- **DIGEST-F2**: Konfigurierbares Versand-Intervall feiner als täglich (z.B. mehrmals pro Tag)

## Out of Scope

Explizit ausgeschlossen, dokumentiert gegen Scope-Creep.

| Feature | Reason |
|---------|--------|
| Klick-Aktionen direkt aus der Digest-Mail (Archivieren/Antworten per Mail-Link) | Erfordert authentifizierte Deep-Action-Links; Digest ist nur Hinweis + Link auf die App |
| Pro-Empfänger individuell konfigurierte Digest-Inhalte/Filter | Zu komplex für jetzt; ein gemeinsamer Digest an alle Empfänger reicht |
| Rich-Text-/HTML-Editor im Reply-Modal | Reply-Logik bleibt unverändert; nur der UI-Container ändert sich |

## Traceability

Welche Phasen welche Requirements abdecken. Während der Roadmap-Erstellung gefüllt.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DIGEST-01 | Phase 20 | Pending |
| DIGEST-02 | Phase 20 | Pending |
| DIGEST-03 | Phase 20 | Pending |
| DIGEST-04 | Phase 20 | Pending |
| DIGEST-05 | Phase 20 | Pending |
| DIGEST-06 | Phase 20 | Pending |
| DIGEST-07 | Phase 20 | Pending |
| REPLY-01 | Phase 21 | Pending |
| REPLY-02 | Phase 21 | Pending |
| REPLY-03 | Phase 21 | Pending |
| REPLY-04 | Phase 21 | Pending |

**Coverage:**
- v1 requirements: 11 total
- Mapped to phases: 11 ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-26*
*Last updated: 2026-06-26 after milestone v1.3 start*
