# Phase 7: RepaymentPhase Backend (Foundation) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-29
**Phase:** 7-repaymentphase-backend-foundation
**Areas discussed:** Status-Naming & API-Lifecycle-Form, State-Machine-Editier-Regeln, Uniqueness & Soft-Delete, Detailfelder & Validierung

---

## Status-Naming & API-Lifecycle-Form

### Sub-Frage 1: Status-Enum-Sprache

| Option | Description | Selected |
|--------|-------------|----------|
| Englisch (Preparation/Open/Closed) wie Assembly | Konsistent mit AssemblyStatus, MemberStatus, ApplicationStatus. Frontend übersetzt via i18n. DB-Werte sind technische Identifier. | ✓ (nach Nachhaken) |
| Deutsch (Vorbereitung/Offen/Abgeschlossen) | REQUIREMENTS.md beschreibt Lifecycle auf Deutsch. Spart Frontend-Übersetzungsschicht. Bricht Genossi-Konvention. | |
| EN-Enum + DE-Display-Field | Backend speichert EN, liefert zusätzlich `status_display` auf DE. Mehr Code für unklaren Nutzen. | (initial) |

**User's choice:** Englisch — nach Nachhaken zurückgezogen vom Display-Field. Begründung der Klärung: kein Use-Case (Frontend i18n macht die Anzeige; Mail/PDF werden in Phase 10/11 separat gelöst, nicht jetzt).
**Notes:** Pattern-Konsistenz mit Assembly war ausschlaggebend.

### Sub-Frage 2: Lifecycle-API-Form

| Option | Description | Selected |
|--------|-------------|----------|
| Dedizierte Endpoints (POST /open, POST /close) wie Assembly | Pattern aus genossi_rest/src/assembly.rs:359-360. Klare Semantik, Permission pro Aktion, idempotent. ROADMAP-Beispiel deckt sich. | ✓ |
| Status im PUT-Body — ein Update-Endpoint | Einfacher, weniger Endpoints. Aber: vermischt Daten-Update mit Lifecycle-Transition. Phase 8/9 müsste aufbrechen. | |
| PATCH-Endpoint mit Status + separate Open/Close | Hybrid. Eher verwirrend. | |

**User's choice:** Dedizierte Endpoints
**Notes:** Bestätigt durch direkten Grep auf `genossi_rest/src/assembly.rs:349-361`. Lifecycle-Endpoints akzeptieren KEIN Body und prüfen KEIN `version`-Feld (Status-Check IS die Defense).

---

## State-Machine-Editier-Regeln

### Sub-Frage 1: Edit-Matrix pro Status

| Option | Description | Selected |
|--------|-------------|----------|
| Streng nach Roadmap | Preparation: alles. Open: nur share_value. Closed: nichts. Entspricht PHAS-04 + SC #5. | ✓ |
| Streng + share_value lock-bar in Vorbereitung | Identisch zur Empfehlung, explizit ausgesprochen. | |
| Auch in Closed share_value korrigierbar | Würde Audit-Drift gegen Phase-9-MemberAction-Snapshot erzeugen. | |

**User's choice:** Streng nach Roadmap
**Notes:** Preview-Tabelle bestätigt: `Preparation EDIT/EDIT/→Open`, `Open LOCKED/EDIT/→Closed`, `Closed LOCKED/LOCKED/final`.

### Sub-Frage 2: HTTP-Status bei Verstoß

| Option | Description | Selected |
|--------|-------------|----------|
| 409 Conflict via ServiceError::Conflict | Pattern aus Phase 6 D-11, Phase 3 close_assembly, PAYO-04. State-Machine-Verstoß ist klassisch 409. | ✓ |
| 400 BadRequest via ValidationError | Für Input-Fehler gedacht, nicht State-Konflikt. | |
| Mix: 400 für Feld, 409 für Transition | Präziser, aber zwei Cases für gleichen Sachverhalt. | |

**User's choice:** 409 Conflict
**Notes:** Konsistent mit existierender ServiceError→RestError-Mapping (genossi_rest/src/assembly.rs:233).

### Sub-Frage 3: Reverse-Transitionen

| Option | Description | Selected |
|--------|-------------|----------|
| Nein — nur Vorwärts | Assembly-Pattern. Vereinfacht State-Machine. Phase 8/9 würde Reverse fragil machen. | |
| Open→Preparation erlaubt | Erlaubt Auto-Befüllung-Reset, aber komplexe Side-Effects. | |
| You decide | Claude entscheidet die einfachste Variante. | ✓ |

**User's choice:** You decide → Claude entschied: Nur Vorwärts (entspricht Option 1)
**Notes:** Begründung: 4 konkrete Gründe in D-06 dokumentiert (Assembly-Pattern, Simplicity, Phase-8/9-Komplexität, Escape-Hatch via Soft-Delete + Neuanlage).

---

## Uniqueness & Soft-Delete

### Sub-Frage 1: Mehrere Phasen pro fiscal_year

| Option | Description | Selected |
|--------|-------------|----------|
| Komplett frei — keine Constraint | Realität: Q1+Q4-Phasen, historische+neue parallel. Vorstand verantwortet via UI. | ✓ |
| Max EINE Preparation/Open pro fiscal_year | Partial-Unique-Index. DB-side enforcement. | |
| Genau EINE pro fiscal_year | Zu restriktiv für Korrektur-/Nachzügler-Fälle. | |

**User's choice:** Komplett frei
**Notes:** Preview-Tabelle zeigte realistischen Fall mit drei parallelen Phasen für 2026.

### Sub-Frage 2: Soft-Delete erlaubt in welchem Status

| Option | Description | Selected |
|--------|-------------|----------|
| Nur Preparation | Audit-konsistent: Open/Closed haben bereits Audit-Einträge + ab Phase 8 RepaymentEntries. | ✓ |
| Preparation + leere Open | Komplexität über Phasen-Grenzen hinweg. | |
| Alle Status | Bricht Audit-Logik (Closed mit MemberAction-Einträgen). | |

**User's choice:** Nur Preparation
**Notes:** Schließt audit-konsistent ab. Escape-Hatch via Neuanlage.

### Sub-Frage 3: Listing soft-gelöschter Phasen

| Option | Description | Selected |
|--------|-------------|----------|
| Nein — default `deleted IS NULL` | Konsistent mit Member/Assembly-Listing. | ✓ |
| Optional `?include_deleted=true` | Keine bekannten Use-Cases. YAGNI. | |
| Filter via `?status=...` | Koppelt zwei Konzepte. | |

**User's choice:** Nein, default-filtered
**Notes:** YAGNI-Begründung war ausschlaggebend.

---

## Detailfelder & Validierung

### 4-fach-Batch (alle parallel beantwortet)

#### fiscal_year-Validierung

| Option | Description | Selected |
|--------|-------------|----------|
| Range 2000–2100, 4-stellig | Verhindert Tippfehler. Genossenschafts-Use-Case in dem Bereich. | ✓ |
| Nur positiv | Lässt 22026 durch. | |
| Keine Validierung | Risiko. | |

#### share_value-Validierung

| Option | Description | Selected |
|--------|-------------|----------|
| Strikt >0, max 1 Mio Euro | Sanity-Guard gegen Komma-Verschiebung. | |
| Nur positiv (>0), keine Obergrenze | User-Entscheidung gegen Empfehlung. | ✓ |
| Strikt > 0, keine weitere Constraint | Minimal. | |

**Notes:** User hat bewusst die Obergrenze rausgenommen. Documented als Claude-Discretion-Eintrag: Falls Planner DB-Schema-Limit braucht, ist `i64::MAX` Cent ok.

#### opened_at / closed_at Timestamps

| Option | Description | Selected |
|--------|-------------|----------|
| Ja, beide — Assembly-Pattern | Nützlich für Filename-Schema Phase 11 + Audit-Lesbarkeit. | ✓ |
| Nein — Audit-Hashchain reicht | Audit-Query für Filename-Generation umständlich. | |

#### REST-Pfad-Konvention

| Option | Description | Selected |
|--------|-------------|----------|
| Singular: /api/repayment-phase | Genossi-Konvention + ROADMAP-Beispiel. | ✓ |
| Plural: /api/repayment-phases | REST-Standard, aber bricht Pattern. | |

---

## Claude's Discretion

- **Reverse-Transitionen sind verboten** (D-06) — entschieden auf „You decide" des Users. 4 Begründungen dokumentiert.
- **PUT-Body-Schema** — Status NICHT akzeptiert; nur `fiscal_year`, `share_value`, `version` (D-05/D-07-Detail).
- **share_value-DB-Limit** — User wollte keine Obergrenze. `i64::MAX` Cent (≈92 Trillionen €) ist ok als Schema-Defense ohne User-Decision zu brechen.
- **OpenAPI-Beispielwerte** — Planner darf wählen (z.B. `fiscal_year: 2026`, `share_value: 12000` für 120,00 €).

## Deferred Ideas

- `?include_deleted=true` Listing-Variante — kein v1.1-Use-Case
- `opened_at`/`closed_at` als Audit-Diff statt eigene Spalten — entschieden für eigene Spalten
- `status_display` DE-Field — verworfen nach Nachhaken; Mail/PDF lösen das per Template-Helper
- DB-Partial-Unique-Index auf `(fiscal_year, status='Open')` — verworfen, nicht in v1.1
- Reverse-Transitionen — explizit out of scope; Soft-Delete + Neuanlage ist der Escape-Hatch
