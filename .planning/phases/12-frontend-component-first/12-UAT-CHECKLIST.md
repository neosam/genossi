# Phase 12 — UAT-Checkliste (Frontend Component-First)

**Tester:** Vorstand
**Datum:** _________
**Staging-URL:** _________
**Backend-Version:** Phase 7-11 alle gruen
**Test-Account:** mit OIDC-admin-Privilege

> **Signoff-Regel (Plan 12-15):** "approved"-Signal erfordert 100% der Items PASS-markiert.
> Jeder FAIL muss in der Defects-Tabelle stehen MIT einem zugeordneten Gap-Closure-Plan
> (oder Inline-Fix VOR Resume). PENDING-Items beim Signoff sind nicht zulaessig — entweder
> durchklicken, oder als FAIL+Defekt dokumentieren.

## Vorbereitung
- [ ] dx serve laeuft; http://localhost:8080 erreichbar
- [ ] Tailwind-Watch aktiv (npx tailwindcss in watch mode)
- [ ] Backend laeuft auf http://localhost:3000 mit Phase-7-11-Migrations
- [ ] Test-Member-Daten vorhanden (>= 5 Mitglieder mit unterschiedlichem current_shares + IBAN)
- [ ] Optional: ein bestehender Test-Eintrag mit exit_date im Vorjahr (fuer Auto-Befuellung)

## A. Listen-Page /repayment-phases (UI-01)
- [ ] Page-Mount: Top-Bar zeigt "Anteils-Rueckzahlung"-Nav-Item zwischen Assemblies und Mail (D-27)
- [ ] Auth-Gate: Aufruf ohne admin-Privilege -> AccessDeniedPage
- [ ] Empty-State: leere Liste zeigt Hinweis + 'Neue Phase anlegen'-CTA-Button
- [ ] Create-Modal: Klick auf 'Neue Phase anlegen' -> Modal mit fiscal_year + share_value (EUR-Input)
- [ ] Create-Modal Validation: Submit mit fiscal_year=0 oder share_value=0 -> Toast/Fehler
- [ ] Create-Modal Submit: 'Speichern' bei "2026" + "60,00" -> Phase angelegt, Modal schliesst, Liste lasst
- [ ] Liste-Sort: Mehrere Phasen vorhanden -> sortiert nach fiscal_year DESC, created DESC
- [ ] Status-Badge sichtbar pro Row mit korrekter Farbe (Preparation=grau)
- [ ] **UI-01 SC#1**: Spalte "Anzahl Einträge" sichtbar; bei Mount '…' Loading-Placeholder, danach exakter Count pro Phase (z.B. "5"); bei API-Error '?' als Fallback
- [ ] Klick auf Phase-Row -> Navigation zu /repayment-phases/{id}

## B. Detail-Page Status Vorbereitung (UI-02)
- [ ] Page-Header: Titel + RepaymentPhaseStatusBadge (grau "Vorbereitung")
- [ ] D-06: 3 Tabs IMMER sichtbar (Stammdaten/Eintraege/Export)
- [ ] Tab Stammdaten: fiscal_year + share_value als read-only Anzeige (+ "Bearbeiten"-Button neben share_value)
- [ ] D-03 Lifecycle-Tile: "Phase oeffnen"-Action-Tile sichtbar (NICHT im Header)
- [ ] Tab Eintraege: Hinweis-Box "Phase noch nicht geoeffnet"
- [ ] Tab Export: Hinweis-Box "Phase noch nicht geoeffnet"
- [ ] D-05 share_value-Inline-Edit: Klick "Bearbeiten" -> Input erscheint -> aendern -> Speichern -> aktualisiert
- [ ] D-05 Audit-Hint: in Vorbereitung KEIN "Korrektur wird auditiert"-Hinweis sichtbar

## C. Phase oeffnen (Lifecycle)
- [ ] Klick "Phase oeffnen" -> KEIN Confirm (D-07) -> Backend-POST -> Liste/Status reload
- [ ] Nach Reload: Status-Badge zeigt "Offen" (blau)
- [ ] D-09: Tab bleibt auf "Stammdaten" (KEIN Auto-Switch zum Einträge-Tab)
- [ ] D-03 Lifecycle-Tile: jetzt "Phase abschliessen"-Action-Tile (rot)
- [ ] Tab Eintraege: jetzt RepaymentEntryList sichtbar mit Auto-Befuellten Eintraegen (oder Empty-State falls keine Vorjahres-Austritte)
- [ ] Tab Export: jetzt voll funktional (kein Hinweis mehr)

## D. RepaymentEntryList (UI-03)
- [ ] Tabelle hat 7 Spalten (Checkbox + Mitgl.-Nr. + Name + Anteile + Betrag + IBAN + Status + Aktionen)
- [ ] D-10 Betrag = share_count_to_pay_out * share_value, deutsch formatiert "X,XX €"
- [ ] D-10 IBAN fehlt -> Spalte zeigt "—"
- [ ] D-12 Status-Filter-Tab-Strip: 4 Tabs (Alle/Offen/Angeschrieben/Ausbezahlt) mit Counts
- [ ] D-12 Klick auf Filter -> nur passende Eintraege sichtbar
- [ ] D-11 Multi-Select: Header-Checkbox waehlt alle gefilterten aus; Per-Row-Checkbox toggle
- [ ] D-11 Bulk-Buttons: bei 0 Selection disabled; bei >= 1 Selection aktiv mit Count-Badge
- [ ] D-14 Default-Sort: Mitgliedsnummer ASC
- [ ] D-13 Inline-Cell-Edit: Klick auf Anteile-Zelle -> Input + Save/Cancel; Save -> PUT mit version
- [ ] D-13 Status=Ausbezahlt -> Cell ist read-only (kein Inline-Edit)
- [ ] D-14 Soft-Delete: Trash-Icon nur sichtbar wenn !=Ausbezahlt; Klick -> Confirm-Modal -> Eintrag aus Liste raus
- [ ] D-14 Empty-State: nach Filter ohne Treffer -> "Keine Eintraege mit diesem Status."
- [ ] D-14 Empty-State (initial): 0 Auto-Befuellt -> "Keine Eintraege — Vorjahres-Austritte fehlen. Eintrag manuell hinzufuegen."

## E. Add-Entry-Modal (UI-04)
- [ ] Klick "Eintrag manuell hinzufuegen" -> Modal mit MemberSearch + Anteile-Input
- [ ] D-21 MemberSearch: Substring-Suche (Vorname/Nachname/Mitgl.-Nr.) funktioniert
- [ ] D-22 Vorbefuellung: Member-Select -> share_count_to_pay_out wird mit current_shares befuellt
- [ ] D-23 Validation: ohne Member ODER share_count<=0 -> Speichern-Button disabled
- [ ] Submit -> Eintrag erscheint in Liste (Plan 12-09 entries_reload_trigger-Counter feuert)
- [ ] **Plan 12-09**: Liste reloaded innerhalb 1-2 Sekunden ohne Page-Refresh (Counter-Pattern verbatim)

## F. Status-Toggle 'Als angeschrieben markieren' (D-20)
- [ ] Multi-Select 2-3 Offene Eintraege
- [ ] Klick "Als angeschrieben markieren (3)" -> Backend POST batch-status
- [ ] Liste reloaded, Status-Badge jetzt blau "Angeschrieben"

## G. PaidOut-Confirm-Modal (UI-05)
- [ ] Multi-Select 2 Angeschriebene
- [ ] Klick "Als ausbezahlt markieren (2)" -> rotes Confirm-Modal
- [ ] D-16: Modal zeigt Listentabelle + Gesamtsumme + 3 rote Warnzeilen + roter "Endgueltig markieren"-Button
- [ ] D-15: Klick "Endgueltig markieren" -> Sequential-Loop (im Browser-DevTools sichtbar als 2 sequentielle POST mark-paid-out)
- [ ] D-15 Summary-Toast: "2 Eintraege als ausbezahlt markiert."
- [ ] D-17: bei PAYO-03-Validation-Fehler (Member.current_shares < share_count) -> per-Entry-Toast
- [ ] Eintraege jetzt Status=Ausbezahlt; Inline-Cell-Edit fuer Anteile ist disabled
- [ ] Member-Sidebar/andere Pages: current_shares ist aktualisiert (refresh_members nach Loop)
- [ ] PAYO-04: Erneuter PaidOut-Versuch auf bereits-ausbezahlten Eintrag -> Backend 409 -> Toast

## H. Massenmail-Flow (UI-06)
- [ ] Multi-Select 2-3 Eintraege
- [ ] Klick "Mail an 3 ausgewaehlte" -> Browser navigiert zu /mail?from=repayment&phase_id=...&members=...
- [ ] Mail-Page: Recipient-Picker zeigt die 3 Mitglieder bereits ausgewaehlt
- [ ] D-19 TemplateVarButtons: 3 orange Buttons sichtbar ("Auszahlbetrag", "Anteile", "Geschaeftsjahr")
- [ ] Click "Auszahlbetrag" -> `{{ payout_amount }}` ins Body eingefuegt
- [ ] **Issue #2 (Plan 12-12)**: Template-Auswahl im TemplateSelector → selected_template_id-Signal wird gesetzt; im Browser-DevTools Network-Tab ist der send-bulk-Body mit `"template_id": "<echte-id>"` zu sehen (NICHT null/None)
- [ ] Subject + Body schreiben mit `{{ payout_amount }}`-Substitution
- [ ] Senden -> Mail-Job angelegt (sichtbar in Mail-Job-Liste)
- [ ] Backend versendet personalisierte Mails (in Browser-DevTools: send-bulk Body enthaelt repayment_phase_id UND template_id)
- [ ] Browser-Zurueck -> Detail-Page wieder; selected_ids ist (erwartet) leer; Vorstand muss manuell "Als angeschrieben markieren" klicken (D-20)

## I. Phase abschliessen (Lifecycle)
- [ ] Klick "Phase abschliessen" -> Confirm-Modal (D-07) mit rotem Bestaetigungs-Button
- [ ] Bei nicht-allen-ausbezahlten Eintraegen -> 409 CloseConflictResponse -> ErrorAlert mit pending_count + Details-Expand zeigt Member-Liste (D-04)
- [ ] Bei allen ausbezahlt -> Confirm-Klick -> Backend POST close -> Status=Abgeschlossen (gruen)
- [ ] D-08: Detail-Page jetzt komplett read-only: keine Inline-Edit, keine Trash-Icons, kein Add-Modal
- [ ] Tab Export bleibt voll funktional

## J. PDF-Export (EXPO-01..03)
- [ ] Tab Export: 3 Radio-Buttons (Open default ausgewaehlt) + grosser blauer "PDF herunterladen"-Anker
- [ ] Klick "PDF herunterladen" mit include=open -> Browser oeffnet PDF im neuen Tab
- [ ] PDF-Inhalt korrekt: Mitgliedsnummer + Name + IBAN + share_count + Betrag + Verwendungszweck (Backend Phase 11 EXPO-02)
- [ ] Filter aendern auf "all"/"paid" -> Download liefert entsprechend gefilterte PDF
- [ ] In Abgeschlossen-Status weiter funktional

## K. Button-Reload-Bug-Check (D-01)
- [ ] In Detail-Page durch alle Tab-Klicks navigieren — keine URL-Query-Anhaengsel (kein Page-Reload)
- [ ] In RepaymentEntryList alle Buttons klicken — kein Reload
- [ ] In CreateRepaymentPhaseForm Submit klicken — Modal schliesst, kein Reload
- [ ] In Add-Entry-Modal Submit klicken — Modal schliesst, kein Reload
- [ ] In PaidOut-Confirm-Modal "Endgueltig markieren" — Modal schliesst, kein Reload
- [ ] In Close-Confirm-Modal "Abschliessen" — Modal schliesst, kein Reload

## L. Auth-Gate (D-25)
- [ ] Helper-Login (mock_auth oder echtes OIDC mit nicht-admin-Account) -> /repayment-phases zeigt AccessDeniedPage
- [ ] /repayment-phases/{id} ebenfalls AccessDeniedPage
- [ ] Top-Bar zeigt KEINEN "Anteils-Rueckzahlung"-NavItem (show_admin-Gate)

## Defects
| # | Beschreibung | Plan-Referenz | Schwere | Gap-Closure-Plan oder Inline-Fix | Status |
|---|---|---|---|---|---|
| 1 | A#5: Create-Modal Validation feuert nicht — Submit mit `0`/`0` löst trotz `e.prevent_default()` einen Full-Page-Reload aus (weißer Flash → Liste), Toast erscheint nicht. Bekannter Dioxus-Bug (Memory `feedback_dioxus_button_type`) — `form { onsubmit }` + `r#type:"submit"` reicht nicht. Auch proaktiv in `repayment_entry_add_modal.rs` (UI-04) gefixt. | 12-04 (Listen-Page Create-Form) + 12-09 (Add-Entry-Modal, proaktiv) | Medium | **Inline-Fix angewendet:** `form` → `div`, Submit-Button `r#type:"button"` + `onclick: submit`-Closure. Files: `genossi-frontend/src/page/repayment_phases.rs`, `genossi-frontend/src/component/repayment_entry_add_modal.rs`. | RESOLVED |
| 2 | A#5: ToastContainer `z-40` vs. TopBar `z-50` → Desktop-Toasts (`md:top-4 md:right-4`) werden komplett von der TopBar verdeckt. Auf Mobile sichtbar (dort positionierten sie sich am Bottom), aber Desktop-Nutzer (Vorstand) sahen Toasts gar nicht. | Component `toast.rs` (außerhalb Phase-12-Scope, aber durch UAT aufgedeckt) | Low | **Inline-Fix angewendet:** `z-40` → `z-[60]` (höher als TopBar). File: `genossi-frontend/src/component/toast.rs`. | RESOLVED |
| 3 | H#3: Nach Redirect auf `/mail?from=repayment&phase_id=...&members=...` ist kein Mitglied ausgewählt. URL kommt korrekt an, aber `use_effect`-basiertes URL-Parsing in `mail_page.rs` race-anfällig — der initiale Render mit leerem `selected_member_ids` läuft durch, bevor das Effect den Signal setzt. Chip-Block prüft zudem gegen `MEMBERS` (auch leer beim ersten Render), Re-Render reflektiert die Selection u.U. nicht zeitig. | 12-12 (mail_page Query-Params-Parse) | High | **Inline-Fix angewendet:** Query-Params SYNCHRON im `use_signal`-Initializer parsen statt im `use_effect`. `selected_member_ids` und `repayment_phase_id` werden direkt mit den URL-Werten initialisiert (Fallback auf global SELECTED_MEMBER_IDS bei leeren URL-Params). File: `genossi-frontend/src/page/mail_page.rs`. | RESOLVED |
| 4 | H#3 (root cause): RepaymentEntryList ruft `on_mail_request(selected_ids)` mit ENTRY-IDs auf, aber `build_mail_redirect_url` baut `?members=<uuid-csv>` und mail_page sucht in `MEMBERS` mit diesen UUIDs — Entry-ID ≠ Member-ID, also kein Match. Defekt #3-Fix (sync URL-Parse) wäre folgenlos gewesen, da der Inhalt schon falsch war. | 12-08 (RepaymentEntryList Bulk-Mail-Button) | **CRITICAL** | **Inline-Fix angewendet:** Im `onclick` der RepaymentEntryBulkMailButton-Schaltfläche selected entry-IDs → `entries.iter().filter(selected).map(e \| e.member_id)` → Vec<Uuid> von Member-IDs durchreichen. File: `genossi-frontend/src/component/repayment_entry_list.rs`. | RESOLVED |
| 5 | H#5-9 (Backend Phase-10): Body mit `{{ payout_amount }}` ohne `{% if payout_amount is defined %}`-Guard → REST send-bulk schlägt mit "Body: Template render error: undefined value" fehl, **obwohl** `repayment_phase_id` mitgeschickt wird. Ursache: `validate_template_with_repayment` rief ZUERST `validate_template` (pure-member-context ohne payout_amount) — die Probe schlug fehl, bevor die merged-context-Probe (die die Vars kennt) jemals lief. Phase-10's "fail-fast"-Design verlangt explizite Guards, was im Phase-12-UX (alle Empfänger gehören zur Phase) sinnlos ist. | Backend Phase-10 (`genossi_mail/src/template.rs`) | High | **Inline-Fix angewendet:** Pure-member-Probe in `validate_template_with_repayment` entfernt — merged-context-Probe deckt alle Render-Fehler ab, inklusive Member-Var-Typos. Test `test_validate_template_with_repayment_catches_missing_guard` → umgeschrieben zu `..._accepts_unguarded_payout_amount`, mit zusätzlichem Test für Member-Var-Typos. Files: `genossi_mail/src/template.rs`. | RESOLVED |
| 6 | H#5-9 (Live-Preview): TemplatePreview ruft `/api/mail/preview` mit pure-member-context auf — kein `repayment_phase_id`-Feld → Live-Preview von `{{ payout_amount }}` schlägt mit "Body: Template render error: undefined value (in <string>:6)" fehl, weil pure-member-render keine Repayment-Vars hat. Defekt #5-Fix nur für send-bulk wirksam; Preview-Pfad blieb defekt. | Backend Phase-10 + Frontend Plan 12-12 | High | **Inline-Fix angewendet:** Backend `PreviewRequest` um `repayment_phase_id: Option<String>` erweitert; `preview_mail`-Handler merget Dummy-Werte (`payout_amount="60,00"`, `share_count=1`, `fiscal_year=2026`) wenn gesetzt. Frontend `PreviewRequest`/`preview_mail`-Signatur + `TemplatePreview`-Component um `repayment_phase_id`-Prop erweitert; `mail_page.rs` reicht `*repayment_phase_id.read()` durch. Files: `genossi_mail/src/rest.rs`, `genossi-frontend/src/api.rs`, `genossi-frontend/src/component/mail_compose/template_preview.rs`, `genossi-frontend/src/page/mail_page.rs`. | RESOLVED |

## Zusammenfassung
- Total Items: __
- PASS: __
- FAIL: __  (jeder FAIL hat einen Defekt-Eintrag oben mit Plan-Referenz)

**Signoff-Regel:** Resume mit "approved" erfordert PASS == Total Items (oder FAIL+Defekt mit Plan-Closure-Pfad).
PENDING-Items beim Signoff nicht zulaessig.

**Tester-Signoff:** _________ Datum: _________
