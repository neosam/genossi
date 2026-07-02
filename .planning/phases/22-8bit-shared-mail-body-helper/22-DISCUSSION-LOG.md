# Phase 22: 8bit + Shared Mail-Body Helper - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-02
**Phase:** 22-8bit-shared-mail-body-helper
**Areas discussed:** Helfer-API-Zuschnitt, Config-Toggle-Form, Test-Strategie, MAIL-04 Verifikations-Doku

---

## Helfer-API-Zuschnitt

Die zentrale Refactor-Entscheidung: wie viel Verantwortung schluckt der geteilte Helfer? Ausgangspunkt: die drei Sendepfade unterscheiden sich in zwei unabhängigen Dimensionen — Body-Part (charset/CTE, der Bug-Ort) und Umschlag (message-id, in-reply-to, Attachments).

| Option | Description | Selected |
|--------|-------------|----------|
| A — Text-Part-Fabrik | Helfer baut nur den `SinglePart` (charset+CTE); jeder Pfad assembliert Message/Attachments selbst. Minimal blast radius. | |
| B — Body-Fabrik | Helfer besitzt auch das Attachment-Wrapping; bräuchte async + `DocumentStorage`, obwohl Test/Digest nie Attachments haben. | |
| C — Message-Fabrik | Voller Message-Builder inkl. from/to/subject/message-id. Maximal DRY, größter blast radius, zwingt Test/Digest zu message-id-Semantik. | |
| D — Message-Fabrik mit I/O-Naht | Pure/synchrone `build_message(...)` (Baustein 1+2) auf Basis bereits geladener Attachment-Bytes; Attachment-Laden bleibt pfadspezifisch. | ✓ |
| E — Test-Mail als versteckter echter Job | Test-Mail → persistierter `MailJob` mit `kind`-Flag, vom Worker abgearbeitet, aus UI gefiltert. Braucht Schema-Migration + async-Test-UX. | |

**User's choice:** D+ mit `build_message`-Split (nicht DI).
**Notes:** Der User hinterfragte zu Recht, dass Test-Mails heute einen komplett eigenen Code-Pfad nehmen und damit nicht testen, was beim echten Versand passiert (Ursache des Charset-Bugs). Er schlug zunächst Option E vor (Test-Mail durch den Worker als geflaggter Job). Nach Aufzeigen der Konsequenzen — Schema-Migration (Phase 22 ist „no-schema") + Umstellung der Test-Mail von synchron auf asynchron (Verlust des sofortigen Feedbacks) — entschied er sich für D+: der Test-Pfad ruft dieselbe geteilte `build_message`-Funktion + `transport.send()` auf (identischer Sende-Code), aber ohne Job zu persistieren und ohne `DocumentStorage`-DI in den Service (stattdessen der build_message-Split, der die Naht bei den geladenen Attachment-Bytes zieht). Option E als Deferred Idea festgehalten.

---

## Config-Toggle-Form

| Option | Description | Selected |
|--------|-------------|----------|
| A — String-Enum `smtp_encoding` | KV-Key `"quoted-printable"` (default) / `"8bit"`, spiegelt `smtp_tls`; internes `MailEncoding`-Enum. | ✓ |
| B — Bool `smtp_8bit` | `true`/`false`, simpler aber nicht erweiterbar/selbstdokumentierend. | |

**User's choice:** A (Enum).
**Notes:** User-Regel: „immer Enum und keine booleans" — auch bei nur zwei Zuständen. Als Projekt-Präferenz in Memory festgehalten.

---

## Test-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| A — Helfer als getestete Single-Source | Unit-Tests direkt gegen `build_message`; MIME-Byte-Asserts für charset + CTE in beiden Modi (quoted-printable + 8bit); worker-Tests rufen den Helfer auf statt zu re-inlinen. | ✓ |
| B — Nur Charset-Regression fixen | Minimale Tests, 8bit bleibt untested (nur Prod-Verify). | |

**User's choice:** A (Empfehlung angenommen).
**Notes:** Die 8bit-CTE ist trotz nicht durchführbarem Prod-Relay-Test auf Byte-Ebene testbar (`Content-Transfer-Encoding: 8bit` im `email.formatted()`-Output). Schließt genau die Lücke, durch die der ursprüngliche Charset-Bug rutschte.

---

## MAIL-04 Verifikations-Doku

| Option | Description | Selected |
|--------|-------------|----------|
| A — Runbook/README-Abschnitt | Konkreter `openssl s_client`-EHLO-Check auf `250-8BITMIME` + Reihenfolge „erst verifizieren, dann Config setzen". | ✓ |
| B — Nur Kommentar am Config-Loader | Weniger sichtbar, kein reproduzierbarer Betreiber-Schritt. | |

**User's choice:** A.
**Notes:** Verify-in-Prod, aus Dev nicht automatisierbar (Relay nur über Prod-Netz erreichbar).

## Claude's Discretion

- Genauer Modul-/Dateiname der geteilten Funktion (`send.rs` o. ä.) und exakte Signatur-Details.
- Ort der Betreiber-/Runbook-Doku für MAIL-04.
- Ob der quoted-printable-Zweig `SinglePart::plain` behält oder ebenfalls explizit CTE setzt.

## Deferred Ideas

- Option E (Test-Mails als versteckte echte `MailJob`-Rows mit `MailJobKind`-Enum) → eigene Phase, falls gewünscht (braucht Migration + async-Test-UX + Filter-Fläche).
- FMT-01 (deutsches Datumsformat) → Phase 23.
- HTML-Mail → Phase 23, WYSIWYG-Editor → Phase 24, Antrags-Datei-Upload → Phase 25.
