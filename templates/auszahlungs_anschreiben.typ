// Phase 13 D-13-05 / D-13-06: RepaymentLetter Default-Template.
// Single-Source-of-Truth fuer Brief-Body via `render-letter`-Funktion.
// Bundle-Variante (auszahlungs_anschreiben_bundle.typ) importiert diese Funktion
// und iteriert ueber recipients[] — siehe Plan 13-01 must_haves.
//
// Verfuegbare Variablen im `r` (RepaymentContext)-Parameter:
//   r.share_count    — Anzahl Anteile zur Auszahlung (Integer)
//   r.share_value    — Anteilswert pro Stueck (deutscher Euro-String "X,YZ", z.B. "120,00")
//                      Quick 260602-r2i: neu — Templates koennen "#r.share_value €" referenzieren.
//   r.payout_amount  — Gesamtauszahlungsbetrag (deutscher Euro-String "X,YZ")
//   r.fiscal_year    — Geschaeftsjahr der Phase (Integer)

#import "@preview/letter-pro:3.0.0": letter-simple

#set text(lang: "de")

// ─── Single-Letter-Modus: sys.inputs lesen ────────────────────────────────────
#let member = json.decode(sys.inputs.at("member"))
#let repayment = json.decode(sys.inputs.at("repayment"))
#let today = sys.inputs.at("today")

// ─── Helper: Anrede-String ableiten (Herr -> "Lieber", Frau -> "Liebe", sonst "Hallo")
#let anrede-for(m) = if m.salutation == "Herr" {
    "Lieber"
  } else if m.salutation == "Frau" {
    "Liebe"
  } else {
    "Hallo"
  }

// ─── Helper: Name-Block ableiten (mit/ohne title) ─────────────────────────────
#let name-for(m) = if m.title != none {
    [#m.title #m.first_name #m.last_name]
  } else {
    [#m.first_name #m.last_name]
  }

// ─── Helper: Kontoinhaber-Name (Fallback auf Mitgliedsname) ──────────────────
// Quick 260607-mw9: Wenn account_holder gesetzt UND nicht leer, im Recipient-
// Adressblock zeigen — der Brief muss dann an den Kontoinhaber adressiert sein,
// damit das Bankhaus die Überweisung korrekt zuordnet.
// Anrede ("Lieber Hans,") bleibt bewusst auf member.first/last_name —
// der Brief richtet sich textlich ans Mitglied, der Adress-Header an den
// Kontoinhaber. `m.at("account_holder", default: none)` ist defensive,
// damit ältere JSON-Payloads ohne das Feld nicht crashen.
#let account-holder-for(m) = if m.at("account_holder", default: none) != none and m.account_holder != "" {
    [#m.account_holder]
  } else {
    name-for(m)
  }

// ─── render-letter: EXPORTED FUNCTION — Single-Source-of-Truth Brief-Body ────
// Wird sowohl direkt im Single-Mode unten aufgerufen als auch von
// auszahlungs_anschreiben_bundle.typ via `#import "auszahlungs_anschreiben.typ": render-letter`.
//
// Hinweis zur Typst-Scope-Semantik: `show: letter-simple.with(...)` wirkt
// auf das umgebende Block-Scope. Innerhalb dieser Funktion (Block-Body)
// ist das valide und betrifft nur die hier emittierten Inhalte. Beim
// Aufruf im Bundle wechseln Recipient/Subject pro Iteration, daher ist
// `show`-im-Funktions-Body erwuenscht.
#let render-letter(m, r, today) = {
  let name = name-for(m)
  let anrede = anrede-for(m)

  show: letter-simple.with(
    sender: (
      name: "nebenan & unverpackt München W. eG",
      address: "Willibaldstr. 18, 80687 München",
      extra: [
        Telefon: #link("tel:08954637600")[+089 - 54 63 76 00]\
        Mitgliederverwaltung: #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de]\
      ],
    ),
    recipient: [
      // Quick 260607-mw9: Recipient = account_holder (wenn gesetzt) sonst Mitgliedsname.
      // Anrede unten im Brief bleibt auf `name` (= name-for(m), Mitgliedsname).
      #account-holder-for(m) \
      #m.street #m.house_number \
      #m.postal_code #m.city
    ],
    date: [#today],
    subject: "Auszahlung deiner Anteile",
    folding-marks: true,
  )

  place(top + left, dx: -0.55cm, dy: -0.5cm,
        image("nebenan-unverpackt-logo.svg", width: 5cm))

  line(length: 16.5cm, stroke: 0.5pt + gray)

  // ─── D-13-06 Baustein 1: Reference-Block ────────────────────────────────────
  // Reihenfolge: Stueck * Wert = Summe (Quick 260602-r2i fuegt Anteilswert ein).
  table(
    columns: (1fr, 1fr),
    stroke: none,
    [*Mitgliedsnummer:*], [#m.member_number],
    [*Anteile zur Auszahlung:*], [#r.share_count],
    [*Hoehe pro Anteil:*], [#r.share_value €],
    [*Auszahlungsbetrag:*], [#r.payout_amount €],
  )

  line(length: 16.5cm, stroke: 0.5pt + gray)
  v(1cm)

  // ─── D-13-06 Baustein 2: Anrede + Auszahlungsbetrag-Absatz ──────────────────
  [#anrede #name,]
  v(0.5em)
  [deine Anteile aus dem Geschäftsjahr #r.fiscal_year werden in Kürze ausgezahlt.]
  v(0.5em)

  // ─── D-13-06 Baustein 3: IBAN-Switch (Pitfall #5) ───────────────────────────
  if m.bank_account != none [
    Wir überweisen den Betrag in Höhe von #r.payout_amount € auf deine
    hinterlegte IBAN: *#m.bank_account*.
  ] else [
    *Wir haben keine IBAN von dir hinterlegt* — bitte teile sie uns unter
    #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de] mit,
    damit wir dir den Betrag in Höhe von #r.payout_amount € überweisen können.
  ]

  v(1cm)

  // ─── D-13-06 Baustein 4: Vorstands-Signatur (hardcoded) ─────────────────────
  [Herzliche Grüße,]
  v(0.5em)
  [Carolin Weidmann, Dina Beier und Simon Goller]
}

// ─── Single-Letter-Modus: render-letter direkt aufrufen ──────────────────────
#render-letter(member, repayment, today)
