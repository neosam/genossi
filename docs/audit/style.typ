// Gemeinsames Layout und Hilfsfunktionen für die Audit-Dokumentation.
//
// Wird von den sprachspezifischen Dokumenten (revisionssicherheit.de.typ,
// compliance.en.typ, template-betreiber.de.typ) per #import eingebunden.

// --------------------------------------------------------------------------
// Datum in lokalisierter Langform.
// Nutzt datetime.today(); Fallback auf das angegebene Datum, falls der
// Compiler ohne Systemzeit läuft (z. B. in restriktiven CI-Umgebungen).
// --------------------------------------------------------------------------
#let de-monate = (
  "Januar", "Februar", "März", "April", "Mai", "Juni",
  "Juli", "August", "September", "Oktober", "November", "Dezember",
)
#let en-monate = (
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
)

#let heute-de(fallback: "2026") = {
  let d = datetime.today()
  if d == none { fallback } else {
    str(d.day()) + ". " + de-monate.at(d.month() - 1) + " " + str(d.year())
  }
}

#let today-en(fallback: "2026") = {
  let d = datetime.today()
  if d == none { fallback } else {
    en-monate.at(d.month() - 1) + " " + str(d.day()) + ", " + str(d.year())
  }
}

// --------------------------------------------------------------------------
// Farbpalette
// --------------------------------------------------------------------------
#let accent = rgb("#1f3a5f")       // Dunkles Navy für Überschriften & Akzente
#let muted = rgb("#5a5a5a")        // Grau für Meta-Text
#let rule-color = rgb("#c8c8c8")   // Linien in Tabellen, Trenner
#let code-bg = rgb("#f4f4f4")      // Hintergrund für Monospace-Blöcke
#let info-bg = rgb("#eef2f7")      // Hinweis-Boxen
#let warn-bg = rgb("#fcf3e2")      // Hinweis auf Grenzen / Anmerkungen

// --------------------------------------------------------------------------
// Hauptkonfiguration
// --------------------------------------------------------------------------
#let conf(
  title: none,
  subtitle: none,
  version: none,
  date: none,
  author: none,
  language: "de",
  compact: false,
  doc,
) = {
  set document(title: title, author: if author != none { author } else { "" })
  set page(
    paper: "a4",
    margin: (x: 2.4cm, y: if compact { 2.0cm } else { 2.6cm }),
    numbering: "1 / 1",
    number-align: center,
    header: if compact { none } else {
      context {
        if counter(page).get().first() > 1 {
          set text(size: 8.5pt, fill: muted)
          grid(
            columns: (1fr, auto),
            align(left, title),
            align(right, if subtitle != none { subtitle } else { "" }),
          )
          v(-0.4em)
          line(length: 100%, stroke: 0.3pt + rule-color)
        }
      }
    },
  )
  set text(
    font: ("Libertinus Serif", "Liberation Serif", "DejaVu Serif"),
    size: 10.5pt,
    lang: language,
  )
  set par(justify: true, leading: 0.65em)
  set heading(numbering: "1.1")

  show heading.where(level: 1): it => {
    if not compact { pagebreak(weak: true) }
    v(if compact { 0.8em } else { 0.4em })
    set text(fill: accent, size: if compact { 14pt } else { 18pt }, weight: "bold")
    block(it)
    v(0.3em)
  }
  show heading.where(level: 2): it => {
    v(0.6em)
    set text(fill: accent, size: 13pt, weight: "bold")
    block(it)
  }
  show heading.where(level: 3): it => {
    v(0.3em)
    set text(fill: accent, size: 11pt, weight: "bold")
    block(it)
  }

  show raw.where(block: false): it => box(
    fill: code-bg,
    inset: (x: 3pt, y: 1pt),
    outset: (y: 2pt),
    radius: 2pt,
    text(font: ("DejaVu Sans Mono", "Liberation Mono"), size: 9pt, it),
  )
  show raw.where(block: true): it => block(
    fill: code-bg,
    inset: 9pt,
    width: 100%,
    radius: 3pt,
    text(font: ("DejaVu Sans Mono", "Liberation Mono"), size: 9pt, it),
  )

  show link: it => text(fill: accent, underline(it))

  let label-version = if language == "de" { "Version:" } else { "Version:" }
  let label-date = if language == "de" { "Stand:" } else { "As of:" }
  let label-author = if language == "de" { "Herausgeber:" } else { "Publisher:" }

  if compact {
    // ---------------------- Kompakter Kopf (für Formulare) ----------------------
    text(size: 9pt, fill: muted, weight: "regular", "Genossi")
    linebreak()
    text(size: 18pt, weight: "bold", fill: accent, title)
    if subtitle != none {
      linebreak()
      text(size: 10pt, fill: muted, subtitle)
    }
    v(0.2em)
    block(width: 100%, stroke: (bottom: 0.3pt + rule-color), inset: (bottom: 4pt))[
      #set text(size: 9pt, fill: muted)
      #grid(
        columns: (1fr, auto, auto),
        gutter: 12pt,
        if author != none [*#label-author* #author] else [],
        if version != none [*#label-version* #version] else [],
        if date != none [*#label-date* #date] else [],
      )
    ]
    v(0.2em)
  } else {
    // ---------------------- Titelseite ----------------------
    set page(header: none, numbering: none)
    v(3.5cm)
    align(center, text(size: 11pt, fill: muted, weight: "regular", "Genossi"))
    v(0.4cm)
    align(center, text(size: 24pt, weight: "bold", fill: accent, title))
    if subtitle != none {
      v(0.3cm)
      align(center, text(size: 13pt, fill: muted, subtitle))
    }

    v(1fr)
    align(center, block(width: 70%, stroke: (top: 0.4pt + rule-color), inset: (top: 10pt))[
      #set text(size: 10pt, fill: muted)
      #grid(
        columns: (1fr, 1fr),
        gutter: 8pt,
        if version != none [*#label-version* #version] else [],
        if date != none [*#label-date* #date] else [],
        if author != none [*#label-author* #author] else [],
        [],
      )
    ])
    v(1.4cm)

    pagebreak()

    // ---------------------- Inhaltsverzeichnis ----------------------
    set page(numbering: "i", number-align: center)
    counter(page).update(1)
    [
      #align(left, text(size: 16pt, fill: accent, weight: "bold",
        if language == "de" { "Inhalt" } else { "Contents" }))
      #v(0.6em)
      #outline(title: none, depth: 3, indent: auto)
    ]

    pagebreak()

    // ---------------------- Hauptteil ----------------------
    set page(numbering: "1", number-align: center, header: context {
      if counter(page).get().first() > 1 {
        set text(size: 8.5pt, fill: muted)
        grid(
          columns: (1fr, auto),
          align(left, title),
          align(right, if subtitle != none { subtitle } else { "" }),
        )
        v(-0.4em)
        line(length: 100%, stroke: 0.3pt + rule-color)
      }
    })
    counter(page).update(1)
  }

  doc
}

// --------------------------------------------------------------------------
// Hinweis-Boxen
// --------------------------------------------------------------------------
#let infobox(title: none, body) = block(
  fill: info-bg,
  inset: 10pt,
  radius: 3pt,
  width: 100%,
  stroke: (left: 2pt + accent),
)[
  #if title != none [
    #text(weight: "bold", fill: accent, title)
    #v(0.3em)
  ]
  #body
]

#let notebox(title: none, body) = block(
  fill: warn-bg,
  inset: 10pt,
  radius: 3pt,
  width: 100%,
  stroke: (left: 2pt + rgb("#c0955a")),
)[
  #if title != none [
    #text(weight: "bold", fill: rgb("#8a6a2c"), title)
    #v(0.3em)
  ]
  #body
]

// --------------------------------------------------------------------------
// Feldtabelle: kompakte Darstellung für Konfigurations-Einstellungen
// --------------------------------------------------------------------------
#let fieldtable(..rows) = {
  table(
    columns: (auto, 1fr),
    stroke: (x, y) => (
      top: if y == 0 { 0.6pt + rule-color } else { 0.3pt + rule-color },
      bottom: 0.3pt + rule-color,
    ),
    inset: (x: 6pt, y: 5pt),
    ..rows.pos()
  )
}
