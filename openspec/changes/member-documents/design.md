## Context

Genossi verwaltet Genossenschaftsmitglieder über eine geschichtete Architektur (DAO → Service → REST → Frontend). Es gibt bisher keine Möglichkeit, Dateien/Dokumente zu Mitgliedern zu speichern. Der Excel-Import nutzt bereits Multipart-Upload, aber es existiert kein generisches Dokumenten-System.

Die Member-Details-Seite hat bereits eine Actions-Sektion mit inline CRUD-Formular — das Dokumente-Feature wird dem gleichen UI-Pattern folgen.

## Goals / Non-Goals

**Goals:**
- Dokumente pro Mitglied hochladen, auflisten, herunterladen und soft-deleten
- Typisierte Dokumente mit Singleton-Semantik (Beitrittserklärung, Beitrittsbestätigung) und Multi-Semantik (Aufstockung, Sonstige)
- Dateien im Dateisystem speichern, Metadaten in SQLite
- Zugriff nur für Vorstände
- Frontend-UI in der Member-Details-Seite

**Non-Goals:**
- Dokumentengenerierung (PDF erzeugen aus Mitgliedsdaten) — kommt später
- Versionierung von Dokumenten — Singleton-Ersetzung nutzt Soft-Delete
- Volltextsuche in Dokumenten
- Vorschau/Preview im Browser
- S3/Cloud-Storage — nur Dateisystem

## Decisions

### 1. Neuer Storage-Layer als Trait

**Entscheidung:** Ein neues `DocumentStorage` Trait wird eingeführt, das Dateisystem-Operationen abstrahiert.

```rust
#[async_trait]
pub trait DocumentStorage: Send + Sync {
    async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError>;
    async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>;
    async fn delete(&self, relative_path: &str) -> Result<(), StorageError>;
}
```

**Warum:** Trennt Datei-I/O vom Business-Logic-Layer. Ermöglicht spätere Migration zu S3 ohne Änderungen am Service-Layer. Macht den Service-Layer testbar mit einem Mock-Storage.

**Alternative:** Datei-I/O direkt im Service-Layer → Schlechter testbar, vermischt Verantwortlichkeiten.

### 2. Opake UUID-Dateinamen im Dateisystem

**Entscheidung:** Dateien werden als `{document-uuid}.{extension}` im konfigurierten Basispfad gespeichert. Keine Unterordner.

**Warum:** Vermeidet Sonderzeichen-Probleme in Dateinamen, Race-Conditions bei Umbenennung, und vereinfacht den Storage-Layer. Die DB ist die einzige Quelle der Wahrheit.

**Alternative:** Ordnerstruktur pro Member-ID → Nett für manuelles Browsing, aber mehr Komplexität für keinen echten Nutzen.

### 3. Dokumenttyp als Text-Enum in der DB

**Entscheidung:** `document_type` wird als TEXT in SQLite gespeichert. Werte: `join_declaration`, `join_confirmation`, `share_increase`, `other`. Bei `other` enthält ein separates `description`-Feld die Beschreibung.

**Warum:** Lesbar in der DB, erweiterbar, und der `Other(description)`-Fall wird sauber über ein separates Feld gelöst statt über String-Encoding im Typ.

**Alternative:** Integer-Enum → Weniger lesbar, spart keine Bytes bei SQLite TEXT-Storage.

### 4. Singleton-Semantik im Service-Layer

**Entscheidung:** Beim Upload eines Singleton-Dokuments (JoinDeclaration, JoinConfirmation) prüft der Service-Layer, ob bereits ein aktives Dokument dieses Typs für das Mitglied existiert. Falls ja, wird es soft-deleted, bevor das neue Dokument erstellt wird. Die Datei auf dem Dateisystem bleibt erhalten.

**Warum:** Konsistent mit dem bestehenden Soft-Delete-Pattern im Gesamtsystem. Bietet Historie ohne manuelle Intervention.

### 5. Kein eigenes DAO-Modul — Integration in bestehendes Pattern

**Entscheidung:** Das Dokument-Feature folgt exakt dem bestehenden Entity-Pattern: `MemberDocumentEntity` im DAO, `MemberDocument` im Service, `MemberDocumentTO` im REST-Layer. Zusätzlich bekommt das DAO eine `find_by_member_id`-Methode.

**Warum:** Konsistenz mit dem Rest der Architektur. Das `gen_service_impl!`-Macro kann genutzt werden.

### 6. REST-Endpunkte als Sub-Ressource von Members

**Entscheidung:** Dokumente werden unter `/members/{member_id}/documents` genested. Upload via Multipart, Download liefert die Rohbytes mit korrektem Content-Type.

```
POST   /members/{id}/documents        → Upload (multipart/form-data)
GET    /members/{id}/documents         → Liste (JSON)
GET    /members/{id}/documents/{doc_id} → Download (Binär)
DELETE /members/{id}/documents/{doc_id} → Soft-Delete
```

**Warum:** RESTful Sub-Ressource ist semantisch korrekt. Das Pattern existiert schon für Member-Actions.

### 7. Frontend als Sektion in Member-Details

**Entscheidung:** Die Dokumente-Sektion wird nach dem Actions-Block in `member_details.rs` eingefügt. Gleiches Pattern: einblendbare Formular-Sektion, Tabelle mit Einträgen, Download- und Delete-Buttons.

**Warum:** Konsistente UX, wenig neuer Code. Multipart-Upload im WASM-Frontend erfolgt über `web_sys::FormData` und `fetch`.

## Risks / Trade-offs

**[Dateisystem-Konsistenz]** → DB und Dateisystem können auseinanderlaufen (DB-Eintrag ohne Datei oder umgekehrt).
→ Mitigation: Service-Layer schreibt zuerst die Datei, dann den DB-Eintrag. Bei DB-Fehler wird die Datei wieder gelöscht. Soft-Delete löscht nur den DB-Eintrag, die Datei bleibt.

**[50 MB Uploads]** → Große Uploads könnten den Server blockieren.
→ Mitigation: Axum Multipart Streaming, keine In-Memory-Pufferung des gesamten Requests. Größenlimit wird im REST-Layer geprüft.

**[WASM File-Upload]** → Dioxus/WASM hat keine native File-Upload-Komponente. Erfordert `web_sys`-Interop.
→ Mitigation: `web_sys::FormData` + `web_sys::Request` ist ein bekanntes Pattern. Bereits im Excel-Import-Feature der Fall (dort allerdings nicht im Frontend).

**[Keine Backup-Strategie für Dateien]** → DB-Backup allein sichert nicht die Dokumente.
→ Mitigation: Dokumentation, dass `DOCUMENT_STORAGE_PATH` ins Backup einbezogen werden muss. Kein Code-Mitigation nötig.
