pub mod attachment_picker;
// Phase 24 Plan 03 Task 6: body_editor.rs deleted; MailBodyEditor replaced
// project-wide by WysiwygEditor (contenteditable-based rich-text component).
pub mod subject_input;
pub mod template_preview;
pub mod template_selector;
pub mod template_tester;
pub mod template_var_buttons;
pub mod wysiwyg_editor;
pub mod wysiwyg_link_dialog;
pub mod wysiwyg_toolbar;

pub use attachment_picker::MailAttachmentPicker;
pub use subject_input::MailSubjectInput;
pub use template_preview::TemplatePreview;
pub use template_selector::TemplateSelector;
pub use template_tester::TemplateTester;
pub use template_var_buttons::TemplateVarButtons;
pub use wysiwyg_editor::WysiwygEditor;
