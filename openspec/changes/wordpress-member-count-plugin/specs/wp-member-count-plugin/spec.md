## ADDED Requirements

### Requirement: Member count shortcode
The plugin SHALL register a WordPress shortcode `[genossi_member_count]` that outputs the current active member count as an HTML `<span>` element with CSS class `genossi-member-count`.

#### Scenario: Shortcode renders member count
- **WHEN** a page contains the shortcode `[genossi_member_count]` and the API-URL is configured and the API returns a valid response
- **THEN** the shortcode SHALL output `<span class="genossi-member-count">{count}</span>` where `{count}` is the numeric value from the API response

#### Scenario: Shortcode with unconfigured API URL
- **WHEN** a page contains the shortcode and no API-URL is configured (neither from genossi-beitritt nor standalone)
- **THEN** the shortcode SHALL output an empty string for visitors and a configuration hint for admins (`manage_options` capability)

#### Scenario: Shortcode with API error
- **WHEN** a page contains the shortcode and the API request fails (network error, 403, 500, etc.) and no cached value exists
- **THEN** the shortcode SHALL output an empty string for visitors

### Requirement: API URL resolution with fallback
The plugin SHALL resolve the Genossi API URL using a fallback chain: first check `genossi_api_url` (set by the genossi-beitritt plugin), then fall back to `genossi_mc_api_url` (the plugin's own option).

#### Scenario: URL from genossi-beitritt plugin
- **WHEN** the WordPress option `genossi_api_url` is set and non-empty
- **THEN** the plugin SHALL use that value as the API base URL

#### Scenario: URL from own setting
- **WHEN** the WordPress option `genossi_api_url` is empty or not set and `genossi_mc_api_url` is set and non-empty
- **THEN** the plugin SHALL use `genossi_mc_api_url` as the API base URL

#### Scenario: No URL configured
- **WHEN** both `genossi_api_url` and `genossi_mc_api_url` are empty or not set
- **THEN** the plugin SHALL treat the API as unconfigured

### Requirement: API request
The plugin SHALL make a GET request to `{api_url}/api/public/member-count` and parse the JSON response field `count` as the member count.

#### Scenario: Successful API response
- **WHEN** the API returns HTTP 200 with JSON body `{"count": 42}`
- **THEN** the plugin SHALL use `42` as the member count

#### Scenario: API returns 403 (feature disabled)
- **WHEN** the API returns HTTP 403
- **THEN** the plugin SHALL treat this as a failure and not cache the error response

#### Scenario: API returns non-200 status
- **WHEN** the API returns any non-200 HTTP status
- **THEN** the plugin SHALL treat this as a failure

### Requirement: Transient caching
The plugin SHALL cache the member count using WordPress Transients with a configurable TTL. The default TTL SHALL be 900 seconds (15 minutes).

#### Scenario: Cache hit
- **WHEN** a valid transient `genossi_member_count` exists and has not expired
- **THEN** the plugin SHALL return the cached value without making an API request

#### Scenario: Cache miss
- **WHEN** no transient exists or it has expired
- **THEN** the plugin SHALL make an API request and store the result as a transient with the configured TTL

#### Scenario: API failure with existing cache
- **WHEN** the API request fails but a previously cached value exists (expired)
- **THEN** the plugin SHALL NOT cache the failure and the expired value will already be gone from the transient store

### Requirement: Settings page
The plugin SHALL register a settings page under WordPress "Einstellungen" menu with the title "Genossi Member Count".

#### Scenario: Beitritt-Plugin URL detected
- **WHEN** the WordPress option `genossi_api_url` is set and non-empty
- **THEN** the settings page SHALL display the URL as read-only with a note that it is inherited from the Genossi Beitritt plugin

#### Scenario: Standalone mode
- **WHEN** the WordPress option `genossi_api_url` is empty or not set
- **THEN** the settings page SHALL display an editable URL input field for `genossi_mc_api_url`

#### Scenario: Cache TTL setting
- **WHEN** the admin views the settings page
- **THEN** the settings page SHALL display a numeric input field for the cache duration in seconds with default value 900

### Requirement: WordPress plugin metadata
The plugin SHALL include standard WordPress plugin headers (Plugin Name, Description, Version, Author, License, Text Domain, Requires PHP).

#### Scenario: Plugin appears in WordPress admin
- **WHEN** the plugin files are placed in `wp-content/plugins/genossi-member-count/`
- **THEN** WordPress SHALL list the plugin with name "Genossi Member Count" in the plugins admin page
