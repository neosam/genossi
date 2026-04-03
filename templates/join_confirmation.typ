// Beitrittsbestätigung (Join Confirmation )
// Available variables (via sys.inputs):
//   member.first_name, member.last_name, member.member_number,
//   member.join_date, member.shares_at_joining, member.current_shares,
//   member.street, member.house_number, member.postal_code, member.city,
//   member.email, member.company, member.exit_date, member.current_balance,
//   member.comment, today

#import "_layout.typ": letter

#let member = json.decode(sys.inputs.at("member"))
#let today = sys.inputs.at("today")

#show: letter.with(
  title: "Beitrittsbestätigung",
  date: today,
)

Hiermit bestätigen wir, dass

#align(center)[
  #text(size: 13pt, weight: "bold")[
    #member.first_name #member.last_name
  ]
]

seit dem *#member.join_date* Mitglied unserer Genossenschaft ist.

#v(0.5cm)

#table(
  columns: (1fr, 1fr),
  stroke: none,
  [*Mitgliedsnummer:*], [#member.member_number],
  [*Beitrittsdatum:*], [#member.join_date],
  [*Geschäftsanteile bei Beitritt:*], [#member.shares_at_joining],
  [*Aktuelle Geschäftsanteile:*], [#member.current_shares],
)

#v(2cm)

#line(length: 6cm, stroke: 0.5pt)
Ort, Datum

#v(1cm)

#line(length: 6cm, stroke: 0.5pt)
Unterschrift Vorstand
