## MODIFIED Requirements

### Requirement: Action create/edit form
The member detail page SHALL provide an inline form to create and edit actions with fields for action_type, date, shares_change, transfer_member_id, effective_date, and comment. The shares_change field SHALL only accept positive values and the frontend SHALL automatically apply the correct sign based on the action type before submitting to the API.

#### Scenario: Create new action
- **WHEN** the user fills in the action form and clicks save
- **THEN** the system SHALL create the action via API and refresh the actions list

#### Scenario: Conditional fields based on action type
- **WHEN** the user selects a status action type (Eintritt, Austritt, Todesfall)
- **THEN** the shares_change field SHALL be hidden (fixed to 0)

#### Scenario: Shares field hidden for Note actions
- **WHEN** the user selects Note as action type
- **THEN** the shares_change field SHALL be hidden (fixed to 0)

#### Scenario: Positive-only shares input for Aufstockung
- **WHEN** the user selects Aufstockung as action type
- **THEN** the shares_change field SHALL be shown with label "Anteile hinzufügen" (i18n: SharesAdd), accept only positive integers (min=1), and submit the value as-is (positive)

#### Scenario: Positive-only shares input for Verkauf
- **WHEN** the user selects Verkauf as action type
- **THEN** the shares_change field SHALL be shown with label "Anteile abgeben" (i18n: SharesRemove), accept only positive integers (min=1), and submit the value negated (negative)

#### Scenario: Positive-only shares input for Übertragung Empfang
- **WHEN** the user selects UebertragungEmpfang as action type
- **THEN** the shares_change field SHALL be shown with label "Anteile empfangen" (i18n: SharesReceive), accept only positive integers (min=1), and submit the value as-is (positive)

#### Scenario: Positive-only shares input for Übertragung Abgabe
- **WHEN** the user selects UebertragungAbgabe as action type
- **THEN** the shares_change field SHALL be shown with label "Anteile übertragen" (i18n: SharesTransfer), accept only positive integers (min=1), and submit the value negated (negative)

#### Scenario: Transfer fields
- **WHEN** the user selects UebertragungEmpfang or UebertragungAbgabe
- **THEN** the transfer_member_id field SHALL be shown and required

#### Scenario: Effective date for Austritt
- **WHEN** the user selects Austritt as action type
- **THEN** the effective_date field SHALL be shown

#### Scenario: Edit existing action with negative shares_change
- **WHEN** the user clicks an action with a negative shares_change value (e.g., Verkauf with -3)
- **THEN** the form SHALL display the absolute value (3) in the shares_change field, and re-apply the negative sign on save

#### Scenario: Edit existing action with positive shares_change
- **WHEN** the user clicks an action with a positive shares_change value (e.g., Aufstockung with 5)
- **THEN** the form SHALL display the value (5) in the shares_change field

## ADDED Requirements

### Requirement: Dynamic shares field labels
The shares_change input field SHALL display a context-specific label based on the selected action type, using i18n keys for all supported languages (DE, EN, CS).

#### Scenario: Label for Aufstockung
- **WHEN** action type is Aufstockung
- **THEN** the label SHALL display the i18n key SharesAdd ("Anteile hinzufügen" in DE)

#### Scenario: Label for Verkauf
- **WHEN** action type is Verkauf
- **THEN** the label SHALL display the i18n key SharesRemove ("Anteile abgeben" in DE)

#### Scenario: Label for UebertragungEmpfang
- **WHEN** action type is UebertragungEmpfang
- **THEN** the label SHALL display the i18n key SharesReceive ("Anteile empfangen" in DE)

#### Scenario: Label for UebertragungAbgabe
- **WHEN** action type is UebertragungAbgabe
- **THEN** the label SHALL display the i18n key SharesTransfer ("Anteile übertragen" in DE)
