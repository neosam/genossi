pub mod attachment_picker;
// Phase 24 Plan 03 Task 6: body_editor.rs deleted; MailBodyEditor replaced
// project-wide by WysiwygEditor (contenteditable-based rich-text component).
pub mod mail_preview_frame;
pub mod subject_input;
pub mod template_preview;
pub mod template_selector;
pub mod template_tester;
pub mod template_var_buttons;
pub mod wysiwyg_editor;
pub mod wysiwyg_link_dialog;
pub mod wysiwyg_toolbar;

pub use attachment_picker::MailAttachmentPicker;
pub use mail_preview_frame::MailPreviewFrame;
pub use subject_input::MailSubjectInput;
pub use template_preview::TemplatePreview;
// Quick 261102-di4: ComposeSource (Textquelle des Compose-Bodys, loest die
// Doppelrolle des alten selected_template_id-Signals auf) und die Nutzlast des
// neuen on_select_sent_job-Callbacks. Die reinen Helper-Funktionen bleiben
// ueber den Modulpfad erreichbar.
pub use template_selector::{ComposeSource, SentMailSelection, TemplateSelector};
pub use template_tester::TemplateTester;
pub use template_var_buttons::TemplateVarButtons;
pub use wysiwyg_editor::{plain_to_html, WysiwygEditor};
