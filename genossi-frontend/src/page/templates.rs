use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, FileTreeEntry};
use crate::component::{MemberSearch, Modal, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::refresh_members;

#[component]
fn FileTreeNode(
    entry: FileTreeEntry,
    selected_path: Signal<Option<String>>,
    on_select: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    match entry {
        FileTreeEntry::File { name, path } => {
            let is_selected = selected_path
                .read()
                .as_ref()
                .map(|s| s == &path)
                .unwrap_or(false);
            let path_click = path.clone();
            let path_delete = path.clone();
            rsx! {
                div {
                    class: "flex items-center group",
                    button {
                        class: if is_selected { "flex-1 text-left px-2 py-1 text-sm bg-blue-100 text-blue-800 rounded truncate" } else { "flex-1 text-left px-2 py-1 text-sm hover:bg-gray-100 rounded truncate" },
                        onclick: move |_| on_select.call(path_click.clone()),
                        span { class: "mr-1", "\u{1F4C4}" }
                        "{name}"
                    }
                    button {
                        class: "hidden group-hover:block px-1 text-red-500 hover:text-red-700 text-xs",
                        onclick: move |_| on_delete.call(path_delete.clone()),
                        "\u{2716}"
                    }
                }
            }
        }
        FileTreeEntry::Directory {
            name,
            path,
            children,
        } => {
            let mut expanded = use_signal(|| true);
            let path_delete = path.clone();
            rsx! {
                div {
                    div {
                        class: "flex items-center group",
                        button {
                            class: "flex-1 text-left px-2 py-1 text-sm font-medium hover:bg-gray-100 rounded truncate",
                            onclick: move |_| {
                                let val = *expanded.read();
                                expanded.set(!val);
                            },
                            span { class: "mr-1",
                                if *expanded.read() { "\u{1F4C2}" } else { "\u{1F4C1}" }
                            }
                            "{name}"
                        }
                        button {
                            class: "hidden group-hover:block px-1 text-red-500 hover:text-red-700 text-xs",
                            onclick: move |_| on_delete.call(path_delete.clone()),
                            "\u{2716}"
                        }
                    }
                    if *expanded.read() {
                        div { class: "ml-3 border-l border-gray-200 pl-1",
                            for child in children {
                                FileTreeNode {
                                    entry: child,
                                    selected_path: selected_path,
                                    on_select: on_select,
                                    on_delete: on_delete,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Templates() -> Element {
    let i18n = use_i18n();

    let mut tree = use_signal(|| Vec::<FileTreeEntry>::new());
    let mut selected_path: Signal<Option<String>> = use_signal(|| None);
    let mut editor_content = use_signal(|| String::new());
    let mut original_content = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success_msg = use_signal(|| None::<String>);

    // Modal states
    let mut show_new_file = use_signal(|| false);
    let mut show_new_folder = use_signal(|| false);
    let mut show_delete_confirm = use_signal(|| false);
    let mut delete_path = use_signal(|| String::new());
    let mut new_path_input = use_signal(|| String::new());
    let mut show_unsaved_warning = use_signal(|| false);
    let mut pending_select_path = use_signal(|| String::new());

    // Preview state
    let mut preview_member_id = use_signal(|| None::<Uuid>);
    let preview_error = use_signal(|| None::<String>);

    let has_unsaved_changes = {
        let editor = editor_content.read().clone();
        let original = original_content.read().clone();
        selected_path.read().is_some() && editor != original
    };

    // Load tree on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::get_templates(&config).await {
                Ok(entries) => tree.set(entries),
                Err(e) => error.set(Some(format!("Failed to load templates: {}", e))),
            }
        });
    });

    // Load members for preview dropdown
    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    let reload_tree = move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(entries) = api::get_templates(&config).await {
                tree.set(entries);
            }
        });
    };

    let load_file = move |path: String| {
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            match api::get_template_content(&config, &path).await {
                Ok(content) => {
                    editor_content.set(content.clone());
                    original_content.set(content);
                    selected_path.set(Some(path));
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let mut on_file_select = move |path: String| {
        if has_unsaved_changes {
            pending_select_path.set(path);
            show_unsaved_warning.set(true);
        } else {
            load_file(path);
        }
    };

    let save_current = move |_| {
        let path = selected_path.read().clone();
        if let Some(path) = path {
            let content = editor_content.read().clone();
            spawn(async move {
                let config = CONFIG.read().clone();
                match api::save_template(&config, &path, &content).await {
                    Ok(()) => {
                        original_content.set(content);
                        success_msg.set(Some("Saved".to_string()));
                        spawn(async move {
                            gloo_timers::future::TimeoutFuture::new(2000).await;
                            success_msg.set(None);
                        });
                    }
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    };

    let confirm_delete = move |_| {
        let path = delete_path.read().clone();
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::delete_template(&config, &path).await {
                Ok(()) => {
                    show_delete_confirm.set(false);
                    if selected_path.read().as_ref() == Some(&path) {
                        selected_path.set(None);
                        editor_content.set(String::new());
                        original_content.set(String::new());
                    }
                    reload_tree();
                }
                Err(e) => {
                    show_delete_confirm.set(false);
                    error.set(Some(e));
                }
            }
        });
    };

    let create_file = move |_| {
        let path = new_path_input.read().clone();
        if path.is_empty() {
            return;
        }
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::save_template(&config, &path, "").await {
                Ok(()) => {
                    show_new_file.set(false);
                    new_path_input.set(String::new());
                    reload_tree();
                    load_file(path);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let create_folder = move |_| {
        let path = new_path_input.read().clone();
        if path.is_empty() {
            return;
        }
        let folder_path = if path.ends_with('/') {
            path
        } else {
            format!("{}/", path)
        };
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::save_template(&config, &folder_path, "").await {
                Ok(()) => {
                    show_new_folder.set(false);
                    new_path_input.set(String::new());
                    reload_tree();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    rsx! {
        TopBar {}
        div { class: "p-4",
            h1 { class: "text-2xl font-bold mb-4", {i18n.t(Key::TemplateEditor)} }

            // Error/Success messages
            if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-100 text-red-700 p-3 rounded mb-4",
                    "{err}"
                    button {
                        class: "ml-2 text-red-900 font-bold",
                        onclick: move |_| error.set(None),
                        "\u{2716}"
                    }
                }
            }
            if let Some(msg) = success_msg.read().as_ref() {
                div { class: "bg-green-100 text-green-700 p-3 rounded mb-4",
                    "{msg}"
                }
            }

            // Toolbar
            div { class: "flex flex-wrap gap-2 mb-4",
                button {
                    class: "px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm",
                    onclick: move |_| {
                        new_path_input.set(String::new());
                        show_new_file.set(true);
                    },
                    {i18n.t(Key::NewFile)}
                }
                button {
                    class: "px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm",
                    onclick: move |_| {
                        new_path_input.set(String::new());
                        show_new_folder.set(true);
                    },
                    {i18n.t(Key::NewFolder)}
                }
            }

            // Main layout: tree + editor
            div {
                class: "flex flex-col md:flex-row gap-4",
                style: "height: 75vh;",
                // File tree panel
                div {
                    class: "border rounded p-2 bg-gray-50 overflow-auto shrink-0",
                    style: "width: 220px; min-width: 220px;",
                    if tree.read().is_empty() {
                        p { class: "text-gray-500 text-sm p-2", {i18n.t(Key::NoTemplates)} }
                    }
                    for entry in tree.read().iter().cloned() {
                        FileTreeNode {
                            entry: entry,
                            selected_path: selected_path,
                            on_select: move |path: String| on_file_select(path),
                            on_delete: move |path: String| {
                                delete_path.set(path);
                                show_delete_confirm.set(true);
                            },
                        }
                    }
                }

                // Editor panel
                div { class: "flex-1 flex flex-col min-w-0",
                    if selected_path.read().is_some() {
                        // File info bar
                        div { class: "flex items-center justify-between bg-gray-100 px-3 py-2 rounded-t border border-b-0",
                            span { class: "text-sm font-mono text-gray-600",
                                {selected_path.read().clone().unwrap_or_default()}
                            }
                            div { class: "flex items-center gap-2",
                                if has_unsaved_changes {
                                    span { class: "text-xs text-orange-600 font-medium", "●" }
                                }
                                button {
                                    class: "px-3 py-1 bg-green-600 text-white rounded hover:bg-green-700 text-sm",
                                    disabled: !has_unsaved_changes,
                                    onclick: save_current,
                                    {i18n.t(Key::SaveTemplate)}
                                }
                            }
                        }

                        // Editor textarea
                        textarea {
                            class: "flex-1 w-full border rounded-b p-3 font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-blue-300",
                            value: "{editor_content}",
                            oninput: move |e| editor_content.set(e.value().clone()),
                            spellcheck: "false",
                        }

                        // Preview section
                        div { class: "mt-4 flex flex-wrap items-center gap-2 bg-gray-50 p-3 rounded border",
                            span { class: "text-sm font-medium", {i18n.t(Key::Preview)} ":" }
                            div { class: "w-64",
                                MemberSearch {
                                    on_select: move |id: Option<Uuid>| preview_member_id.set(id),
                                    selected_id: *preview_member_id.read(),
                                }
                            }
                            button {
                                class: if preview_member_id.read().is_some() { "px-3 py-1 bg-purple-600 text-white rounded hover:bg-purple-700 text-sm" } else { "px-3 py-1 bg-gray-300 text-gray-500 rounded text-sm cursor-not-allowed" },
                                disabled: preview_member_id.read().is_none(),
                                onclick: move |_| {
                                    let path = selected_path.read().clone();
                                    let member_id = *preview_member_id.read();
                                    if let (Some(path), Some(member_id)) = (path, member_id) {
                                        spawn(async move {
                                            let config = CONFIG.read().clone();
                                            let url = api::template_render_url(&config, &path, member_id);
                                            match api::render_template_pdf(&config, &path, member_id).await {
                                                Ok(blob_url) => {
                                                    let window = web_sys::window().unwrap();
                                                    let _ = window.open_with_url_and_target(&blob_url, "_blank");
                                                }
                                                Err(e) => error.set(Some(e)),
                                            }
                                        });
                                    }
                                },
                                {i18n.t(Key::RenderPdf)}
                            }
                            if let Some(err) = preview_error.read().as_ref() {
                                span { class: "text-red-600 text-sm", "{err}" }
                            }
                        }
                    } else {
                        div { class: "flex-1 flex items-center justify-center bg-gray-50 border rounded text-gray-400",
                            {i18n.t(Key::NoTemplates)}
                        }
                    }
                }
            }
        }

        // New File modal
        if *show_new_file.read() {
            Modal {
                div { class: "space-y-4",
                    h2 { class: "text-xl font-bold", {i18n.t(Key::NewFile)} }
                    div {
                        label { class: "block text-sm font-medium mb-1", {i18n.t(Key::TemplatePath)} }
                        input {
                            class: "w-full border rounded px-3 py-2",
                            r#type: "text",
                            placeholder: "example.typ",
                            value: "{new_path_input}",
                            oninput: move |e| new_path_input.set(e.value().clone()),
                        }
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                            onclick: create_file,
                            {i18n.t(Key::Create)}
                        }
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded hover:bg-gray-400",
                            onclick: move |_| show_new_file.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }

        // New Folder modal
        if *show_new_folder.read() {
            Modal {
                div { class: "space-y-4",
                    h2 { class: "text-xl font-bold", {i18n.t(Key::NewFolder)} }
                    div {
                        label { class: "block text-sm font-medium mb-1", {i18n.t(Key::TemplatePath)} }
                        input {
                            class: "w-full border rounded px-3 py-2",
                            r#type: "text",
                            placeholder: "subfolder",
                            value: "{new_path_input}",
                            oninput: move |e| new_path_input.set(e.value().clone()),
                        }
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                            onclick: create_folder,
                            {i18n.t(Key::Create)}
                        }
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded hover:bg-gray-400",
                            onclick: move |_| show_new_folder.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }

        // Delete confirmation modal
        if *show_delete_confirm.read() {
            Modal {
                div { class: "space-y-4",
                    h2 { class: "text-xl font-bold text-red-600", {i18n.t(Key::DeleteTemplate)} }
                    p { {i18n.t(Key::ConfirmDeleteTemplate)} }
                    p { class: "font-mono text-sm bg-gray-100 p-2 rounded",
                        {delete_path.read().clone()}
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700",
                            onclick: confirm_delete,
                            {i18n.t(Key::Confirm)}
                        }
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded hover:bg-gray-400",
                            onclick: move |_| show_delete_confirm.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }

        // Unsaved changes warning modal
        if *show_unsaved_warning.read() {
            Modal {
                div { class: "space-y-4",
                    h2 { class: "text-xl font-bold", {i18n.t(Key::UnsavedChanges)} }
                    p { {i18n.t(Key::UnsavedChangesWarning)} }
                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-orange-600 text-white rounded hover:bg-orange-700",
                            onclick: move |_| {
                                show_unsaved_warning.set(false);
                                let path = pending_select_path.read().clone();
                                load_file(path);
                            },
                            {i18n.t(Key::Discard)}
                        }
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded hover:bg-gray-400",
                            onclick: move |_| show_unsaved_warning.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }
    }
}
