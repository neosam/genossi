pub mod de;
pub mod en;

use dioxus::prelude::*;
use std::rc::Rc;
use web_sys;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    De,
}

impl Default for Locale {
    fn default() -> Self {
        Self::En
    }
}

fn detect_browser_locale() -> Locale {
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        if let Some(language) = navigator.language() {
            if is_german_language(&language) {
                return Locale::De;
            }
        }
        let languages = navigator.languages();
        for i in 0..languages.length() {
            if let Some(lang) = languages.get(i).as_string() {
                if is_german_language(&lang) {
                    return Locale::De;
                }
            }
        }
    }
    Locale::En
}

fn is_german_language(lang: &str) -> bool {
    let lang_lower = lang.to_lowercase();
    lang_lower == "de" || lang_lower.starts_with("de-")
}

#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    // General
    AppTitle,
    Loading,
    Save,
    Cancel,
    Delete,
    Edit,
    Create,
    Search,
    Back,

    // Authentication
    Login,
    Logout,
    RevokeAllSessions,
    RevokeSessionsConfirmTitle,
    RevokeSessionsConfirmText,
    RevokeSessions,
    SessionsRevoked,
    Sessions,
    NotAuthenticated,
    WelcomeTitle,
    PleaseLogin,
    AccessDenied,

    // Navigation
    Home,
    Members,
    Permissions,
    NavAdministration,

    // Member fields
    MemberNumber,
    FirstName,
    LastName,
    Email,
    Company,
    Comment,
    Street,
    HouseNumber,
    PostalCode,
    City,
    JoinDate,
    SharesAtJoining,
    CurrentShares,
    CurrentBalance,
    ExitDate,
    BankAccount,
    Salutation,
    Title,
    CreateMember,
    EditMember,

    // Member Actions
    Actions,
    ActionType,
    Date,
    SharesChange,
    TransferMember,
    EffectiveDate,
    NewAction,
    EditAction,
    NoActions,

    SharesAdd,
    SharesRemove,
    SharesReceive,
    SharesTransfer,

    // Action Types
    ActionEintritt,
    ActionAustritt,
    ActionTodesfall,
    ActionAufstockung,
    ActionVerkauf,
    ActionUebertragungEmpfang,
    ActionUebertragungAbgabe,
    ActionNote,

    // Member Status
    MemberStatus,
    MemberStatusNormal,
    MemberStatusFehlerhaftErfasst,

    // Migration Status
    MigrationStatus,
    Migrated,
    Pending,
    ExpectedShares,
    ActualShares,
    ExpectedActionCount,
    ActualActionCount,

    ConfirmMigration,

    // Documents
    Documents,
    Upload,
    DocumentType,
    Description,
    FileName,
    Download,
    DocJoinDeclaration,
    DocJoinConfirmation,
    DocShareIncrease,
    DocOther,
    NoDocuments,
    UploadDocument,
    Uploaded,
    GenerateAndStore,
    DocumentUploadColumn,
    DocumentAlreadyExists,
    Uploading,
    UploadFailed,
    SelectDocumentType,

    ReferenceDate,
    Active,
    Inactive,
    OnlyActiveMembers,
    ExitedInYear,
    OnlyPendingMigration,

    // Validation
    Validation,
    ValidationNoIssues,
    MemberNumberGaps,
    MissingNumbers,
    UnmatchedTransfers,
    TransferMemberNumber,
    SharesMismatches,
    Expected,
    Actual,
    MissingEntryActions,
    EntryActionCount,
    ExitDateMismatches,
    HasExitDate,
    HasAustrittAction,
    Yes,
    No,
    ActiveMembersNoShares,
    DuplicateMemberNumbers,
    ExitedMembersWithShares,
    MigratedFlagMismatches,
    FlagValue,
    ComputedStatus,

    // Templates
    Templates,
    TemplateEditor,
    NewFile,
    NewFolder,
    UploadFile,
    Preview,
    SaveTemplate,
    DeleteTemplate,
    ConfirmDeleteTemplate,
    TemplatePath,
    NoTemplates,
    RenderPdf,
    SelectMember,
    SelectApplication,
    PreviewMember,
    PreviewApplication,
    GenerateDocument,
    SelectTemplate,
    TemplateRenderError,
    UnsavedChanges,
    UnsavedChangesWarning,
    Discard,

    // Config
    Config,
    ConfigKey,
    ConfigValue,
    ConfigValueType,
    ConfigAddEntry,
    ConfigNoEntries,
    ConfigDeleteConfirm,
    ConfigTypeString,
    ConfigTypeInt,
    ConfigTypeBool,
    ConfigTypeSecret,

    // Mail
    Mail,
    MailCompose,
    MailTo,
    MailSubject,
    MailBody,
    MailSend,
    MailSending,
    MailSent,
    MailFailed,
    MailHistory,
    MailNoHistory,
    MailStatus,
    MailError,
    MailSentAt,
    MailSentSuccess,
    MailSentFailed,
    MailJobs,
    MailJobProgress,
    MailJobDone,
    MailJobRunning,
    MailJobFailed,
    MailJobPending,
    MailRetry,
    MailRecipients,
    MailJobCreated,
    MailTemplateVariables,
    MailTemplateMore,
    MailTemplatePreview,
    MailTemplatePreviewSelect,
    MailTemplateError,

    // SMTP Settings
    SmtpSettings,
    SmtpHost,
    SmtpPort,
    SmtpEncryption,
    SmtpEncryptionNone,
    SmtpEncryptionStarttls,
    SmtpEncryptionTls,
    SmtpUser,
    SmtpPassword,
    SmtpFrom,
    SmtpFromName,
    SmtpTestMail,
    SmtpTestMailTo,
    SmtpTestSuccess,
    SmtpTestFailed,
    SmtpSaving,
    AdvancedConfig,

    // Member selection
    SelectedCount,
    SendMailToSelected,

    // Mail from member detail
    MailSendButton,
    NoEmailAddressHint,

    // Mail templates
    MailTemplateFormal,
    MailTemplateInformal,
    MailTemplateSelect,
    MailTemplates,
    MailTemplateCreate,
    MailTemplateSave,
    MailTemplateDelete,
    MailTemplateManage,
    MailTemplateName,
    MailTemplateSubject,
    MailTemplateBody,
    MailTemplateEmpty,
    MailTemplateDeleteConfirm,

    // Member filter
    NotReachedByMailJob,
    AllMembers,

    // Column picker / inline edit
    Columns,
    Done,

    // Communication timeline
    Communication,
    CommunicationInbound,
    CommunicationOutbound,
    CommunicationNone,
    CommunicationStatusSent,
    CommunicationStatusPending,
    CommunicationStatusFailed,
    CommunicationStatusDone,
    CommunicationStatusReplied,
    CommunicationStatusArchived,

    // Backup
    Backup,
    BackupMemberList,
    BackupMemberListDescription,
    BackupActions,
    BackupActionsDescription,
    BackupDocuments,
    BackupDocumentsDescription,
    BackupDocumentsWarning,
    BackupCutoffDate,

    // WebDAV Backup Config
    WebDavBackup,
    WebDavUrl,
    WebDavUrlPlaceholder,
    WebDavUsername,
    WebDavPassword,
    WebDavDirectory,
    WebDavDirectoryPlaceholder,
    WebDavIntervalHours,
    WebDavEnabled,
    WebDavSaving,
    WebDavLastBackup,
    WebDavNoBackupYet,
    WebDavPasswordSet,
    WebDavTestConnection,
    WebDavTestSuccess,
    WebDavTestFailed,

    // Applications
    Applications,
    ApplicationsDesc,
    StatusOffen,
    StatusBestaetigt,
    StatusAbgelehnt,
    StatusAll,
    NoApplications,
    ConfirmApplication,
    RejectApplication,
    ConfirmApplicationHint,
    RejectApplicationHint,
    ApplicationDetails,
    Shares,
    SubmittedAt,
    ApplicationConfirmed,
    ApplicationRejected,
    CreateApplication,
    CreateApplicationDesc,
    EditApplication,
    SendConfirmationMail,

    // WordPress Integration
    WordPressIntegration,
    WordPressIntegrationDesc,
    GenerateApiKey,
    RegenerateApiKey,
    ApiKeyGenerated,
    ApiKeyCopyHint,
    ApiKeyConfigured,
    ShareValueCents,
    BankIban,
    BankNameConfig,
    BankBic,
    GenossenschaftName,
    SetupInstructions,
    ApiUrl,
    ConfigComplete,
    ConfigIncomplete,
    MissingFields,
    WordPressShortcodeHint,
    CopyToClipboard,
    Copied,
    WpStep1,
    WpStep2,
    WpStep3,
    Generating,
    BankDetails,
    CooperativeSettings,

    // Audit Log
    AuditLog,
    AuditTimestamp,
    AuditUser,
    AuditAction,
    AuditEntityType,
    AuditEntityId,
    AuditFieldName,
    AuditOldValue,
    AuditNewValue,
    AuditVerifyChain,
    AuditVerifySuccess,
    AuditVerifyFailure,
    AuditChainValid,
    AuditChainInvalid,
    AuditBrokenLinks,
    AuditTotalEntries,
    AuditNoEntries,
    AuditFilterEntityType,
    AuditFilterUser,
    AuditFilterAction,
    AuditFilterFrom,
    AuditFilterTo,
    AuditActionCreate,
    AuditActionUpdate,
    AuditActionDelete,
    AuditActionSnapshot,

    // Pagination
    PaginationFirst,
    PaginationPrev,
    PaginationNext,
    PaginationLast,
    PageSize,
    PageOfTotal,
    TotalEntries,

    // Qualified Timestamps
    TimestampTitle,
    TimestampCreateButton,
    TimestampCreating,
    TimestampList,
    TimestampDate,
    TimestampHash,
    TimestampEntryCount,
    TimestampStatus,
    TimestampStatusSuccess,
    TimestampStatusFailed,
    TimestampStatusUploadFailed,
    TimestampVerify,
    TimestampVerifying,
    TimestampTokenValid,
    TimestampTokenInvalid,
    TimestampHashMatches,
    TimestampHashMismatch,
    TimestampAuditConsistent,
    TimestampAuditInconsistent,
    TimestampNoTimestamps,
    TimestampCreated,
    TimestampNoChanges,
    TimestampNotConfigured,
    TimestampTsaConfig,
    TimestampTsaUrl,
    TimestampTsaUser,
    TimestampTsaPass,
    TimestampTsaEnabled,
    TimestampTsaInterval,

    // Status Bar
    OpenApplicationsCount,
    OpenApplicationsNone,
    OpenInboxCount,
    OpenInboxNone,

    // Messages
    NoDataFound,
    ErrorLoadingData,
    ConfirmDelete,
    DeleteMemberConfirmTitle,
    Confirm,

    // ─── Phase 4 ─── Generic UI keys (used across components/pages) ──
    Close,

    // ─── Phase 4 ─── Assembly (Vorstand pages) ──────────────────────
    Assemblies,
    Assembly,
    AssemblyCreate,
    AssemblyName,
    AssemblyDate,
    AssemblyLocation,
    AssemblyOpen,
    AssemblyClose,
    AssemblyStatusPreparation,
    AssemblyStatusOpen,
    AssemblyStatusClosed,
    AssemblyEmpty,
    AssemblyEmptyHint,
    AssemblyOpenConfirmTitle,
    AssemblyOpenConfirmText,
    AssemblyCloseConfirmTitle,
    AssemblyCloseConfirmText,
    AssemblyTabBasics,
    AssemblyTabTokens,
    AssemblyTabAttendance,
    AssemblyAttendanceNotOpenYet,

    // ─── Phase 4 ─── Helper Tokens (Vorstand-Tokens-Tab) ────────────
    HelperTokens,
    HelperTokenCreate,
    HelperTokenMemo,
    HelperTokenMemoPlaceholder,
    HelperTokenStatusOpen,
    HelperTokenStatusUsed,
    HelperTokenStatusRevoked,
    HelperTokenRevoke,
    HelperTokenPrint,
    HelperTokenCardTitle,
    HelperTokenCardManualHint,
    HelperTokenRedeemed,
    HelperTokenWarning,
    HelperTokenEmpty,
    HelperTokenEmptyHint,
    HelperTokenRevokeConfirmTitle,
    HelperTokenRevokeConfirmText,
    // ADR-2026-05-06: re-display of QR + manual code per Vorstand UX.
    HelperTokenShow,
    HelperTokenCodeMissing,
    HelperTokenLoginLink,
    Copy,

    // ─── Phase 4 ─── Helfer-Login + Manual-Code (HLPR-03) ───────────
    HelperLoginTitle,
    HelperLoginSubtitle,
    HelperLoginScanQR,
    HelperLoginScanning,
    HelperLoginManualHeading,
    HelperLoginManualPlaceholder,
    HelperLoginSubmit,
    HelperLoginCameraDenied,
    HelperLoginCameraNotAvailable,
    HelperLoginInvalidFormat,
    HelperLoginErrorNotFound,
    HelperLoginErrorAlreadyUsed,
    HelperLoginErrorAssemblyClosed,
    HelperLoginErrorRateLimit,
    HelperLoginQrFrameHint,
    HelperLoginCameraStarting,

    // ─── Phase 4 ─── Helfer-Shell ───────────────────────────────────
    HelperShellLogout,
    HelperShellAssemblyHeading,

    // ─── Phase 4 ─── Attendance shared (Helfer + Vorstand-Tab) ──────
    AttendanceSearch,
    AttendanceSearchHint,
    AttendanceCounterLong,
    AttendanceCounterLongLoading,
    AttendanceCounterUnknown,
    AttendanceCounterLabel,
    AttendanceEmpty,
    AttendanceEmptyHint,
    AttendanceTogglePresent,
    AttendanceToggleAbsent,
    AttendanceToggleSavingHint,
    AttendanceConnectionLost,
    AttendanceConnectionRestored,
}

pub struct I18n {
    locale: Locale,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn t(&self, key: Key) -> Rc<str> {
        match self.locale {
            Locale::En => en::translate(key),
            Locale::De => de::translate(key),
        }
    }

    pub fn format_date(&self, date: &time::Date) -> String {
        match self.locale {
            Locale::En => {
                format!(
                    "{:04}-{:02}-{:02}",
                    date.year(),
                    date.month() as u8,
                    date.day()
                )
            }
            Locale::De => {
                format!(
                    "{:02}.{:02}.{:04}",
                    date.day(),
                    date.month() as u8,
                    date.year()
                )
            }
        }
    }

    pub fn format_price(&self, cents: i64) -> String {
        let euros = cents as f64 / 100.0;
        match self.locale {
            Locale::En => format!("{:.2} EUR", euros),
            Locale::De => format!("{:.2} EUR", euros).replace('.', ","),
        }
    }

    /// Format an ISO8601 timestamp with minute precision, localised.
    /// Returns the input unchanged if it cannot be parsed.
    pub fn format_datetime(&self, iso: &str) -> String {
        let Some(c) = parse_iso_components(iso) else {
            return iso.to_string();
        };
        match self.locale {
            Locale::En => format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                c.year, c.month, c.day, c.hour, c.minute
            ),
            Locale::De => format!(
                "{:02}.{:02}.{:04} {:02}:{:02}",
                c.day, c.month, c.year, c.hour, c.minute
            ),
        }
    }

    /// Format an ISO8601 timestamp with second precision, localised.
    /// Returns the input unchanged if it cannot be parsed.
    pub fn format_datetime_long(&self, iso: &str) -> String {
        let Some(c) = parse_iso_components(iso) else {
            return iso.to_string();
        };
        match self.locale {
            Locale::En => format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                c.year, c.month, c.day, c.hour, c.minute, c.second
            ),
            Locale::De => format!(
                "{:02}.{:02}.{:04} {:02}:{:02}:{:02}",
                c.day, c.month, c.year, c.hour, c.minute, c.second
            ),
        }
    }
}

struct IsoComponents {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

fn parse_iso_components(s: &str) -> Option<IsoComponents> {
    let (date_part, time_part) = s.split_once('T')?;
    let mut date_it = date_part.split('-');
    let year: i32 = date_it.next()?.parse().ok()?;
    let month: u8 = date_it.next()?.parse().ok()?;
    let day: u8 = date_it.next()?.parse().ok()?;
    if date_it.next().is_some() {
        return None;
    }

    let mut time_it = time_part.split(':');
    let hour: u8 = time_it.next()?.parse().ok()?;
    let minute: u8 = time_it.next()?.parse().ok()?;
    let second_raw = time_it.next()?;
    // Seconds may be followed by fractional seconds and/or a timezone marker;
    // take only the leading integer digits.
    let second_digits: String = second_raw
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if second_digits.is_empty() {
        return None;
    }
    let second: u8 = second_digits.parse().ok()?;

    Some(IsoComponents {
        year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

impl Clone for I18n {
    fn clone(&self) -> Self {
        Self {
            locale: self.locale,
        }
    }
}

/// Global I18N signal. Default-Locale wird beim Mount via `detect_browser_locale()`
/// bestimmt, kann aber von Components überschrieben werden — Phase 4 D-19 / W-07
/// nutzt das in `HelperShell`, um die Helfer-View hart auf `Locale::De` zu fixieren.
pub static I18N: GlobalSignal<I18n> = GlobalSignal::new(|| I18n::new(detect_browser_locale()));

pub fn use_i18n() -> I18n {
    I18N.read().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ISO: &str = "2026-04-16T16:03:34.512345678Z";

    #[test]
    fn format_datetime_de_drops_fractional_seconds() {
        let i18n = I18n::new(Locale::De);
        assert_eq!(i18n.format_datetime(SAMPLE_ISO), "16.04.2026 16:03");
    }

    #[test]
    fn format_datetime_en_uses_iso_date() {
        let i18n = I18n::new(Locale::En);
        assert_eq!(i18n.format_datetime(SAMPLE_ISO), "2026-04-16 16:03");
    }

    #[test]
    fn format_datetime_long_de_includes_seconds() {
        let i18n = I18n::new(Locale::De);
        assert_eq!(i18n.format_datetime_long(SAMPLE_ISO), "16.04.2026 16:03:34");
    }

    #[test]
    fn format_datetime_long_en_includes_seconds() {
        let i18n = I18n::new(Locale::En);
        assert_eq!(i18n.format_datetime_long(SAMPLE_ISO), "2026-04-16 16:03:34");
    }

    #[test]
    fn format_datetime_accepts_timestamp_without_fraction() {
        let i18n = I18n::new(Locale::De);
        assert_eq!(
            i18n.format_datetime("2026-04-16T16:03:34Z"),
            "16.04.2026 16:03"
        );
    }

    #[test]
    fn format_datetime_returns_input_for_unparsable_string() {
        let i18n = I18n::new(Locale::De);
        assert_eq!(i18n.format_datetime("not-a-date"), "not-a-date");
        assert_eq!(i18n.format_datetime(""), "");
        assert_eq!(
            i18n.format_datetime_long("2026-04-16 16:03:34"),
            "2026-04-16 16:03:34"
        );
    }
}
