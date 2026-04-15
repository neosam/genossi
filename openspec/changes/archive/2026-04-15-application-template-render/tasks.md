## 1. Backend: Application-Inputs für PdfGenerator

- [x] 1.1 `build_inputs_application()` Funktion in `genossi_service_impl/src/pdf_generation.rs` erstellen, die Application-Daten als JSON-Dict mit Key `application` und `today` aufbereitet
- [x] 1.2 Unit-Tests für `build_inputs_application()` schreiben (Felder korrekt gemappt, optionale Felder als null, Datumsformat DD.MM.YYYY)

## 2. Backend: Render-Endpoint

- [x] 2.1 Render-Methode für Applications im PdfGenerator hinzufügen (`render_application`), die `build_inputs_application()` nutzt
- [x] 2.2 Neuen REST-Endpoint `POST /api/templates/render-application/{path}/{application_id}` in `genossi_rest/src/template.rs` erstellen
- [x] 2.3 Permission-Check `manage_members` für den neuen Endpoint sicherstellen
- [x] 2.4 Endpoint in OpenAPI/Utoipa-Dokumentation registrieren
- [x] 2.5 Integration-Tests für den neuen Endpoint schreiben (erfolgreicher Render, Application nicht gefunden, fehlende Permission)

## 3. Frontend: ApplicationSearch-Komponente

- [x] 3.1 `ApplicationSearch`-Komponente in `genossi-frontend/src/component/` erstellen, analog zu `MemberSearch`
- [x] 3.2 Suchlogik: Filtert nur offene Applications nach Vor-/Nachname, Anzeige als "Vorname Nachname (N Anteile)"
- [x] 3.3 Application-Daten laden: Globalen State oder lokalen Fetch für offene Applications einrichten

## 4. Frontend: API-Funktion

- [x] 4.1 `render_template_pdf_application(config, path, application_id)` Funktion in `genossi-frontend/src/api.rs` erstellen
- [x] 4.2 `template_render_application_url()` Helper-Funktion für URL-Konstruktion

## 5. Frontend: Template-Editor Preview-Toggle

- [x] 5.1 Toggle-UI ("Mitglied" / "Antrag") im Preview-Bereich von `genossi-frontend/src/page/templates.rs` einbauen
- [x] 5.2 State-Management: Signal für aktiven Tab, beim Umschalten Auswahl zurücksetzen
- [x] 5.3 Render-Button-Logic: Je nach Tab den passenden Render-Endpoint aufrufen

## 6. I18n

- [x] 6.1 Deutsche und englische Übersetzungen für neue UI-Texte hinzufügen (Toggle-Labels, Placeholder-Texte)
