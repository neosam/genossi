## ADDED Requirements

### Requirement: Validation Service
Das System MUSS einen `ValidationService` bereitstellen, der die Datenintegrität aller Mitglieder und Aktionen prüft und ein strukturiertes Ergebnis zurückliefert.

#### Scenario: Erfolgreiche Validierung ohne Fehler
- **WHEN** alle Mitgliedsnummern lückenlos sind und alle Übertragungen einen Gegenpart haben
- **THEN** gibt der Service ein Ergebnis mit leeren Fehlerlisten zurück

#### Scenario: Validierung erfordert Berechtigung
- **WHEN** ein Benutzer ohne `view_members`-Berechtigung die Validierung aufruft
- **THEN** wird ein PermissionDenied-Fehler zurückgegeben

### Requirement: Mitgliedsnummern-Lücken erkennen
Das System MUSS alle fehlenden Mitgliedsnummern im Bereich zwischen der kleinsten und größten vorhandenen Nummer erkennen. Soft-deleted Mitglieder MÜSSEN bei der Bereichsbestimmung berücksichtigt werden.

#### Scenario: Keine Lücken vorhanden
- **WHEN** die Mitglieder die Nummern 1, 2, 3 haben
- **THEN** ist die Liste der fehlenden Nummern leer

#### Scenario: Lücken vorhanden
- **WHEN** die Mitglieder die Nummern 1, 2, 5, 8 haben
- **THEN** werden die fehlenden Nummern 3, 4, 6, 7 gemeldet

#### Scenario: Bereich beginnt nicht bei 1
- **WHEN** die Mitglieder die Nummern 100, 101, 103 haben
- **THEN** wird nur die fehlende Nummer 102 gemeldet (nicht 1-99)

#### Scenario: Soft-deleted Mitglieder zählen mit
- **WHEN** Mitglied 3 soft-deleted ist und Mitglieder 1, 2, 3(deleted), 4 existieren
- **THEN** werden keine Lücken gemeldet

#### Scenario: Keine Mitglieder vorhanden
- **WHEN** keine Mitglieder existieren
- **THEN** ist die Liste der fehlenden Nummern leer

### Requirement: Übertragungen ohne Gegenpart erkennen
Das System MUSS Übertragungsaktionen identifizieren, die keinen korrespondierenden Gegenpart haben. Zwei Aktionen gelten als Paar, wenn alle folgenden Bedingungen erfüllt sind:
- Die Typen sind komplementär (UebertragungAbgabe ↔ UebertragungEmpfang)
- Die `member_id` der einen Aktion entspricht der `transfer_member_id` der anderen (und umgekehrt)
- Die `shares_change`-Werte sind spiegelsymmetrisch (z.B. -3 und +3)
- Das `date`-Feld ist identisch

#### Scenario: Alle Übertragungen haben Gegenpart
- **WHEN** Mitglied A eine UebertragungAbgabe (shares: -3, transfer: B, date: 2024-05-01) hat
- **AND** Mitglied B eine UebertragungEmpfang (shares: +3, transfer: A, date: 2024-05-01) hat
- **THEN** werden keine unmatched Übertragungen gemeldet

#### Scenario: Übertragung ohne Gegenpart
- **WHEN** Mitglied A eine UebertragungAbgabe (shares: -3, transfer: B, date: 2024-05-01) hat
- **AND** Mitglied B keine korrespondierende UebertragungEmpfang hat
- **THEN** wird die Aktion von Mitglied A als unmatched gemeldet

#### Scenario: Shares stimmen nicht überein
- **WHEN** Mitglied A eine UebertragungAbgabe (shares: -3, transfer: B, date: 2024-05-01) hat
- **AND** Mitglied B eine UebertragungEmpfang (shares: +2, transfer: A, date: 2024-05-01) hat
- **THEN** werden beide Aktionen als unmatched gemeldet

#### Scenario: Datum stimmt nicht überein
- **WHEN** Mitglied A eine UebertragungAbgabe (shares: -3, transfer: B, date: 2024-05-01) hat
- **AND** Mitglied B eine UebertragungEmpfang (shares: +3, transfer: A, date: 2024-06-01) hat
- **THEN** werden beide Aktionen als unmatched gemeldet

#### Scenario: Soft-deleted Aktionen werden ignoriert
- **WHEN** eine Übertragungsaktion soft-deleted ist
- **THEN** wird sie bei der Übertragungs-Validierung nicht berücksichtigt

#### Scenario: Keine Übertragungen vorhanden
- **WHEN** keine Übertragungsaktionen existieren
- **THEN** ist die Liste der unmatched Übertragungen leer

### Requirement: REST-Endpoint für Validierung
Das System MUSS einen `GET /api/validation` Endpoint bereitstellen, der die Validierungsergebnisse als JSON zurückgibt.

#### Scenario: Erfolgreicher Abruf
- **WHEN** ein authentifizierter Benutzer `GET /api/validation` aufruft
- **THEN** erhält er HTTP 200 mit einem JSON-Objekt das `member_number_gaps` und `unmatched_transfers` enthält

#### Scenario: Nicht authentifiziert
- **WHEN** ein nicht authentifizierter Benutzer `GET /api/validation` aufruft
- **THEN** erhält er HTTP 401

### Requirement: Frontend-Validierungsseite
Das Frontend MUSS eine Seite bereitstellen, die den Validierungs-Endpoint aufruft und die Ergebnisse übersichtlich anzeigt.

#### Scenario: Seite laden
- **WHEN** der Benutzer die Validierungsseite öffnet
- **THEN** wird automatisch der Validierungs-Endpoint aufgerufen und die Ergebnisse angezeigt

#### Scenario: Keine Probleme gefunden
- **WHEN** die Validierung keine Probleme findet
- **THEN** wird eine Erfolgsmeldung angezeigt

#### Scenario: Probleme gefunden
- **WHEN** die Validierung Lücken oder unmatched Übertragungen findet
- **THEN** werden die Probleme in separaten Sektionen mit Details angezeigt
