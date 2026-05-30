# Requirements: Genossi — v1.1 Anteile-Rückzahlungsphase

**Defined:** 2026-05-29
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit.

**Milestone Goal:** Ersetzt die Excel-Liste für Anteils-Auszahlungen — Vorstand verwaltet Rückzahlungsphasen direkt in Genossi, schreibt Mitglieder per Massenmail an und exportiert auszahlbare Beträge als PDF zur Online-Banking-Übernahme.

## v1 Requirements

### RepaymentPhase Lifecycle

- [x] **PHAS-01**: Vorstand kann eine `RepaymentPhase` mit `fiscal_year` und `share_value` (Cent/Decimal) anlegen — Initial-Status `Vorbereitung`
- [ ] **PHAS-02**: Vorstand kann `RepaymentPhase` öffnen (`Vorbereitung → Offen`) — beim Übergang werden Einträge automatisch befüllt (siehe ENTR-01)
- [ ] **PHAS-03**: Vorstand kann `RepaymentPhase` abschließen (`Offen → Abgeschlossen`) nur wenn alle Einträge Status `ausbezahlt` oder soft-gelöscht haben
- [x] **PHAS-04**: Vorstand kann `share_value` in `Offen`-Status korrigieren — Änderung über `audited_update!` (eigener Audit-Eintrag pro Korrektur)
- [x] **PHAS-05**: `RepaymentPhase` ist auditpflichtig — `audited_create!` beim Anlegen, `audited_update!` für Lifecycle-Übergänge und `share_value`-Korrekturen

### RepaymentEntry Management

- [ ] **ENTR-01**: Beim Phase-Öffnen werden für alle Mitglieder mit `exit_date` im `fiscal_year` automatisch RepaymentEntries angelegt — Initial-Wert `share_count_to_pay_out = Member.current_shares` zum Stichpunkt
- [ ] **ENTR-02**: Vorstand kann manuell weitere Einträge zu einer offenen Phase hinzufügen (Mitglied-Picker + `share_count_to_pay_out`) — für Teil-Abtretungen und verspätet gemeldete Austritte
- [ ] **ENTR-03**: Mehrere Einträge pro Mitglied+Phase sind erlaubt (kein Composite-PK-Constraint auf `(member_id, phase_id)`)
- [ ] **ENTR-04**: Vorstand kann `share_count_to_pay_out` bearbeiten solange Eintragsstatus `offen` oder `angeschrieben`
- [ ] **ENTR-05**: Vorstand kann Eintrag soft-löschen solange Eintragsstatus nicht `ausbezahlt`
- [ ] **ENTR-06**: Status-Toggle `offen → angeschrieben` manuell durch Vorstand; multi-select-fähig (Massen-Toggle nach Mail-Versand)

### Auszahlungs-Buchung

- [ ] **PAYO-01**: Status-Toggle `ausbezahlt` erzeugt atomar in einer Transaktion einen `MemberAction::Verkauf` mit `shares_change = -share_count_to_pay_out` über `audited_create!`
- [ ] **PAYO-02**: Status-Toggle `ausbezahlt` reduziert `Member.current_shares` um `share_count_to_pay_out` atomar in derselben Transaktion
- [ ] **PAYO-03**: Validierung: `ausbezahlt`-Toggle wird blockiert (ServiceError::ValidationError) wenn `Member.current_shares < share_count_to_pay_out`
- [ ] **PAYO-04**: Status `ausbezahlt` ist final — kein Rücksetzen erlaubt (verhindert Audit-Verzerrung und inkonsistente `current_shares`)

### Massenmail

- [ ] **MAIL-01**: Vorstand wählt mehrere Einträge (multi-select) und löst Massenmail aus (gleiches Pattern wie Mitgliederliste-Massenmail)
- [ ] **MAIL-02**: Mail-Template kann `{{ payout_amount }}` referenzieren — berechnet als `share_count_to_pay_out × phase.share_value` zum Zeitpunkt des Mail-Versands
- [ ] **MAIL-03**: Mail-Template kann `{{ share_count }}` (`share_count_to_pay_out` des Eintrags) und `{{ fiscal_year }}` der Phase referenzieren
- [ ] **MAIL-04**: Mail-Versand erzeugt pro Empfänger ein `MemberDocument` mit Template-Referenz (bestehendes Auditpflicht-Pattern)

### Export

- [ ] **EXPO-01**: PDF-Export der Auszahlungsliste verfügbar für `Offen`- **und** `Abgeschlossen`-Phasen (vor Phasen-Abschluss verfügbar für Online-Banking-Vorlage)
- [ ] **EXPO-02**: PDF enthält pro Eintrag: Mitgliedsnummer, Name, IBAN, `share_count_to_pay_out`, Auszahlungs-Betrag, Verwendungszweck — sortiert nach Mitgliedsnummer aufsteigend
- [ ] **EXPO-03**: PDF/CSV-Export unterstützt Filter `?include=open|all|paid` (Default: `open` für Banking-Vorlage)
- [ ] **EXPO-04**: CSV-Export für Buchhaltung mit Semikolon-Separator und UTF-8-BOM (analog Teilnehmerlisten-Export)
- [ ] **EXPO-05**: Export-Endpoints sind Vorstand-only (OIDC), read-only, kein Audit-Hashchain-Eintrag

### Frontend (Component-First)

- [ ] **UI-01**: Page `/repayment-phases` mit Liste aller Phasen (Status, fiscal_year, share_value, Anzahl Einträge)
- [ ] **UI-02**: Page `/repayment-phases/{id}` mit Lifecycle-Aktionen (öffnen/schließen), Eintrags-Tabelle, Export-Tab
- [ ] **UI-03**: Shared Component `RepaymentEntryList` in `genossi-frontend/src/component/` — multi-select, Status-Filter, sortierbar (Mitgliedsnummer, Status)
- [ ] **UI-04**: Modal/Sub-Page zum manuellen Hinzufügen eines Eintrags (Mitglied-Picker mit Suche, share_count-Eingabe)
- [ ] **UI-05**: Eintrag-Status-Aktionen mit Confirm-Dialog für `ausbezahlt` (Warnung: irreversibel + auditiert + reduziert current_shares)
- [ ] **UI-06**: Massenmail-Aktion im Tabellen-Header (analog Mitgliederliste-Pattern), Template-Auswahl + Versenden-Button

## v2 Requirements (deferred)

### Brief-Anschreiben-Automatik

- **BRIEF-01**: Brief-Vorlagen aus Auszahlungs-Eintrag direkt als PDF erzeugen — out of v1.1, Vorstand erzeugt manuell

### SEPA-XML-Export

- **SEPA-01**: SEPA pain.001 XML-Format als Alternative zum PDF für direkten Banking-Sammelüberweisung-Upload

### Status-Rücksetzung

- **STAT-01**: Status `angeschrieben → offen` manuell zurücksetzen (für Mail-Korrekturen) — pragmatisch via Eintrag-Löschung + Neuerstellung umgehbar

## Out of Scope

| Feature | Reason |
|---------|--------|
| Steuerliche Berechnung (Kapitalertragsteuer etc.) | Buchhaltung verarbeitet separat; Genossi liefert nur Brutto-Betrag |
| Anteils-Übertragung Genosse → Genosse | Aktuell nicht angefragt; Rücknahme durch Genossenschaft ist der einzige Use-Case |
| Anteils-Klassen oder einzeln-erfasste Anteile mit Nummerierung | Explizit verworfen — homogene Anteile reichen |
| Member-`share_count`-Migration / Excel-Import der Anteile | Bereits in `Member.current_shares` vorhanden, keine Migration nötig |
| Brief-Anschreiben-Automatik | Vorstand erzeugt manuell außerhalb von Genossi |
| SEPA pain.001 XML | PDF reicht für Online-Banking-Vorlage |
| Audit-Hashchain für Export-Endpoints | Read-only, kein Schreibvorgang — Audit-Belastung ohne Mehrwert |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PHAS-01 | Phase 7 | Complete |
| PHAS-02 | Phase 7 (Skeleton: state-machine + audit-trail) + Phase 8 (full: auto-fill on open) | Skeleton |
| PHAS-03 | Phase 7 (Skeleton: state-machine + audit-trail) + Phase 8 (full: pending-entry validation) | Skeleton |
| PHAS-04 | Phase 7 | Complete |
| PHAS-05 | Phase 7 | Complete |
| ENTR-01 | Phase 8 | Pending |
| ENTR-02 | Phase 8 | Pending |
| ENTR-03 | Phase 8 | Pending |
| ENTR-04 | Phase 8 | Pending |
| ENTR-05 | Phase 8 | Pending |
| ENTR-06 | Phase 8 | Pending |
| PAYO-01 | Phase 9 | Pending |
| PAYO-02 | Phase 9 | Pending |
| PAYO-03 | Phase 9 | Pending |
| PAYO-04 | Phase 9 | Pending |
| MAIL-01 | Phase 10 | Pending |
| MAIL-02 | Phase 10 | Pending |
| MAIL-03 | Phase 10 | Pending |
| MAIL-04 | Phase 10 | Pending |
| EXPO-01 | Phase 11 | Pending |
| EXPO-02 | Phase 11 | Pending |
| EXPO-03 | Phase 11 | Pending |
| EXPO-04 | Phase 11 | Pending |
| EXPO-05 | Phase 11 | Pending |
| UI-01 | Phase 12 | Pending |
| UI-02 | Phase 12 | Pending |
| UI-03 | Phase 12 | Pending |
| UI-04 | Phase 12 | Pending |
| UI-05 | Phase 12 | Pending |
| UI-06 | Phase 12 | Pending |

**Coverage:**
- v1 requirements: 30 total
- Mapped to phases: 30
- Unmapped: 0 ✓

**Phase distribution:**
- Phase 7 (RepaymentPhase Backend): 3 (PHAS-01, PHAS-04, PHAS-05)
- Phase 8 (RepaymentEntry + Auto-Befüllung): 8 (ENTR-01..06 + PHAS-02 + PHAS-03)
- Phase 9 (Auszahlungs-Buchung): 4 (PAYO-01..04)
- Phase 10 (Massenmail): 4 (MAIL-01..04)
- Phase 11 (Export): 5 (EXPO-01..05)
- Phase 12 (Frontend): 6 (UI-01..06)

---
*Requirements defined: 2026-05-29*
*Last updated: 2026-05-29 after milestone v1.1 initial definition*
