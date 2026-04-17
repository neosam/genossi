## Context

Die Genossi-Verwaltung hat heute eine Splash-Seite unter `/` (`genossi-frontend/src/page/home.rs`, 33 Zeilen) mit einem Riesentitel und einem Button „Mitglieder", der zur Mitgliederliste führt. In der Praxis ist die Mitgliederliste die effektive Startseite — der Nutzer klickt sich nur durch.

Mitgliedsanträge (`/applications`) und Posteingang (`/inbox`) sind im Top-Bar-Dropdown unter „Mitglieder" bzw. „Kommunikation" einsortiert. Auf dem Smartphone steckt das gesamte Top-Bar-Menü hinter dem Hamburger-Icon, wodurch Hinweise dort visuell verloren gehen.

Beide Bereiche kennen bereits einen Status „offen":
- `Application.status == "Offen"` (siehe `application_page.rs:45-50`)
- `InboundMail.status == "open"` (siehe `inbox_page.rs:49`)

Beide Listen werden über bestehende REST-Endpoints geliefert, die einen Statusfilter unterstützen.

## Goals / Non-Goals

**Goals:**
- Wegfall der Splash-Seite zugunsten direkter Mitgliederliste.
- Auf einen Blick sichtbarer Hinweis, dass Anträge oder Mails offen sind — auch mobil ohne Hamburger.
- Ein-Klick-Sprung in die jeweilige Liste.
- Wiederverwendbare Komponente, die später um weitere Indikatoren (z. B. Validierung) erweitert werden kann.

**Non-Goals:**
- Kein neuer Backend-Endpoint nur für Counts (bestehende Listen-Endpoints reichen für die Größenordnung).
- Kein Polling oder Live-Update — manuelles Neuladen beim Seitenwechsel genügt.
- Keine Verfeinerung der Berechtigungen (alle aktiven Nutzer sind heute Admin; eigener Change geplant).
- Keine Erweiterung um zusätzliche Indikatoren in diesem Change (Validierung etc. später).

## Decisions

### Statusbalken statt Top-Bar-Badge

Ein Badge in der Top-Bar wäre versteckt, sobald das Menü auf mobilen Geräten in den Hamburger wandert. Ein Statusbalken direkt über der Mitgliederliste bleibt überall sichtbar und nimmt nur eine Zeile ein.

*Alternative:* Badge am Dropdown-Label „Mitglieder ●3". Verworfen, weil der Hinweis erst beim Öffnen des Dropdowns wirkt — und das Mobil-Problem nicht löst.

### Items immer sichtbar, immer klickbar

Auch bei 0 offenen Einträgen wird das Item gezeigt („Keine offenen Anträge") und bleibt klickbar. So entsteht ein konsistenter Reflex: „Anträge ansehen → Statusbalken oben". Der Text wechselt von Zahl auf „Keine offenen …", aber Position und Verhalten bleiben gleich.

*Alternative:* Item bei 0 ausblenden. Verworfen, weil das Layout dann „springt" und der Reflex weg ist; außerdem geht die positive Rückmeldung „alles erledigt" verloren.

### Counts via bestehende Endpoints + `.len()`

Bei zwei Items mit überschaubaren Listen ist ein eigener Count-Endpoint nicht gerechtfertigt. Wenn später vier oder mehr Indikatoren angelegt werden, kann ein Sammel-Endpoint `GET /api/badges` nachgezogen werden.

*Alternative A:* `GET /api/applications/count?status=Offen` und Pendant für Inbox. Mehr Code im Backend ohne erkennbaren Nutzen heute.

*Alternative B:* Sammel-Endpoint sofort. Vorgezogene Komplexität ohne den zweiten oder dritten Konsumenten.

### Wiederverwendbare Komponente `StatusBar` + `StatusBarItem`

Die Komponente lebt unter `genossi-frontend/src/component/status_bar.rs` und nimmt eine Liste von Items entgegen. Jedes Item kapselt: Label-Text (zwei Varianten: mit Zahl und „keine"), Zielroute und optionalen Filter-Hint. Damit folgen wir dem Component-First-Prinzip aus der Frontend-CLAUDE.md und können den Balken später trivial um weitere Items ergänzen.

### Redirect statt Umbau der Home-Page

Wir leiten `/` direkt auf `/members` um, statt die Splash-Page zur Übersicht auszubauen. Begründung:
- Aktuelle Erwartungshaltung der Nutzer: „nach Login direkt Mitgliederliste".
- Die Splash-Seite hat heute keine eigenständige Funktion; der einzige Inhalt ist der Button auf die Mitgliederliste.
- Wenn später echte Dashboard-Inhalte gewünscht sind, kann `/dashboard` neu eingeführt werden, ohne den Workflow erneut zu brechen.

*Alternative:* Home zur Übersichtsseite mit Statusbalken + Schnellaktionen ausbauen. Verworfen, weil die Nutzer dann zwei Klicks bis zur Mitgliederliste brauchen — gegen die geäußerte Erwartung.

### Fehlerverhalten: neutraler Platzhalter

Schlägt einer der Count-Abrufe fehl, zeigt das betroffene Item „—" statt einer Zahl. Die Mitgliederliste bleibt voll bedienbar; das andere Item zeigt seinen Wert wie gewohnt. So führt eine kurze Backend-Hickup-Phase nicht zu einem leeren Bildschirm.

## Risks / Trade-offs

- [Counts sind beim erneuten Besuch der Mitgliederliste evtl. veraltet] → Akzeptiert. Ein erneutes Aufrufen der Seite reicht; Polling lohnt sich für ein internes Tool dieser Größe nicht.
- [Listen-Endpoints liefern volle Datensätze, nur um deren Länge zu zählen] → Akzeptiert für den Anfang. Wenn Antrags- oder Inbox-Listen unerwartet groß werden, ziehen wir einen `count`- oder `badges`-Endpoint nach.
- [Bestehende Bookmarks auf `/` zeigen weiterhin auf eine andere Seite] → Akzeptabel, weil die Umleitung transparent erfolgt; Bookmark-Inhalt bleibt im Sinne der ursprünglichen Absicht.
- [Späterer Umbau zu mehr Indikatoren erweitert nur die Items-Liste] → Bewusst niedrige Einstiegshürde, sodass „Validierung", „fehlgeschlagene Mail-Jobs" etc. später ohne Komponentenumbau dazukommen.
