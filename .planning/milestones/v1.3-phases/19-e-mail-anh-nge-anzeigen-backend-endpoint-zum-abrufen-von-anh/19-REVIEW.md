---
phase: 19-e-mail-anhaenge-anzeigen
reviewed: 2026-06-07T00:00:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - genossi_bin/src/lib.rs
  - genossi_bin/src/main.rs
  - genossi_bin/tests/e2e_tests.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/component/inbox/attachment_list_item.rs
  - genossi-frontend/src/component/inbox/attachment_list.rs
  - genossi-frontend/src/component/inbox/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/main.rs
  - genossi-frontend/src/page/inbox_page.rs
  - genossi-frontend/src/util/format.rs
  - genossi-frontend/src/util/mod.rs
  - genossi_mail/src/dao.rs
  - genossi_mail/src/dao_sqlite.rs
  - genossi_mail/src/inbox_imap.rs
  - genossi_mail/src/inbox_rest.rs
  - genossi_mail/src/inbox.rs
  - genossi_rest/src/http_util.rs
  - migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql
findings:
  critical: 1
  warning: 6
  info: 5
  total: 12
status: issues_found
---

# Phase 19: Code Review Report

**Reviewed:** 2026-06-07
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

Phase 19 fuegt ein Email-Attachment-Persistierungs- und Download-Feature hinzu (DAO + Service + REST + Backfill-Worker + Frontend-Komponenten). Die zentralen Sicherheitsziele werden weitgehend erfuellt: IDOR-Mitigation funktioniert ueber `find_by_id_and_mail`, das IMAP-Read-Only-Invariant wird in `fetch_since`/`fetch_one_by_uid` durch `EXAMINE` + `BODY.PEEK[]` korrekt umgesetzt, die 10-MB-Cap wird konsistent zwischen Poll und Backfill via `persist_attachment` durchgesetzt, und die Save-then-DB-Rollback-Sequenz ist korrekt implementiert.

Es gibt jedoch **eine BLOCKER-Schwachstelle**: der `oversized`-Check basiert ausschliesslich auf der Anzahl der heruntergeladenen Bytes (`bytes.len()`), nicht auf dem in den MIME-Headern angegebenen `Content-Length`. Das `mail_parser::contents()`-Lesen ist `unbounded`. Eine maliziose oder versehentlich grosse Mail (>10 GB Attachment) kann das ganze Worker-Heap-Memory belegen, bevor die Cap greift — ein klassischer Memory-Exhaustion-DoS. Ausserdem wird ein **WARNING** zu IDOR vergeben, weil die `inbound_mail_attachments`-Tabelle keinen UNIQUE-Constraint auf `id` jenseits des PRIMARY-KEY hat (PK ist UUID, also faktisch unique, aber: PK garantiert NICHT, dass die `id`-Spalte selbst global eindeutig ueber mehrere mails ist — hier zwar gegeben durch UUID-Generierung, aber kein DB-Constraint absichert das).

Weitere Befunde betreffen das Frontend (Component-First-Verstoss auf Inbox-Page mit duplizierten Buttons), Worker-Logging-Disziplin (PII in Logs), Frontend-Locale-Inkonsistenz (`"Bild"` hardcoded statt i18n) und Test-Abdeckungsluecken (Backfill-Happy-Path nicht abgedeckt, nur Skip-Pfade).

## Critical Issues

### CR-01: Memory-Exhaustion-DoS — `mail_parser::contents()` wird bedingungslos in `Vec<u8>` kopiert, bevor die 10-MB-Cap greift

**File:** `genossi_mail/src/inbox.rs:199-209, 226-230`
**Issue:**
`extract_attachments` ruft `part.contents().to_vec()` (Zeile 199) und `part.contents().to_vec()` (Zeile 229) auf, **ohne vorher die Groesse zu pruefen**. Diese Aufrufe materialisieren das **gesamte** Attachment-Payload im Heap. Erst danach laeuft in `persist_attachment` der Check `bytes.len() as u64 > ATTACHMENT_MAX_BYTES`.

Das bedeutet: eine eingehende Mail mit einem 5-GB-Attachment laesst sich vom Poll-Worker so verarbeiten, dass:
1. `mail_parser` parsed die gesamte Mail (Base64-Dekodierung in `String`).
2. `extract_attachments` allokiert nochmal denselben Speicher als `Vec<u8>`.
3. `persist_attachment` stellt fest "oversized" und droppt die Bytes.

Bevor Schritt 3 erreicht ist, hat der Worker bereits 10+ GB RAM allokiert. Ein einziges solches Mail-Item reicht aus, um den Server-Prozess via OOM zu killen — IMAP-Polling laeuft als spawned Tokio-Task ohne Memory-Limit.

Verstaerkend: `parse_raw_mail` ruft `extract_attachments` fuer **alle** Attachments einer Mail gleichzeitig auf (Z. 192-233 Loop), und `poll_once` verarbeitet den vollen Batch synchron (`for msg in messages { ... parse_raw_mail(&msg.raw) ... }`). Eine boese Mail mit z.B. 10 × 5-GB Attachments wird im RAM voll materialisiert.

**Fix:**
Die Groessen-Pruefung muss VOR dem `to_vec()` passieren. `mail_parser` exponiert `part.len()` oder das Decoded-Length-Limit kann via API-Konfiguration gesetzt werden. Minimaler Fix:

```rust
fn extract_attachments(msg: &mail_parser::Message) -> Vec<ParsedAttachment> {
    use mail_parser::MimeHeaders;
    let mut out = Vec::new();
    for (idx, part) in msg.attachments().enumerate() {
        // Probe-Read: contents() liefert &[u8] (Referenz), len() VOR to_vec().
        let raw_len = part.contents().len();
        let oversized = raw_len as u64 > ATTACHMENT_MAX_BYTES;
        if part.is_message() {
            let bytes: Vec<u8> = if oversized {
                // Markiere oversized OHNE die Bytes zu kopieren.
                Vec::new()
            } else {
                part.contents().to_vec()
            };
            let name = part.attachment_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("forwarded_{}.eml", idx));
            out.push(ParsedAttachment {
                file_name: name,
                mime_type: "message/rfc822".to_string(),
                bytes,
                // NEU: pass through real size so persist_attachment can decide.
                declared_size: raw_len as u64,
            });
            continue;
        }
        // ... gleiches Pattern fuer den Else-Zweig.
    }
    out
}
```

Plus eine Anpassung in `persist_attachment`, damit der Caller das richtige `size_bytes`-Feld setzt und `oversized=true` auch bei `bytes.is_empty()` korrekt gespeichert wird.

Alternativ kann man die `mail_parser`-API mit einem `decoded_max_size` konfigurieren (falls verfuegbar) oder den Raw-RFC822-Bytes-Check `if msg.raw_len() > N { skip mail entirely }` vorschalten — letzteres ist defensiver, weil `parse_raw_mail` die gesamte Base64-Dekodierung schon im Heap macht. Empfehlung: in `poll_once` bei `if msg.raw.len() > MAX_MAIL_SIZE { tracing::warn!(...); continue; }` mit einem Limit wie 50 MB (Mail-Standard) abbrechen, BEVOR `parse_raw_mail` aufgerufen wird.

## Warnings

### WR-01: i18n-Locale-Konflikt — `short_mime` rendert hardcoded deutsches `"Bild"` und `"Datei"`

**File:** `genossi-frontend/src/component/inbox/attachment_list_item.rs:147-163`
**Issue:**
`short_mime` gibt `&'static str` zurueck mit den hardcoded Werten `"PDF"`, `"Bild"`, `"Word"`, `"Excel"`, `"Datei"`. Das wird ueber `meta_line = format!("{} · {}", size_str, short_mime_label)` (Z. 66) in die UI gerendert — die UI ist sonst i18n-konsistent (alle anderen Strings via `i18n.t(Key::...)`). Im englischen Locale sieht der User dann `"12 KB · Bild"`, was unschoen und inkonsistent ist.

Die Datei hat auch die Phase-19-i18n-Keys (`InboxAttachmentsHeader` etc.) im i18n-Modul; `short_mime` haette ebenfalls ueber die Key-Enum laufen muessen.

**Fix:**
Ersetze `short_mime` durch eine i18n-aware Methode mit zusaetzlichen Key-Enum-Eintraegen:

```rust
// In i18n/mod.rs Key enum:
InboxAttachmentsMimePdf,
InboxAttachmentsMimeImage,
InboxAttachmentsMimeWord,
InboxAttachmentsMimeExcel,
InboxAttachmentsMimeOther,

// In attachment_list_item.rs:
fn short_mime_key(mime: &str) -> Key {
    if mime == "application/pdf" {
        Key::InboxAttachmentsMimePdf
    } else if mime.starts_with("image/") {
        Key::InboxAttachmentsMimeImage
    } else if /* word */ {
        Key::InboxAttachmentsMimeWord
    } /* etc. */
    else {
        Key::InboxAttachmentsMimeOther
    }
}
// caller: let short_mime_label = i18n.t(short_mime_key(&attachment.mime_type));
```

### WR-02: Component-First-Verstoss — `inbox_page.rs` enthaelt inline-RSX-Buttons statt wiederverwendbare Komponenten

**File:** `genossi-frontend/src/page/inbox_page.rs:230-252, 425-449`
**Issue:**
Das Projekt-CLAUDE.md macht **Component-First** zur Pflicht: "Pages should read like a high-level description of the UI, delegating rendering details to components." Der `InboxPageInner` enthaelt jedoch:

1. Drei Filter-Buttons (`Offen`/`Erledigt`/`Alle`, Z. 231-246) inline mit hardcoded Tailwind-Klassen — gleicher Pattern wie z.B. der Application-Status-Filter, koennte als `FilterButtonGroup` extrahiert werden.
2. Vier Aktion-Buttons (`Antworten`/`Zuordnung entfernen`/`Archivieren`/`Erledigt`, Z. 425-448) inline mit duplizierten Klassen-Strings.
3. Hardcoded deutsche Strings (`"Offen"`, `"Erledigt"`, `"Alle"`, `"Neu laden"`, `"Antworten"`, `"Zuordnung entfernen"`, `"Archivieren"`, `"Posteingang"`, `"nicht zugeordnet"`, etc.) — wieder i18n-Verstoss und Component-Duplikation.

Phase 19 erweitert die Inbox-Seite, ohne die bestehende Verletzung zu reduzieren. Die im Plan eingefuehrten Inbox-Components (`InboxMailListItem`, `InboxAttachmentList`, `InboxAttachmentListItem`, `InboxReplyForm`, `InboxStatusBadge`) sind ein guter Schritt, decken aber nicht die Action-Toolbar und Filter-Toolbar ab.

**Fix:**
Extrahiere `InboxFilterButtonGroup` und `InboxActionToolbar` als Komponenten unter `genossi-frontend/src/component/inbox/`. Pages sollten dann so aussehen:
```rust
InboxFilterButtonGroup { current: filter, on_change: move |f| filter.set(f) }
InboxActionToolbar {
    on_reply: ...,
    on_unassign: ...,
    on_archive: ...,
    on_done: ...,
}
```

### WR-03: Server-side IDOR-Test deckt nur DAO-Layer ab, nicht eine direkte URL-Manipulation mit valider mail_id

**File:** `genossi_bin/tests/e2e_tests.rs:4997-5039`
**Issue:**
Der `test_download_attachment_cross_mail_returns_404`-Test prueft, dass `(mail_B, attachment_A1)` → 404 liefert. Das ist der **erwartete** IDOR-Fall. Aber: der Test verwendet `seed_inbound_mail_attachment` aus dem Test-Helper, der direkt in die DB schreibt. Es wird **nicht** ueberprueft, dass die Lookup auch dann fehlschlaegt, wenn ein Angreifer eine `mail_id` von Mail B und eine `attachment_id` von Mail A's Attachment kombiniert, falls beide vom gleichen User-Schoepfer-Kontext stammen.

Wichtiger noch: der Test verifiziert nicht, dass das Service-Layer eine **andere** Logik haette nehmen koennen (z.B. `find_by_id` ohne Mail-Constraint, was die Code-Review-Implementierung NICHT macht — der DAO-`find_by_id_and_mail`-Query enthaelt explizit `WHERE id = ? AND inbound_mail_id = ?`). Defense-in-Depth: ein zweiter Test, der das **gleiche** Attachment-UUID gegen drei verschiedene `mail_id`s wirft, waere robust.

**Fix:**
Ergaenze einen Test, der die Service-Methode `find_attachment(WRONG_mail_id, valid_att_id)` direkt aufruft und sicherstellt, dass `Ok(None)` zurueckkommt:

```rust
#[tokio::test]
async fn test_download_attachment_with_random_mail_id_returns_404() {
    let (server, pool) = setup_with_pool().await;
    let mail_a = seed_inbound_mail(&pool, 100, "a@example.com", "A").await;
    let att = seed_inbound_mail_attachment(&pool, mail_a, "x.pdf", "application/pdf", b"x", false).await;
    let random_mail_id = uuid::Uuid::new_v4();
    let resp = reqwest::Client::new()
        .get(server.url(&format!("/api/inbox/{}/attachments/{}", random_mail_id, att)))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

### WR-04: Backfill-Happy-Path ist nicht getestet — Persist-Pfad bleibt ungetestet

**File:** `genossi_mail/src/inbox.rs:1399-1520`
**Issue:**
Die zwei Backfill-Tests (`test_run_attachment_backfill_silent_skips_imap_error` und `test_run_attachment_backfill_skips_already_backfilled`) verifizieren beide **NUR den Skip-Pfad** (`expect_create().times(0)`, `expect_save().times(0)`). Es gibt keinen Test, der den Happy-Path abdeckt:

1. `mail_dao.list_active()` liefert N Kandidaten.
2. `count_for_mail()` → `Ok(0)` fuer alle.
3. `fetch_one_by_uid()` → `Ok(Some(message))` mit echtem Multipart-Mail-Inhalt.
4. `persist_attachment` wird N-mal mit korrekten Args aufgerufen, und der Logging-Counter (`persisted`) zaehlt korrekt hoch.

Konsequenz: ein Refactoring, das `persist_attachment` versehentlich aus der Backfill-Loop entfernt oder die Variable `any_ok` auf `false` haengen laesst, wird vom Test-Set NICHT gefangen. Genau das ist die kritischere Klasse: ein "stiller" Backfill, der keine Errors logged aber auch nichts persistiert.

**Fix:**
Ergaenze `test_run_attachment_backfill_persists_when_refetch_succeeds`:

```rust
#[tokio::test]
async fn test_run_attachment_backfill_persists_when_refetch_succeeds() {
    let mut mail = sample_mail();
    mail.has_attachments = true;
    // ... config + dao stubs as above
    let raw_with_attachment = b"From: ...\r\n--BOUNDARY\r\n...test.png attachment..."; // same fixture as test A
    imap_client.expect_fetch_one_by_uid()
        .times(1)
        .returning(move |_, _, _| Ok(Some(FetchedMessage { uid: 10, raw: raw_with_attachment.to_vec() })));
    storage.expect_save().times(1).returning(|_, _| Ok(()));
    attachment_dao.expect_create().times(1).returning(|_| Ok(()));
    run_attachment_backfill(...).await;
    // mockall verifies on Drop
}
```

### WR-05: Worker-Loop loggt Datei-Namen + Mail-IDs auf `warn`-Level bei jedem Fehler → potenzieller Log-Spam + PII-Leak

**File:** `genossi_mail/src/inbox.rs:765-772, 918-924`
**Issue:**
In `poll_once` (Z. 766-771) und `run_attachment_backfill` (Z. 918-923) werden bei Attachment-Persist-Fehlern Mail-UUID und **`att.file_name`** auf Warn-Level geloggt. Beides ist nicht "PII" im engeren Sinne, aber:

- `att.file_name` kommt direkt vom Sender (Attacker-controlled). Ein Mail-Spammer kann mit gezielten Filename-Pattern Log-Flooding betreiben.
- Bei einer fehlerhaften Storage-Konfiguration (z.B. Disk full) wird **jede** Attachment-Persist-Operation fehlschlagen — fuer jede einzelne Attachment-Datei einer Mail wird ein Warn-Log erzeugt. Bei einem Polling-Batch von 100 Mails mit je 10 Anhaengen sind das 1000 Warn-Logs pro Polling-Zyklus.
- Geschaeftspartner-Mails enthalten haeufig Dateinamen mit Mitgliedsdaten ("Beitrittserklaerung_Mueller_Mitglied_12345.pdf"). Das landet in `tracing::warn!` -> stderr -> systemd-Journal — also faktisch ein **Datenschutz-Reach** auf Mitgliedsdaten in den Logs ausserhalb der Audit-Tabelle.

**Fix:**
Logge nur Mail-UUID + Anzahl der fehlgeschlagenen Attachments pro Mail, nicht den Filename:

```rust
let mut fail_count = 0;
for att in parsed.attachments.iter() {
    if let Err(e) = persist_attachment(/*...*/).await {
        fail_count += 1;
        // Nur einmal pro Mail, mit Mime-Type + Size statt Filename:
        tracing::warn!(
            "inbox_poll: persist_attachment failed for mail {} (mime={}, size={}): {:?}",
            mail.id, att.mime_type, att.bytes.len(), e
        );
    }
}
```

Alternativ: Filename auf einem niedrigeren Log-Level (`debug!`) belassen, sodass Produktiv-Konfigurationen ihn ausfiltern. Aktuell ist `warn!` per Default in `RUST_LOG=info` aktiv.

### WR-06: `mail_parser`'s `attachment_name()` wird ohne Sanitization weitergereicht, das Frontend rendert ihn via `title="{attachment.file_name}"`

**File:** `genossi_mail/src/inbox.rs:200-203, 222-225` (Persist) + `genossi-frontend/src/component/inbox/attachment_list_item.rs:46-47, 89-90` (Render)
**Issue:**
Der `file_name`, der vom Sender kommt, wird:
1. In `persist_attachment` direkt als `Arc::from(file_name)` ohne Length-Begrenzung oder Char-Stripping in die DB geschrieben (Z. 276).
2. Im Frontend ueber `title: "{attachment.file_name}"` und `"{attachment.file_name}"` direkt in den DOM gerendert.

Dioxus-RSX-String-Interpolation escaped HTML automatisch, also kein XSS-Risiko. Aber:
- Filenames koennen Kontrollzeichen (`\0`, `\r`, `\n`) oder Unicode-Bidi-Override-Chars (U+202E, "Right-to-Left Override") enthalten, was visuell-irrefuehrende Filenames erlaubt (z.B. `harmlos‮fdp.exe` zeigt sich als `harmlosexe.pdf`).
- Beim Download greift `http_util::content_disposition_attachment` (Z. 67-77) zwar mit `sanitize_ascii_filename`, das `\r\n\\"` durch `_` ersetzt — aber **nur fuer die ASCII-Fallback-Variante**. Das `filename*=UTF-8''` percent-encodiert alles ausser `[A-Za-z0-9._~-]`, was Kontrollzeichen sicher kodiert. **Das Header-Bauen ist also sicher.**
- Aber: das `download="{attachment.file_name}"`-Attribut im `<a>`-Element (Z. 101) gibt den unsanitizten Original-Filename an den Browser weiter, der ihn dann als Dateiname auf der Disk verwendet. Ein boeser Filename mit `../../etc/passwd` wuerde vom Browser zwar normalerweise ignoriert/sanitisiert, aber das ist Browser-spezifisch.

Praktisch sind die Risiken klein, aber Defense-in-Depth fehlt.

**Fix:**
Im Persistierungs-Pfad: max length cap + Kontrollzeichen filtern:

```rust
fn sanitize_attachment_filename(s: &str) -> String {
    let cleaned: String = s.chars()
        .filter(|c| !c.is_control() && *c != '\u{202E}' && *c != '\u{202D}')
        .take(255)
        .collect();
    if cleaned.is_empty() { "attachment.bin".to_string() } else { cleaned }
}
```

Vor `Arc::from(file_name)` in `persist_attachment` aufrufen.

## Info

### IN-01: `persist_attachment` ignoriert Storage-Delete-Failures stillschweigend (Best-Effort-Rollback) — sollte konfigurierbar sein

**File:** `genossi_mail/src/inbox.rs:283-296`
**Issue:**
Wenn `dao.create()` fehlschlaegt und `storage.delete()` _ebenfalls_ fehlschlaegt, bleibt ein verwaister File auf der Disk zurueck. Der Code loggt einen `warn!`, gibt aber den urspruenglichen DB-Fehler zurueck. Bei wiederholten Fehlversuchen sammeln sich Orphan-Files an. Die "10 MB-Cap als Schutz"-Annahme begrenzt zwar den maximalen Verlust, aber langfristig fuellt das die Disk.

**Fix:**
Periodischer Janitor-Worker, der `inbound_mail_attachments/*` durchsucht und Files ohne korrespondierenden DB-Eintrag loescht. Alternativ: Backfill-aehnliche Logik beim Server-Start.

### IN-02: Migrations-File hat keinen Index auf `(inbound_mail_id, id)` fuer `find_by_id_and_mail`

**File:** `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql:1-13`
**Issue:**
Die IDOR-Lookup `WHERE id = ? AND inbound_mail_id = ?` (DAO Z. 537) nutzt nur den PRIMARY KEY auf `id` — SQLite findet die Zeile via PK schnell, prueft dann die `inbound_mail_id`-Bedingung row-by-row. Das ist okay, weil PK Lookup O(log n) ist und nur eine Zeile zurueckkommt. Kein echtes Performance-Problem.

**Fix:**
Nur kosmetisch — optional ein UNIQUE-Constraint `UNIQUE(inbound_mail_id, id)` dokumentiert die Beziehung explizit.

### IN-03: `now_primitive` ist doppelt definiert — einmal als private fn in `inbox.rs:404`, einmal als private fn in anderen Modulen

**File:** `genossi_mail/src/inbox.rs:404-407`
**Issue:**
Die Funktion `now_primitive` ist fast identisch mit anderen `now_primitive`-Funktionen im Workspace (z.B. in `service.rs`, `worker.rs`). Code-Duplikation ohne funktionalen Unterschied.

**Fix:**
Extrahiere in ein gemeinsames `genossi_dao::time_util::now_primitive` oder aehnliches.

### IN-04: Pages haben mehrere `use_effect` Calls die `reload()` aufrufen — potenziell racy beim Mount

**File:** `genossi-frontend/src/page/inbox_page.rs:55-98`
**Issue:**
Zwei `use_effect`-Blocks: einer ruft `refresh_members()` auf, einer ruft `reload()` (initial mails fetch), und ein dritter setzt selected_id wenn `initial_id` gesetzt ist. Wenn der initiale Mount-Order der Effects nicht deterministisch ist, koennte `detail_loading` gesetzt werden bevor `loading` true ist — minor UX-Glitch, kein Bug.

**Fix:**
Konsolidiere zu einem einzigen `use_effect` mit klarer Sequenz.

### IN-05: `i18n::mod.rs` test `phase_18_keys_have_distinct_de_en_translations` deckt Phase 19-Keys nicht ab

**File:** `genossi-frontend/src/i18n/mod.rs:984-1086`
**Issue:**
Der bestehende Phase-18-Test verifiziert, dass alle Phase-18-Keys in DE und EN distinct sind. Phase 19 fuegt 7 neue Keys hinzu (`InboxAttachmentsHeader`, `..Download`, `..Preview`, `..EmptyLegacy`, `..Oversized`, `..DownloadError`, `..ImageAltPrefix`). Die DE/EN-Werte sind tatsaechlich distinct (manuelle Pruefung) — aber es gibt KEINEN Test, der das verifiziert. `..Download` ist in DE = `"Herunterladen"`, in EN = `"Download"` — derzeit korrekt, aber kein Schutz vor Copy-Paste-Fehlern bei spaeteren Aenderungen.

**Fix:**
Erweitere den Test um die Phase-19-Keys (oder besser: Mache den Test generisch, dass er **alle** Keys aller Phases abdeckt).

---

## Out-of-Scope Notes (Bestaetigungen, keine Findings)

- **IMAP-Read-Only-Invariant:** `fetch_since` und `fetch_one_by_uid` nutzen `EXAMINE` + `BODY.PEEK[]` (inbox_imap.rs:76, 126, 166) — server-side State unveraendert. `mark_seen` und `move_to_archive` benutzen `SELECT` und sind explizit User-getriggert. Phase 19's Backfill ruft `fetch_one_by_uid` auf — read-only confirmed.
- **Save-then-DB-Rollback:** `persist_attachment` (inbox.rs:247-299) schreibt erst Datei, dann DB-Row; bei DB-Fehler wird die Datei via `storage.delete` zurueckgerollt. Best-effort-Rollback ist explizit dokumentiert.
- **IDOR-Mitigation:** `find_by_id_and_mail` (dao_sqlite.rs:528-547) prueft `WHERE id = ? AND inbound_mail_id = ?` — REST-Handler ruft `state.inbox_service().find_attachment(mail_uuid, att_uuid)` (inbox_rest.rs:476-484) — defensive Layer korrekt.
- **Content-Disposition T-02/T-05:** `content_disposition_attachment` und `content_disposition_inline` (http_util.rs:43-94) schuetzen vor Header-Injection durch `sanitize_ascii_filename` + `percent_encode_utf8`. Tests `test_newline_in_filename` + `test_inline_newline_in_filename` verifizieren CR/LF-Stripping.
- **Dioxus-Button-Reload-Bug:** Die neuen Inbox-Komponenten (`InboxAttachmentList`, `InboxAttachmentListItem`) verwenden **ausschliesslich** Anchor-Elemente fuer Download/Preview-Aktionen — keine `<button onclick=...>`-Form-Reload-Falle. Korrekt umgesetzt.
- **i18n-Vollstaendigkeit:** Alle 7 Phase-19-Keys sind in `de.rs` (Z. 439-445) und `en.rs` (Z. 437-443) definiert. Kein fehlender Key.

---

_Reviewed: 2026-06-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
