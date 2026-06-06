# Phase 18: frontend-component-first - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 18-frontend-component-first
**Areas discussed:** Sub-Choice-Form, Vorschau-Flow, Datepicker, MemberSearch für Übertrag

---

## Sub-Choice-Form

### Q1: Welche Sub-Choice-Form für den MembershipAdjustModal?

| Option | Description | Selected |
|--------|-------------|----------|
| 4 flat Buttons | Kündigung / Teil-Rückgabe / Übertrag / Aufstocken nebeneinander. Klar, kein Nesting, maximale Discoverability. | ✓ |
| 3 mit Nesting | Reduzieren → {Genossenschaft / Mitglied} / Aufstocken / Kündigung. Reduzieren-Konzept geclustert, aber tieferes Menü. | |
| Kündigung-Quickpath | Großer "Kündigen"-Button + "Andere Anpassung..." Untermenü. 80%-Case ein Klick, aber asymmetrisch. | |

**User's choice:** 4 flat Buttons
**Notes:** Roadmap explizit als Discuss-Item markiert; FEATURES.md-Pro-Tabelle bestätigt; Vorstand-User-Base trainiert.

### Q2: Modal-Flow (Sub-Choice → Operation-Sub-View)?

| Option | Description | Selected |
|--------|-------------|----------|
| Single Modal mit Back-Button | Eine Modal-Instance, Step-State, Header zeigt "← Mitgliedschaft anpassen · Kündigung". Vorstand kann Operation wechseln ohne Modal zu schließen. | ✓ |
| Modal-Sequenz (Sub-Choice schließt, neue Modal öffnet) | Modal-Flackern, neu öffnen um zu wechseln. | |
| Inline-Sub-View ohne Back-Button | Form ersetzt direkt Sub-Choice-Buttons, einfacher Code aber kein Operation-Switch. | |

**User's choice:** Single Modal mit Back-Button (Step-State)

### Q3: Modal-State-Architecture für die 4 Sub-Views?

| Option | Description | Selected |
|--------|-------------|----------|
| Enum-State + match-rsx im selben File | Ein File ~400-600 LOC, alles an einem Ort, einfacher State-Sharing. | ✓ |
| Separate Component-Files pro Operation | 4 separate Components, isoliert testbar, mehr Files. | |
| Hybrid: shared Sub-Choice + 4 Sub-Form-Components | Step-State zentral, Form-Body je Component. | |

**User's choice:** Enum-State + match-rsx im selben File

### Q4: Modal-Mount-Pattern auf Member-Detail-Page?

| Option | Description | Selected |
|--------|-------------|----------|
| Toggle-Signal auf Page (use_signal<bool>) | Konsistent mit existing Pattern (RepaymentEntryPaidOutConfirm). | ✓ |
| Always-mounted + Visibility-State | Modal immer im DOM, weicht von genossi-Pattern ab. | |
| Global-State via use_context | App-Level-Context, Overkill für v1.2 (nur Member-Detail-Page). | |

**User's choice:** Toggle-Signal auf MemberDetails-Page

---

## Vorschau-Flow

### Q1: Wie soll die Bestätigung vor dem finalen Commit aussehen?

| Option | Description | Selected |
|--------|-------------|----------|
| Einstufig mit Live-Vorschau im Form-Footer | Form-Felder oben, Live-Vorschau-Box unten, ein Submit-Button. Vorstand sieht Vorschau permanent. | ✓ |
| Zweistufig: Form → Confirm-Dialog | Stufe 1 Form, Stufe 2 Vorschau-Dialog mit "Endgültig"-Button. Maximale Bewusstheit, aber 2 Klicks. | |
| Hybrid: Live-Vorschau + Confirm-Checkbox | Form mit Vorschau-Footer + "Ich verstehe..."-Checkbox vor Submit. | |

**User's choice:** Einstufig mit Live-Vorschau im Form-Footer

### Q2: Vorschau-Inhalt — welche Felder pro Operation?

| Option | Description | Selected |
|--------|-------------|----------|
| Operations-spezifisch (Roadmap-SC-3 Format) | Minimal, fokussiert auf was sich ändert; matched Roadmap-SC-3 wortgenau. | ✓ |
| Diff-Tabelle (Vorher \| Nachher) | Generisch, aber für Übertrag mit 2 Members klobig. | |
| Card-Layout pro Member | Visuell schön, mehr UI-Aufwand. | |

**User's choice:** Operations-spezifisch (Roadmap-SC-3 Format)

### Q3: Voll-Übertrag-Warnung (shares == from.current_shares)?

| Option | Description | Selected |
|--------|-------------|----------|
| Live-Detection im Form | "⚠ Voll-Übertrag: Member A tritt am DD.MM.YYYY aus" in Vorschau-Footer. | ✓ |
| Warnung nach Submit aus Response | Backend-Response zeigt Austritt; Frontend zeigt Erfolgs-Toast. | |
| Beides: Live-Warnung + Post-Submit-Bestätigung | Doppelte Sicherheit, redundant. | |

**User's choice:** Live-Detection im Form

### Q4: After-Success-Behavior?

| Option | Description | Selected |
|--------|-------------|----------|
| Modal schließt + refresh_members() + grüner Toast | Konsistent mit existing v1.1-Pattern. | ✓ |
| Modal zeigt Erfolgs-Screen + "Schließen"-Button | Explizite Bestätigung, extra Klick. | |
| Redirect zur MemberAction-/Audit-Log-Liste | Workflow-Bruch. | |

**User's choice:** Modal schließt + Member-Refresh + Toast

---

## Datepicker

### Q1: Datepicker-Implementierung?

| Option | Description | Selected |
|--------|-------------|----------|
| Native HTML <input type="date"> | Browser-native, Mobile-friendly, codebase nutzt das Pattern bereits. | ✓ |
| Custom DatePicker mit Jahres-Auswahl | Explizite GJ-Auswahl, intuitiver für H1/H2-Denken. | |
| Native + Helper-Text (H1/H2-Erklärung) | Inline-Erklärung der Regel. | |

**User's choice:** Native HTML <input type="date">

### Q2: Datepicker-Wiederverwendung?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline im MembershipAdjustModal | Minimaler Code, kein Component-Overhead. | |
| Neue FiscalYearDateInput-Component | Testbar, Wiederverwendung in v1.3+. | ✓ |
| Inline jetzt, extract später bei 2. Nutzung | YAGNI. | |

**User's choice:** Neue `FiscalYearDateInput`-Component
**Notes:** Wird in 4 Sub-Views verwendet — also sofort 4x Wiederverwendung.

### Q3: FiscalYearDateInput-Validierung?

| Option | Description | Selected |
|--------|-------------|----------|
| Browser min/max + Component Border-rot bei Out-of-Range | UX-Feedback ohne Roundtrip; Backend ist Defense-in-Depth. | ✓ |
| Nur Backend-Validation, Frontend zeigt Server-Fehler | DRY, aber Vorstand merkt erst beim Submit. | |
| Frontend macht harten Block (Submit disabled) | Maximales UX, Frontend duplicate der Bounds-Logik. | |

**User's choice:** Browser min/max + Border-rot + Submit-Button-Disabled

---

## MemberSearch für Übertrag

### Q1: MemberSearch-Daten-Source und Reuse-Pattern?

| Option | Description | Selected |
|--------|-------------|----------|
| Neuer transfer-recipients-Endpoint + Existing MemberSearch UI | Server-side exit_date-Filter (DSGVO-konform via MemberSlimTO), MemberSearch unverändert. | ✓ |
| Existing MemberSearch + lokaler exclude_id-Filter | Kein extra API-Call, aber MEMBERS hat alle PII-Felder. | |
| Hybrid: MemberSearch mit Endpoint-Daten als optionale Prop | Flexibel, aber Breaking Change an MemberSearch-API. | |

**User's choice:** Neuer transfer-recipients-Endpoint + Existing MemberSearch UI

### Q2: MemberSlimTO-Frontend-Integration?

| Option | Description | Selected |
|--------|-------------|----------|
| MemberSearch akzeptiert beides via Adapter | MemberSlim → MemberTO Mapping (PII = None) vor MemberSearch. | ✓ |
| MemberSearch generisch über Trait | Sauber typisiert, Refactor des existing MemberSearch. | |
| Separate TransferRecipientSearch-Component | Klare Trennung, Code-Dup mit MemberSearch. | |

**User's choice:** Adapter-Pattern (MemberSlimTO → MemberTO Mapping)

### Q3: Transfer-Recipients-Loading-Strategie?

| Option | Description | Selected |
|--------|-------------|----------|
| On-Sub-View-Mount via use_resource | Frisch geladen, Loading-Spinner während Fetch. | ✓ |
| Pre-fetch beim Modal-Öffnen | Kein Latency-Spinner, aber unnötig wenn Vorstand andere Operation wählt. | |
| Globaler Cache mit TTL | Cache hit, zusätzliche State-Komplexität für seltene Operation. | |

**User's choice:** On-Sub-View-Mount via `use_resource`

---

## Claude's Discretion

Areas wo Planner-Discretion bewusst überlassen wurde (siehe CONTEXT.md → `<decisions>` § Claude's Discretion):

- Sub-Choice-Button-Layout (2×2 vs. 1×4) — basierend auf Modal-Breite
- State-Reset beim Back-Pfeil (shared Felder behalten oder reset)
- Vorschau-Phase-Calculation: Frontend-Pure-Mirror vs. Backend-Preview-Endpoint vs. Vorschau ohne Stichtag-Anzeige (Empfehlung: Pure-Mirror)
- Auto-Anlegen-Phase-Hinweis-Format (Pre-Submit-Mirror vs. Post-Submit-Response-Read)
- Toast-Component-Wahl (existing toast.rs API verifizieren)
- i18n-Key-Naming-Convention (hierarchisch `MembershipAdjust{Sub}{Element}`, mind. 20 Keys)
- Pure-Frontend-Helpers (`compute_effective_date_mirror`, `to_member_to`, `format_date_german`, `is_voll_uebertrag`) — alle mit Unit-Tests
- Loading-Spinner-Component-Wahl (existing oder neu)
- Error-Variant-Display für "Empfänger-cancelled" (409 von Phase 17) in ErrorAlert mit i18n
- Modal-Höhe/Breite (Tailwind aus existing modal.rs)
- Test-Strategie (Unit-Tests für Pure-Helper + ManualUAT-Sektion)

---

## Deferred Ideas

Während der Discussion erwähnte/erwogene Ideen die für spätere Phasen verschoben wurden:

- Mehrstufiger Workflow (Antrag → Genehmigung → Wirksamkeit) — Vier-Augen-Prinzip ist v1.3+
- Bulk-Operationen — Out-of-Scope per FEATURES.md
- Storno-Knopf im Modal — bleibt manuelle MemberAction-UI
- Mitgliederliste-Integration — Roadmap explizit ausschließt
- Anteilswert-Editierung im Auto-Anlegen-Phase-Branch — Existing v1.1 RepaymentPhase-UI
- Globaler Cache + Pre-Fetch für transfer-recipients
- Backend-Preview-Endpoint für effective_date — Frontend macht Pure-Mirror
- Keyboard-Shortcuts für Sub-Choice
- Browser-Automation-Tests (Playwright/Selenium)
- Generischer MemberSearch-Trait-Refactor — Adapter-Pattern reicht
- MembershipAdjustModal als Top-Bar-Quick-Action in v1.3+

---

*Discussion log archived: 2026-06-06*
