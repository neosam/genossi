# Phase 6: Teilnehmerlisten-Export für Generalversammlungen - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 6-teilnehmerlisten-export-für-generalversammlungen
**Areas discussed:** Format, Inhalt, Lifecycle, Zugriff, PDF-Inhalt, XLSX-Crate, CSV-Format, Daten-Quelle, Sortierung, URL-Design, Filename, Audit

---

## Format

| Option | Description | Selected |
|--------|-------------|----------|
| PDF (an Protokoll heften) | Typst-Template → typst-pdf. Druckbar, signierbar, klassischer Protokoll-Anhang. Stack ist bereits da (siehe join_confirmation.typ). | ✓ |
| CSV (Verband / Weiterverarbeitung) | csv-Crate ist im Stack. Maschinen-lesbar für Excel/LibreOffice, falls Verband Tabellenkalkulation will. | ✓ |
| XLSX | Echtes Excel-Format. Brauchte zusätzliche Dependency (rust_xlsxwriter o. ä.) — nur falls Verband das explizit verlangt. | ✓ |

**User's choice:** Alle drei Formate parallel anbieten.
**Notes:** Verband akzeptiert Excel-Listen ungern; mehrere Formate erhöhen die Wahrscheinlichkeit einer reibungslosen Verbands-Akzeptanz.

---

## Inhalt

| Option | Description | Selected |
|--------|-------------|----------|
| Alle Snapshot-Mitglieder + Anwesenheits-Spalte (Recommended) | Vollständige Liste aus Member-Universe-Snapshot beim GV-Öffnen, plus Spalte 'anwesend (Ja/Nein)'. Verband sieht direkt: 'X von Y'. Auch als Unterschriften-Liste verwendbar. | |
| Nur Anwesende | Kompakter, aber Y im 'X von Y' fehlt im Anhang — Verband muss separat nachrechnen. | |
| Wählbar per Parameter (?include=all\|present) | Maximale Flexibilität, aber zwei Code-Pfade plus UI-Toggle — mehr Aufwand für einen Edge-Case. | ✓ |

**User's choice:** Wählbar per Parameter (`?include=all|present`).
**Notes:** Vorstand möchte beide Use-Cases (Vollliste fürs Protokoll, kompakte Liste als Anwesenheits-Beleg). Default-Wert offen für Planner.

---

## Lifecycle

| Option | Description | Selected |
|--------|-------------|----------|
| Nur 'Geschlossen' (Recommended) | Export = Snapshot des Endstands fürs Protokoll. Klar, eindeutig, keine 'Zwischenstand'-Verwechslung. Post-Close-Korrekturen (ASSY-06) wirken sich aus. | ✓ |
| Geöffnet + Geschlossen | Während der GV kann z. B. eine vorläufige Unterschriften-Liste gedruckt werden. Risiko: Vorstand verwendet versehentlich Zwischenstand fürs Protokoll. | |
| Alle Status (auch Vorbereitung) | In Vorbereitung gibt's noch keinen Snapshot — würde leere Liste liefern oder Live-Mitglieder zeigen. Eher verwirrend. | |

**User's choice:** Nur 'Geschlossen'.
**Notes:** Endbeleg-Logik — keine versehentlichen Zwischenstände.

---

## Zugriff

| Option | Description | Selected |
|--------|-------------|----------|
| REST + UI-Button im Vorstand-Detail-View, nur Vorstand (Recommended) | Endpoint(s) unter /api/assembly/{aid}/export/... + Download-Button in der bestehenden Assembly-Detail-Seite. Nur OIDC-Vorstand-Permission — Helfer sehen Mitgliedsnummer/Name eh nur für laufende GV. | ✓ |
| Nur REST-Endpoint | Kein UI, Vorstand muss Endpoint manuell aufrufen (curl/Browser-URL). Schnell zu bauen, schlechte UX. | |
| REST + UI, auch für Helfer | Helfer könnte Anwesenheits-Liste runterladen. Datenschutz-Bedenken — Helfer-Daten-Exposition war in Phase 2/3 bewusst minimiert (nur live-Liste, kein Persistenz-Zugriff). | |

**User's choice:** REST + UI-Button im Vorstand-Detail-View, nur Vorstand.
**Notes:** Datenschutz-Linie aus Phase 2/3 wird konsistent fortgeführt.

---

## PDF-Inhalt

| Option | Description | Selected |
|--------|-------------|----------|
| Kopf: GV-Titel + Datum + 'X von Y anwesend' (Recommended) | Minimal-Anhang fürs Protokoll. Zähler kommt aus den Snapshot-/Attendance-Daten. Default in jeder Export-Variante. | ✓ |
| Unterschriften-Spalte je Mitglied | Druckbarer Fallback für Papier-Listen (z. B. wenn Tablet ausfällt). Macht das PDF auch im 'include=all'-Modus nützlich, nicht nur als Endbeleg. | |
| Fußzeile: Erzeugt-am + Export-Benutzer + Genossi-Versionsnummer | Macht den Export für Audits nachvollziehbar („Dieser Export wurde am … von … erstellt"). Klein, aber für Verband angenehm. | |
| Genossenschafts-Logo / Briefkopf | Wie bei join_confirmation.typ. Setzt voraus, dass irgendwo eine Genossenschafts-Konfig liegt (gibt's heute schon im Template-Layer?). | |

**User's choice:** Nur der minimale Kopf-Block.
**Notes:** Kein Logo / kein Backup-Drucken / kein Export-User-Audit-Footer — Minimal-Anhang.

---

## XLSX-Crate

| Option | Description | Selected |
|--------|-------------|----------|
| rust_xlsxwriter (Recommended) | Pure-Rust, aktiv gepflegt, gute API. Eine zusätzliche Dependency, aber keine native-Library-Bindings. | ✓ |
| calamine (bereits im Stack) + custom writer | calamine ist allerdings primär ein Reader, kein Writer. Nicht passend. | |
| Du entscheidest beim Research | Researcher prüft 2026er Optionen (rust_xlsxwriter vs. xlsxwriter (FFI) vs. simple-xlsx) und wählt die zur Projekt-Linie 'pure-Rust, minimale FFI' passende. | |

**User's choice:** rust_xlsxwriter.
**Notes:** Pure-Rust-Linie wird gehalten.

---

## CSV-Format

| Option | Description | Selected |
|--------|-------------|----------|
| Semikolon-getrennt, UTF-8 mit BOM (Recommended) | Deutsche Excel-Installation öffnet das ohne 'Daten → Aus Text'-Wizard. Standard für DACH-Office-Welten. | ✓ |
| Komma-getrennt, UTF-8 ohne BOM | RFC-4180-konform, aber deutsches Excel zeigt 'a,b,c' als eine Zelle — schlechte UX. | |
| TSV (Tab-getrennt) | Maximale Kompatibilität, aber unüblicher als CSV; vermeidet Trennzeichen-Konflikte. | |

**User's choice:** Semikolon-getrennt, UTF-8 mit BOM.
**Notes:** DACH-Excel-Kompatibilität priorisiert über RFC-Konformität.

---

## Daten-Quelle

| Option | Description | Selected |
|--------|-------------|----------|
| Snapshot-Daten aus assembly_member_snapshot (Recommended) | Liste spiegelt den GV-Moment wider. Wenn ein Mitglied nach der GV umbenannt/austritt, bleibt der Eintrag wie er war — historisch stabil fürs Protokoll. Konsistent mit ASSY-02-Entscheidung. | ✓ |
| Live-Daten aus members + JOIN auf Snapshot-IDs | Aktueller Name/Mitgliedsnummer im Export. Risiko: GV-Protokoll und neu erzeugter Export stimmen nicht mehr überein (z. B. nach Heirat-Namensänderung). | |
| Snapshot mit Live-Stand als Hinweis-Spalte | Beide — Snapshot ist Wahrheit, Live als 'aktuell heute' Spalte. Vermutlich Over-Engineering für den Verbandsfall. | |

**User's choice:** Snapshot-Daten aus `assembly_member_snapshot`.
**Notes:** Historisch stabiler Endbeleg — Protokoll und nachträglich erzeugter Export stimmen immer überein.

---

## Sortierung

| Option | Description | Selected |
|--------|-------------|----------|
| member_number ASC (Recommended) | Konsistent mit der Live-Liste seit commit ed754fc. Vorstand findet Mitglieder in derselben Reihenfolge im Export wie auf dem Tablet. | ✓ |
| Nachname, Vorname (alphabetisch) | Für Verband u. U. handlicher zum Nachschlagen, aber inkonsistent mit der UI — Vorstand bekommt zwei verschiedene Reihenfolgen. | |
| Anwesende zuerst, dann Abwesende | Sinnvoll bei 'include=all', wenn der/die Anwesenden schnell sichtbar sein sollen. Aber: bei 'include=present' irrelevant. | |

**User's choice:** member_number ASC.
**Notes:** UI-Konsistenz mit der Tablet-Liste wichtig.

---

## URL-Design

| Option | Description | Selected |
|--------|-------------|----------|
| Ein Endpoint, Format via Pfad-Suffix (Recommended) | GET /api/assembly/{aid}/attendance-export/{format} mit format ∈ {csv,pdf,xlsx}. ?include=all\|present als Query. Klar in OpenAPI/Swagger, intuitiv im Browser/curl. | ✓ |
| Drei separate Endpoints | /attendance-export.csv, /attendance-export.pdf, /attendance-export.xlsx. Klar getrennt, aber drei Handler-Signaturen mit viel Boilerplate. | |
| Ein Endpoint, Format via Accept-Header | REST-purist (content negotiation), aber im Browser/Download-Link schwer auslösbar und für Vorstand-Debugging unüblich. | |

**User's choice:** Ein Endpoint mit Pfad-Suffix.
**Notes:** Pragmatik vor Purismus — Browser-/curl-Debugging soll einfach bleiben.

---

## Filename

| Option | Description | Selected |
|--------|-------------|----------|
| gv-{YYYY-MM-DD}-teilnehmer.{ext} (Recommended) | Beispiel: gv-2026-05-15-teilnehmer.pdf. Datum aus assembly.date; eindeutig pro GV, sortierbar im Dateimanager, kein Sonderzeichen-Risiko. | ✓ |
| {assembly.title-slug}-teilnehmer.{ext} | Beispiel: jahres-gv-2026-teilnehmer.pdf. Lesbarer Titel, aber Slugify-Logik nötig (Umlaute, Spaces) und Risiko von Duplikat-Filenamen bei ähnlich benannten GVs. | |
| {date}_{title-slug}_{include}.{ext} | Maximal explizit: 2026-05-15_jahres-gv_anwesende.csv. Längerer Filename, dafür ist 'include'-Variante im Filename sichtbar. | |

**User's choice:** `gv-{YYYY-MM-DD}-teilnehmer.{ext}`.
**Notes:** Keine Slugify-Komplexität, sortierbar nach Datum.

---

## Audit

| Option | Description | Selected |
|--------|-------------|----------|
| Nein — Read-Only auf bereits-vorhandene Daten (Recommended) | Konsistent mit ATTN-05 (Anwesenheit nicht auditiert). Read-Zugriffe werden in Genossi generell nicht auditiert (nur create/update/delete). Verband fordert nur Zahlen, nicht Export-Log. | ✓ |
| Ja — audited_log-Entry pro Export | Wer-wann-welche-GV-welches-Format. Nützlich für Vorstand-interne Nachvollziehbarkeit, aber neuer Entity-Typ in der Hash-Chain nötig. | |
| Nur tracing::info!-Log, kein Audit-Hashchain-Eintrag | Operative Sichtbarkeit in Server-Logs ohne Audit-Schwergewicht. Pragmatischer Mittelweg. | |

**User's choice:** Kein Audit.
**Notes:** Read-Only-Linie wird konsistent fortgeführt; tracing::info! im Backend bleibt Standard.

---

## Claude's Discretion

- Default-Wert von `?include` (Empfehlung Planner: `all`).
- Darstellung der Anwesenheits-Spalte pro Format (`"ja"`/`"nein"` vs. `✓`/leer).
- Ob `member_id` (UUID) im Export-File erscheint (Empfehlung: weglassen).
- Spaltenbreiten/Layout-Details im Typst-Template.
- Ob UI-Knopf als Dropdown oder drei separate Buttons erscheint.
- Ob Backend in neuer Crate `genossi_export` oder im bestehenden `genossi_service_impl` lebt.

## Deferred Ideas

- Sammelexport mehrerer GVs (Jahresliste) — eigene Phase.
- Automatischer Versand per E-Mail an Verband — Komfort-Feature für später.
- Unterschriften-Spalte im PDF als Papier-Backup — separate Phase, falls Use-Case auftaucht.
- Logo / Briefkopf — setzt Genossenschafts-Konfig voraus, die heute nicht existiert.
- XLSX Multi-Sheet — Over-Engineering.
- Export-Audit — bewusst verworfen, könnte nachgezogen werden falls Datenschutz-Auditoren das fordern.
