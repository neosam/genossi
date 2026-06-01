// Auszahlungsliste fuer RepaymentPhase — Phase 11 (EXPO-02)
// Inputs via sys.inputs:
//   meta (JSON string): {
//     "title": str,
//     "date": str,
//     "fiscal_year": int,
//     "row_count": int,
//     "total_amount_str": str,
//     "phase_id": str,
//   }
//   rows (JSON string): [{
//     "member_number": int,
//     "name": str,
//     "iban": str,
//     "share_count": int,
//     "amount_str": str,
//     "purpose": str,
//   }, ...]
//
// D-04 / D-05: purpose-Strings enthalten Umlaute (z. B. "Anteilsrückzahlung")
// im Original — KEINE ASCII-Sanitization. Typst rendert UTF-8 nativ.
// D-07: iban kann leerer String sein (Member.bank_account == None) — wird als
// leere Zelle gerendert (kein Crash, kein Visual-Marker).

#import "_layout.typ": letter

#let meta = json.decode(sys.inputs.at("meta"))
#let rows = json.decode(sys.inputs.at("rows"))

#show: letter.with(
  title: meta.title,
  date: meta.date,
)

#text(size: 11pt)[
  *Geschaeftsjahr #meta.fiscal_year — #meta.row_count Auszahlung(en)*
]

#v(0.5cm)

#table(
  columns: (auto, 1fr, auto, auto, auto, 1fr),
  align: (right, left, left, right, right, left),
  stroke: 0.5pt,
  table.header(
    repeat: true,
    [*Nr.*], [*Name*], [*IBAN*], [*Anteile*], [*Betrag*], [*Verwendungszweck*],
  ),
  ..rows.map(r => (
    [#r.member_number],
    [#r.name],
    [#r.iban],
    [#r.share_count],
    [#r.amount_str],
    [#r.purpose],
  )).flatten()
)

// Summenzeile (Planner-Discretion, CONTEXT.md erlaubt): Nice-to-Have fuer
// Banking-Vorstand. Phase-10-D-04-Pattern: Service liefert pre-formatted
// EUR-String (z. B. "360,00").
#v(0.5cm)
#text(size: 10pt)[
  Gesamt: *#meta.row_count Eintraege* — Summe *#meta.total_amount_str EUR*
]
