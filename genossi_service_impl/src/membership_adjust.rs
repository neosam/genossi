//! Foundation für v1.2 Mitgliedschafts-Anpassungen (Kuendigung, Teil-Rueckgabe, Uebertrag, Aufstockung).
//!
//! Phase 14 liefert ausschliesslich die Pure-Function `compute_effective_date`. Phase 15-17
//! wird dieses Modul mit Service-Methoden + `MembershipAdjustService`-Trait erweitern.

use async_trait::async_trait;
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::{ActionType, MemberActionDao, MemberActionEntity};
use genossi_dao::repayment_entry::{
    RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus,
};
use genossi_dao::repayment_phase::{
    RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus,
};
use genossi_dao::TransactionDao;
use genossi_service::member::Member;
use genossi_service::member_action::MemberAction;
use genossi_service::membership_adjust::MembershipAdjustService;
use genossi_service::permission::{Authentication, PermissionService, ADMIN_PRIVILEGE};
use genossi_service::repayment_entry::RepaymentEntry;
use genossi_service::repayment_phase::RepaymentPhase;
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use time::Date;
use uuid::Uuid;

use crate::gen_service_impl;

/// Audit-Process-String fuer cancel_membership (D-15-02).
const CANCEL_PROCESS: &str = "member-adjust.cancel";

/// Audit-Process-String fuer increase_shares (D-15-02).
const UPGRADE_PROCESS: &str = "member-adjust.upgrade";

/// Audit-Process-String fuer partial_repayment (D-15-02 / D-16-13).
const PARTIAL_REPAYMENT_PROCESS: &str = "member-adjust.partial-repayment";

/// Shared Audit-Process-String fuer ALLE Cascade-Writes des Uebertrags (D-17-04 / AUDT-02).
/// Filter `WHERE process = 'member-adjust.transfer'` findet ALLE Writes eines
/// Uebertrag-Vorgangs (2 oder 3 MemberAction-Creates + 2 Member-Updates).
#[allow(dead_code)] // Plan 17-02 verwendet die Konstante in der Pipeline-Impl.
const TRANSFER_PROCESS: &str = "member-adjust.transfer";

/// Audit-Process-String fuer den inline auto-erzeugten RepaymentPhase-Create
/// in `partial_repayment` (D-16-02 + Resolved Open Question #4).
///
/// Identisch mit `genossi_service_impl::repayment_phase::REPAYMENT_PHASE_PROCESS_CREATE`
/// — die Konstante wird hier lokal dupliziert, damit der Audit-Log forensisch nicht
/// von einer regulaeren `RepaymentPhaseService::create_repayment_phase`-Operation zu
/// unterscheiden ist. Cross-Modul-Import absichtlich vermieden (Modul-Boundary
/// sauber halten).
const REPAYMENT_PHASE_CREATE_PROCESS: &str = "repayment-phase.create";

/// Fallback fuer Auto-Anlegen-Phase wenn keine Vorgaenger-RepaymentPhase existiert
/// (D-16-06/07). Entspricht 100 EUR pro Anteil — Standardwert in Genossi-
/// Installationen. Vorstand sieht die neue Phase im Audit-Log und kann den Wert
/// nachtraeglich via existing v1.1 RepaymentPhase-Update-Endpoint korrigieren.
pub(crate) const DEFAULT_SHARE_VALUE_CENT: i64 = 10000;

gen_service_impl! {
    struct MembershipAdjustServiceImpl: MembershipAdjustService = MembershipAdjustServiceDeps {
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        // Phase 16 (D-16-02 Inlining-Strategy / D-16-08 Sum-Check): zwei neue DAO-Deps.
        // RepaymentPhaseDao: inlined create + dump_all-Lookup fuer Auto-Anlegen (D-16-01/05/06).
        // RepaymentEntryDao: find_by_member_and_phase fuer Sum-Check (D-16-08) +
        //                    audited_create!(RepaymentEntry) (PART-03).
        RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> = repayment_phase_dao,
        RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> = repayment_entry_dao,
    }
}

#[async_trait]
impl<Deps: MembershipAdjustServiceDeps> MembershipAdjustService for MembershipAdjustServiceImpl<Deps> {
    type Context = <Deps as MembershipAdjustServiceDeps>::Context;
    type Transaction = <Deps as MembershipAdjustServiceDeps>::Transaction;

    async fn cancel_membership(
        &self,
        member_id: Uuid,
        willensbekundung_date: Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberAction, Member), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        // PERM-01: ADMIN_PRIVILEGE-Funnel fuer alle v1.2-Ops.
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // PERM-02: Datum-Bounds-Validierung (D-15-05..08).
        let today = time::OffsetDateTime::now_utc().date();
        let validation_errors = validate_willensbekundung_date(willensbekundung_date, today);
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // CANC-01: Member existence.
        let member_entity = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        // D-15-12 / ROADMAP: Already-Cancelled -> HTTP 409 via Conflict-Mapping.
        if member_entity.exit_date.is_some() {
            return Err(ServiceError::Conflict(Arc::from("member already cancelled")));
        }

        // CANC-02 / D-14-04..07: H1/H2-Stichtag berechnen (Phase-14-Pure-Function).
        let effective = compute_effective_date(willensbekundung_date);

        let now = time::OffsetDateTime::now_utc();
        let new_action = MemberAction {
            id: self.uuid_service.new_v4().await,
            member_id,
            action_type: ActionType::Austritt,
            date: willensbekundung_date,
            shares_change: 0, // CANC-03
            transfer_member_id: None,
            effective_date: Some(effective.effective_date),
            comment: None,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        // AUDT-01 / D-15-01 / D-15-02: audited_create! statt direkter DAO-Call.
        let action_entity: MemberActionEntity = (&new_action).into();
        crate::audited_create!(
            self,
            self.member_action_dao,
            &action_entity,
            CANCEL_PROCESS,
            &user_id,
            tx
        );

        // CANC-04: exit_date via recalc_dates Free-Function (KEINE direkte Member.exit_date-Mutation).
        crate::member_action::recalc_dates(
            &*self.member_dao,
            &*self.member_action_dao,
            member_id,
            tx.clone(),
        )
        .await?;

        // Re-Read fuer Response — recalc_dates hat exit_date gesetzt.
        let updated_entity = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        self.transaction_dao.commit(tx).await?;

        Ok((new_action, Member::from(&updated_entity)))
    }

    async fn increase_shares(
        &self,
        member_id: Uuid,
        shares: i32,
        willensbekundung_date: Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberAction, Member), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        // PERM-01: ADMIN_PRIVILEGE-Funnel fuer alle v1.2-Ops.
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Pre-validation: shares > 0 (Planner-Discretion / CONTEXT specifics).
        if shares <= 0 {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("shares"),
                message: Arc::from("shares must be greater than 0"),
            }]));
        }

        // PERM-02: Datum-Bounds-Validierung (D-15-05..08).
        let today = time::OffsetDateTime::now_utc().date();
        let validation_errors = validate_willensbekundung_date(willensbekundung_date, today);
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // UPGD-01: Member existence.
        let member_entity = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        // UPGD-04: gekuendigte Mitglieder blocken -> HTTP 400 via ValidationError.
        if member_entity.exit_date.is_some() {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("member_id"),
                message: Arc::from("cannot upgrade cancelled member"),
            }]));
        }

        // UPGD-02: MemberAction::Aufstockung — sofort wirksam, kein H1/H2 (effective_date=None).
        let now = time::OffsetDateTime::now_utc();
        let new_action = MemberAction {
            id: self.uuid_service.new_v4().await,
            member_id,
            action_type: ActionType::Aufstockung,
            date: willensbekundung_date,
            shares_change: shares, // UPGD-03 positiv
            transfer_member_id: None,
            effective_date: None, // UPGD-02 sofort wirksam
            comment: None,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        // AUDT-01 / D-15-01 / D-15-02: audited_create! mit UPGRADE_PROCESS.
        let action_entity: MemberActionEntity = (&new_action).into();
        crate::audited_create!(
            self,
            self.member_action_dao,
            &action_entity,
            UPGRADE_PROCESS,
            &user_id,
            tx
        );

        // UPGD-03 (atomar in derselben Tx) / D-15-03: Member.current_shares-Bump
        // via generischem MemberDao::update + audited_update! (NICHT targeted DAO-method).
        // exit_date wird NICHT angefasst — Aufstockung beeinflusst kein Exit-Date.
        //
        // Optimistic-Locking-Note (Rule-1 fix entdeckt via E2E in Plan 04):
        // `MemberDao::update` (`genossi_dao_impl_sqlite/src/member.rs:209-300`) liest die
        // ALTE Version aus `entity.version` (WHERE-Klausel) und generiert die NEUE
        // Version INTERN. Deshalb wird `entity.version` hier NICHT gebumpt — sonst
        // matcht die WHERE-Klausel nicht und der Update gibt `Version mismatch`
        // zurueck. Genau dieses Pattern nutzt auch `MemberActionService::update`
        // (`member_action.rs:399-408`): Entity unveraendert in den Macro pumpen.
        let mut updated_entity = member_entity.clone();
        updated_entity.current_shares += shares;

        crate::audited_update!(
            self,
            self.member_dao,
            member_entity.id,
            &updated_entity,
            UPGRADE_PROCESS,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;

        Ok((new_action, Member::from(&updated_entity)))
    }

    /// PART-01..06 / D-16-01..19: Teil-Rueckgabe.
    ///
    /// INLINING-STRATEGY (D-16-04 + Phase 16 research finding #2): Die Phase-Auto-Create-
    /// Branch reproduziert die ~33 LOC von `RepaymentPhaseServiceImpl::create_repayment_phase`
    /// inline, damit die aeussere `tx` geteilt werden kann. Trait-Erweiterung Variante (a)
    /// wurde verworfen (Aenderung an Phase 7/15/17-Code); separate Tx Variante (c) ist von
    /// D-16-04 explizit ausgeschlossen. Audit-Process-String fuer die inlined Phase-Create
    /// bleibt `"repayment-phase.create"` (semantisch identisch zur Service-Delegation,
    /// Open Question #4 Recommendation).
    async fn partial_repayment(
        &self,
        member_id: Uuid,
        shares: i32,
        willensbekundung_date: Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError> {
        // Step 1: Tx-Lifecycle (D-16-04 single tx atomar).
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Step 2: user_id (SYSTEM fallback per Phase 15 convention).
        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        // Step 3: PERM-01 / D-15-01 ADMIN_PRIVILEGE-Funnel.
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Step 4: Member existence.
        let member_entity = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        // Step 5: D-16-10 — Cancelled member -> HTTP 409 Conflict.
        // DIVERGENCE vom Phase 15 UPGD-04 (das gibt 400 ValidationError zurueck).
        // PART hat explizit eigene Semantik: gekuendigte Members gehen via v1.1-PaidOut-
        // Cascade in die naechste Auszahlungsphase — Conflict signalisiert "falscher
        // Workflow", nicht "ungueltige Eingabe".
        if member_entity.exit_date.is_some() {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot start partial repayment for cancelled member (exit_date={:?})",
                member_entity.exit_date
            ))));
        }

        // Step 6: D-16-11/12 — Range-Validation via Pure-Helper aus Plan 01.
        if let Err(errs) = validate_partial_repayment_shares(shares, member_entity.current_shares) {
            return Err(ServiceError::ValidationError(errs));
        }

        // Step 7: D-15-05..08 / D-16-18 — Datum-Bounds-Validierung (Phase 15 reuse).
        let today = time::OffsetDateTime::now_utc().date();
        let validation_errors = validate_willensbekundung_date(willensbekundung_date, today);
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Step 8: CANC-02 / D-14-04..07 — H1/H2-Stichtag via Phase-14-Pure-Function.
        let effective = compute_effective_date(willensbekundung_date);

        // Step 9: ensure_repayment_phase — find existing or auto-create (D-16-01 Variante B).
        // `all()` (Default-Impl) filtert soft-deleted aus. SQLite-Impl von dump_all ordnet
        // `ORDER BY fiscal_year DESC, created DESC` (RESEARCH §"Don't Hand-Roll"), also ist
        // `first()` die juengste Phase fuer den share_value-Fallback (D-16-05).
        let all_phases = self.repayment_phase_dao.all(tx.clone()).await?;
        let target_phase_existing = all_phases
            .iter()
            .find(|p| p.fiscal_year == effective.fiscal_year)
            .cloned();

        // Phase 16.05 / CR-01 — D-11.1-Status-Guard: Eine geschlossene Phase darf
        // keinen neuen Entry aufnehmen. Preparation und Open passieren (Preparation =
        // Phase-14-Pre-Workflow-Reuse, Open = Standardfall, Auto-Create unten erzeugt
        // ohnehin Open). Closed -> HTTP 409 Conflict.
        if let Some(ref existing) = target_phase_existing {
            if existing.status == RepaymentPhaseStatus::Closed {
                return Err(ServiceError::Conflict(Arc::from(format!(
                    "Phase for fiscal_year {} is closed (D-11.1)",
                    effective.fiscal_year
                ))));
            }
        }

        let now_offset = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());

        let (target_phase, was_created): (RepaymentPhaseEntity, bool) = match target_phase_existing
        {
            Some(p) => (p, false),
            None => {
                // D-16-05/06/07 — share_value aus juengster Phase oder Default-Fallback.
                let share_value = all_phases
                    .first()
                    .map(|p| p.share_value)
                    .unwrap_or(DEFAULT_SHARE_VALUE_CENT);

                let auto_phase = RepaymentPhaseEntity {
                    id: self.uuid_service.new_v4().await,
                    fiscal_year: effective.fiscal_year,
                    share_value,
                    // D-16-01 Variante B: Auto-Create in Status `Open` (NICHT Preparation).
                    // Direkt-Insert in die Phase ist sofort moeglich; Skip-Pattern in
                    // open_repayment_phase (Plan 03) verhindert Duplikate.
                    status: RepaymentPhaseStatus::Open,
                    opened_at: Some(now_pdt),
                    closed_at: None,
                    created: now_pdt,
                    deleted: None,
                    version: self.uuid_service.new_v4().await,
                };

                // INLINING (research finding #2): audited_create! statt
                // self.repayment_phase_service.create_repayment_phase(...). Audit-Process-
                // String "repayment-phase.create" = identisch mit der Service-Methode,
                // damit der Audit-Log forensisch nicht unterscheidbar ist.
                crate::audited_create!(
                    self,
                    self.repayment_phase_dao,
                    &auto_phase,
                    REPAYMENT_PHASE_CREATE_PROCESS,
                    &user_id,
                    tx
                );

                (auto_phase, true)
            }
        };

        // Step 10: Sum-Check Foundation — existing entries fuer (member, phase).
        // Research finding #1: find_by_member_and_phase existiert bereits auf Trait UND
        // SQLite-Impl seit Phase 14. KEINE neue DAO-Methode.
        let existing = self
            .repayment_entry_dao
            .find_by_member_and_phase(member_id, target_phase.id, tx.clone())
            .await?;

        // Step 11: D-16-08/09 Sum-Check — Filter status != PaidOut (Contacted zaehlt mit).
        let sum_open: i32 = existing
            .iter()
            .filter(|e| e.status != RepaymentEntryStatus::PaidOut)
            .map(|e| e.share_count_to_pay_out)
            .sum();

        if sum_open + shares > member_entity.current_shares {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("shares"),
                message: Arc::from(format!(
                    "sum of open repayments ({}) plus new ({}) exceeds current_shares ({})",
                    sum_open, shares, member_entity.current_shares
                )),
            }]));
        }

        // Step 12: PART-03 — RepaymentEntry erzeugen (Status Open) via audited_create!.
        let new_entry = RepaymentEntryEntity {
            id: self.uuid_service.new_v4().await,
            member_id,
            phase_id: target_phase.id,
            share_count_to_pay_out: shares,
            status: RepaymentEntryStatus::Open,
            created: now_pdt,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        crate::audited_create!(
            self,
            self.repayment_entry_dao,
            &new_entry,
            PARTIAL_REPAYMENT_PROCESS,
            &user_id,
            tx
        );

        // Step 13: Commit. PART-06 / D-16-19: KEIN recalc_dates, KEIN audited_update!
        // auf Member, KEIN MemberAction. v1.1-PaidOut-Cascade uebernimmt das beim Toggle.
        self.transaction_dao.commit(tx).await?;

        // Step 14: Return tuple. Member wird unveraendert zurueckgegeben (keine Mutation).
        let member_dto = Member::from(&member_entity);
        let entry_dto = RepaymentEntry::from(&new_entry);
        let phase_dto = if was_created {
            Some(RepaymentPhase::from(&target_phase))
        } else {
            None
        };
        Ok((member_dto, entry_dto, phase_dto))
    }

    /// Stub fuer Plan 17-01 — Plan 17-02 implementiert die 15-step Pipeline
    /// (Permission-Funnel, Validierung, Cascade-Writes mit `TRANSFER_PROCESS`).
    /// Signatur ist hier eingefroren, damit Plan 17-02 nur den Body ersetzen muss
    /// und Trait-Drift vermieden wird (D-17-04 / C-17-CF-01).
    async fn transfer_shares(
        &self,
        _from_id: Uuid,
        _to_id: Uuid,
        _shares: i32,
        _transfer_date: Date,
        _context: Authentication<Self::Context>,
        _tx: Option<Self::Transaction>,
    ) -> Result<(Vec<MemberAction>, Member, Member), ServiceError> {
        unimplemented!("Plan 17-02 implements the 15-step pipeline")
    }
}

/// Berechnet den Wirksamkeits-Stichtag nach Verbands-Konvention H1/H2.
///
/// **Konvention** (Verbands-Vorgabe, siehe `.planning/REQUIREMENTS.md` §CANC-02):
/// - H1 (Monat 1-6): Stichtag = 31.12. des laufenden Geschaeftsjahres, `fiscal_year` = aktuelles Jahr
/// - H2 (Monat 7-12): Stichtag = 31.12. des folgenden Geschaeftsjahres, `fiscal_year` = aktuelles Jahr + 1
///
/// Grenzwerte (siehe D-14-04..06):
/// - 30.06. zaehlt zu H1 (`month <= 6`)
/// - 01.07. zaehlt zu H2
/// - 31.12. zaehlt zu H2 -> Stichtag = 31.12. naechstes Jahr
/// - 29.02. (Schaltjahr) zaehlt zu H1 -> 31.12. desselben Jahres
///
/// Edge-Cases werden im `tests`-Submodul abgedeckt (D-14-14).
pub(crate) fn compute_effective_date(willensbekundung: Date) -> EffectiveDate {
    let fiscal_year = if (willensbekundung.month() as u8) <= 6 {
        willensbekundung.year()
    } else {
        willensbekundung.year() + 1
    };
    let effective_date = Date::from_calendar_date(fiscal_year, time::Month::December, 31)
        .expect("31. Dezember ist in jedem Jahr ein gueltiges Datum (kein Schalttag)");
    EffectiveDate {
        fiscal_year,
        effective_date,
    }
}

/// Ergebnis der H1/H2-Stichtagsberechnung (D-14-01).
///
/// `Copy`-able, weil `i32` und `time::Date` beide `Copy` sind. Vereinfacht
/// Call-Site-Pattern wie `let r = compute_effective_date(d); use r.fiscal_year`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EffectiveDate {
    pub fiscal_year: i32,
    pub effective_date: Date,
}

/// Validiert das Willensbekundungs-Datum gegen die Kalender-Jahr-Bounds (D-15-06, PERM-02).
///
/// Erlaubt sind nur das aktuelle und das naechste Kalender-Jahr (relativ zu `today`).
/// Die Funktion ist pure (kein clock-bezogener Aufruf wie `now_utc`, D-15-07), damit der
/// Aufrufer (Service-Layer in Plan 02/03) `today` kontrolliert testbar uebergeben kann.
pub(crate) fn validate_willensbekundung_date(
    date: Date,
    today: Date,
) -> Vec<ValidationFailureItem> {
    let current_fy = today.year();
    let next_fy = current_fy + 1;
    if date.year() == current_fy || date.year() == next_fy {
        Vec::new()
    } else {
        vec![ValidationFailureItem {
            field: Arc::from("willensbekundung_date"),
            message: Arc::from(format!(
                "must be in fiscal year {} or {}",
                current_fy, next_fy
            )),
        }]
    }
}

/// Pure-Function range-validator fuer Teil-Rueckgabe (D-16-11 + D-16-12).
///
/// Wirft `ValidationError` bei:
/// - `shares <= 0` (mindestens 1 Anteil)
/// - `shares >= current_shares` (Voll-Rueckgabe-Block; Verweis auf cancel_membership)
///
/// Returns `Ok(())` fuer `1 <= shares < current_shares`. Sum-Check gegen offene
/// Repayment-Entries der gleichen Phase erfolgt SPAETER im Service (D-16-08),
/// nachdem die Ziel-Phase aufgeloest ist — diese Funktion ist DAO-frei.
pub(crate) fn validate_partial_repayment_shares(
    shares: i32,
    current_shares: i32,
) -> Result<(), Vec<ValidationFailureItem>> {
    if shares <= 0 {
        return Err(vec![ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from("shares must be at least 1"),
        }]);
    }
    if shares == current_shares {
        return Err(vec![ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from(
                "shares must be strictly less than current_shares — use cancel_membership for full return",
            ),
        }]);
    }
    if shares > current_shares {
        return Err(vec![ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from(format!(
                "shares ({}) exceeds current_shares ({})",
                shares, current_shares
            )),
        }]);
    }
    Ok(())
}

/// Pure-Function range-validator fuer Uebertrag (D-17-09 / TRSF-07).
///
/// Sammelt ALLE Verletzungen (kein early-return), damit das Frontend mehrere
/// Felder gleichzeitig anzeigen kann.
///
/// Wirft einen `ValidationFailureItem` pro Verletzung bei:
/// - `from_id == to_id` (TRSF-07 self-transfer, field=`to_member_id`)
/// - `shares <= 0` (mindestens 1 Anteil, field=`shares`)
/// - `shares > from_current_shares` (field=`shares`; Voll-Uebertrag-Boundary
///   `shares == from_current_shares` ist GUELTIG -- Voll-Uebertrag-Branch
///   wird im Service ausgewertet, D-17-01)
///
/// Returns empty `Vec` fuer alle gueltigen Eingaben (`1 <= shares <= from_current_shares`
/// und `from_id != to_id`).
#[allow(dead_code)] // Plan 17-02 ruft die Funktion aus der Pipeline auf.
pub(crate) fn validate_transfer_inputs(
    from_id: Uuid,
    to_id: Uuid,
    shares: i32,
    from_current_shares: i32,
) -> Vec<ValidationFailureItem> {
    let mut errors = Vec::new();
    if from_id == to_id {
        errors.push(ValidationFailureItem {
            field: Arc::from("to_member_id"),
            message: Arc::from("cannot transfer to self"),
        });
    }
    if shares <= 0 {
        errors.push(ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from("shares must be at least 1"),
        });
    }
    if shares > from_current_shares {
        errors.push(ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from(format!(
                "shares ({}) exceeds from.current_shares ({})",
                shares, from_current_shares
            )),
        });
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_compute_effective_date_30_juni_is_h1() {
        let input = Date::from_calendar_date(2026, Month::June, 30).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2026, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_01_juli_is_h2() {
        let input = Date::from_calendar_date(2026, Month::July, 1).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2027);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2027, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_31_dezember_is_h2_next_year() {
        let input = Date::from_calendar_date(2026, Month::December, 31).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2027);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2027, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_01_januar_is_h1() {
        let input = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2026, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_schaltjahr_29_februar_is_h1() {
        let input = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2024);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2024, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_mittiges_datum_15_maerz_is_h1() {
        let input = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2026, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_validate_willensbekundung_aktuelles_jahr_valid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2026, Month::June, 15).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    #[test]
    fn test_validate_willensbekundung_naechstes_jahr_valid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2027, Month::June, 15).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    #[test]
    fn test_validate_willensbekundung_vorjahr_invalid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2025, Month::December, 31).unwrap();
        let errors = validate_willensbekundung_date(date, today);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "willensbekundung_date");
        assert!(errors[0].message.contains("2026"));
        assert!(errors[0].message.contains("2027"));
    }

    #[test]
    fn test_validate_willensbekundung_uebernaechstes_jahr_invalid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2028, Month::January, 1).unwrap();
        let errors = validate_willensbekundung_date(date, today);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "willensbekundung_date");
    }

    #[test]
    fn test_validate_willensbekundung_today_31_dezember_naechstes_jahr_valid() {
        let today = Date::from_calendar_date(2026, Month::December, 31).unwrap();
        let date = Date::from_calendar_date(2027, Month::December, 31).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    #[test]
    fn test_validate_willensbekundung_schaltjahr_29_februar_valid() {
        let today = Date::from_calendar_date(2024, Month::January, 15).unwrap();
        let date = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    // -------------------------------------------------------------------------
    // validate_partial_repayment_shares — D-16-11 + D-16-12 Range-Validation
    // (Phase 16, 7 cases per Plan-16-01 Task 3).
    // -------------------------------------------------------------------------

    #[test]
    fn validate_partial_repayment_shares_zero_rejected() {
        let errs = validate_partial_repayment_shares(0, 5).expect_err("shares=0 must reject");
        assert_eq!(errs.len(), 1);
        assert_eq!(&*errs[0].field, "shares");
        assert!(errs[0].message.contains("at least 1"));
    }

    #[test]
    fn validate_partial_repayment_shares_negative_rejected() {
        let errs = validate_partial_repayment_shares(-5, 5).expect_err("negative shares must reject");
        assert!(errs[0].message.contains("at least 1"));
    }

    #[test]
    fn validate_partial_repayment_shares_equal_to_current_rejected_with_cancel_hint() {
        let errs = validate_partial_repayment_shares(10, 10)
            .expect_err("shares == current_shares must reject");
        assert!(
            errs[0].message.contains("cancel_membership"),
            "error must reference cancel_membership (D-16-11), got: {}",
            errs[0].message
        );
    }

    #[test]
    fn validate_partial_repayment_shares_above_current_rejected() {
        let errs = validate_partial_repayment_shares(11, 10)
            .expect_err("shares > current_shares must reject");
        assert!(errs[0].message.contains("exceeds current_shares"));
    }

    #[test]
    fn validate_partial_repayment_shares_full_one_member_rejected() {
        // Member has 1 share; any positive shares-value equals current_shares = Voll-Rueckgabe.
        let errs = validate_partial_repayment_shares(1, 1).expect_err("1/1 is voll, must reject");
        assert!(errs[0].message.contains("cancel_membership"));
    }

    #[test]
    fn validate_partial_repayment_shares_happy_path_minimum() {
        validate_partial_repayment_shares(1, 2).expect("1 of 2 must accept");
    }

    #[test]
    fn validate_partial_repayment_shares_happy_path_middle() {
        validate_partial_repayment_shares(5, 10).expect("5 of 10 must accept");
    }

    // -------------------------------------------------------------------------
    // D-17-09 / TRSF-07: validate_transfer_inputs (7 Edge-Cases, Plan 17-01).
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_transfer_n_zero_invalid() {
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        let errs = super::validate_transfer_inputs(from, to, 0, 5);
        assert_eq!(errs.len(), 1, "n=0 must produce exactly one error");
        assert_eq!(&*errs[0].field, "shares");
        assert!(errs[0].message.contains("at least 1"));
    }

    #[test]
    fn test_validate_transfer_n_negative_invalid() {
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        let errs = super::validate_transfer_inputs(from, to, -1, 5);
        assert_eq!(errs.len(), 1, "n=-1 must produce exactly one error");
        assert_eq!(&*errs[0].field, "shares");
    }

    #[test]
    fn test_validate_transfer_n_equal_current_shares_valid() {
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        let errs = super::validate_transfer_inputs(from, to, 5, 5);
        assert!(
            errs.is_empty(),
            "Voll-Uebertrag-Boundary (n == current_shares) must be valid; got {:?}",
            errs
        );
    }

    #[test]
    fn test_validate_transfer_n_exceeds_current_shares_invalid() {
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        let errs = super::validate_transfer_inputs(from, to, 6, 5);
        assert_eq!(
            errs.len(),
            1,
            "n>current_shares must produce exactly one error"
        );
        assert_eq!(&*errs[0].field, "shares");
        assert!(errs[0].message.contains("exceeds"));
    }

    #[test]
    fn test_validate_transfer_self_invalid() {
        let id = uuid::Uuid::new_v4();
        let errs = super::validate_transfer_inputs(id, id, 1, 5);
        assert!(
            errs.iter().any(|e| &*e.field == "to_member_id"),
            "self-transfer must produce a to_member_id error; got {:?}",
            errs
        );
        assert!(
            errs.iter()
                .any(|e| e.message.contains("cannot transfer to self")),
            "must include the canonical message; got {:?}",
            errs
        );
    }

    #[test]
    fn test_validate_transfer_n_one_valid() {
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        let errs = super::validate_transfer_inputs(from, to, 1, 5);
        assert!(
            errs.is_empty(),
            "n=1 (Teil-Uebertrag) must be valid; got {:?}",
            errs
        );
    }

    #[test]
    fn test_validate_transfer_multiple_violations_accumulate() {
        let id = uuid::Uuid::new_v4();
        // Self-transfer AND shares=0 -- beide Fehler muessen gesammelt werden.
        let errs = super::validate_transfer_inputs(id, id, 0, 5);
        assert_eq!(
            errs.len(),
            2,
            "two distinct violations must accumulate; got {:?}",
            errs
        );
    }
}

#[cfg(test)]
mod service_tests {
    //! Service-Tests fuer cancel_membership (Plan 15-02).
    //!
    //! Per-File-Mock-Pattern statt globalem `#[automock]`, weil `gen_service_impl!`
    //! `Debug` auf Transaction-Type verlangt; genossi_dao::MockTransaction hat KEIN
    //! Debug-Derive. Vorbild: genossi_service_impl/src/member.rs:412-712.

    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::mock;

    /// Test-local Transaction with Debug — MockTransaction in genossi_dao hat kein Debug,
    /// aber gen_service_impl! verlangt es.
    #[derive(Clone, Debug)]
    pub struct TestTransaction;

    #[async_trait]
    impl Transaction for TestTransaction {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao {
            type Transaction = TestTransaction;
            async fn transaction(&self) -> Result<TestTransaction, DaoError>;
            async fn use_transaction(&self, tx: Option<TestTransaction>) -> Result<TestTransaction, DaoError>;
            async fn commit(&self, tx: TestTransaction) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub TestMemberDao {}
        #[async_trait]
        impl MemberDao for TestMemberDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn create(&self, entity: &MemberEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<MemberEntity>, DaoError>;
            async fn update_migrated(&self, id: Uuid, migrated: bool, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update_dates(&self, id: Uuid, join_date: time::Date, exit_date: Option<time::Date>, tx: TestTransaction) -> Result<(), DaoError>;
            async fn find_by_member_number(&self, member_number: i64, tx: TestTransaction) -> Result<Option<MemberEntity>, DaoError>;
            async fn count_active(&self, today: time::Date, tx: TestTransaction) -> Result<u64, DaoError>;
            async fn next_member_number(&self, tx: TestTransaction) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestMemberActionDao {}
        #[async_trait]
        impl MemberActionDao for TestMemberActionDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[MemberActionEntity]>, DaoError>;
            async fn create(&self, entity: &MemberActionEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberActionEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[MemberActionEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<MemberActionEntity>, DaoError>;
            async fn find_by_member_id(&self, member_id: Uuid, tx: TestTransaction) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        }
    }

    // Phase 16 Plan 02 — mocks for the two new DAO-deps.

    mock! {
        pub TestRepaymentPhaseDao {}
        #[async_trait]
        impl RepaymentPhaseDao for TestRepaymentPhaseDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn create(&self, entity: &RepaymentPhaseEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &RepaymentPhaseEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<RepaymentPhaseEntity>, DaoError>;
        }
    }

    mock! {
        pub TestRepaymentEntryDao {}
        #[async_trait]
        impl RepaymentEntryDao for TestRepaymentEntryDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn create(&self, entity: &RepaymentEntryEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &RepaymentEntryEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<RepaymentEntryEntity>, DaoError>;
            async fn find_by_phase_id(&self, phase_id: Uuid, tx: TestTransaction) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn find_by_member_and_phase(&self, member_id: Uuid, phase_id: Uuid, tx: TestTransaction) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
        }
    }

    mock! {
        pub TestAuditLogDao {}
        #[async_trait]
        impl AuditLogDao for TestAuditLogDao {
            type Transaction = TestTransaction;
            async fn create_entries(&self, entries: &[AuditLogEntry], tx: TestTransaction) -> Result<(), DaoError>;
            async fn get_latest_hash(&self, tx: TestTransaction) -> Result<Option<String>, DaoError>;
            async fn get_by_entity(&self, entity_type: &str, entity_id: Uuid, tx: TestTransaction) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn get_all_ordered(&self, tx: TestTransaction) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn query(&self, filter: AuditQueryFilter, limit: i64, offset: i64, tx: TestTransaction) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn count(&self, filter: AuditQueryFilter, tx: TestTransaction) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = MockContext;
            async fn check_permission(&self, privilege: &str, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn current_user_id(&self, context: Authentication<MockContext>) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(&self, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(&self, user: genossi_service::auth_types::UserTO, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn delete_user(&self, username: String, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_all_roles(&self, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(&self, role: genossi_service::auth_types::RoleTO, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn delete_role(&self, role_name: String, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_all_privileges(&self, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(&self, privilege: genossi_service::auth_types::PrivilegeTO, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn delete_privilege(&self, privilege_name: String, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn assign_user_role(&self, user_role: genossi_service::auth_types::UserRole, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn remove_user_role(&self, user_role: genossi_service::auth_types::UserRole, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_user_roles(&self, username: String, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(&self, role_privilege: genossi_service::auth_types::RolePrivilege, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn remove_role_privilege(&self, role_privilege: genossi_service::auth_types::RolePrivilege, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_role_privileges(&self, role_name: String, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(&self, username: String, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &MockContext) -> Result<bool, ServiceError>;
        }
    }

    /// Static UUID-Service fuer Tests — neue UUIDs jedes Mal.
    #[derive(Clone)]
    pub struct StaticUuidService;
    #[async_trait]
    impl UuidService for StaticUuidService {
        async fn new_v4(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    /// TestDeps wires the local mocks as associated types.
    pub struct TestDeps;
    impl MembershipAdjustServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type MemberActionDao = MockTestMemberActionDao;
        type MemberDao = MockTestMemberDao;
        type AuditLogDao = MockTestAuditLogDao;
        type PermissionService = MockTestPermissionService;
        type UuidService = StaticUuidService;
        type TransactionDao = MockTestTxDao;
        // Phase 16 Plan 02 — DAO-deps for partial_repayment.
        type RepaymentPhaseDao = MockTestRepaymentPhaseDao;
        type RepaymentEntryDao = MockTestRepaymentEntryDao;
    }

    fn sample_member_entity(id: Uuid, exit_date: Option<time::Date>) -> MemberEntity {
        let join = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        MemberEntity {
            id,
            member_number: 1001,
            first_name: Arc::from("Klaus"),
            last_name: Arc::from("Kuendigung"),
            salutation: Some(Salutation::Herr),
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: join,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date,
            bank_account: None,
            status: MemberStatus::Normal,
            created: time::PrimitiveDateTime::new(join, time::Time::MIDNIGHT),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn setup_tx_dao() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao.expect_use_transaction().returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn build_service(
        member_dao: MockTestMemberDao,
        member_action_dao: MockTestMemberActionDao,
        audit_log_dao: MockTestAuditLogDao,
        permission_service: MockTestPermissionService,
        tx_dao: MockTestTxDao,
    ) -> MembershipAdjustServiceImpl<TestDeps> {
        // Phase 16 Plan 02: Phase-15-Tests benoetigen die zwei neuen DAO-Deps NICHT
        // (cancel_membership + increase_shares ruefen sie nicht auf). Wir injizieren
        // leere Mocks ohne Expectations — mockall faellt bei einem unerwarteten Call
        // mit Panic. Das macht das Pattern selbst-validierend: falls Phase 15 versehentlich
        // anfangen wuerde, die RepaymentPhase/Entry-DAOs zu nutzen, schlugen die
        // bestehenden Tests sofort fehl.
        let repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        let repayment_entry_dao = MockTestRepaymentEntryDao::new();
        MembershipAdjustServiceImpl {
            member_action_dao: Arc::new(member_action_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(audit_log_dao),
            permission_service: Arc::new(permission_service),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(tx_dao),
            repayment_phase_dao: Arc::new(repayment_phase_dao),
            repayment_entry_dao: Arc::new(repayment_entry_dao),
        }
    }

    /// Phase 16 Plan 02 — Service-Builder fuer partial_repayment-Tests
    /// (mit den zwei neuen DAO-Deps).
    fn build_service_part(
        member_dao: MockTestMemberDao,
        member_action_dao: MockTestMemberActionDao,
        audit_log_dao: MockTestAuditLogDao,
        permission_service: MockTestPermissionService,
        tx_dao: MockTestTxDao,
        repayment_phase_dao: MockTestRepaymentPhaseDao,
        repayment_entry_dao: MockTestRepaymentEntryDao,
    ) -> MembershipAdjustServiceImpl<TestDeps> {
        MembershipAdjustServiceImpl {
            member_action_dao: Arc::new(member_action_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(audit_log_dao),
            permission_service: Arc::new(permission_service),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(tx_dao),
            repayment_phase_dao: Arc::new(repayment_phase_dao),
            repayment_entry_dao: Arc::new(repayment_entry_dao),
        }
    }

    /// Phase 16 Plan 02 — sample MemberEntity with explicit `current_shares`.
    fn sample_member_entity_with_shares(
        id: Uuid,
        current_shares: i32,
        exit_date: Option<time::Date>,
    ) -> MemberEntity {
        let mut m = sample_member_entity(id, exit_date);
        m.current_shares = current_shares;
        m
    }

    /// Phase 16 Plan 02 — sample RepaymentPhaseEntity for a given fiscal_year.
    fn sample_repayment_phase(fiscal_year: i32, share_value: i64) -> RepaymentPhaseEntity {
        let date = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year,
            share_value,
            status: RepaymentPhaseStatus::Open,
            opened_at: Some(datetime),
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    /// Phase 16 Plan 02 — sample existing RepaymentEntryEntity for sum-check tests.
    fn sample_repayment_entry(
        member_id: Uuid,
        phase_id: Uuid,
        share_count: i32,
        status: RepaymentEntryStatus,
    ) -> RepaymentEntryEntity {
        let date = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id,
            phase_id,
            share_count_to_pay_out: share_count,
            status,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    // ---------- Test 1: Happy-Path H1 ----------
    #[tokio::test]
    async fn test_cancel_membership_happy_path_h1() {
        // Datum-Fragility-Fix: leite das Test-Datum relativ zu today() ab,
        // damit der Test nicht beim Year-Rollover bricht. H1 = Januar..Juni desselben Jahres.
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .expect("March in today.year() exists")
            .replace_day(15)
            .expect("Day 15 always valid");
        let expected_effective =
            time::Date::from_calendar_date(today.year(), time::Month::December, 31)
                .expect("year-12-31 always valid");

        let member_id = Uuid::new_v4();

        let mut member_dao = MockTestMemberDao::new();
        let mid = member_id;
        // find_by_id wird 3x aufgerufen: 1x fuer existence-check im Service-Body,
        // 1x intern in recalc_dates(), 1x fuer re-read nach recalc_dates.
        member_dao
            .expect_find_by_id()
            .times(3)
            .returning(move |_, _| Ok(Some(sample_member_entity(mid, None))));
        // recalc_dates ruft update_dates auf.
        member_dao
            .expect_update_dates()
            .returning(|_, _, _, _| Ok(()));

        let mut member_action_dao = MockTestMemberActionDao::new();
        // recalc_dates ruft find_by_member_id auf.
        member_action_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberActionEntity>::new())));
        // audited_create! ruft create() auf mit Process="member-adjust.cancel".
        member_action_dao
            .expect_create()
            .withf(|_entity, process, _tx| process == "member-adjust.cancel")
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut audit_log_dao = MockTestAuditLogDao::new();
        audit_log_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_log_dao
            .expect_create_entries()
            .returning(|_, _| Ok(()));

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        permission_service
            .expect_check_permission()
            .withf(|priv_, _| priv_ == "admin")
            .returning(|_, _| Ok(()));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .cancel_membership(member_id, willensbekundung, Authentication::Full, None)
            .await;

        let (action, _member) = result.expect("cancel_membership should succeed");
        assert_eq!(action.action_type, ActionType::Austritt);
        assert_eq!(action.shares_change, 0);
        assert_eq!(action.date, willensbekundung);
        assert_eq!(action.effective_date, Some(expected_effective));
        assert_eq!(action.transfer_member_id, None);
    }

    // ---------- Test 2: Happy-Path H2 ----------
    #[tokio::test]
    async fn test_cancel_membership_happy_path_h2() {
        let today = time::OffsetDateTime::now_utc().date();
        // H2 = August desselben Jahres -> effective = next-year-12-31.
        let willensbekundung = today
            .replace_month(time::Month::August)
            .expect("August in today.year() exists")
            .replace_day(15)
            .expect("Day 15 always valid");
        let expected_effective =
            time::Date::from_calendar_date(today.year() + 1, time::Month::December, 31)
                .expect("(year+1)-12-31 always valid");

        let member_id = Uuid::new_v4();

        let mut member_dao = MockTestMemberDao::new();
        let mid = member_id;
        // find_by_id wird 3x aufgerufen: Service-Body existence-check, recalc_dates intern, Re-Read.
        member_dao
            .expect_find_by_id()
            .times(3)
            .returning(move |_, _| Ok(Some(sample_member_entity(mid, None))));
        member_dao
            .expect_update_dates()
            .returning(|_, _, _, _| Ok(()));

        let mut member_action_dao = MockTestMemberActionDao::new();
        member_action_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberActionEntity>::new())));
        member_action_dao
            .expect_create()
            .withf(|_, process, _| process == "member-adjust.cancel")
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut audit_log_dao = MockTestAuditLogDao::new();
        audit_log_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_log_dao
            .expect_create_entries()
            .returning(|_, _| Ok(()));

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Ok(()));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .cancel_membership(member_id, willensbekundung, Authentication::Full, None)
            .await;

        let (action, _member) = result.expect("cancel_membership H2 should succeed");
        assert_eq!(action.action_type, ActionType::Austritt);
        assert_eq!(action.effective_date, Some(expected_effective));
    }

    // ---------- Test 3: Permission denied ----------
    #[tokio::test]
    async fn test_cancel_membership_permission_denied() {
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .unwrap()
            .replace_day(15)
            .unwrap();

        // Keine DAO-Calls erwartet — Permission-Denied bricht VOR DAO-Touches ab.
        let member_dao = MockTestMemberDao::new();
        let member_action_dao = MockTestMemberActionDao::new();
        let audit_log_dao = MockTestAuditLogDao::new();

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("user".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .cancel_membership(Uuid::new_v4(), willensbekundung, Authentication::Full, None)
            .await;

        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "expected PermissionDenied (mapped to HTTP 401 via genossi_rest/src/lib.rs:115), got {:?}",
            result
        );
    }

    // ---------- Test 4: Already-Cancelled ----------
    #[tokio::test]
    async fn test_cancel_membership_already_cancelled() {
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .unwrap()
            .replace_day(15)
            .unwrap();
        let already_exit =
            time::Date::from_calendar_date(today.year(), time::Month::December, 31).unwrap();

        let member_id = Uuid::new_v4();

        let mut member_dao = MockTestMemberDao::new();
        let mid = member_id;
        member_dao
            .expect_find_by_id()
            .times(1) // nur 1x — Conflict bricht vor recalc_dates ab.
            .returning(move |_, _| Ok(Some(sample_member_entity(mid, Some(already_exit)))));

        // KEINE audit/create-Calls erwartet.
        let member_action_dao = MockTestMemberActionDao::new();
        let audit_log_dao = MockTestAuditLogDao::new();

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Ok(()));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .cancel_membership(member_id, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(msg.contains("already cancelled"), "unexpected msg: {}", msg);
            }
            other => panic!("Expected Conflict, got {:?}", other),
        }
    }

    // ---------- Test 5: Happy-Path Aufstockung ----------
    #[tokio::test]
    async fn test_increase_shares_happy_path() {
        // Datum-Fragility-Fix: leite Test-Datum relativ zu today() ab,
        // damit der Test nicht beim Year-Rollover bricht.
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .expect("March in today.year() exists")
            .replace_day(15)
            .expect("Day 15 always valid");

        let member_id = Uuid::new_v4();

        let mut member_dao = MockTestMemberDao::new();
        let mid = member_id;
        // find_by_id wird 2x aufgerufen: 1x Service-Body, 1x audited_update! Macro
        // (find_by_id auf member_dao fuer old-entity-load).
        member_dao
            .expect_find_by_id()
            .times(2)
            .returning(move |_, _| {
                let mut m = sample_member_entity(mid, None);
                m.current_shares = 2;
                Ok(Some(m))
            });
        // audited_update! ruft member_dao.update mit current_shares=5 (=2+3) und
        // Process="member-adjust.upgrade".
        member_dao
            .expect_update()
            .withf(|entity, process, _| {
                process == "member-adjust.upgrade" && entity.current_shares == 5
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut member_action_dao = MockTestMemberActionDao::new();
        // audited_create! ruft create() mit Process="member-adjust.upgrade".
        member_action_dao
            .expect_create()
            .withf(|_entity, process, _tx| process == "member-adjust.upgrade")
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut audit_log_dao = MockTestAuditLogDao::new();
        // 2x get_latest_hash + create_entries (1x fuer Action, 1x fuer Member-Diff).
        audit_log_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_log_dao
            .expect_create_entries()
            .returning(|_, _| Ok(()));

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        permission_service
            .expect_check_permission()
            .withf(|priv_, _| priv_ == "admin")
            .returning(|_, _| Ok(()));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .increase_shares(member_id, 3, willensbekundung, Authentication::Full, None)
            .await;

        let (action, member) = result.expect("increase_shares should succeed");
        assert_eq!(action.action_type, ActionType::Aufstockung);
        assert_eq!(action.shares_change, 3);
        assert_eq!(action.effective_date, None); // UPGD-02 sofort wirksam
        // Direkter pub-Feld-Zugriff per genossi_service::member::Member.
        // Erwartung: 2 (sample) + 3 (shares) = 5.
        assert_eq!(member.current_shares, 5);
    }

    // ---------- Test 6: Cancelled-Member-Block (UPGD-04) ----------
    #[tokio::test]
    async fn test_increase_shares_cancelled_member_blocked() {
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .unwrap()
            .replace_day(15)
            .unwrap();
        let already_exit =
            time::Date::from_calendar_date(today.year(), time::Month::December, 31).unwrap();

        let member_id = Uuid::new_v4();

        let mut member_dao = MockTestMemberDao::new();
        let mid = member_id;
        member_dao
            .expect_find_by_id()
            .times(1) // Block bricht VOR audited_create!/audited_update! ab
            .returning(move |_, _| Ok(Some(sample_member_entity(mid, Some(already_exit)))));

        // KEINE create/update-Calls erwartet.
        let member_action_dao = MockTestMemberActionDao::new();
        let audit_log_dao = MockTestAuditLogDao::new();

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Ok(()));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .increase_shares(member_id, 3, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::ValidationError(errs)) => {
                assert!(
                    errs.iter().any(|e| e.message.contains("cancelled")),
                    "expected ValidationError containing 'cancelled', got: {:?}",
                    errs
                );
            }
            other => panic!("Expected ValidationError, got {:?}", other),
        }
    }

    // ---------- Test 7: Permission Denied ----------
    #[tokio::test]
    async fn test_increase_shares_permission_denied() {
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .unwrap()
            .replace_day(15)
            .unwrap();

        // Keine DAO-Calls erwartet — Permission-Denied bricht VOR DAO-Touches ab.
        let member_dao = MockTestMemberDao::new();
        let member_action_dao = MockTestMemberActionDao::new();
        let audit_log_dao = MockTestAuditLogDao::new();

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("user".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .increase_shares(Uuid::new_v4(), 3, willensbekundung, Authentication::Full, None)
            .await;

        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "expected PermissionDenied (mapped to HTTP 401 via genossi_rest/src/lib.rs:115), got {:?}",
            result
        );
    }

    // ---------- Test 8: Invalid shares (shares=0) ----------
    #[tokio::test]
    async fn test_increase_shares_invalid_shares_zero() {
        let today = time::OffsetDateTime::now_utc().date();
        let willensbekundung = today
            .replace_month(time::Month::March)
            .unwrap()
            .replace_day(15)
            .unwrap();

        // Keine DAO-Calls erwartet — Pre-Validation bricht VOR DAO-Touches ab.
        let member_dao = MockTestMemberDao::new();
        let member_action_dao = MockTestMemberActionDao::new();
        let audit_log_dao = MockTestAuditLogDao::new();

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Ok(()));

        let service = build_service(
            member_dao,
            member_action_dao,
            audit_log_dao,
            permission_service,
            setup_tx_dao(),
        );

        let result = service
            .increase_shares(Uuid::new_v4(), 0, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::ValidationError(errs)) => {
                assert!(
                    errs.iter().any(|e| &*e.field == "shares"),
                    "expected ValidationError with field='shares', got: {:?}",
                    errs
                );
            }
            other => panic!("Expected ValidationError with field=shares, got {:?}", other),
        }
    }

    // =========================================================================
    // Phase 16 Plan 02 — partial_repayment service tests (10 cases)
    //
    // Coverage:
    //   1) happy_path (existing phase, sum-check OK, entry created)
    //   2) n_zero_invalid (validation rejects, no DAO writes)
    //   3) n_negative_invalid
    //   4) n_equals_current_shares_blocked (D-16-11 cancel hint)
    //   5) cancelled_member_blocked (D-16-10 -> Conflict 409, NOT ValidationError)
    //   6) sum_check_violation
    //   7) auto_create_uses_previous_share_value (latest phase share_value)
    //   8) auto_create_fallback_default_share_value (no phases at all)
    //   9) permission_denied (no DAO touches at all)
    //  10) paid_out_entries_excluded_from_sum (D-16-09)
    //
    // All tests use the Plan-15-mock-pattern but inject the two new DAO mocks
    // (MockTestRepaymentPhaseDao, MockTestRepaymentEntryDao). Test helpers
    // sample_repayment_phase / sample_repayment_entry are defined above.
    // =========================================================================

    /// Test date that always falls into H1 of the current calendar year.
    fn h1_test_date() -> Date {
        let today = time::OffsetDateTime::now_utc().date();
        today
            .replace_month(time::Month::March)
            .expect("March in today.year() is valid")
            .replace_day(15)
            .expect("Day 15 always valid")
    }

    /// Test date that always falls into H2 of the current calendar year.
    /// `compute_effective_date(this) -> fiscal_year = today.year() + 1`.
    fn h2_test_date() -> Date {
        let today = time::OffsetDateTime::now_utc().date();
        today
            .replace_month(time::Month::December)
            .expect("December in today.year() is valid")
            .replace_day(15)
            .expect("Day 15 always valid")
    }

    /// Returns the fiscal_year H1-Tests target (= today.year()).
    fn h1_target_fy() -> i32 {
        time::OffsetDateTime::now_utc().date().year()
    }

    /// Returns the fiscal_year H2-Tests target (= today.year() + 1).
    fn h2_target_fy() -> i32 {
        time::OffsetDateTime::now_utc().date().year() + 1
    }

    fn allow_admin_perms() -> MockTestPermissionService {
        let mut p = MockTestPermissionService::new();
        p.expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        p.expect_check_permission().returning(|_, _| Ok(()));
        p
    }

    fn allow_audit_log() -> MockTestAuditLogDao {
        let mut a = MockTestAuditLogDao::new();
        a.expect_get_latest_hash().returning(|_| Ok(None));
        a.expect_create_entries().returning(|_, _| Ok(()));
        a
    }

    // ---------- Test 1: Happy-Path ----------
    #[tokio::test]
    async fn test_partial_repayment_happy_path() {
        // 5 current shares, request 2; existing matching phase, no existing entries.
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();
        let target_fy = h1_target_fy();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        let phase = sample_repayment_phase(target_fy, 12000);
        let phase_id = phase.id;
        let phases: Arc<[RepaymentPhaseEntity]> = Arc::from(vec![phase]);
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao
            .expect_all()
            .returning(move |_| Ok(phases.clone()));
        // No phase auto-create:
        repayment_phase_dao.expect_create().times(0);

        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        let empty_entries: Arc<[RepaymentEntryEntity]> = Arc::from(Vec::<RepaymentEntryEntity>::new());
        repayment_entry_dao
            .expect_find_by_member_and_phase()
            .withf(move |m, p, _| *m == member_id && *p == phase_id)
            .returning(move |_, _, _| Ok(empty_entries.clone()));
        repayment_entry_dao
            .expect_create()
            .withf(|entity, process, _| {
                process == "member-adjust.partial-repayment" && entity.share_count_to_pay_out == 2
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let (_member, entry, phase_opt) = service
            .partial_repayment(member_id, 2, willensbekundung, Authentication::Full, None)
            .await
            .expect("happy path must succeed");

        assert_eq!(entry.share_count_to_pay_out, 2);
        assert!(
            phase_opt.is_none(),
            "existing phase reused — Option must be None"
        );
    }

    // ---------- Test 2: shares=0 invalid ----------
    #[tokio::test]
    async fn test_partial_repayment_n_zero_invalid() {
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();

        let mut member_dao = MockTestMemberDao::new();
        // find_by_id may run (validate-after-member-load order). Allow at most 1 call.
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        // No DAO writes expected.
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao.expect_create().times(0);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(member_id, 0, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::ValidationError(errs)) => {
                assert!(errs.iter().any(|e| e.message.contains("at least 1")));
            }
            other => panic!("Expected ValidationError 'at least 1', got {:?}", other),
        }
    }

    // ---------- Test 3: shares=-1 invalid ----------
    #[tokio::test]
    async fn test_partial_repayment_n_negative_invalid() {
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao.expect_create().times(0);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(member_id, -1, willensbekundung, Authentication::Full, None)
            .await;

        assert!(
            matches!(result, Err(ServiceError::ValidationError(_))),
            "expected ValidationError for negative shares, got {:?}",
            result
        );
    }

    // ---------- Test 4: shares == current_shares blocked (Voll-Rueckgabe) ----------
    #[tokio::test]
    async fn test_partial_repayment_n_equals_current_shares_blocked() {
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 3, None))));

        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao.expect_create().times(0);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(member_id, 3, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::ValidationError(errs)) => {
                assert!(
                    errs.iter().any(|e| e.message.contains("cancel_membership")),
                    "expected message to reference cancel_membership (D-16-11), got: {:?}",
                    errs
                );
            }
            other => panic!("Expected ValidationError mentioning cancel_membership, got {:?}", other),
        }
    }

    // ---------- Test 5: Cancelled-Member -> HTTP 409 Conflict (D-16-10) ----------
    #[tokio::test]
    async fn test_partial_repayment_cancelled_member_blocked() {
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();
        let exit = time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, Some(exit)))));

        // No DAO writes; no phase lookup either (exit_date check is BEFORE phase lookup).
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao.expect_create().times(0);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(member_id, 2, willensbekundung, Authentication::Full, None)
            .await;

        // D-16-10: MUST be Conflict (HTTP 409), NOT ValidationError (HTTP 400).
        assert!(
            matches!(result, Err(ServiceError::Conflict(_))),
            "expected ServiceError::Conflict (D-16-10 -> HTTP 409, distinct from Phase 15 UPGD-04 which uses ValidationError), got {:?}",
            result
        );
    }

    // ---------- Test 6: Sum-Check Violation ----------
    #[tokio::test]
    async fn test_partial_repayment_sum_check_violation() {
        // Member has 5 shares; one existing Open entry with 3 -> sum=3.
        // Requesting +3 more -> 3+3=6 > 5 -> ValidationError.
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();
        let target_fy = h1_target_fy();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        let phase = sample_repayment_phase(target_fy, 12000);
        let phase_id = phase.id;
        let phases: Arc<[RepaymentPhaseEntity]> = Arc::from(vec![phase]);
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao
            .expect_all()
            .returning(move |_| Ok(phases.clone()));
        repayment_phase_dao.expect_create().times(0);

        let existing_entry =
            sample_repayment_entry(member_id, phase_id, 3, RepaymentEntryStatus::Open);
        let existing: Arc<[RepaymentEntryEntity]> = Arc::from(vec![existing_entry]);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao
            .expect_find_by_member_and_phase()
            .returning(move |_, _, _| Ok(existing.clone()));
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(member_id, 3, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::ValidationError(errs)) => {
                assert!(
                    errs.iter().any(|e| e.message.contains("sum of open repayments")),
                    "expected message to mention sum-of-open-repayments, got: {:?}",
                    errs
                );
            }
            other => panic!("Expected ValidationError for sum-check, got {:?}", other),
        }
    }

    // ---------- Test 7: Auto-Create uses previous-phase share_value ----------
    #[tokio::test]
    async fn test_partial_repayment_auto_create_uses_previous_share_value() {
        // No phase for target FY (H2 -> next year); but a previous phase exists with
        // share_value=20000 -> auto-created phase must use 20000 (D-16-05).
        let member_id = Uuid::new_v4();
        let willensbekundung = h2_test_date();
        let target_fy = h2_target_fy();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        // Previous phase for an EARLIER fiscal_year (target_fy - 1) with share_value=20000.
        let prev_phase = sample_repayment_phase(target_fy - 1, 20000);
        let phases: Arc<[RepaymentPhaseEntity]> = Arc::from(vec![prev_phase]);
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao
            .expect_all()
            .returning(move |_| Ok(phases.clone()));
        // Auto-create must run exactly once with share_value=20000 and status=Open.
        repayment_phase_dao
            .expect_create()
            .withf(move |entity, process, _| {
                process == "repayment-phase.create"
                    && entity.share_value == 20000
                    && entity.status == RepaymentPhaseStatus::Open
                    && entity.fiscal_year == target_fy
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        let empty_entries: Arc<[RepaymentEntryEntity]> =
            Arc::from(Vec::<RepaymentEntryEntity>::new());
        repayment_entry_dao
            .expect_find_by_member_and_phase()
            .returning(move |_, _, _| Ok(empty_entries.clone()));
        repayment_entry_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let (_member, _entry, phase_opt) = service
            .partial_repayment(member_id, 2, willensbekundung, Authentication::Full, None)
            .await
            .expect("auto-create with previous share_value must succeed");

        let phase = phase_opt.expect("phase auto-created => Some(_)");
        assert_eq!(phase.share_value, 20000);
        assert_eq!(phase.fiscal_year, target_fy);
    }

    // ---------- Test 8: Auto-Create falls back to DEFAULT_SHARE_VALUE_CENT ----------
    #[tokio::test]
    async fn test_partial_repayment_auto_create_fallback_default_share_value() {
        // No phases at all -> auto-created phase uses DEFAULT_SHARE_VALUE_CENT=10000.
        let member_id = Uuid::new_v4();
        let willensbekundung = h2_test_date();
        let target_fy = h2_target_fy();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        let empty_phases: Arc<[RepaymentPhaseEntity]> =
            Arc::from(Vec::<RepaymentPhaseEntity>::new());
        repayment_phase_dao
            .expect_all()
            .returning(move |_| Ok(empty_phases.clone()));
        repayment_phase_dao
            .expect_create()
            .withf(move |entity, process, _| {
                process == "repayment-phase.create"
                    && entity.share_value == DEFAULT_SHARE_VALUE_CENT
                    && entity.status == RepaymentPhaseStatus::Open
                    && entity.fiscal_year == target_fy
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        let empty_entries: Arc<[RepaymentEntryEntity]> =
            Arc::from(Vec::<RepaymentEntryEntity>::new());
        repayment_entry_dao
            .expect_find_by_member_and_phase()
            .returning(move |_, _, _| Ok(empty_entries.clone()));
        repayment_entry_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let (_member, _entry, phase_opt) = service
            .partial_repayment(member_id, 1, willensbekundung, Authentication::Full, None)
            .await
            .expect("fallback auto-create must succeed");

        let phase = phase_opt.expect("phase auto-created => Some(_)");
        assert_eq!(phase.share_value, DEFAULT_SHARE_VALUE_CENT);
    }

    // ---------- Test 9: Permission Denied -> NO DAO touches ----------
    #[tokio::test]
    async fn test_partial_repayment_permission_denied() {
        let willensbekundung = h1_test_date();

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_current_user_id()
            .returning(|_| Ok(Some("user".to_string())));
        permission_service
            .expect_check_permission()
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        // No DAO touches at all.
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao.expect_create().times(0);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            MockTestMemberDao::new(),
            MockTestMemberActionDao::new(),
            MockTestAuditLogDao::new(),
            permission_service,
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(Uuid::new_v4(), 2, willensbekundung, Authentication::Full, None)
            .await;

        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "expected PermissionDenied, got {:?}",
            result
        );
    }

    // ---------- Test 10: PaidOut entries excluded from sum-check (D-16-09) ----------
    #[tokio::test]
    async fn test_partial_repayment_paid_out_entries_excluded_from_sum() {
        // Member has 5 shares. Existing PaidOut entry with share_count=10 (would
        // block if counted). Sum-check must exclude PaidOut so sum_open = 0, and
        // a new entry with shares=3 must be created.
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();
        let target_fy = h1_target_fy();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 5, None))));

        let phase = sample_repayment_phase(target_fy, 12000);
        let phase_id = phase.id;
        let phases: Arc<[RepaymentPhaseEntity]> = Arc::from(vec![phase]);
        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao
            .expect_all()
            .returning(move |_| Ok(phases.clone()));
        repayment_phase_dao.expect_create().times(0);

        let paid_out_entry =
            sample_repayment_entry(member_id, phase_id, 10, RepaymentEntryStatus::PaidOut);
        let existing: Arc<[RepaymentEntryEntity]> = Arc::from(vec![paid_out_entry]);
        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        repayment_entry_dao
            .expect_find_by_member_and_phase()
            .returning(move |_, _, _| Ok(existing.clone()));
        repayment_entry_dao
            .expect_create()
            .withf(|entity, process, _| {
                process == "member-adjust.partial-repayment" && entity.share_count_to_pay_out == 3
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let (_member, entry, phase_opt) = service
            .partial_repayment(member_id, 3, willensbekundung, Authentication::Full, None)
            .await
            .expect("PaidOut-exclusion must allow this request");

        assert_eq!(entry.share_count_to_pay_out, 3);
        assert!(phase_opt.is_none(), "existing phase reused");
    }

    // ---------- Test 11: Closed phase rejected (Phase 16.05 / CR-01) ----------
    // Phase 16.05 / CR-01 — Closed phase MUST be rejected with ServiceError::Conflict.
    // Service must not call repayment_phase_dao.create (no auto-create on Closed) and
    // not call repayment_entry_dao.create (no entry write). The guard short-circuits
    // BEFORE the sum-check, so find_by_member_and_phase must not be called either.
    #[tokio::test]
    async fn test_partial_repayment_rejects_closed_phase() {
        let member_id = Uuid::new_v4();
        let willensbekundung = h1_test_date();
        let target_fy = h1_target_fy();

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(sample_member_entity_with_shares(member_id, 3, None))));

        // Existing phase for target_fy in status Closed (Phase 14 prep workflow finished).
        let mut closed_phase = sample_repayment_phase(target_fy, 10000);
        closed_phase.status = RepaymentPhaseStatus::Closed;
        let phases: Arc<[RepaymentPhaseEntity]> = Arc::from(vec![closed_phase]);

        let mut repayment_phase_dao = MockTestRepaymentPhaseDao::new();
        repayment_phase_dao
            .expect_all()
            .returning(move |_| Ok(phases.clone()));
        // Guard short-circuits BEFORE auto-create.
        repayment_phase_dao.expect_create().times(0);

        let mut repayment_entry_dao = MockTestRepaymentEntryDao::new();
        // Guard short-circuits BEFORE sum-check + entry write.
        repayment_entry_dao.expect_find_by_member_and_phase().times(0);
        repayment_entry_dao.expect_create().times(0);

        let service = build_service_part(
            member_dao,
            MockTestMemberActionDao::new(),
            allow_audit_log(),
            allow_admin_perms(),
            setup_tx_dao(),
            repayment_phase_dao,
            repayment_entry_dao,
        );

        let result = service
            .partial_repayment(member_id, 1, willensbekundung, Authentication::Full, None)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                let text = msg.to_string();
                assert!(
                    text.contains("closed"),
                    "expected message to mention 'closed', got: {}",
                    text
                );
                assert!(
                    text.contains(&target_fy.to_string()),
                    "expected message to mention fiscal_year {}, got: {}",
                    target_fy,
                    text
                );
            }
            Err(other) => panic!("expected ServiceError::Conflict, got {:?}", other),
            Ok(_) => panic!("expected ServiceError::Conflict, got Ok(_)"),
        }
    }
}
