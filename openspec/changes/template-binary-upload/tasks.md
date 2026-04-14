## 1. Backend: TemplateStorage erweitern

- [x] 1.1 Neue Methode `write_file_bytes(&self, relative_path: &str, content: &[u8])` in `TemplateStorage` hinzufügen
- [x] 1.2 Test für `write_file_bytes` mit Binärdaten (z.B. PNG-Header-Bytes)

## 2. Backend: REST-Handler anpassen

- [x] 2.1 `write_template` Handler: Body-Typ von `String` auf `axum::body::Bytes` ändern
- [x] 2.2 `write_template` Handler: `write_file_bytes` statt `write_file` aufrufen
- [x] 2.3 `read_template` Handler: Für Nicht-Text-Dateien `application/octet-stream` als Content-Type zurückgeben statt `text/plain`

## 3. Frontend: API-Funktion

- [x] 3.1 Neue API-Funktion `upload_template_file(config, path, bytes)` in `api.rs` die Binärdaten per PUT sendet

## 4. Frontend: Upload-Button

- [x] 4.1 Upload-Button in die Toolbar des Template-Editors einfügen (neben "New File" und "New Folder")
- [x] 4.2 Bei Klick einen versteckten File-Input triggern, Datei auswählen und per `upload_template_file` hochladen
- [x] 4.3 Nach Upload den File-Tree neu laden
- [x] 4.4 Binärdateien (nicht-.typ) im File-Tree anzeigen aber nicht im Code-Editor öffnen (Klick ignorieren oder Hinweis zeigen)

## 5. Frontend: i18n

- [x] 5.1 Neue i18n-Keys für Upload-Button-Text hinzufügen (Key::UploadFile)
