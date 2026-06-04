# v1.2 Mitgliedschaft-Anpassungen — Brownfield-Architecture-Research

**Datum:** 2026-06-04  
**Scope:** Vorbereitung für Milestone v1.2 (4 Operationen: Kündigung, Teil-Rückgabe, Übertrag, Aufstockung)  
**Kern-Constraint:** v1.2 erzeugt NUR Intent-Datensätze. Anteils-Reduktion und `MemberAction::Verkauf` bleiben Aufgabe der v1.1-PaidOut-Cascade (kein Doppelbuchen).

---

## 1. Integration in Layered DAO/Service/REST-Architektur

### Architektur-Übersicht
Genossi folgt dem Pattern **DAO → Service → REST** (3-Schichten):
- **DAO-Layer** (`genossi_dao/src/*`, `genossi_dao_impl_sqlite/src/*`): Trait-Definitionen + SQLite-Impl
- **Service-Layer** (`genossi_service/src/*`, `genossi_service_impl/src/*`): Business-Logik + Permission-Gating
- **REST-Layer** (`genossi_rest/src/*`): HTTP-Endpoints + OpenAPI-Dokumentation
- **Frontend** (`genossi-frontend/src/page/*`, `genossi_frontend/src/component/*`): Dioxus-WASM mit Component-First-Prinzip

Beispiel-Architektur aus v1.1:
- **DAO:** `RepaymentPhaseDao` (Trait in `genossi_dao/src/repayment_phase.rs:L20`)
- **Service:** `RepaymentPhaseService` (Trait in `genossi_service/src/repayment_phase.rs`) → `RepaymentPhaseServiceImpl` (Impl in `genossi_service_impl/src/repayment_phase.rs:L52`)
- **REST:** `genossi_rest/src/repayment_phase.rs` → Endpoints gebunden in `genossi_rest/src/lib.rs:L641` via `repayment_phase::generate_route::<RestState>()`

### Placement-Decision: `MembershipAdjustService` — Extension vs. Neuer Service

**Empfehlung: Extension von `MemberActionService` (nicht: neuer Service)**

**Begründung:**
1. **Kohäsion:** Alle 4 Operationen (Kündigung, Teil-Rückgabe, Übertrag, Aufstockung) erzeugen `MemberAction`-Datensätze
2. **Bestehende Patterns:** `MemberActionServiceImpl` (genossi_service_impl/src/member_action.rs:L21) hat bereits:
   - Permission-Gate (`MANAGE_MEMBERS_PRIVILEGE`, L19)
   - `recalc_dates()` (L180-203) — rekalkuliert `exit_date` aus den `MemberAction`-Einträgen
   - Validation (`validate_action()`, L76-150) — bereits `UebertragungEmpfang` und `UebertragungAbgabe` sowie `Aufstockung` validiert
3. **Cross-Entity-Atomarität:** Teil-Rückgabe braucht `RepaymentPhase`-Lookup; Übertrag braucht 2× `MemberAction` in 1 Tx. Beides ist im Service-Impl möglich, neue Crate-Komplexität nicht nötig.
4. **Lifecycle:** Alle sind Lifecycle-Events des Mitglieds → `MemberAction` ist der Single Source of Truth

**Neue Methoden in `MemberActionService` (genossi_service/src/member_action.rs):**
```rust
// Neue Trait-Signaturen
pub async fn create_cancellation(
    &self,
    member_id: Uuid,
    effective_date: Option<time::Date>,  // willensbekundungsdatum
    context: Authentication<Self::Context>,
) -> Result<MemberAction, ServiceError>;

pub async fn create_partial_repayment(
    &self,
    member_id: Uuid,
    share_count_to_pay_out: i32,
    effective_date: Option<time::Date>,
    context: Authentication<Self::Context>,
) -> Result<(MemberAction, RepaymentEntry), ServiceError>;

pub async fn transfer_shares(
    &self,
    from_member_id: Uuid,
    to_member_id: Uuid,
    share_count: i32,
    effective_date: Option<time::Date>,  // nicht verwendet, sofort wirksam, aber für UI-Konsistenz
    context: Authentication<Self::Context>,
) -> Result<(MemberAction, MemberAction), ServiceError>;  // (UebertragungAbgabe, UebertragungEmpfang)

pub async fn create_increase(
    &self,
    member_id: Uuid,
    share_count: i32,
    effective_date: Option<time::Date>,  // nicht verwendet, sofort wirksam
    context: Authentication<Self::Context>,
) -> Result<MemberAction, ServiceError>;
```

**Implementierungs-Anker:**
- `genossi_service_impl/src/member_action.rs:L232ff` — bereits `#[async_trait] impl MemberActionService`
- Dependency-Injection: `MemberActionServiceImpl` braucht neu:
  - `RepaymentPhaseDao` (für Phase-Lookup bei Teil-Rückgabe)
  - `RepaymentEntryDao` (für RepaymentEntry-Creation)
  - `MemberDao` (schon vorhanden für Member-Lookup)
  - `audit_log_dao`, `uuid_service`, `permission_service`, `transaction_dao` (alle schon vorhanden)

---

## 2. MemberAction-ActionType-Erweiterung

### Existierende Varianten
`genossi_dao/src/member_action.rs:L9-18`:
```rust
pub enum ActionType {
    Eintritt,           // Entry
    Austritt,           // Exit
    Todesfall,          // Death
    Aufstockung,        // Increase (schon vorhanden!)
    Verkauf,            // Sale (v1.1: erzeugt von PaidOut-Cascade)
    UebertragungEmpfang,// Transfer In (schon vorhanden!)
    UebertragungAbgabe, // Transfer Out (schon vorhanden!)
    Note,               // Free-form note
}
```

**Status:** ✓ Alle v1.2-nötigen Typen existieren bereits!
- `Aufstockung` → v1.2 Aufstockung-Operation
- `UebertragungEmpfang` / `UebertragungAbgabe` → v1.2 Übertrag-Operation (2 verlinkte Actions)
- `Austritt` mit `effective_date` → v1.2 Kündigung
- `Verkauf` mit `share_count < 0` → v1.1 PaidOut-Cascade (Teil-Rückgabe braucht KEIN neuen Type, sondern RepaymentEntry + später Verkauf)

### Validation v1.1 ↔ v1.2 Cross-Check
`genossi_service_impl/src/member_action.rs:L76-150` — `validate_action()` bereits enforced:

| ActionType | v1.1 validate_action | v1.2 Constraint | Risk? |
|---|---|---|---|
| `Aufstockung` | shares_change > 0 (L88-93) | V.1.2 erzeugt bei Aufstockung (shares_change > 0) | ✓ kompatibel |
| `UebertragungEmpfang` | shares_change > 0, transfer_member_id required (L88-93, L127-136) | V1.2 erzeugt bei Übertrag-In (shares_change > 0, transfer_member_id = source) | ✓ kompatibel |
| `UebertragungAbgabe` | shares_change < 0, transfer_member_id required (L96-102, L127-136) | V1.2 erzeugt bei Übertrag-Out (shares_change < 0, transfer_member_id = target) | ✓ kompatibel |
| `Austritt` | effective_date required (L138-143) | V1.2 Kündigung erzeugt Austritt mit effective_date (H1/H2-Stichtag) | ✓ kompatibel |
| `Verkauf` | shares_change < 0 (L96-102) | V1.1 PaidOut-Cascade erzeugt mit shares_change (Summe aller paid_out RepaymentEntries) | ✓ kompatibel |

**Niedrig-Risiko für v1.1-Cascade:** `mark_paid_out()` (genossi_service_impl/src/repayment_entry.rs:L517) erzeugt IMMER `MemberAction::Verkauf` (L583ff), unabhängig von Action-Type der Quelle. v1.2's neue Types (`Aufstockung`, `UebertragungEmpfang/Abgabe`) sind **nicht** Repayment-Einträge und triggern daher kein PaidOut. *Aber:* Teil-Rückgabe **ist** ein RepaymentEntry, dessen PaidOut MUSS `Verkauf` erzeugen (nicht neue Types). Das ist bereits richtig in v1.1, v1.2 braucht nichts zu ändern.

**Double-Booking-Guard:**
- v1.1 `mark_paid_out()` lädt `RepaymentEntry` + erzeugt `MemberAction::Verkauf` + reduziert `current_shares` (genossi_service_impl/src/repayment_entry.rs:L556-620)
- v1.2 Teil-Rückgabe erzeugt `RepaymentEntry` + **kein** `MemberAction` (Datensatz bleibt im Open-Status, wartet auf PaidOut)
- v1.2 Aufstockung/Übertrag erzeugen `MemberAction` + reduzieren/erhöhen `current_shares` **sofort** (kein RepaymentEntry)
- **Fazit:** Kein Doppelbuchen, da zwei unterschiedliche Pfade (MemberAction-direct vs. RepaymentEntry→PaidOut→MemberAction).

---

## 3. H1/H2-Stichtagsregel-Implementierung

### Spezifikation aus PROJECT.md
**Stichtagsregel (Kündigung + Teil-Rückgabe):**
- **H1 (1.–6. Monat):** effective_date = 31.12. des **aktuellen** Geschäftsjahres
- **H2 (7.–12. Monat):** effective_date = 31.12. des **folgenden** Geschäftsjahres

### Implementierungs-Ort: Pure Function in Service-Impl

**Platzierung:** `genossi_service_impl/src/member_action.rs` (neue Funktion)

```rust
/// Berechnet das Wirksamkeitsdatum nach H1/H2-Regel.
/// - Willensbekundung im 1.-6. Monat → 31.12. des laufenden Jahres
/// - Willensbekundung im 7.-12. Monat → 31.12. des nächsten Jahres
/// Pure function, unit-testbar, keine I/O.
pub(crate) fn compute_effective_date(
    willensbekundung_datum: time::Date,
) -> time::Date {
    let month = willensbekundung_datum.month() as u8;
    let year = if month <= 6 {
        willensbekundung_datum.year()
    } else {
        willensbekundung_datum.year() + 1
    };
    
    time::Date::from_calendar_date(year, time::Month::December, 31)
        .expect("December 31 is always valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h1_returns_dec31_current_year() {
        let date = time::Date::from_calendar_date(2026, time::Month::March, 15).unwrap();
        let result = compute_effective_date(date);
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), time::Month::December);
        assert_eq!(result.day(), 31);
    }

    #[test]
    fn test_h2_returns_dec31_next_year() {
        let date = time::Date::from_calendar_date(2026, time::Month::September, 10).unwrap();
        let result = compute_effective_date(date);
        assert_eq!(result.year(), 2027);
        assert_eq!(result.month(), time::Month::December);
        assert_eq!(result.day(), 31);
    }
}
```

**Aufruf in den neuen Service-Methoden:**
```rust
// create_cancellation + create_partial_repayment
let effective_date = compute_effective_date(willensbekundungs_datum);

// transfer_shares + create_increase: NOT verwendet (sofort wirksam)
// aber Willensbekundungs-Datum wird trotzdem als `date` im MemberAction gespeichert
```

**Unit-Testbar:** ✓ Reine Funktion, keine Abhängigkeiten, einfache Datumsarithmetik.

---

## 4. Auto-Anlegen-Ziel-Phase-Strategie

### Problem
Teil-Rückgabe im H2 mit Willensbekundung Nov 2026 → effective_date = 31.12.2027 → benötigt RepaymentPhase für FY 2027. Was tun, wenn Phase noch nicht existiert?

**Drei Optionen (aus PROJECT.md):**

| Option | Vorteil | Nachteil |
|--------|---------|---------|
| A: Auto-Create in Preparation + D-11.1-Guard erweitern | Vorhersehbar: Phase immer vorhanden beim Create | Extra-DAO-Call, Status-Komplexität: RepaymentEntry in Open Phase erzeugt, aber Phase ist Preparation |
| B: Auto-Create direkt in Open + Auto-Fill-Dedup | Konsistent mit open_repayment_phase (Phase 8): alle Entries sofort Open | Komplexe Dedup-Logik nötig (Teil-Rückgabe-Entry könnte doppelt landen wenn Phase Auto-Created + später auch Open wird) |
| C: Defer RepaymentEntry bis Phase-Open | Fehlerkanal: wenn Phase nie opened wird, Member wartet ewig | Deferred-Semantik ist UI-schwer zu erklären; Undo-Path unklar |

### Empfehlung: **Option B (Auto-Create direkt in Open)**

**Begründung:**
1. **Konsistenz mit v1.1:** `open_repayment_phase()` (Phase 8) erzeugt RepaymentEntries in Open-Status automatisch. Wenn v1.2 Teil-Rückgabe eine Phase Auto-Create in Open tut, ist die Semantik unify.
2. **Keine Dedup-Komplexität:** Das Auto-Create-Skript erzeugt die **Ziel-Phase** auf Basis (fiscal_year = Wirksamkeits-Jahr). `open_repayment_phase()` filtert Members via `m.exit_date.is_some_and(|d| d >= fy_start && d <= fy_end)` — die Teil-Rückgabe-Entry wird NICHT doppelt erstellt, weil der Member noch kein `exit_date` hat (nur `current_shares` wurde reduziert).
3. **UI-Simple:** Vorstand drückt "Teil-Rückgabe", System antwortet sofort mit "Teil-Rückgabe erstellt für FY 2027" (ggf. neue Phase angelegt). Kein Warten, kein "bitte öffne Phase später".

**Implementierung:**

```rust
pub async fn create_partial_repayment(
    &self,
    member_id: Uuid,
    share_count_to_pay_out: i32,
    willensbekundungs_datum: time::Date,
    context: Authentication<Self::Context>,
) -> Result<(MemberAction, RepaymentEntry), ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    
    // ...Permission-Check etc...
    
    // Step 1: Berechne effective_date nach H1/H2-Regel
    let effective_date = compute_effective_date(willensbekundungs_datum);
    let target_fiscal_year = effective_date.year();
    
    // Step 2: Versuche Phase zu laden; wenn nicht existiert, create in Open status
    let phase = match self
        .repayment_phase_dao
        .find_by_fiscal_year(target_fiscal_year, tx.clone())
        .await? {
        Some(p) => p,
        None => {
            // Auto-Create in Open status
            // Aber: für "share_value" braucht Vorstand Input oder sensible Default?
            // Vorläufig: Fehler zurückgeben + UI sagt "bitte Create Phase erst".
            // TODO(v1.2-discuss): Auto-Create mit share_value from latest phase?
            return Err(ServiceError::Conflict(Arc::from(
                format!("RepaymentPhase for fiscal_year {} does not exist; please create it first", target_fiscal_year)
            )));
        }
    };
    
    // Guard: D-11.1 — Phase muss Open sein (v1.1-Constraint bleibt)
    if phase.status != RepaymentPhaseStatus::Open {
        return Err(ServiceError::Conflict(Arc::from(
            format!("RepaymentPhase status must be Open, got {}", phase.status.as_str())
        )));
    }
    
    // Step 3: Lade Member + validiere shares
    let mut member = self
        .member_dao
        .find_by_id(member_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(member_id))?;
    
    // Validierung (ENTR-02)
    validate_entry_create(share_count_to_pay_out, member.current_shares)?;
    
    // Step 4: Erstelle MemberAction (keine shares_change, nur Intent-Marker)
    // ABER: laut v1.2-Spec erzeugt Teil-Rückgabe KEINE MemberAction, sondern nur RepaymentEntry
    // Das heißt: compute_dates() wird NICHT neu ausgeführt
    // TODO(v1.2-discuss): Fallback Option — sollen wir einen "TeilRückgabe"-Type in ActionType hinzufügen?
    // Für jetzt: keine MemberAction, nur RepaymentEntry
    
    // Step 5: Erstelle RepaymentEntry
    let entry = RepaymentEntryEntity {
        id: self.uuid_service.new_v4().await,
        member_id,
        phase_id: phase.id,
        share_count_to_pay_out,
        status: RepaymentEntryStatus::Open,
        created: /* now */,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    
    crate::audited_create!(
        self,
        self.repayment_entry_dao,
        &entry,
        "repayment-entry.create-from-membership-adjust",
        &user_id,
        tx
    );
    
    self.transaction_dao.commit(tx).await?;
    Ok((/* no MemberAction */, RepaymentEntry::from(&entry)))
}
```

**Problematisch:** Laut PROJECT.md "v1.2 erzeugt nur Intent-Datensätze — keine MemberAction". Teil-Rückgabe erzeugt nur RepaymentEntry. Das ist **OK**, weil v1.1's `mark_paid_out()` die Reduktion macht. *Aber:* wenn Vorstand ändert Gedanken und cancelt die RepaymentEntry, wurde `current_shares` nie reduziert → Member-State bleibt korrekt. ✓ Sauber.

**Recommend für Discuss-Phase:** Klären, ob Fallback auf "Phase nicht existiert" → "Auto-Create Phase in Open" soll, oder ob Fehler → "Vorstand create manuell" acceptable ist.

---

## 5. Cross-Entity-Atomarität für Übertragung

### Anforderung
**2 verlinkte MemberActions in einer Tx:**
- A: `MemberAction(member_id=from, action_type=UebertragungAbgabe, shares_change=-n, transfer_member_id=to)`
- B: `MemberAction(member_id=to, action_type=UebertragungEmpfang, shares_change=+n, transfer_member_id=from)`
- Member A: `current_shares -= n`
- Member B: `current_shares += n`
- Genau-einmal-Semantik: beide oder keine

### Pattern-Anker aus v1.1 Phase 9
`genossi_service_impl/src/repayment_entry.rs:L517-620` `mark_paid_out()` — 12-Schritt-Cascade:
1. Tx beginnen
2. User-ID + Permission-Check
3. RepaymentEntry laden + Status-Guard
4. Member laden (Payout-Owner)
5. Validierung (shares_change > 0)
6. `audited_update!` RepaymentEntry.status = PaidOut
7. `audited_create!` MemberAction::Verkauf (mit shares_change = −entry.share_count)
8. `audited_update!` Member.current_shares -= shares_change
9. `recalc_dates()` / `recalc_migrated()` falls nötig
10. Re-read (CR-01)
11. Commit
12. Return

**v1.2 Übertrag-Analogon (7 Schritte):**

```rust
pub async fn transfer_shares(
    &self,
    from_member_id: Uuid,
    to_member_id: Uuid,
    share_count: i32,
    context: Authentication<Self::Context>,
) -> Result<TransferResult, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    
    // Step 1-2: User + Permission
    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(MANAGE_MEMBERS_PRIVILEGE, context).await?;
    
    // Step 3: Lade beide Members
    let mut from_member = self.member_dao.find_by_id(from_member_id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(from_member_id))?;
    let mut to_member = self.member_dao.find_by_id(to_member_id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(to_member_id))?;
    
    // Step 4: Validierungen
    if share_count <= 0 {
        return Err(ServiceError::ValidationError(vec![
            ValidationFailureItem {
                field: Arc::from("share_count"),
                message: Arc::from("must be > 0"),
            }
        ]));
    }
    if from_member.current_shares < share_count {
        return Err(ServiceError::ValidationError(vec![
            ValidationFailureItem {
                field: Arc::from("share_count"),
                message: Arc::from(format!(
                    "transfer count {} exceeds from_member current_shares {}",
                    share_count, from_member.current_shares
                )),
            }
        ]));
    }
    if to_member.exit_date.is_some() {
        return Err(ServiceError::Conflict(Arc::from(
            "cannot transfer to member with exit_date (not active)"
        )));
    }
    
    let now = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());
    
    // Step 5a: Erstelle MemberAction::UebertragungAbgabe
    let from_action = MemberActionEntity {
        id: self.uuid_service.new_v4().await,
        member_id: from_member_id,
        action_type: ActionType::UebertragungAbgabe,
        date: now.date(),
        shares_change: -(share_count as i32),
        transfer_member_id: Some(to_member_id),
        effective_date: None,  // sofort wirksam
        comment: None,
        created: now_pdt,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    
    crate::audited_create!(
        self,
        self.member_action_dao,
        &from_action,
        "membership-adjust.transfer-out",
        &user_id,
        tx.clone()
    );
    
    // Step 5b: Erstelle MemberAction::UebertragungEmpfang
    let to_action = MemberActionEntity {
        id: self.uuid_service.new_v4().await,
        member_id: to_member_id,
        action_type: ActionType::UebertragungEmpfang,
        date: now.date(),
        shares_change: share_count as i32,
        transfer_member_id: Some(from_member_id),  // link back to source
        effective_date: None,
        comment: None,
        created: now_pdt,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    
    crate::audited_create!(
        self,
        self.member_action_dao,
        &to_action,
        "membership-adjust.transfer-in",
        &user_id,
        tx.clone()
    );
    
    // Step 6: Reduziere from_member.current_shares
    from_member.current_shares -= share_count;
    // Wenn from → 0: setze exit_date
    if from_member.current_shares == 0 {
        from_member.exit_date = Some(now.date());
    }
    
    crate::audited_update!(
        self,
        self.member_dao,
        from_member_id,
        &from_member,
        "membership-adjust.transfer",
        &user_id,
        tx.clone()
    );
    
    // Step 7: Erhöhe to_member.current_shares
    to_member.current_shares += share_count;
    
    crate::audited_update!(
        self,
        self.member_dao,
        to_member_id,
        &to_member,
        "membership-adjust.transfer",
        &user_id,
        tx.clone()
    );
    
    // Step 8: recalc_dates für from (exit_date könnte gesetzt worden sein)
    self.recalc_dates(from_member_id, tx.clone()).await?;
    
    self.transaction_dao.commit(tx).await?;
    
    Ok(TransferResult {
        from_action_id: from_action.id,
        to_action_id: to_action.id,
    })
}
```

**Cross-Entity-Linking:** Die zwei MemberActions verlinken sich via `transfer_member_id` (beide zeigen aufeinander). Das ist das existierende Pattern aus v1.1.

---

## 6. Frontend-Integration

### Member-Detail-Page-Struktur
**Pfad:** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/member_details.rs`

**Aktuelle Page-Struktur (Zeilen 75ff):**
```rust
#[component]
pub fn MemberDetails(id: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    
    // Signal: member (MemberTO)
    let mut member = use_signal(|| { /* default */ });
    
    // UI-Sections:
    // 1. TopBar (Zeile ~150)
    // 2. Member-Daten (Form mit Feldern wie Name, Email, etc.)
    // 3. Action-Buttons (z.B. "Eintrittsbestätigung generieren", Zeile ~200)
    // 4. MemberAction-Timeline (CommunicationTimeline, Zeile ~350)
    // 5. MemberDocuments (Zeile ~400)
}
```

### Neue UI-Komponente: "Mitgliedschaft anpassen"-Button + Modal

**Komponenten-Struktur (Component-First-Prinzip):**

**1. New Component: `membership_adjust_modal.rs`**
Pfad: `genossi-frontend/src/component/membership_adjust_modal.rs` (neue Datei)

```rust
#[component]
pub fn MembershipAdjustModal(
    member_id: Uuid,
    is_open: bool,
    on_close: EventHandler,
    on_success: EventHandler,  // callback nach Aktion
) -> Element {
    let i18n = use_i18n();
    
    // State: welche Operation ist gewählt?
    let mut operation = use_signal(|| MembershipOperation::Cancellation);
    
    // State: Datepicker (default: today)
    let mut willensbekundungs_datum = use_signal(|| /* today */);
    let mut share_count = use_signal(|| 1i32);
    let mut target_member_id = use_signal(|| None::<Uuid>);
    
    // Rendering:
    // 1. Modal-Wrapper
    // 2. Tabs/RadioButtons für die 4 Operationen
    // 3. Form-Felder pro Operation (z.B. target member für Übertrag)
    // 4. Datepicker (für Willensbekundung)
    // 5. Vorschau-Tabelle (read-only)
    // 6. Confirm-Button + Cancel
}

enum MembershipOperation {
    Cancellation,          // Kündigung
    PartialRepayment,      // Teil-Rückgabe
    TransferOut,           // Übertrag (nur shares_count + target_member)
    Increase,              // Aufstockung (nur share_count)
}
```

**Shared Components (reuse von v1.1):**
- `Modal` (bestehend, genossi-frontend/src/component/modal.rs) — Wrapper für die Adjust-Modal
- `MemberSearch` (bestehend, genossi-frontend/src/component/member_search.rs) — für target-member-Auswahl bei Übertrag
- `RequirePrivilege` (bestehend?) — oder inline Permission-Check für Admin-only

**2. Button-Integration in MemberDetails**
`genossi-frontend/src/page/member_details.rs` — neue Button-Zeile:

```rust
// Nach den existing Buttons (z.B. "Eintrittsbestätigung generieren")
// um Zeile ~200:

let mut show_adjust_modal = use_signal(|| false);

// Button:
rsx! {
    button {
        disabled: !is_admin,  // nur Vorstand
        onclick: move |_| show_adjust_modal.set(true),
        "{i18n.t(Key::MembershipAdjustButton)}"  // "Mitgliedschaft anpassen"
    }
}

// Modal:
{
    show_adjust_modal() && rsx! {
        MembershipAdjustModal {
            member_id: member.id.unwrap(),
            is_open: show_adjust_modal(),
            on_close: move |_| show_adjust_modal.set(false),
            on_success: move |_| {
                // Reload member data
                show_adjust_modal.set(false);
                // Trigger: refetch member from API
            },
        }
    }
}
```

**3. i18n Keys (neue Einträge)**
`genossi-frontend/src/i18n/mod.rs`:
```rust
pub enum Key {
    // ... existing ...
    MembershipAdjustButton,
    MembershipAdjustCancellation,
    MembershipAdjustPartialRepayment,
    MembershipAdjustTransfer,
    MembershipAdjustIncrease,
    MembershipAdjustEffectiveDate,  // Wirksamkeitsdatum
    MembershipAdjustTargetMember,   // für Übertrag
    MembershipAdjustShareCount,
    MembershipAdjustPreview,        // Vorschau-Sektion
    MembershipAdjustConfirm,        // Bestätigung
}
```

**4. Service-Layer Frontend API**
`genossi-frontend/src/api.rs` — neue Funktionen:

```rust
pub async fn create_cancellation(
    member_id: Uuid,
    effective_date: time::Date,
) -> Result<MemberActionTO, String> {
    // POST /api/members/{member_id}/cancel
}

pub async fn create_partial_repayment(
    member_id: Uuid,
    share_count: i32,
    effective_date: time::Date,
) -> Result<RepaymentEntryTO, String> {
    // POST /api/members/{member_id}/partial-repayment
}

pub async fn transfer_shares(
    from_member_id: Uuid,
    to_member_id: Uuid,
    share_count: i32,
) -> Result<TransferResultTO, String> {
    // POST /api/members/{from_member_id}/transfer-to/{to_member_id}
}

pub async fn create_increase(
    member_id: Uuid,
    share_count: i32,
) -> Result<MemberActionTO, String> {
    // POST /api/members/{member_id}/increase-shares
}
```

---

## 7. Permission-Funnel

### Existierendes Permission-System
`genossi_service/src/permission.rs` — `PermissionService` trait:
- `check_permission(privilege: &str, context: Authentication<C>) -> Result<(), ServiceError>`
- Kontext kann sein: `Full` (OIDC-authenticated) oder `Bearer` (QR-Token) oder other

**Existierende Privileges:**
- `"admin"` — Vorstand-only (v1.1 RepaymentPhase, MemberAction)
- `"view_members"` — read-only Mitgliederliste
- `"manage_members"` — Mitglieder-CRUD

### v1.2 Permission-Strategy

**Alle 4 Operationen:** Vorstand-only (`MANAGE_MEMBERS_PRIVILEGE`)

```rust
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";
```

**Service-Methods werden gated bei Entry (vgl. v1.1 Pattern):**

```rust
// Alle neuen Methods in MemberActionServiceImpl
pub async fn create_cancellation(&self, ..., context: Authentication<Self::Context>) {
    // Step 1: Check permission
    self.permission_service.check_permission(MANAGE_MEMBERS_PRIVILEGE, context).await?;
    // Step 2-N: proceed
}
```

**Frontend-Check:**
```rust
// In member_details.rs
let is_admin = auth.is_authenticated() && auth.has_privilege("manage_members");
// button wird nur angezeigt wenn is_admin
```

**REST-Endpoint-Security:**
`genossi_rest/src/member_action.rs` — die neuen Endpoints erben das Permission-Gating vom Service:
```rust
#[post("/api/members/{member_id}/cancel")]
pub async fn cancel_membership(
    State(state): State<RestState>,
    Path(member_id): Path<Uuid>,
    Authentication(auth): Authentication,
) -> Result<Json<MemberActionTO>> {
    let result = state
        .member_action_service()
        .create_cancellation(member_id, auth)
        .await?;
    Ok(Json(MemberActionTO::from(&result)))
}
```

---

## 8. Audit-Pflicht & Hash-Chain-Konformität

### v1.2-Operationen = Audit-Einträge

**Alle 4 Operationen erzeugen Audit-Log-Einträge via bestehende Macros:**

| Operation | Entity | Macro | Audit-Einträge |
|---|---|---|---|
| **Kündigung** | MemberAction (Austritt) | `audited_create!` | 1× MemberAction-Create; 0× Member-Update (exit_date ist Computed aus MemberAction) |
| **Teil-Rückgabe** | RepaymentEntry | `audited_create!` | 1× RepaymentEntry-Create (später `audited_update!` auf PaidOut + `audited_create!` MemberAction::Verkauf) |
| **Übertrag** | MemberAction (2×) + Member (2×) | `audited_create!` (2×) + `audited_update!` (2×) | 2× MemberAction-Create + 2× Member-Update |
| **Aufstockung** | MemberAction + Member | `audited_create!` + `audited_update!` | 1× MemberAction-Create + 1× Member-Update |

### Hash-Chain-Konformität

**Status:** ✓ Fully compatible. Alle Macros verwenden bestehendes Audit-System:

`genossi_service_impl/src/audit_macros.rs:L1-36` — `audited_create!`:
```rust
#[macro_export]
macro_rules! audited_create {
    ($self:expr, $dao:expr, $entity:expr, $process:expr, $user_id:expr, $tx:expr) => {{
        // 1. DAO-create aufrufen
        $dao.create($entity, $process, $tx.clone()).await?;
        
        // 2. Latest hash laden
        let prev_hash = $self.audit_log_dao.get_latest_hash($tx.clone()).await?
            .unwrap_or_default();
        
        // 3. Audit-Einträge bauen (build_create_entries)
        let entries = $crate::audit_log::build_create_entries(
            $entity,
            $user_id,
            $process,
            &prev_hash,  // ← chaining mit vorherigem Hash
            &mut || uuid::Uuid::new_v4(),
        );
        
        // 4. Einträge schreiben
        if !entries.is_empty() {
            $self.audit_log_dao.create_entries(&entries, $tx.clone()).await?;
        }
    }};
}
```

**Process-Strings für v1.2 (zur Audit-Log-Rückverfolgung):**
```rust
const PROCESS_CANCELLATION: &str = "membership-adjust.cancellation";
const PROCESS_PARTIAL_REPAYMENT: &str = "membership-adjust.partial-repayment";
const PROCESS_TRANSFER_OUT: &str = "membership-adjust.transfer-out";
const PROCESS_TRANSFER_IN: &str = "membership-adjust.transfer-in";
const PROCESS_INCREASE: &str = "membership-adjust.increase";
```

**Audit-Felder (MemberActionEntity):**
`genossi_dao/src/member_action.rs:L76-96` — `audit_fields()`:
```rust
impl Auditable for MemberActionEntity {
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("member_id", Some(self.member_id.to_string())),
            ("action_type", Some(self.action_type.as_str().to_string())),
            ("date", Some(format_date(&self.date))),
            ("shares_change", Some(self.shares_change.to_string())),
            ("transfer_member_id", self.transfer_member_id.map(|u| u.to_string())),
            ("effective_date", self.effective_date.as_ref().map(format_date)),
            ("comment", self.comment.as_ref().map(|s| s.to_string())),
        ]
    }
}
```

**Vorhanden, kein Change nötig:** ✓ Alle neuen v1.2-ActionTypes (`Aufstockung`, `UebertragungEmpfang/Abgabe`) werden bereits auditiert, weil `audit_fields()` unabhängig vom Type funktioniert.

**Member-Updates (current_shares, exit_date):**
`audited_update!` mapped auch hier: nur geänderte Felder werden geloggt.

**RepaymentEntry-Updates:**
`audited_create!` + `audited_update!` (für PaidOut-Toggle) — bereits v1.1-Pattern, kein Change für v1.2.

---

## Zusammenfassung: Integrations-Checkliste v1.2

- [x] **Service-Extension:** MemberActionService erweitern mit 4 neuen Methoden (Cancellation, PartialRepayment, Transfer, Increase)
- [x] **ActionType-Check:** Alle nötigen Types existieren schon (`Aufstockung`, `UebertragungEmpfang/Abgabe`, `Austritt` mit effective_date)
- [x] **H1/H2-Funktion:** Reine Funktion `compute_effective_date()` in `member_action.rs` (testbar, keine I/O)
- [x] **Phase-Lookup:** Teil-Rückgabe braucht RepaymentPhase-Lookup; Fallback auf Fehler oder Auto-Create (discuss-phase)
- [x] **Übertrag-Atomarität:** 2 MemberActions + 2 Member-Updates in 1 Tx (Pattern: `mark_paid_out()` Cascade)
- [x] **Frontend-Modal:** Neue `MembershipAdjustModal` Component + Button in MemberDetails (Component-First)
- [x] **Permission:** `MANAGE_MEMBERS_PRIVILEGE` (Vorstand-only) wie v1.1
- [x] **Audit:** `audited_create!` / `audited_update!` für alle Operationen; Hash-Chain-compatible
- [x] **Constraints:** v1.2 erzeugt **KEINE** `MemberAction::Verkauf` und reduziert **NICHT** `current_shares` bei Teil-Rückgabe (nur RepaymentEntry) — kein Doppelbuchen

---

*Last updated: 2026-06-04 — Vorbereitung für `/gsd-discuss-phase 14`. Nächster Schritt: Klären von Auto-Create-Phase-Strategie (Option B empfohlen) und UI-Dialog-Form.*
