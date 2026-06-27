# Phase 21: Reply-Komfort — Antwort im vollflächigen Modal - Context

**Gathered:** 2026-06-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Das Antworten auf eine eingegangene Posteingangs-Mail öffnet künftig in einem
**vollflächigen Modal** (bestehende `Modal`-Component, `genossi-frontend/src/component/modal.rs`)
statt im schmalen Inline-Feld unterhalb der Mail-Detailansicht. Das bestehende
`InboxReplyForm` (Subject, Template-Selector, Var-Buttons, Attachment-Picker,
Preview, Body) wandert **unverändert** in das Modal. Abbrechen ohne Senden ist
möglich; das Absenden nutzt die **unveränderte** bestehende Sende-Logik
(`api::reply_inbox_mail`) und zeigt Erfolg-/Fehler-Feedback wie bisher.

**In scope:** Reply-UI vom Inline-Bereich ins Modal verlagern; Close-Affordances
(X-Icon + Abbrechen); Draft-Schutz-Confirm bei geändertem Text.

**Out of scope:** Änderungen an der Sende-Logik, am Backend, am Compose-Flow,
am Attachment-Picker oder am Quote-/Footer-Aufbau. Reine Frontend-Phase.
</domain>

<decisions>
## Implementation Decisions

### Close-/Abbrechen-Affordances
- **D-01:** Schließen erfolgt über ein **X-Icon im Modal-Header** (oben rechts)
  **plus** einen expliziten **«Abbrechen»-Button** — konsistent mit dem
  etablierten Projekt-Muster in `membership_adjust_modal.rs:144-151`
  (`flex items-center justify-between border-b pb-3`-Header mit X-Button, der
  `on_close.call(())` auslöst).
- **D-02:** **Kein Backdrop-Klick-Close und kein Escape-Close.** Die bestehende
  `Modal`-Component hat ohnehin keinen Backdrop-Handler; bestehende Modals im
  Projekt schließen nicht per Backdrop. Verhindert versehentlichen Entwurfs-Verlust.
- **D-03:** `InboxReplyForm` bekommt einen neuen `on_close: EventHandler<()>`-Prop
  und rendert seinen eigenen Header (Titel + X-Icon). Die Page
  (`inbox_page.rs`) umschließt es mit `Modal { InboxReplyForm { … on_close: … } }`
  — Child-rendert-Header-Muster wie bei den anderen Modals.

### Draft-Schutz beim Abbrechen
- **D-04:** Beim Schließen (X **oder** «Abbrechen») wird **nur dann** eine
  Bestätigung verlangt, wenn der Entwurf **gegenüber dem Initialzustand geändert**
  wurde. Unveränderter Entwurf → sofort schließen, keine Rückfrage.
- **D-05:** **Baseline-Falle (kritisch für Planner):** Der Reply-Body wird
  **asynchron** vorbefüllt — `reply_body` startet mit dem Quote, dann lädt der
  Footer on-mount (`use_effect` → `compose_initial_body`) und **überschreibt** den
  Body. Der Dirty-Vergleich darf NICHT gegen den Erst-Quote-Wert laufen, sonst
  meldet er fälschlich „geändert". Die Baseline (Subject + Body) muss **nach** dem
  Footer-Load als Snapshot festgehalten werden (z. B. eigenes
  `baseline_body`/`baseline_subject`-Signal, gesetzt im selben `use_effect`, der
  den initialen Body komponiert). Dirty = `reply_subject != baseline_subject ||
  reply_body != baseline_body`.
- **D-06:** Dirty-Check als **pure, unit-testbare Helper-Funktion** extrahieren
  (analog zu den bestehenden `build_original_quote` / `compose_initial_body`
  + Tests in `reply_form.rs`). CLAUDE.md verlangt Tests für Änderungen.
- **D-07:** Confirm-Mechanismus = **Claude's Discretion** (siehe unten). Native
  `web_sys::window().confirm_with_message(...)` ist pragmatisch akzeptabel, da das
  Reply-Modal bereits offen ist und ein verschachteltes In-App-Confirm-Modal
  umständlich wäre. Planner entscheidet final.

### Großes Textfeld
- **D-08:** Der `MailBodyEditor` (`mail_compose/body_editor.rs`, fest `h-40`)
  wird **NICHT** verändert. Er wird auch vom Compose-Flow genutzt; eine globale
  Vergrößerung würde Compose mitverändern. Der „große"-Textfeld-Effekt entsteht
  allein durch den **geräumigeren Modal-Kontext** (vollflächiges Layout,
  `max-h-[90vh]` scrollbar). Editor-Höhe bleibt faktisch `h-40`.

### Umfang im Modal
- **D-09:** Das **komplette** `InboxReplyForm` wandert unverändert ins Modal:
  Subject-Input, Template-Selector, Var-Buttons, Attachment-Picker (MemberDocuments
  + StaticDocuments), Template-Preview, Body-Editor, Senden-Button. **Kein**
  Verhaltensbruch, keine reduzierte Variante.

### Open-/Toggle-Verhalten
- **D-10:** Der bisherige «Antworten»/«Antwort abbrechen»-Toggle-Button in
  `inbox_page.rs:426-433` öffnet künftig nur noch das Modal (`show_reply.set(true)`).
  Das Schließen geschieht im Modal (X / Abbrechen / nach erfolgreichem Senden via
  bestehendem `on_sent` → `show_reply.set(false)`). Button-Label-Logik ggf.
  vereinfachen (nur noch „Antworten"). Planner prüft, ob das `show_reply`-Toggle
  beim erneuten Klick weiterhin sinnvoll ist.

### Claude's Discretion
- Exakter Confirm-Mechanismus (native `window.confirm` vs. In-App), Header-Titel-Text
  des Modals, genaue Tailwind-Klassen, ob `on_close` und Confirm in einem gemeinsamen
  `attempt_close`-Closure gebündelt werden. Alles innerhalb der gelockten Entscheidungen.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Reply-Feature (zu verändern)
- `genossi-frontend/src/component/inbox/reply_form.rs` — bestehendes `InboxReplyForm`;
  bekommt `on_close`-Prop + Header; enthält die Baseline-/Footer-Async-Logik (D-05);
  Vorbild für pure-fn + Tests (D-06).
- `genossi-frontend/src/page/inbox_page.rs` §415-493 — `show_reply`-Toggle (Z.52,
  119, 429-433) und aktuelle Inline-Einbindung des Forms (Z.451-487), die in
  `Modal { … }` umzubauen ist.

### Modal-Muster (Vorbild, NICHT ändern)
- `genossi-frontend/src/component/modal.rs` — `Modal`-Component (Children-Wrapper,
  kein eingebauter Close-Handler, `max-w-3/4 max-h-[90vh] overflow-y-auto`).
- `genossi-frontend/src/component/membership_adjust_modal.rs` §123, §142-151 —
  Referenz-Muster: `on_close: EventHandler<()>` + Header mit `justify-between
  border-b pb-3` + X-Button → `on_close.call(())`.

### Nicht anfassen
- `genossi-frontend/src/component/mail_compose/body_editor.rs` — `MailBodyEditor`,
  bleibt `h-40` (D-08), da auch von Compose genutzt.
- `genossi-frontend/src/api.rs` → `reply_inbox_mail` — Sende-Logik unverändert.

### i18n
- `genossi-frontend/src/i18n/mod.rs` (Key-Enum) + `de.rs` + `en.rs` — neue
  user-facing Strings (Header-Titel, Abbrechen, Confirm-Text) in **beiden** Locales
  ergänzen (nur En + De existieren; siehe `genossi-frontend/CLAUDE.md`).
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Modal` (`modal.rs`): vollflächiges Overlay, exakt wofür Phase 21 gedacht ist —
  einfach `InboxReplyForm` als Child reinhängen.
- `InboxReplyForm` (`reply_form.rs`): wird unverändert wiederverwendet, nur um
  `on_close` + Header + Dirty-Check erweitert.
- Header-Muster aus `membership_adjust_modal.rs`: direkt übernehmbar.

### Established Patterns
- Component-First (`genossi-frontend/CLAUDE.md`): keine Inline-RSX-Duplikate;
  Reply-Logik bleibt in der Komponente, Page komponiert nur.
- Modal-Child-rendert-Header-mit-X + `on_close`-EventHandler ist das etablierte
  Projekt-Muster.
- Pure-Helper + `#[cfg(test)] mod tests` direkt in der Komponentendatei
  (`reply_form.rs` hat bereits 8 Tests für quote/compose) — Dirty-Check analog.

### Integration Points
- `inbox_page.rs`: Inline-Block `if *show_reply.read() { … InboxReplyForm … }`
  (Z.451-487) → `Modal { InboxReplyForm { …, on_close: move |_| show_reply.set(false) } }`.
- `on_sent`-Callback bleibt wie heute (info-Toast „Antwort gesendet", reload,
  load_detail, show_reply=false).
</code_context>

<specifics>
## Specific Ideas

- „Vollflächiges Modal" = die bestehende `Modal`-Component (vom Phasen-Ziel
  explizit vorgegeben), kein neues Overlay.
- Close-Verhalten soll sich **anfühlen wie die anderen Modals** des Projekts
  (X oben rechts), nicht wie ein neues Pattern.
</specifics>

<deferred>
## Deferred Ideas

- Body-Editor mit konfigurierbarer Höhe / echtem „großen" Textfeld per Prop wurde
  bewusst **nicht** gewählt (D-08) — falls später doch ein größeres Eingabefeld
  gewünscht ist, wäre ein optionaler Höhen-Prop am `MailBodyEditor` der saubere,
  Component-First-konforme Weg (Default `h-40` → Compose unverändert).
- Backdrop-/Escape-Close (D-02 bewusst abgelehnt) könnte später projektweit als
  einheitliches Modal-Verhalten eingeführt werden — dann aber für alle Modals,
  nicht nur Reply.

None weiter — Diskussion blieb im Phasen-Scope.
</deferred>

---

*Phase: 21-reply-komfort-antwort-im-vollfl-chigen-modal*
*Context gathered: 2026-06-27*
