// Teilnehmerliste fuer Generalversammlung — Phase 6 (D-04, D-08, D-10)
// Inputs via sys.inputs:
//   meta (JSON string): {"title": str, "date": str, "present": int, "total": int|null}
//   rows (JSON string): [{"member_number": int, "first_name": str, "last_name": str,
//                         "salutation": str|null, "title": str|null, "is_present": bool}, ...]

#import "_layout.typ": letter

#let meta = json.decode(sys.inputs.at("meta"))
#let rows = json.decode(sys.inputs.at("rows"))

#show: letter.with(
  title: meta.title,
  date: meta.date,
)

#text(size: 12pt)[
  #if meta.at("total", default: none) != none [
    *#meta.present von #meta.total anwesend*
  ] else [
    *#meta.present anwesend*
  ]
]

#v(0.5cm)

#table(
  columns: (auto, 1fr, 1fr, auto, auto, auto),
  align: (right, left, left, left, left, center),
  stroke: 0.5pt,
  table.header(
    repeat: true,
    [*Nr.*], [*Nachname*], [*Vorname*], [*Anrede*], [*Titel*], [*anwesend*],
  ),
  ..rows.map(r => (
    [#r.member_number],
    [#r.last_name],
    [#r.first_name],
    [#r.at("salutation", default: "")],
    [#r.at("title", default: "")],
    if r.is_present [✓] else [],
  )).flatten()
)
