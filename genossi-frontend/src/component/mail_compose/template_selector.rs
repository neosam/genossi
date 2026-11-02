use dioxus::prelude::*;

use crate::api::{self, MailJobTO, MailTemplateTO};
use crate::component::mail_compose::plain_to_html;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;

/// Quick 261102-di4: Remount-Key der Quelle `ComposeSource::None`.
pub const COMPOSE_SOURCE_NONE_KEY: &str = "__no_source__";

/// Quick 261102-di4 (D-06): Obergrenze der im Dropdown angebotenen versendeten
/// Mails. Bis hierhin bleibt ein `<select>` ohne Suche bedienbar.
pub const SENT_MAIL_SOURCE_LIMIT: usize = 20;

/// Quick 261102-di4 (D-02): die aktive Textquelle des Compose-Bodys.
///
/// Loest die Doppelrolle des alten `selected_template_id`-Signals auf, das
/// gleichzeitig Versand-Parameter (`template_id`) UND Remount-Key des
/// `WysiwygEditor` war. Mit zwei Quellen ist das nicht mehr dasselbe: eine
/// versendete Mail hat keine Vorlagen-ID, braucht aber trotzdem einen Key, der
/// sich bei jeder Auswahl aendert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposeSource {
    None,
    Template(String),
    SentJob(String),
}

impl ComposeSource {
    /// Remount-Key des `WysiwygEditor`.
    ///
    /// Muss sich bei **jedem** Quellen-Wechsel aendern, auch zwischen zwei
    /// aufeinanderfolgenden Job-Auswahlen — der Editor seedet `innerHTML` nur
    /// beim Mount, ohne Key-Wechsel bliebe der Text der vorher gewaehlten Mail
    /// stehen (D-02).
    pub fn editor_key(&self) -> String {
        match self {
            ComposeSource::None => COMPOSE_SOURCE_NONE_KEY.to_string(),
            ComposeSource::Template(id) => format!("tpl:{id}"),
            ComposeSource::SentJob(id) => format!("job:{id}"),
        }
    }

    /// Versand-Parameter `template_id` fuer `send_bulk_mail`.
    ///
    /// Nur eine Vorlagen-Quelle liefert eine ID. Bei einer Job-Quelle existiert
    /// keine Vorlage — `None` ist dort korrekt und legitim (Massenversand ohne
    /// Template-Aufloesung); die Job-Id darf NIE als `template_id` durchgereicht
    /// werden (D-02).
    pub fn template_id(&self) -> Option<&str> {
        match self {
            ComposeSource::Template(id) => Some(id.as_str()),
            ComposeSource::None | ComposeSource::SentJob(_) => None,
        }
    }

    /// Quick 261102-elc: die Quelle, die nach einem Callback-Aufruf gilt.
    ///
    /// Das ist die EINE Stelle, an der ein Selector-Callback auf eine
    /// `ComposeSource` abgebildet wird — `mail_page.rs` ruft sie in seinen
    /// Handlern auf, und der Test faltet sie mit `compose_source_after` ueber
    /// eine ganze Callback-Folge. Dadurch kann die Verdrahtung nicht mehr still
    /// von dem abweichen, was der Test prueft.
    ///
    /// `Body` laesst die Quelle absichtlich unveraendert: dieser Callback
    /// befuellt nur Body/Body-HTML und laeuft im Vorlagen-Pfad VOR dem
    /// `Id`, das die Quelle setzt.
    pub fn apply(&self, call: &SelectorCallback) -> ComposeSource {
        match call {
            SelectorCallback::Body(_) => self.clone(),
            SelectorCallback::Id(None) => ComposeSource::None,
            SelectorCallback::Id(Some(id)) => ComposeSource::Template(id.clone()),
            SelectorCallback::SentJob(sel) => ComposeSource::SentJob(sel.job_id.clone()),
        }
    }
}

/// Quick 261102-di4: Nutzlast des `on_select_sent_job`-Callbacks — Betreff und
/// Text einer frueher versendeten Mail. Anhaenge sind bewusst nicht enthalten
/// (D-05).
#[derive(Clone, Debug, PartialEq)]
pub struct SentMailSelection {
    pub job_id: String,
    pub subject: String,
    pub body: String,
    pub body_html: String,
}

/// Quick 261102-elc: ein Callback-Aufruf, den die `<select>`-Auswahl ausloest.
///
/// Macht die Reihenfolge der Callbacks zu Daten statt zu Kontrollfluss im
/// `onchange`-Handler. Grund: der Handler rief fuer eine Job-Auswahl erst
/// `on_select_sent_job` und danach `on_select_id(None)` — der zweite Aufruf
/// ueberschrieb das `compose_source`, das der erste gerade gesetzt hatte, der
/// Remount-Key blieb `__no_source__`, und der `WysiwygEditor` zeigte weiter
/// leeren Text. Als Daten ist diese Folge testbar.
#[derive(Clone, Debug, PartialEq)]
pub enum SelectorCallback {
    /// Prop `on_select` — Plain-Text-Body einer Vorlage.
    Body(String),
    /// Prop `on_select_id` — Versand-Parameter `template_id`, `None` setzt
    /// die Auswahl zurueck.
    Id(Option<String>),
    /// Prop `on_select_sent_job` — Betreff/Text einer versendeten Mail.
    SentJob(SentMailSelection),
}

/// Quick 261102-elc: die REINE Entscheidungslogik des `onchange`-Handlers.
///
/// Liefert fuer eine Auswahl die vollstaendige, geordnete Folge der zu
/// feuernden Callbacks. Der Handler ist danach nur noch ein Adapter, der diese
/// Folge abspielt — die Entscheidung selbst ist ohne Dioxus-Runtime testbar.
///
/// Wichtig fuer den Job-Pfad: die Folge enthaelt **ausschliesslich**
/// `SentJob`. Ein nachgelagertes `Id(None)` waere genau der Bug,
/// den dieser Quick behebt — `ComposeSource::SentJob` liefert per Konstruktion
/// `template_id() == None`, ein Reset ist also nicht nur schaedlich, sondern
/// ueberfluessig.
///
/// `on_select` laeuft auf dem Job-Pfad bewusst nicht: er haengt in
/// `mail_page.rs` den Footer an, der im Body einer versendeten Mail schon
/// enthalten ist (261102-di4 D-08).
pub fn selector_callbacks(
    sent_mode: bool,
    value: &str,
    jobs: &[MailJobTO],
    templates: &[MailTemplateTO],
) -> Vec<SelectorCallback> {
    if sent_mode {
        return match jobs.iter().find(|j| j.id == value) {
            Some(job) => vec![SelectorCallback::SentJob(SentMailSelection {
                job_id: job.id.clone(),
                subject: job.subject.clone(),
                body: job.body.clone(),
                body_html: job_body_html(job),
            })],
            // Platzhalter-Option (leerer Wert) oder unbekannte Id.
            None => vec![SelectorCallback::Id(None)],
        };
    }
    if value.is_empty() {
        // "Vorlage waehlen..."-Option: Auswahl zuruecksetzen.
        return vec![SelectorCallback::Id(None)];
    }
    match templates.iter().find(|t| t.id == value) {
        // Phase 12 D-18: erst der Body, dann die Id — `Id` ist der
        // Aufruf, der die Quelle setzt, und muss deshalb zuletzt kommen.
        Some(tpl) => vec![
            SelectorCallback::Body(tpl.body.clone()),
            SelectorCallback::Id(Some(tpl.id.clone())),
        ],
        // Unbekannte Vorlagen-Id: wie bisher passiert nichts.
        None => Vec::new(),
    }
}

/// Quick 261102-di4 (D-06): die als Textquelle anbietbaren Mail-Jobs.
///
/// Behaelt nur Jobs, die tatsaechlich jemanden erreicht haben (`sent_count > 0`)
/// und einen nicht-leeren Body haben: ein fehlgeschlagener Job ist keine
/// „versendete Mail", sondern Rauschen in einem Dropdown, das nach Betreff
/// gelesen wird; ein leerer Body hat nichts zu uebernehmen.
///
/// Sortiert absteigend nach `created` (ISO8601-UTC, lexikografisch korrekt) und
/// schneidet auf `limit` ab. Die Sortierung vertraut bewusst NICHT der
/// Backend-Reihenfolge (`ORDER BY created DESC`), damit der Helper deterministisch
/// testbar und nicht an eine undokumentierte Zusage gekoppelt ist.
pub fn sent_mail_sources(jobs: &[MailJobTO], limit: usize) -> Vec<MailJobTO> {
    let mut filtered: Vec<MailJobTO> = jobs
        .iter()
        .filter(|j| j.sent_count > 0 && !j.body.trim().is_empty())
        .cloned()
        .collect();
    filtered.sort_by(|a, b| b.created.cmp(&a.created));
    filtered.truncate(limit);
    filtered
}

/// Quick 261102-di4 (D-04): das HTML, mit dem der `WysiwygEditor` fuer eine
/// Job-Quelle geseedet wird.
///
/// Bevorzugt das serverseitig (ammonia-)sanitisierte `body_html` des Jobs — wird
/// es ignoriert und stattdessen aus dem Plain-Text rekonstruiert, verliert die
/// Uebernahme Listen, Ueberschriften und jede weitere Formatierung der
/// Originalmail. Fallback auf `plain_to_html(body)` fuer Legacy-Jobs ohne HTML.
pub fn job_body_html(job: &MailJobTO) -> String {
    match job.body_html.as_deref() {
        Some(h) if !h.trim().is_empty() => h.to_string(),
        _ => plain_to_html(&job.body),
    }
}

/// Quick 261102-di4: Label einer `<option>` im „Versendete Mail"-Dropdown.
///
/// Bewusst locale-frei: der Aufrufer liefert das bereits via
/// `i18n.format_datetime` lokalisierte Datum, damit diese Funktion rein und
/// ohne Dioxus-/i18n-Kontext testbar bleibt.
pub fn sent_mail_option_label(subject: &str, formatted_date: &str) -> String {
    match (subject.trim().is_empty(), formatted_date.trim().is_empty()) {
        (false, false) => format!("{subject} — {formatted_date}"),
        (true, false) => formatted_date.to_string(),
        (false, true) => subject.to_string(),
        (true, true) => String::new(),
    }
}

/// Mail-Template-Selector.
///
/// Phase 12 D-18 / Issue #2 BLOCKER-Fix: `on_select_id` is an additional
/// EventHandler that delivers the selected template's ID (or `None` on reset)
/// alongside the existing `on_select(body)` callback. Backward-compatible:
/// callers that don't need the ID can pass a no-op closure (or use the
/// `#[props(default)]` default).
#[component]
pub fn TemplateSelector(
    on_select: EventHandler<String>,
    #[props(default)] on_select_id: EventHandler<Option<String>>,
    // Phase 32 (D-03): optionaler Zielgruppen-Filter. Ist er gesetzt (z.B.
    // "application"), zeigt das Dropdown nur Vorlagen dieses Typs — die
    // Compose-Seite fuer Antragsteller-Mails blendet damit Mitglieder-Vorlagen
    // aus. `None` = unveraendertes Verhalten (alle Vorlagen), damit die
    // bestehende MailPage-Nutzung identisch bleibt.
    #[props(default)] filter_type: Option<String>,
    // Phase 32 (D-03): optional vorbefuellte Vorlage. Ist eine ID gesetzt, wird
    // sie als initial ausgewaehlter Wert des `<select>` gespiegelt, damit die
    // Compose-Seite mit der „Zahlungserinnerung" vorbefuellt oeffnet. `None` =
    // Platzhalter „Vorlage waehlen…" wie bisher.
    #[props(default)] initial_template_id: Option<String>,
    // Quick 261102-di4 (D-03): opt-in zweite Textquelle „versendete Mail".
    // `false` (Default) haelt Rendering UND Netzwerkverhalten identisch zum
    // Stand vor diesem Quick — insbesondere wird `get_mail_jobs` dann nie
    // aufgerufen. `reply_form.rs` (Antwort auf eine EINGEGANGENE Mail) und
    // `application_compose.rs` brauchen diese Quelle fachlich nicht.
    #[props(default)] allow_sent_mails: bool,
    // Quick 261102-di4 (D-08): eigener Callback fuer die Job-Quelle. Bewusst
    // NICHT `on_select`: der Vorlagen-Pfad in `mail_page.rs` haengt dort den
    // Footer an, der im Body einer versendeten Mail schon enthalten ist — ein
    // zweites Anhaengen wuerde die Grussformel verdoppeln.
    #[props(default)] on_select_sent_job: EventHandler<SentMailSelection>,
) -> Element {
    let i18n = use_i18n();
    let mut templates = use_signal(Vec::<MailTemplateTO>::new);
    // Quick 261102-di4: geladene Mail-Jobs als zweite Textquelle; bleibt leer,
    // solange `allow_sent_mails` false ist (der Request passiert dann nicht).
    let mut jobs = use_signal(Vec::<MailJobTO>::new);
    // Quick 261102-di4: aktive Betriebsart des `<select>` — false = Vorlagen
    // (unveraendertes Verhalten), true = versendete Mails.
    let mut sent_mode = use_signal(|| false);
    // Phase 32 (D-03): gespiegelter Auswahl-Zustand, damit das `<select>` die
    // vorbefuellte bzw. zuletzt gewaehlte Vorlage anzeigt (controlled input).
    let mut selected = use_signal(|| initial_template_id.clone().unwrap_or_default());

    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(data) = api::list_mail_templates(&config).await {
                templates.set(data);
            }
        });
    });

    // Quick 261102-di4 (D-03): Mail-Jobs NUR laden, wenn die zweite Textquelle
    // ueberhaupt angeboten wird. Dass dieser Request bei `allow_sent_mails ==
    // false` nicht passiert, ist die messbare Rueckwaerts-Kompatibilitaets-
    // Garantie fuer die beiden anderen Aufrufstellen. Fehler werden still
    // verworfen — identisch zur Template-Ladelogik oberhalb.
    use_effect(move || {
        if !allow_sent_mails {
            return;
        }
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(data) = api::get_mail_jobs(&config).await {
                jobs.set(data);
            }
        });
    });

    // Phase 32 (D-03): optionale Filterung auf eine Zielgruppe. Bei `None`
    // unveraendert alle geladenen Vorlagen (Backward-Compat MailPage).
    let visible: Vec<MailTemplateTO> = match filter_type.as_ref() {
        Some(t) => api::filter_templates_by_type(&templates.read(), t),
        None => templates.read().clone(),
    };

    // Quick 261102-di4: der Job-Modus ist nur erreichbar, wenn die Quelle
    // ueberhaupt erlaubt ist.
    let in_sent_mode = allow_sent_mails && *sent_mode.read();
    let sent_jobs: Vec<MailJobTO> = if allow_sent_mails {
        sent_mail_sources(&jobs.read(), SENT_MAIL_SOURCE_LIMIT)
    } else {
        Vec::new()
    };
    const SOURCE_BTN_BASE: &str = "px-3 py-1.5 text-sm rounded";
    const SOURCE_BTN_ACTIVE: &str = "bg-blue-600 text-white";
    const SOURCE_BTN_INACTIVE: &str = "bg-white text-gray-700 border";
    let (tpl_btn_class, sent_btn_class) = if in_sent_mode {
        (SOURCE_BTN_INACTIVE, SOURCE_BTN_ACTIVE)
    } else {
        (SOURCE_BTN_ACTIVE, SOURCE_BTN_INACTIVE)
    };

    rsx! {
        div {
            if allow_sent_mails {
                // Quick 261102-di4: Umschaltung der Textquelle IM Selector
                // (Nutzer-Vorgabe: kein zweiter Knopf daneben). `r#type:
                // "button"` + `onclick` — nie `form`/`onsubmit`, das loest in
                // diesem Frontend einen Page-Reload aus.
                div { class: "mb-2",
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::MailSourceLabel)}
                    }
                    div { class: "flex gap-2",
                        button {
                            r#type: "button",
                            class: "{SOURCE_BTN_BASE} {tpl_btn_class}",
                            onclick: move |_| {
                                sent_mode.set(false);
                                // Controlled `<select>`: ohne Reset zeigte es
                                // eine ID aus der anderen Liste.
                                selected.set(String::new());
                                // D-02: beim Quellen-Wechsel darf kein altes
                                // `template_id` als Versand-Parameter stehen
                                // bleiben.
                                on_select_id.call(None);
                            },
                            {i18n.t(Key::MailSourceTemplate)}
                        }
                        button {
                            r#type: "button",
                            class: "{SOURCE_BTN_BASE} {sent_btn_class}",
                            onclick: move |_| {
                                sent_mode.set(true);
                                selected.set(String::new());
                                on_select_id.call(None);
                            },
                            {i18n.t(Key::MailSourceSentMail)}
                        }
                    }
                }
            } else {
                label { class: "block text-sm font-medium text-gray-700 mb-1", "Vorlage" }
            }
            select {
                class: "w-full border rounded px-3 py-2 text-sm",
                value: "{selected}",
                onchange: move |e| {
                    let val = e.value();
                    selected.set(val.clone());
                    // Quick 261102-elc: die Entscheidung trifft die reine
                    // `selector_callbacks`; dieser Handler ist nur noch ihr
                    // Adapter. Die Folge wird VOR dem ersten Callback
                    // vollstaendig berechnet, damit kein Read-Guard auf
                    // `jobs`/`templates` offen ist, waehrend der Aufrufer
                    // Signale schreibt.
                    let calls = {
                        let jobs_ref = jobs.read();
                        let templates_ref = templates.read();
                        selector_callbacks(
                            allow_sent_mails && *sent_mode.read(),
                            &val,
                            &jobs_ref,
                            &templates_ref,
                        )
                    };
                    for call in calls {
                        match call {
                            SelectorCallback::Body(body) => on_select.call(body),
                            SelectorCallback::Id(id) => on_select_id.call(id),
                            SelectorCallback::SentJob(sel) => on_select_sent_job.call(sel),
                        }
                    }
                },
                if in_sent_mode {
                    option { value: "", {i18n.t(Key::MailSourceSentMailSelect)} }
                    for job in sent_jobs.iter() {
                        option {
                            value: "{job.id}",
                            {sent_mail_option_label(&job.subject, &i18n.format_datetime(&job.created))}
                        }
                    }
                } else {
                    option { value: "", {i18n.t(Key::MailTemplateSelect)} }
                    for tpl in visible.iter() {
                        option {
                            value: "{tpl.id}",
                            "{tpl.name}"
                        }
                    }
                }
            }
            div { class: "mt-1",
                Link {
                    to: Route::MailTemplatesPage {},
                    class: "text-sm text-blue-600 hover:underline",
                    {i18n.t(Key::MailTemplateManage)}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(
        id: &str,
        created: &str,
        subject: &str,
        body: &str,
        body_html: Option<&str>,
        sent_count: i64,
    ) -> MailJobTO {
        MailJobTO {
            id: id.to_string(),
            created: created.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            status: "done".to_string(),
            total_count: sent_count,
            sent_count,
            failed_count: 0,
            repayment_phase_id: None,
            body_html: body_html.map(|h| h.to_string()),
        }
    }

    /// Quick 261102-elc: die Quelle nach einer ganzen Callback-Folge.
    ///
    /// Faltet die produktive `ComposeSource::apply` ueber die Folge aus
    /// `selector_callbacks` und beantwortet damit die Frage, die der Fehler
    /// aufwarf: welche Quelle — und damit welcher Remount-Key des
    /// `WysiwygEditor` — gilt am Ende einer Auswahl? Bewusst nur eine Faltung
    /// ohne eigene Logik, damit hier nichts simuliert wird, was produktiv
    /// anders laeuft.
    fn compose_source_after(previous: &ComposeSource, calls: &[SelectorCallback]) -> ComposeSource {
        calls.iter().fold(previous.clone(), |acc, c| acc.apply(c))
    }

    fn tpl(id: &str, name: &str, body: &str) -> MailTemplateTO {
        MailTemplateTO {
            id: id.to_string(),
            name: name.to_string(),
            subject: "Betreff".to_string(),
            body: body.to_string(),
            body_html: None,
            template_type: "member".to_string(),
            version: "v1".to_string(),
        }
    }

    // --- Quick 261102-elc: Handler-Ebene ---------------------------------
    //
    // Die Tests oberhalb/unterhalb pruefen `ComposeSource` und die Helper
    // ISOLIERT. Genau das liess den Fehler durch: der Handler rief fuer eine
    // Job-Auswahl `on_select_sent_job` UND danach `on_select_id(None)`, was
    // das gerade gesetzte `compose_source` sofort auf `None` zuruecksetzte —
    // ein korrektes Enum, von der Verdrahtung aufgehoben. Die folgenden Tests
    // pruefen deshalb die Callback-FOLGE und die daraus resultierende Quelle.

    /// Der Regressionstest fuer 261102-elc: eine Job-Auswahl feuert GENAU
    /// einen Callback. Ein nachgelagertes `SelectId(None)` laesst diesen Test
    /// scheitern — sowohl an der Folge selbst als auch an der resultierenden
    /// Quelle.
    #[test]
    fn selecting_sent_mail_fires_only_the_job_callback() {
        let jobs = vec![job(
            "job-a",
            "2026-11-01T10:00:00Z",
            "Einladung",
            "Hallo {{ first_name }}",
            Some("<p>Hallo {{ first_name }}</p>"),
            3,
        )];
        let calls = selector_callbacks(true, "job-a", &jobs, &[]);

        assert_eq!(
            calls,
            vec![SelectorCallback::SentJob(SentMailSelection {
                job_id: "job-a".to_string(),
                subject: "Einladung".to_string(),
                body: "Hallo {{ first_name }}".to_string(),
                body_html: "<p>Hallo {{ first_name }}</p>".to_string(),
            })],
            "eine Job-Auswahl darf kein nachgelagertes on_select_id(None) feuern — \
             das ueberschreibt die gerade gesetzte Quelle"
        );
    }

    /// Dieselbe Eigenschaft als Wirkung formuliert: die Quelle nach der
    /// Auswahl ist die Job-Quelle, nicht `None`. Mit dem alten
    /// `on_select_id.call(None)` waere das Ergebnis `ComposeSource::None` und
    /// der Editor-Key der unveraenderte `__no_source__` — kein Remount, leeres
    /// Textfeld.
    #[test]
    fn selecting_sent_mail_results_in_sent_job_source() {
        let jobs = vec![job(
            "job-a",
            "2026-11-01T10:00:00Z",
            "Einladung",
            "Text",
            None,
            1,
        )];
        let calls = selector_callbacks(true, "job-a", &jobs, &[]);
        let source = compose_source_after(&ComposeSource::None, &calls);

        assert_eq!(source, ComposeSource::SentJob("job-a".to_string()));
        assert_ne!(
            source.editor_key(),
            COMPOSE_SOURCE_NONE_KEY,
            "ohne Key-Wechsel gegenueber dem Ausgangszustand remountet der \
             WysiwygEditor nicht und bleibt leer"
        );
        assert!(
            source.template_id().is_none(),
            "eine versendete Mail ist keine Vorlage"
        );
    }

    /// Dieselbe Aussage wie `editor_key_differs_between_two_sent_jobs`, aber
    /// eine Ebene hoeher: ueber die Callback-Folge des Handlers statt ueber das
    /// Enum allein. Die zweite Auswahl startet von der Quelle, die die erste
    /// hinterlassen hat.
    #[test]
    fn two_sent_mails_in_a_row_yield_different_editor_keys() {
        let jobs = vec![
            job("job-a", "2026-11-02T10:00:00Z", "A", "Text A", None, 1),
            job("job-b", "2026-11-01T10:00:00Z", "B", "Text B", None, 1),
        ];

        let first = compose_source_after(
            &ComposeSource::None,
            &selector_callbacks(true, "job-a", &jobs, &[]),
        );
        let second = compose_source_after(&first, &selector_callbacks(true, "job-b", &jobs, &[]));

        assert_eq!(first, ComposeSource::SentJob("job-a".to_string()));
        assert_eq!(second, ComposeSource::SentJob("job-b".to_string()));
        assert_ne!(first.editor_key(), second.editor_key());
    }

    /// Der Vorlagen-Pfad bleibt unveraendert: Body-Callback zuerst, dann die
    /// Id, und die Quelle ist die Vorlage samt Versand-Parameter.
    #[test]
    fn selecting_template_still_sets_template_source_and_id() {
        let templates = vec![tpl("tpl-1", "Zahlungserinnerung", "Hallo")];
        let calls = selector_callbacks(false, "tpl-1", &[], &templates);

        assert_eq!(
            calls,
            vec![
                SelectorCallback::Body("Hallo".to_string()),
                SelectorCallback::Id(Some("tpl-1".to_string())),
            ]
        );

        let source = compose_source_after(&ComposeSource::None, &calls);
        assert_eq!(source, ComposeSource::Template("tpl-1".to_string()));
        assert_eq!(source.template_id(), Some("tpl-1"));
    }

    /// Nach einer Job-Auswahl muss eine Vorlagen-Auswahl die Quelle wieder auf
    /// die Vorlage drehen — die Gegenrichtung des behobenen Fehlers.
    #[test]
    fn template_after_sent_mail_overwrites_the_job_source() {
        let jobs = vec![job("job-a", "2026-11-01T10:00:00Z", "A", "Text", None, 1)];
        let templates = vec![tpl("tpl-1", "Vorlage", "Hallo")];

        let after_job = compose_source_after(
            &ComposeSource::None,
            &selector_callbacks(true, "job-a", &jobs, &[]),
        );
        let after_tpl = compose_source_after(
            &after_job,
            &selector_callbacks(false, "tpl-1", &jobs, &templates),
        );

        assert_eq!(after_tpl, ComposeSource::Template("tpl-1".to_string()));
        assert_eq!(after_tpl.template_id(), Some("tpl-1"));
    }

    /// Die Platzhalter-Option setzt in BEIDEN Modi zurueck — dort ist
    /// `on_select_id(None)` korrekt und bleibt erhalten.
    #[test]
    fn placeholder_option_resets_the_source_in_both_modes() {
        let jobs = vec![job("job-a", "2026-11-01T10:00:00Z", "A", "Text", None, 1)];
        let templates = vec![tpl("tpl-1", "Vorlage", "Hallo")];

        for (sent_mode, previous) in [
            (true, ComposeSource::SentJob("job-a".to_string())),
            (false, ComposeSource::Template("tpl-1".to_string())),
        ] {
            let calls = selector_callbacks(sent_mode, "", &jobs, &templates);
            assert_eq!(calls, vec![SelectorCallback::Id(None)]);
            assert_eq!(
                compose_source_after(&previous, &calls),
                ComposeSource::None,
                "Platzhalter-Option muss die Quelle zuruecksetzen (sent_mode={sent_mode})"
            );
        }
    }

    /// Eine unbekannte Job-Id (Liste inzwischen neu geladen) verhaelt sich wie
    /// die Platzhalter-Option; eine unbekannte Vorlagen-Id aendert wie bisher
    /// nichts.
    #[test]
    fn unknown_ids_keep_the_previous_behaviour() {
        assert_eq!(
            selector_callbacks(true, "ghost", &[], &[]),
            vec![SelectorCallback::Id(None)]
        );
        assert_eq!(
            selector_callbacks(false, "ghost", &[], &[]),
            Vec::<SelectorCallback>::new()
        );
    }

    /// `SelectBody` darf die Quelle nicht anfassen — genau diese Eigenschaft
    /// macht die Reihenfolge im Vorlagen-Pfad unkritisch.
    #[test]
    fn select_body_leaves_the_source_untouched() {
        let previous = ComposeSource::SentJob("job-a".to_string());
        assert_eq!(
            previous.apply(&SelectorCallback::Body("Hallo".to_string())),
            previous
        );
    }

    /// Zentraler Regressions-Pin fuer D-02: ohne diese Eigenschaft bleibt der
    /// Remount-Key bei der zweiten Job-Auswahl gleich, der WysiwygEditor
    /// remountet nicht und zeigt weiter den Text der ERSTEN Mail — das Feature
    /// sieht dann schlicht kaputt aus.
    #[test]
    fn editor_key_differs_between_two_sent_jobs() {
        let a = ComposeSource::SentJob("job-a".to_string());
        let b = ComposeSource::SentJob("job-b".to_string());
        assert_ne!(
            a.editor_key(),
            b.editor_key(),
            "zwei verschiedene versendete Mails muessen verschiedene Editor-Keys ergeben"
        );
    }

    #[test]
    fn editor_key_differs_between_template_and_job_with_same_id() {
        let tpl = ComposeSource::Template("same-id".to_string());
        let job = ComposeSource::SentJob("same-id".to_string());
        assert_ne!(tpl.editor_key(), job.editor_key());
    }

    #[test]
    fn editor_key_for_none_is_the_none_constant() {
        assert_eq!(ComposeSource::None.editor_key(), COMPOSE_SOURCE_NONE_KEY);
    }

    /// Pin fuer „Job-Quelle setzt kein template_id": eine versendete Mail ist
    /// keine Vorlage, die Job-Id darf nie als Versand-Parameter durchgereicht
    /// werden.
    #[test]
    fn template_id_is_none_for_sent_job_and_none_source() {
        assert!(ComposeSource::SentJob("job-a".to_string())
            .template_id()
            .is_none());
        assert!(ComposeSource::None.template_id().is_none());
    }

    #[test]
    fn template_id_is_some_for_template() {
        assert_eq!(
            ComposeSource::Template("tpl-1".to_string()).template_id(),
            Some("tpl-1")
        );
    }

    #[test]
    fn job_body_html_prefers_sanitized_html() {
        let j = job(
            "j1",
            "2026-11-01T10:00:00Z",
            "S",
            "a\nb",
            Some("<ul><li>a</li></ul>"),
            1,
        );
        assert_eq!(job_body_html(&j), "<ul><li>a</li></ul>");
    }

    #[test]
    fn job_body_html_falls_back_to_plain_to_html() {
        let without = job("j1", "2026-11-01T10:00:00Z", "S", "a\nb", None, 1);
        assert_eq!(job_body_html(&without), plain_to_html("a\nb"));
        assert_eq!(job_body_html(&without), "a<br>b");

        let blank = job("j2", "2026-11-01T10:00:00Z", "S", "a\nb", Some("   "), 1);
        assert_eq!(job_body_html(&blank), plain_to_html("a\nb"));
    }

    #[test]
    fn sent_mail_sources_skips_jobs_without_successful_send() {
        let jobs = vec![
            job(
                "fail",
                "2026-11-02T10:00:00Z",
                "Fehlgeschlagen",
                "Text",
                None,
                0,
            ),
            job("ok", "2026-11-01T10:00:00Z", "Versendet", "Text", None, 2),
        ];
        let out = sent_mail_sources(&jobs, SENT_MAIL_SOURCE_LIMIT);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "ok");
    }

    #[test]
    fn sent_mail_sources_skips_empty_bodies() {
        let jobs = vec![
            job("empty", "2026-11-02T10:00:00Z", "Leer", "   \n ", None, 3),
            job("ok", "2026-11-01T10:00:00Z", "Versendet", "Text", None, 1),
        ];
        let out = sent_mail_sources(&jobs, SENT_MAIL_SOURCE_LIMIT);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "ok");
    }

    #[test]
    fn sent_mail_sources_sorts_newest_first_and_applies_limit() {
        let jobs = vec![
            job("mid", "2026-10-15T10:00:00Z", "B", "Text", None, 1),
            job("old", "2026-01-01T10:00:00Z", "C", "Text", None, 1),
            job("new", "2026-11-02T10:00:00Z", "A", "Text", None, 1),
        ];
        let out = sent_mail_sources(&jobs, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "new");
        assert_eq!(out[1].id, "mid");
    }

    #[test]
    fn sent_mail_option_label_joins_subject_and_date() {
        assert_eq!(
            sent_mail_option_label("Einladung GV", "02.11.2026 14:30"),
            "Einladung GV — 02.11.2026 14:30"
        );
    }

    #[test]
    fn sent_mail_option_label_without_subject_is_date_only() {
        assert_eq!(
            sent_mail_option_label("", "02.11.2026 14:30"),
            "02.11.2026 14:30"
        );
    }

    #[test]
    fn sent_mail_option_label_without_date_is_subject_only() {
        assert_eq!(sent_mail_option_label("Einladung GV", ""), "Einladung GV");
    }
}
