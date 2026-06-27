# Phase 21: Reply-Komfort — Antwort im vollflächigen Modal - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-27
**Phase:** 21-reply-komfort-antwort-im-vollfl-chigen-modal
**Areas discussed:** Close-/Abbrechen-Affordances, Draft-Schutz beim Abbrechen, Großes Textfeld umsetzen, Umfang im Modal

---

## Close-/Abbrechen-Affordances

| Option | Description | Selected |
|--------|-------------|----------|
| X-Icon im Header + Abbrechen-Button, kein Backdrop | Konsistent mit membership_adjust_modal (Header mit X rechts, on_close). Kein versehentliches Schließen durch Backdrop-Klick. | ✓ |
| Zusätzlich Backdrop-Klick + Escape | Bequemer, aber Risiko: versehentlicher Backdrop-Klick verwirft Entwurf. Neues Muster im Projekt. | |
| Nur Abbrechen-Button, kein X-Icon | Minimal, bricht aber mit dem etablierten X-Icon-Header-Muster. | |

**User's choice:** X-Icon im Header + Abbrechen-Button, kein Backdrop
**Notes:** Folgt dem bestehenden Projekt-Muster (`membership_adjust_modal.rs:144-151`); `InboxReplyForm` bekommt `on_close`-Prop + eigenen Header.

---

## Draft-Schutz beim Abbrechen

| Option | Description | Selected |
|--------|-------------|----------|
| Kein Confirm — Abbrechen schließt direkt | Entspricht Phasen-Ziel + bestehenden Modals. Einfachste Variante. | |
| Confirm nur wenn Text geändert | confirm nur wenn Body/Subject vom Initialwert abweicht. Mehr Schutz, neues Muster + Vergleichslogik. | ✓ |
| Immer Confirm vor Verwerfen | Jeder Abbruch fragt nach. Sicherste, aber nörgeligste Variante. | |

**User's choice:** Confirm nur wenn Text geändert
**Notes:** Kritische Implikation festgehalten (CONTEXT D-05): Reply-Body wird async vorbefüllt (Footer lädt on-mount), daher muss die Dirty-Baseline NACH dem Footer-Load geschnappt werden, sonst false-positive „geändert". Dirty-Check als pure, unit-testbare Helper-Funktion (D-06).

---

## Großes Textfeld umsetzen

| Option | Description | Selected |
|--------|-------------|----------|
| Optionaler Höhen-Prop, Default h-40, Reply nutzt größer | Component-First: MailBodyEditor bekommt optionales height-Prop. | |
| Modal gibt Raum, Editor-Höhe bleibt h-40 | Keine Editor-Änderung; nur Modal-Kontext wirkt größer. | ✓ |
| Body-Editor global vergrößern | h-40 global erhöhen — betrifft auch Compose. | |

**User's choice:** Modal gibt Raum, Editor-Höhe bleibt h-40
**Notes:** `MailBodyEditor` bleibt unverändert (auch von Compose genutzt). „Großes Textfeld" entsteht durch geräumigeren Modal-Kontext. Konfigurierbarer Höhen-Prop als Deferred Idea notiert.

---

## Umfang im Modal

| Option | Description | Selected |
|--------|-------------|----------|
| Komplettes InboxReplyForm unverändert | Subject/Templates/Attachments/Preview/Body — alles wie heute, nur im Modal. Kein Verhaltensbruch. | ✓ |
| Nur Body + Senden im Modal | Reduziert aufs Textfeld; Templates/Attachments entfallen. Größerer Umbau. | |

**User's choice:** Komplettes InboxReplyForm unverändert
**Notes:** Reine Verlagerung ins Modal; keine funktionale Reduktion.

## Claude's Discretion

- Exakter Confirm-Mechanismus (native `window.confirm` vs. In-App).
- Modal-Header-Titeltext, genaue Tailwind-Klassen, Bündelung von `on_close` + Confirm in einem `attempt_close`-Closure.

## Deferred Ideas

- Konfigurierbarer Höhen-Prop am `MailBodyEditor` für ein echtes „großes" Eingabefeld (Component-First-konform, Default h-40).
- Backdrop-/Escape-Close projektweit einheitlich für alle Modals (nicht nur Reply).
