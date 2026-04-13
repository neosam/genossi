## Context

The mail compose UI already supports Jinja-style template variables ({{ first_name }}, {{ salutation }}, etc.) that are resolved per recipient by the mail worker. However, writing correct gender-aware salutations requires complex Jinja conditionals that users shouldn't have to author manually. Two predefined templates (formal and informal) cover the most common use cases.

## Goals / Non-Goals

**Goals:**
- Provide two selectable templates (formal, informal) that pre-fill subject and body
- Templates use existing Jinja variable syntax so the mail worker handles them without changes
- Users can edit the pre-filled content before sending

**Non-Goals:**
- User-managed custom templates (CRUD, database storage)
- Backend changes — templates are frontend string constants
- Template management UI

## Decisions

### Frontend-only implementation
Templates are hardcoded Rust string constants in the frontend. No backend API, no database table.

**Rationale**: Only two templates are needed. Adding backend storage would be overengineering. If more templates are needed later, this can be extracted.

### Template content

**Formal template:**
```
Subject: (empty — user fills in)
Body:
Sehr geehrte{% if salutation == "Herr" %}r Herr{% elif salutation == "Frau" %} Frau{% else %}s Mitglied{% endif %}{% if title %} {{ title }}{% endif %} {{ last_name }},

[Inhalt]

Mit freundlichen Grüßen
```

**Informal template:**
```
Subject: (empty — user fills in)
Body:
{% if salutation == "Herr" %}Lieber{% elif salutation == "Frau" %}Liebe{% else %}Hallo{% endif %}{% if title %} {{ title }}{% endif %} {{ first_name }},

[Inhalt]

Viele Grüße
```

### UI: Dropdown above body field
A `<select>` dropdown with options: "— Vorlage wählen —" (default/empty), "Formell", "Informell". Selecting a template replaces the body content. Selecting the empty option clears nothing (no-op).

**Rationale**: Dropdown is minimal UI footprint and consistent with other form elements. Buttons would take more space for just two options.

## Risks / Trade-offs

- **Body overwrite**: Selecting a template replaces existing body content. → Acceptable since templates are used at the start of composing. A confirmation dialog would be overkill for this.
- **Hardcoded German**: Templates are German-only. → Matches the current user base. i18n can be added later if needed.
