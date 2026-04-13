## 1. Backend: Footer-Endpunkt

- [x] 1.1 Footer-Rendering-Funktion im Mail-Service erstellen: Config `mail_footer` laden, User Preference `sender_name` laden, mit minijinja rendern
- [x] 1.2 REST-Endpunkt `GET /api/mail/footer` implementieren, der den gerenderten Footer für den aktuellen User zurückgibt
- [x] 1.3 Tests für Footer-Rendering: mit/ohne sender_name, mit/ohne mail_footer Config, ungültiges Template

## 2. Frontend: Footer-Integration im Mail Compose

- [x] 2.1 API-Funktion `get_mail_footer()` im Frontend erstellen
- [x] 2.2 Mail-Compose-Formular: Beim Öffnen den Footer laden und als Initialwert ins Body-Textfeld setzen (`"\n\n" + footer`)
- [x] 2.3 TemplateSelector: Beim Einfügen einer Vorlage den gecachten Footer anhängen (`vorlage + "\n" + footer`)

## 3. Frontend: Vorlagen anpassen

- [x] 3.1 TEMPLATE_FORMAL kürzen: Grußformel ("Mit freundlichen Grüßen") entfernen
- [x] 3.2 TEMPLATE_INFORMAL kürzen: Grußformel ("Viele Grüße") entfernen

## 4. Frontend: Inbox Reply Footer

- [x] 4.1 Inbox Reply-Formular: Footer beim Öffnen laden und ins Body-Textfeld setzen

## 5. Config UI

- [x] 5.1 Eingabefeld für `mail_footer` in den SMTP/Mail-Einstellungen hinzufügen
- [x] 5.2 Eingabefeld für `sender_name` in den User-Einstellungen hinzufügen

## 6. E2E Tests

- [x] 6.1 E2E-Test: Footer-Endpunkt mit gesetztem sender_name und mail_footer Config
- [x] 6.2 E2E-Test: Footer-Endpunkt ohne Konfiguration gibt leeren String zurück
