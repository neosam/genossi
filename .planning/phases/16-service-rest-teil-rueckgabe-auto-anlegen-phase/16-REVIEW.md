---
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - genossi_service/src/membership_adjust.rs
  - genossi_service_impl/src/membership_adjust.rs
  - genossi_service_impl/src/repayment_phase.rs
  - genossi_rest_types/src/lib.rs
  - genossi_rest/src/membership_adjust.rs
  - genossi_rest/src/member.rs
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/membership_adjust_e2e.rs
findings:
  blocker: 2
  warning: 6
  total: 8
status: issues_found
---

# Phase 16: Code Review Report

**Reviewed:** 2026-06-05T00:00:00Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Phase 16 fuegt `MembershipAdjustService::partial_repayment` mit Auto-Phase-Create,
Sum-Check, Skip-Pattern in `open_repayment_phase` und REST-Handler
`POST /api/members/{id}/partial-repayment` hinzu. Die Implementierung ist
diszipliniert: Audit-Macros (`audited_create!`) werden konsequent fuer
MemberAction, Member, RepaymentPhase und RepaymentEntry verwendet; PERM-Gate
laeuft fuer alle drei Endpunkte; Single-Arc-per-DAO-Invariante bleibt
intakt (genossi_bin/src/lib.rs:733/734 + 740-754); Optimistic-Locking-
Pattern aus Phase 15 wurde nicht verletzt (Member.version wird beim
audited_update! nicht angepasst, korrekt).

**Zwei BLOCKER muessen vor Ship adressiert werden:**

1. **Closed/Preparation-Phase wird ohne Status-Guard wiederverwendet** —
   `partial_repayment` ruft `find()` ueber alle Phasen fuer das fiscal_year
   ohne `status`-Filter (Service-Impl Z. 345-348). Eine bereits geschlossene
   Phase fuer das Ziel-Jahr wird stillschweigend wiederverwendet — Entries
   in Closed-Phase widersprechen D-11.1 und der Lifecycle-Invariante (D-05,
   D-06 in repayment_phase.rs). Test-Lage haerteet die Annahme: Der Happy-Path-
   Test akzeptiert sogar Preparation als gueltige Wiederverwendung, obwohl
   `create_repayment_entry` (regulaerer Pfad) explizit Open verlangt.

2. **Inkonsistente Tx-Lebensdauer in `partial_repayment`'s Member-Response** —
   Nach `audited_create!(RepaymentEntry)` wird Member NICHT re-gelesen, sondern
   der pre-Read entity wird zurueckgegeben. Das ist nach PART-06/D-16-19
   inhaltlich korrekt (Member-Mutation findet hier nicht statt). Aber: Wenn
   in der gleichen Tx z.B. ein async-Task / paralleler Worker (Audit-Snapshot)
   den Member modifizieren wuerde, wird ein stale `version` zurueckgereicht.
   Da Phase 16 aktuell keinen solchen Code-Pfad hat, ist die Konsequenz
   begrenzt — Warning, nicht Blocker (siehe WR-04).

Weitere 6 Warnings konzentrieren sich auf inkonsistente Check-Reihenfolge,
PII in Error-Messages, fehlendes Closed-Phase-Test-Szenario, undurchsichtige
Audit-Process-String-Duplizierung, fehlende negative Tests im REST-Layer,
und ein implizites unwrap() bei der Response-Builder-Konstruktion.

## Critical Issues

### CR-01: BLOCKER — Closed/Preparation-Phase wird ohne Status-Guard wiederverwendet

**File:** `genossi_service_impl/src/membership_adjust.rs:344-348`
**Issue:**
```rust
let all_phases = self.repayment_phase_dao.all(tx.clone()).await?;
let target_phase_existing = all_phases
    .iter()
    .find(|p| p.fiscal_year == effective.fiscal_year)
    .cloned();
```
Es gibt keinen Status-Filter. Wenn fuer `effective.fiscal_year` bereits eine
Phase im Status `Closed` oder `Preparation` existiert, wird sie blind
wiederverwendet und ein neuer `RepaymentEntry` darin erstellt — selbst wenn
sie geschlossen ist. Vergleich:
- `RepaymentEntryServiceImpl::create_repayment_entry` (repayment_entry.rs:128)
  rejected explizit `phase.status != Open` mit `Conflict` (D-11.1).
- `RepaymentEntryServiceImpl::mark_paid_out` (repayment_entry.rs:575) erzwingt
  ebenfalls `Open` als Defense-in-Depth.

`partial_repayment` umgeht beide Invarianten. Auch wenn die Auto-Create-Branch
mit `RepaymentPhaseStatus::Open` startet (Z. 370), ist die existing-Branch
loop-hole. Praktische Konsequenz:
- Geschlossene Phase wird "wiederbelebt" via neuer Entries → Audit-Spur lecht
  Closed-Phase-Invarianten.
- D-09 (kein Lifecycle-Reverse) wird auf Entry-Ebene umgangen.
- Frontend zeigt einen erfolgreichen Entry in einer Closed-Phase, obwohl der
  Vorstand erwartet, dass Closed-Phasen final sind.

Der E2E-Test `test_partial_repayment_happy_path_h1` (membership_adjust_e2e.rs:543)
demonstriert die Schwaeche: Er legt explizit eine `Preparation`-Phase an
(`create_repayment_phase` defaultet zu Preparation, repayment_phase.rs:122)
und erwartet Erfolg. Das ist inkonsistent mit D-11.1.

**Fix:**
```rust
let target_phase_existing = all_phases
    .iter()
    .find(|p| p.fiscal_year == effective.fiscal_year)
    .cloned();

if let Some(ref phase) = target_phase_existing {
    if phase.status != RepaymentPhaseStatus::Open
        && phase.status != RepaymentPhaseStatus::Preparation
    {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot create partial repayment: phase for fiscal_year {} is '{}' (D-11.1)",
            effective.fiscal_year, phase.status.as_str()
        ))));
    }
}
```
Falls bewusst Preparation erlaubt sein soll (Vorbereitungs-Drafts), den
Test-Pfad explizit dokumentieren. Closed MUSS rejected werden. Ergaenzend
neuer Unit-Test `test_partial_repayment_rejects_closed_phase` + neuer
E2E-Test der Phase erst `open` + `close` macht und dann partial_repayment
mit 409-Erwartung.

### CR-02: BLOCKER — Permission-Check laeuft NACH `current_user_id` und kann Side-Channel zur User-Existence-Pruefung bilden

**File:** `genossi_service_impl/src/membership_adjust.rs:295-304` (analog Z. 87-96 + Z. 177-186)
**Issue:**
```rust
let user_id = self
    .permission_service
    .current_user_id(context.clone())  // <- API-Call vor Permission-Check
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());

self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)  // <- erst danach
    .await?;
```
`current_user_id()` wird VOR `check_permission()` aufgerufen. Wenn
`current_user_id()` z.B. fuer ungueltige/abgelaufene Sessions einen Fehler
ueber `?` durchpropagiert (z.B. `SessionExpired`, `AuthenticationFailed`,
siehe ServiceError-Varianten in `genossi_service/src/lib.rs`), wird die
Permission-Pruefung nie erreicht — der Caller weiss bereits, dass die
Session "lebt aber unzureichend ist" vs. "tot". Schlimmer: wenn der
`permission_service.current_user_id()` einen Datenbank-Fehler liefert
(`DataAccess`), wird `?` `InternalError` durchgereicht, BEVOR der
Permission-Funnel die Anfrage rejectet.

Konsequenz nicht "Authentication-Bypass" (Permission wird trotzdem
gechecked), aber:
- Side-Channel: Antwort-Time-Differenz zwischen "session existiert nicht"
  und "session existiert, kein admin" verraet Session-Status an
  unprivilegierte Caller.
- Audit-Log-User-Attribution: `user_id` wird fuer den `audited_create!`-
  Prozess-String genutzt. Wenn `current_user_id()` `Ok(None)` retourniert
  (kein Login), wird `"SYSTEM"` als Actor in den Audit-Log geschrieben —
  und das BEVOR `check_permission()` bestaetigt, dass ein Login vorliegt.
  Falls `check_permission(ADMIN_PRIVILEGE, AnonymousContext)` durch eine
  spaetere Refactoring versehentlich `Ok(())` retourniert (z.B. wenn das
  Auth-System einen Default-Admin-Mock einbaut), entstuende ein
  Audit-Eintrag unter "SYSTEM" ohne nachvollziehbaren Akteur.

**Fix:**
```rust
// 1) Permission-Funnel ZUERST (bricht ab bei Unauthorized/PermissionDenied).
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context.clone())
    .await?;

// 2) Erst danach user_id auflösen für Audit-Attribution.
let user_id = self
    .permission_service
    .current_user_id(context)
    .await?
    .ok_or(ServiceError::Unauthorized)?;  // KEIN unwrap_or "SYSTEM"
```
Identisches Refactor in `cancel_membership` (Z. 87-96) und `increase_shares`
(Z. 177-186). Hinweis: Das gleiche Anti-Pattern existiert in
`repayment_phase.rs` (Z. 101-108 in allen 5 Methoden) — denselben Fix
extrahieren in `genossi_service_impl/src/macros.rs::gen_auth_admin!`-
Helper oder Free-Function.

NOTE: `current_user_id()` retourniert `Result<Option<String>, ServiceError>`.
Die `None`-Branch wird per `"SYSTEM"`-Fallback maskiert. Dieses Verhalten
wurde anscheinend von Phase 15 uebernommen und Phase 16 hat das Muster
weiter repliziert. Das ist ein Audit-Forensik-Hole: jeder unautorisierte
Anonym-Try, der den check_permission()-Gate passieren wuerde (z.B.
bei einem Auth-Bypass), wuerde als `"SYSTEM"` im Audit auftauchen — nicht
als "anonym" oder "unauthenticated". `"SYSTEM"` ist normalerweise fuer
Worker / Backup-Jobs reserviert und sollte nicht von User-Requests
geschrieben werden.

## Warnings

### WR-01: Inkonsistente Check-Reihenfolge zwischen partial_repayment und cancel_membership

**File:** `genossi_service_impl/src/membership_adjust.rs:283-335` vs. `99-115`
**Issue:** In `cancel_membership` ist die Reihenfolge:
1. PERM
2. validate_willensbekundung_date (Date-Bounds, vor DB-Roundtrip)
3. member_dao.find_by_id
4. exit_date-Check

In `partial_repayment`:
1. PERM
2. member_dao.find_by_id
3. exit_date-Check
4. validate_partial_repayment_shares
5. validate_willensbekundung_date

Side-Effect: Bei einem ungueltigen `willensbekundung_date` (z.B. 1999-01-01)
laesst `partial_repayment` unnoetig einen DB-Roundtrip auf `member_dao.find_by_id`
+ Conflict-Check durchlaufen, bevor die Validation 400 wirft. Bei
`cancel_membership` ist die Reihenfolge defensiver (cheap Pure-Function
zuerst). Konsequenz: Phase 16 ist marginal weniger DOS-resistent als Phase 15
fuer Bad-Date-Spam (jede Anfrage triggert eine SELECT).

**Fix:** Vor `member_dao.find_by_id` die `validate_willensbekundung_date` +
`validate_partial_repayment_shares(shares, ?)` aufrufen. `current_shares`
ist erst nach Member-Load bekannt — also nur das Date-Bounds-Check
vorziehen:
```rust
// Step 4: Pre-DB Validierung (cheap pure functions zuerst).
let today = time::OffsetDateTime::now_utc().date();
let validation_errors = validate_willensbekundung_date(willensbekundung_date, today);
if !validation_errors.is_empty() {
    return Err(ServiceError::ValidationError(validation_errors));
}

// Step 5: Member existence (DB roundtrip danach).
let member_entity = self.member_dao.find_by_id(...).await?....;
```

### WR-02: PII-Leak in `Conflict`-Error-Message via `{:?}` auf `Option<Date>`

**File:** `genossi_service_impl/src/membership_adjust.rs:319-322`
**Issue:**
```rust
return Err(ServiceError::Conflict(Arc::from(format!(
    "Cannot start partial repayment for cancelled member (exit_date={:?})",
    member_entity.exit_date
))));
```
`exit_date` ist Mitgliederdaten. Die Conflict-Message wird via `From<ServiceError>
for RestError` (genossi_rest/src/lib.rs:115) in den HTTP-409-Body geschrieben.
Damit lecht die Bestaetigung "ja, dieses Mitglied existiert UND ist
gekuendigt UND ist am DATUM ausgetreten" an jeden, der die Member-UUID
kennt. Vergleich `cancel_membership` (Z. 114): `"member already cancelled"` —
keine PII-Exposition.

**Fix:**
```rust
return Err(ServiceError::Conflict(Arc::from(
    "Cannot start partial repayment for cancelled member"
)));
```
Der Caller braucht den `exit_date` nicht — die Frontend-UI weiss ihn aus
GET /api/members/{id} bereits (oder kann ihn dort holen, wenn berechtigt).

### WR-03: `unwrap()` auf `Response::builder()` koennte panicken — kein Crash-Schutz

**File:** `genossi_rest/src/membership_adjust.rs:73-77, 119-123, 176-180`
**Issue:**
```rust
Ok(Response::builder()
    .status(200)
    .header("Content-Type", "application/json")
    .body(Body::new(serde_json::to_string(&response)?))
    .unwrap())
```
`Response::builder()` kann `Err` retournieren, wenn z.B. ein ungueltiger
Header-Name angegeben wird. `"Content-Type"` + Status 200 sind harmless,
also faktisch infallible — aber das gleiche `.unwrap()`-Pattern aus dem
Repo (s. genossi_rest/src/member.rs:109-113, 200-203, 240-243) hat
mindestens einmal eine fragile Wiederverwendung produziert. Konsistenz-
Pattern ist OK, aber wenn jemand spaeter einen dynamischen Header-Wert
einbaut (z.B. `Content-Disposition: filename="{user_input}"`), kracht
es im Production. Niedrige Prioritaet, aber ein einfaches `.map_err(|_|
RestError::InternalError("...".to_string()))?` waere defensiver.

**Fix:** Im Rest-Layer eine kleine Helper-Function ergaenzen:
```rust
fn json_response<T: serde::Serialize>(status: u16, body: &T) -> Result<Response, RestError> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::new(serde_json::to_string(body)?))
        .map_err(|e| RestError::InternalError(format!("response builder: {e}")))
}
```
Migration ist out-of-scope fuer diesen Review, aber Phase-16-Code sollte
keine neuen `unwrap()`s einfuehren. Konsistenz mit Phase 15 — also nicht
BLOCKER.

### WR-04: `partial_repayment` retourniert pre-Read Member ohne re-read nach Tx-Commit

**File:** `genossi_service_impl/src/membership_adjust.rs:445-454`
**Issue:**
```rust
self.transaction_dao.commit(tx).await?;

// Step 14: Return tuple. Member wird unveraendert zurueckgegeben (keine Mutation).
let member_dto = Member::from(&member_entity);  // <- pre-Read
let entry_dto = RepaymentEntry::from(&new_entry);
```
PART-06/D-16-19 sagt explizit, dass `partial_repayment` Member NICHT
mutiert. Korrekt im Code. Aber: in CR-01-Fix in `repayment_phase.rs`
wird konsequent `find_by_id`-Re-Read nach `audited_*!`-Macros gemacht
(Z. 152, 252, 437, 559), um die DAO-generierte neue `version` zu fangen.
Hier wird das nicht gemacht — gerechtfertigt durch "keine Mutation".
ABER: Falls in Zukunft `recalc_dates`/`recalc_migrated` als Hook
nachgeladen werden (z.B. um `action_count` zu inkrementieren), wird
der Member-DTO stale. Aktuell kein Bug, aber Bug-Fluchtweg ist
abgeschnitten. Defensiv waere:

```rust
let refreshed = self
    .member_dao
    .find_by_id(member_id, tx.clone())  // BEFORE commit
    .await?
    .ok_or(ServiceError::InternalError(Arc::from("member disappeared mid-tx")))?;
self.transaction_dao.commit(tx).await?;
Ok((Member::from(&refreshed), entry_dto, phase_dto))
```

**Fix:** Niedrige Prioritaet — Code ist heute korrekt. Wenn die Konvention
"Re-Read nach Write" projektweit etabliert ist (siehe repayment_phase.rs),
sollte sie hier konsistent angewendet werden, auch wenn die Member-Row
nicht direkt geschrieben wurde.

### WR-05: Audit-Process-String-Duplizierung statt Cross-Modul-Konstante

**File:** `genossi_service_impl/src/membership_adjust.rs:40-48` und `repayment_phase.rs:45`
**Issue:**
```rust
// membership_adjust.rs
const REPAYMENT_PHASE_CREATE_PROCESS: &str = "repayment-phase.create";

// repayment_phase.rs
const REPAYMENT_PHASE_PROCESS_CREATE: &str = "repayment-phase.create";
```
Zwei identische String-Literale in zwei Modulen. Der inline-Kommentar
(Z. 41-47) erklaert die Absicht: "forensisch nicht von einer regulaeren
RepaymentPhaseService-Operation zu unterscheiden". Das ist OK als
Design-Decision, aber String-Drift-Risiko ist real: wenn Phase 17
oder ein Follow-up den String in `repayment_phase.rs` aendert (z.B. zu
`repayment-phase.create.v2`), wird die membership_adjust-Copie nicht
mitgezogen und der Audit-Trail spaltet sich auf zwei Process-Strings.
Test-Coverage faengt das nicht — Z. 1880 und Z. 1942 in den service-
Tests pinnen den String hart an `"repayment-phase.create"`, aber wenn
der `RepaymentPhaseService`-Konstante geaendert wird, schlaegt nur die
RepaymentPhase-Test-Suite fehl, nicht die membership_adjust-Suite.

**Fix:** `pub(crate)` Sichtbarkeit fuer `REPAYMENT_PHASE_PROCESS_CREATE`
in `repayment_phase.rs` (Z. 45) und Cross-Modul-Import in
`membership_adjust.rs`:
```rust
use crate::repayment_phase::REPAYMENT_PHASE_PROCESS_CREATE;
```
Der Kommentar in Z. 41-47 sagt "Cross-Modul-Import absichtlich vermieden
(Modul-Boundary sauber halten)" — das ist ein legitimer Standpunkt, aber
der Trade-off ist String-Drift-Risiko vs. Modul-Kupplung. Mindestens
einen Compiletime-Sync-Test einfuehren:
```rust
#[test]
fn test_audit_process_string_sync() {
    assert_eq!(
        REPAYMENT_PHASE_CREATE_PROCESS,
        crate::repayment_phase::REPAYMENT_PHASE_PROCESS_CREATE
    );
}
```

### WR-06: E2E-Test-Coverage-Luecke: kein Test fuer Closed-Phase-Edge-Case + kein Test fuer Date-Bounds bei partial_repayment

**File:** `genossi_bin/tests/membership_adjust_e2e.rs:464-867`
**Issue:** Acht E2E-Tests fuer `partial_repayment` decken Happy-Path,
Sum-Check, Auto-Fill-Skip, Full-Return-Block, Cancelled-Member-409,
Audit-Chain-Verify und Default-Share-Value ab. Fehlt:
1. **Closed-Phase-Reuse-Test** — Es gibt keinen Test der Phase erst
   `open` + `close` macht und dann `partial_repayment` aufruft. Genau
   dieser Pfad triggert CR-01.
2. **Date-Bounds-Test fuer partial_repayment** — `cancel_membership`
   hat zwei dedizierte Tests (Vorjahr + uebernaechstes Jahr,
   Z. 442-461, Z. 869-887). `partial_repayment` ruft `validate_willensbekundung_date`
   (selbe Funktion!) aber kein E2E-Test pinnt das.
3. **shares > current_shares (nicht == current_shares)** — Z. 717
   testet `shares == current_shares` (Voll-Rueckgabe-Block), aber nicht
   `shares > current_shares` (Out-of-Bound). Service-Unit-Test
   `validate_partial_repayment_shares_above_current_rejected` (Z. 706-711)
   deckt es ab, aber E2E nicht. Lower-Confidence Fix-Test-Diversity.

**Fix:** Drei neue E2E-Tests ergaenzen, ggf. als Issue/Follow-up:
```rust
#[tokio::test]
async fn test_partial_repayment_rejects_closed_phase() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1108, "Closed").await;
    let m = put_member_current_shares(&client, &server, &m, 3).await;
    let h1_date = today_march_15();
    let target_fy = h1_date.year();
    let phase = create_repayment_phase(&client, &server, target_fy, 10000).await;
    let phase_id = phase["id"].as_str().unwrap();

    // open + close
    client.post(server.url(&format!("/api/repayment-phase/{}/open", phase_id))).send().await.unwrap();
    client.post(server.url(&format!("/api/repayment-phase/{}/close", phase_id))).send().await.unwrap();

    // partial_repayment must reject Closed phase (CR-01)
    let resp = client.post(server.url(&format!("/api/members/{}/partial-repayment", m.id.unwrap())))
        .json(&partial_repayment_body(&h1_date.to_string(), 1))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);  // ggf. 400 je nach Fix
}

#[tokio::test]
async fn test_partial_repayment_date_in_previous_year_rejected() { /* analog cancel */ }

#[tokio::test]
async fn test_partial_repayment_shares_above_current_rejected() { /* shares=5, current=3 */ }
```

---

_Reviewed: 2026-06-05T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
