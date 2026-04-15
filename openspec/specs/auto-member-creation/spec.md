## ADDED Requirements

### Requirement: Application data model includes title
The system SHALL store applications with an optional `title` field (String). The field SHALL accept free-text values such as "Dr.", "Prof.", "Prof. Dr.".

#### Scenario: Application created with title
- **WHEN** an application is created with `title` set to "Dr."
- **THEN** the system SHALL store the application with `title` = "Dr."

#### Scenario: Application created without title
- **WHEN** an application is created without a `title` field
- **THEN** the system SHALL store the application with `title` = NULL

### Requirement: Title transferred on application confirmation
The system SHALL copy the `title` field from the application to the newly created member when an application is confirmed.

#### Scenario: Confirmed application with title
- **WHEN** an application with `title` = "Dr." is confirmed
- **THEN** the created member SHALL have `title` = "Dr."

#### Scenario: Confirmed application without title
- **WHEN** an application with `title` = NULL is confirmed
- **THEN** the created member SHALL have `title` = NULL

### Requirement: Application template rendering includes title
The `build_inputs_application()` function SHALL include the `title` field in the JSON data passed to Typst templates.

#### Scenario: Template accesses title
- **WHEN** a Typst template accesses `app.title` from an application with `title` = "Dr."
- **THEN** the value SHALL be "Dr."

#### Scenario: Template accesses null title
- **WHEN** a Typst template accesses `app.title` from an application without a title
- **THEN** the value SHALL be `null`
