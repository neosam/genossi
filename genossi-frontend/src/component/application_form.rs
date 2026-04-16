use dioxus::prelude::*;
use rest_types;
use uuid::Uuid;

use crate::api::{self, AdminCreateApplicationRequest, ApplicationTO, UpdateApplicationRequest};
use crate::component::Modal;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[derive(Clone, PartialEq)]
pub enum ApplicationFormMode {
    Create,
    Edit { application: ApplicationTO },
}

fn salutation_to_string(s: &rest_types::SalutationTO) -> String {
    match s {
        rest_types::SalutationTO::Herr => "Herr".to_string(),
        rest_types::SalutationTO::Frau => "Frau".to_string(),
        rest_types::SalutationTO::Firma => "Firma".to_string(),
    }
}

#[component]
pub fn ApplicationForm(
    mode: ApplicationFormMode,
    on_close: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let is_edit = matches!(mode, ApplicationFormMode::Edit { .. });

    let (
        init_salutation,
        init_title,
        init_first_name,
        init_last_name,
        init_email,
        init_street,
        init_house_number,
        init_postal_code,
        init_city,
        init_shares,
        init_version,
    ) = match &mode {
        ApplicationFormMode::Create => (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "1".to_string(),
            None,
        ),
        ApplicationFormMode::Edit { application } => (
            application
                .salutation
                .as_ref()
                .map(salutation_to_string)
                .unwrap_or_default(),
            application.title.clone().unwrap_or_default(),
            application.first_name.clone(),
            application.last_name.clone(),
            application.email.clone().unwrap_or_default(),
            application.street.clone().unwrap_or_default(),
            application.house_number.clone().unwrap_or_default(),
            application.postal_code.clone().unwrap_or_default(),
            application.city.clone().unwrap_or_default(),
            application.shares.to_string(),
            application.version,
        ),
    };

    let app_id = match &mode {
        ApplicationFormMode::Edit { application } => Some(application.id),
        ApplicationFormMode::Create => None,
    };

    let mut salutation = use_signal(|| init_salutation);
    let mut title = use_signal(|| init_title);
    let mut first_name = use_signal(|| init_first_name);
    let mut last_name = use_signal(|| init_last_name);
    let mut email = use_signal(|| init_email);
    let mut street = use_signal(|| init_street);
    let mut house_number = use_signal(|| init_house_number);
    let mut postal_code = use_signal(|| init_postal_code);
    let mut city = use_signal(|| init_city);
    let mut shares = use_signal(|| init_shares);
    let mut send_mail = use_signal(|| false);
    let mut submitting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let heading = if is_edit {
        i18n.t(Key::EditApplication)
    } else {
        i18n.t(Key::CreateApplication)
    };

    let submit_label = if is_edit {
        i18n.t(Key::Save)
    } else {
        i18n.t(Key::CreateApplication)
    };

    let label_class = "block text-sm font-medium text-gray-700 mb-1";
    let input_class = "w-full border rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500";

    rsx! {
        Modal {
            div { class: "flex justify-between items-center mb-4",
                h2 { class: "text-xl font-semibold", {heading} }
                button {
                    class: "text-gray-400 hover:text-gray-600 text-2xl leading-none",
                    onclick: move |_| on_close.call(()),
                    "\u{00d7}"
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-700 text-sm",
                    "{err}"
                }
            }

            form {
                class: "space-y-4",
                onsubmit: move |evt| {
                    evt.prevent_default();
                    spawn(async move {
                        submitting.set(true);
                        error.set(None);

                        let shares_val = shares.read().parse::<i32>().unwrap_or(1);
                        let salutation_val = {
                            let s = salutation.read().clone();
                            match s.as_str() {
                                "Herr" => Some(rest_types::SalutationTO::Herr),
                                "Frau" => Some(rest_types::SalutationTO::Frau),
                                "Firma" => Some(rest_types::SalutationTO::Firma),
                                _ => None,
                            }
                        };
                        let title_val = {
                            let t = title.read().clone();
                            if t.trim().is_empty() { None } else { Some(t) }
                        };
                        let email_val = {
                            let e = email.read().clone();
                            if e.trim().is_empty() { None } else { Some(e) }
                        };
                        let street_val = {
                            let s = street.read().clone();
                            if s.trim().is_empty() { None } else { Some(s) }
                        };
                        let hn_val = {
                            let h = house_number.read().clone();
                            if h.trim().is_empty() { None } else { Some(h) }
                        };
                        let pc_val = {
                            let p = postal_code.read().clone();
                            if p.trim().is_empty() { None } else { Some(p) }
                        };
                        let city_val = {
                            let c = city.read().clone();
                            if c.trim().is_empty() { None } else { Some(c) }
                        };

                        let config = CONFIG.read().clone();
                        let result = if let Some(id) = app_id {
                            let request = UpdateApplicationRequest {
                                first_name: first_name.read().clone(),
                                last_name: last_name.read().clone(),
                                salutation: salutation_val,
                                title: title_val,
                                email: email_val,
                                street: street_val,
                                house_number: hn_val,
                                postal_code: pc_val,
                                city: city_val,
                                shares: shares_val,
                                version: init_version.unwrap_or(Uuid::nil()),
                            };
                            api::update_application(&config, id, &request).await
                        } else {
                            let send = *send_mail.read();
                            let request = AdminCreateApplicationRequest {
                                first_name: first_name.read().clone(),
                                last_name: last_name.read().clone(),
                                salutation: salutation_val,
                                title: title_val,
                                email: email_val,
                                street: street_val,
                                house_number: hn_val,
                                postal_code: pc_val,
                                city: city_val,
                                shares: shares_val,
                                send_mail: if send { Some(true) } else { None },
                            };
                            api::create_application(&config, &request).await
                        };

                        match result {
                            Ok(_) => on_saved.call(()),
                            Err(e) => error.set(Some(format!("{}", e))),
                        }
                        submitting.set(false);
                    });
                },

                // Salutation + Title row
                div { class: "grid grid-cols-3 gap-4",
                    div {
                        label { class: "{label_class}", {i18n.t(Key::Salutation)} }
                        select {
                            class: "{input_class}",
                            value: "{salutation}",
                            onchange: move |evt| salutation.set(evt.value()),
                            option { value: "", "" }
                            option { value: "Herr", "Herr" }
                            option { value: "Frau", "Frau" }
                            option { value: "Firma", "Firma" }
                        }
                    }
                    div { class: "col-span-2",
                        label { class: "{label_class}", {i18n.t(Key::Title)} }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            placeholder: "Dr., Prof., ...",
                            value: "{title}",
                            oninput: move |evt| title.set(evt.value()),
                        }
                    }
                }

                // Name row
                div { class: "grid grid-cols-2 gap-4",
                    div {
                        label { class: "{label_class}", {i18n.t(Key::FirstName)} " *" }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            required: true,
                            value: "{first_name}",
                            oninput: move |evt| first_name.set(evt.value()),
                        }
                    }
                    div {
                        label { class: "{label_class}", {i18n.t(Key::LastName)} " *" }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            required: true,
                            value: "{last_name}",
                            oninput: move |evt| last_name.set(evt.value()),
                        }
                    }
                }

                // Email
                div {
                    label { class: "{label_class}", {i18n.t(Key::Email)} }
                    input {
                        class: "{input_class}",
                        r#type: "email",
                        value: "{email}",
                        oninput: move |evt| email.set(evt.value()),
                    }
                }

                // Address row
                div { class: "grid grid-cols-3 gap-4",
                    div { class: "col-span-2",
                        label { class: "{label_class}", {i18n.t(Key::Street)} }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            value: "{street}",
                            oninput: move |evt| street.set(evt.value()),
                        }
                    }
                    div {
                        label { class: "{label_class}", "Nr." }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            value: "{house_number}",
                            oninput: move |evt| house_number.set(evt.value()),
                        }
                    }
                }

                div { class: "grid grid-cols-3 gap-4",
                    div {
                        label { class: "{label_class}", "PLZ" }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            value: "{postal_code}",
                            oninput: move |evt| postal_code.set(evt.value()),
                        }
                    }
                    div { class: "col-span-2",
                        label { class: "{label_class}", {i18n.t(Key::City)} }
                        input {
                            class: "{input_class}",
                            r#type: "text",
                            value: "{city}",
                            oninput: move |evt| city.set(evt.value()),
                        }
                    }
                }

                // Shares
                div {
                    label { class: "{label_class}", {i18n.t(Key::Shares)} " *" }
                    input {
                        class: "{input_class} w-24",
                        r#type: "number",
                        min: "1",
                        required: true,
                        value: "{shares}",
                        oninput: move |evt| shares.set(evt.value()),
                    }
                }

                // Send mail toggle (only in create mode)
                if !is_edit {
                    div { class: "flex items-center space-x-2 pt-2",
                        input {
                            r#type: "checkbox",
                            id: "send_mail",
                            checked: *send_mail.read(),
                            onchange: move |evt| send_mail.set(evt.checked()),
                        }
                        label {
                            r#for: "send_mail",
                            class: "text-sm text-gray-700",
                            {i18n.t(Key::SendConfirmationMail)}
                        }
                    }
                }

                // Submit
                div { class: "flex justify-end space-x-3 pt-4 border-t",
                    button {
                        class: "px-4 py-2 border rounded hover:bg-gray-50",
                        r#type: "button",
                        onclick: move |_| on_close.call(()),
                        {i18n.t(Key::Cancel)}
                    }
                    button {
                        class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                        r#type: "submit",
                        disabled: *submitting.read(),
                        if *submitting.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {submit_label}
                        }
                    }
                }
            }
        }
    }
}
