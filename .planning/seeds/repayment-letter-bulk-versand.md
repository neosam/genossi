---
title: RepaymentLetter — Bulk-Anschreiben-PDF für Nicht-Email-Mitglieder
trigger_condition: Vor der nächsten Auszahlungsphase, in der Mitglieder ohne Email-Adresse angeschrieben werden müssen (spätestens bei der zweiten produktiven Anwendung des v1.1-Workflows)
planted_date: 2026-06-01
---

# RepaymentLetter — Bulk-Anschreiben-PDF für Nicht-Email-Mitglieder

## Kurzbeschreibung

Ergänzt v1.1 (Anteile-Rückzahlungsphase) um einen **Brief-Kanal** für die
Mitglieder, die per Mail nicht erreichbar sind. Vorstand wählt auf der
`RepaymentPhase`-Detail-Page (Phase 12) die betreffenden Entries
multi-select an, klickt "Anschreiben erzeugen", erhält ein gebündeltes PDF
zum Drucken und sieht bei jedem Mitglied einen auditierten
`MemberDocument`-Eintrag mit dem PDF-File.

Komplettiert damit die Phase-10-Mail-Pipeline: Mail-Kanal **und** Brief-
Kanal nutzen denselben aggregierten Repayment-Kontext (payout_amount,
share_count, fiscal_year), aber unterschiedliche Render-Pfade.

## Scope (grob)

**Backend:**
- Neuer Service-Trait `RepaymentLetterService` in `genossi_service/`
- Impl `RepaymentLetterServiceImpl` in `genossi_service_impl/`:
  - Permission-Funnel: Vorstand-only via `check_permission("admin", ...)`
  - Status-Gate: Phase MUSS `Offen` ODER `Abgeschlossen` sein
- Neuer Service-Helper `RepaymentContextResolver` (Free-Function oder Trait) —
  zentralisiert die Aggregations-Logik aus Phase-10 D-04
- `PdfGenerator::render_repayment_letter(...)` analog `render_attendance_list`
- Neuer `DocumentType::RepaymentLetter`-Variante + `is_singleton = false`
- `audited_create!` pro Brief → `MemberDocument` mit echtem PDF-File-Pfad
- Neuer REST-Endpoint (z.B. `POST /api/repayment-phase/{phase_id}/letters/generate`)
- DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()`
- 5-6 E2E-Tests: Happy Path, Permission-Denied, Phase-Status-Gate, fehlende
  IBAN (leere Spalte analog Phase 11 D-06), Audit-Chain-Verify

**Frontend:**
- Auf `RepaymentPhase`-Detail-Page (Phase 12): zweiter Bulk-Button neben
  "Mail senden" → "Anschreiben erzeugen"
- Download-Trigger + Toast "N Briefe erzeugt"
- Component-First: existing Multi-Select-Pattern wiederverwenden

**Templates:**
- Neues `templates/defaults/auszahlungs_anschreiben.typ` (oder ähnlicher Slug)
- Layout-Vorbild: `templates/zahlungsanfrage.typ` (letter-pro mit
  Falzmarken, Logo, Vorstands-Footer)
- Registrierung in `template_storage.rs::DEFAULT_TEMPLATES`

**Refactor (separat, siehe Todo):**
- Phase-10-Mail-Worker auf den neuen `RepaymentContextResolver` migrieren
  → siehe [[phase-10-worker-refactor-resolver]]

## Architektur-Entscheidungen

Siehe [[repayment-letter-architecture]] für die 5 getroffenen Decisions
(Engine, Template-Pflege, Trigger, Persistenz, Aggregation).

## Offene Punkte (Plan-Klärung)

- Bundle-Format Single-PDF vs. ZIP — [[questions]]
- Standard-Wortlaut des Brief-Bodys
- Re-Generierung idempotent?
- Status-Toggle-Cascade (Open → Contacted)?

## Aufwandsschätzung (sehr grob)

Vermutlich **1 Phase mit 5-7 Plans** (analog Phase 11 Skalierung), weil viele
Patterns aus Phase 6 + 10 + 11 wiederverwendet werden können:
- 1 Plan: Migration + DocumentType + Resolver-Helper
- 1 Plan: Service-Trait + Impl + PDF-Render
- 1 Plan: REST-Handler + OpenAPI + Bundle-Format-Entscheidung
- 1 Plan: Template + DEFAULT_TEMPLATES-Registrierung
- 1 Plan: DI-Wiring + E2E-Tests
- 1 Plan: Frontend-Bulk-Button + Download
- (optional) 1 Plan: Worker-Refactor auf Resolver — kann auch separater Quick-Task

## Routing

Wenn aktiviert: Neue Phase in v1.2-Milestone (oder als Phase 13 angehängt
an v1.1 falls Milestone noch nicht geclosed). `/gsd-add-phase` aufrufen
und dann `/gsd-discuss-phase` → `/gsd-plan-phase`.
