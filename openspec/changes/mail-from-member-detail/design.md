## Context

Die Mitgliederliste (`members.rs:615`) bietet bereits einen Knopf „Mail senden", der die Markierung mehrerer Mitglieder über den globalen Signal `SELECTED_MEMBER_IDS` an die Mail-Page übergibt. Die Mail-Page (`mail_page.rs:52`) liest beim Laden diesen Signal und füllt die Empfänger-Auswahl entsprechend.

Die Detailseite (`member_details.rs`) zeigt das Mitglied isoliert — der Sprung in den Mail-Versand ist heute nur über den Umweg „zurück zur Liste, markieren, klicken". Wir können denselben Mechanismus aus der Detailseite anstoßen.

## Goals / Non-Goals

**Goals:**
- Ein-Klick-Sprung in den Mail-Versand für genau das gerade angesehene Mitglied.
- Wiederverwendung des bestehenden `SELECTED_MEMBER_IDS`-Patterns ohne neue Infrastruktur.
- Klares Empty-State-Verhalten, wenn keine E-Mail-Adresse hinterlegt ist.

**Non-Goals:**
- Kein eigener Compose-Modal-Pfad auf der Detailseite. Der Mail-Versand bleibt eine zentrale Seite mit einem Workflow.
- Keine Vorauswahl eines Templates aus dem Detail-Kontext (das könnte ein späterer Folge-Change sein).
- Keine Änderung am Mail-Page-Verhalten selbst.

## Decisions

### Sprung statt Modal

Die Compose-UI lebt unter `component/mail_compose/` mit Subject, Body-Editor, Template-Auswahl, Variablen und Vorschau. Diese Bausteine in einem Modal auf der Detailseite zu rekonstruieren wäre Code-Duplikation und würde zwei Mail-Workflows mit unterschiedlichen Verhaltensweisen schaffen. Eine Weiterleitung auf `/mail` mit vorgewähltem Empfänger ergibt einen einheitlichen Workflow.

*Alternative:* Modal mit eingebetteter Compose-Form. Verworfen wegen Code-Duplikation und potentiell divergierenden UI-Pfaden.

### Wiederverwendung des Selection-Patterns

Wir setzen den globalen Signal so wie es die Mitgliederliste tut:

```rust
SELECTED_MEMBER_IDS.write().clear();
SELECTED_MEMBER_IDS.write().toggle(member_id);
nav.push(Route::MailPage {});
```

Das ist null neue Infrastruktur. Falls jemand später den Selection-Mechanismus ändert (z. B. auf Query-Param), passt sich dieser Knopf automatisch an, weil er denselben Pfad nutzt.

*Alternative:* Eigene Service-Funktion „mail_to_member(id)" als Wrapper. Verworfen, weil der direkte Aufruf trivial ist und kein gemeinsames Verhalten kapselt.

### Empty-State: disabled statt versteckt

Wenn das Mitglied keine E-Mail-Adresse hat, wird der Knopf disabled angezeigt — nicht versteckt. Begründung: Konsistente Position des Knopfes über alle Mitgliedsansichten hinweg, plus expliziter Hinweis „warum geht das gerade nicht". Verstecken würde den Admin im Unklaren lassen, ob der Knopf existiert.

*Alternative:* Knopf nur zeigen, wenn E-Mail vorhanden. Verworfen wegen schlechterer Erkennbarkeit der fehlenden Adresse.

### Anlegen-Modus: Knopf nicht zeigen

Im „neues Mitglied anlegen"-Modus existiert die Member-ID noch nicht — Mail-Versand ist semantisch nicht möglich. Hier wird der Knopf gar nicht gerendert, um den Anlegen-Flow nicht mit irrelevanten Aktionen zu belasten.

## Risks / Trade-offs

- [Wenn der Nutzer auf `/mail` zurückkehrt und den Browser-Back nutzt, ist die Auswahl evtl. überraschend] → Akzeptiert. Verhalten ist konsistent mit dem heutigen Pattern aus der Mitgliederliste.
- [Falls jemand `SELECTED_MEMBER_IDS` umbenennt oder die Semantik ändert, müssen beide Aufrufstellen angepasst werden] → Akzeptiert; Build-Fehler würden das aufdecken.
- [Wenn die Mail-Page später einen anderen Übergabe-Mechanismus bekommt (z. B. URL-Param), muss dieser Knopf mitgezogen werden] → Akzeptiert; gemeinsame Anpassung ist überschaubar.
