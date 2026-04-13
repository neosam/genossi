## MODIFIED Requirements

### Requirement: Predefined mail templates
The predefined mail templates (Formell/Informell) SHALL contain only the salutation/greeting line without a closing formula. The closing formula (e.g. "Mit freundlichen Grüßen", "Viele Grüße") SHALL be provided by the mail footer instead.

#### Scenario: Formal template content
- **WHEN** user selects the "Formell" template
- **THEN** the template contains the formal salutation (e.g. "Sehr geehrter Herr...") followed by empty lines for the body, but no closing formula

#### Scenario: Informal template content
- **WHEN** user selects the "Informell" template
- **THEN** the template contains the informal greeting (e.g. "Lieber/Liebe...") followed by empty lines for the body, but no closing formula
