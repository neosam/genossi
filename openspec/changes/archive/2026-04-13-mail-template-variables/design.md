## Context

E-Mails werden aktuell mit identischem Subject und Body an alle Empfänger gesendet. Der Worker in `genossi_mail/src/worker.rs` nimmt `job.subject` und `job.body` 1:1 und sendet sie. Jeder `MailRecipient` hat bereits eine `member_id`, die aber nur für Tracking genutzt wird, nicht für Personalisierung.

Member-Daten liegen in `MemberEntity` (genossi_dao) mit Feldern wie `first_name`, `last_name`, `salutation`, `email`, `company`, Adressfelder, Finanzdaten etc. Der `member-salutation-title` Change fügt `salutation` und `title` hinzu.

Das Frontend (`genossi-frontend/src/page/mail_page.rs`) hat ein einfaches Textarea für den Body und ein Input für Subject.

## Goals / Non-Goals

**Goals:**
- Jinja2-artige Template-Syntax in Subject und Body mit allen Member-Feldern
- Bedingte Logik (`{% if %}`, `{% elif %}`, `{% endif %}`)
- Template-Validierung vor Job-Erstellung (Syntax + Probe-Rendering)
- Preview-Endpoint zum Testen gegen ein konkretes Mitglied
- Variablen-Einfüge-Buttons im Frontend
- Template-Auflösung pro Empfänger im Worker zur Sendezeit

**Non-Goals:**
- Loops (`{% for %}`) — kein Anwendungsfall für Member-E-Mails
- Custom Filter/Functions in Templates
- HTML-E-Mails (bleibt Plain-Text)
- CodeMirror-Integration für Template-Editor (separater Change)
- Template-Speicherung/Wiederverwendung (kein Template-Bibliothek-Feature)

## Decisions

### 1. Template-Engine: MiniJinja

**Entscheidung:** MiniJinja (`minijinja` Crate) als Template-Engine.

**Alternativen:**
- **Handlebars**: Etabliert, aber `{{#if}}` / `{{/if}}` Syntax ist weniger intuitiv als Jinja2. Stärkere JS-Heritage.
- **Tera**: Voller Jinja2-Support, aber deutlich größere Dependency. Mehr Features als nötig.
- **Custom Regex-Replace**: Einfach für Variablen, aber keine bedingte Logik ohne eigenen Parser.

**Begründung:** MiniJinja bietet Jinja2-Syntax (bekannt, intuitiv), ist leichtgewichtig, hat gute Fehlermeldungen (wichtig für User-geschriebene Templates), und wird aktiv gepflegt (Armin Ronacher).

### 2. Template-Auflösung im Worker (nicht beim Job-Erstellen)

**Entscheidung:** Der Worker löst Templates pro Recipient auf, nicht der Service beim `create_job`.

**Alternativen:**
- **Auflösung beim Erstellen**: Jeder Recipient bekommt ein eigenes `resolved_body`-Feld. Kein Worker-Änderung nötig, aber DB-Schema-Änderung und viel Speicher bei großen Mailings.
- **Auflösung im Worker**: Job speichert Template, Worker rendert pro Empfänger.

**Begründung:**
- Kein DB-Schema-Change nötig (`job.body` = Template-String, `member_id` pro Recipient existiert bereits)
- Nachvollziehbarkeit: Man sieht im Job, welches Template verwendet wurde
- Weniger Speicher: Template einmal gespeichert statt N aufgelöste Bodies

**Trade-off:** Worker braucht Zugriff auf MemberDao. Wird als zusätzlicher Generic-Parameter hinzugefügt — konsistent mit bestehendem Pattern.

### 3. Validierung: Syntax-Check + Probe-Rendering vor Senden

**Entscheidung:** Beim `create_job` (send-bulk) wird das Template gegen alle Empfänger-Members probe-gerendert. Fehler werden sofort zurückgegeben, kein Job wird erstellt.

**Begründung:** Ein Tippfehler (`{{ frist_name }}`) soll nicht erst nach 50 gesendeten Mails auffallen. Die Validierung ist günstig (reine String-Operation in-memory), und die Member-Daten müssen ohnehin geladen werden um die member_ids zu prüfen.

### 4. Preview als eigener Endpoint

**Entscheidung:** `POST /api/mail/preview` nimmt subject, body (als Templates) und eine member_id, gibt den gerenderten Text zurück.

**Begründung:** Sauber getrennt von der Send-Logik. Frontend kann Preview on-demand aufrufen ohne Job-Erstellung.

### 5. Template-Kontext aus MemberEntity

**Entscheidung:** Alle Felder von `MemberEntity` werden als flache Key-Value-Paare in den Template-Kontext geschrieben. `Option<T>` wird zu `null` (Jinja2 truthiness: `{% if company %}` funktioniert).

Kontext-Mapping:
```
member_number: i64
first_name: String
last_name: String
email: String | null
company: String | null
comment: String | null
street: String | null
house_number: String | null
postal_code: String | null
city: String | null
join_date: String (ISO-Date)
shares_at_joining: i32
current_shares: i32
current_balance: i64
exit_date: String | null (ISO-Date)
bank_account: String | null
migrated: bool
salutation: String | null  (aus member-salutation-title)
title: String | null        (aus member-salutation-title)
```

Neue Funktion `member_to_template_context(entity: &MemberEntity) -> minijinja::Value` kapselt dieses Mapping zentral.

### 6. Frontend: Variablen-Buttons mit Einfügen am Ende

**Entscheidung:** Eine Button-Leiste über dem Body-Textarea mit den häufigsten Variablen. Klick fügt `{{ variable }}` am Ende des Textfeldes ein.

**Alternativen:**
- **Cursor-Position**: Erfordert JavaScript-Interop für `selectionStart` in WASM — fragil.
- **CodeMirror**: Syntax-Highlighting, Autocomplete — deutlich mehr Aufwand, separater Change.

**Begründung:** Pragmatischer Start. Variablen-Namen müssen nicht auswendig gelernt werden. Cursor-Position oder CodeMirror können später ergänzt werden.

## Risks / Trade-offs

- **Worker-Komplexität steigt** → MemberDao als zusätzliche Dependency ist etabliertes Pattern, überschaubar.
- **Template-Fehler zur Sendezeit** (Member-Daten ändern sich zwischen Validierung und Senden) → Unwahrscheinlich bei kurzen Zeitfenstern. Worker loggt Fehler und markiert Recipient als failed, Retry ist möglich.
- **MiniJinja-Dependency** → Leichtgewichtig (~minimal compile-time impact). Gut gepflegt.
- **Kein Cursor-Position bei Variablen-Einfügung** → Akzeptabler Kompromiss für v1. User können auch manuell tippen.
- **Performance bei Validierung großer Mailings** → Template-Rendering ist in-memory String-Operation, schnell. Bei 500 Members: ~ms.
