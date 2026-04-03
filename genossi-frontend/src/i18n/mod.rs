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
    lang_lower == "de"
        || lang_lower.starts_with("de-")
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
    NotAuthenticated,
    WelcomeTitle,
    PleaseLogin,
    AccessDenied,

    // Navigation
    Home,
    Members,
    Permissions,

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

    // Action Types
    ActionEintritt,
    ActionAustritt,
    ActionTodesfall,
    ActionAufstockung,
    ActionVerkauf,
    ActionUebertragungEmpfang,
    ActionUebertragungAbgabe,

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
    Preview,
    SaveTemplate,
    DeleteTemplate,
    ConfirmDeleteTemplate,
    TemplatePath,
    NoTemplates,
    RenderPdf,
    SelectMember,
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

    // Member filter
    NotReachedByMailJob,
    AllMembers,

    // Messages
    NoDataFound,
    ErrorLoadingData,
    ConfirmDelete,
    DeleteMemberConfirmTitle,
    Confirm,
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
}

impl Clone for I18n {
    fn clone(&self) -> Self {
        Self {
            locale: self.locale,
        }
    }
}

static I18N: GlobalSignal<I18n> = GlobalSignal::new(|| I18n::new(detect_browser_locale()));

pub fn use_i18n() -> I18n {
    I18N.read().clone()
}
