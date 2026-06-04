---
phase: 14-dao-domain-foundation
reviewed: 2026-06-04T08:06:53Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - genossi_bin/tests/transfer_recipients_e2e.rs
  - genossi_dao/src/repayment_entry.rs
  - genossi_dao_impl_sqlite/src/repayment_entry.rs
  - genossi_rest/src/member.rs
  - genossi_rest_types/src/lib.rs
  - genossi_service/src/member.rs
  - genossi_service_impl/src/lib.rs
  - genossi_service_impl/src/member.rs
  - genossi_service_impl/src/membership_adjust.rs
findings:
  critical: 0
  warning: 5
  info: 8
  total: 13
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-06-04T08:06:53Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Phase 14 liefert die DAO/Domain-Foundation fuer v1.2 Membership-Adjustments:
Pure-Function `compute_effective_date` (H1/H2-Stichtag), neue DAO-Methode
`find_by_member_and_phase`, sowie ein neuer Slim-Endpunkt
`GET /api/members/transfer-recipients`. Die wichtigsten Pitfalls aus der
RESEARCH-Phase (Sub-Route-Ordering, PII-Leak-Guard, PermissionDenied→401,
mockall-Default-Override, 3-Step exit_date setup) sind sauber adressiert und mit
Tests verifiziert. Audit-Hash-Chain-Konsistenz (frozen Order der Audit-Felder)
ist getestet, ebenso ISO8601/Slim-DTO/Permission-Pfade.

**Auffaellig (Warnings):**

- Permission-Filter wird _nach_ dem voll geladenen `member_dao.all()` durchgefuehrt
  (information-leakage-resistente Reihenfolge ist OK, aber lesbare-but-unnoetige
  PII-Materialisierung; siehe WR-01).
- Keine Negativ-Tests fuer `PermissionDenied` auf `list_transfer_recipients` —
  Pitfall 4 (Mapping auf 401) ist in Doc-Comments adressiert, aber nicht durch
  einen Test gegen den Service oder Endpoint verifiziert (siehe WR-02).
- Nil-UUID als `exclude_self` waere ein Sentinel-Wert, der heute matchen koennte
  (kein Member hat Nil-UUID, aber semantisch undefiniert); kein expliziter
  Validierungs-Test (siehe WR-03).
- E2E-PII-Guard prueft nur eine Whitelist gegebener Felder per `body.contains(...)`,
  was Substring-Matches mit anderen Feldnamen ermoeglichen koennte (siehe WR-04).
- `RepaymentEntryDb::share_count_to_pay_out` wird mit `i32::try_from(i64)` sauber
  geguarded, der gegenlaeufige Pfad (`entity.share_count_to_pay_out as i64`) tut
  das nicht — i32→i64 ist immer sicher, aber inkonsistente Konventionen
  schwaechen Audits (siehe WR-05).

**Strikt out of scope:** Performance (Pre-Filter via SQL, Indexes), Frontend.

## Warnings

### WR-01: list_transfer_recipients laedt _alle_ Members, dann filtert in-memory — leaks all_members durch member_dao.all()

**File:** `genossi_service_impl/src/member.rs:126-133`
**Issue:** `list_transfer_recipients` ruft `member_dao.all()` auf und filtert
danach in-memory ueber `exit_date.is_none() && id != exclude_member_id`. Auch
wenn `member_dao.all()` bereits soft-deleted-Eintraege filtert, wird der gesamte
Members-Datensatz inkl. PII (email, bank_account, street, ...) in den Service-
Layer materialisiert. Wenn der Service spaeter in einem Logger oder einer
Exception via `?` einen Member-Dump emittiert, lecken PII-Felder potenziell auf
diesem Pfad — obwohl der Endpunkt nur den Slim-DTO zurueckgibt. Pattern-
Vorlage `find_by_phase_id` in `repayment_entry.rs` zeigt, wie ein dediziertes
DAO-Method-SQL-Filter besser ist (WHERE-Klausel direkt im SQL).

Auch korrektheits-relevant: Der In-Memory-Filter ist O(N) und skaliert nicht;
fuer Phase 14 als _Foundation_ ist dies aber Out-of-Scope (Performance laut
Review-Spec nicht im Scope) und nur als Hinweis fuer spaetere Phasen.

**Fix:** Plan-naechste-Phase eine `MemberDao::find_active_excluding(exclude_id)`-
Methode hinzufuegen mit SQL-WHERE `WHERE deleted IS NULL AND exit_date IS NULL
AND id != ?`. Default-Impl filtert via `dump_all`, SQLite-Impl mit echtem SQL
(Pattern 1:1 wie `find_by_member_and_phase`). Fuer Phase 14 ist die jetzige
Loesung akzeptabel, sollte aber im 15-CONTEXT als Tech-Debt vermerkt werden.

```rust
// Phase 15 sketch:
async fn find_active_excluding(
    &self,
    exclude_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[MemberEntity]>, DaoError> {
    // SQL: WHERE deleted IS NULL AND exit_date IS NULL AND id != ?
}
```

### WR-02: Kein Negativ-Test fuer PermissionDenied auf list_transfer_recipients

**File:** `genossi_service_impl/src/member.rs:713-817` (test module)
**Issue:** Alle drei Service-Unit-Tests fuer `list_transfer_recipients`
(`happy_path_filters_self`, `all_cancelled_returns_empty`,
`only_self_returns_empty`) setzen `permission_service.expect_check_permission()
.returning(|_, _| Ok(()))`. Es existiert kein Test, der das Verhalten verifiziert,
wenn `check_permission` `ServiceError::PermissionDenied` zurueckliefert. Damit
ist (a) der dokumentierte 401-Pfad (Pitfall 4 aus RESEARCH) nur in Doc-Comments
behauptet, nicht getestet, und (b) ein Refactor, der das Permission-Gate
versehentlich entfernt (z.B. durch Reihenfolge-Vertausch), wuerde keine Tests
brechen.

Auch der `happy_path`-Test prueft via `.withf(|priv_, _ctx| priv_ == "admin")`
zwar, dass `ADMIN_PRIVILEGE` (= "admin") uebergeben wird — gut. Aber der
Fail-Pfad fehlt.

**Fix:** Negativ-Test hinzufuegen:

```rust
#[tokio::test]
async fn test_list_transfer_recipients_returns_permission_denied_for_non_admin() {
    let mut member_dao = MockTestMemberDao::new();
    member_dao
        .expect_all()
        .returning(|_| Ok(Arc::from(Vec::<MemberEntity>::new())));

    let mut permission_service = MockTestPermissionService::new();
    permission_service
        .expect_check_permission()
        .returning(|_, _| Err(ServiceError::PermissionDenied));

    let service = build_service(member_dao, permission_service);
    let result = service
        .list_transfer_recipients(Uuid::new_v4(), Authentication::Full, None)
        .await;

    assert!(matches!(result, Err(ServiceError::PermissionDenied)));
}
```

### WR-03: exclude_self = Uuid::nil() ist semantisch undefiniert und nicht validiert

**File:** `genossi_rest/src/member.rs:96-99,117-143`; `genossi_service_impl/src/member.rs:113-137`
**Issue:** `TransferRecipientsQuery.exclude_self: Uuid` und der Service-Parameter
`exclude_member_id: Uuid` haben keine explizite Validierung, dass die UUID
einem realen Member entspricht. Wenn ein Caller `Uuid::nil()` uebergibt
(00000000-0000-0000-0000-000000000000), filtert `e.id != exclude_member_id` alle
Members zurueck — denn kein Member hat eine Nil-UUID. Das ist faktisch ein
no-op-Filter und nicht das, was ein vernuenftiger Client erwartet. Auch ein
Random-UUID-Wert, der zu keinem Member korrespondiert, liefert _alle_ aktiven
Members — d.h. ein Bug im Frontend (z.B. uninitialisiertes Member-ID-State)
fuehrt zu einer fast-vollstaendigen Member-Liste. Da `MemberSlimTO` jedoch nur 6
PII-arme Felder hat, ist der Impact begrenzt — aber semantisch
befriedigt der Endpunkt nicht "die Empfaenger _ohne_ self".

Trade-off: Falls man "exclude_member_id muss existieren" enforced, wird der
Endpunkt von einer transient nicht existierenden ID abgesichert; falls nicht,
ist der jetzige Code defensiv und liefert "alle aktiven Members" zurueck —
nicht falsch, aber irrefuehrend.

**Fix:** Entweder im Service eine explizite Validierung `if exclude_member_id ==
Uuid::nil() { return Err(ValidationError(...)); }` oder im REST-Handler ein
expliziter Sanity-Check. Sicherheits-konservativ ware ein optionaler
`find_by_id`-Check, der bei "exclude_id existiert nicht" eine
ValidationError zurueckliefert — aber das verdoppelt die DAO-Last fuer einen
Edge-Case, der praktisch unwahrscheinlich ist. Mindestens einen Test
hinzufuegen, der das jetzige Verhalten dokumentiert ("Nil-UUID liefert alle
Members zurueck"), damit ein spaeterer Reviewer den Bug nicht uebersehen.

### WR-04: PII-Leak-Guard im E2E-Test ist substring-basiert und kann False-Negatives erzeugen

**File:** `genossi_bin/tests/transfer_recipients_e2e.rs:218-264`
**Issue:** Die PII-Leak-Assertions im E2E-Test pruefen via
`!body.contains("\"iban\"")` etc. — das ist robust gegen exakte Match-Faelle,
aber:

1. Falls ein zukuenftiges Feld den Substring "email" enthielte (z.B. ein
   hypothetisches `email_verified`-Feld oder ein `emails`-Array), wuerde
   `body.contains("\"email\"")` schon bei `"emails"` triggern und ein
   `"email_verified"`-Feld wiederum als _false negative_ durchrutschen lassen
   (Substring "email" matched, aber das Feld waere kein direkter PII-Leak im
   Sinne des Tests).
2. Die Whitelist ist statisch und folgt nicht der `MemberSlimTO`-Definition.
   Wenn jemand `MemberSlimTO` um ein neues Feld (z.B. `birth_date`) erweitert,
   schlaegt der Test nicht an. Der bessere Pattern ist die Iteration ueber
   die JSON-Keys gegen eine Whitelist (siehe Vorbild
   `attendance_to_tests::test_attendance_member_to_does_not_contain_pii_keys`).
3. Die `member_slim_to_tests` im `genossi_rest_types` decken die Whitelist
   bereits sauber ab via `as_object().keys()`. Der E2E-Test koennte denselben
   Pattern uebernehmen.

**Fix:** Substring-Checks ersetzen mit Key-Whitelist-Iteration:

```rust
let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse JSON");
let arr = parsed.as_array().expect("MemberSlimTO array");
let allowed: std::collections::HashSet<&str> = [
    "id", "member_number", "salutation", "title", "first_name", "last_name",
].iter().copied().collect();
for item in arr {
    let obj = item.as_object().expect("MemberSlimTO obj");
    for key in obj.keys() {
        assert!(
            allowed.contains(key.as_str()),
            "MemberSlimTO leaked unexpected field '{}'",
            key
        );
    }
}
```

### WR-05: Inkonsistente Numeric-Coercion-Konvention in repayment_entry_dao_impl

**File:** `genossi_dao_impl_sqlite/src/repayment_entry.rs:36-41,102,139`
**Issue:** Der Read-Pfad (`TryFrom<&RepaymentEntryDb> for RepaymentEntryEntity`)
guarded i64→i32 explizit mit `i32::try_from(...)` und liefert
`DaoError::ParseError` auf out-of-range. Der Write-Pfad
(`create` Z. 102 und `update` Z. 139) verwendet jedoch `as i64` ohne Guard:

```rust
let share_count = entity.share_count_to_pay_out as i64;
```

i32→i64 ist immer sicher (kein Overflow moeglich), aber die Inkonsistenz
schwaecht die "Single Source of Truth"-Konvention. Wenn jemand spaeter
`share_count_to_pay_out` auf `u32` oder `i64` aendert, wuerde `as` silent
truncieren oder negative Werte signed/unsigned-flippen. `i64::from(...)` waere
explizit, idiomatisch und nicht silent bei Typaenderungen.

Auch der Read-Pfad nutzt `format!` fuer den ParseError-String, was bei vielen
DB-Reads (z.B. 10k Eintraege) String-Alloc verursacht — Phase 14 unkritisch,
aber ein `Arc::from(format!("..."))` waere konsistenter mit dem Rest der
DaoError-Variante.

**Fix:**

```rust
// Z. 102 und Z. 139:
let share_count = i64::from(entity.share_count_to_pay_out);
```

Das ist verlustfrei, explizit, und ueberlebt jeden Typ-Refactor von i32 → i64
ohne stille semantische Verschiebung.

## Info

### IN-01: compute_effective_date Test-Liste deckt Schaltjahr- + Mid-Year-Cases ab, aber 30.06.+1ms / Year-Boundary 1.1./31.12. nur teilweise

**File:** `genossi_service_impl/src/membership_adjust.rs:50-114`
**Issue:** Die sechs Tests decken die wichtigen Stichtage ab (30.06., 01.07.,
31.12., 01.01., 29.02., 15.03.), aber:

- Kein Test fuer `year() = i32::MAX` (Overflow waere im Code keine Frage, weil
  `year() + 1` an `Date::from_calendar_date` weitergereicht und expect-panicked
  wird — siehe Z. 28). Praktisch unkritisch.
- Kein Test fuer das Verbands-konventionelle "Eingang nach Geschaeftsschluss"-
  Edge-Case (z.B. wenn Mitteilung am 30.06. 23:59:59 eingeht — aber
  `compute_effective_date` nimmt nur `Date`, nicht `DateTime`, also korrekt
  by-design).

**Fix:** Test fuer "31.12.2026 → fiscal_year=2027, effective_date=31.12.2027"
ist bereits drin. Optional einen Property-basierten Test (z.B. via `proptest`)
fuer "fuer jeden Tag im Jahr, fiscal_year == year() XOR year()+1": gerne, aber
nicht blocking.

### IN-02: `pub(crate)` Sichtbarkeit von compute_effective_date kollidiert mit moeglicher Cross-Crate-Verwendung

**File:** `genossi_service_impl/src/membership_adjust.rs:21,40`
**Issue:** `compute_effective_date` und `EffectiveDate` sind `pub(crate)`. Das
ist gut fuer Phase 14 (keine externe Sichtbarkeit fuer eine reine Pure-Function),
aber falls Phase 15+ einen `MembershipAdjustService` einfuehrt, der diese
Funktion uebersetzt fuer den REST-Layer, muss die Sichtbarkeit auf `pub`
eskalieren. Dann verschwindet der Schutz "nur intern verwendet". Vermerken in
der Phase-15-Planning, dass die `pub(crate)`-Grenze bewusst beibehalten werden
soll und der Service den Pure-Function-Output kapselt.

**Fix:** Keine Aktion in Phase 14. Im Phase-15-PLAN.md vermerken: "Pure-
Function bleibt `pub(crate)`; nur der `MembershipAdjustService`-Trait
delegiert/exposed sie via Domain-Methoden."

### IN-03: PostalCode + Bank-Account werden im sample_member-Helper im E2E-Test belegt — koennte besser stories teilen

**File:** `genossi_bin/tests/transfer_recipients_e2e.rs:52-80`
**Issue:** `sample_member(...)` setzt `bank_account: Some("DE89...")` und
`street: Some("Musterstraße")` etc. Das ist gut, weil der PII-Leak-Guard im
selben Test diese Felder absent-pruefen kann (sonst waeren sie auch im echten
Backend `None`). Aber die Werte sind hart-coded und mehrfach in der File
dupliziert (wenn weitere Tests dazukommen). Ein dedizierter `pii_sample_member()`-
Helper, der explizit "alle PII-Felder belegt" dokumentiert, waere klarer.

**Fix:** Refactor optional — fuer Phase 14 nicht blocking. Falls weitere Tests
dazukommen, einen `with_max_pii(MemberTO) -> MemberTO`-Builder einfuehren.

### IN-04: TransferRecipientsQuery hat keine Default-Impl + kein opt-out fuer exclude_self

**File:** `genossi_rest/src/member.rs:95-99`
**Issue:** `exclude_self: Uuid` ist Pflicht. Wenn ein Client kein
`?exclude_self=` mitgibt, liefert axum 400 BadRequest (`Query<...>` schlaegt
fehl). Der OpenAPI-Schema-Eintrag listet 400 als moeglichen Status — gut. Aber:

- Es waere semantisch sauberer, `exclude_self` als `Option<Uuid>` zu modellieren
  und im Service einen `None`-Fall zu erlauben ("alle aktiven Members ohne Self-
  Filter"). Dann wuerde der Endpunkt _ohne_ exclude_self-Query auch funktionieren
  — heute schlaegt er hart fehl.
- Die Doc-Comment behauptet "wird aus der Empfaenger-Liste ausgefiltert" — gut.

**Fix:** Optional — falls das Frontend `exclude_self` immer mitgibt, bleibt
Pflicht-Feld. Falls nicht, `Option<Uuid>` modellieren und im Service ein
`Option::Some(id) => filter` Branch.

### IN-05: Hardcoded admin-String in expect_check_permission().withf-Assertion

**File:** `genossi_service_impl/src/member.rs:736-738`
**Issue:** Der `happy_path`-Test guarded das Privilege via Substring:

```rust
.withf(|priv_, _ctx| priv_ == "admin")
```

Das ist OK fuer Phase 14 — aber wenn `ADMIN_PRIVILEGE` in
`genossi_service/src/permission.rs` umbenannt wird (z.B. "vorstand"), bleibt
der Test gruen und behauptet weiterhin "admin-Gate aktiv". Bessere Praxis:
Den Konstanten-Import verwenden:

```rust
use genossi_service::permission::ADMIN_PRIVILEGE;
.withf(|priv_, _ctx| priv_ == ADMIN_PRIVILEGE)
```

**Fix:** String-Literal durch Konstante ersetzen.

### IN-06: D-14-08 referenziert "SQL-Override fuer Skalierung" — Default-Impl bleibt aktiv im automock

**File:** `genossi_dao_impl_sqlite/src/repayment_entry.rs:191-217` und
`genossi_dao/src/repayment_entry.rs:159-175`
**Issue:** Die Default-Impl von `find_by_member_and_phase` filtert in-memory
ueber `dump_all`. Die SQLite-Impl ueberschreibt mit SQL-WHERE. Beides ist gut.
Aber: Das automock-Mock (`MockRepaymentEntryDao`) ueberschreibt alle Methoden
inklusive der Default-Impl. Tests, die die Default-Impl verifizieren, muessen
explizit `expect_find_by_member_and_phase()` setzen ODER eine hand-gerollte
DAO-Implementierung verwenden (wie `TestRepaymentEntryDao` Z. 324-356 im
`tests`-Submodul). Das Pattern ist sauber dokumentiert in Z. 156-163. Sehr gut.

**Fix:** Keine. Anmerkung als Hinweis fuer Phase 15+ — der Pattern muss erhalten
bleiben.

### IN-07: MemberSlimTO::From<&Member> ist explizit per Doc-Comment "EXKLUSIV"; kein Compile-Time-Lock

**File:** `genossi_rest_types/src/lib.rs:336-376`
**Issue:** Die Doc-Comment auf `MemberSlimTO` (Z. 337-347) instruiert, _nie_
einen `impl From<&MemberTO> for MemberSlimTO` zu schreiben. Das ist eine
"Code-Review-Regel" — kein Compile-Time-Check. Ein zukuenftiger Contributor
koennte versehentlich genau das tun, wenn er den Doc-Comment uebersieht.

Lautlich vorhandene Tests (`test_member_slim_to_serializes_no_pii_fields`,
`test_member_slim_to_serializes_exactly_six_keys_when_all_present`) schlagen
nur an, wenn das neue Feld in der Whitelist nicht erlaubt ist; ein Field das
durch `MemberTO`-Import gleitet, wuerde nur dann gesperrt werden.

**Fix:** Optional — ein zusaetzlicher `compile_fail`-Test wird komplex und ist
Out-of-Scope. Mindestens den Doc-Comment im PR-Template referenzieren, damit
zukuenftige Reviews darauf achten.

### IN-08: E2E-Test verlaesst sich auf `#[cfg(feature = "mock_auth")]` und enthaelt KEINEN echten Permission-Test

**File:** `genossi_bin/tests/transfer_recipients_e2e.rs:1`
**Issue:** Der E2E-Test laeuft nur unter `mock_auth`. Das ist Standard fuer
E2E-Tests, ueberprueft aber nicht, dass das echte (OIDC-)Permission-Gate
`ADMIN_PRIVILEGE` enforced. Pitfall 4 aus RESEARCH (PermissionDenied → 401) ist
in Doc-Comments des REST-Handlers behauptet, aber weder in den Service-Unit-
Tests (siehe WR-02) noch in den E2E-Tests durch einen echten 401-Pfad
verifiziert.

**Fix:** Im Phase-15+ einen E2E-Test gegen einen non-admin Mock-User
hinzufuegen, der gegen das geschuetzte Endpoint einen 401 erwartet. Fuer Phase 14
nicht blocking, weil die Foundation-Layer ist und der Permission-Test (sowie
das Mapping PermissionDenied → 401) schon in `lib.rs:115` als Cross-Cutting-
Pattern existieren.

---

_Reviewed: 2026-06-04T08:06:53Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
