# Phase 6: Teilnehmerlisten-Export für Generalversammlungen - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Vorstand kann nach Schluss einer Generalversammlung die Teilnehmerliste in drei Formaten (PDF, CSV, XLSX) exportieren und als Anhang an das schriftliche GV-Protokoll heften. Datenbasis ist der unveränderliche Member-Universe-Snapshot aus Phase 1 plus der Anwesenheits-Stand aus Phase 3 — Phase 6 liest, sie schreibt nichts an Mitglieds- oder Anwesenheitsdaten.

**In scope:**
- REST-Endpoint zum Export pro GV in drei Formaten
- UI-Button im Vorstand-Detail-View einer geschlossenen GV
- Wahlmöglichkeit „alle Mitglieder mit Anwesenheits-Spalte" vs. „nur Anwesende"
- Konsistentes Filename-Schema und DACH-taugliche Datei-Formate

**Out of scope (gehört in spätere Phasen oder ist explizit nicht gewollt):**
- Export für offene oder noch in Vorbereitung befindliche GVs
- Helfer-Zugriff auf den Export
- Automatischer Versand des Exports per E-Mail oder Upload an Verband
- Multi-GV-Sammelexport (z. B. Jahresübersicht)
- Audit-Hashchain-Einträge für Export-Aktionen
- Zusatzfelder im PDF (Logo, Unterschriftenspalte, Fußzeile mit Export-User)

</domain>

<decisions>
## Implementation Decisions

### Formate

- **D-01:** Drei Formate werden parallel angeboten: PDF (Protokoll-Anhang), CSV (Verband / Weiterverarbeitung), XLSX (Excel-natives Format).
- **D-02:** XLSX wird mit `rust_xlsxwriter` erzeugt — Pure-Rust, aktiv gepflegt, keine FFI-Bindings. Crate ist neue Workspace-Dependency.
- **D-03:** CSV ist Semikolon-getrennt und UTF-8 mit BOM kodiert, damit deutsche Excel-Installationen die Datei ohne „Daten → Aus Text"-Wizard direkt öffnen.
- **D-04:** PDF wird über die bestehende Typst-Toolchain (`typst` + `typst-pdf`) erzeugt, parallel zum bereits etablierten Pattern in `genossi_service_impl/src/pdf_generation.rs` (siehe `join_confirmation.typ`, `zahlungsanfrage.typ`).

### Inhalt & Daten

- **D-05:** Datenquelle ist der `assembly_member_snapshot` plus die `attendance`-Tabelle — historisch stabil. Nachträgliche Member-Mutationen (Namensänderung, Austritt) wirken sich NICHT auf bereits generierte Exporte aus. Konsistenz mit ASSY-02 (Snapshot beim Open der GV) und ASSY-05 (Persistenz nach Schluss).
- **D-06:** Spalten-Whitelist im Export: `member_number`, `first_name`, `last_name`, `salutation`, `title`, `is_present` — exakt die ATTN-01-DSGVO-Whitelist aus `AttendanceMemberRow` (`genossi_dao/src/attendance.rs:45`). `member_id` (UUID) ist im REST nötig für Frontend-PUT/DELETE, im Export-File aber NICHT relevant — Researcher prüft, ob die UUID-Spalte im Export weggelassen werden soll.
- **D-07:** Sortierung ist `member_number ASC` — konsistent mit der Live-Liste seit commit `ed754fc` (`feat(attendance): sort member list by member_number ASC`).
- **D-08:** PDF enthält einen Kopf-Block mit GV-Titel, GV-Datum und Zähler „X von Y anwesend" (X = anwesende, Y = `total` aus `assembly_member_snapshot`). Danach folgt die Tabelle. Keine Unterschriftenspalte, kein Logo, keine Fußzeile mit Export-User/Versionsnummer.

### Auswahl & Filterung

- **D-09:** Mitglieder-Auswahl ist über Query-Parameter `?include=all|present` steuerbar — `all` liefert alle Snapshot-Mitglieder plus Anwesenheits-Spalte, `present` liefert nur die anwesenden. Default-Wert von `include` und die genaue Darstellung der Anwesenheits-Spalte (z. B. `"ja"`/`"nein"`, `1`/`0`, leer/`✓`) entscheidet der Planner — Recommendation: Default `all`, Darstellung `"ja"`/`"nein"` für CSV/XLSX, Häkchen-Glyphe `✓` / leer für PDF.
- **D-10:** `include=present` ergibt im PDF einen reduzierten Kopf — „X anwesend" statt „X von Y" — weil Y aus der Liste nicht ableitbar ist; Planner kann auch entscheiden, Y trotzdem aus dem Snapshot zu lesen und im Kopf mitzuzeigen. Beide Varianten sind tragbar.

### Lifecycle & Berechtigung

- **D-11:** Export ist NUR für GVs im Status `Geschlossen` erlaubt. Aufruf gegen `Vorbereitung` oder `Geöffnet` liefert 409 Conflict (mit klarer Fehlermeldung). Hintergrund: Export ist Endbeleg fürs Protokoll — Zwischenstände wurden bewusst ausgeschlossen, damit Vorstand nicht versehentlich einen vorläufigen Stand ans Protokoll heftet.
- **D-12:** Post-Close-Korrekturen (ASSY-06) wirken sich auf nachfolgende Exporte aus — der Export reflektiert immer den aktuellen Stand der `attendance`-Tabelle. Dokumentation in OpenAPI sollte das explizit erwähnen.
- **D-13:** Zugriff nur für OIDC-authentifizierten Vorstand (Permission-Check identisch zum bestehenden Assembly-Detail-Endpoint). Helfer haben KEINEN Zugriff auf den Export — Datenexposition für Helfer bleibt auf den live-Abruf während der laufenden GV begrenzt (Phase 2/3 Datenschutz-Linie).

### REST-API

- **D-14:** Ein Endpoint mit Format via Pfad-Suffix: `GET /api/assembly/{aid}/attendance-export/{format}` mit `format ∈ {csv, pdf, xlsx}`. Query-Parameter `?include=all|present`.
- **D-15:** Content-Disposition-Filename folgt dem Schema `gv-{YYYY-MM-DD}-teilnehmer.{ext}` — Datum aus `assembly.date`, Extension matched Format. Beispiel: `gv-2026-05-15-teilnehmer.pdf`. Sortierbar im Dateimanager, keine Sonderzeichen-Probleme, keine Slugify-Logik nötig.
- **D-16:** MIME-Types: `application/pdf` (existierender Pattern in `genossi_rest/src/member_document.rs`), `text/csv; charset=utf-8` mit BOM im Body-Vorspann, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` für XLSX.

### Audit & Logging

- **D-17:** KEIN Audit-Hashchain-Eintrag für Export-Aufrufe — Export ist Read-Only auf bereits existierende Daten. Konsistent mit ATTN-05 (Anwesenheits-Markierungen nicht auditiert) und der projektweiten Linie „nur create/update/delete sind auditiert" (Member, MemberAction, MemberDocument, Application, Assembly, HelperToken).
- **D-18:** Server-seitiges `tracing::info!` mit GV-ID, Format, `include`-Parameter und User-ID ist ausreichend für operative Sichtbarkeit (Standard-Pattern im Codebase). Kein eigenes Audit-Entity nötig.

### UI

- **D-19:** Download-Button(s) sitzen in der bestehenden Assembly-Detail-Seite des Vorstand-Frontends. Sichtbar/aktivierbar NUR wenn `assembly.status == Closed`. Drei separate Download-Knöpfe (CSV/PDF/XLSX) oder ein einzelner Knopf mit Format-Dropdown — Planner entscheidet basierend auf UI-Konsistenz mit bestehenden Pattern (`AssemblyStatusBadge`, `TabStrip`, etc. aus Phase 4).
- **D-20:** Component-First gilt: Wenn der Download-Knopf-Block (Format-Auswahl + Include-Toggle) auf zwei Pages auftauchen sollte, geht er in `genossi-frontend/src/component/`. Im Phase-6-Scope ist nur EINE Page betroffen — Komponentisierung nur falls Wiederverwendung absehbar ist.

### Claude's Discretion

Der Planner / Researcher hat Spielraum bei:
- Default-Wert von `?include` (Empfehlung: `all`).
- Darstellung der Anwesenheits-Spalte pro Format (z. B. `"ja"`/`"nein"` vs. `✓`/leer).
- Ob `member_id` (UUID) im Export-File erscheint oder nicht (Empfehlung: weglassen — Export ist für Menschen, REST liefert UUIDs separat).
- Ob im PDF Spaltenbreiten/Layout per Typst-Helper gesetzt werden oder die Defaults reichen.
- Ob der UI-Button als Dropdown oder als drei separate Buttons erscheint.
- Ob die Backend-Implementierung als neue Crate `genossi_export` oder im bestehenden `genossi_service_impl`/`genossi_rest` lebt.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Anforderungen
- `.planning/ROADMAP.md` §Phase 6 — Phase-Eintrag (Goal noch TBD, wird im Plan-Step gefüllt)
- `.planning/PROJECT.md` — Core Value, Outcome-Liste; relevant: „GV-Ergebnis (Anzahl Anwesender + Anwesenheits-Liste) bleibt nach GV-Schluss persistent für Protokoll und Statistik"
- `.planning/REQUIREMENTS.md` — Phase-1–4-Requirements als Grundlage (ASSY-02, ASSY-05, ATTN-01)

### Datenmodell & DAO-Layer
- `genossi_dao/src/attendance.rs:40-53` — `AttendanceMemberRow` mit DSGVO-7-Spalten-Whitelist; SELECT-Whitelist-Kommentar als Schutz vor PII-Leaks
- `genossi_dao/src/assembly.rs` (und `assembly_member_snapshot`) — Snapshot-Entity, deren Daten der Export liest
- `genossi_dao_impl_sqlite/src/attendance.rs` — SQLite-Implementierung der `AttendanceDao::list_members`-Query, Vorlage für die Export-Variante
- `genossi_service_impl/src/attendance.rs` — `AttendanceServiceImpl::check_assembly_access` + 4 Methods; der Export-Service teilt sich Permission-Funnel und Snapshot-Lookups

### REST & Frontend-Pattern
- `genossi_rest/src/attendance.rs` — Handler-Pattern für die bestehenden Attendance-Endpoints, inkl. `map_attendance_error` (PermissionDenied → 403)
- `genossi_rest/src/member_document.rs` — bestehender File-Download-Endpoint mit `Content-Disposition: attachment; filename=...` und `application/pdf` als MIME — Pattern-Anker für Phase 6
- `genossi_rest_types/src/lib.rs:1700-1734` — `AttendanceMemberTO` + `From<&AttendanceMemberRow>`; identische Whitelist soll für Export-Body gelten
- `genossi-frontend/src/page/assembly_details.rs` — Vorstand-Detail-View, in den der Download-Button(-Block) integriert wird
- `genossi-frontend/src/api.rs:1639-1655` — `AttendanceMemberTO` / `AttendanceStatsTO` (Frontend); Vorbild für eventuelle Frontend-Typen oder direkte Blob-Downloads
- `genossi-frontend/CLAUDE.md` — Component-First-Pflicht für wiederverwendbare UI

### PDF/Typst-Toolchain
- `genossi_service_impl/src/pdf_generation.rs` — Typst-`World`-Implementierung, Package-Cache, eingebettete Fonts; Phase 6 nutzt denselben Mechanismus
- `templates/join_confirmation.typ`, `templates/zahlungsanfrage.typ`, `templates/_layout.typ` — bestehende Typst-Templates als Stil-Anker für das neue `teilnehmerliste.typ`
- `fonts/LiberationSans-*.ttf` — bereits eingebettete Fonts, verfügbar für Phase 6

### Audit-Linie (zur Begründung, warum NICHT auditiert)
- `genossi_service_impl/src/audit_macros.rs` — die Macros, die hier bewusst NICHT verwendet werden
- Prior-Phase-Decision aus `.planning/PROJECT.md:52` und `.planning/REQUIREMENTS.md:36` (ATTN-05) — Read-only und Anwesenheits-Daten sind nicht im Hashchain-Scope

### CLAUDE.md (Verhaltensregeln)
- `CLAUDE.md` §„Audit Log System" — Neue auditierte Entitäten müssten `Auditable` implementieren; Phase 6 hat KEINE neuen auditierten Entitäten
- `CLAUDE.md` §„Component-First Frontend" — gilt für eventuelle UI-Wiederverwendung
- `CLAUDE.md` §„Datetime Handling" / „Transfer Objects" — relevante Konventionen falls Export-API neue TOs einführt

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`AttendanceMemberRow` (DAO) + `AttendanceMemberTO` (REST)** — bestehende 7-Spalten-Whitelist deckt fast den vollständigen Export-Inhalt ab; vermutlich kann ein neuer DAO-Call `list_for_export(assembly_id, include)` direkt diese Row-Struktur wiederverwenden (oder eine kleine Erweiterung).
- **`AttendanceServiceImpl::check_assembly_access`** — Permission-Funnel + Assembly-Status-Lookup; Export-Service kann denselben Funnel verwenden (admin-only branch genügt, Helper-Branch entfällt komplett).
- **`assembly_member_snapshot_dao`** — Snapshot-DAO ist bereits Arc-shared zwischen AssemblyServiceImpl und AttendanceServiceImpl (Pattern aus Plan 03-06); Export-Service hängt sich ohne Mehraufwand dran.
- **Typst-`World` + Package-Cache + eingebettete Fonts** in `pdf_generation.rs` — komplette PDF-Toolchain ist da; nur ein neues Template + ein neuer Service-Call sind nötig.
- **File-Download-Pattern in `member_document.rs`** — `Content-Disposition: attachment; filename=...`, `application/pdf` MIME, Body-Bytes als `Body::new(bytes)` — direkt übertragbar auf alle drei Export-Formate.
- **Frontend-Component-Inventar aus Phase 4** — `AssemblyStatusBadge`, `Toast`, `TabStrip` etc. sind verfügbar; Download-Button-Bereich kann sich konsistent einreihen.

### Established Patterns
- **Layered Architecture (DAO → Service → REST → Frontend)** — Reihenfolge bei der Implementierung. Genossi-Konvention „Backend-First".
- **DSGVO-SELECT-Whitelist** in DAO-Queries — keine `SELECT m.*`; explizite Spalten. Phase 6 muss diese Linie halten, auch wenn das Risiko hier geringer ist (Export geht eh nur an Vorstand).
- **`ServiceError` → `RestError` → HTTP-Status** — Conflict (409) für falschen GV-Status, PermissionDenied (403) für Helfer-Versuche (sollte gar nicht erst durchkommen wenn UI/Permission-Funnel sauber sind).
- **`#[instrument(skip(rest_state))]`** auf REST-Handlern für Tracing.
- **Workspace-Dependency hinzufügen** — `rust_xlsxwriter` läuft analog zu `qrcode 0.14` / `rand 0.8` aus Phase 2 (`Cargo.toml` Top-Level + per-crate `[dependencies]`).
- **E2E-Test-Pattern** mit `start_test_server` + in-memory SQLite + `reqwest`-Client (siehe `genossi_bin/tests/e2e_tests.rs`); Tests für alle drei Formate + Permission + Status-Check müssen rein.

### Integration Points
- **REST-Router-Registration** in `genossi_rest/src/lib.rs` — neuer Sub-Router `attendance_export::generate_route()` unter `/api/assembly`, analog zu `assembly::generate_route()` und `attendance::generate_stats_route()`.
- **DI-Wiring** in `genossi_bin/src/lib.rs::RestStateImpl::new()` — neuer Service `AttendanceExportServiceImpl` mit Deps (`AssemblyDao`, `AttendanceDao` oder einer dedizierten Export-DAO, `AssemblyMemberSnapshotDao`, `PermissionService`).
- **Frontend-API-Layer** in `genossi-frontend/src/api.rs` — Blob-Download-Funktionen mit `web-sys` (FormData/Headers) oder direkter Anker-Tag-Approach mit Browser-Native-Download.
- **Frontend-Page** `genossi-frontend/src/page/assembly_details.rs` — Export-Block in einem geeigneten Tab (vermutlich Basics oder ein neuer „Export"-Tab); muss `assembly.status == Closed` als Sichtbarkeits-Gate haben.

</code_context>

<specifics>
## Specific Ideas

- **„Verbandskonform"** — Verband akzeptiert Excel-Listen ungern; Genossi-Export muss als gleichwertiger Ersatz funktionieren. Reihenfolge der Spalten und die saubere Anwesenheits-Markierung sollten dem entsprechen, was ein typisches Verbands-Excel zeigt: Nummer | Name | Vorname | (Anrede/Titel) | anwesend.
- **Realer GV-Einsatz hat geklappt** — die Hotfixes aus dem produktiven Einsatz (live-counter, button-types, sort by member_number, token-codes magic-link) zeigen, dass der Datenstand zuverlässig ist. Phase 6 baut auf bewährter Datenbasis auf.
- **Snapshot ist die Wahrheit** — Vorstand-Diskussion war eindeutig: das PDF, das beim Verband landet, soll exakt den GV-Moment widerspiegeln; nachträgliche Member-Mutationen dürfen das Protokoll nicht im Nachhinein „umschreiben".

</specifics>

<deferred>
## Deferred Ideas

- **Sammelexport mehrerer GVs** (z. B. Jahres-Liste mit allen GVs eines Jahres) — eigene Phase, nicht im Scope.
- **Automatischer Versand per E-Mail an Verband-Adresse** — könnte später als Komfort-Feature kommen; benötigt SMTP-Pfad-Erweiterung und Konfiguration des Verband-Empfängers.
- **Unterschriften-Spalte im PDF als Papier-Backup-Liste** — der User hat das in der Diskussion explizit nicht ausgewählt; falls später ein Backup-Drucken-Use-Case auftaucht (z. B. Tablet-Ausfall), könnte ein separater Endpoint `?layout=signature` ergänzt werden. NICHT in Phase 6.
- **Logo / Briefkopf** — würde eine Genossenschafts-Konfiguration (Logo-Upload, Adresse) im System voraussetzen, die heute noch nicht existiert. Eigene Phase.
- **XLSX mit Multi-Sheet** (z. B. Sheet 1 = Liste, Sheet 2 = Stats, Sheet 3 = Audit-Log-Export) — Over-Engineering für Verband.
- **Export-Audit** (wer hat wann exportiert) — bewusst verworfen (D-17). Falls Datenschutz-Auditoren das später fordern, könnte ein eigener `export_log`-Pfad nachgezogen werden.

### Reviewed Todos (not folded)
Keine — `gsd-sdk query todo.match-phase 6` lieferte 0 Matches.

</deferred>

---

*Phase: 6-teilnehmerlisten-export-für-generalversammlungen*
*Context gathered: 2026-05-17*
