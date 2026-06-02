# Phase 12: Frontend (Component-First) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-01
**Phase:** 12-Frontend (Component-First)
**Areas discussed:** Detail-Page Lifecycle-UX, RepaymentEntryList Component, `ausbezahlt`-Confirm + Massenmail-Flow, Add-Entry-Modal + Member-Picker

---

## Detail-Page Lifecycle-UX

### Frage 1: Wo sitzen die Lifecycle-Buttons „Öffnen" / „Schließen"?

| Option | Description | Selected |
|--------|-------------|----------|
| Im Page-Header (oberhalb der Tabs) | Immer sichtbar wie `assembly_details.rs`-Vorbild | |
| Im 'Stamm-Daten'-Tab als große Action-Kachel | Tabs sauber gekapselt; Vorstand muss aktiv den Tab öffnen | ✓ |
| Im Tab-Header des betroffenen Tabs | Kontext-nah, aber inkonsistent über 2 Tabs verteilt | |

**User's choice:** Stamm-Daten-Tab als Action-Kachel.

### Frage 2: Reaktion auf 409 `CloseConflictResponse` beim Schließen?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline-Banner + Auto-Switch zum Einträge-Tab | Actionable feedback mit Status-Filter | |
| Toast mit Fehlermeldung, ohne Tab-Switch | Bestehendes Toast-Pattern, simpler | ✓ |
| Modal mit pending-Liste + Direktlinks | Sichtbarstes UX, zusätzliches Pattern | |

**User's choice:** Toast ohne Tab-Switch.

### Frage 3: Wo lebt die `share_value`-Korrektur (PHAS-04)?

| Option | Description | Selected |
|--------|-------------|----------|
| Stamm-Tab: inline editierbar mit „Speichern" | Eine UI-Stelle, drei Status-Modi | ✓ |
| Stamm-Tab: read-only + separater Modal | Bewusst extra Klick gegen versehentliche Änderung | |
| Einträge-Tab: oberhalb der Tabelle mit Live-Preview | Spannender Effekt, mischt Concerns | |

**User's choice:** Inline-Edit im Stamm-Tab.

### Frage 4: Wie sehen die 3 Tabs im Status `Vorbereitung` aus?

| Option | Description | Selected |
|--------|-------------|----------|
| Tabs immer sichtbar; Einträge/Export-Tab zeigen Hinweis-Box | Konsistent mit Phase-4 D-13 Assembly-Pattern | ✓ |
| Einträge/Export-Tab disabled (grau) | Visuell deutlich, schwerer zu rendern | |
| Nur Stamm-Tab sichtbar, andere dynamisch | Reduziert, seltenes Pattern in der App | |

**User's choice:** Tabs immer sichtbar mit Hinweis-Box.

### Frage 5: Braucht 'Schließen' einen Pre-Confirm-Dialog?

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — Confirm-Modal vor POST | Defensiv, konsistent zu `ausbezahlt`-Confirm | ✓ |
| Nein — direkt POST | Doppelter Klick ist Reibung | |
| Confirm nur für Schließen, Öffnen direkt | Asymmetrisch | |

**User's choice:** Confirm-Modal vor `Schließen`.

### Frage 6: Verhalten der Detail-Page nach Status `Abgeschlossen`?

| Option | Description | Selected |
|--------|-------------|----------|
| Read-only; Einträge-Tab ohne Aktionen; Export aktiv | Klare visuelle Trennung 'fertig' | ✓ |
| Read-only PLUS Audit-Log-Link im Stamm-Tab | Verbandskonformer Audit-Spur-Tieflink | |
| Read-only + 'Phase abgeschlossen am'-Badge | Weniger Klick-Tiefe, ohne neue Verknüpfungen | |

**User's choice:** Read-only ohne Audit-Link (deferred).

### Frage 7: Was passiert auf der Detail-Page nach erfolgreichem `Öffnen`?

| Option | Description | Selected |
|--------|-------------|----------|
| Page reloaded; Vorstand klickt selbst zum Einträge-Tab | Simpel, kein Cross-Component-Trigger | |
| Auto-Switch zum Einträge-Tab + Toast mit N | Sofortiges Feedback, mehr Glue-Code | |
| Reload + Inline-Highlight der Auto-Befüllt-Rows | Höchster UX-Aufwand | |

**User's choice:** Free-text "weniger Aufwand im Code". Claude entschied Variante 1 (Reload, kein Auto-Switch).
**Notes:** Vorstand sieht den neuen State nach Tab-Switch ohnehin; Cross-Component-Tab-Trigger ist neue Konvention.

---

## RepaymentEntryList Component

### Frage 1: Spalten-Set der Tabelle?

| Option | Description | Selected |
|--------|-------------|----------|
| 6 Spalten: Nr, Name, Anteile, Betrag, Status, Actions (kein IBAN) | Banking-Pre-View ohne IBAN-Spalte | |
| 7 Spalten: + IBAN | Volle Transparenz; IBAN-Nullables sofort sichtbar | ✓ |
| 5 Spalten Minimal: Nr, Name, Status, Anteile, Actions | Mobil-freundlich, ohne Betrag | |

**User's choice:** 7 Spalten inkl. IBAN.

### Frage 2: Multi-Select-Pattern?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-Row + Header-Checkbox immer sichtbar | Standard-Pattern, Tablet-tauglich | ✓ |
| Checkbox nur bei Hover sichtbar | Visuell ruhiger, schlecht für Touch | |
| Per-Row immer + Action-Bar dynamisch | Gmail-artig, mehr UX-Politur | |

**User's choice:** Standard immer-sichtbar.

### Frage 3: Status-Filter-UX?

| Option | Description | Selected |
|--------|-------------|----------|
| Tab-Strip-im-Tab mit Count-Badges | Vier visuelle Sub-Tabs | ✓ |
| Multi-Select-Pill-Filter | Mehrere Status gleichzeitig | |
| Default 'Offen+Angeschrieben' + Toggle | Pragmatisch, schlankste UX | |

**User's choice:** Tab-Strip-im-Tab.

### Frage 4: Wo wird `share_count_to_pay_out` bearbeitet?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline-Cell-Edit in der Tabelle | Schnellster Workflow, neuer Component | ✓ |
| Edit-Modal über Action-Spalte | Modal-Pattern existiert, mehr Klicks | |
| Eigene Sub-Page `/entries/{id}/edit` | Konsequent Page-Layered, Overkill | |

**User's choice:** Inline-Cell-Edit.

### Frage 5: Button-Pattern-Grep-Gate?

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — Grep-Gate verpflichtend, alle Phase-12-Buttons | Verhindert Drift wie Backend-`audited_*!`-Gate | ✓ |
| Ja — nur neue Files, kein retroaktiver Roll-out | Pragmatisch | |
| Nein — nur in CONTEXT.md dokumentiert | Soft-Approach, Drift-Risiko | |

**User's choice:** Hartes Grep-Gate (D-01/D-02).
**Notes:** User-Initialinput: „bei den Buttons aufgepasst werden, dass die nicht ihre Standard Submit Aktion im Browser durchführen [...] dass da kein prevent default gemacht wurde."

### Frage 6: List-Tail (Default-Sort, Empty-State, Soft-Delete, Status-Farben)?

**User's choice:** Free-text "Ich denke, wir können starten. Ich muss die UI erst sehen und danach können wir ggf noch fixen."
**Notes:** Claude entschied alle 4 Sub-Items als Claude's-Discretion-Defaults: Mitgliedsnummer ASC Sort, Empty-State-Box mit Add-CTA, Trash-Icon Soft-Delete + Confirm (nur ≠ ausbezahlt), Status-Farben grau/blau/grün. UAT/Verify kann fein-justieren.

---

## `ausbezahlt`-Confirm + Massenmail-Flow

### Frage 1: `ausbezahlt`-Toggle Multi-Select?

| Option | Description | Selected |
|--------|-------------|----------|
| Strikt Single-Row (Multi-Select nicht erlaubt) | Konsequent zu Backend, klar | |
| Frontend-Loop + per-Row-Confirm | Komfort, N Bestätigungen | |
| Frontend-Loop + EIN Sammel-Confirm am Anfang | Spart N-1 Confirms, riskanter Klick | ✓ |

**User's choice:** Frontend-Loop + ein Sammel-Confirm.

### Frage 2: Confirm-Dialog-Inhalt?

| Option | Description | Selected |
|--------|-------------|----------|
| Listentabelle + Summe + 3-Punkt-Warnliste | Maximale Transparenz | ✓ |
| Kurzform + Type-to-Confirm | Sicherste, mehr Friction | |
| Standard-Confirm: nur Warntext + Button | Einfachstes Pattern, weniger Transparenz | |

**User's choice:** Listentabelle + Summe + 3-Punkt-Warnliste.

### Frage 3: Massenmail-Flow?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline-Modal mit reused mail_compose-Components | Maximaler Reuse, neuer Modal | |
| Redirect zur /mail-Page mit Pre-Selection | Kontextwechsel, 0 neue Composer-UI | ✓ |
| Neue /repayment-phases/{id}/mail-Sub-Page | Eigene Routen, Overkill | |

**User's choice:** Redirect zur /mail-Page.

### Frage 4: Status-Übergang `offen → angeschrieben`?

| Option | Description | Selected |
|--------|-------------|----------|
| Manuell in der RepaymentEntryList via Multi-Select-Button | ENTR-06 wortwörtlich-manuell | ✓ |
| Halbautomatisch: Vorschlag-Banner nach Mail-Versand | Bessere UX, semi-automatisch | |
| Komplett entkoppelt, keine Mail-Verknüpfung | Maximal flexibel, riskiert Vergessen | |

**User's choice:** Manuelle Aktion via Multi-Select-Button.

---

## Add-Entry-Modal + Member-Picker

### Frage 1: `MemberSearch`-Reuse?

| Option | Description | Selected |
|--------|-------------|----------|
| Direkter Reuse ohne Änderungen | 0 LOC neue Picker-Logik | ✓ |
| Reuse + optionales `show_shares`-Prop | Anteile sichtbar im Dropdown | |
| Neuer `repayment_member_picker.rs` | Duplikat-Risiko, Memory-Verletzung | |

**User's choice:** Direkter Reuse.

### Frage 2: `share_count_to_pay_out`-Vorbefüllung?

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-Vorbefüllung mit `member.current_shares` | Standard-Use-Case Voll-Auszahlung | ✓ |
| Leer lassen, aktiv eingeben | Defensiv | |
| Vorbefüllung mit `1` (Minimum) | Neutraler Default | |

**User's choice:** Auto-Vorbefüllung mit `current_shares`.

### Frage 3: Client-Side-Validation?

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal: nur > 0 und Member ausgewählt | Backend-Backstop reicht | ✓ |
| Wie 1 + Soft-Warnung wenn > current_shares | Frühe Sichtbarkeit, false-positive-Risiko | |
| Wie 1 + Hard-Block wenn > current_shares | ENTR-03-Violation | |

**User's choice:** Minimal.

### Frage 4: Add vs Edit ein oder zwei Components?

| Option | Description | Selected |
|--------|-------------|----------|
| Add = Modal, Edit = Inline-Cell-Edit (zwei Patterns) | Klare Trennung der Concerns | ✓ |
| Geteilter Modal-Component (Picker bei Edit disabled) | Widerspricht Inline-Edit-Decision | |

**User's choice:** Zwei distinkte Patterns.

---

## Claude's Discretion

Areas wo User die Entscheidung an Claude überließ:

- **Reload nach `Öffnen`** (D-09): User „weniger Aufwand im Code". Claude wählte Variante 1 (Reload, kein Auto-Tab-Switch).
- **RepaymentEntryList-Tail-Defaults** (D-14): User „Ich muss die UI erst sehen". Claude wählte: Mitgliedsnummer-ASC-Sort, Empty-State-Box mit CTA, Trash-Icon-Soft-Delete + Confirm, Status-Badge-Farben grau/blau/grün.
- **Listen-Page-Default-Sort, Listen-Page-Filter, Modal-Reuse, Toast-Pattern, Auto-Befüllungs-Empty-State-Wortlaut, exakte i18n-Key-Liste** — alle laut CONTEXT.md `<decisions>`-Section bewusst als Claude-Discretion markiert.

---

## Deferred Ideas

- **Halbautomatische Status-Übergänge** (nach Mail-Send): Vorschlag-Banner statt rein manuell — UAT entscheidet, ob nachgeholt wird.
- **`share_value`-Korrektur als Modal** (statt Inline): Defensiver, falls UAT versehentliche Korrekturen zeigt.
- **Audit-Log-Verlinkung** in der Detail-Page nach `Abgeschlossen`.
- **Listen-Page-Filter und sortierbare Header**: erst beim Skalierungs-Schmerzpunkt.
- **Bulk-`ausbezahlt` als Backend-atomar**: Service-Layer-Erweiterung mit eigener Cascade-Test-Suite.
- **Re-Open einer abgeschlossenen Phase**: v2-Diskussion mit Audit-Konsequenzen.
- **CSV-Export-Tab**: EXPO-04 v2-deferred per Phase 11 D-12.
- **WASM-/E2E-Test-Pipeline**: out-of-scope für v1.1.
- **Mobile-Layout-Optimierung**: Desktop-First in Phase 12.
- **`Öffnen`-Toast mit Auto-Befüll-Counter** + ggf. Auto-Tab-Switch: kann später UX-aufgewertet werden, wenn Vorstand-Feedback es zeigt.
