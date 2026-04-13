## Context

Genossi hat ein zweiseitiges Mail-System: Outbound (Bulk-Mail-Queue mit `mail_jobs`/`mail_recipients`) und Inbound (IMAP-Polling in `inbound_mails`). Outbound-Mails speichern bereits die `message_id` pro Recipient. Inbound-Mails parsen `in_reply_to` aus dem RFC 5322-Header. Allerdings gibt es keine Möglichkeit, direkt aus der Inbox auf eine eingehende Mail zu antworten.

Das Mail-Compose-UI (Subject, Body, Template-Variablen, Template-Selector, Preview) ist aktuell inline in `mail_page.rs` (~900 Zeilen). Für die Inbox-Antwort werden dieselben UI-Elemente benötigt, daher werden sie zuerst als wiederverwendbare Komponenten extrahiert.

## Goals / Non-Goals

**Goals:**
- Admin kann direkt aus der Inbox-Detail-Ansicht auf eine eingehende Mail antworten
- Antwort erscheint beim Empfänger im selben E-Mail-Thread (korrekte `In-Reply-To`/`References` Header)
- Inbound-Mail zeigt Status `replied` nach Beantwortung
- Mail-Compose-UI als wiederverwendbare Komponenten, genutzt von MailPage und InboxPage
- Bestehende MailPage-Funktionalität bleibt unverändert

**Non-Goals:**
- CC/BCC-Unterstützung bei Replies
- Zitieren des Original-Mailtexts in der Antwort (kann später ergänzt werden)
- Anhänge bei Replies (kann über Static Documents ergänzt werden, aber kein MVP-Ziel)
- Thread-Ansicht (gruppierte Darstellung aller Nachrichten einer Konversation)
- Inbox-Mails von der Mail-Seite aus beantworten (Reply nur über Inbox-Seite)

## Decisions

### 1. Komponenten-Extraktion vor Feature-Bau

Die Mail-Compose-Bausteine werden als erstes extrahiert, bevor das Reply-Feature gebaut wird. So kann die MailPage sofort refactored werden und die InboxPage dieselben Bausteine nutzen.

**Komponenten-Struktur:**
```
component/mail_compose/
├── mod.rs
├── subject_input.rs        — Label + Input, Props: value, on_change
├── body_editor.rs          — Label + Textarea, Props: value, on_change
├── template_var_buttons.rs — Primary/secondary vars mit "Mehr"-Toggle, Props: on_insert
├── template_selector.rs    — Formal/Informal Dropdown, Props: on_select
└── template_preview.rs     — Member-Auswahl + API-Preview, Props: subject, body, member_ids
```

**Alternative erwogen:** Eine einzelne `MailComposeForm`-Komponente mit Feature-Flags. Verworfen, weil die InboxPage die Bausteine anders anordnet (kein Empfänger-Widget, Reply-Kontext oben) und nicht alle Bausteine braucht.

### 2. Reply über dedizierten Backend-Endpoint

`POST /api/inbox/{id}/reply` mit Body `{ subject, body }`. Der Endpoint:
1. Lädt die Inbound-Mail
2. Erstellt einen `MailJob` mit `reply_to_inbound_mail_id` gesetzt
3. Erstellt einen `MailRecipient` mit `to_address = inbound_mail.from_address`
4. Setzt `inbound_mail.status = "replied"`
5. Gibt den erstellten Job zurück

**Alternative erwogen:** Bestehenden `create_job`-Endpoint um `reply_to_inbound_mail_id` erweitern. Verworfen, weil der Inbox-Reply eine atomare Operation ist (Job erstellen + Status setzen), die nicht in zwei API-Calls aufgeteilt werden sollte.

### 3. reply_to_inbound_mail_id auf mail_jobs (nicht mail_recipients)

Ein Reply-Job ist konzeptionell eine Antwort auf eine bestimmte Inbound-Mail. Da Replies immer genau einen Empfänger haben, ist die Verknüpfung auf Job-Ebene sauberer — der gesamte Job IST die Antwort.

**Migration:** `ALTER TABLE mail_jobs ADD COLUMN reply_to_inbound_mail_id BLOB` (nullable, kein FK-Constraint nötig da SQLite).

### 4. In-Reply-To und References Header im Worker

`send_mail_for_recipient` prüft, ob der zugehörige Job ein `reply_to_inbound_mail_id` hat. Wenn ja:
- Liest die `inbound_mails`-Zeile, um die `message_id` zu ermitteln (aus dem `in_reply_to`-Feld oder aus der IMAP-UID-basierten Message-ID)
- Setzt `In-Reply-To: <message_id>` und `References: <message_id>` auf der ausgehenden Mail

**Problem:** `inbound_mails` speichert aktuell nicht die eigene `message_id` der eingehenden Mail — nur `in_reply_to` (die Message-ID auf die SIE antwortet). Für korrekte Threading brauchen wir die Message-ID der Inbound-Mail selbst.

**Lösung:** Neues Feld `message_id` auf `inbound_mails`. Der IMAP-Poller parst es aus dem `Message-ID`-Header der eingehenden Mail (bereits im raw RFC 5322 enthalten, `parse_raw_mail` muss erweitert werden).

### 5. Status replied als Nicht-Endzustand

`replied` reiht sich in das Status-Modell ein:
```
new → assigned → replied
 ↓       ↓         ↓
 └───────┴────→ archived
 └───────┴────→ ignored
```

`replied` kann von `new` oder `assigned` aus erreicht werden. Nach `replied` kann noch archiviert oder ignoriert werden, falls man die Inbox aufräumen will.

### 6. Reply-UI in der Inbox-Detail-Ansicht

Das Reply-Formular wird direkt in der Detail-Spalte der Inbox-Seite eingebettet, unterhalb der bestehenden Aktions-Buttons. Es wird über einen "Antworten"-Button aufgeklappt.

Vorausgefüllt:
- **An:** `from_address` der Inbound-Mail (read-only angezeigt, nicht editierbar)
- **Betreff:** `Re: {original_subject}` (editierbar)
- **Body:** leer
- **Template-Variablen:** nur sichtbar wenn ein Mitglied zugeordnet ist

## Risks / Trade-offs

- **[Message-ID-Feld nachrüsten]** → Bestehende `inbound_mails`-Einträge haben keine `message_id`. Migration setzt NULL, neue Mails bekommen sie automatisch. Replies auf alte Mails ohne Message-ID funktionieren trotzdem, nur ohne Thread-Verknüpfung. Akzeptabler Trade-off.
- **[Kein Quoting]** → Antworten enthalten nicht den Original-Text. Für den Vereins-Kontext (kurze Anfragen, kurze Antworten) ist das akzeptabel. Kann später ergänzt werden.
- **[Single-Recipient Reply]** → Es wird immer nur an den `from_address` geantwortet. Wenn die Original-Mail mehrere Absender hätte (selten), gehen die verloren. Akzeptabel für MVP.
