// Phase 13 D-13-01: Bundle-Variante — N Briefe in EINEM Typst-Compile,
// mit #pagebreak() zwischen Recipients.
// Single-Source-of-Truth: importiert render-letter aus dem Single-Letter-Template.

#import "auszahlungs_anschreiben.typ": render-letter

#set text(lang: "de")

#let recipients = json.decode(sys.inputs.at("recipients"))
#let today = sys.inputs.at("today")

#for (i, r) in recipients.enumerate() {
  render-letter(r.member, r.repayment, today)
  if i < recipients.len() - 1 {
    pagebreak()
  }
}
