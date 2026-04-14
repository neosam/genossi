#import "@preview/letter-pro:3.0.0": letter-simple

#set text(lang: "de")
#let application = json.decode(sys.inputs.at("application"))
#let today = sys.inputs.at("today")
#let name = [#application.first_name #application.last_name]

 #let anrede = if application.salutation == "Herr" {
    "Lieber"
  } else if application.salutation == "Frau" {
    "Liebe"
  } else {
    "Hallo"
  }


#show: letter-simple.with(
  sender: (
    name: "nebenan & unverpackt München W. eG",
    address: "Willibaldstr. 18, 80687 München",
    extra: [
      Telefon: #link("tel:08954637600")[+089 - 54 63 76 00]\
      //E-Mail: #link("mailto:info@nebenan-unverpackt.de")[info\@nebenan-unverpackt.de]\
      Mitgliederverwaltung: #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de]\
    ],
  ),
  
  //annotations: [Einschreiben - Rückschein],
  recipient: [
    #name \
    #application.street #application.house_number \
    #application.postal_code #application.city
  ],
  
  //reference-signs: (
  //  ([Steuernummer], [333/24692/5775]),
  //),
  
  date: [#today],
  subject: "Eintrittsbestätigung",
  folding-marks: true
)

#place(top + left, dx: -0.55cm, dy: -0.5cm, image("nebenan-unverpackt-logo.svg", width: 5cm))

#line(length: 16.5cm, stroke: 0.5pt + gray)

#table(
  columns: (1fr, 1fr),
  stroke: none,
  [*Name:*], [#name],
  [*Gezeichnete Anteile:*], [#application.shares],
)

#line(length: 16.5cm, stroke: 0.5pt + gray)
#v(1cm)

#anrede #name,

herzlich willkommen in der Genossenschaft nebenan & unverpackt München West eG. 
Die Satzung der Nebenan & Unverpackt München West eG findest du auf unserer Homepage
unter: www.nebenan-unverpackt.de.

Wir freuen uns auf ein gutes Miteinander!

#v(1cm)

Herzliche Grüße,

Carolin Weidmann, Dina Beier und Simon Goller
