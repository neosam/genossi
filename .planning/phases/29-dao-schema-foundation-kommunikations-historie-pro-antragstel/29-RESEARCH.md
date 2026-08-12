# Phase 29: DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller) — Research

**Researched:** 2026-08-12
**Domain:** Additive DAO/Schema-Erweiterung des bestehenden `genossi_mail`-Subsystems — `application_id`-Linkage auf `mail_recipients`, outbound-only Application-Timeline-Query, Carry-over bei `confirm()`
**Confidence:** HIGH — alle Aussagen sind direkt gegen den aktuellen Code verifiziert (Datei:Zeile-Referenzen durchgehend). Ausnahme: die D2-Empfehlung ist eine begründete Architektur-Entscheidung, kein verifizierter Fakt.

---

## User Constraints (aus ROADMAP.md Phase-29-Detail + REQUIREMENTS.md D1/D2/D3)

> Es existiert KEINE phasen-spezifische CONTEXT.md. Die verbindlichen Entscheidungen stehen in ROADMAP.md (Phase-29-Detail) und REQUIREMENTS.md. Sie werden hier als Locked Decisions behandelt.

### Locked Decisions

- **APHIST-01** `[CITED: REQUIREMENTS.md:28]`: Alle an eine Application gesendeten Mails werden über `application_id`-Linkage an `mail_recipients` erfasst — KEIN `member_id`-Overload. Endpoint (später, Phase 31): `GET /api/applications/{id}/communications`.
- **APHIST-03** `[CITED: REQUIREMENTS.md:30]`: Nach `confirm()` erscheint die als Antragsteller gesendete Erinnerung in der Mitglieds-Timeline des neuen Mitglieds (Carry-over, D2). Mechanismus wird in DIESER Phase (Planung) festgelegt; e2e-verifiziert: Erinnerung → confirm → sichtbar.
- **D2** `[CITED: ROADMAP.md:120]`: Carry-over-Mechanismus (Back-fill `member_id` / Union-at-read / Link-Spalte) wird in Phase-29-Planung entschieden.
- **Audit-Scope v1.6** `[CITED: ROADMAP.md:123]`: KEIN Audit-Log für die `application_id`-Linkage. KEIN neues Feld auf dem auditierten `ApplicationEntity`. KEINE neue Backend-Dependency ("add nothing").
- **Migration-Contract** `[CITED: ROADMAP.md:228]`: forward-only, additiv (nullable `application_id BLOB` + Index). Bestehende Zeilen ohne `application_id` bleiben byte-identisch (NULL-Legacy-Roundtrip). Jede `mail_recipients`-SQL-Spaltenliste ist auf die neue Spalte zu prüfen.

### Claude's Discretion (in der Planung zu entscheiden)

- Konkreter D2-Mechanismus (diese Research empfiehlt A — siehe D2-Analyse).
- Ob die neue DAO-Methode `get_application_communications` heißt und ob der Carry-over als neue `MailService`-Methode oder als neue `CommunicationDao`-Methode implementiert wird.
- Timestamp/Dateiname der Migration im Projekt-Konventions-Stil.

### Deferred Ideas (OUT OF SCOPE für Phase 29)

- REST-Endpoint `POST /api/applications/{id}/mail` + `GET /api/applications/{id}/communications` → **Phase 31**.
- `ApplicationService::send_mail`, Status-Guard (`Offen`-only), "zuletzt gesendet" → **Phase 31**.
- Template-Kontext, `application_to_template_context`, offener Betrag → **Phase 30**.
- Frontend Compose-Dialog → **Phase 32**.
- Inbound-/Reply-Threading in die Antragsteller-Timeline (Antragsteller-Timeline ist **outbound-only**).

---

## Phase Requirements

| ID | Beschreibung | Research Support |
|----|--------------|------------------|
| APHIST-01 | Application-gesendete Mails via `application_id`-Linkage erfassbar, kein `member_id`-Overload | Ist-Kartierung `mail_recipients`/`MailRecipient`/`RecipientInput`/`create_job`; Migration-Muster; neue outbound-only DAO-Query |
| APHIST-03 | Carry-over bei `confirm()` → Erinnerung in Member-Timeline sichtbar | `confirm()`-Flow kartiert (application.rs:288-557); D2-Trade-off-Analyse mit Empfehlung A |

---

## Zusammenfassung & Empfehlung

Diese Phase ist reine additive Integration im bestehenden `genossi_mail`-Crate — keine neue Dependency, keine neue Entität, kein Audit. Der Bauplan ist ein exakter Spiegel des vorhandenen `member_id`-Pfads: `mail_recipients` bekommt eine zweite nullable Linkage-Spalte `application_id BLOB`, die durch Struct (`MailRecipient`), Service-Input (`RecipientInput`), `create_job` und alle ~5 verbatim SQL-Spaltenlisten gefädelt wird; dazu eine neue outbound-only Timeline-Query analog `get_member_communications`.

**Kernbefund für D2 (Carry-over):** Sowohl `MemberEntity` als auch `ApplicationEntity` sind auditiert `[VERIFIED: genossi_dao/src/application.rs:60, member ist ebenfalls auditiert]`. Jede Link-Spalte auf einer dieser beiden Entitäten löst Audit-Ripple aus und bricht den gelockten Test `test_auditable_fields_count` (`assert_eq!(fields.len(), 11)`, application.rs:176). Das disqualifiziert Option C und die Stored-Link-Variante von Option B faktisch im Rahmen der Scope-Grenzen. **Die einzige Carry-over-Variante, die ohne Audit-Berührung auskommt, ist Option A: Back-fill der ECHTEN neuen `member_id` auf den `mail_recipients`-Zeilen (nicht-auditierte Tabelle) mit passendem `application_id`.** Die Member-Timeline-Query (`WHERE r.member_id = ?1`) bleibt dabei unverändert, und Success-Kriterium 2 ist gewahrt, weil niemals die Application-UUID, sondern immer die genuine neue Member-UUID gesetzt wird.

**Kritische architektonische Randbedingung:** Die Mail-DAOs (`MailRecipientDaoSqlite`) operieren direkt auf `Arc<SqlitePool>` `[VERIFIED: genossi_bin/src/lib.rs:960]`, NICHT über die `TransactionDao`-Transaktion, die `confirm()` nutzt `[VERIFIED: genossi_service_impl/src/application.rs:293]`. Ein Back-fill kann daher **nicht atomar** in die `confirm()`-Transaktion eingebunden werden. Er muss als **best-effort Post-Commit-Schritt** laufen — konsistent mit den bereits vorhandenen Post-Commit-Best-Effort-Operationen in `confirm()` (Datei-Cleanup, application.rs:547) und `send_confirmation_mail`.

**Primary recommendation:** Additive nullable `application_id BLOB`-Spalte + Index spiegelbildlich zu `member_id`; alle 5 verbatim SQL-Listen in einem Commit anfassen; neue `get_application_communications`-Query (outbound-only, ohne den Inbound-`UNION`-Zweig); **D2 = Option A (Back-fill echte member_id, best-effort post-commit)** via neuer `MailService`-Methode, aufgerufen am Ende von `confirm()`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `application_id`-Persistenz | DAO/SQLite (`genossi_mail`) | Service (`create_job`) | Spalte muss existieren+persistiert werden, bevor der Service sie stempelt (harte Reihenfolge) |
| Application-Timeline-Read | DAO/SQLite (`CommunicationDao`) | REST (Phase 31) | Query lebt neben `get_member_communications`; `CommunicationEntry` ist bereits subjekt-agnostisch |
| Carry-over bei confirm | Service (`ApplicationServiceImpl::confirm`) | Mail-DAO (UPDATE) | Fachlogik-Hook im confirm-Flow; Mutation auf nicht-auditierter Mail-Tabelle |
| Namespace-Integrität (member_id sauber) | DAO/Service-Contract | Test/Grep-Gate | Invariante: Application-UUID nie in `member_id` |

---

## Ist-Zustand Code-Kartierung

### 1. `mail_recipients`-Schema (Migrationen)

Basistabelle `[VERIFIED: migrations/sqlite/20260403000004_create_mail_recipients_table.sql]`:

```sql
CREATE TABLE IF NOT EXISTS mail_recipients (
    id BLOB PRIMARY KEY,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    mail_job_id BLOB NOT NULL REFERENCES mail_jobs(id),
    to_address TEXT NOT NULL,
    member_id BLOB,            -- nullable, KEIN NOT NULL (Vorbild für application_id)
    status TEXT NOT NULL,
    error TEXT,
    sent_at TEXT
);
```

Additive Folgemigrationen (jede fügt nur eine nullable Spalte hinzu, forward-only, kein DROP):
- `20260409000000_add_message_id_to_mail_recipients.sql`: `ALTER TABLE mail_recipients ADD COLUMN message_id TEXT;`
- `20260614000000_mail_recipient_rendered_subject_body.sql`: `ADD COLUMN rendered_subject TEXT; ADD COLUMN rendered_body TEXT;` — enthält den kanonischen Kommentar zur NULL-Legacy-Semantik und Forward-Only-Policy `[VERIFIED]`.
- `20260702000002_mail_recipients_add_rendered_html_body.sql`: `ADD COLUMN rendered_html_body TEXT NULL;` — dokumentiert "NULL-legacy semantics" explizit.

Index-Vorbild `[VERIFIED: migrations/sqlite/20260411000001_add_member_communication_indexes.sql]`:
```sql
CREATE INDEX IF NOT EXISTS idx_mail_recipients_member_id ON mail_recipients(member_id);
```

**Befund:** `member_id BLOB` ist bereits nullable und wird als Timeline-Join-Key genutzt. `application_id BLOB` wird 1:1 als Geschwisterspalte ergänzt. Es gibt **keine** Down-Migrationen im Projekt (SQLite < 3.35 kann Spalten nicht droppen) — die Spaltenform muss beim ersten Mal stimmen.

### 2. `MailRecipient` (DAO-Struct) und `RecipientInput` (Service-Input)

`MailRecipient` `[VERIFIED: genossi_mail/src/dao.rs:55-80]` — `member_id: Option<Uuid>` bei :62. `application_id: Option<Uuid>` additiv daneben ergänzen.

`RecipientInput` `[VERIFIED: genossi_mail/src/service.rs:54-57]`:
```rust
pub struct RecipientInput {
    pub address: String,
    pub member_id: Option<Uuid>,   // application_id: Option<Uuid> additiv daneben
}
```

**SQLite-Row-Struct** `MailRecipientDb` `[VERIFIED: genossi_mail/src/dao_sqlite.rs:207-225]` und `TryFrom<&MailRecipientDb> for MailRecipient` (:227-250) — auch hier ein `application_id: Option<Vec<u8>>`-Feld + `parse_optional_uuid(&db.application_id)?` ergänzen (Muster: `member_id: parse_optional_uuid(&db.member_id)?`, :239).

### 3. Alle verbatim `mail_recipients`-SQL-Spaltenlisten (Success-Kriterium 4)

Grep-verifiziert `[VERIFIED: genossi_mail/src/dao_sqlite.rs]` — die Spaltenliste steht an **5 produktiven Stellen** plus 1 Test-DDL:

| Zeile | Kontext | Anfassung nötig |
|-------|---------|-----------------|
| :272-273 | `INSERT INTO mail_recipients (...)` in `create()` | **JA** — Spalte + Bind (`.bind(application_id)`) ergänzen |
| :295-296 | `SELECT ... FROM mail_recipients WHERE mail_job_id` (`find_by_job_id`) | **JA** — Spalte in SELECT-Liste |
| :311-312 | `SELECT r.... FROM mail_recipients r ...` (`next_pending`) | **JA** — Spalte in SELECT-Liste |
| :363-364 | `SELECT member_id FROM mail_recipients ...` (`find_sent_member_ids_by_job_id`) | Nein (selektiert nur member_id) — aber prüfen |
| :379-380 | `SELECT ... FROM mail_recipients` (`find_recipients_without_rendered`) | **JA** — Spalte in SELECT-Liste |
| :1080-1082 | Timeline-UNION `FROM mail_recipients r ... WHERE r.member_id = ?1` | Vorlage für neue Query (nicht ändern; klonen) |
| :1341-1361 | Test-DDL `CREATE TABLE mail_recipients (...)` (In-Memory-Test-Schema) | **JA** — muss `application_id BLOB` enthalten, sonst brechen DAO-Tests |

> **Achtung Test-DDL (:1341):** Die DAO-Unit-Tests bauen ihr eigenes Schema in-memory (NICHT über die Migrationsdateien). Die neue Spalte muss dort mit ergänzt werden, sonst schlägt der INSERT-Roundtrip fehl. Das ist eine leicht übersehene 6. Stelle.

**INSERT-Detail** (:271-287): Die VALUES-Liste mischt Binds und Literale — `VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?, NULL)`. `application_id` als weiteres `?` mit `.bind(recipient.application_id.map(|a| a.as_bytes().to_vec()))` hinzufügen (Muster: `member_id` bind, :269+:280).

### 4. `create_job` (Job-/Recipient-Erzeugung)

`[VERIFIED: genossi_mail/src/service.rs:356-473]`. Signatur bleibt **unverändert** — die Linkage reitet auf `RecipientInput`. In der Recipient-Schleife (:432-450) wird `MailRecipient` gebaut:

```rust
for input in &recipients {
    let recipient = MailRecipient {
        ...
        member_id: input.member_id,      // :440
        // application_id: input.application_id,   // NEU — additiv
        ...
    };
    self.recipient_dao.create(&recipient).await?;
```

**Befund:** Genau eine Zeile (`application_id: input.application_id,`) in `create_job`; keine Signaturänderung, keine Auswirkung auf bestehende Aufrufer (Feld defaultet konzeptionell auf `None`, wo es nicht gesetzt wird — bestehende `RecipientInput`-Literale müssen aber ggf. um `application_id: None` ergänzt werden, siehe unten).

**Wichtige Nebenwirkung:** `RecipientInput` hat kein `#[derive(Default)]` und wird an mehreren Stellen als Struct-Literal gebaut (`send_confirmation_mail` application.rs:132; Massenmail-Pfade service.rs:708/712; diverse Tests). Ein neues Pflichtfeld `application_id` zwingt jede dieser Stellen zur Ergänzung `application_id: None`. **Empfehlung:** Feld hinzufügen und alle Literale mechanisch anpassen (Compiler findet sie alle) — ODER `RecipientInput` ein `Default`/Builder geben. Die mechanische Anpassung ist konsistenter mit dem bestehenden Stil (kein Builder im Codebase).

### 5. `CommunicationDao` / Member-Timeline-Leseweg

Trait `[VERIFIED: genossi_mail/src/dao.rs:280-285]`:
```rust
pub trait CommunicationDao: Send + Sync + 'static {
    async fn get_member_communications(&self, member_id: Uuid) -> Result<Arc<[CommunicationEntry]>, MailDaoError>;
    // NEU: async fn get_application_communications(&self, application_id: Uuid) -> Result<Arc<[CommunicationEntry]>, MailDaoError>;
}
```

`CommunicationEntry` `[VERIFIED: genossi_mail/src/dao.rs:258-276]` ist **subjekt-agnostisch** (kein `member_id`-Feld) — kein Struct-Change nötig. `CommunicationEntryTO` (communication_rest.rs) ebenfalls agnostisch — kein TO-Change nötig.

Bestehende Member-Query `[VERIFIED: genossi_mail/src/dao_sqlite.rs:1042-1097]` ist ein `inbound UNION ALL outbound`. Der **Inbound-Zweig** (:1050-1063, `FROM inbound_mails WHERE assigned_member_id = ?1`) existiert für Applications NICHT (Antragsteller haben keine `assigned_member_id`-Zuordnung). Die neue Query ist daher **nur der Outbound-Zweig** (:1067-1084):

```sql
SELECT
    'outbound' AS direction,
    COALESCE(r.sent_at, r.created) AS date,
    j.subject,
    NULL AS inbox_id, NULL AS from_address,
    NULL AS inbound_done, NULL AS inbound_replied, NULL AS inbound_archived,
    j.id AS mail_job_id, r.id AS recipient_id, r.to_address, r.status AS outbound_status
FROM mail_recipients r
JOIN mail_jobs j ON j.id = r.mail_job_id
WHERE r.application_id = ?1        -- statt r.member_id
  AND r.deleted IS NULL
  AND j.deleted IS NULL
ORDER BY date DESC
```

> **Wichtig:** Die Soft-Delete-Filter `r.deleted IS NULL AND j.deleted IS NULL` (:1083-1084) müssen im Application-Zweig erhalten bleiben. Die `CommunicationEntryDb`-Row-Form (12 Spalten) bleibt identisch, weil das `SELECT`-Schema für `query_as::<_, CommunicationEntryDb>` gleich ist — es müssen alle 12 Spalten (inkl. der NULL-Inbound-Spalten) selektiert werden, sonst passt `FromRow` nicht.

**Wiring:** `CommunicationRestState` `[VERIFIED: genossi_bin/src/lib.rs:1956-1959]` + Mount `/api/members/{member_id}/communications` (`genossi_rest/src/lib.rs:647-650`). Der Application-Endpoint kommt in Phase 31; Phase 29 liefert nur die DAO-Methode + deren Tests.

### 6. `ApplicationService::confirm()`-Flow (D2-Hook-Punkt)

`[VERIFIED: genossi_service_impl/src/application.rs:288-557]`. Ablauf innerhalb `let tx = transaction_dao.use_transaction(None)` (:293):

1. Permission-Check `MANAGE_MEMBERS_PRIVILEGE` (:298-300).
2. `current_user_id` (:302-306).
3. `application_dao.find_by_id(id, tx)` — filtert soft-deleted (:308-312).
4. Status-Guard `!= Offen → Conflict` (:314-319).
5. Neues Member: `member_id = uuid_service.new_v4()` (:326) — **unabhängig von `application.id`**; `audited_create!` MemberEntity (:362).
6. `audited_create!` Eintritt- und Aufstockung-Actions (:385, :408).
7. **Phase-25-Analogon:** Falls ein `application_document` existiert → Move-Transfer zu auditiertem `MemberDocument` im selben `tx` (:417-527). **Genau das ist das nächste Vorbild** für additiven Carry-over.
8. `entity.status = Bestaetigt; audited_update!` (:530-539).
9. `transaction_dao.commit(tx)` (:541).
10. **Post-Commit best-effort:** alte Datei löschen (:547-555) — Fehler rollt NICHT zurück, nur `tracing::warn!`.

**Hook-Punkt für D2-A:** Der Back-fill gehört **nach `commit(tx)`** (nach :541), als weiterer best-effort Schritt neben dem Datei-Cleanup — weil das Mail-Subsystem auf einer separaten Connection/Transaktion läuft (siehe unten). `member_id` (:326) steht zu diesem Zeitpunkt bereits fest und ist die genuine neue Member-UUID.

### 7. Transaktions-Grenze (kritisch für D2)

`[VERIFIED: genossi_bin/src/lib.rs:959-960, 973]`: `mail_recipient_dao = MailRecipientDao::new(pool.clone())` — die Mail-DAOs teilen zwar denselben `Arc<SqlitePool>`, operieren aber via `self.pool.as_ref()` `[VERIFIED: dao_sqlite.rs:285]` — also auf einer **eigenen Pool-Connection**, NICHT auf der `TransactionDao`-Transaktion, die `confirm()` durchreicht. `ApplicationServiceDeps` enthält `MailService` (application.rs:39), aber weder eine `CommunicationDao`- noch eine `MailRecipientDao`-Abhängigkeit, die einen `tx`-Parameter akzeptiert.

**Konsequenz:** Ein Back-fill kann prinzipiell nicht atomar in die `confirm()`-Transaktion eingebunden werden. Er ist inhärent Post-Commit/best-effort — was exakt zum bestehenden Muster passt (Confirmation-Mail und Datei-Cleanup laufen ebenfalls post-commit).

### 8. `send_confirmation_mail` als Referenz-Scaffold

`[VERIFIED: genossi_service_impl/src/application.rs:44-157]`. Setzt bereits den **korrekten Präzedenzfall**: `RecipientInput { address: email, member_id: None }` (:132-135) für einen Application-Empfänger. Für Phase 31 wird hier `application_id: Some(app.id)` gesetzt. **Für Phase 29 nur als Kontext relevant** — der eigentliche Versand kommt in Phase 31. (Achtung Anti-Pattern für Phase 31: diese Methode gibt `()` zurück und schluckt alle Fehler — Pitfall 3.)

---

## Migration-Muster

Konkreter additiver Vorschlag im Projekt-Stil (Timestamp fortlaufend nach der jüngsten Migration `20260723000000`; Planer wählt finalen Timestamp):

```sql
-- migrations/sqlite/20260812000000_mail_recipients_add_application_id.sql
--
-- v1.6 Phase 29 (APHIST-01): per-Antragsteller Kommunikations-Historie.
-- Fügt eine nullable application_id-Linkage zu mail_recipients hinzu —
-- Geschwisterspalte zu member_id (20260403000004). Ein Recipient linkt auf
-- ein Mitglied ODER einen Antragsteller (mutually exclusive), nie hart erzwungen
-- (SQLite ohne echte FK-Enforcement hier).
--
-- NULL-Legacy-Semantik: bestehende/Member-Zeilen lesen application_id=NULL
-- zurück und bleiben byte-identisch. Forward-only, kein DROP COLUMN
-- (SQLite < 3.35; gleiche Konvention wie 20260702000002_..._rendered_html_body.sql).

ALTER TABLE mail_recipients ADD COLUMN application_id BLOB;

CREATE INDEX IF NOT EXISTS idx_mail_recipients_application_id
    ON mail_recipients(application_id);
```

**Begründung der Details:**
- `BLOB` (nicht `BLOB NULL` explizit) spiegelt exakt `member_id BLOB` aus der Basistabelle; SQLite-Spalten sind ohne `NOT NULL` nullable.
- Index spiegelt `idx_mail_recipients_member_id` — ohne ihn full-scannt die Timeline-Query `mail_recipients` (Architecture-Research: einziger performance-relevanter Schritt).
- Kein `DEFAULT` — Legacy-Zeilen lesen `NULL`, exakt wie beim `member_id`-Pattern.
- **Kein `cargo sqlx prepare` nötig:** Die Mail-DAOs nutzen ausschließlich Runtime-`sqlx::query`/`query_as::<_, T>` (KEINE compile-time `query!`-Makros) `[VERIFIED: grep — nur genossi_dao_impl_sqlite/src/permission.rs nutzt compile-time Makros, referenziert `mail_recipients` nicht]`. Das `.sqlx/`-Offline-Cache betrifft nur Permission-Queries. → **Korrektur zu SUMMARY.md:20**, das "cargo sqlx prepare" als Schritt nennt; für diese Migration ist es nicht erforderlich.

---

## D2-Carry-over Trade-off-Analyse (A / B / C)

**Gemeinsame Randbedingung:** `confirm()` erzeugt eine neue `member_id` ohne Rück-Link auf `application.id` `[VERIFIED: application.rs:326]`. Beide betroffenen Entitäten (`MemberEntity`, `ApplicationEntity`) sind auditiert — der gelockte Test `test_auditable_fields_count` erwartet exakt 11 Application-Audit-Felder `[VERIFIED: genossi_dao/src/application.rs:176]`.

### Option A — Back-fill echte `member_id` bei `confirm()`

**Was:** Nach `commit(tx)` in `confirm()`: `UPDATE mail_recipients SET member_id = ?new_member_id WHERE application_id = ?app_id AND member_id IS NULL`. Die Member-Timeline-Query (`WHERE r.member_id = ?1`) bleibt völlig unverändert und liefert die Erinnerung automatisch.

**Code-Auswirkung:**
- Neue `MailService`-Methode, z.B. `link_application_recipients_to_member(application_id, member_id) -> Result<(), MailServiceError>` (führt das UPDATE auf `MailRecipientDao` aus). Eine neue `MailRecipientDao`-Methode `link_application_to_member(...)` dahinter.
- Ein Aufruf am Ende von `confirm()` (post-commit, best-effort, `tracing::warn!` bei Fehler — Muster: application.rs:547-555).
- Member-Timeline-Query: **0 Änderungen**.

**Testbarkeit e2e:** Sehr gut. `get_member_communications(new_member_id)` liefert direkt den Eintrag. Der e2e "Erinnerung → confirm → sichtbar" ist ein einfacher Roundtrip ohne Query-Sonderfall.

**Scope-Konsistenz:** ✅ Kein Audit-Ripple (Mutation nur auf nicht-auditierter `mail_recipients`). ✅ Kein neues Feld auf `ApplicationEntity`/`MemberEntity`. ✅ Success-Kriterium 2 gewahrt: es wird die **genuine neue member_id** gesetzt, NIEMALS die Application-UUID.

**Risiken:**
- Zeile trägt nach Back-fill **beide** Keys (`member_id` UND `application_id`) — semantisch korrekt (diese Mail war eine Antragsteller-Mail UND gehört jetzt zum Mitglied). `find_sent_member_ids_by_job_id` (:357, filtert `member_id IS NOT NULL`) würde diese Zeilen nach Back-fill mitzählen — das ist bei Application-Sends unkritisch, weil deren Job typischerweise ein Einzel-Versand ist; dennoch in der Planung bewusst notieren.
- Best-effort: schlägt der Back-fill fehl, ist das Mitglied erstellt, aber die Carry-over-Zeile fehlt bis zu einer Wiederholung. Recoverable (Re-Run des UPDATE), geringe Severity, konsistent mit bestehendem Post-Commit-Best-Effort.
- Historische Zeilen werden mutiert — im nicht-auditierten `mail_recipients` unbedenklich (dort wird routinemäßig `status`/`sent_at`/`rendered_*` mutiert).

### Option B — Union-at-read

**Was:** Die Member-Timeline-Query unioniert zusätzlich outbound-Zeilen, deren `application_id` zu einer Application gehört, die in dieses Mitglied konvertiert wurde.

**Code-Auswirkung:** Die Query braucht die Beziehung Application→Member. Die existiert **nicht** — `confirm()` speichert weder `application_id` auf dem Member noch `member_id` auf der Application. Also erfordert B eine gespeicherte Link-Beziehung (→ wird effektiv zu Option C) ODER einen Match über kopierte Felder (Name/Email — fragil, nicht eindeutig, abzulehnen). Zusätzlich wird der bereits nicht-triviale UNION-Query (`get_member_communications`) noch komplexer und muss bei jeder Member-Timeline-Abfrage die Application-Join-Kette mitziehen.

**Testbarkeit e2e:** Möglich, aber die Query-Komplexität erhöht das Regressionsrisiko für den bestehenden, getesteten Member-Pfad.

**Scope-Konsistenz:** ❌ Benötigt eine Stored-Link-Spalte (siehe C) → Audit-Ripple. Ohne Stored-Link nur über fragiles Feld-Matching realisierbar.

**Risiken:** Mutiert/verkompliziert den funktionierenden Member-Timeline-Leseweg; braucht trotzdem eine Persistenz-Entscheidung. Schlechtestes Aufwand/Nutzen-Verhältnis.

### Option C — Link-Spalte (bestätigte member_id auf Application, oder application_id auf Member)

**Was:** Persistiere den Link (z.B. `application.member_id` oder `member.application_id`), join beim Lesen.

**Code-Auswirkung:** Neues Feld auf einer auditierten Entität.

**Scope-Konsistenz:** ❌ **Direkter Verstoß** gegen ROADMAP.md:123 ("KEIN neues Feld auf dem auditierten `ApplicationEntity`") und gegen Pitfall 10. Bricht `test_auditable_fields_count` (`assert_eq!(fields.len(), 11)`), erzeugt Audit-Log-Zeilen für einen reinen Mail-Buchhaltungs-Link und verrauscht die verbandsrelevante Audit-Spur. `MemberEntity` ist ebenfalls auditiert — ein `application_id`-Feld dort hätte dasselbe Problem.

**Risiken:** Bricht gelockte Tests, verletzt explizite Scope-Grenze. Abzulehnen.

### Empfehlung: **Option A**

| Kriterium | A (Back-fill) | B (Union-at-read) | C (Link-Spalte) |
|-----------|:---:|:---:|:---:|
| Kein Audit-Ripple | ✅ | ⚠️ (nur ohne Stored-Link) | ❌ |
| Member-Query unverändert | ✅ | ❌ | ⚠️ (Join nötig) |
| Kein Feld auf auditierter Entität | ✅ | ⚠️ | ❌ |
| e2e einfach testbar | ✅ | ⚠️ | ✅ |
| Success-Kriterium 2 (member_id sauber) | ✅ (echte member_id) | ✅ | ✅ |
| Aufwand | niedrig | hoch | mittel |

**Begründung:** A ist die einzige Option, die alle Scope-Grenzen (kein Audit, kein Entitäts-Feld) einhält, ohne den getesteten Member-Leseweg anzufassen, und ist am einfachsten e2e-verifizierbar. Der oft zitierte Einwand gegen A ("Namespace-Poisoning", Pitfall 2) greift **nicht**, weil A die genuine neue `member_id` setzt — die Zeile gehört nach der Bestätigung tatsächlich zu einem echten Mitglied. Pitfall 2 verbietet nur, die **Application-UUID** in `member_id` zu stopfen; das tut A nicht.

**Planer darf abweichen**, sollte dann aber begründen, wie er ohne Stored-Link (C) und ohne Member-Query-Umbau (B) auskommt.

---

## Don't Hand-Roll

| Problem | Nicht selbst bauen | Stattdessen | Warum |
|---------|-------------------|-------------|-------|
| Polymorphe `(subject_type, subject_id)`-Linkage | Refactor von `mail_recipients` auf polymorphe Spalte | Geschwister-Spalte `application_id` neben `member_id` | Refactort den funktionierenden, getesteten Member-Pfad für null funktionalen Gewinn; erzwingt Daten-Migration bestehender Zeilen (Architecture Anti-Pattern 3) |
| Euro-/Betragslogik, Template-Kontext | in Phase 29 anfassen | Phase 30 | Klare Phasen-Trennung; Phase 29 ist reine Schema/DAO-Foundation |
| Atomarer Back-fill in confirm-tx | Mail-DAO mit `tx`-Param nachrüsten | Post-Commit best-effort (bestehendes Muster) | Mail-Subsystem läuft auf eigener Pool-Connection; atomare Einbindung würde die Layer-Grenze verletzen |
| Audit für Mail-Linkage | `Auditable` auf `mail_recipients` | Nichts — bleibt nicht-auditiert | Nur Member/MemberAction/MemberDocument/Application sind auditiert (CLAUDE.md); Scope-Grenze v1.6 |

---

## Runtime State Inventory

> Diese Phase ist additiv (neue nullable Spalte), kein Rename/Refactor. Kartierung dennoch, weil eine Daten-nahe Migration involviert ist.

| Kategorie | Befund | Aktion |
|-----------|--------|--------|
| Stored data | `mail_recipients` in Produktions-SQLite: bestehende Zeilen haben kein `application_id` | Keine Daten-Migration — neue Spalte liest `NULL` (NULL-Legacy-Roundtrip). Nur Schema-`ALTER`. |
| Live service config | Keine — Migration läuft automatisch beim Startup (sqlx migrate) | Keine |
| OS-registered state | Keine | Keine |
| Secrets/env vars | Keine | Keine |
| Build artifacts | `.sqlx/`-Offline-Cache betrifft nur `permission.rs`-Queries, nicht `mail_recipients` | Kein `cargo sqlx prepare` nötig (verifiziert) |

**Historische Zeilen:** Bleiben byte-identisch — der `ALTER TABLE ... ADD COLUMN` ohne `DEFAULT`/`NOT NULL` berührt bestehende Zeilen nicht; sie lesen `application_id = NULL` zurück (Success-Kriterium 4).

---

## Common Pitfalls

### Pitfall 5 (aus PITFALLS.md — in Success-Kriterium 2 referenziert) — EXAKTES ZITAT

> **Pitfall 5: No status guard — reminding `Abgelehnt` / already-`Bestaetigt` (now-member) / withdrawn applicants**
>
> **What goes wrong:** `confirm()` and `reject()` both guard `status == Offen` and return 409 otherwise (application.rs:314, 583). A naive mail endpoint has **no** such guard, so the Vorstand can fire a payment reminder at a rejected applicant (no legal basis — DSGVO problem, see Pitfall 8) or at a confirmed applicant who is now a Member and already gets member mail on a different channel (duplicate/confusing). Soft-deleted applications (`deleted.is_some()`) must also be unreachable — but `find_by_id` already filters `deleted.is_none()` (application.rs:126–136), so route the endpoint through the service `get`/`find_by_id`, not a raw dump.
>
> **Phase to address:** P-SEND

**Einordnung für Phase 29:** Pitfall 5 (Status-Guard beim Versand) ist inhaltlich eine **Phase-31**-Angelegenheit (P-SEND). Success-Kriterium 2 dieser Phase referenziert Pitfall 5 aber im Kontext der **Namespace-Sauberkeit**: Das eigentliche Foundation-Ziel ist, dass eine Application-UUID **niemals** in `RecipientInput.member_id` landet (das ist Pitfall 2, "Faking `member_id`"). Der Verweis auf "Pitfall 5" im Success-Kriterium sichert per Test/Grep-Gate ab, dass die neue `application_id`-Linkage nicht durch `member_id`-Overload umgangen wird. **→ Offene Frage 1: mutmaßlicher Referenz-Versatz Pitfall 5 vs. Pitfall 2 im Success-Kriterium.** Für Phase 29 sind BEIDE relevant: das Grep-Gate zielt auf Pitfall 2, der Status-Guard-Test ist Phase 31.

### Pitfall 2 (die eigentliche Phase-29-Kern-Falle) — Namespace-Poisoning

**Was schiefgeht:** Um die Timeline "einfach funktionieren" zu lassen, wird die Application-UUID in `member_id` gestopft. Das vergiftet den Member-Namespace: `find_sent_member_ids_by_job_id` (:357, `member_id IS NOT NULL`) liefert dann Application-IDs, und die Mail ist forensisch nicht mehr von einer echten Member-Mail unterscheidbar.
**Vermeidung:** `member_id: None` für alle Application-Sends beibehalten; separate `application_id`-Spalte nutzen. **Grep-Gate:** kein `member_id: Some(app` / kein `RecipientInput { member_id: Some(application`.
**Warnsignal:** Application-UUIDs in Member-Timelines; `find_sent_member_ids_by_job_id` liefert IDs, die nicht in `members` stehen.

### Pitfall 6 — Migration/Spaltenlisten-Vollständigkeit

**Was schiefgeht:** Die `mail_recipients`-Spaltenliste steht verbatim an ~5-6 Stellen (siehe Ist-Kartierung Abschnitt 3); wird eine übersehen, gibt es einen stillen Column-Count/Order-Mismatch oder das Feld wird nie befüllt.
**Vermeidung:** Alle `mail_recipients`-SQL-Strings greppen und in **einem Commit** ändern; **Test-DDL bei :1341 nicht vergessen**; Roundtrip-Test (INSERT mit `application_id=Some` → SELECT → Feld gleich) plus NULL-Legacy-Roundtrip (INSERT ohne → liest `None`).
**Warnsignal:** `SELECT`/`INSERT` Column-Count-Panic in Tests; neue Spalte bleibt nach `create_job` NULL.

### Pitfall 10 — Audit-Ripple auf `ApplicationEntity`

**Was schiefgeht:** Reminder-/Link-State wird als Feld auf `ApplicationEntity` getrackt → `audit_fields()` wächst, `test_auditable_fields_count` (`== 11`) bricht, Audit-Spur verrauscht.
**Vermeidung:** Carry-over/Linkage NUR auf nicht-auditierten `mail_recipients`-Zeilen (Option A). `audit_fields()` nicht anfassen.
**Warnsignal:** Neue nullable Spalte auf `application`; `test_auditable_fields_count` auf neue Zahl editiert.

### Pitfall (Phase-spezifisch) — Transaktions-Grenzen-Annahme beim Back-fill

**Was schiefgeht:** Der Back-fill wird als atomarer Teil der `confirm()`-Transaktion angenommen, obwohl die Mail-DAOs auf einer separaten Pool-Connection laufen.
**Vermeidung:** Back-fill post-commit best-effort (nach application.rs:541) implementieren, mit `tracing::warn!` bei Fehler — analog zum Datei-Cleanup (:547-555).
**Warnsignal:** Erwartung, dass ein `confirm()`-Rollback den Back-fill zurückrollt; Versuch, `MailRecipientDao` einen `tx`-Parameter aufzuzwingen.

---

## Code Examples

### Neue DAO-Methode (Muster: `get_member_communications`)

```rust
// genossi_mail/src/dao.rs — CommunicationDao trait
async fn get_application_communications(
    &self,
    application_id: Uuid,
) -> Result<Arc<[CommunicationEntry]>, MailDaoError>;

// genossi_mail/src/dao_sqlite.rs — impl (outbound-only, KEIN inbound-UNION)
// Quelle: geklont aus get_member_communications (dao_sqlite.rs:1042-1097),
// Inbound-Zweig entfernt, WHERE r.member_id → WHERE r.application_id.
```

### Back-fill (D2 Option A, post-commit best-effort)

```rust
// genossi_service_impl/src/application.rs — am Ende von confirm(), NACH commit(tx)
// Muster: Datei-Cleanup application.rs:547-555 (best-effort, warn-log).
if let Err(e) = self.mail_service
    .link_application_recipients_to_member(id /* application_id */, member_id)
    .await
{
    tracing::warn!(application_id = %id, member_id = %member_id, error = ?e,
        "Failed to carry over applicant communications to member (best-effort)");
}
```

### RecipientInput additiv

```rust
// genossi_mail/src/service.rs
pub struct RecipientInput {
    pub address: String,
    pub member_id: Option<Uuid>,
    pub application_id: Option<Uuid>,   // NEU — bestehende Literale um `application_id: None` ergänzen
}
```

---

## Testansatz

> `.planning/config.json` hat `nyquist_validation: false` — daher keine formale Validation-Architecture-Sektion. Testregeln folgen dennoch CLAUDE.md ("Always make sure you have tests for the changes") und dem bestehenden `cargo test`-Setup.

**Test-Ebenen (bestehende Infrastruktur wiederverwenden):**

1. **DAO-Roundtrip-Unit-Tests** (`genossi_mail/src/dao_sqlite.rs` Test-Modul, In-Memory-SQLite):
   - `application_id=Some(uuid)` → `create` → `find_by_job_id`/SELECT → Feld identisch zurück.
   - **NULL-Legacy-Roundtrip:** Recipient ohne `application_id` (nur `member_id`) → liest `None` zurück; bestehende `member_id`-Tests (`test_find_sent_member_ids_by_job_id` :1887, `get_member_communications`-Tests :2269-2369) bleiben grün.
   - Test-DDL bei :1341 um `application_id BLOB` erweitern.

2. **Neue Timeline-Query-Test:** `get_application_communications(app_id)` liefert outbound-Eintrag; liefert **keine** Zeilen für fremde `application_id`; respektiert `deleted IS NULL` (Recipient- und Job-Soft-Delete).

3. **Namespace-Grep-Gate (Success-Kriterium 2, gegen Pitfall 2):** Source-Invariant-Test (Muster: die `include_str!`-Grep-Gates aus Phase 26/27, z.B. `26-02-PLAN`), der sicherstellt, dass kein Produktivcode `member_id: Some(` mit einer Application-UUID koppelt bzw. dass Application-Sends `member_id: None` setzen. Alternativ ein Unit-Test, der `create_job` mit einem Application-`RecipientInput` aufruft und asserted, dass die persistierte Zeile `member_id IS NULL AND application_id = ?` hat.

4. **e2e Carry-over (Success-Kriterium 3):** In `genossi_bin/tests/e2e_tests.rs` (bestehende Confirm-e2e-Tests als Vorbild, z.B. `test_confirm_application_creates_member` :6740, `application_upload_confirm_carryover_audited` :6867):
   - Setup: Application (`Offen`) anlegen → einen `mail_recipients`-Eintrag mit `application_id` erzeugen (in Phase 29 ggf. direkt über den Mail-Service/DAO, da der Versand-Endpoint erst Phase 31 bringt) → `confirm()` → `get_member_communications(new_member_id)` enthält den Eintrag.
   - **Hinweis:** Da `POST /api/applications/{id}/mail` erst Phase 31 kommt, muss der e2e in Phase 29 die Erinnerung über den DAO/Service direkt seeden (nicht über HTTP). Der vollständige HTTP-e2e "Erinnerung senden → confirm → sichtbar" gehört zu Phase 31.

5. **Migration-Test:** additive Spalte + Index laufen sauber gegen ein bestehendes Schema (In-Memory-Migrations-Run oder e2e-Setup, das alle Migrationen anwendet).

---

## Security Domain

> `security_enforcement` ist in `.planning/config.json` nicht gesetzt (= enabled). Phase 29 ist reine DAO/Schema-Foundation ohne neue Auth-Oberfläche; die meisten Kontrollen (Admin-Gate, Status-Guard/DSGVO-Rechtsgrundlage, Content-Scoping) gehören zu **Phase 31**. Für Phase 29 relevant:

| ASVS-Kategorie | Gilt | Standard-Kontrolle |
|----------------|------|--------------------|
| V5 Input Validation | teilweise | UUID-Parsing der `application_id` via `parse_uuid`/`parse_optional_uuid` (bestehendes Muster, dao_sqlite.rs) |
| V4 Access Control | nein (Phase 31) | Endpoint-Admin-Gate kommt mit `POST/GET`-Handlern in Phase 31 |
| V6 Cryptography | nein | Keine Krypto berührt |

| Threat-Pattern | STRIDE | Mitigation |
|----------------|--------|-----------|
| Datenintegrität: falsche Subjekt-Zuordnung (Application-Mail erscheint bei falschem Mitglied) | Tampering / Information Disclosure | Back-fill setzt ausschließlich die genuine neue `member_id`; Application-UUID nie in `member_id` (Grep-Gate); Timeline-Query mit korrektem `WHERE`-Key + Soft-Delete-Filter |
| DSGVO-Datenminimierung (Antragsteller sehen nur ihre eigene Kommunikation) | Information Disclosure | Timeline outbound-only, `WHERE application_id = ?`; Inbound-Zweig bewusst weggelassen (Antragsteller haben keine `assigned_member_id`-Zuordnung) |

---

## Offene Fragen / Annahmen

1. **Pitfall-Referenz-Versatz im Success-Kriterium 2** `[ASSUMED]`: Success-Kriterium 2 nennt "Pitfall 5", inhaltlich zielt der Namespace-Schutz aber auf **Pitfall 2** (Faking `member_id`). Pitfall 5 (Status-Guard) ist eine Phase-31-Angelegenheit. Annahme: Das Grep/Test-Gate in Phase 29 zielt auf die Namespace-Invariante (Pitfall 2); der Status-Guard-Test bleibt Phase 31. **Risiko wenn falsch:** Planer baut einen fehlplatzierten Status-Guard-Test in Phase 29 gegen einen noch nicht existierenden Versand-Pfad.

2. **REST-Endpoint-Scope in Success-Kriterium 1** `[ASSUMED]`: Kriterium 1 formuliert die Verifikation über `GET /api/applications/{id}/communications`, aber der Endpoint entsteht laut ROADMAP erst in Phase 31 (Phase-29-Ziel-Note: "der REST-Endpoint kommt erst in Phase 31"). Annahme: Phase 29 verifiziert auf **DAO-Ebene** (`get_application_communications` liefert den Eintrag); der HTTP-e2e ist Phase 31. **Risiko wenn falsch:** Planer versucht in Phase 29 einen HTTP-Endpoint zu bauen, der eigentlich Phase 31 gehört (Scope-Creep).

3. **`RecipientInput`-Pflichtfeld vs. Default** `[ASSUMED]`: Empfehlung ist, `application_id: None` mechanisch an allen Literalen zu ergänzen (kein Builder-Pattern im Codebase). Falls der Planer `#[derive(Default)]` bevorzugt, muss geprüft werden, dass `RecipientInput` keine Nicht-Default-Felder blockiert (`address: String` ist Default-fähig). **Risiko:** minimal — reiner Stil.

4. **Back-fill via `MailService` vs. neue DAO-Dep** `[ASSUMED]`: Empfehlung ist eine neue `MailService`-Methode (ApplicationServiceDeps hat bereits `MailService`), um keine neue DAO-Abhängigkeit in `ApplicationServiceDeps` einzuziehen. Alternative: `CommunicationDao`/`MailRecipientDao` direkt als Dep. **Risiko:** gering — beides funktioniert; MailService hält die Layer-Grenze sauberer.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/Cargo | Build/Test | ✓ (Projekt aktiv) | 2021 edition | — |
| SQLite/SQLx | Migration + DAO | ✓ | sqlx 0.8 | — |
| sqlx-cli (`cargo sqlx prepare`) | — | nicht erforderlich für diese Migration | — | Mail-DAOs nutzen Runtime-Queries (kein compile-time Makro) |

**Keine fehlenden Dependencies.** "Add nothing" — reine Wiederverwendung.

---

## State of the Art

| Alt | Aktuell (dieser Phase) | Wann | Impact |
|-----|------------------------|------|--------|
| Nur `member_id`-Linkage auf `mail_recipients` | zusätzliche `application_id`-Geschwisterspalte | Phase 29 | Per-Antragsteller-Historie möglich, ohne Member-Namespace zu berühren |
| Member-Timeline `inbound UNION outbound` | Application-Timeline outbound-only | Phase 29 | Einfachere Query; korrekt, da Antragsteller keine Inbound-Zuordnung haben |

**Deprecated/abzulehnen:** polymorphe `(subject_type, subject_id)`-Linkage; `member_id`-Overload; Link-Feld auf auditierter Entität.

---

## Assumptions Log

| # | Claim | Section | Risiko wenn falsch |
|---|-------|---------|--------------------|
| A1 | Success-Kriterium 2 "Pitfall 5" meint effektiv Pitfall 2 (Namespace) | Pitfalls / Offene Fragen | Fehlplatzierter Status-Guard-Test in Phase 29 |
| A2 | REST-Endpoint (Success-Kriterium 1) ist Phase 31; Phase 29 verifiziert auf DAO-Ebene | Offene Fragen 2 | Scope-Creep (HTTP-Endpoint zu früh) |
| A3 | `RecipientInput`-Literale werden mechanisch um `application_id: None` ergänzt | Ist-Kartierung 4 | reiner Stil, minimal |
| A4 | Back-fill via neue `MailService`-Methode, post-commit best-effort | D2-Analyse | gering — beide Wirings funktionieren |
| A5 | D2 = Option A ist die beste Wahl | D2-Analyse | Planer kann abweichen; B/C haben dokumentierte Scope-Konflikte |

---

## Sources

### Primary (HIGH confidence — direkte Codebase-Lesung)
- `migrations/sqlite/20260403000004_create_mail_recipients_table.sql`, `..._add_message_id...`, `..._rendered_subject_body.sql`, `..._rendered_html_body.sql`, `20260411000001_add_member_communication_indexes.sql`, `20260703000000_create_application_documents_table.sql` — Schema-Muster, NULL-Legacy-Konvention, Index-Vorbild, Phase-25-Analogon
- `genossi_mail/src/dao.rs:55-80, 258-285` — `MailRecipient`, `CommunicationEntry`, `CommunicationDao`-Trait
- `genossi_mail/src/dao_sqlite.rs:207-393, 1040-1097, 1341-1361` — Row-Struct, alle SQL-Spaltenlisten, Member-Timeline-Query, Test-DDL
- `genossi_mail/src/service.rs:54-57, 356-473` — `RecipientInput`, `create_job`
- `genossi_mail/src/communication_rest.rs` — `CommunicationEntryTO` (subjekt-agnostisch), Router-Muster
- `genossi_service_impl/src/application.rs:26-40, 44-157, 288-557` — Deps, `send_confirmation_mail`, `confirm()`-Flow, Post-Commit-Best-Effort-Muster
- `genossi_dao/src/application.rs:41-93, 176` — `ApplicationEntity`, `Auditable`-Impl, gelockter Feld-Count-Test
- `genossi_bin/src/lib.rs:959-960, 973, 1956-1959` — Mail-DAO-Wiring (eigene Pool-Connection), `CommunicationRestState`
- `genossi_rest/src/lib.rs:206, 294, 647-650` — Route-Mounting, `CommunicationApiDoc`
- `genossi_bin/tests/e2e_tests.rs:6740, 6867` — bestehende Confirm-e2e-Vorbilder
- Grep-Verifikation: nur `genossi_dao_impl_sqlite/src/permission.rs` nutzt compile-time sqlx-Makros → kein `cargo sqlx prepare` für diese Migration

### Secondary (MEDIUM confidence — Milestone-Research)
- `.planning/research/ARCHITECTURE.md`, `SUMMARY.md`, `PITFALLS.md` (Pitfall 2/5/6/10 exakt), `REQUIREMENTS.md`, `ROADMAP.md`

## Metadata

**Confidence breakdown:**
- Ist-Kartierung: HIGH — jede Aussage mit Datei:Zeile verifiziert
- Migration-Muster: HIGH — direkt aus bestehenden Migrationen abgeleitet
- D2-Empfehlung: HIGH (Faktenbasis: Transaktions-Grenze + Audit-Locks verifiziert), die Wahl selbst ist eine begründete Architektur-Entscheidung
- Pitfalls: HIGH — Pitfall-5-Zitat wortgetreu aus PITFALLS.md, Referenz-Versatz explizit geflaggt

**Research date:** 2026-08-12
**Valid until:** ~2026-09-11 (stabil; interner Code, keine schnell-bewegten externen Abhängigkeiten)

## RESEARCH COMPLETE
