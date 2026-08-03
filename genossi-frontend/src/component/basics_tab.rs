//! BasicsTab Component (Phase 4 Plan 06 / W-04 + W-01) — Stamm-Daten + Edit + Open/Close.
//!
//! W-01: D-08-konformer Edit-Mode für update_assembly (D-22). Edit-Button NUR sichtbar
//! im Preparation-Status — nach Open ist update_assembly idR ungewollt (verbandskonform:
//! Stamm-Daten der eröffneten GV bleiben fix; falls Backend trotzdem Update erlaubt,
//! ist das Plan-9-Refinement).
use dioxus::prelude::*;

use crate::api::{self, AssemblyStatusTO, AssemblyTO, UpdateAssemblyRequest};
use crate::component::Modal;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BasicsMode {
    ReadOnly,
    Edit,
}

#[component]
pub fn BasicsTab(
    assembly: AssemblyTO,
    on_changed: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut mode = use_signal(|| BasicsMode::ReadOnly);
    let mut show_open_confirm = use_signal(|| false);
    let mut show_close_confirm = use_signal(|| false);
    let aid = assembly.id;
    let status = assembly.status.clone();
    let date_display = assembly.date.clone().unwrap_or_default();
    let location_display = assembly.location.clone().unwrap_or_default();
    let name_display = assembly.name.clone();
    let version = assembly.version;

    // Edit-Mode local-state
    let mut name_edit = use_signal(|| name_display.clone());
    let mut date_edit = use_signal(|| date_display.clone());
    let mut location_edit = use_signal(|| location_display.clone());
    let mut submitting = use_signal(|| false);

    // Edit only allowed in Preparation
    let can_edit = matches!(status, AssemblyStatusTO::Preparation);
    let is_preparation = matches!(status, AssemblyStatusTO::Preparation);
    let is_open = matches!(status, AssemblyStatusTO::Open);
    let current_mode = *mode.read();

    // Snapshot the display values for re-use inside event closures (avoid moving borrowed strings)
    let name_for_reset = name_display.clone();
    let date_for_reset = date_display.clone();
    let location_for_reset = location_display.clone();

    // Dioxus form+onsubmit+prevent_default loest trotzdem einen Page-Reload aus
    // (Button-Reload-Bug). Fix analog repayment_phases.rs: div-Wrapper +
    // r#type:"button" + onclick statt <form>/submit.
    let submit = move |_| {
        submitting.set(true);
        let req = UpdateAssemblyRequest {
            name: {
                let n = name_edit.read().trim().to_string();
                if n.is_empty() { None } else { Some(n) }
            },
            date: {
                let d = date_edit.read().trim().to_string();
                if d.is_empty() { None } else { Some(d) }
            },
            location: {
                let l = location_edit.read().trim().to_string();
                if l.is_empty() { None } else { Some(l) }
            },
            version: version.unwrap_or_default(),
        };
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::update_assembly(&config, aid, &req).await {
                Ok(_) => {
                    mode.set(BasicsMode::ReadOnly);
                    on_changed.call(());
                }
                Err(err) => on_error.call(err.message),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "bg-white p-6 rounded-lg border border-gray-200",
            if current_mode == BasicsMode::ReadOnly {
                dl { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mb-6",
                    div {
                        dt { class: "text-sm text-gray-500", "{i18n.t(Key::AssemblyName)}" }
                        dd { class: "text-base font-medium", "{name_display}" }
                    }
                    div {
                        dt { class: "text-sm text-gray-500", "{i18n.t(Key::AssemblyDate)}" }
                        dd { class: "text-base", "{date_display}" }
                    }
                    div {
                        dt { class: "text-sm text-gray-500", "{i18n.t(Key::AssemblyLocation)}" }
                        dd { class: "text-base", "{location_display}" }
                    }
                }
                div { class: "flex gap-2",
                    if can_edit {
                        button {
                            r#type: "button",
                            class: "px-4 py-2 text-blue-600 border border-blue-600 hover:bg-blue-50 rounded min-h-[44px]",
                            onclick: move |_| {
                                name_edit.set(name_for_reset.clone());
                                date_edit.set(date_for_reset.clone());
                                location_edit.set(location_for_reset.clone());
                                mode.set(BasicsMode::Edit);
                            },
                            "{i18n.t(Key::Edit)}"
                        }
                    }
                    if is_preparation {
                        button {
                            r#type: "button",
                            class: "bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded min-h-[44px]",
                            onclick: move |_| show_open_confirm.set(true),
                            "{i18n.t(Key::AssemblyOpen)}"
                        }
                    }
                    if is_open {
                        button {
                            r#type: "button",
                            class: "bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded min-h-[44px]",
                            onclick: move |_| show_close_confirm.set(true),
                            "{i18n.t(Key::AssemblyClose)}"
                        }
                    }
                }
            } else {
                div {
                    class: "flex flex-col gap-3",
                    label { class: "flex flex-col gap-1",
                        span { class: "text-sm text-gray-700", "{i18n.t(Key::AssemblyName)}" }
                        input {
                            class: "border border-gray-300 rounded px-3 py-2",
                            r#type: "text",
                            value: "{name_edit}",
                            oninput: move |e| name_edit.set(e.value()),
                        }
                    }
                    label { class: "flex flex-col gap-1",
                        span { class: "text-sm text-gray-700", "{i18n.t(Key::AssemblyDate)}" }
                        input {
                            class: "border border-gray-300 rounded px-3 py-2",
                            r#type: "datetime-local",
                            value: "{date_edit}",
                            oninput: move |e| date_edit.set(e.value()),
                        }
                    }
                    label { class: "flex flex-col gap-1",
                        span { class: "text-sm text-gray-700", "{i18n.t(Key::AssemblyLocation)}" }
                        input {
                            class: "border border-gray-300 rounded px-3 py-2",
                            r#type: "text",
                            value: "{location_edit}",
                            oninput: move |e| location_edit.set(e.value()),
                        }
                    }
                    div { class: "flex gap-2 justify-end mt-2",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| mode.set(BasicsMode::ReadOnly),
                            "{i18n.t(Key::Cancel)}"
                        }
                        button {
                            r#type: "button",
                            onclick: submit,
                            class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded disabled:opacity-50 min-h-[44px]",
                            disabled: *submitting.read(),
                            "{i18n.t(Key::Save)}"
                        }
                    }
                }
            }
        }
        // Open/Close Confirm-Dialogs (Plan 08 hatte sie inline; sind hier zentralisiert)
        if *show_open_confirm.read() {
            Modal {
                div { class: "flex flex-col gap-4",
                    h2 { class: "text-xl font-semibold",
                        "{i18n.t(Key::AssemblyOpenConfirmTitle)}"
                    }
                    p { class: "text-sm text-gray-700",
                        "{i18n.t(Key::AssemblyOpenConfirmText)}"
                    }
                    div { class: "flex gap-2 justify-end mt-2",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| show_open_confirm.set(false),
                            "{i18n.t(Key::Cancel)}"
                        }
                        button {
                            r#type: "button",
                            class: "bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded min-h-[44px]",
                            onclick: move |_| {
                                show_open_confirm.set(false);
                                spawn(async move {
                                    let config = CONFIG.read().clone();
                                    match api::open_assembly(&config, aid).await {
                                        Ok(_) => on_changed.call(()),
                                        Err(err) => on_error.call(err.message),
                                    }
                                });
                            },
                            "{i18n.t(Key::AssemblyOpen)}"
                        }
                    }
                }
            }
        }
        if *show_close_confirm.read() {
            Modal {
                div { class: "flex flex-col gap-4",
                    h2 { class: "text-xl font-semibold",
                        "{i18n.t(Key::AssemblyCloseConfirmTitle)}"
                    }
                    p { class: "text-sm text-gray-700",
                        "{i18n.t(Key::AssemblyCloseConfirmText)}"
                    }
                    div { class: "flex gap-2 justify-end mt-2",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| show_close_confirm.set(false),
                            "{i18n.t(Key::Cancel)}"
                        }
                        button {
                            r#type: "button",
                            class: "bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded min-h-[44px]",
                            onclick: move |_| {
                                show_close_confirm.set(false);
                                spawn(async move {
                                    let config = CONFIG.read().clone();
                                    match api::close_assembly(&config, aid).await {
                                        Ok(_) => on_changed.call(()),
                                        Err(err) => on_error.call(err.message),
                                    }
                                });
                            },
                            "{i18n.t(Key::AssemblyClose)}"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics_mode_copy_eq() {
        let a = BasicsMode::ReadOnly;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(BasicsMode::ReadOnly, BasicsMode::Edit);
    }
}
