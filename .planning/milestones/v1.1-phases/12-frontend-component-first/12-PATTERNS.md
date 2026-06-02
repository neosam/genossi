# Phase 12: Frontend (Component-First) - Pattern Map

**Mapped:** 2026-06-01
**Files analyzed:** 17 (10 neu, 7 erweitert)
**Analogs found:** 17 / 17 (alle Reuse-Anker im Code verifiziert)

## File Classification

### Neue Dateien (10)

| Neue Datei | Role | Data Flow | Closest Analog | Match Quality |
|-----------|------|-----------|----------------|---------------|
| `genossi-frontend/src/page/repayment_phases.rs` | page (Liste + Create-Modal) | request-response (GET-Liste + POST-Create) | `genossi-frontend/src/page/assemblies.rs` | exact (Liste + Create-Modal-Pattern) |
| `genossi-frontend/src/page/repayment_phase_details.rs` | page (3-Tab-Detail) | request-response + Lifecycle | `genossi-frontend/src/page/assembly_details.rs` | exact (TabStrip + Lifecycle + ExportTab) |
| `genossi-frontend/src/component/repayment_phase_status_badge.rs` | component (Badge) | pure-render | `genossi-frontend/src/component/assembly_status_badge.rs` | exact (1:1 Klon, andere Farbpalette) |
| `genossi-frontend/src/component/repayment_entry_status_badge.rs` | component (Badge) | pure-render | `genossi-frontend/src/component/assembly_status_badge.rs` | exact (1:1 Klon, andere Farbpalette) |
| `genossi-frontend/src/component/repayment_entry_list.rs` | component (Multi-Select-Table) | client-side join + filter + bulk-actions | `genossi-frontend/src/page/mail_page.rs` (Recipient-Picker, Z. 226-373) + `genossi-frontend/src/component/attendance_list.rs` (List + on_toggle) | partial (Multi-Select aus mail_page; Listen-Skelett aus attendance_list) |
| `genossi-frontend/src/component/repayment_entry_add_modal.rs` | component (Modal + Form) | request-response (POST) | `genossi-frontend/src/page/assemblies.rs::CreateAssemblyForm` Z. 96-184 | exact (Form-in-Modal-Pattern) — kombiniert mit `MemberSearch`-Direct-Reuse |
| `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` | component (Confirm-Modal + Bulk-Loop) | sequential POST + toast-fan-out | `genossi-frontend/src/component/basics_tab.rs` Z. 215-249 (Close-Confirm-Modal) + `genossi-frontend/src/component/attendance_list.rs` on_toggle-Wiring | partial (Modal-Pattern + Sequential-Loop-Wiring) |
| `genossi-frontend/src/component/editable_share_count_cell.rs` | component (Inline-Cell-Edit, NEU) | local-state + commit-callback | **KEIN direkter Analog** — siehe „No Analog Found" | new pattern |
| `genossi-frontend/src/component/repayment_phase_list_row.rs` (optional) | component (List-Row) | pure-render | `genossi-frontend/src/component/assembly_list_row.rs` | exact |
| `genossi-frontend/src/page/repayment_phase_details.rs::BasicsTab` (inline-fn ODER eigener Component) | component (Stamm-Tab + Lifecycle + share_value-Edit) | request-response (PUT, open, close) | `genossi-frontend/src/component/basics_tab.rs` | exact (ReadOnly/Edit-Mode + Confirm-Modals) |

### Modifizierte Dateien (7)

| Modifizierte Datei | Role | Modification | Analog für die Erweiterung |
|---|---|---|---|
| `genossi-frontend/src/api.rs` | api-extension | +12 API-Funktionen + 2 lokale 409-Response-Strukturen + `repayment_phase_id` Optional-Feld in `SendBulkMailRequest` | api.rs Z. 1659-1722 (Assembly-API-Block) als Standard-Pattern |
| `genossi-frontend/src/router.rs` | router | +2 Route-Enum-Varianten + 2 Page-Re-Exports | router.rs Z. 34-37 (Assemblies + AssemblyDetails) |
| `genossi-frontend/src/page/mod.rs` | mod-reexport | +2 `pub mod ... pub use ...` | page/mod.rs Z. 3-4 + 24-25 |
| `genossi-frontend/src/component/mod.rs` | mod-reexport | +5-7 Component-Re-Exports + Phase-12-Kommentar | component/mod.rs Z. 60-92 (Phase-4-Plan-06-Block) |
| `genossi-frontend/src/component/top_bar.rs` | nav-extension | +1 NavItem „Anteils-Rückzahlung" in verwaltung_items oder kommunikation_items | top_bar.rs Z. 90-103 (NavItem-Push-Pattern) |
| `genossi-frontend/src/component/mail_compose/template_var_buttons.rs` | component (Erweiterung) | +Optional-Prop `extra_vars` ODER `show_repayment_vars` + bedingte Render-Schleife | template_var_buttons.rs Z. 28-90 (vorhandener PRIMARY_VARS + Toggle-Show-More) |
| `genossi-frontend/src/page/mail_page.rs` | page (Erweiterung) | +Query-Param-Parsing via `web_sys::UrlSearchParams` in `use_effect` + Pre-Selection von `selected_member_ids` + `repayment_phase_id` an `send_bulk_mail`-Body | mail_page.rs Z. 51-77 (`selected_member_ids`-Signal + `use_effect`-Init) |
| `genossi-frontend/src/i18n/mod.rs` + `de.rs` + `en.rs` | i18n | +~25-30 Key-Enum-Varianten + Match-Arms in beiden Locales | i18n/mod.rs (existierender `Key`-Enum-Block); de.rs Z. 4-30 + en.rs (Match-Pattern) |

## Pattern Assignments

### 1. `genossi-frontend/src/page/repayment_phases.rs` (page, request-response Liste + Create)

**Analog:** `genossi-frontend/src/page/assemblies.rs` (1:1-Vorbild — Liste mit Create-Modal, ToastContainer, RequirePrivilege)

**Imports-Pattern** (assemblies.rs Z. 6-13):
```rust
use dioxus::prelude::*;

use crate::api::{self, AssemblyTO, CreateAssemblyRequest};
use crate::auth::RequirePrivilege;
use crate::component::{AssemblyListRow, Modal, ToastContainer, TopBar, show_toast};
use crate::i18n::{use_i18n, Key};
use crate::page::access_denied::AccessDeniedPage;
use crate::service::config::CONFIG;
```

**Auth-Gate + Page-Shell** (assemblies.rs Z. 41-46):
```rust
rsx! {
    RequirePrivilege {
        privilege: "admin",
        fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
        TopBar {}
        div { class: "container mx-auto px-4 py-6",
            // ... page content
```

**Liste laden + Toast-Error** (assemblies.rs Z. 25-39):
```rust
let load = move || {
    spawn(async move {
        loading.set(true);
        let config = CONFIG.read().clone();
        match api::list_assemblies(&config).await {
            Ok(list) => assemblies.set(list),
            Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
        }
        loading.set(false);
    });
};

use_effect(move || {
    load();
});
```

**Create-Modal-Trigger + Modal-Mount** (assemblies.rs Z. 78-89):
```rust
if *show_create.read() {
    Modal {
        CreateAssemblyForm {
            on_close: move |_| show_create.set(false),
            on_created: move |_| {
                show_create.set(false);
                load();
            },
            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
        }
    }
}
```

**Header-Button-Pattern (D-01 button type)** (assemblies.rs Z. 49-54):
```rust
button {
    r#type: "button",
    class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm min-h-[44px]",
    onclick: move |_| show_create.set(true),
    "{i18n.t(Key::AssemblyCreate)}"
}
```

**Empty-State** (assemblies.rs Z. 59-69):
```rust
} else if assemblies.read().is_empty() {
    div { class: "text-center py-12",
        p { class: "text-lg font-medium text-gray-700", "{i18n.t(Key::AssemblyEmpty)}" }
        p { class: "text-sm text-gray-500 mt-2 mb-6", "{i18n.t(Key::AssemblyEmptyHint)}" }
        button {
            r#type: "button",
            class: "bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded min-h-[44px]",
            onclick: move |_| show_create.set(true),
            "{i18n.t(Key::AssemblyCreate)}"
        }
    }
}
```

**Default-Sort-Anpassung (D-14):** `fiscal_year DESC, created DESC` als Pure-Func `sort_phases_default(phases: &[RepaymentPhaseTO]) -> Vec<&RepaymentPhaseTO>` analog `member_search.rs::filter_members` Z. 9-35 (Pure-Func + Unit-Tests).

---

### 2. `genossi-frontend/src/page/repayment_phase_details.rs` (page, 3-Tab-Detail)

**Analog:** `genossi-frontend/src/page/assembly_details.rs` (1:1-Vorbild — TabStrip + Lifecycle + ExportTab)

**Imports + UUID-Parse-Pattern** (assembly_details.rs Z. 17-40):
```rust
use dioxus::prelude::*;
use std::str::FromStr;
use uuid::Uuid;

use crate::api::{self, AssemblyStatusTO, AssemblyTO, ...};
use crate::auth::RequirePrivilege;
use crate::component::{
    AssemblyStatusBadge, ..., Modal, TabDef, TabStrip, ToastContainer, TopBar, show_toast,
};
use crate::i18n::{use_i18n, Key};
use crate::page::access_denied::AccessDeniedPage;
use crate::service::config::CONFIG;

#[component]
pub fn AssemblyDetails(id: String) -> Element {
    let i18n = use_i18n();
    let assembly_id = match Uuid::from_str(&id) {
        Ok(u) => u,
        Err(_) => return rsx! { div { class: "p-4 text-red-600", "Invalid assembly id" } },
    };

    let mut assembly = use_signal(|| Option::<AssemblyTO>::None);
    let mut loading = use_signal(|| true);
    let mut active_tab = use_signal(|| "basics".to_string());
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);
```

**TabStrip-Mount + Static-3-Tab-Layout (D-06 — alle Tabs immer sichtbar)** (assembly_details.rs Z. 83-152, ANGEPASST: kein dynamischer 4. Tab):
```rust
let tab_defs = vec![
    TabDef { key: "basics",  label: i18n.t(Key::RepaymentPhaseTabBasics).to_string() },
    TabDef { key: "entries", label: i18n.t(Key::RepaymentPhaseTabEntries).to_string() },
    TabDef { key: "export",  label: i18n.t(Key::RepaymentPhaseTabExport).to_string() },
];
let active_key = active_tab.read().clone();
let status_value = phase.status.clone();
rsx! {
    TabStrip {
        tabs: tab_defs,
        active_key: active_key.clone(),
        on_change: move |k: String| active_tab.set(k),
        match active_key.as_str() {
            "basics" => rsx! { BasicsTab { phase: phase_for_basics, on_changed: move |_| load(), on_error: ... } },
            "entries" => match status_value {
                RepaymentPhaseStatusTO::Preparation => rsx! {
                    div { class: "text-center py-12 text-gray-500",
                        "{i18n.t(Key::RepaymentEntriesNotOpenYet)}"
                    }
                },
                _ => rsx! { RepaymentEntryList { phase: phase_for_list, ... } },
            },
            "export" => match status_value {
                RepaymentPhaseStatusTO::Preparation => rsx! {
                    div { class: "text-center py-12 text-gray-500",
                        "{i18n.t(Key::RepaymentExportNotOpenYet)}"
                    }
                },
                _ => rsx! { RepaymentExportTab { phase: phase_for_export, on_error: ... } },
            },
            _ => rsx! {},
        }
    }
}
```

**Header (Titel + Status-Badge, KEINE Lifecycle-Buttons — D-03):** (assembly_details.rs Z. 78-81):
```rust
div { class: "flex items-center justify-between mb-4",
    h1 { class: "text-2xl font-bold", "{phase_fiscal_year_title}" }
    RepaymentPhaseStatusBadge { status: phase.status.clone() }
}
```

**Toast + ConnectionBanner-Footer:** (assembly_details.rs Z. 158):
```rust
ToastContainer { messages: toast_messages }
```

**ExportTab als Page-internal Component** (assembly_details.rs Z. 322-511, kürzer-Variante für PDF-only):
- Format-Picker (PDF hardcoded für Phase 12; CSV deferred)
- Include-Filter-Radio: `open` | `all` | `paid` (D-26)
- Download via `<a>`-Element + `web_sys::HtmlElement::click()` (assembly_details.rs Z. 369-385)
- ODER simpler: direkter `<a href="..." target="_blank">`-Link (Plan-Discretion per D-26)

---

### 3. `genossi-frontend/src/component/repayment_phase_status_badge.rs` + `repayment_entry_status_badge.rs` (Badges)

**Analog:** `genossi-frontend/src/component/assembly_status_badge.rs` (1:1-Klon, andere Farbpalette + Status-Enum)

**Komplette Vorlage** (assembly_status_badge.rs Z. 1-39):
```rust
use dioxus::prelude::*;

use crate::api::AssemblyStatusTO;  // → RepaymentPhaseStatusTO / RepaymentEntryStatusTO
use crate::i18n::{use_i18n, Key};

fn status_label(i18n: &crate::i18n::I18n, status: &AssemblyStatusTO) -> String {
    match status {
        AssemblyStatusTO::Preparation => i18n.t(Key::AssemblyStatusPreparation).to_string(),
        AssemblyStatusTO::Open => i18n.t(Key::AssemblyStatusOpen).to_string(),
        AssemblyStatusTO::Closed => i18n.t(Key::AssemblyStatusClosed).to_string(),
    }
}

fn status_badge_class(status: &AssemblyStatusTO) -> &'static str {
    match status {
        AssemblyStatusTO::Preparation => {
            "bg-gray-100 text-gray-800 px-2 py-1 rounded text-xs font-medium"
        }
        AssemblyStatusTO::Open => {
            "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium"
        }
        AssemblyStatusTO::Closed => {
            "bg-blue-100 text-blue-800 px-2 py-1 rounded text-xs font-medium"
        }
    }
}

#[component]
pub fn AssemblyStatusBadge(status: AssemblyStatusTO) -> Element {
    let i18n = use_i18n();
    let label = status_label(&i18n, &status);
    let class = status_badge_class(&status);
    rsx! { span { class: "{class}", "{label}" } }
}
```

**Phase-12-Adaption (Farbpalette laut D-14 + Claude's Discretion):**
- **RepaymentPhaseStatusBadge:** Preparation=grau, **Open=blau**, **Closed=grün** (NOTE: nicht identisch zu Assembly — Phase-12 dreht Open/Closed-Farben)
- **RepaymentEntryStatusBadge:** Open=grau, **Contacted=blau**, **PaidOut=grün**

**Unit-Tests-Pattern** (assembly_status_badge.rs Z. 41-81): 1 Test pro Farb-Branch + 1 Test für gemeinsame Pill-Klassen.

---

### 4. `genossi-frontend/src/component/repayment_entry_list.rs` (multi-select table)

**Analog (Multi-Select):** `genossi-frontend/src/page/mail_page.rs` Z. 51-373 (selected_member_ids-Signal + filter_members-Reuse + Header-Action-Leiste)

**Analog (Dumb-List + on_toggle):** `genossi-frontend/src/component/attendance_list.rs` Z. 42-80 (Toggle-Request-Payload + Polling-Free-Liste)

**Multi-Select-Signal-Pattern** (mail_page.rs Z. 51-54, 250-258, 318-322, 343-364):
```rust
let mut selected_entry_ids = use_signal(|| Vec::<Uuid>::new());

// Add:
selected_entry_ids.write().push(entry_id);

// Remove:
selected_entry_ids.write().retain(|id| *id != entry_id);

// All:
selected_entry_ids.set(all_filtered_ids);

// Clear:
selected_entry_ids.set(Vec::new());

// Count:
let count = selected_entry_ids.read().len();
```

**Toggle-Request-Payload-Pattern** (attendance_list.rs Z. 42-46):
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct AttendanceToggleRequest {
    pub member_id: Uuid,
    pub current_is_present: bool,
}
```
Adaption für Phase 12: `RepaymentEntryAction { entry_id: Uuid, action_kind: ActionKind }` ODER direkt EventHandler-Props (`on_mark_contacted`, `on_mark_paidout`, `on_edit_share_count`, `on_delete`).

**Header-Action-Leiste mit Count-Badges (D-11)** — Pattern aus mail_page.rs Z. 342-364 (Header-Buttons "Alle"/"Clear" mit Count):
```rust
button {
    r#type: "button",
    class: if count == 0 { "bg-gray-200 ... opacity-50 cursor-not-allowed" } else { "bg-blue-600 hover:bg-blue-700 text-white ..." },
    disabled: count == 0,
    onclick: move |_| on_bulk_mark_contacted.call(selected_entry_ids.read().clone()),
    "Als angeschrieben markieren ({count})"
}
```

**Status-Filter-Tab-Strip-im-Tab (D-12):**
Filter-Tabs IM Tab-Body (NICHT der Page-Level TabStrip). Pattern: `use_signal(|| StatusFilter::All)` + button-Liste mit `bg-blue-500`/`bg-gray-200`-Toggle. Verifiziert: Pattern `if is_selected { ... } else { ... }`-Klassen-Toggle in `assembly_details.rs::ExportTab` Z. 432-436 (Format-Cards).

**Client-Side-Join Member ↔ Entry (D-10)** — Pattern aus mail_page.rs Z. 232-236 (find_member-by-id):
```rust
let members_state = MEMBERS.read();
for entry in filtered_entries.iter() {
    let member: Option<&MemberTO> = members_state.items.iter().find(|m| m.id == Some(entry.member_id));
    // … render row with member.first_name etc.
}
```

**Pure-Helper für Tests** (analog `member_search.rs::filter_members` Z. 9-35):
- `fn filter_entries_by_status(entries: &[RepaymentEntryTO], filter: StatusFilter) -> Vec<&RepaymentEntryTO>`
- `fn sort_entries_default(entries: &[&RepaymentEntryTO], members: &[MemberTO]) -> Vec<&RepaymentEntryTO>` (Mitgliedsnummer ASC, sekundär created ASC)
- `fn format_payout_eur(share_count: i32, share_value_cents: i64) -> String` (D-10: „60,00 €" mit Euro-Symbol — `i18n::format_price` liefert „60,00 EUR", siehe RESEARCH Risk 5)

**Format-EUR-Helper (D-10, NICHT i18n::format_price reusen — siehe Pitfall):**
```rust
fn format_payout_eur(share_count: i32, share_value_cents: i64) -> String {
    let total_cents = (share_count as i64) * share_value_cents;
    let euros = total_cents / 100;
    let cents_rem = (total_cents.abs() % 100) as u32;
    format!("{},{:02} €", euros, cents_rem)
}
```

---

### 5. `genossi-frontend/src/component/repayment_entry_add_modal.rs` (Add-Modal + Member-Picker)

**Analog (Form-in-Modal):** `genossi-frontend/src/page/assemblies.rs::CreateAssemblyForm` Z. 96-184

**Analog (Member-Picker-Reuse):** `genossi-frontend/src/component/member_search.rs::MemberSearch` Z. 41-46 (direkter Reuse, D-21)

**Form-Pattern** (assemblies.rs Z. 110-139):
```rust
rsx! {
    form {
        class: "flex flex-col gap-4",
        onsubmit: move |e| {
            e.prevent_default();      // synchron VOR spawn (D-01)
            if selected_member_id.read().is_none() {
                on_error.call("Bitte Mitglied auswählen".into());
                return;
            }
            submitting.set(true);
            let req = CreateRepaymentEntryRequest {
                phase_id,
                member_id: selected_member_id.read().unwrap(),
                share_count_to_pay_out: *share_count.read(),
            };
            spawn(async move {
                let config = CONFIG.read().clone();
                match api::create_repayment_entry(&config, &req).await {
                    Ok(_) => on_created.call(()),
                    Err(e) => on_error.call(e.message),
                }
                submitting.set(false);
            });
        },
        // labels + inputs + buttons …
    }
}
```

**MemberSearch-Reuse-Pattern (D-21)** — Aufruf-Signatur (member_search.rs Z. 41-46):
```rust
MemberSearch {
    on_select: move |id: Option<Uuid>| {
        selected_member_id.set(id);
        // D-22: bei Select mit current_shares vorbefüllen
        if let Some(uid) = id {
            let members = MEMBERS.read();
            if let Some(m) = members.items.iter().find(|m| m.id == Some(uid)) {
                share_count.set(m.current_shares);
            }
        }
    },
    selected_id: selected_member_id.read().clone(),
    exclude_id: None,
}
```

**Submit-Button (D-01)** (assemblies.rs Z. 175-180):
```rust
button {
    r#type: "submit",
    class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded disabled:opacity-50 min-h-[44px]",
    disabled: *submitting.read() || selected_member_id.read().is_none() || *share_count.read() <= 0,
    "{i18n.t(Key::Save)}"
}
```

**Pure-Helper für Tests:**
- `fn validate_create_entry(member_id: Option<Uuid>, share_count: i32) -> Result<(), ValidationError>` (D-23 minimal: > 0 + Member-Pflicht)

---

### 6. `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` (Bulk-Confirm + Sequential-Loop)

**Analog (Confirm-Modal):** `genossi-frontend/src/component/basics_tab.rs` Z. 215-249 (Close-Confirm-Modal mit ja/nein-Buttons)

**Analog (Sequential-Loop + Toast pro Eintrag):** `genossi-frontend/src/page/assembly_details.rs` Z. 200-218 (`on_toggle`-Wiring mit per-Klick API-Call + Error → on_error.call)

**Confirm-Modal-Skelett** (basics_tab.rs Z. 215-249):
```rust
if *show_paidout_confirm.read() {
    Modal {
        div { class: "flex flex-col gap-4",
            h2 { class: "text-xl font-semibold",
                "{i18n.t(Key::RepaymentEntryPaidOutConfirmTitle)}"
            }
            // D-16: Listentabelle + Gesamtsumme + 3-Punkt-Warnliste
            table { class: "w-full text-sm",
                thead { tr { th { "Mitgl.-Nr." } th { "Name" } th { "Anteile" } th { "Betrag" } } }
                tbody {
                    for entry in selected_entries.iter() {
                        tr { /* … */ }
                    }
                }
            }
            div { class: "text-right font-bold",
                "Summe: {format_payout_eur_total(selected_entries, share_value)}"
            }
            ul { class: "text-sm text-red-700 list-disc list-inside",
                li { "⚠ Diese Aktion ist final — kein Rückweg möglich." }
                li { "⚠ Erzeugt einen Verkauf-Audit-Eintrag pro Mitglied." }
                li { "⚠ Reduziert current_shares der betroffenen Mitglieder." }
            }
            div { class: "flex gap-2 justify-end mt-2",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                    onclick: move |_| show_paidout_confirm.set(false),
                    "{i18n.t(Key::Cancel)}"
                }
                button {
                    r#type: "button",
                    class: "bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded min-h-[44px]",
                    onclick: move |_| {
                        show_paidout_confirm.set(false);
                        spawn(async move {
                            // D-15: Sequential-Loop, Backend-Atomarität nur pro Entry
                            let config = CONFIG.read().clone();
                            let mut success_count = 0;
                            let mut failure_count = 0;
                            for entry_id in selected_ids.iter() {
                                match api::mark_repayment_entry_paid_out(&config, *entry_id).await {
                                    Ok(_) => success_count += 1,
                                    Err(e) => {
                                        failure_count += 1;
                                        on_error.call(format!("Eintrag {entry_id}: {}", e.message));
                                    }
                                }
                            }
                            // D-15: Summary-Toast
                            on_summary.call((success_count, failure_count));
                            // Specifics: MEMBERS-Refresh nach Cascade
                            crate::service::member::refresh_members().await;
                            on_changed.call(());
                        });
                    },
                    "Endgültig markieren"
                }
            }
        }
    }
}
```

**Pure-Helper für Tests:**
- `fn sum_payout_amounts(entries: &[RepaymentEntryTO], share_value_cents: i64) -> i64` (D-16 Gesamtsumme)

---

### 7. `genossi-frontend/src/component/editable_share_count_cell.rs` (NEU — siehe „No Analog Found")

**Kein direkter Analog im Code.** `member_details.rs` Z. 480-538 zeigt **Page-Level-Edit-Toggle**, nicht Inline-Cell-Edit. Vorgeschlagene API basiert auf Phase-12-Need:

```rust
#[component]
pub fn EditableShareCountCell(
    value: i32,
    disabled: bool,           // read-only wenn status == PaidOut
    on_save: EventHandler<i32>,
) -> Element {
    let mut editing = use_signal(|| false);
    let mut local_value = use_signal(|| value);

    if *editing.read() {
        rsx! {
            div { class: "flex items-center gap-1",
                input {
                    r#type: "number",
                    class: "w-16 px-2 py-1 border border-gray-300 rounded",
                    value: "{local_value.read()}",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<i32>() {
                            local_value.set(n);
                        }
                    },
                }
                button {
                    r#type: "button",
                    class: "text-green-600 hover:text-green-800",
                    onclick: move |_| {
                        on_save.call(*local_value.read());
                        editing.set(false);
                    },
                    "✓"
                }
                button {
                    r#type: "button",
                    class: "text-red-600 hover:text-red-800",
                    onclick: move |_| {
                        local_value.set(value);
                        editing.set(false);
                    },
                    "✗"
                }
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                disabled: disabled,
                class: if disabled { "text-gray-500" } else { "hover:bg-blue-50 cursor-pointer px-2 py-1 rounded" },
                onclick: move |_| if !disabled { editing.set(true); },
                "{value}"
            }
        }
    }
}
```

**Pure-Helper für Tests:**
- `fn is_share_count_valid(n: i32) -> bool { n > 0 }` (analog filter_members Z. 9-35: pure-func + unit-test)

---

### 8. `genossi-frontend/src/api.rs` (api-extension)

**Analog:** Bestehender Assembly-API-Block (api.rs Z. 1659-1722) als Standard-Pattern

**12 neue API-Funktionen — Pattern komplett 1:1 (api.rs Z. 1685-1722):**
```rust
// Repayment-Phase
pub async fn list_repayment_phases(config: &Config) -> Result<Vec<RepaymentPhaseTO>, AppError> {
    info!("Listing repayment phases");
    let url = format!("{}/api/repayment-phase", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

pub async fn create_repayment_phase(
    config: &Config,
    req: &CreateRepaymentPhaseRequest,
) -> Result<RepaymentPhaseTO, AppError> {
    info!("Creating repayment phase");
    let url = format!("{}/api/repayment-phase", config.backend);
    let response = reqwest::Client::new().post(url).json(req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn open_repayment_phase(config: &Config, id: Uuid) -> Result<RepaymentPhaseTO, AppError> {
    info!("Opening repayment phase {id}");
    let url = format!("{}/api/repayment-phase/{id}/open", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

pub async fn close_repayment_phase(config: &Config, id: Uuid) -> Result<RepaymentPhaseTO, AppError> {
    info!("Closing repayment phase {id}");
    let url = format!("{}/api/repayment-phase/{id}/close", config.backend);
    let response = reqwest::Client::new().post(url).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}
// … weitere: get_repayment_phase, update_repayment_phase,
//             list_repayment_entries(phase_id), create_repayment_entry,
//             get_repayment_entry, update_repayment_entry, delete_repayment_entry,
//             batch_toggle_repayment_status, mark_repayment_entry_paid_out
```

**Soft-Delete-Pattern** (api.rs Z. 220-226 — `delete_member`):
```rust
pub async fn delete_repayment_entry(config: &Config, id: Uuid) -> Result<(), AppError> {
    info!("Deleting repayment entry {id}");
    let url = format!("{}/api/repayment-entry/{id}", config.backend);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}
```

**Lokale 409-Response-Strukturen (RESEARCH Open-Question 1):**
```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CloseConflictResponse {
    pub error: String,
    pub pending_count: usize,
    pub pending_member_numbers: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchFailureResponse {
    pub failure_index: usize,
    pub failure_id: String,
    pub failure_reason: String,
}
```
Pattern: lokal in api.rs deklariert, analog `MailJobTO` Z. 819-829.

**`SendBulkMailRequest` Erweiterung (D-18 + Phase-10 D-03):**
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendBulkMailRequest {
    pub to_addresses: Vec<BulkRecipient>,
    pub subject: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub static_document_ids: Vec<String>,
    // NEU (D-18):
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repayment_phase_id: Option<Uuid>,
}
```
Backend ist seit Phase 10 ready (D-03/D-12). `send_bulk_mail`-Funktionssignatur ergänzen mit `template_id: Option<&str>, repayment_phase_id: Option<Uuid>`.

---

### 9. `genossi-frontend/src/router.rs` (router extension)

**Analog:** router.rs Z. 33-37 (Assembly-Routes)

**+2 Route-Varianten** (router.rs Z. 33-37):
```rust
// ─── Phase 12 ─── Vorstand repayment routes (admin-gated)
#[route("/repayment-phases")]
RepaymentPhases {},
#[route("/repayment-phases/:id")]
RepaymentPhaseDetails { id: String },
```

**+2 Page-Re-Exports** (router.rs Z. 6-7):
```rust
pub use crate::page::RepaymentPhases;
pub use crate::page::RepaymentPhaseDetails;
```

---

### 10. `genossi-frontend/src/component/top_bar.rs` (nav-extension)

**Analog:** top_bar.rs Z. 66-69 (`mitglieder_items.push(NavItem { label: Assemblies, route: Route::Assemblies {} })`)

**+1 NavItem (D-27)** (top_bar.rs Z. 66-69):
```rust
if show_admin {
    mitglieder_items.push(NavItem {
        label: i18n.t(Key::RepaymentPhases).to_string(),  // „Anteils-Rückzahlung"
        route: Route::RepaymentPhases {},
    });
}
```
Placement-Discretion: D-27 sagt „zwischen Anwesenheit und Mail (oder Plan-Discretion)". Pattern erlaubt auch `kommunikation_items`-Group oder `verwaltung_items`. Empfehlung: `mitglieder_items` direkt nach Assemblies (verwandt mit Mitglieder-Workflow).

---

### 11. `genossi-frontend/src/component/mail_compose/template_var_buttons.rs` (component-extension)

**Analog:** Eigene bestehende Struktur (template_var_buttons.rs Z. 5-90)

**Vorhandenes Pattern** (template_var_buttons.rs Z. 5-26):
```rust
const PRIMARY_VARS: &[(&str, &str)] = &[
    ("first_name", "Vorname"),
    ("last_name", "Nachname"),
    // …
];

const SECONDARY_VARS: &[(&str, &str)] = &[
    // …
];
```

**Erweiterung (D-19):** Neuer Optional-Prop UND neue Var-Konstante:
```rust
const REPAYMENT_VARS: &[(&str, &str)] = &[
    ("payout_amount", "Auszahlbetrag"),
    ("share_count", "Anteile"),
    ("fiscal_year", "Geschäftsjahr"),
];

#[component]
pub fn TemplateVarButtons(
    on_insert: EventHandler<String>,
    #[props(default)] show_repayment_vars: bool,   // NEU (D-19)
) -> Element {
    // … bestehender Render-Pfad
    // PLUS Bedingung:
    if show_repayment_vars {
        for (var_name, label) in REPAYMENT_VARS.iter() {
            // gleiche button-Render-Schleife wie PRIMARY_VARS Z. 39-55
        }
    }
}
```

**Aufrufer-Anpassung (mail_page.rs):** `TemplateVarButtons { on_insert: ..., show_repayment_vars: repayment_phase_id.read().is_some() }`.

---

### 12. `genossi-frontend/src/page/mail_page.rs` (page extension — Query-Param-Parsing)

**Analog:** mail_page.rs Z. 51-77 (`selected_member_ids`-Signal + `use_effect`-Init mit globalem Signal)

**Pattern (RESEARCH Pitfall 4 + Open-Question 4):**
```rust
let mut repayment_phase_id = use_signal(|| Option::<Uuid>::None);

use_effect(move || {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                if let Some(phase_id_str) = params.get("phase_id") {
                    if let Ok(uid) = Uuid::parse_str(&phase_id_str) {
                        repayment_phase_id.set(Some(uid));
                    }
                }
                if let Some(members_str) = params.get("members") {
                    let ids: Vec<Uuid> = members_str
                        .split(',')
                        .filter_map(|s| Uuid::parse_str(s.trim()).ok())
                        .collect();
                    if !ids.is_empty() {
                        selected_member_ids.set(ids);
                    }
                }
            }
        }
    }
});
```

**Send-Button-Body-Erweiterung (mail_page.rs Z. 402-540):** Beim Aufruf `api::send_bulk_mail(...)` zusätzlich `repayment_phase_id: repayment_phase_id.read().clone()` weitergeben.

**Pure-Helper für Tests:** `fn parse_mail_query(search: &str) -> ParsedMailContext` mit `ParsedMailContext { phase_id: Option<Uuid>, member_ids: Vec<Uuid> }` — testbar ohne web_sys-Kontext (Eingabe direkt als String).

---

### 13. `genossi-frontend/src/i18n/{mod,de,en}.rs` (i18n extension)

**Analog:** i18n/de.rs Z. 4-30 (Match-Arms-Pattern für jeden Key)

**Enum-Erweiterung (i18n/mod.rs):**
```rust
pub enum Key {
    // ... existing
    // ─── Phase 12 ─── RepaymentPhase ────────────────────────────
    RepaymentPhases,
    RepaymentPhaseCreate,
    RepaymentPhaseTabBasics,
    RepaymentPhaseTabEntries,
    RepaymentPhaseTabExport,
    RepaymentPhaseStatusPreparation,
    RepaymentPhaseStatusOpen,
    RepaymentPhaseStatusClosed,
    RepaymentEntryStatusOpen,
    RepaymentEntryStatusContacted,
    RepaymentEntryStatusPaidOut,
    RepaymentEntryMarkContacted,
    RepaymentEntryMarkPaidOut,
    RepaymentEntryPaidOutConfirmTitle,
    RepaymentEntryPaidOutConfirmWarn1,
    RepaymentEntryPaidOutConfirmWarn2,
    RepaymentEntryPaidOutConfirmWarn3,
    RepaymentEntryAdd,
    RepaymentEntryDelete,
    RepaymentEntriesNotOpenYet,
    RepaymentExportNotOpenYet,
    RepaymentPhaseOpen,
    RepaymentPhaseClose,
    RepaymentPhaseCloseConfirmTitle,
    RepaymentPhaseCloseConfirmText,
    RepaymentPhaseShareValueEditHint,
    RepaymentEntryEmptyAutoFill,
    RepaymentEntryEmptyFilter,
    BulkMailLink,
}
```

**Match-Arm-Pattern (de.rs Z. 6-13):**
```rust
pub fn translate(key: Key) -> Rc<str> {
    match key {
        // ... existing
        Key::RepaymentPhases => "Anteils-Rückzahlung".into(),
        Key::RepaymentPhaseCreate => "Neue Phase anlegen".into(),
        Key::RepaymentPhaseTabBasics => "Stammdaten".into(),
        Key::RepaymentPhaseTabEntries => "Einträge".into(),
        Key::RepaymentPhaseTabExport => "Export".into(),
        // …
    }
}
```

en.rs analog (zweite Locale verpflichtend — Phase 4 D-19).

---

## Shared Patterns

### Authentication / Auth-Gate (alle Vorstand-Pages)

**Source:** `genossi-frontend/src/auth.rs` Z. 34-48 (`RequirePrivilege`)
**Apply to:** `repayment_phases.rs`, `repayment_phase_details.rs`

```rust
use crate::auth::RequirePrivilege;
use crate::page::access_denied::AccessDeniedPage;

rsx! {
    RequirePrivilege {
        privilege: "admin",
        fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
        TopBar {}
        div { class: "container mx-auto px-4 py-6",
            // … page content
        }
    }
}
```

---

### Button-Pattern (D-01 / D-02 verpflichtend)

**Source:** `genossi-frontend/src/component/member_search.rs` Z. 76-86 + `genossi-frontend/src/component/assembly_status_badge.rs` (komplett) + Hotfix-Commits e245013, c6f41fd, bb1be0b
**Apply to:** ALLE neuen `button { ... }`-Tags in `genossi-frontend/src/{component,page}/repayment_*.rs`

```rust
button {
    r#type: "button",                       // ZWINGEND — D-01
    class: "...",
    onclick: move |_| { /* sync handler, spawn(async) drinnen */ },
    "Label"
}
```

**Form-Submit-Pattern (NUR bei echter Form-Semantik):**
```rust
form {
    onsubmit: move |e| {
        e.prevent_default();                 // SYNCHRON zuerst — D-01
        // dann optional spawn(async)
        spawn(async move { /* … */ });
    },
    button { r#type: "submit", "Speichern" }
}
```

**Grep-Gate (D-02):** Plan-Acceptance-Test:
```bash
rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' \
   genossi-frontend/src/component/repayment_*.rs \
   genossi-frontend/src/page/repayment_*.rs \
| grep -v 'r#type:' | grep 'button {'
# Erwartet: 0 Treffer
```

---

### Error-Handling / Toast-Pattern

**Source:** `genossi-frontend/src/component/toast.rs` Z. 14-47 (`show_toast` + `ToastContainer`)
**Apply to:** ALLE neuen Pages + Components mit asynchronen API-Calls

```rust
let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
let mut toast_counter = use_signal(|| 0u64);

// On error:
Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),

// At bottom of page:
ToastContainer { messages: toast_messages }
```

**Single-Banner-Variante (D-04 — CloseConflictResponse):** `ErrorAlert { error: app_error, on_dismiss: ... }` (error_alert.rs Z. 1-48) mit „Details anzeigen"-Expand, weil pending_member_numbers strukturiert dargestellt werden können.

---

### API-Call-Pattern

**Source:** `genossi-frontend/src/api.rs` Z. 199-226 (`create_member`/`update_member`/`delete_member`) + Z. 1685-1722 (Assembly-Group)
**Apply to:** Alle 12 neuen Repayment-API-Funktionen

```rust
pub async fn <action>_repayment_<entity>(
    config: &Config,
    [id_or_request]: ...,
) -> Result<<TO>, AppError> {
    info!("<action> repayment <entity>");
    let url = format!("{}/api/repayment-<path>", config.backend);
    let response = reqwest::Client::new().<method>(url).json(&req).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}
```

**Standard-Error-Mapping:** `AppError`-Struct (api.rs Z. 14-19) + `status_to_message` (Z. 73-87) liefert deutsche Meldung. Strukturierte 409-Bodies via Caller-Deserialisierung von `error.detail` als `CloseConflictResponse` / `BatchFailureResponse`.

---

### Modal-Pattern

**Source:** `genossi-frontend/src/component/modal.rs` Z. 8-16 (Wrapper)
**Apply to:** Add-Entry-Modal (UI-04), PaidOut-Confirm-Modal (UI-05), Close-Confirm-Modal (D-07)

```rust
if *show_modal.read() {
    Modal {
        // children Element (Form ODER Confirm-UI)
    }
}
```

---

### Tab-Strip-Pattern

**Source:** `genossi-frontend/src/component/tab_strip.rs` Z. 17-50 (Generic TabStrip)
**Apply to:** `repayment_phase_details.rs` (UI-02 Page-Level-Tabs); zusätzlich Status-Filter-Tab-Strip-IM-Tab in `repayment_entry_list.rs` (D-12)

```rust
let tab_defs = vec![
    TabDef { key: "basics", label: i18n.t(Key::RepaymentPhaseTabBasics).to_string() },
    // …
];
TabStrip {
    tabs: tab_defs,
    active_key: active_key.clone(),
    on_change: move |k: String| active_tab.set(k),
    match active_key.as_str() { /* … */ }
}
```

---

### Component-First / Pure-Helper-Test-Pattern

**Source:** `genossi-frontend/src/component/member_search.rs` Z. 9-35 (Pure `filter_members`) + Z. 135-247 (`#[cfg(test)] mod tests`)
**Apply to:** ALLE neuen Components mit Filter-/Sort-/Validation-/Formatierung-Logik

Pattern: Extrahiere reine Funktionen aus dem Component (kein RSX, kein Signal-Access), platziere sie auf Datei-Top, schreibe `#[cfg(test)] mod tests` mit Edge-Case-Coverage.

**Phase-12-Pflicht-Pure-Helpers** (RESEARCH Wave 0 Gaps):
- `repayment_entry_list.rs`: `filter_entries_by_status`, `sort_entries_default`, `format_payout_eur`
- `repayment_phases.rs`: `sort_phases_default`
- `repayment_phase_details.rs`: `is_share_value_editable`, `is_entry_editable`, `should_show_lifecycle_button`
- `repayment_entry_paidout_confirm.rs`: `sum_payout_amounts`
- `repayment_entry_add_modal.rs`: `validate_create_entry`
- `mail_page.rs`: `parse_mail_query`
- `repayment_phase_status_badge.rs` + `repayment_entry_status_badge.rs`: `status_label`, `status_badge_class` (analog assembly_status_badge.rs Z. 41-81)

---

### Optimistic-Locking-Reload-Pattern (RESEARCH Pitfall 2 + Pitfall 7)

**Source:** Phase 8 Plan 10 Regression-Test + assembly_details.rs Z. 56-57 (`api::get_assembly(&config, assembly_id)` nach jeder Mutation)
**Apply to:** Nach jedem PUT/POST der Phase oder eines Entries

```rust
// Statt Response-Body-Version direkt zu verwenden, re-fetch:
match api::update_repayment_phase(&config, &req).await {
    Ok(_) => {
        // Re-load mit frischer version
        load_phase();
    }
    Err(e) if e.status == Some(409) => {
        on_error.call("Konflikt — bitte erneut speichern".into());
        load_phase();  // Re-fetch
    }
    Err(e) => on_error.call(e.message),
}
```

---

### MEMBERS-Refresh nach mark-paid-out (RESEARCH Pitfall 3)

**Source:** `genossi-frontend/src/service/member.rs` Z. 11-26 (`refresh_members`)
**Apply to:** Ende der Bulk-PaidOut-Loop in `repayment_entry_paidout_confirm.rs`

```rust
// Nach erfolgreicher (auch teilweiser) Bulk-Loop:
crate::service::member::refresh_members().await;
```

---

## No Analog Found

Files ohne enge Entsprechung in der Codebase — Plan-Phase muss neuen Pattern designen:

| File | Role | Data Flow | Reason | Suggested Pattern Source |
|------|------|-----------|--------|--------------------------|
| `genossi-frontend/src/component/editable_share_count_cell.rs` | component (Inline-Cell-Edit) | local-signal + commit-callback | **Page-Level-Edit-Toggle in `member_details.rs` Z. 480-538 ist NICHT Inline-Cell-Edit** — D-13 verlangt explizit cell-level click→input→save/cancel | RESEARCH Pitfall #5 spezifiziert API; Test-Pattern aus `member_search.rs::filter_members` (Pure-Func + Unit-Test); Render-Skelett oben in §7 vorgeschlagen |

**Hinweis für Planner:** Dieser eine Component-Baustein hat KEIN 1:1-Vorbild und ist Phase-12-Eigen-Design. Plan-Phase muss in der Acceptance explizit dokumentieren, dass dies kein Klon-Job ist. Empfehlung (RESEARCH Open-Question 3): spezialisierte Variante `EditableShareCountCell` (i32, status-aware) statt generischer `EditableCell<T>` — weniger Generics, klarer Use-Case-Fit.

## Metadata

**Analog search scope:**
- `genossi-frontend/src/component/` (alle 41 Dateien gescannt)
- `genossi-frontend/src/page/` (alle 21 Dateien gescannt)
- `genossi-frontend/src/api.rs` (2014 Zeilen — Sections Assembly/Mail/Member als Pattern-Donor)
- `genossi-frontend/src/router.rs` (Route-Enum)
- `genossi-frontend/src/service/member.rs` (MEMBERS-Global-Signal)
- `genossi-frontend/src/auth.rs` (RequirePrivilege)
- `genossi-frontend/src/i18n/{mod,de,en}.rs` (Key-Enum + Match-Arms)

**Files scanned:** ~70
**Files read (full or targeted-range):**
- `component/assembly_status_badge.rs` (komplett 1-82)
- `component/tab_strip.rs` (komplett 1-79)
- `component/modal.rs` (komplett 1-33)
- `component/member_search.rs` (komplett 1-248)
- `component/error_alert.rs` (komplett 1-48)
- `component/toast.rs` (komplett 1-48)
- `component/basics_tab.rs` (komplett 1-265)
- `component/nav_group.rs` (komplett 1-47)
- `component/mail_compose/template_var_buttons.rs` (komplett 1-91)
- `component/attendance_list.rs` (Z. 1-80 — Toggle-Request-Pattern)
- `component/mod.rs` (komplett)
- `component/top_bar.rs` (Z. 40-180 — NavGroup-Befüllung)
- `page/assemblies.rs` (komplett 1-185)
- `page/assembly_details.rs` (komplett 1-563)
- `page/mail_page.rs` (Z. 30-130 + 230-370 — selected_member_ids-Pattern)
- `page/member_details.rs` (Z. 480-580 — Edit-Toggle-Pattern, als Negativ-Beispiel für editable_cell)
- `page/mod.rs` (komplett)
- `api.rs` (Z. 1-230 + 810-895 + 1659-1830 — Standard-Patterns)
- `router.rs` (komplett 1-70)
- `auth.rs` (komplett 1-49)
- `service/member.rs` (komplett 1-27)
- `i18n/mod.rs` (Z. 600-720 — format_price/format_datetime)
- `i18n/de.rs` (Z. 1-30 — Match-Arm-Pattern)

**Pattern extraction date:** 2026-06-01
