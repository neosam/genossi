## 1. MiniJinja-Dependency und Template-Modul

- [x] 1.1 Add `minijinja` crate to `genossi_mail/Cargo.toml`
- [x] 1.2 Create `genossi_mail/src/template.rs` with `member_to_template_context(entity: &MemberEntity) -> minijinja::Value` mapping all MemberEntity fields
- [x] 1.3 Create `render_template(template_str: &str, context: &minijinja::Value) -> Result<String, TemplateError>` function
- [x] 1.4 Create `validate_template(subject: &str, body: &str, members: &[MemberEntity]) -> Result<(), Vec<String>>` that probe-renders against all members
- [x] 1.5 Write unit tests for template rendering: simple substitution, conditionals, null fields, unknown variables, syntax errors

## 2. Preview-Endpoint

- [x] 2.1 Add `PreviewRequest` and `PreviewResponse` types to `genossi_mail/src/rest.rs`
- [x] 2.2 Add `preview` method to `MailService` trait that takes subject, body, member_id and returns rendered result
- [x] 2.3 Implement `preview` in `MailServiceImpl` — load member via MemberDao, render templates, return result with errors
- [x] 2.4 Add `POST /api/mail/preview` endpoint in `genossi_mail/src/rest.rs`
- [x] 2.5 Wire up MemberDao access in MailRestState / MailServiceImpl (add generic parameter)
- [x] 2.6 Write tests for preview endpoint: successful preview, syntax error, unknown member

## 3. Template-Validierung bei Job-Erstellung

- [x] 3.1 Extend `create_job` to require `member_id` for all recipients (return error if any is None)
- [x] 3.2 Load all recipient MemberEntities in `create_job` and validate templates before creating job
- [x] 3.3 Return descriptive 400 error on validation failure (syntax errors, unknown variables)
- [x] 3.4 Write tests for validation: valid template passes, syntax error rejected, unknown variable rejected, missing member_id rejected

## 4. Worker Template-Rendering

- [x] 4.1 Add MemberDao generic parameter to `start_mail_worker` function signature
- [x] 4.2 In worker loop: load MemberEntity for recipient's member_id before sending
- [x] 4.3 Render subject and body templates with member context before passing to `send_mail_for_recipient`
- [x] 4.4 Handle missing/deleted member gracefully: mark recipient as failed with error message
- [x] 4.5 Pass-through for templates without Jinja syntax (plain text bodies work unchanged)
- [x] 4.6 Update `start_mail_worker` call site in `genossi_bin` to pass MemberDao
- [x] 4.7 Write tests for worker template rendering: successful render, missing member, plain text passthrough

## 5. Frontend: Variablen-Buttons

- [x] 5.1 Create variable button data structure with primary variables (first_name, last_name, salutation, title, member_number, company) and secondary variables (remaining fields)
- [x] 5.2 Add variable button row above body textarea — click appends `{{ variable }}` to body
- [x] 5.3 Add variable button row above subject input — click appends `{{ variable }}` to subject
- [x] 5.4 Add "Mehr" toggle to show/hide secondary variable buttons

## 6. Frontend: Preview-Panel

- [x] 6.1 Add preview member selector (dropdown from selected recipients)
- [x] 6.2 Add `preview_mail` API function in `genossi-frontend/src/api.rs`
- [x] 6.3 Add preview panel below compose form showing rendered subject and body
- [x] 6.4 Trigger preview on template change with debounce (e.g. 500ms after last keystroke)
- [x] 6.5 Show template errors in preview panel when syntax is invalid
- [x] 6.6 Add i18n keys for new UI labels (preview, variables, template errors)
