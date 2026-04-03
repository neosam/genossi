## Context

Mitglieder werden aktuell über `GET /api/members` geladen und alle Filter (aktiv, ausgetreten, Suche etc.) laufen clientseitig im Frontend. Der Mail-Versand nutzt `mail_jobs` und `mail_recipients` Tabellen, wobei `mail_recipients.member_id` die Verknüpfung zum Mitglied herstellt und `mail_recipients.status` den Versandstatus ("pending", "sent", "failed") enthält.

Um "nicht erreichte" Mitglieder zu identifizieren, muss die Information aus zwei unabhängigen Datenbereichen zusammengeführt werden: Mitgliederliste und Mail-Job-Recipients.

## Goals / Non-Goals

**Goals:**
- Serverseitiger Endpoint, der zu einem Mail-Job alle aktiven Mitglieder zurückgibt, die nicht erfolgreich erreicht wurden
- Integration als Filter in die bestehende Mitgliederliste im Frontend
- Die bestehende Mitgliederliste und deren Filter bleiben unverändert

**Non-Goals:**
- CSV-Export oder Typst-Serienbriefe (kommt als separates Feature)
- Änderungen an bestehenden Mail- oder Member-Endpoints
- Historische Auswertung über mehrere Mail-Jobs hinweg

## Decisions

### 1. Neuer dedizierter Endpoint statt Query-Parameter auf bestehender Member-Route

Der neue Endpoint wird `GET /api/members/not-reached-by/{job_id}` sein.

**Alternativen:**
- Query-Parameter auf `GET /api/members?not_reached_by=<job_id>`: Würde die bestehende Route und deren Service-Methode verkomplizieren. Da die Member- und Mail-Module getrennt sind, würde das unnötige Kopplung erzeugen.
- Clientseitige Verknüpfung: Frontend lädt Mitglieder und Job-Recipients separat und filtert selbst. Funktioniert, skaliert aber schlechter und verlagert Logik ins Frontend.

**Entscheidung:** Eigener Endpoint, da die Logik eine SQL-Query über zwei Module hinweg ist und serverseitig sauberer bleibt.

### 2. Endpoint im Member-REST-Modul, Logik im Mail-Service

Der Endpoint lebt im `genossi_rest` Member-Modul, da er MemberTOs zurückgibt. Die eigentliche Query-Logik (JOIN über member und mail_recipients) wird im `genossi_mail` Service implementiert, da dieser Zugriff auf die mail_recipients-Tabelle hat.

**Alternativen:**
- Alles im Member-Modul: Würde erfordern, dass der Member-Service Kenntnis von mail_recipients bekommt — unerwünschte Kopplung.
- Alles im Mail-Modul: Der Endpoint gibt Member-Daten zurück, die im Mail-Modul fremd sind.

**Entscheidung:** Mail-Service liefert die Liste der nicht-erreichten Member-IDs, die REST-Schicht nutzt den Member-Service um die vollständigen Member-Daten zu laden. So bleibt die Modulgrenze sauber.

### 3. Frontend: Neues Dropdown in bestehender Filterliste

Ein zusätzliches Dropdown "Nicht erreicht durch Mail-Job" wird neben den bestehenden Filtern eingefügt. Wenn ein Job ausgewählt wird, ersetzt der API-Call die normale Mitgliederliste. Die übrigen Filter (Suche, aktiv etc.) arbeiten dann clientseitig auf dem gefilterten Ergebnis weiter.

## Risks / Trade-offs

- **Kopplung zwischen Mail- und Member-Modul auf REST-Ebene:** Der neue Endpoint muss beide Services ansprechen. → Mitigation: Nur die REST-Schicht kennt beide Services; die Service-Traits bleiben getrennt.
- **Performance bei vielen Mitgliedern/Recipients:** Die SQL-Query macht einen LEFT JOIN. → Mitigation: Bei der aktuellen Vereinsgröße kein Problem. Index auf `mail_recipients.mail_job_id` existiert implizit durch Foreign Key.
