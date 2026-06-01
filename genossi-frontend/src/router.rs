use dioxus::prelude::*;

pub use crate::page::inbox_page::InboxDetail;
pub use crate::page::mail_page::MailJobDetail;
pub use crate::page::ApplicationsPage;
pub use crate::page::Assemblies;
pub use crate::page::AssemblyDetails;
pub use crate::page::AuditLogPage;
pub use crate::page::BackupPage;
pub use crate::page::ConfigPage;
pub use crate::page::HelperAttendance;
pub use crate::page::HelperLogin;
pub use crate::page::Home;
pub use crate::page::InboxPage;
pub use crate::page::MailPage;
pub use crate::page::MailTemplatesPage;
pub use crate::page::MemberDetails;
pub use crate::page::Members;
pub use crate::page::Permissions;
// ─── Phase 12 ─── repayment pages ───────────────────────────────────
pub use crate::page::RepaymentPhaseDetails;
pub use crate::page::RepaymentPhases;
pub use crate::page::StaticDocumentsPage;
pub use crate::page::Templates;
pub use crate::page::Validation;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    // ─── Phase 4 ─── Helper routes (no auth-wrapper, see app.rs branching)
    #[route("/helper")]
    HelperLogin {},
    #[route("/helper/attendance")]
    HelperAttendance {},
    // ─── Phase 4 ─── Vorstand assembly routes (admin-gated in Plan 04-08)
    #[route("/assemblies")]
    Assemblies {},
    #[route("/assemblies/:id")]
    AssemblyDetails { id: String },
    // ── Phase 12 ── Anteils-Rückzahlung (Vorstand-only, admin-gated über RequirePrivilege in der Page) ──
    #[route("/repayment-phases")]
    RepaymentPhases {},
    #[route("/repayment-phases/:id")]
    RepaymentPhaseDetails { id: String },
    // ─── existing routes ─────────────────────────
    #[route("/members")]
    Members {},
    #[route("/members/:id")]
    MemberDetails { id: String },
    #[route("/permissions")]
    Permissions {},
    #[route("/validation")]
    Validation {},
    #[route("/templates")]
    Templates {},
    #[route("/applications")]
    ApplicationsPage {},
    #[route("/config")]
    ConfigPage {},
    #[route("/mail")]
    MailPage {},
    #[route("/mail/templates")]
    MailTemplatesPage {},
    #[route("/mail/jobs/:id")]
    MailJobDetail { id: String },
    #[route("/inbox")]
    InboxPage {},
    #[route("/inbox/:id")]
    InboxDetail { id: String },
    #[route("/documents")]
    StaticDocumentsPage {},
    #[route("/backup")]
    BackupPage {},
    #[route("/audit")]
    AuditLogPage {},
}
