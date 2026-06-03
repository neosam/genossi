---
title: Frontend-Checkbox für attach_repayment_letter in mail_page.rs
date: 2026-06-03
priority: medium
blocked_by: keiner — Backend ist fertig (Quick 260603-cz6, Commit 62e62b7)
---

# Frontend-Checkbox für attach_repayment_letter

## Was

Im Bulk-Mail-Compose-Flow (`genossi-frontend/src/page/mail_page.rs`) eine Checkbox „RepaymentLetter automatisch anhängen" einbauen, die das neue Backend-Feld `SendBulkMailRequest.attach_repayment_letter: bool` setzt.

Bedingt sichtbar/aktiviert: nur wenn der Vorstand bereits eine `repayment_phase_id` für den Bulk-Mail-Job ausgewählt hat (existiert über den bestehenden Repayment-Mail-Flow, wo `repayment_phase_id` schon im Request steht).

## Warum

Aktuell ist `attach_repayment_letter` nur über die Swagger-UI testbar. Der Vorstand braucht im normalen Mail-Compose-Flow einen Klick statt händischem JSON.

Vorgelagert: ohne UI ist das Feature für den Vorstand effektiv nicht nutzbar.

## Vorbedingung

- Backend-Endpoint akzeptiert das Feld (✓ Quick 260603-cz6)
- `mail_page.rs` hat bereits einen Repayment-Mail-Pfad mit `repayment_phase_id` (✓ Phase 12 D-19)

## Schritte (grob)

1. `mail_page.rs`: neues `Signal<bool>` für die Checkbox
2. UI-Rendering: Checkbox **nur** zeigen wenn `repayment_phase_id.is_some()` (sonst macht die Option keinen Sinn — Backend wirft 400)
3. Label deutsch: „RepaymentLetter (Anschreiben) als persönliches PDF anhängen"
4. Hinweis-Text unterhalb der Checkbox: „Empfänger ohne generierten Brief in dieser Phase werden als fehlgeschlagen markiert."
5. Beim Submit: `attach_repayment_letter: checkbox.read()` in den Request einbauen
6. Component-First-Prinzip beachten (siehe [[component-first]]): falls die Checkbox + Hinweis-Text irgendwo wiederverwendbar wäre, in eigene Component extrahieren
7. i18n-Keys in `mod.rs` + `de.rs` + `en.rs` (nur diese zwei Locales, siehe genossi-frontend/CLAUDE.md)

## Akzeptanz

- Checkbox sichtbar nur bei gesetzter `repayment_phase_id`
- Klick setzt `attach_repayment_letter=true` im REST-Request
- Backend antwortet 202 (Worker übernimmt), Empfänger ohne Brief erscheinen als `failed` im Job-Detail
- `cargo check --manifest-path genossi-frontend/Cargo.toml` clean
- Keine i18n-Keys ohne Übersetzung in beiden Locales

## Routing

Mit `/gsd-quick` ausführen, sobald jemand Zeit hat. UI-Only-Quick, kein Backend-Touch, ~30-60 Min.

## Cross-Refs

- Backend-Commit: `62e62b7` (siehe [[quick-260603-cz6-summary]])
- Verwandt: [[frontend-uat-empfaenger-status-no-repayment-letter]] — UI-Anzeige im Job-Detail
- Verwandt: [[backend-pre-flight-check-attach-repayment-letter]] — Pre-Flight-Validation
