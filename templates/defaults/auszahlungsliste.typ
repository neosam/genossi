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
//     "account_holder": str | none,  // Quick 260607-mw9: Kontoinhaber separat
//   }, ...]
//
// D-04 / D-05: purpose-Strings enthalten Umlaute (z. B. "Anteilsrückzahlung")
// im Original — KEINE ASCII-Sanitization. Typst rendert UTF-8 nativ.
// D-07: iban kann leerer String sein (Member.bank_account == None) — wird als
// leere Zelle gerendert (kein Crash, kein Visual-Marker).
// Quick 260607-mw9: account_holder fehlt/none/leer → Fallback auf row.name
// (Defense-in-Depth via `r.at("account_holder", default: none)` damit alte
// JSON-Payloads ohne das Feld nicht crashen).

#import "_layout.typ": letter

#let meta = json.decode(sys.inputs.at("meta"))
#let rows = json.decode(sys.inputs.at("rows"))

// ─── Helper: Kontoinhaber mit Fallback auf Mitgliedsname ────────────────────
// Quick 260607-mw9: Wenn account_holder gesetzt UND nicht leerer String,
// verwenden — sonst row.name. Defensive `.at(..., default: none)` damit
// alte Payloads ohne das Feld nicht panicen.
#let account-holder-for(r) = {
  let ah = r.at("account_holder", default: none)
  if ah != none and ah != "" {
    ah
  } else {
    r.name
  }
}

// Quick 260607-mw9: Querformat — bei 7 Spalten (Nr/Name/Kontoinhaber/IBAN/
// Anteile/Betrag/Verwendungszweck) wird A4 hoch sonst zu eng.
#show: letter.with(
  title: meta.title,
  date: meta.date,
  landscape: true,
)

#text(size: 11pt)[
  *Geschaeftsjahr #meta.fiscal_year — #meta.row_count Auszahlung(en)*
]

#v(0.5cm)

#table(
  columns: (auto, 1fr, 1fr, auto, auto, auto, 1fr),
  align: (right, left, left, left, right, right, left),
  stroke: 0.5pt,
  table.header(
    repeat: true,
    [*Nr.*], [*Name*], [*Kontoinhaber*], [*IBAN*], [*Anteile*], [*Betrag*], [*Verwendungszweck*],
  ),
  ..rows.map(r => (
    [#r.member_number],
    [#r.name],
    [#account-holder-for(r)],
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
