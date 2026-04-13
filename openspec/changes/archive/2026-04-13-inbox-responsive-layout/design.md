## Context

Die Inbox-Seite (`genossi-frontend/src/page/inbox_page.rs`) verwendet ein festes Zwei-Spalten-Layout mit `w-1/2` fuer Liste und Detail. Es gibt keine responsiven Breakpoints und keine Viewport-Hoehenbegrenzung. Auf mobilen Geraeten sind beide Spalten zu schmal und die Seite scrollt unkontrolliert.

Der State `selected_id: Signal<Option<String>>` existiert bereits und steuert, welche Mail im Detail angezeigt wird. Dieser State kann direkt fuer die mobile List/Detail-Navigation wiederverwendet werden.

## Goals / Non-Goals

**Goals:**
- Inbox auf mobilen Geraeten (< 768px) benutzbar machen mit List/Detail-Pattern
- Viewport-fixiertes Layout, das internes Scrollen statt Seiten-Scrollen verwendet
- Desktop-Layout beibehalten (zwei Spalten nebeneinander)

**Non-Goals:**
- Redesign der Inbox-Funktionalitaet (Filter, Actions, Reply bleiben gleich)
- Responsive Anpassungen fuer andere Seiten
- Swipe-Gesten oder andere Mobile-spezifische Interaktionen
- Anpassung der TopBar fuer Mobil

## Decisions

### 1. CSS-only Responsive mit Tailwind-Breakpoints

Die mobile/desktop-Umschaltung erfolgt ausschliesslich ueber Tailwind-CSS-Klassen (`hidden`, `md:flex`, `md:hidden`). Kein zusaetzlicher Rust-State fuer "ist mobil".

**Warum:** Der bestehende `selected_id`-State reicht aus, um zu entscheiden, ob Liste oder Detail sichtbar ist. Ein separater `is_mobile`-State wuerde Komplexitaet hinzufuegen und koennte mit dem tatsaechlichen Viewport desynchronisieren.

**Steuerung:**
- `selected_id == None`: Liste sichtbar, Detail versteckt (auf Mobil)
- `selected_id == Some(...)`: Detail sichtbar, Liste versteckt (auf Mobil)
- Auf Desktop (`md:`): immer beide sichtbar

### 2. Viewport-fixiertes Layout mit `h-[calc(100vh-4rem)]`

Der aeussere Container bekommt eine feste Hoehe basierend auf dem Viewport minus der TopBar-Hoehe (4rem). Die Mail-Liste und das Detail-Panel scrollen intern.

**Warum:** Die Alternative waere `h-screen` mit `overflow-hidden` auf dem Body, aber das wuerde andere Seiten beeinflussen. `calc(100vh-4rem)` ist lokal zur Inbox-Seite.

**Struktur:**
```
Container:     flex flex-col h-[calc(100vh-4rem)]
  Header:      flex-none (Titel, Fehler-Banner)
  Content:     flex flex-1 min-h-0 gap-4
    Liste:     flex flex-col overflow-y-auto
    Detail:    flex flex-col overflow-hidden
      Header:  flex-none
      Body:    flex-1 overflow-y-auto
      Actions: flex-none
```

`min-h-0` auf dem Content-Container ist entscheidend — ohne das ignoriert Flexbox die Overflow-Einstellungen der Kinder.

### 3. Zurueck-Button nur auf Mobil sichtbar

Ein Zurueck-Button wird im Detail-View hinzugefuegt mit `md:hidden`. Er setzt `selected_id` auf `None` und `detail` auf `None`.

**Warum:** Auf Desktop ist der Zurueck-Button unnoetig, da die Liste immer sichtbar ist.

## Risks / Trade-offs

- **TopBar-Hoehe hardcoded (4rem):** Wenn die TopBar-Hoehe sich aendert, stimmt das `calc()` nicht mehr. → Mitigation: Die TopBar ist stabil und aendert sich selten. Bei Bedarf kann spaeter eine CSS-Variable eingefuehrt werden.
- **Reply-Form auf Mobil:** Die Reply-Form ist relativ lang. Sie muss innerhalb des scrollbaren Detail-Bereichs liegen, damit sie nicht den Container sprengt. → Mitigation: Die Reply-Form ist bereits innerhalb des Detail-Containers, muss nur im scrollbaren Bereich bleiben.
- **`max-h-96` auf dem Body-Pre entfernen:** Aktuell begrenzt `max-h-96` den Mail-Body. Das wird durch `flex-1 overflow-y-auto` ersetzt, damit der Body den verfuegbaren Platz dynamisch fuellt. → Kein Risiko, da der aeussere Container jetzt die Hoehe begrenzt.
