# Phase 30: Application-Template-Kontext (Antragsteller-Vorlagen) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-12
**Phase:** 30-application-template-kontext-antragsteller-vorlagen
**Areas discussed:** Vorlagentyp-Trennung (D1), Kontext-Umfang & Platzhalter, Validierung (APTPL-04), format_eur_de + Seed-Content

**Format:** Alle Gray Areas als Fließtext-Batch präsentiert (Text-Liste, kein AskUserQuestion-Popup, auf Wunsch des Users). User-Antwort: 2a explizit geklärt, „Rest Default" (alle Empfehlungen übernommen).

---

## Vorlagentyp-Trennung (D1-Mechanismus)

| Option | Description | Selected |
|--------|-------------|----------|
| (A) Neue Spalte `template_type` auf mail_templates | Additive Migration, DEFAULT 'member', Werte member/application; ein Tabellen-/Entity-Set | ✓ |
| (B) Separate Tabelle `application_mail_templates` | Sauberste Trennung, dupliziert aber DAO/Service/REST/Frontend-CRUD | |
| (C) Bool-Flag `is_application` | Funktional wie A, schlechter erweiterbar für 3. Typ | |

**User's choice:** (A) — Default übernommen.
**Notes:** Frontend-Selector filtert Member-Pool auf `template_type='member'` (Default „Ja"). Erweiterbarkeit für APTPL-FUT-01 (mehrstufige Erinnerungen) war ausschlaggebend gegen den Bool-Flag.

---

## Kontext-Umfang & Platzhalter

| Option | Description | Selected |
|--------|-------------|----------|
| (A) Application-Felder + Config-Bankdaten (unsere IBAN) | bank_iban/bank_name/bank_bic/genossenschaft_name aus Config in den Kontext | ✓ |
| (B) Nur Application-Felder + offener Betrag | Schlanker, aber Seed-Vorlage inhaltlich wertlos | |

**User's choice:** (A), vom User explizit präzisiert.
**Notes:** User-Zitat sinngemäß: „Wir brauchen nicht die IBAN vom Mitglied, sondern wir schicken UNSERE IBAN raus. Das Mitglied muss ja überweisen. Unsere IBAN steht dann im Template." → Bankdaten kommen aus der Config (dieselbe Quelle wie send_confirmation_mail), nicht aus der Application (die hat kein Bankfeld). Variablennamen member-kompatibel (Default), offener Betrag als vorformatierter String `open_amount` (Default).

---

## Validierung (APTPL-04)

| Option | Description | Selected |
|--------|-------------|----------|
| (A) Dummy-Application-Kontext-Probe | Fester Sentinel, analog validate_template_with_repayment; kein DB-Zugriff, deterministisch | ✓ |
| (B) Probe gegen echte Applications aus DB | Realistischer, aber teurer und leer ohne Applications | |

**User's choice:** (A) — Default übernommen.
**Notes:** Generischer `validate_rendered`-Kern extrahiert; `validate_template`-Signatur bleibt unverändert (~40 Member-Tests grün). Validierung greift bei Create/Update in rest_templates.rs (Default „Ja").

---

## format_eur_de + Seed-Content

| Option | Description | Selected |
|--------|-------------|----------|
| Helper in genossi_service | Neben iban::mask_iban, reine Domänen-Formatierung | ✓ |
| send_confirmation_mail umstellen (A) | Ersetzt naives format!("{},{:02} €"), Konsistenz | ✓ |
| send_confirmation_mail unangetastet (B) | Minimaler Blast-Radius, zwei Formatter | |

**User's choice:** Default übernommen — Helper in genossi_service, send_confirmation_mail umgestellt, Null-/Negativ-Fall robust + getestet.
**Notes:** Seed „Zahlungserinnerung" formell, fixe UUID `…0003`, template_type='application', mit Bankverbindung + Verwendungszweck. Frage nach fixer UUID vom User via „Rest Default" bejaht.

---

## Claude's Discretion

- Exakter Wortlaut/Formatierung des Seed-Vorlagen-Texts (formell, deutsch, strict-render-sicher).
- Sentinel-Werte des Dummy-Application-Kontexts.
- body_html für den Seed gesetzt oder text-only.

## Deferred Ideas

- Mehrstufige Erinnerungs-Vorlagen (APTPL-FUT-01) — Zukunfts-Phase; `template_type` hält den Weg offen.
- Versand + REST + Guardrails → Phase 31.
- Frontend-Compose-Dialog + Selector-Filterung im UI → Phase 32.
