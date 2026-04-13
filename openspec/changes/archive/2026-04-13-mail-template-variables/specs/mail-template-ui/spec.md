## ADDED Requirements

### Requirement: Variable insertion buttons
The mail compose form SHALL display a row of buttons for inserting template variables into the email body. Each button SHALL insert the corresponding `{{ variable_name }}` text at the end of the body textarea when clicked.

#### Scenario: Insert first_name variable
- **WHEN** the user clicks the "first_name" variable button
- **THEN** `{{ first_name }}` is appended to the current body text

#### Scenario: Insert variable into empty body
- **WHEN** the body textarea is empty and the user clicks the "last_name" variable button
- **THEN** the body textarea contains `{{ last_name }}`

### Requirement: Variable button categories
The variable buttons SHALL be organized with the most commonly used variables shown directly and remaining variables accessible via a "Mehr" (more) dropdown or expandable section. The primary variables SHALL include: `first_name`, `last_name`, `salutation`, `title`, `member_number`, `company`.

#### Scenario: Primary variables visible
- **WHEN** the mail compose form is displayed
- **THEN** buttons for `first_name`, `last_name`, `salutation`, `title`, `member_number`, and `company` are visible

#### Scenario: Additional variables accessible
- **WHEN** the user clicks the "Mehr" button
- **THEN** buttons for all remaining member variables become visible (street, house_number, postal_code, city, join_date, shares_at_joining, current_shares, current_balance, exit_date, bank_account)

### Requirement: Variable buttons for subject field
The mail compose form SHALL also provide variable insertion for the subject field. The same variable buttons SHALL be available for both subject and body.

#### Scenario: Insert variable into subject
- **WHEN** the user clicks a variable button while the subject field is focused or via a subject-specific button area
- **THEN** `{{ variable_name }}` is appended to the current subject text

### Requirement: Template preview in compose form
The mail compose form SHALL provide a preview panel that shows the rendered email for a selected member. The user SHALL be able to select a member from the recipient list to preview the email.

#### Scenario: Preview with selected member
- **WHEN** the user selects a member from the preview member dropdown and subject/body contain template variables
- **THEN** the preview panel shows the rendered subject and body with the selected member's data

#### Scenario: Preview updates on template change
- **WHEN** the user modifies the subject or body text while a preview member is selected
- **THEN** the preview panel updates to reflect the new template content (with reasonable debounce)

#### Scenario: Preview shows errors
- **WHEN** the template contains a syntax error and a preview member is selected
- **THEN** the preview panel shows the error message instead of rendered content

#### Scenario: No preview without member selection
- **WHEN** no member is selected for preview
- **THEN** the preview panel shows a prompt to select a member for preview
