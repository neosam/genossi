## ADDED Requirements

### Requirement: Render template with application data
The system SHALL provide a REST endpoint `POST /api/templates/render-application/{path}/{application_id}` that renders a Typst template with application data and returns a PDF.

#### Scenario: Render template for open application
- **WHEN** a board member sends `POST /api/templates/render-application/zahlungsaufforderung.typ/{application_id}` with a valid application ID
- **THEN** the system SHALL fetch the application data, pass it to the Typst template under the key `application`, and return a PDF with `Content-Type: application/pdf`

#### Scenario: Application not found
- **WHEN** a board member sends a render request with a non-existent application ID
- **THEN** the system SHALL return HTTP 404

#### Scenario: Template compilation error
- **WHEN** a board member renders a template that contains Typst syntax errors
- **THEN** the system SHALL return an error response with the compilation error messages

### Requirement: Application data structure for templates
The system SHALL provide application data to Typst templates as a JSON-encoded string under `sys.inputs.at("application")`. The JSON SHALL contain the fields: `first_name`, `last_name`, `salutation` (optional), `email` (optional), `street` (optional), `house_number` (optional), `postal_code` (optional), `city` (optional), `shares` (integer), `status` (string), and `created` (date formatted as "DD.MM.YYYY"). The system SHALL also provide `sys.inputs.at("today")` with the current date formatted as "DD.MM.YYYY".

#### Scenario: Template accesses application fields
- **WHEN** a Typst template contains `#let app = json.decode(sys.inputs.at("application"))`
- **THEN** `app.first_name`, `app.last_name`, `app.shares`, `app.created`, and `app.status` SHALL be available as valid values

#### Scenario: Optional fields are null when not set
- **WHEN** an application was created without street, house_number, postal_code, or city
- **THEN** the corresponding fields in the JSON SHALL be `null`

### Requirement: Render endpoint requires manage_members permission
The render-application endpoint SHALL require the `manage_members` permission, consistent with the existing member render endpoint.

#### Scenario: Unauthorized access
- **WHEN** a user without `manage_members` permission sends a render-application request
- **THEN** the system SHALL return HTTP 403
