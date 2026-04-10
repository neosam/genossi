## Why

Ausgehende Mails haben aktuell keinen standardisierten Footer mit Absenderinformationen. Benutzer müssen die Grußformel und ihren Namen manuell eintippen. Ein konfigurierbarer Footer mit Absendername spart Zeit und sorgt für ein einheitliches Erscheinungsbild.

## What Changes

- Neues User-Preference-Feld `sender_name` (Freitext), das pro Application-User gepflegt werden kann
- Neues globales Config-Feld `mail_footer` als minijinja-Template (z.B. `"Mit freundlichen Grüßen\n{{ sender_name }}"`)
- Neuer API-Endpunkt, der den gerenderten Footer zurückgibt (Footer-Template + sender_name des aktuellen Users)
- Frontend: Beim Öffnen des Mail-Compose-Formulars wird der gerenderte Footer als Initialwert ins Textfeld eingefügt
- Frontend: Beim Einfügen einer Vorlage (Formell/Informell) wird der Footer nahtlos angehängt
- Vorlagen (Formell/Informell) enthalten keine Grußformel mehr — diese kommt aus dem Footer
- Der Footer ist nach dem Einfügen normaler editierbarer Text, keine Sonderbehandlung beim Senden

## Capabilities

### New Capabilities
- `mail-footer`: Konfigurierbarer Mail-Footer mit Template-Rendering und Absendername

### Modified Capabilities
- `mail-sending`: Vorlagen (Formell/Informell) verlieren ihre eingebettete Grußformel, da diese durch den Footer ersetzt wird

## Impact

- **Backend**: `genossi_mail` (neuer Footer-Endpunkt, Template-Rendering), `genossi_rest` (user_preferences API für sender_name), Config-Service (mail_footer Feld)
- **Frontend**: `mail_compose` Komponenten (Footer-Initialisierung), `template_selector` (Vorlagen ohne Grußformel), `inbox/reply_form` (Footer auch bei Antworten)
- **Datenbank**: Keine Schema-Änderung nötig — nutzt bestehende `user_preferences` und `config` Tabellen
