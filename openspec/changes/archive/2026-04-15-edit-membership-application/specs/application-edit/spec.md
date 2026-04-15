## ADDED Requirements

### Requirement: Admin kann Mitgliedsantrag aktualisieren
Das System MUSS Admins ermöglichen, die Felder eines bestehenden Mitgliedsantrags über `PUT /api/applications/{id}` zu aktualisieren. Aktualisierbare Felder: `salutation`, `title`, `first_name`, `last_name`, `email`, `street`, `house_number`, `postal_code`, `city`, `shares`. Das `version`-Feld MUSS für Optimistic Locking mitgesendet werden.

#### Scenario: Erfolgreiche Aktualisierung
- **WHEN** ein Admin einen PUT-Request mit gültigen Daten und korrekter Version an `/api/applications/{id}` sendet
- **THEN** werden die Felder des Antrags aktualisiert und der aktualisierte Antrag mit neuer Version zurückgegeben

#### Scenario: Versionskonflikt
- **WHEN** ein Admin einen PUT-Request mit veralteter Version sendet
- **THEN** gibt das System einen 409 Conflict-Fehler zurück

#### Scenario: Antrag nicht gefunden
- **WHEN** ein Admin einen PUT-Request an eine nicht existierende Antrags-ID sendet
- **THEN** gibt das System einen 404 Not Found-Fehler zurück

#### Scenario: Pflichtfelder fehlen
- **WHEN** ein Admin einen PUT-Request ohne `first_name`, `last_name` oder mit `shares` < 1 sendet
- **THEN** gibt das System einen 422 Validation Error zurück

### Requirement: Wiederverwendbares Antragsformular
Das Frontend MUSS ein einzelnes `ApplicationForm`-Komponente bereitstellen, das sowohl für das Erstellen als auch das Bearbeiten von Anträgen verwendet wird. Im Edit-Modus MUSS das Formular mit den bestehenden Antragsdaten vorbefüllt sein.

#### Scenario: Formular im Create-Modus
- **WHEN** das Formular im Create-Modus geöffnet wird
- **THEN** sind alle Felder leer, die "Mail senden"-Checkbox ist sichtbar, und Submit erstellt einen neuen Antrag

#### Scenario: Formular im Edit-Modus
- **WHEN** das Formular im Edit-Modus mit einem bestehenden Antrag geöffnet wird
- **THEN** sind alle Felder mit den Antragsdaten vorbefüllt, die "Mail senden"-Checkbox ist nicht sichtbar, und Submit aktualisiert den Antrag

### Requirement: Edit-Button in Detailansicht
Die `ApplicationDetail`-Ansicht MUSS einen "Bearbeiten"-Button anzeigen. Klick auf den Button öffnet das Antragsformular im Edit-Modus.

#### Scenario: Bearbeiten-Button öffnet Formular
- **WHEN** ein Admin in der Detailansicht eines Antrags auf "Bearbeiten" klickt
- **THEN** wird das Antragsformular im Edit-Modus mit den Daten des Antrags geöffnet

#### Scenario: Nach erfolgreicher Bearbeitung
- **WHEN** der Admin das Edit-Formular erfolgreich absendet
- **THEN** wird das Formular geschlossen und die Antragsliste aktualisiert
