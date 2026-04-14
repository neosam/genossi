## Why

Aktuell können Typst-Templates nur mit Member-Daten gerendert werden. Für eingegangene Eintrittserklärungen (Applications), die noch im Status "Offen" sind, muss der Vorstand teilweise Briefe verschicken — z.B. Zahlungsaufforderungen. Dafür braucht es einen eigenen Render-Endpoint, der Application-Daten an Typst-Templates übergibt, sowie eine Auswahlmöglichkeit im Frontend-Template-Editor.

## What Changes

- Neuer REST-Endpoint `POST /api/templates/render-application/{path}/{application_id}` zum Rendern von Templates mit Application-Daten
- Neue `build_inputs_application()` Funktion im PdfGenerator, die Application-Daten als JSON-Dict für Typst aufbereitet (Key: `application`)
- Frontend: Neuer `ApplicationSearch`-Komponente analog zu `MemberSearch`, filtert nur offene Applications
- Frontend: Toggle im Template-Editor zwischen "Member" und "Antrag" für die Preview-Auswahl
- Frontend: Neue API-Funktion `render_template_pdf_application()` für den neuen Endpoint

## Capabilities

### New Capabilities
- `application-template-render`: Rendern von Typst-Templates mit Application-Daten über einen eigenen Endpoint und Frontend-Integration im Template-Editor

### Modified Capabilities
- `template-editor`: Preview-Bereich bekommt Toggle zwischen Member- und Application-Auswahl

## Impact

- **Backend**: `genossi_rest/src/template.rs` (neuer Endpoint), `genossi_service_impl/src/pdf_generation.rs` (neue build_inputs Funktion)
- **Frontend**: `genossi-frontend/src/page/templates.rs` (Toggle UI), `genossi-frontend/src/api.rs` (neue API-Funktion), neue `ApplicationSearch`-Komponente
- **API**: Neuer POST-Endpoint, keine Breaking Changes an bestehenden Endpoints
