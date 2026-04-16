#import "style.typ": conf, infobox, notebox, fieldtable, accent, muted, rule-color, heute-de

#show: doc => conf(
  title: "Revisionssicherheit: Angaben des Betreibers",
  subtitle: "Ergänzung zur Software-Dokumentation 'Revisionssicherheit des Audit-Logs'",
  version: "Template 1.0",
  date: heute-de(),
  author: "[Name der Genossenschaft]",
  language: "de",
  compact: true,
  doc,
)

#infobox(title: "Hinweise zur Verwendung")[
  Ausfüllbare Ergänzung zur Software-Dokumentation "Revisionssicherheit des
  Audit-Logs". Zum Bearbeiten die Typst-Quelldatei unter
  #link("https://typst.app") im Browser öffnen, Platzhalter (`[...]`)
  ersetzen, als PDF exportieren und gemeinsam mit der Hauptdokumentation
  dem Prüfer übergeben.
]

= Angaben zur Genossenschaft

#fieldtable(
  [*Name*], [[Name der Genossenschaft]],
  [*Anschrift*], [[Straße, PLZ, Ort]],
  [*Registereintrag*], [[Registergericht, GnR-Nummer]],
  [*Prüfungsverband*], [[zuständiger Prüfungsverband]],
  [*IT-Ansprechpartner*], [[Name, E-Mail oder Telefon]],
)

= Eingesetzter Zeitstempeldienst

#fieldtable(
  [*Anbieter*],
  [[Name des Vertrauensdiensteanbieters]],

  [*Qualifiziert nach eIDAS*],
  [[ja / nein]],

  [*EU Trust List*],
  [[Eintrag verlinken oder "nicht gelistet"]],

  [*Stempelintervall*],
  [[z. B. alle 7 Tage, Standardwert der Software]],
)

#notebox(title: "Qualifizierter Zeitstempel")[
  Für die Beweisqualität nach Art. 42 eIDAS-VO ist eine notifizierte
  qualifizierte TSA erforderlich. Liste:
  #link("https://eidas.ec.europa.eu/efda/tl-browser/").
]

= Datensicherung

#fieldtable(
  [*Regelmäßige Sicherung der Datenbank*],
  [[ja / nein -- falls ja: Frequenz grob, z. B. täglich]],

  [*Externe Aufbewahrung der Zeitstempel*],
  [[WebDAV aktiv? ja / nein]],
)

= Bestätigung des Vorstands

Der Vorstand bestätigt die Richtigkeit der Angaben und den produktiven
Einsatz der in der Software-Dokumentation beschriebenen Mechanismen.

#v(2.0cm)

#grid(
  columns: (1fr, 1fr),
  gutter: 2cm,
  [
    #line(length: 100%, stroke: 0.5pt)
    #set text(size: 9pt, fill: muted)
    Ort, Datum
  ],
  [
    #line(length: 100%, stroke: 0.5pt)
    #set text(size: 9pt, fill: muted)
    Unterschrift, Funktion
  ],
)
