# Requirements: Genossi — v1.6 Antragsteller-Kommunikation

**Defined:** 2026-08-12
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit.

**Milestone-Goal:** Vorstände können Personen mit abgegebener Beitrittserklärung (Applications) direkt per E-Mail kontaktieren — insbesondere Zahlungserinnerungen —, mit wiederverwendbaren Vorlagen und nachvollziehbarer Kommunikations-Historie, auch bevor die Person Mitglied ist.

**Research:** `.planning/research/SUMMARY.md` (HIGH confidence; "add nothing" Stack, 4-Phasen-Build-Order, DSGVO transaktional). Kernpunkt: `application_id`-Linkage statt `member_id`-Overload; keine neuen Dependencies.

## v1.6 Requirements

### Versand (APMAIL)

- [ ] **APMAIL-01**: Vorstand kann einer Beitrittserklärung (Application) mit Status `Offen` eine einzelne E-Mail senden — Empfänger = `application.email`, `RecipientInput` mit `member_id: None` + gesetztem `application_id`; via `POST /api/applications/{id}/mail`, nur für Vorstand (`admin`-Rolle).
- [ ] **APMAIL-02**: Der Versand gibt echten Erfolg/Fehler zurück (`Result<_, ServiceError>`) — keine stille `200-OK-ohne-Versand`-Falle wie beim bestehenden `send_confirmation_mail`; Fehler (kein SMTP, fehlende Config) werden dem Vorstand sichtbar gemeldet.
- [ ] **APMAIL-03**: Fehlende E-Mail-Adresse wird sauber behandelt — der „E-Mail senden"-Button ist deaktiviert/annotiert, nie ein stiller Fehlversuch.
- [ ] **APMAIL-04**: Vorstand sieht vor dem Absenden eine Vorschau mit aufgelösten Platzhaltern und bestätigt den Versand bewusst (confirm-before-send).

### Vorlagen (APTPL)

- [ ] **APTPL-01**: Vorlagen können gegen einen Application-Kontext gerendert werden — Platzhalter: Anrede, Vorname, Nachname, Titel, Anzahl Anteile, offener Betrag; über eine eigene `application_to_template_context`-Funktion (kein Member-Kontext mit gelöschten Feldern). Application-Vorlagen sind ein **eigener „Antragsteller"-Vorlagentyp**, getrennt vom Member-Vorlagen-Pool (Entscheid D1).
- [ ] **APTPL-02**: „Offener Betrag" wird zur Laufzeit berechnet (`Anteile × Anteilswert`), niemals auf der Application gespeichert; `share_value_cents` stammt aus **derselben Config-Quelle wie die bestehende Bestätigungsmail** (`send_confirmation_mail`, Entscheid D3); korrekte deutsche Euro-Formatierung (Tausender, Komma, negativer/Null-Fall korrekt).
- [ ] **APTPL-03**: Eine deutsche Standard-Vorlage „Zahlungserinnerung" wird mitgeliefert (Seed-Content), sodass der Haupt-Use-Case in wenigen Klicks erledigt ist.
- [ ] **APTPL-04**: Template-Validierung für Application-Vorlagen schlägt bei unbekannten/Member-only-Platzhaltern kontrolliert fehl (kein `strict`-Render-Crash beim Versand); die ~40 bestehenden Member-Template-Tests bleiben grün.

### Kommunikations-Historie (APHIST)

- [x] **APHIST-01**: Alle an eine Application gesendeten Mails werden in einer Kommunikations-Historie pro Antragsteller erfasst — über `application_id`-Linkage an `mail_recipients`/Communication-Entry, kein `member_id`-Overload; Endpoint `GET /api/applications/{id}/communications`.
- [ ] **APHIST-02**: Der Vorstand sieht auf der Application-Detailseite prominent „zuletzt gesendet am …" — der zentrale Anti-Doppelversand-/Spam-Guard.
- [x] **APHIST-03**: Nach Bestätigung (`confirm` → Mitglied) erscheint die als Antragsteller gesendete Erinnerungs-Kommunikation **in der Mitglieds-Timeline** des neuen Mitglieds (Carry-over, Entscheid D2). *(Mechanismus — Back-fill `member_id` beim Bestätigen / Union-at-read / Link-Spalte — wird in Phase 1 Planung entschieden; verifiziert per e2e: Erinnerung → confirm → sichtbar in Member-Timeline.)*

### Compliance / Guardrails (APCMP)

- [ ] **APCMP-01**: Versand ist nur bei Status `Offen` möglich (sonst HTTP 409, analog `confirm`/`reject`) — deckt zugleich die DSGVO-Rechtsgrundlage (transaktional, bezogen auf die eigene Beitrittserklärung) und verhindert Mails an abgelehnte oder bereits bestätigte (jetzt Mitglied) Antragsteller.
- [ ] **APCMP-02**: Der Inhalt ist auf die eigene Beitrittserklärung/Zahlung bezogen — kein Massenversand, keine Newsletter/Marketing-Inhalte, kein Open-/Click-Tracking.

### Frontend Dialog (APUI)

- [ ] **APUI-01**: „E-Mail senden"-Button auf der Application-Detailseite öffnet einen Compose-Dialog (Vorbild-Pattern: `member_details.rs`); bei fehlender Adresse deaktiviert.
- [ ] **APUI-02**: Der Dialog nutzt die bestehenden `component/mail_compose/`-Bausteine (Betreff-Input, WYSIWYG-Editor, Template-Selector, Preview) — kein geforktes UI (Component-First).
- [ ] **APUI-03**: Die Kommunikations-Historie wird über die bestehende `communication_timeline.rs`-Komponente unverändert (prop-driven) auf der Application-Detailseite/im Dialog angezeigt.

## Future Requirements

Deferred zu späteren Milestones — sinnvoll, aber nicht kritisch für v1.6.

- **APHIST-FUT-01**: Vorlage/Betreff und Body-Snapshot je Timeline-Eintrag speichern (Deep-Link auf exakt gesendeten Inhalt)
- **APMAIL-FUT-01**: Massen-Erinnerung an alle Antragsteller mit Status `Offen` (Bulk-Send) — bewusst nach v1.6 verschoben
- **APTPL-FUT-01**: Mehrstufige Erinnerungs-Vorlagen (1./2. Erinnerung mit Eskalationstext)

## Out of Scope

Explizit ausgeschlossen — dokumentiert, um Scope-Creep zu verhindern.

| Feature | Grund |
|---|---|
| Massen-/Bulk-Versand an alle „Offen"-Antragsteller | Bewusst einzeln pro Antragsteller in v1.6; Bulk ist Future-Requirement |
| Automatischer Dunning-Zeitplan / Auto-Eskalation | Vorstand entscheidet manuell wann erinnert wird; kein Scheduler in v1.6 |
| Open-/Click-Tracking-Pixel | DSGVO-Risiko, kein Mehrwert für den Use-Case |
| Newsletter-/Marketing-Inhalte an Antragsteller | §7 UWG-Risiko; Rechtsgrundlage deckt nur transaktionale Erinnerung zur eigenen Erklärung |
| Formale Mahnungs-Mechanik (Verzug, Zinsen, Mahngebühren) | Rechtlich eigenständig, nicht Kern-Value; einfache Zahlungserinnerung genügt |
| Gespeichertes Zahlungsstatus-Feld auf der Application | „Offener Betrag" wird berechnet; kein neuer Zustand am auditierten `ApplicationEntity` (Audit-Ripple/Locked-Test-Bruch vermeiden) |
| Datei-Anhänge / generierte PDFs an Antragsteller | Nicht im Erinnerungs-Use-Case; hält den Pfad schlank |
| Freitext-Versand an beliebige Adressen | Empfänger ist immer die Application selbst (Rechtsgrundlage + Privacy) |
| Reply-/Posteingang-Threading in die Antragsteller-Timeline | Antragsteller haben keine `assigned_member_id`-Inbound-Zuordnung; Timeline ist outbound-only |

## Getroffene Produkt-Entscheidungen (2026-08-12, mit User)

- **D1 — Template-Pool:** Eigener „Antragsteller"-Vorlagentyp, getrennt vom Member-Pool. Vermeidet die strict-render-Bombe (Member-only-Platzhalter). → APTPL-01
- **D2 — Historie-Carry-over bei `confirm()`:** Als Antragsteller gesendete Erinnerungen erscheinen nach der Bestätigung in der **Mitglieds-Timeline**. Konkreter Mechanismus (Back-fill / Union-at-read / Link-Spalte) wird in Phase-1-Planung festgelegt. → APHIST-03
- **D3 — `share_value_cents`-Quelle:** Dieselbe Config-Quelle wie die bestehende Bestätigungsmail (`send_confirmation_mail`) — Konsistenz. → APTPL-02

## Traceability

Phase-Mapping aus `.planning/ROADMAP.md` (v1.6 Phases 29-32, fortlaufende Nummerierung nach v1.5). Coverage: 16/16 Requirements gemappt, keine Orphans, keine Duplikate.

| Requirement | Phase | Phase Name | Status |
|-------------|-------|------------|--------|
| APHIST-01 | Phase 29 | DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller) | Complete |
| APHIST-03 | Phase 29 | DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller) | Complete |
| APTPL-01 | Phase 30 | Application-Template-Kontext (Antragsteller-Vorlagen) | Pending |
| APTPL-02 | Phase 30 | Application-Template-Kontext (Antragsteller-Vorlagen) | Pending |
| APTPL-03 | Phase 30 | Application-Template-Kontext (Antragsteller-Vorlagen) | Pending |
| APTPL-04 | Phase 30 | Application-Template-Kontext (Antragsteller-Vorlagen) | Pending |
| APMAIL-01 | Phase 31 | Service + REST Versand (Versand + Guardrails) | Pending |
| APMAIL-02 | Phase 31 | Service + REST Versand (Versand + Guardrails) | Pending |
| APCMP-01 | Phase 31 | Service + REST Versand (Versand + Guardrails) | Pending |
| APCMP-02 | Phase 31 | Service + REST Versand (Versand + Guardrails) | Pending |
| APHIST-02 | Phase 31 | Service + REST Versand (Versand + Guardrails) | Pending |
| APMAIL-03 | Phase 32 | Frontend Compose-Dialog | Pending |
| APMAIL-04 | Phase 32 | Frontend Compose-Dialog | Pending |
| APUI-01 | Phase 32 | Frontend Compose-Dialog | Pending |
| APUI-02 | Phase 32 | Frontend Compose-Dialog | Pending |
| APUI-03 | Phase 32 | Frontend Compose-Dialog | Pending |
