# Requirements: v1.4 Mail-Formatierung & Antrags-Dokumente

**Milestone:** v1.4
**Defined:** 2026-06-29
**Goal:** Vorstände versenden professionell formatierte HTML-Mails (statt nur Rohtext) und können den originalen Mitgliedsantrag als Datei am Antrag hinterlegen, die beim Aktivieren automatisch ans Mitglied übergeht.

Research: `.planning/research/SUMMARY.md` (+ STACK / FEATURES / ARCHITECTURE / PITFALLS). Net neue Backend-Dependency: genau eine — `ammonia` (HTML-Sanitization, serverseitig, nie WASM).

---

## v1.4 Requirements

### MAIL — Mail-Versand-Foundation (8bit & geteilter Body-Helfer)

- [x] **MAIL-01**: Alle ausgehenden Mails werden über einen einzigen geteilten Body-Bau-Helfer in `genossi_mail` erzeugt, sodass die drei heute divergierenden Sendepfade (`worker.rs`, `service.rs` Test-Mail, `service.rs`/`digest.rs` Digest) konsistent denselben Content-Aufbau und `charset=utf-8` verwenden.
- [x] **MAIL-02**: Der Text-Teil ausgehender Mails kann als `8bit` statt `quoted-printable` kodiert werden, sodass keine sichtbaren `=`-Soft-Line-Breaks mehr entstehen.
- [x] **MAIL-03**: Die Kodierung (8bit vs quoted-printable) ist per Konfiguration umschaltbar; Default bleibt quoted-printable als sicherer Fallback, bis `8BITMIME` am Produktiv-Relay bestätigt ist.
- [ ] **MAIL-04**: Vor Aktivierung von 8bit in Produktion wird per EHLO/`8BITMIME`-Capability-Check am konfigurierten Relay verifiziert, dass der Server 8bit unterstützt (dokumentierter Verifikations-Schritt; aus der Dev-Umgebung nicht durchführbar, da Relay nur über Produktiv-Netz erreichbar).
- [x] **MAIL-05**: Bestehende reine Textmails (Massenmail, Test-Mail, Digest) funktionieren unverändert weiter (Backward-Compatibility).

### HTML — HTML-Mail-Backend (multipart/alternative)

- [x] **HTML-01**: Eine Mail kann mit Text- UND HTML-Teil als `multipart/alternative` versendet werden (Text zuerst, niemals HTML-only), inklusive korrekter Verschachtelung mit Anhängen (`mixed{ alternative{plain, html}, attachments }`).
- [x] **HTML-02**: Der Plain-Text-Teil stammt aus dem bestehenden, vom Autor verfassten `body` (keine Ableitung aus HTML, keine zusätzliche Crate).
- [x] **HTML-03**: Mail-Templates und Mail-Jobs können einen optionalen HTML-Body (`body_html`) speichern; Migration ist forward-only `ALTER TABLE … ADD COLUMN … NULL`. Legacy-Zeilen (NULL) ergeben weiterhin reine Textmails.
- [x] **HTML-04**: Template-Variablen werden sowohl in den Text- als auch in den HTML-Body interpoliert; die HTML-Render-Variante nutzt eine separate autoescapende minijinja-Env, sodass mitglieds-/nutzergelieferte Werte HTML-escaped werden, während die vom Autor verfasste Markup-Struktur erhalten bleibt. Die bestehende `strict_env()` bleibt für Text und Subject unverändert.
- [x] **HTML-05**: Vom Vorstand verfasstes HTML wird serverseitig mit `ammonia` saniert (Whitelist: fett/kursiv/Links/Listen/Absätze), bevor es gespeichert/versendet wird — angewendet an allen Eintritts-Punkten (`create_job`, Template-Create/Update, Test-Mail-Pfad). Frontend-Sanitization gilt nicht als Sicherheitsgrenze.

### EDIT — WYSIWYG-Editor (Frontend)

- [x] **EDIT-01**: Vorstände verfassen formatierte Mails in einem WYSIWYG-Editor (mindestens fett, kursiv, Links, Aufzählungs-/nummerierte Listen) als wiederverwendbare Dioxus-Component, die den bestehenden `body_editor`-Textarea im Mail-Compose-Flow ersetzt.
- [x] **EDIT-02**: Der Editor erzeugt sauberes, sanitisierbares HTML (`styleWithCSS=false` erzwingen → `<b>/<i>`-Tags statt inline-`style`, damit ammonia die Formatierung nicht strippt) und keine neuen Frontend-Dependencies (contenteditable + `execCommand` über vorhandenes web-sys).
- [x] **EDIT-03**: Der HTML-Inhalt des Editors wird beim Absenden zuverlässig aus dem contenteditable-DOM ausgelesen und mit dem Dioxus-State synchronisiert (kein Datenverlust beim Submit).
- [x] **EDIT-04**: Eingefügter Inhalt (Paste, z. B. aus Word/Browser) wird beim Einfügen bereinigt, sodass kein verschmutztes Markup in den Mail-Body gelangt.
- [x] **EDIT-05**: Eine Live-Vorschau zeigt dem Vorstand das gerenderte HTML vor dem Versand.

### APDOC — Antrags-Dokument & Auto-Übernahme

- [ ] **APDOC-01**: Ein Admin kann eine Datei (z. B. eingescannter Original-Antrag als PDF) an eine `Application` hochladen; die Datei wird über `DocumentStorage` im Filesystem gespeichert (nicht in der DB), Endpunkt spiegelt das bestehende `member_document`-Upload-Muster (Multipart, `DefaultBodyLimit`, MIME-Allowlist, UUID-Pfad gegen Path-Traversal).
- [ ] **APDOC-02**: Der Upload-Endpunkt ist admin-only (der Antrags-Submit-Pfad bleibt `PUBLIC`); dabei wird die carry-forward CR-02 Permission-Check-Ordering an dieser Stelle korrekt umgesetzt.
- [ ] **APDOC-03**: Beim Aktivieren (`confirm`) einer `Application` wird ein hinterlegtes Antrags-Dokument **übernommen** (Ownership-Übergabe — Move-Semantik: die `application_documents`-Zeile wird soft-deleted und die Datei physisch an den Member-Pfad verschoben) und als auditiertes `MemberDocument` am Mitglied angelegt — innerhalb derselben atomaren Aktivierungs-Transaktion, via `audited_create!` unter `APPLICATION_SERVICE_PROCESS`, mit `DocumentType::Other` + beschreibender Bezeichnung („Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)“).
- [ ] **APDOC-04**: Aktivierung ist robust gegen Edge-Cases: Antrag ohne Dokument (übernimmt nichts, kein Fehler), Re-Aktivierung wird durch den bestehenden `Offen`-Status-Guard verhindert (keine Doppel-Übernahme), fehlende Datei auf dem Filesystem → Transaktion rollt zurück.
- [ ] **APDOC-05**: Das Antrags-Dokument ist im Frontend an der Application sichtbar und herunterladbar (admin-only).

### FMT — Mail-Datumsformatierung

- [x] **FMT-01**: Datums-Template-Variablen (`join_date`, `exit_date`, ggf. weitere) werden in Mails im deutschen Format `DD.MM.YYYY` (z. B. `02.07.2026`) gerendert statt im technischen `.to_string()`-Default — konsistent in Text- und HTML-Mails. Root-Cause: `genossi_mail/src/template.rs:17-18` nutzt `.to_string()`; Fix via `time::format_description`-Vorlage `"[day].[month].[year]"` (kleiner geteilter Helfer + Unit-Test analog `test_exit_date_null`).

---

## Future Requirements (deferred — nicht in v1.4)

- **HTML-Mail-Bilder/Branding**: Eingebettete Bilder, Briefkopf/Logo, Inline-CSS-Branding — differenzierend, nicht table-stakes für v1.4.
- **Gespeicherte HTML-Templates-Bibliothek**: Vorlagen-Verwaltung über das bestehende Template-Feature hinaus.
- **`List-Unsubscribe`-Header**: Für größere Massenmailings sinnvoll; v1.4-Zielgruppe ist klein (Vorstand → Mitglieder).
- **Inbound-HTML-Rendering (Posteingang)**: Sichere sandboxed Anzeige eingehender HTML-Mails im Frontend — separater Sicherheits-Scope.
- **Mehrere Dateien pro Application**: v1.4 deckt das eine Original-Antrags-Dokument ab.

## Out of Scope (explizite Ausschlüsse)

- **Full Email-Builder / Drag-and-Drop-Blöcke**: Überdimensioniert für die Zielgruppe; WYSIWYG mit Basis-Formatierung genügt.
- **JS-Editor-Bibliotheken (Quill/TipTap/Trix) via wasm-bindgen-Bundle**: Bewusst abgelehnt (Bundle-Größe, Interop-Komplexität) — contenteditable + execCommand reicht.
- **Hand-verfasste duale Bodies**: Autor schreibt nicht separat Text + HTML von Hand; Text = bestehender body, HTML = Editor-Ausgabe.
- **DB-BLOB-Speicherung von Anhängen/Dokumenten**: Bleibt beim Filesystem-`DocumentStorage`.
- **`html2text`-Ableitung des Text-Teils**: Verworfen zugunsten des bestehenden body.

---

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| MAIL-01 | Phase 22 | Complete |
| MAIL-02 | Phase 22 | Complete |
| MAIL-03 | Phase 22 | Complete |
| MAIL-04 | Phase 22 | pending |
| MAIL-05 | Phase 22 | Complete |
| HTML-01 | Phase 23 | Complete |
| HTML-02 | Phase 23 | Complete |
| HTML-03 | Phase 23 | Complete |
| HTML-04 | Phase 23 | Complete |
| HTML-05 | Phase 23 | Complete |
| FMT-01 | Phase 23 | Complete |
| EDIT-01 | Phase 24 | Complete |
| EDIT-02 | Phase 24 | Complete |
| EDIT-03 | Phase 24 | Complete |
| EDIT-04 | Phase 24 | Complete |
| EDIT-05 | Phase 24 | Complete |
| APDOC-01 | Phase 25 | pending |
| APDOC-02 | Phase 25 | pending |
| APDOC-03 | Phase 25 | pending |
| APDOC-04 | Phase 25 | pending |
| APDOC-05 | Phase 25 | pending |

**Coverage:** 21/21 v1.4-Requirements gemappt (100%). Keine Orphans, keine Duplikate.

**Phasen-Mapping:**

- **Phase 22 — 8bit + Shared Mail-Body Helper:** MAIL-01..05 (Service-only, keine Schema-Änderung, kein Audit)
- **Phase 23 — HTML Mail Backend:** HTML-01..05 + FMT-01 (DAO→Service→REST, forward-only Migration `body_html`, ammonia-Gate, deutsches Datumsformat in Template-Variablen, kein Audit)
- **Phase 24 — WYSIWYG Frontend Editor:** EDIT-01..05 (Frontend, keine neuen Deps, kein Audit)
- **Phase 25 — Application File Upload + Audited Carryover:** APDOC-01..05 (DAO→Service→REST→Frontend; `application_documents`-Tabelle NICHT auditiert, Carryover-`MemberDocument` IST auditiert; unabhängig/parallelisierbar zu 22→23→24)

_Befüllt vom Roadmapper 2026-06-29._
