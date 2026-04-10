## Why

Admins können eingehende Mails in der Inbox sehen, Mitgliedern zuordnen und archivieren — aber nicht darauf antworten. Um auf eine Anfrage zu reagieren, müssen sie manuell zur Mail-Seite wechseln, die Empfängeradresse abtippen und den Betreff kopieren. Dabei geht der E-Mail-Thread-Kontext (In-Reply-To/References-Header) verloren, sodass Antworten beim Empfänger nicht als Thread gruppiert werden.

Gleichzeitig sind die UI-Bausteine des Mail-Compose-Formulars (Subject-Input, Body-Editor, Template-Selector, Template-Variablen-Buttons, Template-Preview) direkt inline in `mail_page.rs` implementiert. Um sie für die Inbox-Antwort wiederzuverwenden, müssen sie zuerst als eigenständige Komponenten extrahiert werden — ein Schritt, der auch dem Component-First-Architekturprinzip des Projekts entspricht.

## What Changes

- **Extraktion von Mail-Compose-Komponenten**: Subject-Input, Body-Editor, Template-Variablen-Buttons, Template-Selector und Template-Preview werden aus `mail_page.rs` in wiederverwendbare Komponenten unter `component/mail_compose/` extrahiert
- **Refactoring der MailPage**: Die bestehende Mail-Seite wird auf die neuen Komponenten umgestellt (keine funktionale Änderung)
- **Neuer Status `replied`**: Inbound-Mails erhalten einen neuen Status `replied`, der in Liste und Detail angezeigt wird
- **Reply-Formular in der Inbox**: Die Detail-Ansicht einer Inbound-Mail erhält ein aufklappbares Antwort-Formular mit vorausgefülltem Empfänger (`from_address`) und Betreff (`Re: ...`)
- **Verknüpfung Inbound ↔ Outbound**: `mail_jobs` erhält ein optionales `reply_to_inbound_mail_id`-Feld, das den Job als Antwort auf eine Inbound-Mail kennzeichnet
- **In-Reply-To-Header auf ausgehenden Mails**: Der SMTP-Worker setzt `In-Reply-To` und `References` Header, wenn der Job ein `reply_to_inbound_mail_id` hat und die Original-Mail eine bekannte Message-ID besitzt
- **Backend-Endpoint**: Neuer `POST /api/inbox/{id}/reply` Endpoint, der einen Mail-Job erstellt, die Inbound-Mail auf `replied` setzt und die Verknüpfung herstellt

## Capabilities

### New Capabilities
- `inbox-reply`: Antwort auf eingehende Mails aus der Inbox heraus, inklusive Thread-Verknüpfung via E-Mail-Header
- `mail-compose-components`: Wiederverwendbare Frontend-Komponenten für das Mail-Compose-Formular

### Modified Capabilities
- `mail-sending`: Ausgehende Mails können optional `In-Reply-To` und `References` Header tragen, gesteuert über `reply_to_inbound_mail_id` auf dem Mail-Job

## Impact

- **Backend**: `genossi_mail` (DAO, Service, REST, Worker) — neues DB-Feld, neuer Endpoint, Worker-Erweiterung für Reply-Header
- **Frontend**: `genossi-frontend` — Komponenten-Extraktion in `component/mail_compose/`, Refactoring von `mail_page.rs`, Erweiterung von `inbox_page.rs`
- **DB-Migration**: Neue Spalte `reply_to_inbound_mail_id` auf `mail_jobs`
- **API**: Neuer Endpoint `POST /api/inbox/{id}/reply`, erweiterter `InboundMailTO` mit `replied`-Status
