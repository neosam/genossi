use dioxus::prelude::*;

use crate::api::{self, MailTemplateTO};
use crate::auth::RequirePrivilege;
use crate::component::mail_compose::TemplateVarButtons;
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
    let mut edit_version = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);

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
        edit_version.set(tpl.version.clone());
        editor_mode.set(EditorMode::Edit(tpl.id.clone()));
        confirm_delete.set(false);
        error.set(None);
    };

    let on_create_new = move |_| {
        edit_name.set(String::new());
        edit_subject.set(String::new());
        edit_body.set(String::new());
        edit_version.set(String::new());
        editor_mode.set(EditorMode::Create);
        confirm_delete.set(false);
        error.set(None);
    };

    let on_save = move |_| {
        let name = edit_name.read().clone();
        let subject = edit_subject.read().clone();
        let body = edit_body.read().clone();
        let version = edit_version.read().clone();
        let mode = editor_mode.read().clone();
        spawn(async move {
            saving.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            let result = match &mode {
                EditorMode::Create => {
                    api::create_mail_template(&config, &name, &subject, &body).await
                }
                EditorMode::Edit(id) => {
                    api::update_mail_template(&config, id, &name, &subject, &body, &version).await
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
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
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
                                            },
                                        }

                                        // Body
                                        div {
                                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                                {i18n.t(Key::MailTemplateBody)}
                                            }
                                            textarea {
                                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 font-mono text-sm",
                                                rows: "12",
                                                value: "{edit_body}",
                                                oninput: move |e| edit_body.set(e.value()),
                                            }
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
