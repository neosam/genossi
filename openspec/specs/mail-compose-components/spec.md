## Purpose

Reusable mail compose UI components -- extracted from the mail page to enable sharing between bulk mail compose and inbox reply forms.

## Requirements

### Requirement: Reusable mail subject input component
The system SHALL provide a `MailSubjectInput` component in `component/mail_compose/` that renders a labeled text input for the mail subject. The component SHALL accept `value` and `on_change` props.

#### Scenario: Used on mail page
- **WHEN** the mail compose page is rendered
- **THEN** it uses the shared `MailSubjectInput` component for the subject field

#### Scenario: Used on inbox reply
- **WHEN** the inbox reply form is rendered
- **THEN** it uses the same `MailSubjectInput` component, pre-filled with `Re: {original_subject}`

### Requirement: Reusable mail body editor component
The system SHALL provide a `MailBodyEditor` component that renders a labeled textarea for the mail body. The component SHALL accept `value` and `on_change` props.

#### Scenario: Consistent styling
- **WHEN** the body editor is rendered on both mail page and inbox page
- **THEN** the textarea has identical styling and behavior

### Requirement: Reusable template variable buttons component
The system SHALL provide a `TemplateVarButtons` component that renders clickable buttons for inserting template variables (e.g., `{{ first_name }}`) into the mail body. The component SHALL accept an `on_insert` callback prop. The component SHALL support primary and secondary variable sets with a "Mehr/Weniger" toggle.

#### Scenario: Insert variable
- **WHEN** the user clicks the "Vorname" button
- **THEN** the `on_insert` callback is called with `"{{ first_name }}"`

#### Scenario: Toggle secondary variables
- **WHEN** the user clicks "Mehr"
- **THEN** additional variable buttons (street, postal_code, etc.) become visible

### Requirement: Reusable template selector component
The system SHALL provide a `TemplateSelector` component that renders a dropdown to select predefined mail templates (formal, informal). The component SHALL accept an `on_select` callback that receives the template body text.

#### Scenario: Select formal template
- **WHEN** the user selects "Formell" from the dropdown
- **THEN** the `on_select` callback is called with the formal template text

### Requirement: Reusable template preview component
The system SHALL provide a `TemplatePreview` component that allows selecting a member and displaying a rendered preview of the mail. The component SHALL accept `subject`, `body`, and `member_ids` props.

#### Scenario: Preview with member
- **WHEN** the user selects a member from the preview dropdown
- **THEN** the component calls the preview API and displays the rendered subject and body

#### Scenario: No members available
- **WHEN** no member IDs are provided to the component
- **THEN** the component shows a hint that no members are available for preview

### Requirement: MailPage uses extracted components
The system SHALL refactor `mail_page.rs` to use the extracted components from `component/mail_compose/`. The MailPage MUST NOT contain inline implementations of subject input, body editor, template variables, template selector, or template preview after refactoring.

#### Scenario: No functional change
- **WHEN** the mail page is used after refactoring
- **THEN** all existing functionality (compose, send, preview, templates, attachments) works identically as before
