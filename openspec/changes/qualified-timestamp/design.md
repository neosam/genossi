## Context

Das Audit-Log-System (Change `audit-log`) implementiert eine SHA256-Hash-Chain über alle Änderungen an Member, MemberAction, MemberDocument und Application. Die Hash-Chain erkennt Manipulationen innerhalb der Datenbank, kann aber von einem Admin mit Datenbankzugang komplett neu berechnet werden.

Um einen gerichtsfesten, externen Nachweis zu schaffen, muss der aktuelle Hash-Stand regelmäßig bei einem qualifizierten Vertrauensdiensteanbieter gemäß eIDAS Art. 41 Abs. 2 verankert werden. Dies schafft eine Beweislastumkehr — die Gegenseite müsste beweisen, dass die Daten manipuliert wurden.

Das System hat bereits einen Backup-Worker, der periodisch Daten auf WebDAV/Nextcloud hochlädt, sowie einen Config-Store für die Konfiguration externer Dienste.

**Voraussetzung**: Change `audit-log` muss implementiert sein.

## Goals / Non-Goals

**Goals:**
- Wöchentliche (konfigurierbar) Verankerung des aktuellen Audit-Log-Hash bei einem qualifizierten RFC 3161 Zeitstempeldienst
- Eigenständiger Timestamp-Worker, unabhängig vom Backup-Worker
- Manueller Timestamp-Trigger für Admins über REST-API und Frontend
- Speicherung der signierten .tsr-Token-Dateien lokal in SQLite und optional auf WebDAV/Nextcloud
- Konfiguration über Config-Store mit Frontend-UI (TSA-URL, Credentials, Aktivierung)
- Verifizierung der Zeitstempel über REST-API und Frontend-UI

**Non-Goals:**
- Eigener Zeitstempeldienst betreiben
- Unterstützung mehrerer TSA-Anbieter gleichzeitig
- Automatischer Wechsel des TSA-Anbieters bei Ausfall
- Blockchain-basierte Verankerung (OpenTimestamps etc.)
- Langzeit-Archivierung der .tsr-Dateien über WebDAV hinaus

## Decisions

### 1. RFC 3161 Timestamp Protocol

**Entscheidung**: Verwendung des RFC 3161 Timestamp Protocol über HTTP(S) zur Kommunikation mit dem Zeitstempeldienst.

**Rationale**: RFC 3161 ist der Standard für qualifizierte Zeitstempel unter eIDAS. Alle qualifizierten Anbieter in Deutschland (DGN, D-TRUST, secrypt) unterstützen dieses Protokoll. Ein HTTP-POST mit einem TimeStampReq (ASN.1/DER) liefert eine TimeStampResp mit dem signierten Token zurück.

**Alternative**: Proprietäre APIs einzelner Anbieter — weniger portabel, kein Vorteil.

### 2. DGN als empfohlener Anbieter, aber konfigurierbar

**Entscheidung**: Der TSA-Endpoint ist über den Config-Store konfigurierbar. Dokumentation empfiehlt DGN Zeitstempeldienst Standard (0,06 EUR/Stempel, keine Mindestmenge, kein Vertrag).

**Rationale**: DGN ist günstig, qualifiziert, und unkompliziert. Durch Konfigurierbarkeit bleibt das System aber anbieterunabhängig.

### 3. Eigenständiger Timestamp-Worker

**Entscheidung**: Der Zeitstempel wird durch einen eigenständigen periodischen Worker erstellt, unabhängig vom Backup-Worker. Das Intervall wird über den Config-Key `tsa_interval_hours` gesteuert (Default: 168 = 7 Tage). Bei 5 kostenlosen Stempeln/Monat bei DGN reicht ein wöchentlicher Stempel für den kostenlosen Betrieb. (Revidiert 2026-04-15)

**Rationale**: Entkopplung von Backup und Timestamping ermöglicht unabhängige Konfiguration der Frequenzen. Ein wöchentlicher Stempel ist für eine Genossenschaftsverwaltung ausreichend — das maximale Manipulationsfenster von 7 Tagen ist akzeptabel. Der Timestamp-Worker benötigt keinen WebDAV-Zugang als Voraussetzung; wenn WebDAV konfiguriert ist, wird die .tsr-Datei hochgeladen, sonst nur lokal gespeichert.

**Verworfene Alternative**: Integration in den Backup-Worker — einfacher, aber koppelt zwei unabhängige Anliegen und erzwingt gleiches Intervall.

### 4. .tsr-Token auf WebDAV speichern

**Entscheidung**: Die signierten Zeitstempel-Token werden als `.tsr`-Dateien im Verzeichnis `audit-timestamps/` auf dem konfigurierten WebDAV-Server gespeichert. Dateiname: `audit-checkpoint-{ISO8601-Timestamp}.tsr`.

**Rationale**: WebDAV ist bereits angebunden. Nextcloud bietet automatische Versionierung. Die Dateien sind winzig (~2-5 KB). Der Beweiswert steckt in der DGN-Signatur, nicht im Speicherort — selbst wenn der Admin die .tsr-Datei löscht, kann er keine gültige neue erstellen.

### 5. Lokale Timestamp-Tracking-Tabelle

**Entscheidung**: Eine `audit_timestamp` Tabelle in SQLite speichert Metadaten zu jedem erstellten Zeitstempel:

```sql
CREATE TABLE audit_timestamp (
    id BLOB NOT NULL PRIMARY KEY,
    timestamp TEXT NOT NULL,
    audit_hash TEXT NOT NULL,
    audit_entry_count INTEGER NOT NULL,
    tsr_token BLOB NOT NULL,
    webdav_path TEXT,
    status TEXT NOT NULL
);
```

**Rationale**: Ermöglicht die Verifizierung ohne WebDAV-Zugriff (Token ist lokal gespeichert). Die WebDAV-Kopie dient als externes Backup. Status trackt ob der Upload erfolgreich war.

### 6. Verifizierung

**Entscheidung**: Verifizierung in zwei Stufen:
1. **Token-Signatur prüfen**: Prüfen, ob das TSA-Zertifikat gültig ist und das Token nicht manipuliert wurde
2. **Hash-Abgleich**: Prüfen, ob der im Token verankerte Hash mit dem aktuellen Audit-Log-Hash (zum Zeitpunkt des Stempels) übereinstimmt

**Für Stufe 1** wird das öffentliche Zertifikat des TSA-Anbieters benötigt. Dies kann im Config-Store als Pfad hinterlegt oder fest eingebaut werden.

### 7. Manueller Timestamp-Trigger

**Entscheidung**: Admins können über einen REST-Endpoint `POST /api/audit/timestamps` jederzeit manuell einen Zeitstempel auslösen. Das Frontend zeigt dafür einen Button im Audit-/Timestamp-Bereich. Die Duplikat-Erkennung (Hash unverändert seit letztem Stempel → überspringen) greift auch bei manuellen Triggern. (Entschieden 2026-04-15)

**Rationale**: Ermöglicht gezielte Verankerung bei wichtigen Anlässen (z.B. vor Mitgliederversammlungen, nach großen Imports). Kostenrisiko ist vernachlässigbar — bei DGN kosten zusätzliche Stempel über die 5 Gratis/Monat hinaus 0,09 EUR/Stück.

### 8. ASN.1/RFC 3161 Bibliothek

**Entscheidung**: Verwendung der RustCrypto-Crates (`der`, `cms`, `x509-cert`, `sha2`) für ASN.1-Encoding des TimeStampReq und Parsing der TimeStampResp. Der HTTP-Transport erfolgt über das bereits vorhandene `reqwest`.

**Alternative**: `openssl`-Bindings — schwerer zu kompilieren, größere Dependency, aber battle-tested. Falls die RustCrypto-Crates für RFC 3161 nicht ausreichen, Fallback auf `openssl`.

## Risks / Trade-offs

**[TSA-Anbieter nicht erreichbar]** → Timestamp-Worker loggt den Fehler, setzt Status auf "tsa_failed", versucht beim nächsten Zyklus erneut. Der reguläre Backup-Worker ist davon nicht betroffen. Lücke im Timestamp-Nachweis, aber kein Datenverlust.

**[ASN.1-Komplexität]** → RFC 3161 Request/Response ist ein komplexes ASN.1-Format. Mitigation: Minimale Implementierung — nur TimeStampReq erstellen und TimeStampResp parsen, keine vollständige RFC-3161-Bibliothek.

**[Zertifikatswechsel beim TSA]** → Wenn der TSA-Anbieter sein Zertifikat erneuert, müssen alte Tokens noch mit dem alten Zertifikat verifizierbar sein. Mitigation: Tokens enthalten das Zertifikat oder eine Referenz darauf; bei der Verifizierung das eingebettete Zertifikat verwenden.

**[Kosten bei Anbieter-Ausfall]** → Falls DGN seinen Dienst einstellt, muss auf einen anderen Anbieter gewechselt werden. Mitigation: Config-Store-basierte Konfiguration ermöglicht Anbieterwechsel ohne Code-Änderung.

**[Admin löscht lokale .tsr-Dateien]** → Kein Problem: Kopie liegt auf WebDAV. Wenn Admin auch WebDAV kontrolliert: Nextcloud-Versionshistorie. Im schlimmsten Fall: Token kann beim TSA-Anbieter nicht erneut angefragt werden, aber der Hash-Chain-Zustand zum Stempelzeitpunkt ist durch das Token belegt, falls irgendeine Kopie existiert.

## Resolved Questions

- **Fehlendes TSA-Config**: Der Backup-Worker läuft normal weiter und überspringt den Timestamp-Schritt ohne Warnung. (Entschieden 2026-04-15)
- **UI für TSA-Konfiguration**: Es wird eine Frontend-UI geben, um den TSA-Anbieter zu konfigurieren (URL, Credentials, Aktivierung). Die Config-Store-API allein reicht nicht. (Entschieden 2026-04-15)
