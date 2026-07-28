use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, MailTemplateTO};
use crate::auth::RequirePrivilege;
use crate::component::mail_compose::{
    plain_to_html, TemplateTester, TemplateVarButtons, WysiwygEditor,
};
use crate::component::{ErrorAlert, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;

#[derive(Clone, PartialEq)]
enum EditorMode {
    None,
    Create,
    Edit(String), // template id
}

#[component]
pub fn MailTemplatesPage() -> Element {
    let i18n = use_i18n();
    let mut templates = use_signal(Vec::<MailTemplateTO>::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);
    let mut editor_mode = use_signal(|| EditorMode::None);
    let mut edit_name = use_signal(String::new);
    let mut edit_subject = use_signal(String::new);
    let mut edit_body = use_signal(String::new);
    // Phase 24 Plan 03 Task 4 (EDIT-01, D-01): companion HTML body pushed
    // from WysiwygEditor DOM. Initialized from template.body_html on Edit;
    // empty on Create. On save, forwarded to create/update_mail_template
    // via the empty→None backwards-compat rule.
    let mut edit_body_html = use_signal(String::new);
    let mut edit_version = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    // Phase 28 (PREV-02, D-03): GENAU EINE Mitglieds-Auswahl für die Vorschau
    // auf dieser Seite. Vorher standen hier zwei verschachtelte Auswahlen
    // nebeneinander — die Suche im TemplateTester und das Auswahlfeld in der
    // TemplatePreview darin. Jetzt teilen sie sich dieses Signal, das zugleich
    // die Device-Vorschau im WysiwygEditor speist.
    let preview_member_id = use_signal(|| None::<Uuid>);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_mail_templates(&config).await {
                Ok(data) => {
                    templates.set(data);
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let mut on_select_template = move |tpl: MailTemplateTO| {
        edit_name.set(tpl.name.clone());
        edit_subject.set(tpl.subject.clone());
        edit_body.set(tpl.body.clone());
        // Fallback: Legacy-Templates (Phase <24) haben body_html=None; ohne
        // Fallback wäre der WysiwygEditor leer. Konvertiere den Plain-Text-
        // Body zu HTML (escape + \n→<br>), damit der Nutzer nicht denkt,
        // das Template sei gelöscht.
        let html_seed = tpl
            .body_html
            .as_deref()
            .filter(|h| !h.trim().is_empty())
            .map(|h| h.to_string())
            .unwrap_or_else(|| plain_to_html(&tpl.body));
        edit_body_html.set(html_seed);
        edit_version.set(tpl.version.clone());
        editor_mode.set(EditorMode::Edit(tpl.id.clone()));
        confirm_delete.set(false);
        error.set(None);
    };

    let on_create_new = move |_| {
        edit_name.set(String::new());
        edit_subject.set(String::new());
        edit_body.set(String::new());
        edit_body_html.set(String::new());
        edit_version.set(String::new());
        editor_mode.set(EditorMode::Create);
        confirm_delete.set(false);
        error.set(None);
    };

    let on_save = move |_| {
        // Phase 24 Plan 03 Task 4 Submit-Guard (Pitfall 5, D-01
        // belt-and-suspenders): re-read the DOM's innerHTML+innerText
        // before Save so a late toolbar-click that missed on_command is
        // still captured on write.
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.get_element_by_id("wysiwyg-editor") {
                let html = el.inner_html();
                let plain = wasm_bindgen::JsCast::dyn_ref::<web_sys::HtmlElement>(&el)
                    .map(|he| he.inner_text())
                    .unwrap_or_default();
                edit_body.set(plain);
                edit_body_html.set(html);
            }
        }
        let name = edit_name.read().clone();
        let subject = edit_subject.read().clone();
        let body = edit_body.read().clone();
        let body_html_value = edit_body_html.read().clone();
        let version = edit_version.read().clone();
        let mode = editor_mode.read().clone();
        spawn(async move {
            saving.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            // Phase 24 Plan 03 Task 4 (D-01): empty→None backwards-compat
            // rule — templates saved without any HTML markup stay legacy
            // plaintext-only templates.
            let body_html_opt: Option<&str> = if body_html_value.trim().is_empty() {
                None
            } else {
                Some(body_html_value.as_str())
            };
            let result = match &mode {
                EditorMode::Create => {
                    api::create_mail_template(&config, &name, &subject, &body, body_html_opt).await
                }
                EditorMode::Edit(id) => {
                    api::update_mail_template(
                        &config,
                        id,
                        &name,
                        &subject,
                        &body,
                        body_html_opt,
                        &version,
                    )
                    .await
                }
                EditorMode::None => return,
            };
            match result {
                Ok(tpl) => {
                    edit_version.set(tpl.version.clone());
                    if matches!(mode, EditorMode::Create) {
                        editor_mode.set(EditorMode::Edit(tpl.id.clone()));
                    }
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let on_delete = move |_| {
        if !*confirm_delete.read() {
            confirm_delete.set(true);
            return;
        }
        let mode = editor_mode.read().clone();
        if let EditorMode::Edit(id) = mode {
            spawn(async move {
                saving.set(true);
                error.set(None);
                let config = CONFIG.read().clone();
                match api::delete_mail_template(&config, &id).await {
                    Ok(()) => {
                        editor_mode.set(EditorMode::None);
                        edit_name.set(String::new());
                        edit_subject.set(String::new());
                        edit_body.set(String::new());
                        edit_body_html.set(String::new());
                        edit_version.set(String::new());
                        confirm_delete.set(false);
                        reload();
                    }
                    Err(e) => error.set(Some(e)),
                }
                saving.set(false);
            });
        }
    };

    let is_editing = !matches!(*editor_mode.read(), EditorMode::None);

    rsx! {
        RequirePrivilege {
            privilege: crate::auth::PRIVILEGE_ADMIN,
            fallback: rsx! { AccessDeniedPage { required_privilege: crate::auth::PRIVILEGE_ADMIN.to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::MailTemplates)}
                    }

                    if let Some(ref err) = *error.read() {
                        ErrorAlert {
                            error: err.clone(),
                            on_dismiss: move |_| error.set(None),
                        }
                    }

                    div { class: "flex gap-6",
                        // Left column: template list
                        div { class: "w-1/3",
                            div { class: "bg-white rounded-lg shadow p-4",
                                div { class: "flex items-center justify-between mb-4",
                                    h2 { class: "text-lg font-semibold",
                                        {i18n.t(Key::MailTemplates)}
                                    }
                                    button {
                                        class: "bg-blue-500 hover:bg-blue-600 text-white px-3 py-1 rounded text-sm",
                                        onclick: on_create_new,
                                        {i18n.t(Key::MailTemplateCreate)}
                                    }
                                }

                                if *loading.read() {
                                    p { class: "text-gray-500", {i18n.t(Key::Loading)} }
                                } else if templates.read().is_empty() {
                                    p { class: "text-gray-500 text-sm italic",
                                        {i18n.t(Key::MailTemplateEmpty)}
                                    }
                                } else {
                                    div { class: "space-y-1",
                                        for tpl in templates.read().iter() {
                                            {
                                                let tpl_clone = tpl.clone();
                                                let is_active = matches!(&*editor_mode.read(), EditorMode::Edit(id) if id == &tpl.id);
                                                let item_class = if is_active {
                                                    "w-full text-left px-3 py-2 rounded bg-blue-100 text-blue-800 font-medium cursor-pointer"
                                                } else {
                                                    "w-full text-left px-3 py-2 rounded hover:bg-gray-100 cursor-pointer"
                                                };
                                                rsx! {
                                                    button {
                                                        class: "{item_class}",
                                                        onclick: move |_| on_select_template(tpl_clone.clone()),
                                                        "{tpl.name}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Right column: editor
                        div { class: "w-2/3",
                            if is_editing {
                                div { class: "bg-white rounded-lg shadow p-6",
                                    h2 { class: "text-lg font-semibold mb-4",
                                        if matches!(*editor_mode.read(), EditorMode::Create) {
                                            {i18n.t(Key::MailTemplateCreate)}
                                        } else {
                                            {i18n.t(Key::MailTemplateSave)}
                                        }
                                    }
                                    div { class: "space-y-4",
                                        // Name
                                        div {
                                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                                {i18n.t(Key::MailTemplateName)}
                                            }
                                            input {
                                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                                r#type: "text",
                                                value: "{edit_name}",
                                                oninput: move |e| edit_name.set(e.value()),
                                            }
                                        }

                                        // Subject
                                        div {
                                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                                {i18n.t(Key::MailTemplateSubject)}
                                            }
                                            input {
                                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                                r#type: "text",
                                                value: "{edit_subject}",
                                                oninput: move |e| edit_subject.set(e.value()),
                                            }
                                        }

                                        // Template variable buttons
                                        TemplateVarButtons {
                                            on_insert: move |var_text: String| {
                                                edit_body.write().push_str(&var_text);
                                                // Phase 24 Plan 03 Task 4:
                                                // mirror the inserted var
                                                // (HTML-escaped) into
                                                // edit_body_html so both
                                                // signals stay in sync until
                                                // the next user keystroke
                                                // resyncs from the DOM.
                                                let escaped = var_text
                                                    .replace('&', "&amp;")
                                                    .replace('<', "&lt;")
                                                    .replace('>', "&gt;");
                                                edit_body_html.write().push_str(&escaped);
                                            },
                                            show_repayment_vars: true,
                                        }

                                        // Body — Phase 24 Plan 03 Task 4:
                                        // migrated from a plain textarea to
                                        // the WysiwygEditor (Component-First).
                                        // The editor owns both edit_body
                                        // (innerText → plain-text template)
                                        // and edit_body_html (innerHTML → HTML
                                        // template rendered by the backend's
                                        // autoescape env in Phase 23 D-04).
                                        div {
                                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                                {i18n.t(Key::MailTemplateBody)}
                                            }
                                            // `key` erzwingt Remount, wenn ein
                                            // anderes Template gewählt wird —
                                            // WysiwygEditor::onmounted seedet
                                            // innerHTML nur beim Mount, ohne
                                            // Remount bliebe der Editor-Inhalt
                                            // beim Template-Wechsel stehen.
                                            {
                                                let editor_key = match &*editor_mode.read() {
                                                    EditorMode::Edit(id) => id.clone(),
                                                    EditorMode::Create => "__create__".to_string(),
                                                    EditorMode::None => String::new(),
                                                };
                                                rsx! {
                                                    // Phase 28 (PREV-02, D-03):
                                                    // Device-Vorschau im Editor —
                                                    // sie liest die EINE
                                                    // Mitglieds-Auswahl der Seite,
                                                    // dieselbe, die der
                                                    // TemplateTester unten führt.
                                                    // Das optionale
                                                    // Rueckzahlungs-Kontext-Prop
                                                    // wird hier bewusst NICHT
                                                    // gesetzt: diese Seite kennt
                                                    // keinen solchen Kontext, und
                                                    // ein Template mit
                                                    // entsprechenden Platzhaltern
                                                    // soll den Render-Fehler
                                                    // sichtbar im roten
                                                    // Fehler-Block zeigen.
                                                    WysiwygEditor {
                                                        key: "{editor_key}",
                                                        value: edit_body_html.read().clone(),
                                                        on_change: move |(plain, html): (String, String)| {
                                                            edit_body.set(plain);
                                                            edit_body_html.set(html);
                                                        },
                                                        preview_member_id: *preview_member_id.read(),
                                                    }
                                                }
                                            }
                                        }

                                        // Quick 260603-jtf: Template-Tester.
                                        // Component-First — Member-Selector,
                                        // Preview und Send-Button werden hier
                                        // bewusst NICHT inline implementiert.
                                        // Phase 24 Plan 03 Task 4: forwards
                                        // edit_body_html to TemplatePreview
                                        // via the new body_html prop.
                                        // Phase 28 (PREV-02, D-03): dasselbe
                                        // Signal wie am Editor oben — die
                                        // Member-Suche hier und das
                                        // Auswahlfeld der TemplatePreview
                                        // darin zeigen ab jetzt denselben
                                        // Zustand statt zweier konkurrierender
                                        // Auswahlen.
                                        TemplateTester {
                                            subject: edit_subject,
                                            body: edit_body,
                                            body_html: edit_body_html,
                                            selected_member_id: preview_member_id,
                                        }

                                        // Action buttons
                                        div { class: "flex items-center gap-3",
                                            button {
                                                class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                                disabled: *saving.read() || edit_name.read().is_empty(),
                                                onclick: on_save,
                                                {i18n.t(Key::MailTemplateSave)}
                                            }
                                            if matches!(*editor_mode.read(), EditorMode::Edit(_)) {
                                                button {
                                                    class: if *confirm_delete.read() {
                                                        "bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded disabled:opacity-50"
                                                    } else {
                                                        "bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded disabled:opacity-50"
                                                    },
                                                    disabled: *saving.read(),
                                                    onclick: on_delete,
                                                    if *confirm_delete.read() {
                                                        {i18n.t(Key::MailTemplateDeleteConfirm)}
                                                    } else {
                                                        {i18n.t(Key::MailTemplateDelete)}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { class: "bg-white rounded-lg shadow p-6 text-gray-500 text-center",
                                    p { "Wähle eine Vorlage aus der Liste oder erstelle eine neue." }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
