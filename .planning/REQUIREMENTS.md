# Requirements: Genossi — GV-Anwesenheits-Erfassung

**Defined:** 2026-05-02
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar, mit weniger manueller Arbeit. Dieser Milestone bringt papierlose Anwesenheits-Erfassung auf der Generalversammlung.

## v1 Requirements

Anforderungen für den GV-Anwesenheits-Milestone. Jede Anforderung wird im Roadmap einer Phase zugeordnet.

### Assembly (GV-Lifecycle)

- [ ] **ASSY-01**: Vorstand kann eine Generalversammlung anlegen mit Datum und Titel; Initial-Status `Vorbereitung`
- [ ] **ASSY-02**: Vorstand kann eine vorbereitete GV öffnen (Status `Offen`); beim Öffnen wird ein Member-Universe-Snapshot persistiert (alle aktiven Mitglieder zum Zeitpunkt des Öffnens — definiert das stabile „Y" im Anwesenheits-Counter)
- [ ] **ASSY-03**: Vorstand kann eine offene GV schließen (Status `Geschlossen`); alle Helfer-Sessions zu dieser GV werden ab diesem Moment ungültig
- [x] **ASSY-04**: Vorstand sieht während einer offenen GV einen Live-Counter „X von Y anwesend" (X = aktuelle Anwesende, Y = Member-Universe-Snapshot aus ASSY-02); Aktualisierung bei Refresh/Polling, kein Push
- [ ] **ASSY-05**: GV-Daten (Anwesenheits-Liste + Anzahl Anwesender + Member-Universe-Snapshot) bleiben nach GV-Schluss persistent für Protokoll-Export und retrospektive Statistik
- [ ] **ASSY-06**: Vorstand kann auch nach GV-Schluss noch Anwesenheits-Einträge hinzufügen oder entfernen (z. B. nachgemeldete Anwesenheiten); GV-Status bleibt dabei `Geschlossen` (kein Re-Open)
- [ ] **ASSY-07**: Assembly-Lifecycle-Vorgänge (`create`, `open`, `close`, post-close-Korrektur) werden via bestehender Audit-Hashchain protokolliert

### Helfer-Token & Session

- [ ] **HLPR-01**: Vorstand kann pro Helfer einen Helfer-Token erzeugen mit einem Freitext-Namen als Memo (z. B. „Anna", „Bernd"); das Backend liefert den Token sowohl als QR-Code-Bild (SVG) als auch als 8–12-Zeichen-alphanumerischen Klartext-Code
- [ ] **HLPR-02**: Helfer kann sich per QR-Scan in der Helfer-Ansicht einloggen; Backend redeemed den Token atomar (`UPDATE ... WHERE used_at IS NULL RETURNING ...`) und bindet eine Helfer-Session an die GV
- [ ] **HLPR-03**: Helfer kann alternativ den 8–12-Zeichen-Code manuell in ein Eingabefeld tippen und damit dieselbe Session erzeugen — als Fallback bei Camera-Permission-Verweigerung oder Scanner-Fehlfunktion
- [ ] **HLPR-04**: Token ist One-Time-Use — ein zweiter Redeem-Versuch (egal ob QR oder Code) schlägt mit klarem Fehler fehl
- [ ] **HLPR-05**: Helfer-Session ist gültig bis zum Schließen der zugehörigen GV (Cookie-Expiry an `assembly.closed_at` gebunden); danach ist sie ungültig, auch wenn Cookie noch im Browser liegt
- [ ] **HLPR-06**: Vorstand sieht in der GV-Detail-Ansicht eine Liste der erzeugten Token mit Memo-Namen und Status (offen/eingelöst); kann offene Token vor GV-Beginn revoken
- [ ] **HLPR-07**: Token-Erzeugung wird via bestehender Audit-Hashchain protokolliert (Memo-Name, Erzeuger, Timestamp, GV-Bezug)

### Anwesenheits-Erfassung

- [x] **ATTN-01**: Helfer-Ansicht zeigt eine Mitgliederliste mit ausschließlich diesen Spalten: Mitgliedsnummer, Name, Titel, Anrede — keine weiteren Felder werden vom Backend ausgeliefert
- [x] **ATTN-02**: Helfer kann in der Liste suchen (Substring-Match auf Name oder Mitgliedsnummer)
- [x] **ATTN-03**: Helfer kann ein Mitglied als anwesend markieren; API ist idempotent (PUT, Doppel-Klick erzeugt keinen Fehler, kein duplizierter Eintrag)
- [x] **ATTN-04**: Helfer kann ein Mitglied wieder austragen (anwesend → nicht-anwesend); ebenfalls idempotent
- [ ] **ATTN-05**: Anwesenheits-Markierungen werden bewusst **nicht** in der Audit-Hashchain protokolliert (vom User explizit ausgeschlossen)
- [x] **ATTN-06**: Helfer-View ist auch für eingeloggte Vorstands-User direkt aufrufbar — ohne QR-Token, mit derselben UI; Permission-Check auf Service-Layer akzeptiert beide Auth-Pfade

### Sync zwischen mehreren Helfern

- [ ] **SYNC-01**: Helfer sehen aktualisierte Anwesenheits-Status beim nächsten Refresh oder beim nächsten Such-Vorgang; kein Live-Push (SSE/WebSocket) erforderlich
- [x] **SYNC-02**: Doppel-Markierung durch zwei Helfer gleichzeitig erzeugt keinen Fehlertext und keinen doppelten Anwesenheits-Eintrag (Konsequenz aus ATTN-03 Idempotenz)

## v2 Requirements

Bewusst nach v1 verschoben — Roadmap dieses Milestones umfasst sie nicht.

### Protokoll-Export

- **EXPO-01**: PDF-Export der Anwesenheits-Liste über bestehende Typst-Pipeline (mit Genossenschafts-Layout, Datum, Anzahl-Anwesende für Niederschrifts-Anlage)
- **EXPO-02**: CSV/Excel-Export der Anwesenheits-Liste

### Bulk-Operationen

- **BULK-01**: Bulk-QR-Erzeugung für N Helfer in einem Schritt (Druck-Vorlage)
- **BULK-02**: Druck-Vorlage mit allen QR-Codes auf einer A4-Seite

### Stimmrechte / Vollmachten / Quorum

- **VOTE-01**: Vollmacht-Verwaltung (Mitglied A vertritt Mitglied B)
- **VOTE-02**: Stimmgewichts-Modellierung (1 Mitglied = 1 Stimme oder genossenschaftsspezifische Regeln)
- **VOTE-03**: Quorum-Berechnung gemäß Satzung
- **VOTE-04**: Live-Beschluss-Erfassung mit Pro/Contra-Auswertung

## Out of Scope

Explizit ausgeschlossen — nicht in v1, nicht in v2 dieses Milestones (separate spätere Diskussion erforderlich).

| Feature | Reason |
|---------|--------|
| Audit-Hashchain pro Anwesenheits-Markierung | Vom User explizit ausgeschlossen — Verband fordert nur Anzahl-Anwesende im Protokoll, nicht den Vorgang des Abhakens |
| Re-Open einer geschlossenen GV | Lifecycle final; Korrekturen via ASSY-06 ohne Status-Wechsel — vermeidet Helfer-Session-Ping-Pong und Counter-Recompute |
| Live-Push zwischen Helfern (SSE/WebSocket) | Refresh-only Sync explizit gewählt; Doppel-Markierung über Idempotenz abgefangen, kein Sync-Aufwand |
| Offline-Modus / Sync-Engine | Helfer brauchen Netzwerk; Conflict-Resolution würde den Scope sprengen |
| Stimmgewichts-/Anteils-Daten in Helfer-Ansicht | Helfer-View bleibt bewusst minimal (Datenschutz, Genossi-Konformität) |
| Self-Check-in für Mitglieder per persönlichem QR-Code | Verbandsrechtlich heikel — Helfer-Sichtkontakt zum Mitglied bleibt erforderlich |
| Identitäts-Verifikation per QR-Code | Token ist Bearer, nicht Identitätsnachweis — Helfer prüfen Mitglied physisch |
| Native Mobile-App | Web-First; Helfer nutzen Browser auf Tablet/Laptop/Handy |

## Traceability

Phase-Mapping aus Roadmap (2026-05-02). Coverage: 22/22 v1 Requirements gemappt, 0 Orphans.

| Requirement | Phase | Status |
|-------------|-------|--------|
| ASSY-01 | Phase 1 | Pending |
| ASSY-02 | Phase 1 | Pending |
| ASSY-03 | Phase 1 | Pending |
| ASSY-04 | Phase 3 | Complete |
| ASSY-05 | Phase 1 | Pending |
| ASSY-06 | Phase 3 | Pending |
| ASSY-07 | Phase 1 | Pending |
| HLPR-01 | Phase 2 | Pending |
| HLPR-02 | Phase 2 | Pending |
| HLPR-03 | Phase 4 | Pending |
| HLPR-04 | Phase 2 | Pending |
| HLPR-05 | Phase 2 | Pending |
| HLPR-06 | Phase 2 | Pending |
| HLPR-07 | Phase 2 | Pending |
| ATTN-01 | Phase 3 | Complete |
| ATTN-02 | Phase 3 | Complete |
| ATTN-03 | Phase 3 | Complete |
| ATTN-04 | Phase 3 | Complete |
| ATTN-05 | Phase 3 | Pending |
| ATTN-06 | Phase 3 | Complete |
| SYNC-01 | Phase 4 | Pending |
| SYNC-02 | Phase 3 | Complete |

**Coverage:**
- v1 requirements: 22 total
- Mapped to phases: 22 (Phase 1: 6, Phase 2: 6, Phase 3: 8, Phase 4: 2, Phase 5: operativ)
- Unmapped: 0

**Phase 5 (Pre-GV-Generalprobe)** mappt keine direkten Requirements, sondern verifiziert die in Phasen 1–4 gelieferten Requirements unter realen Live-Event-Bedingungen.

---
*Requirements defined: 2026-05-02*
*Last updated: 2026-05-02 after roadmap creation (traceability filled)*
