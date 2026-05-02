---
phase: 01-assembly-aggregat-audit-hardening
reviewed: 2026-05-02T00:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
  - genossi_dao/src/assembly.rs
  - genossi_dao/src/assembly_member_snapshot.rs
  - genossi_dao/src/lib.rs
  - genossi_dao_impl_sqlite/src/assembly.rs
  - genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs
  - genossi_dao_impl_sqlite/src/lib.rs
  - genossi_rest/src/assembly.rs
  - genossi_rest/src/lib.rs
  - genossi_rest/src/test_server.rs
  - genossi_rest_types/Cargo.toml
  - genossi_rest_types/src/lib.rs
  - genossi_service/src/assembly.rs
  - genossi_service/src/lib.rs
  - genossi_service_impl/Cargo.toml
  - genossi_service_impl/src/assembly.rs
  - genossi_service_impl/src/lib.rs
  - migrations/sqlite/20260502000000_create_assembly_table.sql
  - migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql
findings:
  critical: 1
  warning: 9
  info: 0
  total: 10
status: issues_found
---

# Phase 01: Code Review Report — Assembly-Aggregat & Audit-Hardening

**Reviewed:** 2026-05-02
**Depth:** standard
**Files Reviewed:** 21 (Source) — 2 Migrationen, 9 Rust-Module, 1 e2e-Testfile, 2 Cargo-Manifests, sowie unterstuetzende lib.rs-Dateien
**Status:** issues_found

## Summary

Die Implementierung des Assembly-Aggregats folgt sauber den im Phasenplan vorgegebenen Patterns (DAO-Trait, Service-Layer mit `gen_service_impl!`, REST-Handler mit `error_handler`). State-Transition-Guards (Pitfall 3) und atomare Snapshot-Befuellung (Pitfall 2) sind korrekt umgesetzt; der Audit-Trail ist via `audited_create!`/`audited_update!`-Macros angeschlossen, und es gibt einen e2e-Test, der die Hash-Chain ueber den vollstaendigen Lifecycle verifiziert.

Es gibt jedoch eine **BLOCKER-Klasse Concurrency-Luecke**: Das DAO meldet Versions-Konflikte als `DaoError::ConflictError`, der bestehende `From<DaoError> for ServiceError`-Mapper degradiert das aber zu `ServiceError::DataAccess` und damit zu HTTP 500 statt 409. Im Normalfall faengt der Service-Level-Check den Konflikt ab und gibt korrekt 409 zurueck, aber bei einer Race zwischen `find_by_id` und `update` bekommt der Client unkorrekte 500er. Das Problem ist nicht Assembly-spezifisch, wird aber durch die neuen DAO-Pfade exponiert.

Daneben gibt es mehrere kleinere Funde: redundante DB-Queries in `update_assembly`/`open_assembly`, fehlender `join_date`-Filter in der Snapshot-Logik, FK-Constraint in der Migration ohne aktivierten `PRAGMA foreign_keys`, dekorative Tautologien in Tests, und Byte-statt-Zeichen-laenge in der Validation.

## Critical Issues

### CR-01: DAO-Versions-Konflikt wird zu HTTP 500 (statt 409) bei Race zwischen find_by_id und update

**File:** `genossi_dao_impl_sqlite/src/assembly.rs:200-202`, gemappt durch `genossi_service/src/lib.rs:66-73`
**Issue:** Wenn zwei gleichzeitige `update_assembly`/`open_assembly`/`close_assembly`-Calls beide den Service-Level-Versions-Check passieren (Sequenz: A liest `version=v1` -> B liest `version=v1` -> A schreibt `version=v2` -> B versucht zu schreiben mit `version=v1`), liefert die DAO `Err(DaoError::ConflictError(Arc::from("Version mismatch")))` (`assembly.rs:201`). Der globale Mapper `From<DaoError> for ServiceError` in `genossi_service/src/lib.rs:66-73` matcht jedoch nur auf `DaoError::NotFound`; alles andere — inklusive `ConflictError` — wird auf `ServiceError::DataAccess(Arc::from(format!("{:?}", e)))` abgebildet. `From<ServiceError> for RestError` (`genossi_rest/src/lib.rs:91-105`) hat keinen Match-Arm fuer `DataAccess` und faellt in den Default `RestError::InternalError(format!("{:?}", e))` -> HTTP 500.

Konsequenz: Concurrent Editing produziert 500 statt 409. Der dokumentierte Status-Code-Contract (`update_assembly`: 409 Conflict in OpenAPI-Doc, `assembly.rs:228`) wird verletzt. Auditierende werden den 500 als Bug interpretieren, obwohl es ein erwartbarer Race ist.

Im Normalfall passiert das nicht, weil der Service-Level-Check (`assembly.rs:128-131` in `genossi_service_impl`) die Versionen vorab vergleicht — aber genau dazwischen klafft das Race-Fenster. Der DAO-Check ist explizit als "letzte Verteidigung" angelegt (siehe Kommentar `dao_impl_sqlite/src/assembly.rs:166-167`).

**Fix:** Den Mapper in `genossi_service/src/lib.rs:66-73` erweitern, sodass `DaoError::ConflictError` auf `ServiceError::Conflict` gemappt wird:
```rust
impl From<genossi_dao::DaoError> for ServiceError {
    fn from(e: genossi_dao::DaoError) -> Self {
        match e {
            genossi_dao::DaoError::NotFound => ServiceError::EntityNotFound(uuid::Uuid::nil()),
            genossi_dao::DaoError::ConflictError(msg) => ServiceError::Conflict(msg),
            _ => ServiceError::DataAccess(Arc::from(format!("{:?}", e))),
        }
    }
}
```
Ein zugehoeriger Test sollte den Pfad e2e abdecken (z. B. `update_assembly` mit gestale-ter Version, der den DAO-Pfad triggert, indem der Service-Level-Check umgangen wird — oder ein Unit-Test, der `MockAssemblyDao::update` ein `Err(DaoError::ConflictError(...))` zurueckgeben laesst und prueft, dass der Service-Layer ein `ServiceError::Conflict` weiterreicht).

## Warnings

### WR-01: Tautologische Assertion in Test verschleiert Test-Intent

**File:** `genossi_rest/src/assembly.rs:463`
**Issue:** `assert!(req.version != Uuid::nil() || req.version == Uuid::nil());` ist eine konstante Wahrheit (X || !X). Die Assertion testet nichts — wahrscheinlich war ein "version is set, not nil" gemeint. Der vorausgehende Kommentar "version is mandatory by type — present here." passt zur Intention, aber die Assertion liefert keinen Wert.
**Fix:** Entweder die Assertion entfernen (der Typ erzwingt schon Nicht-Optional), oder durch eine sinnvolle Aussage ersetzen, z. B. `assert_ne!(req.version, Uuid::nil(), "version must be a real UUID, not nil")` falls der Test eine konkrete Eigenschaft pruefen soll.

### WR-02: Snapshot-Logik filtert nicht auf `join_date <= opened_date`

**File:** `genossi_service_impl/src/assembly.rs:200-210`
**Issue:** Der Snapshot-Filter uebernimmt 1:1 die `count_active`-Logik (`m.deleted.is_none() && m.status.is_normal() && exit_date.map_or(true, |d| d > opened_date)`), beruecksichtigt aber nicht `join_date`. Ein Mitglied mit zukuenftigem `join_date` (z. B. neu erfasst, Beitritt in 6 Monaten) wuerde im GV-Snapshot landen, obwohl es zum Zeitpunkt der GV noch kein Stimmrecht hat. `member_dao.count_active` hat dasselbe Verhalten — d. h. das Problem ist hier nur uebernommen, aber gerade fuer Generalversammlung-Anwesenheit ist die Semantik kritisch (Verbandskonform, siehe Projekt-Constraint zur Protokoll-Auswertung).
**Fix:** Filter ergaenzen:
```rust
.filter(|m| m.join_date <= opened_date)
```
Falls bewusst die `count_active`-Logik gespiegelt werden soll, das im Code-Kommentar dokumentieren ("Bewusste Inkonsistenz: Mitglieder mit zukuenftigem join_date werden inkludiert, identisch zu count_active"). Andernfalls Filter haerten und einen Test ergaenzen, der einen Future-Joiner explizit aus dem Snapshot ausschliesst.

### WR-03: FK-Constraint in Migration ohne `PRAGMA foreign_keys=ON` wirkungslos

**File:** `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql:6-7`
**Issue:** Die Migration definiert `FOREIGN KEY (assembly_id) REFERENCES assembly(id)` und `FOREIGN KEY (member_id) REFERENCES member(id)`, aber in der Codebase wird nirgends `PRAGMA foreign_keys=ON` gesetzt (gesucht via `grep -rn "PRAGMA\|foreign_keys" genossi_bin/src/ genossi_dao_impl_sqlite/src/`). Per SQLite-Default sind FK-Constraints inaktiv — der DAO-Test im selben PR (`assembly_member_snapshot.rs:144-147`) bestaetigt das explizit ("PRAGMA foreign_keys is off by default"). Ergo: Schema dokumentiert eine Beziehung, die zur Runtime nicht erzwungen wird. Eine `assembly_id`, die auf eine geloeschte oder nicht existente `assembly` zeigt, wird nicht abgewiesen — das ist eine Datenintegritaets-Falle, die spaeter unter Last auffaellt.
**Fix:** Entweder `PRAGMA foreign_keys=ON` zentral beim Connection-Setup im `genossi_bin`/`genossi_dao_impl_sqlite/src/transaction.rs` aktivieren (typischer Weg ueber `SqlitePoolOptions::after_connect`), oder die FK-Definition in der Migration entfernen und stattdessen einen expliziten Service-Level-Validation-Step ergaenzen, der die Existenz vor `create_batch` prueft. Die zweite Variante ist defensiver, der erste Weg konsistenter mit dem DDL.

### WR-04: Doppelte `find_by_id`-Query in `update_assembly` und `open_assembly`/`close_assembly`

**File:** `genossi_service_impl/src/assembly.rs:115-119`, `168-172`, `238-242`
**Issue:** `update_assembly`, `open_assembly` und `close_assembly` rufen jeweils `assembly_dao.find_by_id(id, tx.clone())` explizit auf, um Status- und Versions-Checks durchzufuehren. Anschliessend ruft das `audited_update!`-Macro intern erneut `find_by_id` auf, um die OLD-Entity fuer den Audit-Diff zu laden (`audit_macros.rs:47-50`). Bei `dump_all`-basierten DAOs (Standard im Projekt) bedeutet das zwei Vollabfragen pro Operation — bei wachsender `assembly`-Tabelle wird das messbar. Performance ist out-of-scope, aber der Code-Smell ist real und beeintraechtigt Konsistenz: Wenn das DAO zwischen den beiden Reads gegen den Read-Cache laeuft, koennen Diskrepanzen entstehen (im normalen Single-Tx-Pfad allerdings nicht).
**Fix:** Wenn moeglich, Macro-API erweitern, sodass eine bereits geladene OLD-Entity uebergeben werden kann (z. B. `audited_update_with_old!`). Alternativ den Service-Level-`find_by_id` weglassen und Status-/Versions-Check auf das Macro-OLD-Result aufbauen — verlangt aber Macro-Refactoring. Pragmatisch: Code-Kommentar ergaenzen, der die Doppelabfrage als bekannt markiert, damit zukuenftige Reviewer den Pfad nicht versehentlich "optimieren" und damit den Audit-Trail brechen.

### WR-05: Validation laesst Byte-Laengen statt Zeichen-Laengen pruefen

**File:** `genossi_rest/src/assembly.rs:39, 54`
**Issue:** `validate_required_field` und `validate_optional_max_len` benutzen `value.len()` (Bytes) und vergleichen gegen `max_len` (256). UTF-8 multi-byte Zeichen (deutsche Umlaute = 2 Bytes, Emoji = 4 Bytes) werden also unterschiedlich gewichtet. Ein Vereins-Heimname wie "Vereinsheim Großmünchen" (23 sichtbare Zeichen) verbraucht 24 Bytes — bei 256-Byte-Limit unkritisch, aber der Limit-Begriff "256" ist semantisch missverstaendlich. Wenn ein Anwender die Limits exakt fuer seine Datenbank-Constraints kalibriert, kann das zu unerwarteten 400er-Antworten fuehren.
**Fix:** Entweder `value.chars().count() > max_len` benutzen (zeichen-basiert), oder die Doc-Strings/Schema-Beispiele explizit auf "max 256 Bytes (UTF-8)" festlegen. Konsistenz mit `validate_required_field` aus `application.rs` pruefen — wenn dort dasselbe Pattern liegt, projektweit normieren.

### WR-06: Fehlender Test fuer `get_assembly` mit korrekter Snapshot-Count

**File:** `genossi_service_impl/src/assembly.rs:272-297`, `genossi_bin/tests/e2e_tests.rs:8348-8541`
**Issue:** Die Service-Methode `get_assembly` summiert `assembly_member_snapshot_dao.count_by_assembly_id`. Es gibt aber keinen Unit-Test (`assembly.rs` Test-Modul) und keinen e2e-Test, der nach `open_assembly` ein `get_assembly` macht und prueft, ob die Anzahl mit der erwarteten aktiven-Mitglieder-Zahl uebereinstimmt. Der existierende `test_open_assembly_from_preparation_succeeds_atomic` verifiziert nur, dass `create_batch` mit der richtigen `entities.len()` aufgerufen wird — nicht, dass das Detail-API spaeter dieselbe Zahl ausweist. Die "Verbandskonformitaet" haengt explizit am Protokoll-tauglichen Anwesenheits-Count (Projekt-Constraint).
**Fix:** Im e2e-Test `test_assembly_lifecycle_audit_chain_intact` ein paar Test-Mitglieder anlegen (via `/api/members`), dann `open_assembly` aufrufen und anschliessend `GET /api/assembly/{id}` parsen und `snapshot_member_count` gegen die Anzahl angelegter Member assertieren. Zusaetzlich Unit-Test in `genossi_service_impl/src/assembly.rs` mit gemocktem `count_by_assembly_id`-Return.

### WR-07: `assembly` Entity hat `deleted`-Feld, aber kein Code-Pfad setzt/liest es

**File:** `genossi_dao/src/assembly.rs:54`, `genossi_service_impl/src/assembly.rs` (kein `audited_delete!`-Call)
**Issue:** `AssemblyEntity` hat ein `deleted: Option<PrimitiveDateTime>`-Feld (per Konvention) und `AssemblyDao::all` filtert via `e.deleted.is_none()`. Aber: Es gibt keinen REST-Endpunkt, keine Service-Methode und keinen Code-Pfad, der `deleted` setzt. Wenn die Faehigkeit zum Soft-Delete absichtlich (Phase 1 = "kein Loeschen") nicht implementiert ist, ist das OK — nur ist die Semantik fuer Folge-Phasen unklar. Das Feld ist zudem in `audit_fields()` ausgeschlossen (Lifecycle-Feld), sodass ein zukuenftiges Soft-Delete den Audit-Trail nicht selbsttaetig dokumentieren wuerde.
**Fix:** In einem Code-Kommentar oben in `genossi_service_impl/src/assembly.rs` festhalten: "Phase 1 implementiert kein Delete; das `deleted`-Feld ist Schema-Vorbereitung. Phase 2/3 muss `audited_delete!` und einen DELETE-Endpunkt nachruesten." Alternativ Feld komplett entfernen, bis es wirklich gebraucht wird (riskanter, da Migration). Der Hinweis verhindert, dass Folge-Reviewer das Feld als "tot" entfernen.

### WR-08: `format_dt` in `audit_fields()` liefert leeren String bei Format-Fehler

**File:** `genossi_dao/src/assembly.rs:67-72`
**Issue:** `audit_fields()` benutzt `format_dt`, das bei `format()`-Fehler `unwrap_or_default()` aufruft -> leerer String. Falls `dt.assume_utc().format(&Iso8601::DEFAULT)` aus irgendeinem Grund fehlschlaegt (extreme Datumsbereiche, Format-Fehler), landen leere Strings im Audit-Log statt einer aussagekraeftigen Fehlermeldung. Der Hash-Chain bleibt formell intakt, aber die Audit-Spur ist unbrauchbar — und der Aufrufer hat keinen Hinweis darauf.
**Fix:** `format_dt` an dieser Stelle sollte loggen oder einen sentinel String wie `"<invalid datetime>"` zurueckgeben, damit Audit-Fehler in der Forensik sichtbar sind. Alternativ: `audit_fields()` koennte `Result<...>` zurueckgeben, aber das aendert die Trait-Signatur projektweit. Pragmatisch:
```rust
let format_dt = |dt: &time::PrimitiveDateTime| {
    dt.assume_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to format datetime for audit: {:?}", e);
            "<invalid datetime>".to_string()
        })
};
```

### WR-09: e2e-Datum-String benutzt PrimitiveDateTime ohne TZ-Suffix; brittle gegen Iso8601-Strict-Parser-Updates

**File:** `genossi_bin/tests/e2e_tests.rs:8363, 8470, 8504`
**Issue:** Die e2e-Tests senden `"date": "2026-06-15T18:00:00.000000000"` ohne TZ-Suffix (kein `Z`, kein Offset). Der Deserialisierer in `genossi_rest_types/src/lib.rs:38-44` benutzt `PrimitiveDateTime::parse(&s, &Iso8601::DEFAULT)`. PrimitiveDateTime ignoriert TZ-Info, akzeptiert aber bei `Iso8601::DEFAULT` durch Toleranzeinstellung den String. Der Unit-Test `test_create_assembly_request_full_json` (`genossi_rest_types/src/lib.rs:1369`) sendet hingegen MIT `Z`-Suffix. Diese Inkonsistenz macht den Tests gegen Crate-Updates (insbes. `time` 0.3.x) brittle: Wenn `Iso8601::DEFAULT` strikter wird oder der `time`-Crate seine Parsing-Tolerance aendert, faellt entweder der e2e- oder der Unit-Test, ohne dass der Code geaendert wurde.
**Fix:** e2e-Tests auf das gleiche Format wie Unit-Tests bringen (mit `Z`):
```rust
"date": "2026-06-15T18:00:00.000000000Z"
```
Damit ist die Wire-Form dokumentiert eine UTC-zeitstempelte ISO8601-Form. Falls bewusst beide Formate getestet werden sollen, einen expliziten Test "ohne TZ"-Form ergaenzen, der dokumentiert, dass das Backwards-kompatibel akzeptiert wird.

---

_Reviewed: 2026-05-02_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
