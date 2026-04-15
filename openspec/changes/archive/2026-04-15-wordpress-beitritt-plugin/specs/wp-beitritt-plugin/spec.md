## ADDED Requirements

### Requirement: Plugin activation and settings
The plugin SHALL register a settings page under "Settings > Genossi Beitritt" in the WordPress admin area. The settings page SHALL provide fields for:
- `genossi_api_url` (String, required): The base URL of the Genossi API
- `genossi_api_key` (String, required): The API key for authentication

Settings SHALL be stored using the WordPress Options API.

#### Scenario: Settings page accessible
- **WHEN** an admin navigates to "Settings > Genossi Beitritt"
- **THEN** the settings page displays fields for API URL and API Key

#### Scenario: Save settings
- **WHEN** an admin enters the API URL and API Key and clicks "Save"
- **THEN** the values are stored in the WordPress options table

#### Scenario: Settings validation
- **WHEN** an admin saves the settings with an empty API URL
- **THEN** the system displays a validation error

### Requirement: Shortcode registration
The plugin SHALL register a shortcode `[genossi_beitritt]` that renders the membership application form on any WordPress page or post.

#### Scenario: Shortcode renders form
- **WHEN** a page containing `[genossi_beitritt]` is loaded
- **THEN** the plugin renders an HTML form with fields for Vorname, Nachname, Anrede, E-Mail, Straße, Hausnummer, PLZ, Ort, and Anzahl Geschäftsanteile

#### Scenario: Settings not configured
- **WHEN** a page containing `[genossi_beitritt]` is loaded but API URL or API Key are not configured
- **THEN** the plugin displays a notice to administrators ("Plugin nicht konfiguriert") and shows nothing to regular visitors

### Requirement: Form fields
The form SHALL contain the following fields:
- Anrede (select, optional: Herr, Frau, Firma)
- Vorname (text, required)
- Nachname (text, required)
- E-Mail (email, required)
- Straße (text, required)
- Hausnummer (text, required)
- PLZ (text, required)
- Ort (text, required)
- Anzahl Geschäftsanteile (number, required, minimum 1, default 1)
- Checkbox: Datenschutzerklärung gelesen und akzeptiert (required)
- Checkbox: Satzung gelesen und akzeptiert (required)

The form SHALL include a WordPress nonce field for CSRF protection.

#### Scenario: Form displays all fields
- **WHEN** the form is rendered
- **THEN** all specified fields are visible with appropriate HTML input types and required attributes

#### Scenario: Client-side validation
- **WHEN** a user tries to submit the form without filling in required fields
- **THEN** the browser prevents submission and highlights the missing fields

#### Scenario: Shares default value
- **WHEN** the form is rendered
- **THEN** the shares field has a default value of 1 and a minimum of 1

### Requirement: Form submission processing
The plugin SHALL process form submissions via a standard HTTP POST to the same page. Upon submission, the plugin SHALL:
1. Verify the WordPress nonce
2. Validate required fields server-side
3. Send a POST request to the Genossi API (`{api_url}/api/public/join`) with the form data as JSON and the API key in the `X-Api-Key` header
4. Display a success message or error message based on the API response

The API call SHALL be made server-side using `wp_remote_post()`.

#### Scenario: Successful submission
- **WHEN** a user fills in all required fields, checks both checkboxes, and submits the form
- **THEN** the plugin sends the data to the Genossi API and displays a success message ("Vielen Dank für Ihre Beitrittserklärung! Sie erhalten in Kürze eine E-Mail mit den Überweisungsdaten.")

#### Scenario: Invalid nonce
- **WHEN** a POST is received with an invalid or missing nonce
- **THEN** the plugin rejects the submission and displays an error

#### Scenario: Missing required field server-side
- **WHEN** a POST is received with a missing required field (e.g., empty email)
- **THEN** the plugin displays a validation error next to the relevant field without calling the API

#### Scenario: API returns 422
- **WHEN** the Genossi API returns HTTP 422 with validation errors
- **THEN** the plugin displays the error messages to the user in a readable format

#### Scenario: API returns 401
- **WHEN** the Genossi API returns HTTP 401 (invalid API key)
- **THEN** the plugin displays a generic error message ("Ein Fehler ist aufgetreten. Bitte versuchen Sie es später erneut.") and logs the actual error for administrators

#### Scenario: API unreachable
- **WHEN** the Genossi API is not reachable (connection timeout, DNS failure)
- **THEN** the plugin displays a generic error message and logs the connection error

#### Scenario: Form values preserved on error
- **WHEN** a submission fails due to validation or API error
- **THEN** the form is re-displayed with the previously entered values pre-filled

### Requirement: Minimal styling
The plugin SHALL include a minimal CSS file that provides basic form layout (labels, inputs, buttons, error messages). The styling SHALL use standard CSS classes that integrate with common WordPress themes without overriding theme styles.

#### Scenario: Form styled with plugin CSS
- **WHEN** the shortcode is rendered on a page
- **THEN** the plugin enqueues its CSS file only on pages containing the shortcode

#### Scenario: Theme compatibility
- **WHEN** the form is displayed on a WordPress site with a standard theme (e.g., Twenty Twenty-Four)
- **THEN** the form inherits the theme's typography and colors while maintaining a readable layout
