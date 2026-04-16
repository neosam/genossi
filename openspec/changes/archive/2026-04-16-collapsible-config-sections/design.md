## Context

Die Konfigurationsseite (`config_page.rs`, 1245 Z.) trägt sechs bis sieben Sektionen, jede etwa 100–250 Zeilen RSX. Heute ist alles entfaltet sichtbar — der Admin scrollt durch viel Inhalt, den er gerade nicht braucht. Zwei Sektionen sind bereits als eigene Komponenten ausgelagert (`TsaConfigSection`, `WordPressIntegrationSection`); die übrigen leben inline in `config_page.rs`.

Frontend-Konvention (siehe `genossi-frontend/CLAUDE.md`): Component-First. Wiederverwendbare UI-Bausteine leben unter `src/component/`, Pages komponieren sie. Eine zusammenklappbare Sektion ist genau so ein Baustein und passt zur Konvention.

## Goals / Non-Goals

**Goals:**
- Sofortiger Übersichtsgewinn auf der Konfigurationsseite.
- Wiederverwendbare Komponente, die ohne weiteren Aufwand auf anderen langen Seiten (z. B. Mitglieds-Detail, Mail-Page) eingesetzt werden kann.
- Bestehende Inhalte und Steuerelemente in den Sektionen bleiben semantisch und funktional unverändert.

**Non-Goals:**
- Keine Anwendung auf andere Seiten in diesem Change. `member_details.rs` und `mail_page.rs` können in eigenen Folge-Changes umgezogen werden.
- Kein Persistieren des Aufgeklappt-Zustands über Seitenwechsel hinweg (z. B. via localStorage). Wäre ein netter Folge-Change, hier aber nicht nötig.
- Kein URL-Fragment, das eine Sektion direkt öffnet (z. B. `/config#smtp`). Folge-Change.
- Keine globale „Alle aufklappen" / „Alle einklappen"-Steuerung. Falls später nötig, separat.

## Decisions

### Mehrere Sektionen gleichzeitig offen erlaubt

Striktes Akkordion (immer nur eine offen) wäre verlockend für mehr Übersicht, kollidiert aber mit echten Workflows: SMTP konfigurieren *und* gleichzeitig Mail-Footer prüfen ist ein realistischer Fall. Nutzer dürfen mehrere Sektionen offen halten.

*Alternative:* Striktes Akkordion. Verworfen wegen Workflow-Friktion.

### Lokaler State pro Komponente

Jede `CollapsibleSection` hält ihren eigenen Open/Close-State (`use_signal(false)` initial). Kein globaler State, kein Sync zwischen Sektionen. Einfach, ausreichend, keine Race Conditions.

*Alternative:* Globaler State, der speichert, welche Sektionen offen sind. Verworfen — würde Komplexität ohne Gegenwert einführen.

### Klick auf gesamten Header schaltet um

Großzügige Klickfläche reduziert Frustration. Pfeil-Icon ist nur visueller Cue, nicht eigenes Klick-Target. Tastatur-Bedienung: Header ist `<button>`-semantisch, damit Enter/Space funktionieren.

*Alternative:* Nur Pfeil-Icon klickbar. Verworfen wegen kleiner Trefffläche, schlechter UX.

### Bestehende Komponenten weiter nutzen

`TsaConfigSection` und `WordPressIntegrationSection` werden nicht refaktoriert — sie werden in eine `CollapsibleSection` gewickelt. Diese Komponenten bekommen also einen Wrapper, ihr Inhalt bleibt wie er ist. So vermeiden wir, mehrere Refactor-Stränge zu vermischen.

*Alternative:* Auch die ausgelagerten Komponenten umbauen, sodass sie selbst eine `CollapsibleSection` werden. Verworfen wegen unnötiger Komplexität.

## Risks / Trade-offs

- [Nutzer fragt sich nach dem Aufklappen, was der ursprüngliche Zustand war (alle zu)] → Akzeptiert. Refresh genügt zum Zurücksetzen, Lernkurve ist gering.
- [Versteckte Inhalte erschweren das Suchen via Browser-Suche (Ctrl+F)] → Akzeptiert. Falls relevant, kann ein Folge-Change „Alle aufklappen"-Knopf einführen.
- [Sektion-Inhalt bleibt im DOM, auch wenn eingeklappt — könnte bei sehr großem Inhalt Memory kosten] → Bei Konfigurationsformularen vernachlässigbar. Falls später relevant: konditional rendern.
- [Animations-/Transition-Effekte beim Auf-/Zuklappen sind nicht spezifiziert] → Erstmal ohne Animation. Falls gewünscht, später ergänzbar via CSS.
