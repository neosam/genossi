// Genossi Default Layout
// This layout template provides a standard letter format for cooperative documents.
// Override by editing this file in the template editor.

#let letter(
  title: none,
  date: none,
  content,
) = {
  set page(
    paper: "a4",
    margin: (top: 3cm, bottom: 2.5cm, left: 2.5cm, right: 2cm),
  )
  set text(
    font: "Liberation Sans",
    size: 11pt,
    lang: "de",
  )
  set par(
    leading: 0.8em,
    justify: true,
  )

  // Date
  if date != none {
    align(right)[#date]
    v(1cm)
  }

  // Title
  if title != none {
    text(size: 14pt, weight: "bold")[#title]
    v(0.5cm)
  }

  // Content
  content
}
