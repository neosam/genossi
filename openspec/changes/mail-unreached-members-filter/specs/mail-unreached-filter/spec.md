## ADDED Requirements

### Requirement: API liefert nicht erreichte Mitglieder zu einem Mail-Job
Das System SHALL einen Endpoint `GET /api/members/not-reached-by/{job_id}` bereitstellen, der alle aktiven (nicht gelöschten) Mitglieder zurückgibt, die durch den angegebenen Mail-Job nicht erfolgreich erreicht wurden. Ein Mitglied gilt als "nicht erreicht", wenn es entweder keinen Eintrag in `mail_recipients` für diesen Job hat oder sein Eintrag einen Status ungleich "sent" aufweist.

#### Scenario: Mitglied ohne E-Mail-Adresse (nicht im Job enthalten)
- **WHEN** ein Mail-Job existiert und ein aktives Mitglied keinen Eintrag in `mail_recipients` für diesen Job hat
- **THEN** SHALL das Mitglied in der Ergebnisliste enthalten sein

#### Scenario: Mitglied mit fehlgeschlagenem Versand
- **WHEN** ein Mail-Job existiert und ein aktives Mitglied einen `mail_recipients`-Eintrag mit status "failed" hat
- **THEN** SHALL das Mitglied in der Ergebnisliste enthalten sein

#### Scenario: Mitglied mit erfolgreichem Versand
- **WHEN** ein Mail-Job existiert und ein aktives Mitglied einen `mail_recipients`-Eintrag mit status "sent" hat
- **THEN** SHALL das Mitglied NICHT in der Ergebnisliste enthalten sein

#### Scenario: Mitglied mit ausstehender Zustellung
- **WHEN** ein Mail-Job existiert und ein aktives Mitglied einen `mail_recipients`-Eintrag mit status "pending" hat
- **THEN** SHALL das Mitglied in der Ergebnisliste enthalten sein

#### Scenario: Gelöschtes Mitglied
- **WHEN** ein Mitglied soft-deleted ist (deleted IS NOT NULL)
- **THEN** SHALL das Mitglied NICHT in der Ergebnisliste enthalten sein, unabhängig vom Mail-Status

#### Scenario: Ungültige Job-ID
- **WHEN** die angegebene job_id keinem existierenden Mail-Job entspricht
- **THEN** SHALL der Endpoint einen 404-Fehler zurückgeben

### Requirement: Mail-Service liefert nicht-erreichte Member-IDs
Der Mail-Service SHALL eine Methode bereitstellen, die zu einem gegebenen Mail-Job die UUIDs aller Mitglieder zurückgibt, die erfolgreich erreicht wurden (status = "sent"). Die Invertierung (alle aktiven Mitglieder minus erfolgreich erreichte) erfolgt in der REST-Schicht.

#### Scenario: Job mit gemischten Ergebnissen
- **WHEN** ein Mail-Job 3 Recipients hat (2x sent, 1x failed)
- **THEN** SHALL die Methode genau die 2 Member-IDs mit status "sent" zurückgeben

#### Scenario: Job ohne erfolgreiche Zustellungen
- **WHEN** ein Mail-Job existiert, aber kein Recipient status "sent" hat
- **THEN** SHALL die Methode eine leere Liste zurückgeben

### Requirement: Frontend-Filter für nicht erreichte Mitglieder
Die Mitgliederliste im Frontend SHALL ein Dropdown bereitstellen, das die vorhandenen Mail-Jobs auflistet. Bei Auswahl eines Jobs SHALL die Mitgliederliste nur Mitglieder anzeigen, die durch diesen Job nicht erfolgreich erreicht wurden.

#### Scenario: Kein Job ausgewählt
- **WHEN** kein Mail-Job im Dropdown ausgewählt ist
- **THEN** SHALL die Mitgliederliste wie gewohnt alle Mitglieder anzeigen (bestehende Filter gelten weiter)

#### Scenario: Job ausgewählt
- **WHEN** ein Mail-Job im Dropdown ausgewählt wird
- **THEN** SHALL die Mitgliederliste durch den neuen Endpoint gefiltert werden und nur nicht erreichte Mitglieder anzeigen

#### Scenario: Kombination mit bestehenden Filtern
- **WHEN** ein Mail-Job-Filter aktiv ist und gleichzeitig der Suchfilter verwendet wird
- **THEN** SHALL die Suche auf der bereits gefilterten (nicht erreichten) Liste arbeiten
