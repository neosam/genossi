## 1. Template Constants

- [x] 1.1 Define formal and informal template body strings as constants in `mail_page.rs`
- [x] 1.2 Add i18n keys for template dropdown: label ("Vorlage"), option names ("Formell", "Informell"), placeholder ("Vorlage wählen...")

## 2. UI Integration

- [x] 2.1 Add template dropdown `<select>` between subject field and body textarea in `mail_page.rs`
- [x] 2.2 Wire dropdown `onchange` to pre-fill body signal with selected template content

## 3. Tests

- [x] 3.1 Add template rendering tests verifying formal template output for Herr/Frau/no salutation with and without title
- [x] 3.2 Add template rendering tests verifying informal template output for Herr/Frau/no salutation with and without title
