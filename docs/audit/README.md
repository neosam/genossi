# Audit-Dokumentation

Dieses Verzeichnis enthält die Dokumentation des Audit-Log- und
Zeitstempel-Systems von Genossi in prüferfreundlicher Form.

## Dokumente

| Datei | Sprache | Zweck |
| ----- | ------- | ----- |
| `revisionssicherheit.de.typ` | Deutsch | Technische Hauptdokumentation für Prüfer und Betreiber. Beschreibt Architektur, Hash-Kette, RFC-3161-Zeitstempel, Verifikationspfade, Grenzen der Implementierung und Begriffe. |
| `compliance.en.typ` | Englisch | Englischsprachiges Pendant zur Hauptdokumentation. |
| `template-betreiber.de.typ` | Deutsch | Ausfüllbare Vorlage für den Betreiber einer konkreten Installation (TSA-Anbieter, Konfiguration, Administratoren, Backup-Strategie). Wird ergänzend zur Hauptdokumentation an den Prüfer übergeben. |
| `style.typ` | — | Gemeinsames Layout (Farbschema, Titelseite, Kopfzeile, Info-Boxen). Wird von allen drei Dokumenten per `#import` eingebunden. |

Kompilierte PDFs liegen unter `pdf/`. Sie sind ins Repo eingecheckt, damit
Prüfer und Betreiber die Dokumente auch ohne lokale Typst-Installation
verwenden können.

## Bauen

Typst ist nicht zwingend im Systempfad vorausgesetzt — der folgende Befehl
funktioniert auf jedem System mit `nix` (z. B. NixOS, Nix auf Linux/macOS):

```bash
cd docs/audit

SOURCE_DATE_EPOCH=$(date +%s) nix-shell -p typst --run '
  typst compile revisionssicherheit.de.typ pdf/revisionssicherheit.de.pdf &&
  typst compile compliance.en.typ           pdf/compliance.en.pdf &&
  typst compile template-betreiber.de.typ   pdf/template-betreiber.de.pdf
'
```

### Warum `SOURCE_DATE_EPOCH`?

Typst verwendet zu Gunsten reproduzierbarer Builds standardmäßig den
Zeitstempel `1970-01-01` für `datetime.today()`. Mit `SOURCE_DATE_EPOCH`
setzen wir beim Kompilieren die aktuelle Systemzeit, damit die Titelseite
das Datum der letzten Kompilierung ausweist. Ohne diese Variable würde
`Stand: 1. Januar 1980` erscheinen.

Wenn statt eines Kompilier-Datums ein festes Veröffentlichungsdatum im
Dokument stehen soll (z. B. nach einem Release-Freeze), kann
`SOURCE_DATE_EPOCH` auf den gewünschten Unix-Zeitstempel gesetzt werden:

```bash
SOURCE_DATE_EPOCH=$(date -d 2026-04-16 +%s) nix-shell -p typst --run '...'
```

## Template-Workflow

Das Dokument `template-betreiber.de.typ` ist als kompakte *Vorlage* konzipiert
(vier Abschnitte, rund drei Seiten). Empfohlener Workflow:

1. Die Quelldatei entweder lokal kopieren oder direkt bei
   [typst.app](https://typst.app) im Browser hochladen — dort lässt sich das
   Dokument wie in einem Online-Editor bearbeiten und direkt als PDF
   exportieren, ohne lokale Tool-Installation.
2. Platzhalter in eckigen Klammern (`[Name der Genossenschaft]`, `[…]`)
   durch die konkreten Werte der Installation ersetzen.
3. Nicht zutreffende Angaben streichen.
4. Durch die vertretungsberechtigte Person unterzeichnen lassen und
   gemeinsam mit der Hauptdokumentation an den Prüfer übergeben.

Das Template nutzt bewusst den kompakten Modus von `style.typ`
(`compact: true`) --- keine Titelseite, kein Inhaltsverzeichnis, alle
Metadaten im Kopf der ersten Seite. So wirkt das Dokument wie ein Formular
und nicht wie ein Bericht.

## Aktualisierung

Bei inhaltlichen Änderungen an der Audit-Implementierung sind die
Dokumente entsprechend anzupassen:

- `revisionssicherheit.de.typ` und `compliance.en.typ` inhaltsgleich halten,
- Versionsnummer am Dokumentkopf erhöhen,
- PDFs neu bauen und einchecken (`pdf/` ist Teil des Repos).

## Font-Voraussetzungen

Die Dokumente sind auf unter Nix typischerweise verfügbare freie Fonts
ausgelegt (`Libertinus Serif`, `DejaVu Sans Mono`). Auf Systemen, auf
denen diese Fonts fehlen, fällt Typst stillschweigend auf alternative
Systemschriften zurück; das Layout bleibt dabei erhalten, das Schriftbild
kann geringfügig abweichen.
