## Why

Users currently write mail body and subject from scratch every time they compose a bulk mail. For common use cases (formal letters, informal greetings) the Jinja template syntax for salutation/title handling is complex and error-prone. Predefined templates with correct salutation logic would save time and prevent mistakes.

## What Changes

- Add two predefined mail templates selectable via dropdown in the mail compose UI:
  - **Formell**: "Sehr geehrter Herr / Sehr geehrte Frau / Sehr geehrtes Mitglied" with "Mit freundlichen Grüßen"
  - **Informell**: "Lieber / Liebe / Hallo" with "Viele Grüße"
- Templates include correct gender-aware salutation, optional title, and closing formula
- Selecting a template pre-fills the subject and body fields (user can still edit)
- Templates are hardcoded (not user-managed), since only two are needed

## Capabilities

### New Capabilities
- `predefined-mail-templates`: Dropdown-selectable mail templates that pre-fill the compose form with gender-aware salutation, body placeholder, and closing formula

### Modified Capabilities
- `mail-sending`: The compose UI gains a template selector dropdown above the body field

## Impact

- **Frontend**: `mail_page.rs` — add template dropdown and pre-fill logic
- **i18n**: New translation keys for template names and dropdown label
- **No backend changes needed** — templates are frontend-only string constants applied before the existing template variable rendering
