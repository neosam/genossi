## Why

Wenn der Verein eine Generalversammlung einberuft, werden Mitglieder zuerst per E-Mail eingeladen (günstiger, weniger Aufwand). Alle Mitglieder, die per E-Mail nicht erreicht werden konnten — sei es weil der Versand fehlschlug oder weil keine E-Mail-Adresse hinterlegt ist — müssen per Post eingeladen werden. Aktuell gibt es keine Möglichkeit, diese "nicht erreichten" Mitglieder zu identifizieren.

## What Changes

- Neuer Backend-Endpoint, der zu einem gegebenen Mail-Job alle aktiven Mitglieder zurückgibt, die **nicht** erfolgreich per E-Mail erreicht wurden (status != "sent" oder gar nicht im Job enthalten)
- Neuer Filter in der Frontend-Mitgliederliste: Dropdown zur Auswahl eines Mail-Jobs, das die Liste auf nicht erreichte Mitglieder einschränkt

## Capabilities

### New Capabilities
- `mail-unreached-filter`: Serverseitiger Filter, der aktive Mitglieder identifiziert, die durch einen bestimmten Mail-Job nicht erfolgreich erreicht wurden (failed oder nicht enthalten)

### Modified Capabilities

## Impact

- **Backend**: Neuer REST-Endpoint (z.B. `GET /members/not-reached-by/{job_id}`), neue Service- und DAO-Methoden
- **Frontend**: Erweiterung der Mitgliederliste um Mail-Job-Filter-Dropdown, zusätzlicher API-Call
- **Bestehende APIs**: Keine Änderungen an bestehenden Endpoints
