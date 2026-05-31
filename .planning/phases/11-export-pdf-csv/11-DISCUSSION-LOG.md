# Phase 11: Export (PDF) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 11-export-pdf-csv (note: directory slug stays; phase name changes to "Export (PDF)" per D-12)
**Areas discussed:** include-Filter-Semantik, Verwendungszweck-Text im PDF, IBAN-NULL-Edge-Case, Aggregation pro Mitglied vs. Entry, CSV-Scope

---

## include-Filter-Semantik

### Frage 1: Was bedeutet `?include=open`?

| Option | Description | Selected |
|--------|-------------|----------|
| **Open ∪ Contacted** | **Banking-Vorlage = alles noch nicht ausbezahlt; konsistent mit Phase 10 D-04 Mail-Worker-Aggregation** | ✓ |
| Strikt nur Status=Open | Vorstand will sehen "Wen muss ich noch anschreiben"; Contacted hat eigene Bedeutung | |
| Beide Varianten separat exposen | `pending` (=Open) und `outstanding` (=Open+Contacted); aber ROADMAP-Pfad lockt `open\|all\|paid` | |

**User's choice:** Open ∪ Contacted (Recommended).

### Frage 2: Soft-Deletes

| Option | Description | Selected |
|--------|-------------|----------|
| **Soft-Deleted komplett ausschließen** | **`entry.deleted IS NULL AND member.deleted IS NULL` in jedem Filter; konsistent mit Phase 8 D-09 etc.** | ✓ |
| ?include=all liefert auch soft-deleted | Für Buchhaltungs-Audit, aber Bruch mit Konvention | |
| Separates ?include=deleted-Flag | Roadmap-Scope-Erweiterung | |

**User's choice:** Soft-Deleted komplett ausschließen (Recommended).

**Notes:** `all` = Open ∪ Contacted ∪ PaidOut (kein deleted). `paid` = nur PaidOut (kein deleted). Default-Parameter ist `open` (ROADMAP SC #2). Captured as D-01 (filter semantics), D-02 (all/paid/soft-delete-exclusion), D-03 (default+derive).

---

## Verwendungszweck-Text im PDF

| Option | Description | Selected |
|--------|-------------|----------|
| Schema mit #{member_number} | `Anteilsrückzahlung GJ 2026 #1234` (~35 Zeichen) | |
| Generisch ohne Mitgliedsnummer | `Anteilsrückzahlung GJ 2026` (gleich für alle) | |
| Pro RepaymentPhase konfigurierbar | Neue DB-Spalte + Edit-Matrix + Frontend; Scope-Erweiterung | |
| Spalte leer / nur Buchhaltungs-Hinweis | Reduziert PDF-Nutzen drastisch | |
| **Free-text: mit member_number + first_name + last_name** | **`Anteilsrückzahlung GJ {fiscal_year} {member_number} {first_name} {last_name}`** | ✓ |

**User's choice (Free-text/Custom-Answer):** `Anteilsrückzahlung GJ {fiscal_year} ${member_number} ${first_name} ${last_name}` — Beispiel: `"Anteilsrückzahlung GJ 2026 1234 Max Mustermann"` (ca. 47 Zeichen, weit unter SEPA-140-Zeichen-Limit auch bei langen Namen).

**Notes:** Templating-Syntax-Mix (`{...}` vs. `${...}`) im User-Wortlaut wurde in CONTEXT-D-04 zu einheitlich `{name}` konsolidiert (Konvention im Service-Pre-Computing). Keine SEPA-Sonderzeichen-Sanitization (Banking-Software ersetzt ä/ö/ü/ß selbst). Captured as D-04 + D-05.

---

## IBAN-NULL-Edge-Case

| Option | Description | Selected |
|--------|-------------|----------|
| **Zeile mit leerer IBAN, alle anderen Spalten gefüllt** | **Export blockiert nie; Vorstand sieht Lücke und pflegt nach** | ✓ |
| Zeile + Visual-Warnzeile/-Highlight im PDF | Wie oben + rot/fett-Marker; PDF-Template komplexer | |
| Eintrag still überspringen | Risiko: Vorstand merkt fehlende Auszahlung nicht | |
| 409 Conflict — Export bricht ab | Idiotensicher, aber hohe Reibung | |

**User's choice:** Zeile mit leerer IBAN (Recommended).

**Notes:** Captured as D-06 + D-07. Visual-Marker als Phase-12-Frontend-Option in `<deferred>`.

---

## Aggregation pro Mitglied vs. pro Entry

| Option | Description | Selected |
|--------|-------------|----------|
| **Eine Zeile pro Entry** | **1:1-Mapping; zwei Entries pro Mitglied = zwei PDF-Zeilen; konsistent mit Phase 8 D-04** | ✓ |
| Pro Mitglied aggregiert (SUM × share_value) | Banking-PDF kompakter; verliert Audit-Detail | |
| PDF aggregiert, CSV pro Entry | Zwei Sichten; doppelter Render-Aufwand | |
| Mehrfach-Entries kommen in der Praxis nicht vor | Niedrige Aufmerksamkeit; Default eine Zeile pro Entry | |

**User's choice:** Eine Zeile pro Entry (Recommended).

**Notes:** Sub-Sort bei Mehrfach-Entries: `entry.created ASC` als Default-Konvention (Planner-Discretion). Captured as D-08 + D-09.

---

## CSV-Scope-Klärung (Folge-Frage nach „Lass CSV erst mal weg")

| Option | Description | Selected |
|--------|-------------|----------|
| **CSV komplett aus Phase 11 streichen** | **EXPO-04 nach v1.2; ROADMAP-Phase umbenannt; Format-Whitelist nur `pdf`** | ✓ |
| CSV bleibt, Spalten-Set Planner-Discretion | Default: exakt PDF-Spalten | |
| CSV bleibt + Recherche nach DATEV/Standard-Format | Researcher prüft offizielle Buchhaltungs-Specs | |

**User's choice:** CSV komplett aus Phase 11 streichen.

**Notes:** Scope-relevante Entscheidung. Captured as D-12 inkl. Konsequenz-Liste (REQUIREMENTS.md, ROADMAP.md, SC#3/#5-Anpassung, Format-Whitelist). Re-Add in v1.2 ist additiv, bricht nichts.

---

## Areas considered but NOT discussed (already locked by SC / Phase 6)

| Area | Why not discussed |
|------|-------------------|
| Phase-Status-Gate | EXPO-01 lockt explizit "Offen und Abgeschlossen"; `Vorbereitung` → 409 (D-10 reflektiert das) |
| Permission (Vorstand-only / 403) | EXPO-05 + Phase 6 D-13 — Pattern 1:1 übernehmen (D-11) |
| Audit (0 Aufrufe) | EXPO-05 + Phase 6 D-17 — Grep-Gate-Test im E2E (D-11) |
| Filename-Schema | ROADMAP SC #2: `auszahlung-{fiscal_year}-{include}.{ext}` |
| REST-Pfad | ROADMAP SC #2 + Phase 7 D-14 Singular-Konvention |
| Format-Path-Whitelist-Pattern | Phase 6 D-14 — exakt übernehmen |
| Reuse PdfGenerator / DEFAULT_TEMPLATES / _layout.typ | Phase 6 Code-Patterns — keine alternative Diskussion notwendig |
| Euro-Format Lokalisierung | Phase 10 D-04 — `"60,00"` deutsche Lokalisierung übernommen |
| XLSX | Nie im v1.1-Scope per ROADMAP; in `<deferred>` vermerkt |

---

*Discussion completed: 2026-05-31*
*Final decisions: see CONTEXT.md (D-01..D-12 + Claude's Discretion items)*
