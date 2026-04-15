## MODIFIED Requirements

### Requirement: Reusable template selector component
The system SHALL provide a `TemplateSelector` component that renders a dropdown to select mail templates. The component SHALL load templates from the API (`GET /api/mail/templates`) on mount and populate the dropdown dynamically. The component SHALL accept an `on_select` callback that receives the template body text. The component SHALL include a "Vorlagen verwalten" link that navigates to `/mail/templates`.

#### Scenario: Select template from API
- **WHEN** the mail compose form is rendered
- **THEN** the `TemplateSelector` SHALL load available templates from `GET /api/mail/templates`
- **AND** display them as options in the dropdown

#### Scenario: Select a template
- **WHEN** the user selects a template from the dropdown
- **THEN** the `on_select` callback SHALL be called with the template's body text

#### Scenario: Navigate to template management
- **WHEN** the user clicks the "Vorlagen verwalten" link in the TemplateSelector
- **THEN** the user SHALL be navigated to `/mail/templates`
