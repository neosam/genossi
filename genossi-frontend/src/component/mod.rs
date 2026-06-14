pub mod application_create_form;
pub mod application_detail;
pub mod application_form;
pub mod application_list;
pub mod application_search;
pub mod base_components;
pub mod collapsible_section;
pub mod communication_timeline;
pub mod dropdown_base;
pub mod error_alert;
pub mod error_view;
pub mod footer;
pub mod inbox;
pub mod mail_compose;
pub mod member_search;
pub mod modal;
pub mod nav_group;
pub mod overlay;
pub mod page_size_select;
pub mod pagination_controls;
pub mod revoke_sessions_button;
pub mod status_bar;
pub mod timestamp_section;
pub mod top_bar;
pub mod tsa_config;
pub mod wordpress_integration;

pub use application_create_form::ApplicationCreateForm;
pub use application_detail::ApplicationDetail;
pub use application_form::{ApplicationForm, ApplicationFormMode};
pub use application_list::ApplicationList;
pub use application_search::ApplicationSearch;
pub use collapsible_section::CollapsibleSection;
pub use communication_timeline::CommunicationTimeline;
pub use error_alert::ErrorAlert;
pub use footer::Footer;
pub use member_search::MemberSearch;
pub use modal::Modal;
pub use page_size_select::PageSizeSelect;
pub use pagination_controls::PaginationControls;
pub use revoke_sessions_button::RevokeSessionsButton;
pub use status_bar::{StatusBar, StatusBarItem};
pub use timestamp_section::TimestampSection;
pub use top_bar::TopBar;
pub use tsa_config::TsaConfigSection;
pub use wordpress_integration::WordPressIntegrationSection;

// ─── Phase 4 Plan 04 ─── shared attendance components ────────────
pub mod attendance_list;
pub mod attendance_search;
pub mod connection_banner;
pub mod live_counter;

// ─── Phase 4 Plan 05 ─── helper login components ─────────────────
pub mod helper_shell;
pub mod manual_code_input;
pub mod qr_card;
pub mod qr_scanner;

// ─── Phase 4 Plan 06 ─── vorstand layout components ──────────────
pub mod assembly_list_row;
pub mod assembly_status_badge;
pub mod tab_strip;
pub mod toast;

// ─── Phase 4 Plan 06 (W-04 extraction from assembly_details) ─────
pub mod basics_tab;
pub mod create_token_form;
pub mod token_row;

// ─── Phase 4 Plan 04 ─── shared attendance components ────────────
pub use attendance_list::{AttendanceList, AttendanceToggleRequest};
pub use attendance_search::AttendanceSearch;
pub use connection_banner::ConnectionBanner;
pub use live_counter::{ConnState, LiveCounter};

// ─── Phase 4 Plan 05 ─── helper login components ─────────────────
pub use helper_shell::HelperShell;
pub use manual_code_input::ManualCodeInput;
pub use qr_card::QrCard;
pub use qr_scanner::{decide_camera_path, CameraPath, QrScanner};

// ─── Phase 4 Plan 06 ─── vorstand layout components ──────────────
pub use assembly_list_row::AssemblyListRow;
pub use assembly_status_badge::AssemblyStatusBadge;
pub use tab_strip::{TabDef, TabStrip};
pub use toast::{show_toast, ToastContainer};

// ─── Phase 4 Plan 06 (W-04 extraction from assembly_details) ─────
pub use basics_tab::BasicsTab;
pub use create_token_form::CreateTokenForm;
pub use token_row::TokenRow;

// ─── Phase 12 ─── RepaymentPhase / RepaymentEntry helpers + badges ──
pub mod repayment_entry_status_badge;
pub mod repayment_format;
pub mod repayment_phase_status_badge;
pub use repayment_entry_status_badge::RepaymentEntryStatusBadge;
pub use repayment_phase_status_badge::RepaymentPhaseStatusBadge;

// ─── Phase 12 ─── EditableShareCountCell (D-13 Inline-Edit) ─────────
pub mod editable_share_count_cell;
pub use editable_share_count_cell::EditableShareCountCell;

// ─── Phase 12 Plan 12-08 ─── RepaymentEntryList (UI-03) ─────────────
pub mod repayment_entry_list;
pub use repayment_entry_list::{
    entry_counts_by_status, filter_entries_by_status, member_for_entry, sort_entries_default,
    RepaymentEntryList, StatusCounts, StatusFilter,
};

// ─── Phase 12 Plan 12-09 ─── RepaymentEntryAddModal (UI-04) ─────────
pub mod repayment_entry_add_modal;
pub use repayment_entry_add_modal::{validate_create_entry, RepaymentEntryAddModal};

// ─── Phase 12 Plan 12-10 ─── RepaymentEntryPaidOutConfirm (UI-05) ────
pub mod repayment_entry_paidout_confirm;
pub use repayment_entry_paidout_confirm::{sum_payout_amounts, RepaymentEntryPaidOutConfirm};

// ─── Quick 260602-sgp ─── RepaymentLetterDownloadButton ─────────────
pub mod repayment_letter_download_button;
pub use repayment_letter_download_button::RepaymentLetterDownloadButton;

// ─── Quick 260603-evf ─── MailRecipientStatusBadge ──────────────────
pub mod mail_recipient_status_badge;
pub use mail_recipient_status_badge::{is_no_repayment_letter_failure, MailRecipientStatusBadge};

// ─── Quick 260614-9zf ─── MailRecipientRenderedContent ──────────────
pub mod mail_recipient_rendered_content;
pub use mail_recipient_rendered_content::MailRecipientRenderedContent;

// ─── Quick 260614-ckn ─── MailJobsList (Job-Liste ausgelagert) ──────
pub mod mail_jobs_list;
pub use mail_jobs_list::MailJobsList;

// ─── Quick 260603-evf ─── NoRepaymentLetterAction ───────────────────
pub mod no_repayment_letter_action;
pub use no_repayment_letter_action::{
    button_label_for_state, find_entry_for_member, ButtonState, NoRepaymentLetterAction,
    NoRepaymentLetterActionProps,
};

// ─── Phase 18 ─── FiscalYearDateInput + Toast-Erweiterungen ────
pub mod fiscal_year_date_input;
pub use fiscal_year_date_input::{is_valid_fiscal_year_date, FiscalYearDateInput};

// Phase 18 — Toast-Erweiterungen aus Plan 02 (toast.rs) re-exportieren.
// `show_toast` + `ToastContainer` sind bereits oben in der Phase-4-Sektion re-exportiert.
pub use toast::{show_success_toast, SuccessToastContainer, ToastVariant};

// ─── Phase 18 ─── MembershipAdjustModal (Plan 06) ────
pub mod membership_adjust_modal;
pub use membership_adjust_modal::{
    compute_effective_date_mirror, format_date_german, is_voll_uebertrag, to_member_to,
    MembershipAdjustModal,
};
