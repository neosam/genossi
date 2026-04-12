## ADDED Requirements

### Requirement: Bulk-Endpunkt für Dokumenten-Counts
Das System MUSS einen Endpunkt `GET /api/member-documents/counts?type={document_type}` bereitstellen, der pro Mitglied die Anzahl aktiver (nicht gelöschter) Dokumente eines bestimmten Typs liefert.

#### Scenario: Counts für einen Dokumenttyp abrufen
- **WHEN** ein Vorstandsmitglied `GET /api/member-documents/counts?type=join_declaration` aufruft
- **THEN** liefert das System eine JSON-Map mit Member-IDs als Schlüssel und Dokumenten-Anzahl als Wert
- **AND** nur Mitglieder mit mindestens einem aktiven Dokument dieses Typs werden aufgeführt

#### Scenario: Kein Mitglied hat Dokumente des Typs
- **WHEN** kein Mitglied ein aktives Dokument des angefragten Typs hat
- **THEN** liefert das System eine leere JSON-Map `{}`

#### Scenario: Soft-deleted Dokumente werden nicht gezählt
- **WHEN** ein Mitglied nur gelöschte Dokumente des angefragten Typs hat
- **THEN** wird dieses Mitglied nicht in der Ergebnis-Map aufgeführt

#### Scenario: Ungültiger Dokumenttyp
- **WHEN** ein ungültiger Wert für den `type`-Parameter übergeben wird
- **THEN** liefert das System einen 400-Fehler

#### Scenario: Fehlender type-Parameter
- **WHEN** der `type`-Parameter fehlt
- **THEN** liefert das System einen 400-Fehler

### Requirement: Board-only Zugriff auf Counts-Endpunkt
Der Counts-Endpunkt MUSS Vorstandsberechtigungen erfordern. Nicht-Vorstandsmitglieder MÜSSEN einen 403-Fehler erhalten.

#### Scenario: Nicht-Vorstand ruft Counts ab
- **WHEN** ein Benutzer ohne Vorstandsberechtigung den Counts-Endpunkt aufruft
- **THEN** liefert das System einen 403-Fehler

#### Scenario: Vorstand ruft Counts ab
- **WHEN** ein Vorstandsmitglied den Counts-Endpunkt aufruft
- **THEN** wird die Anfrage zugelassen
