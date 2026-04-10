## Context

Ausgehende Mails haben aktuell keinen standardisierten Footer. Benutzer müssen ihren Namen und eine Grußformel manuell eintippen. Die Vorlagen (Formell/Informell) enthalten zwar Grußformeln, aber keinen Absendernamen. Es gibt bereits ein User-Preferences Key-Value-System und ein Config-Store-System, die beide genutzt werden können.

Das Template-System basiert auf minijinja und kennt bisher nur Member-Variablen. Für den Footer brauchen wir eine neue Variable `sender_name`, die aus den User Preferences des eingeloggten Users kommt.

## Goals / Non-Goals

**Goals:**
- Konfigurierbares Footer-Template als globale Config-Einstellung
- `sender_name` als User Preference pro Application-User
- Gerenderter Footer wird beim Öffnen des Mail-Compose ins Textfeld vorausgefüllt
- Beim Einfügen einer Vorlage wird der Footer nahtlos angehängt
- Footer ist nach dem Einfügen normaler editierbarer Text
- Vorlagen verlieren ihre eingebettete Grußformel

**Non-Goals:**
- HTML-Mails oder Rich-Text-Footer
- Mehrere Footer-Templates zur Auswahl
- Automatisches Anhängen beim Senden (der Footer steht im Body-Text)
- Absendername aus externen Systemen (LDAP etc.)

## Decisions

### 1. Footer-Rendering im Frontend via API-Call

Der Footer wird beim Öffnen des Compose-Formulars über einen neuen Endpunkt `GET /api/mail/footer` abgerufen. Das Backend rendert das Footer-Template mit dem `sender_name` des aktuellen Users und gibt den fertigen Text zurück.

**Alternative**: Footer-Template + sender_name getrennt ans Frontend schicken und dort rendern. Abgelehnt, weil das Template-Rendering (minijinja) im Backend liegt und dort bleiben soll.

### 2. sender_name als User Preference

Der Absendername wird als `sender_name` Key in der bestehenden `user_preferences`-Tabelle gespeichert. Das nutzt die vorhandene Infrastruktur ohne Schema-Änderung.

**Alternative**: Eigenes User-Profil-Modell mit eigener Tabelle. Abgelehnt, weil aktuell nur ein einzelnes Feld benötigt wird und user_preferences genau dafür existiert.

### 3. mail_footer als globale Config

Das Footer-Template wird als `mail_footer` Key im Config Store gespeichert. Alle User teilen sich dasselbe Template, nur `sender_name` variiert.

**Default-Wert**: Wenn `mail_footer` nicht gesetzt ist, wird ein leerer String zurückgegeben (kein Footer).

### 4. Vorlagen ohne Grußformel

Die hartcodierten Vorlagen (TEMPLATE_FORMAL, TEMPLATE_INFORMAL) werden gekürzt — die Grußformel ("Mit freundlichen Grüßen" / "Viele Grüße") wird entfernt. Beim Einfügen einer Vorlage hängt das Frontend den gerenderten Footer nahtlos an.

### 5. Footer-Einfügung im Frontend

Das Frontend cached den gerenderten Footer beim Laden des Compose-Formulars. Der Footer wird eingefügt:
- Als Initialwert: `"\n\n" + footer` im Textfeld
- Beim Vorlage-Einfügen: `vorlage + "\n" + footer`

Nach dem Einfügen ist der Footer normaler Text — keine Markierung, kein Schutz.

## Risks / Trade-offs

- **[Kein sender_name gesetzt]** → Footer wird mit leerem sender_name gerendert, was unschön aussehen kann. Mitigation: Frontend zeigt Hinweis wenn sender_name leer ist.
- **[Footer-Template-Fehler]** → Ungültiges minijinja-Template führt zu Rendering-Fehler. Mitigation: API gibt Fehlermeldung zurück, Frontend zeigt Warnung.
- **[Vorlage überschreibt bearbeiteten Footer]** → Wenn User den Footer bearbeitet hat und dann eine Vorlage einfügt, wird der alte Text ersetzt. Das ist gewolltes Verhalten (Vorlage ersetzt immer den gesamten Body).
