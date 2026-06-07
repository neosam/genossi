---
phase: 19-e-mail-anhaenge-anzeigen
plan: 07
type: execute
wave: 1
depends_on:
  - 19-01-dao-and-migration
  - 19-02-service-and-imap
  - 19-03-rest-endpoints
  - 19-04-backfill-worker
files_modified:
  - genossi_mail/src/inbox.rs
autonomous: true
gap_closure: true
requirements:
  - D-02
tags:
  - security
  - dos-protection
  - rust
  - mail-parser

must_haves:
  truths:
    - "Worker materialisiert Attachment-Bytes erst dann, wenn die 10-MB-Cap geprüft ist (D-02 als Schutz vor Memory-DoS)"
    - "Bei oversized-Attachment wird KEINE Heap-Allokation der gesamten Bytes durchgeführt (Vec::new() statt to_vec())"
    - "persist_attachment kennt die deklarierte Größe auch bei leerem bytes-Slice (über declared_size-Pfad)"
    - "Der bestehende Test test_persist_attachment_oversized_skips_storage bleibt grün (10-MB-Cap als Persistenz-Marker)"
  artifacts:
    - path: "genossi_mail/src/inbox.rs"
      provides: "Probe-Read in extract_attachments + declared_size-Feld auf ParsedAttachment + erweiterte persist_attachment-Signatur"
      contains: "let raw_len = part.contents().len()"
    - path: "genossi_mail/src/inbox.rs"
      provides: "Neuer Test test_extract_attachments_oversized_skips_materialization"
      contains: "test_extract_attachments_oversized_skips_materialization"
  key_links:
    - from: "extract_attachments (probe-read)"
      to: "ParsedAttachment.declared_size"
      via: "Vec::new() bei oversized, raw_len als declared_size durchreichen"
      pattern: "declared_size: raw_len as u64"
    - from: "ParsedAttachment.declared_size"
      to: "persist_attachment(oversized=true, size_bytes=declared_size)"
      via: "Aufruf in poll_once + run_attachment_backfill"
      pattern: "persist_attachment\\(.*declared_size"
    - from: "persist_attachment"
      to: "InboundMailAttachment-Row mit korrektem size_bytes"
      via: "size_bytes := declared_size (NICHT bytes.len()), oversized := declared_size > CAP"
      pattern: "oversized = declared_size > ATTACHMENT_MAX_BYTES"
---

<objective>
Schließt die einzige BLOCKER-Lücke aus dem Code-Review (CR-01 / VERIFICATION Truth #3): `extract_attachments` materialisiert Attachment-Bytes via `part.contents().to_vec()` unbedingt vor der 10-MB-Cap-Prüfung. Eine bösartige Mail mit Multi-GB-Attachment kann den Worker-Prozess OOM-killen, bevor D-02 greift.

**Purpose:** D-02 (Memory-DoS-Schutz) muss VOR der Heap-Allokation greifen, nicht erst beim Persist-Schritt. Phase 19 darf erst nach diesem Fix als komplett deklariert werden.

**Output:** Refactor in `genossi_mail/src/inbox.rs`:
1. `ParsedAttachment` um `declared_size: u64` erweitert (real-size aus `part.contents().len()`)
2. `extract_attachments` führt Probe-Read durch: `raw_len > ATTACHMENT_MAX_BYTES` → `bytes = Vec::new()` (keine Heap-Allokation der vollen Payload)
3. `persist_attachment`-Signatur erweitert um `declared_size: u64` — `oversized` + `size_bytes` werden anhand `declared_size` berechnet (nicht `bytes.len()`), sodass oversized-Marker auch bei leerem `bytes`-Slice korrekt persistiert wird
4. Beide Caller (`poll_once`, `run_attachment_backfill`) reichen `att.declared_size` durch
5. Neuer Unit-Test `test_extract_attachments_oversized_skips_materialization` beweist: bytes-Vec bleibt bei oversized leer (`is_empty() == true`)
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-REVIEW.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-VERIFICATION.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-02-SUMMARY.md

<interfaces>
<!-- Aktuelle Vertragsdefinitionen aus inbox.rs — der Executor soll diese direkt verwenden statt sie aus dem Codebase erneut zu extrahieren. -->

From genossi_mail/src/inbox.rs (current state — to be modified):

```rust
// Zeile 158-163 — wird erweitert um declared_size: u64
pub struct ParsedAttachment {
    pub file_name: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
    // NEU (dieser Plan): tatsaechliche Groesse aus part.contents().len(),
    // unabhaengig davon ob bytes materialisiert wurden oder leer sind.
    // pub declared_size: u64,
}

// Zeile 186 — bleibt unveraendert
const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024;

// Zeile 192-233 — extract_attachments wird auf Probe-Read umgebaut
fn extract_attachments(msg: &mail_parser::Message) -> Vec<ParsedAttachment> { ... }

// Zeile 247-299 — persist_attachment Signatur wird erweitert um declared_size
async fn persist_attachment(
    storage: &dyn DocumentStorage,
    dao: &dyn InboundMailAttachmentDao,
    inbound_mail_id: Uuid,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
    // NEU (dieser Plan): declared_size: u64,
) -> Result<InboundMailAttachment, MailServiceError>
```

From genossi_mail/src/dao.rs (read-only reference):

```rust
// Zeile 111-122 — InboundMailAttachment Entity (unchanged)
pub struct InboundMailAttachment {
    pub id: Uuid,
    pub inbound_mail_id: Uuid,
    pub created: PrimitiveDateTime,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,        // <-- bei oversized muss declared_size hier landen, nicht 0
    pub relative_path: Option<Arc<str>>,
    pub oversized: bool,
}
```
</interfaces>

<gap_evidence>
**Failing Truth (VERIFICATION.md gaps[0]):**
"Worker materialisiert Attachment-Bytes erst dann, wenn die 10-MB-Cap geprüft ist (D-02 als Schutz vor Memory-DoS)"

**Reason for fail (VERIFICATION.md):**
`extract_attachments` ruft `part.contents().to_vec()` unbedingt VOR jeder Größenprüfung auf — die Cap greift erst in `persist_attachment`, nachdem die kompletten Bytes im Heap allokiert sind. Eine bösartige Mail mit Multi-GB-Attachment kann den Worker-Prozess OOM-killen, bevor D-02 greifen kann. Vom Review-Report (CR-01) als BLOCKER markiert.

**Affected lines:**
- `genossi_mail/src/inbox.rs:199` — `let bytes = part.contents().to_vec();` (is_message-Branch)
- `genossi_mail/src/inbox.rs:229` — `bytes: part.contents().to_vec(),` (else-Branch)
- `genossi_mail/src/inbox.rs:257` — bisheriger Cap-Check (zu spät: `bytes.len() as u64 > ATTACHMENT_MAX_BYTES`)

**Out of scope (explicitly per VERIFICATION recommendation):**
- WR-01 (`short_mime` hardcoded i18n), WR-02 (inline Toolbar), WR-04 (Backfill-Happy-Path-Test), WR-05 (PII-Logging), WR-06 (Filename-Sanitization), IN-01..IN-05 — alle Follow-up.
- Optional `MAX_MAIL_SIZE`-pre-parse-Guard in `poll_once` (Defense-in-Depth-Empfehlung aus VERIFICATION) — NICHT Teil dieses Fix-Plans. Begründung: Reduziert Komplexität; der Probe-Read schließt CR-01 vollständig. Pre-parse-Guard kann als separate Follow-up gegen `mail_parser`-Heap-Verbrauch adressiert werden, wenn ein konkreter Bedarf entsteht.
</gap_evidence>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| External IMAP-Server → genossi-Worker | Eingehende Mails aus dem IMAP-Server (Attacker-controlled via Mail-Sender) durchlaufen `parse_raw_mail` → `extract_attachments` → `persist_attachment`. Mail-Sender kann Multi-GB-Attachments einschmuggeln. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-19-07-01 | D (Denial of Service) | `extract_attachments` in `genossi_mail/src/inbox.rs` | mitigate | Probe-Read-Pattern: `let raw_len = part.contents().len()` VOR jeder `to_vec()`-Allokation. Bei `raw_len > ATTACHMENT_MAX_BYTES` wird `Vec::new()` allokiert; der originale Slice wird verworfen. ASVS L1 12.1.1 (Anti-DoS Resource Limits). Severity: HIGH. |
| T-19-07-02 | I (Information Disclosure / Integritaet) | `persist_attachment` Row mit `size_bytes` | mitigate | Bei oversized wird `size_bytes = declared_size` (echte Senderangabe) in die DB geschrieben, nicht `bytes.len() = 0`. Verhindert Inkonsistenz zwischen `oversized=true` und `size_bytes=0` — Frontend kann die echte Mail-Attachment-Groesse anzeigen. |

**Severity:** HIGH (Blocker per Code-Review CR-01). Eine einzelne maliziose Mail kann den Worker-Prozess OOM-killen.

**Verification:** Neuer Test `test_extract_attachments_oversized_skips_materialization` mit synthetischem multipart/mixed-Body, dessen Attachment-Decoded-Size > `ATTACHMENT_MAX_BYTES`. Test prueft, dass `attachments[0].bytes.is_empty()` UND `attachments[0].declared_size > ATTACHMENT_MAX_BYTES`. Damit ist bewiesen, dass `to_vec()` NICHT auf die volle Payload aufgerufen wurde.
</threat_model>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: ParsedAttachment.declared_size + Probe-Read in extract_attachments + persist_attachment-Signatur erweitert</name>
  <files>genossi_mail/src/inbox.rs</files>

  <read_first>
    - genossi_mail/src/inbox.rs (KOMPLETT) — Ist-Zustand der `ParsedAttachment`-Struct (Z. 158-163), `extract_attachments` (Z. 192-233), `persist_attachment` (Z. 247-299), `poll_once` (Z. 691-780, insbesondere persist_attachment-Aufrufstelle Z. 756-772) und `run_attachment_backfill` (Z. 810-939, insbesondere persist_attachment-Aufrufstelle Z. 903-925)
    - genossi_mail/src/dao.rs — Z. 111-122 für die `InboundMailAttachment`-Entity, um zu verstehen welche Felder `persist_attachment` setzen muss (insbesondere `size_bytes: i64`)
    - genossi_mail/src/inbox.rs Z. 1362-1390 — bestehender Test `test_persist_attachment_oversized_skips_storage`, damit sein Aufruf von `persist_attachment` mit der neuen Signatur weiterhin grün bleibt
    - genossi_mail/src/inbox.rs Z. 1524-1550 — bestehender Test `test_persist_attachment_rollback_on_db_fail` — selbe Anpassung
  </read_first>

  <behavior>
    - **Test 1 (NEU, RED-vor-GREEN):** `test_extract_attachments_oversized_skips_materialization` — Konstruiere eine multipart/mixed-Mail mit einem Attachment, dessen DECODIERTE Groesse > `ATTACHMENT_MAX_BYTES` ist. Pattern: ein Plain-Text-Attachment (`Content-Transfer-Encoding: 8bit`, kein Base64) mit > 10 MB Inhalt — z.B. via `vec![b'A'; ATTACHMENT_MAX_BYTES as usize + 1]` als Body-Part. Erwartung: `parse_raw_mail(raw).attachments[0].bytes.is_empty() == true` UND `parse_raw_mail(raw).attachments[0].declared_size > ATTACHMENT_MAX_BYTES`. Beweist: `to_vec()` wurde NICHT auf die volle Payload aufgerufen (sonst waere `bytes.len() == declared_size`).
    - **Test 2 (existierend, weiterhin gruen):** `test_persist_attachment_oversized_skips_storage` — Storage.save() wird 0-mal aufgerufen; DB-Row hat `oversized=true` + `relative_path=None`. Der bestehende Test ruft `persist_attachment(..., &bytes)` mit einem direkt allokierten 10MB+1-byte Vec auf — er muss um den neuen `declared_size`-Parameter erweitert werden (Wert: `bytes.len() as u64`, also `(ATTACHMENT_MAX_BYTES as usize + 1) as u64`).
    - **Test 3 (existierend, weiterhin gruen):** `test_persist_attachment_rollback_on_db_fail` — selbe Anpassung: `declared_size = bytes.len() as u64`.
    - **Test 4 (existierend, weiterhin gruen):** `test_parse_raw_mail_extracts_attachments` — kleines PNG-Attachment, unter dem Cap. Erwartung: `attachments[0].bytes.is_empty() == false`, `attachments[0].declared_size > 0`.
    - **Verhalten produktiver Code:** `extract_attachments` ruft NIE `part.contents().to_vec()` auf, wenn `part.contents().len() as u64 > ATTACHMENT_MAX_BYTES`. Stattdessen: `Vec::new()`. `declared_size` enthaelt IMMER `part.contents().len() as u64` (auch wenn `bytes` leer ist).
    - **persist_attachment Verhalten:** `oversized := declared_size > ATTACHMENT_MAX_BYTES`. `size_bytes := declared_size as i64`. KEINE Abhaengigkeit mehr von `bytes.len()` fuer die Oversized-Entscheidung (alte Logik aus Z. 256-257 wird ersetzt). Der bisherige `bytes.len() as i64` (Z. 256) wird durch `declared_size as i64` ersetzt.
  </behavior>

  <action>
    **Schritt 1 (RED): Neuen Test schreiben, der initial FAILT.**
    Fuege im `#[cfg(test)] mod tests`-Block (vor dem Schluss-`}`) folgenden Test ein:

    ```rust
    /// Phase 19 gap-closure (CR-01): extract_attachments fuehrt einen Probe-Read
    /// durch und allokiert die Bytes NICHT, wenn das Attachment die 10-MB-Cap
    /// (ATTACHMENT_MAX_BYTES) ueberschreitet. Beweist D-02 als Memory-DoS-Schutz
    /// VOR der Heap-Allokation.
    #[test]
    fn test_extract_attachments_oversized_skips_materialization() {
        // Body-part mit > ATTACHMENT_MAX_BYTES roher Payload (kein Base64,
        // damit decoded length == raw length und der probe-read greift).
        let oversized_payload = vec![b'A'; (ATTACHMENT_MAX_BYTES as usize) + 1024];
        let mut raw = Vec::new();
        raw.extend_from_slice(
            b"From: sender@example.com\r\n\
              To: inbox@example.com\r\n\
              Subject: Oversized Anhang\r\n\
              MIME-Version: 1.0\r\n\
              Content-Type: multipart/mixed; boundary=BOUNDARY\r\n\
              \r\n\
              --BOUNDARY\r\n\
              Content-Type: text/plain\r\n\
              \r\n\
              Anhang folgt.\r\n\
              --BOUNDARY\r\n\
              Content-Type: application/octet-stream\r\n\
              Content-Transfer-Encoding: 8bit\r\n\
              Content-Disposition: attachment; filename=\"huge.bin\"\r\n\
              \r\n",
        );
        raw.extend_from_slice(&oversized_payload);
        raw.extend_from_slice(b"\r\n--BOUNDARY--\r\n");

        let parsed = parse_raw_mail(&raw);
        assert_eq!(parsed.attachments.len(), 1, "expected exactly one attachment");
        let att = &parsed.attachments[0];
        assert!(
            att.declared_size > ATTACHMENT_MAX_BYTES,
            "declared_size ({}) must exceed cap ({}) — probe-read must record the real size",
            att.declared_size,
            ATTACHMENT_MAX_BYTES
        );
        assert!(
            att.bytes.is_empty(),
            "oversized attachment MUST NOT materialize bytes (got {} bytes; expected 0). \
             This proves part.contents().to_vec() was NOT called above the cap.",
            att.bytes.len()
        );
    }
    ```

    Lauf: `cargo test -p genossi_mail --lib test_extract_attachments_oversized_skips_materialization`. **MUSS jetzt FAILEN** (Compile-Error: `declared_size` existiert nicht; oder Assertion-Fail: `bytes` ist nicht leer). Das ist der RED-Schritt.

    **Schritt 2 (GREEN): ParsedAttachment um `declared_size: u64` erweitern.**

    Aktuell (Z. 158-163):
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedAttachment {
        pub file_name: String,
        pub mime_type: String,
        pub bytes: Vec<u8>,
    }
    ```

    Neu:
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedAttachment {
        pub file_name: String,
        pub mime_type: String,
        /// Materialized bytes — empty Vec when the attachment exceeds
        /// `ATTACHMENT_MAX_BYTES` (Probe-Read pattern, D-02 Memory-DoS guard).
        pub bytes: Vec<u8>,
        /// Real attachment size as reported by `mail_parser`'s
        /// `part.contents().len()`. Unlike `bytes.len()` this is always the
        /// declared size — even when `bytes` is empty due to the oversized
        /// guard. `persist_attachment` uses this to set `size_bytes` and to
        /// decide whether `oversized=true`.
        pub declared_size: u64,
    }
    ```

    **Schritt 3 (GREEN): `extract_attachments` auf Probe-Read umbauen.**

    Aktuell (Z. 192-233): unbedingtes `part.contents().to_vec()`.

    Neu — fuer JEDEN der beiden Pfade (is_message + else):
    ```rust
    fn extract_attachments(msg: &mail_parser::Message) -> Vec<ParsedAttachment> {
        use mail_parser::MimeHeaders;

        let mut out = Vec::new();
        for (idx, part) in msg.attachments().enumerate() {
            // Probe-Read (D-02 / CR-01): NEVER materialize bytes above the
            // cap. `part.contents()` returns &[u8] without allocation — only
            // `to_vec()` copies into the heap.
            let raw_len = part.contents().len();
            let oversized = raw_len as u64 > ATTACHMENT_MAX_BYTES;
            let declared_size = raw_len as u64;

            if part.is_message() {
                let bytes: Vec<u8> = if oversized {
                    Vec::new()
                } else {
                    part.contents().to_vec()
                };
                let name = part
                    .attachment_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("forwarded_{}.eml", idx));
                out.push(ParsedAttachment {
                    file_name: name,
                    mime_type: "message/rfc822".to_string(),
                    bytes,
                    declared_size,
                });
                continue;
            }

            let mime = part
                .content_type()
                .map(|ct| {
                    let mut s = String::from(ct.ctype());
                    if let Some(sub) = ct.subtype() {
                        s.push('/');
                        s.push_str(sub);
                    }
                    s
                })
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let name = part
                .attachment_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("attachment_{}.bin", idx));
            let bytes: Vec<u8> = if oversized {
                Vec::new()
            } else {
                part.contents().to_vec()
            };
            out.push(ParsedAttachment {
                file_name: name,
                mime_type: mime,
                bytes,
                declared_size,
            });
        }
        out
    }
    ```

    **Schritt 4 (GREEN): `persist_attachment`-Signatur um `declared_size` erweitern.**

    Aktuelle Signatur (Z. 247-254):
    ```rust
    async fn persist_attachment(
        storage: &dyn DocumentStorage,
        dao: &dyn InboundMailAttachmentDao,
        inbound_mail_id: Uuid,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<InboundMailAttachment, MailServiceError>
    ```

    Neue Signatur — `declared_size` als zusaetzlicher Parameter:
    ```rust
    async fn persist_attachment(
        storage: &dyn DocumentStorage,
        dao: &dyn InboundMailAttachmentDao,
        inbound_mail_id: Uuid,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
        declared_size: u64,
    ) -> Result<InboundMailAttachment, MailServiceError>
    ```

    Im Body von `persist_attachment` (aktuell Z. 255-257):
    - **Alt:** `let size = bytes.len() as i64;` + `let oversized = bytes.len() as u64 > ATTACHMENT_MAX_BYTES;`
    - **Neu:** `let size = declared_size as i64;` + `let oversized = declared_size > ATTACHMENT_MAX_BYTES;`

    Der Rest (storage.save(rel_path, bytes), Rollback-Pfad) bleibt unveraendert. WICHTIG: `storage.save` darf bei `oversized` nicht aufgerufen werden — das ist ueber `if let Some(ref rel_path) = relative_path` bereits korrekt verzweigt; `relative_path` ist bei `oversized` `None`.

    **Schritt 5 (GREEN): Beide Caller anpassen.**

    a) `poll_once` (Z. 755-773): `persist_attachment(...).await` Aufrufstelle. `att.declared_size` als achten Parameter durchreichen:
    ```rust
    if let Err(e) = persist_attachment(
        storage,
        attachment_dao,
        mail.id,
        &att.file_name,
        &att.mime_type,
        &att.bytes,
        att.declared_size,
    )
    .await
    { ... }
    ```

    b) `run_attachment_backfill` (Z. 903-925): selbe Anpassung:
    ```rust
    match persist_attachment(
        storage.as_ref(),
        attachment_dao.as_ref(),
        mail.id,
        &att.file_name,
        &att.mime_type,
        &att.bytes,
        att.declared_size,
    )
    .await
    { ... }
    ```

    **Schritt 6 (GREEN): Bestehende Tests an die neue persist_attachment-Signatur anpassen.**

    a) `test_persist_attachment_oversized_skips_storage` (Z. 1362-1390): Der Aufruf
    ```rust
    let result = persist_attachment(&storage, &dao, mail_id, "big.bin", "image/png", &bytes)
        .await
        .unwrap();
    ```
    wird zu:
    ```rust
    let declared = bytes.len() as u64;
    let result = persist_attachment(&storage, &dao, mail_id, "big.bin", "image/png", &bytes, declared)
        .await
        .unwrap();
    ```

    b) `test_persist_attachment_rollback_on_db_fail` (Z. 1524-1550): selbe Anpassung. Aufruf:
    ```rust
    let declared = bytes.len() as u64;
    let result =
        persist_attachment(&storage, &dao, mail_id, "doc.pdf", "application/pdf", &bytes, declared).await;
    ```

    c) `test_parse_raw_mail_extracts_attachments` (Z. 1329-1358): KEINE Aenderung am Test-Setup noetig — `declared_size` ist nur an `ParsedAttachment` neu. Optional: Assertion ergaenzen
    ```rust
    assert!(p.attachments[0].declared_size > 0);
    assert_eq!(p.attachments[0].declared_size as usize, p.attachments[0].bytes.len());
    ```
    (Belegt: unter dem Cap stimmen `declared_size` und `bytes.len()` ueberein.)

    **Schritt 7 (verify): `cargo test -p genossi_mail --lib` → muss komplett gruen sein (175 vorherige + 1 neue = 176 Tests passed).**
  </action>

  <verify>
    <automated>cargo test -p genossi_mail --lib 2>&1 | tail -20</automated>
    <automated>grep -c 'pub declared_size: u64' genossi_mail/src/inbox.rs</automated>
    <automated>grep -cE 'let raw_len = part\.contents\(\)\.len\(\)' genossi_mail/src/inbox.rs</automated>
    <automated>grep -nE 'part\.contents\(\)\.to_vec\(\)' genossi_mail/src/inbox.rs</automated>
    <automated>grep -c 'fn test_extract_attachments_oversized_skips_materialization' genossi_mail/src/inbox.rs</automated>
    <automated>cargo check --workspace --exclude genossi-frontend 2>&1 | tail -5</automated>
  </verify>

  <acceptance_criteria>
    - `grep -c 'pub declared_size: u64' genossi_mail/src/inbox.rs` returns exactly `1` (Feld definiert auf ParsedAttachment)
    - `grep -cE 'let raw_len = part\.contents\(\)\.len\(\)' genossi_mail/src/inbox.rs` returns `1` (probe-read in extract_attachments einmalig — die Variable wird fuer beide Pfade wiederverwendet)
    - `grep -nE 'part\.contents\(\)\.to_vec\(\)' genossi_mail/src/inbox.rs` zeigt: jede verbliebene `to_vec()`-Stelle liegt INNERHALB eines `if oversized { Vec::new() } else { part.contents().to_vec() }`-Ausdrucks (manuelle Verifikation: 2 hits erwartet, beide im else-Zweig)
    - `grep -c 'fn test_extract_attachments_oversized_skips_materialization' genossi_mail/src/inbox.rs` returns `1`
    - `grep -c 'declared_size: u64' genossi_mail/src/inbox.rs` returns >= 2 (struct-Feld + persist_attachment-Parameter)
    - `grep -cE 'oversized = declared_size > ATTACHMENT_MAX_BYTES' genossi_mail/src/inbox.rs` returns `1` (Cap-Check anhand declared_size statt bytes.len())
    - `cargo test -p genossi_mail --lib` ist gruen mit mindestens 176 Tests (175 vorher + 1 neu) — kein FAILED, kein ignored
    - `cargo test -p genossi_mail --lib test_extract_attachments_oversized_skips_materialization -- --nocapture` ist gruen
    - `cargo test -p genossi_mail --lib test_persist_attachment_oversized_skips_storage -- --nocapture` ist gruen (Regression: die bestehende Cap-as-Marker-Logik bleibt korrekt)
    - `cargo test -p genossi_mail --lib test_persist_attachment_rollback_on_db_fail -- --nocapture` ist gruen
    - `cargo check --workspace --exclude genossi-frontend` ist 0 Errors (`poll_once` + `run_attachment_backfill` callsites korrekt aktualisiert, keine inkonsistenten Signaturen)
  </acceptance_criteria>

  <done>
    `ParsedAttachment.declared_size` existiert und wird von `extract_attachments` immer auf `part.contents().len() as u64` gesetzt. Bei `declared_size > ATTACHMENT_MAX_BYTES` enthaelt `bytes` einen leeren `Vec` — beweisbar durch den neuen Unit-Test `test_extract_attachments_oversized_skips_materialization`. `persist_attachment` berechnet `oversized` und `size_bytes` aus `declared_size`, nicht aus `bytes.len()`. Beide Caller (`poll_once`, `run_attachment_backfill`) reichen `att.declared_size` durch. Alle bestehenden 175 Tests in `genossi_mail/lib` bleiben gruen, der neue Test ist gruen, `cargo check --workspace --exclude genossi-frontend` 0 Errors.
  </done>
</task>

<task type="auto">
  <name>Task 2: Workspace-Build + Full-Test-Suite Regression Check</name>
  <files>(no file modifications — verification only)</files>

  <read_first>
    - genossi_mail/src/inbox.rs (final state after Task 1) — um zu pruefen, dass keine `to_vec()`-Aufrufe ausserhalb von `else { ... }`-Zweigen uebrig geblieben sind
  </read_first>

  <action>
    Verifikation, dass der Fix keine Regressionen verursacht:

    1. **Full workspace check:** `cargo check --workspace --exclude genossi-frontend` — 0 Errors.
    2. **Genossi-mail tests:** `cargo test -p genossi_mail` (lib + integration falls vorhanden) — alle gruen.
    3. **Manuelle Code-Inspektion via Grep:**
       - `grep -nE 'part\.contents\(\)\.to_vec\(\)' genossi_mail/src/inbox.rs` — JEDE verbleibende Zeile muss im else-Zweig eines `if oversized` liegen. Falls eine Zeile UNCONDITIONAL ist (kein vorangehender `if oversized` im selben Block), ist das ein FAIL.
       - `grep -nE 'persist_attachment\(' genossi_mail/src/inbox.rs` — jede Call-Site muss 7 Argumente haben (storage, dao, inbound_mail_id, file_name, mime_type, bytes, declared_size). Es gibt 4 Aufrufe: 2 in produktiven Callern (poll_once, run_attachment_backfill), 2 in Tests (test_persist_attachment_oversized_skips_storage, test_persist_attachment_rollback_on_db_fail).
    4. **Clippy als Quality-Gate:** `cargo clippy -p genossi_mail --all-targets --no-deps -- -D warnings` — KEINE neuen Warnings durch den Fix einfuehren. Falls bestehende Warnings vorhanden sind, nur diejenigen ignorieren, die nicht in `inbox.rs` liegen (Pre-existing).

    Falls Schritt 1, 2 oder 4 fehlschlaegt: zurueck zu Task 1, Fehlerursache fixen, dann erneut diesen Task ausfuehren.
  </action>

  <verify>
    <automated>cargo check --workspace --exclude genossi-frontend 2>&1 | tail -3</automated>
    <automated>cargo test -p genossi_mail 2>&1 | tail -10</automated>
    <automated>grep -nE 'part\.contents\(\)\.to_vec\(\)' genossi_mail/src/inbox.rs</automated>
    <automated>grep -cE 'persist_attachment\(' genossi_mail/src/inbox.rs</automated>
    <automated>cargo clippy -p genossi_mail --all-targets --no-deps 2>&1 | grep -E 'warning|error' | grep 'inbox.rs' | head -20</automated>
  </verify>

  <acceptance_criteria>
    - `cargo check --workspace --exclude genossi-frontend` Exit-Code 0, Output endet mit "Finished `dev` profile"
    - `cargo test -p genossi_mail` zeigt "test result: ok. X passed; 0 failed" wobei X >= 176
    - `grep -nE 'part\.contents\(\)\.to_vec\(\)' genossi_mail/src/inbox.rs`: jede Zeile, die ausgegeben wird, liegt in den selben 5 Zeilen vor der Zeile ein `else {`-Block-Opener (manuelle Inspektion); ABER der einzige robuste Grep-Gate ist `grep -cE '^\s*part\.contents\(\)\.to_vec\(\)' genossi_mail/src/inbox.rs` → muss `0` ergeben (keine unconditional Top-Level-Statements). Bytes-Felder-Initialisierungen wie `bytes: part.contents().to_vec(),` als Feld in einer Struct-Literal-Initialisierung sind NICHT erlaubt; diese muessen `bytes` als Variable verwenden, die zuvor in `if oversized { Vec::new() } else { part.contents().to_vec() }` zugewiesen wurde.
    - `grep -cE 'persist_attachment\(' genossi_mail/src/inbox.rs` zeigt mindestens `4` (definition + 3 Aufrufe: poll_once + run_attachment_backfill + 2 Tests = total 5 mit Definition. Akzeptiert: >= 4).
    - `cargo clippy -p genossi_mail --all-targets --no-deps` zeigt KEINE neuen Warnings, die durch diesen Fix in inbox.rs entstanden sind (Vergleich gegen Pre-Fix-Zustand: 0 neue inbox.rs-Warnings).
  </acceptance_criteria>

  <done>
    Workspace baut sauber, alle `genossi_mail`-Tests gruen, keine `part.contents().to_vec()`-Aufrufe ausserhalb von `if oversized` guards, keine neuen Clippy-Warnings auf `inbox.rs`. Phase 19 ist damit BLOCKER-frei.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p genossi_mail --lib` → 176+ Tests passed, 0 failed
- `cargo check --workspace --exclude genossi-frontend` → 0 Errors
- Neuer Test `test_extract_attachments_oversized_skips_materialization` beweist Probe-Read-Verhalten:
  - `attachments[0].declared_size > ATTACHMENT_MAX_BYTES`
  - `attachments[0].bytes.is_empty() == true`
- Bestehender Test `test_persist_attachment_oversized_skips_storage` bleibt gruen (D-02 als Persistenz-Marker)
- Bestehender Test `test_persist_attachment_rollback_on_db_fail` bleibt gruen (T-07 save-then-DB-Rollback)
- Grep-Check: keine unbedingten `part.contents().to_vec()`-Aufrufe in `extract_attachments`
- Grep-Check: `pub declared_size: u64` exakt 1× definiert
</verification>

<success_criteria>
**Definition of Done (Gap-Closure):**

- [ ] `ParsedAttachment` Struct enthaelt `pub declared_size: u64`
- [ ] `extract_attachments` fuehrt Probe-Read durch (`let raw_len = part.contents().len()`) VOR jeder `to_vec()`-Allokation
- [ ] Bei `raw_len as u64 > ATTACHMENT_MAX_BYTES` wird `Vec::new()` allokiert; `to_vec()` NIE aufgerufen
- [ ] `persist_attachment` akzeptiert `declared_size: u64` als Parameter; berechnet `oversized` + `size_bytes` daraus
- [ ] `poll_once` und `run_attachment_backfill` reichen `att.declared_size` an `persist_attachment` durch
- [ ] Neuer Test `test_extract_attachments_oversized_skips_materialization` ist gruen
- [ ] Alle 175 vorherigen `genossi_mail`-lib-Tests bleiben gruen (Regression-frei)
- [ ] `cargo check --workspace --exclude genossi-frontend` 0 Errors
- [ ] Keine neuen Clippy-Warnings auf `genossi_mail/src/inbox.rs`
- [ ] Threat T-19-07-01 (Memory-Exhaustion-DoS) gemildert: VERIFICATION-Truth #3 ("D-02 Memory-DoS-Schutz: 10-MB-Cap greift VOR Heap-Allokation") ist nun erfuellbar
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-07-SUMMARY.md` documenting:
- Probe-Read-Pattern in `extract_attachments` umgesetzt (Zeilenangaben)
- `ParsedAttachment` um `declared_size` erweitert (semantische Begruendung)
- `persist_attachment`-Signatur erweitert; alle Caller migriert
- Neuer Test deckt das Memory-DoS-Verhalten ab
- VERIFICATION-Truth #3 ist damit erfuellt; Phase 19 BLOCKER-frei
- Out-of-scope: WR-01/02/04/05/06 + IN-01..IN-05 verbleiben als Follow-up (Defense-in-Depth)
</output>
