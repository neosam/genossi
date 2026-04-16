#import "style.typ": conf, infobox, notebox, fieldtable, accent, muted, rule-color, heute-de

#show: doc => conf(
  title: "Revisionssicherheit des Audit-Logs",
  subtitle: "Technische Dokumentation für Prüfer und Betreiber",
  version: "1.0",
  date: heute-de(),
  author: "Genossi-Projekt",
  language: "de",
  doc,
)

= Zweck und Geltungsbereich

Dieses Dokument beschreibt die Mechanismen, mit denen die Software *Genossi* die
Nachvollziehbarkeit und Unveränderbarkeit schreibender Änderungen an
geschäftsrelevanten Daten sicherstellt. Es richtet sich an:

- *Prüfer* (insbesondere Genossenschaftsprüfer gemäß § 53 GenG), die sich einen
  Überblick über das Kontrollsystem verschaffen möchten, sowie
- *Betreiber* der Software, die den Nachweis gegenüber Dritten (Prüfungsverband,
  Aufsichtsrat, externe Revision) führen wollen.

Die Darstellung ist auf die für die Prüfung wesentlichen Aspekte beschränkt.
Detaillierte Implementierungshinweise und Entwickler-Informationen finden sich
in der Code-Dokumentation des Projekts.

== Zugrunde gelegte Normen

Die nachfolgend beschriebenen Mechanismen sind am Maßstab der folgenden
Regelwerke ausgerichtet:

#fieldtable(
  [*GoBD*],
  [Grundsätze zur ordnungsmäßigen Führung und Aufbewahrung von Büchern,
   Aufzeichnungen und Unterlagen in elektronischer Form sowie zum Datenzugriff
   (BMF-Schreiben)],

  [*§ 239 HGB*],
  [Anforderungen an die Führung von Handelsbüchern (u. a. Unveränderbarkeit,
   Nachvollziehbarkeit)],

  [*eIDAS-VO*],
  [Verordnung (EU) Nr. 910/2014, insbesondere Art. 41 und 42 (qualifizierte
   elektronische Zeitstempel)],

  [*RFC 3161*],
  [Internet X.509 Public Key Infrastructure Time-Stamp Protocol --- technisches
   Protokoll für den Austausch mit Zeitstempeldiensten],

  [*§ 53 GenG*],
  [Prüfungspflicht eingetragener Genossenschaften],
)

#v(0.6em)

#infobox(title: "Kernaussage")[
  Änderungen an geschützten Entitäten werden in einer kryptografisch verketteten
  Hash-Kette (SHA-256) protokolliert. In regelmäßigen Abständen wird das
  aktuelle Ende der Kette durch einen *qualifizierten elektronischen
  Zeitstempel* nach eIDAS an einen externen Vertrauensdienst gebunden. Damit
  lässt sich nachträgliche Veränderung oder Rückdatierung von Datensätzen
  nachweisbar ausschließen.
]

= Architekturüberblick

Die Audit-Funktion ist in mehrere Schichten gegliedert und nutzt ausschließlich
etablierte Standards (SHA-256, ISO 8601, RFC 3161):

#align(center, block(width: 100%, inset: 8pt, stroke: 0.4pt + rule-color, radius: 3pt)[
  #set text(size: 9pt, font: ("DejaVu Sans Mono", "Liberation Mono"))
```
                        Schreibende Geschäftsoperation
                       (Create / Update / Delete eines
                       Mitglieds, Antrags, Dokuments …)
                                     │
                                     ▼
              ┌────────────────────────────────────────────┐
              │  Audit-Makros                              │
              │  berechnen je geändertem Feld einen        │
              │  Eintrag, verketten per SHA-256-Hash       │
              │  mit dem letzten Eintrag der Tabelle.      │
              └────────────────────┬───────────────────────┘
                                   │
                                   ▼
              ┌────────────────────────────────────────────┐
              │  Tabelle audit_log                         │
              │  – eine Zeile pro geändertem Feld –        │
              │  Transaktions-ID verbindet zusammen-       │
              │  gehörige Feld-Änderungen einer Operation. │
              └────────────────────┬───────────────────────┘
                                   │
                                   ▼ (periodisch, durch Worker)
              ┌────────────────────────────────────────────┐
              │  Qualifizierter Zeitstempel (RFC 3161)     │
              │  Aktueller Kettenend-Hash wird an einen    │
              │  externen TSA-Dienst gesendet; das         │
              │  signierte TSR-Token wird gespeichert.     │
              └────────────────────┬───────────────────────┘
                                   │
                                   ▼
              ┌────────────────────────────────────────────┐
              │  Tabelle audit_timestamp                   │
              │  Ablage der Zeitstempel inkl. TSR-Token    │
              │  (optional zusätzlich über WebDAV          │
              │  extern gespiegelt).                       │
              └────────────────────────────────────────────┘
```
])

Die Trennung zwischen *innerer Integrität* (Hash-Kette) und *externer
Verankerung* (qualifizierter Zeitstempel) ist bewusst gewählt: Die Kette
erkennt jede nachträgliche Manipulation innerhalb der Datenbank; der
Zeitstempel verhindert, dass eine manipulierte Kette unbemerkt auf einen
konsistenten Zustand zu einem früheren Zeitpunkt zurückgerechnet werden kann.

= Audit-Log: Inhalte

== Protokollierte Entitäten

Geschützte Geschäftsdaten werden bei jeder schreibenden Operation protokolliert.
Aktuell sind folgende Entitätstypen erfasst:

- *Member* --- Mitgliedsdatensätze der Genossenschaft
- *MemberAction* --- an einem Mitglied durchgeführte Vorgänge
- *MemberDocument* --- einem Mitglied zugeordnete Dokumente
- *Application* --- Beitrittsanträge

Weitere Entitäten können durch Implementierung des internen `Auditable`-Merkmals
und Verwendung der zugehörigen Audit-Makros ergänzt werden. Der
Protokollierungsmechanismus ist für zukünftige Erweiterungen vorgesehen.

== Protokollierte Operationen

Erfasst werden die folgenden Operationstypen:

#fieldtable(
  [`create`], [Anlegen eines neuen Datensatzes],
  [`update`], [Änderung bestehender Feldwerte],
  [`delete`], [Löschung (als "weiches" Löschen über ein Lösch-Datum)],
  [`snapshot`], [Vollständige Abbildung eines Datensatzes
                 (administrative Nachdokumentation)],
)

== Felder eines Audit-Eintrags

Jede Zeile der Tabelle `audit_log` entspricht genau einem geänderten Feld eines
Datensatzes und enthält:

#fieldtable(
  [`id`],             [Eindeutige Kennung des Audit-Eintrags (UUID)],
  [`timestamp`],      [Zeitpunkt der Erfassung in UTC, ISO 8601],
  [`user_id`],        [Anmelde-Identität des auslösenden Benutzers
                       (bei Systemaktionen: `SYSTEM`)],
  [`process`],        [Bezeichnung des auslösenden Prozesses
                       (z. B. `member-service`, `audit-snapshot`)],
  [`transaction_id`], [UUID, die alle Feldänderungen *einer* Geschäftsoperation
                       zusammenfasst],
  [`entity_type`],    [Typ der betroffenen Entität (z. B. `member`)],
  [`entity_id`],      [UUID der betroffenen Entität],
  [`action`],         [`create` \/ `update` \/ `delete` \/ `snapshot`],
  [`field_name`],     [Name des geänderten Feldes],
  [`old_value`],      [Vorheriger Wert (bei `create` leer)],
  [`new_value`],      [Neuer Wert (bei `delete` leer)],
  [`prev_hash`],      [Hash-Wert des vorangehenden Audit-Eintrags],
  [`entry_hash`],     [Hash-Wert dieses Eintrags (siehe Abschnitt 4)],
)

#v(0.4em)

#infobox(title: "Granularität")[
  Pro geändertem Feld wird eine *eigene* Zeile geschrieben. Eine Operation, die
  drei Felder eines Mitglieds ändert, erzeugt drei Zeilen mit derselben
  `transaction_id`. Das ermöglicht eine feldgenaue Rekonstruktion der
  Änderungshistorie.
]

= Unveränderbarkeit durch Hash-Kette

== Prinzip

Die Audit-Einträge bilden eine lineare Kette: Der `entry_hash` eines Eintrags
geht in den `prev_hash` des *nächsten* Eintrags ein. Jede nachträgliche
Änderung an einem bereits geschriebenen Eintrag zerstört die Konsistenz aller
nachfolgenden Hashes und ist damit eindeutig erkennbar.

#v(0.4em)

#align(center, block(width: 100%, inset: 8pt, stroke: 0.4pt + rule-color, radius: 3pt)[
  #set text(size: 9pt, font: ("DejaVu Sans Mono", "Liberation Mono"))
```
  ┌────────────────┐      ┌────────────────┐      ┌────────────────┐
  │   Eintrag 1    │      │   Eintrag 2    │      │   Eintrag 3    │
  │ ──────────── 　│      │ ──────────── 　│      │ ──────────── 　│
  │ prev_hash  = ""│      │ prev_hash = h₁ │      │ prev_hash = h₂ │
  │ Felder …       │      │ Felder …       │      │ Felder …       │
  │ entry_hash= h₁ │━━━━━▶│ entry_hash= h₂ │━━━━━▶│ entry_hash= h₃ │
  └────────────────┘      └────────────────┘      └────────────────┘
```
])

#v(0.4em)

Eine Manipulation z. B. des `new_value` in Eintrag 2 ändert `h₂`. Da `h₂` aber
in `prev_hash` von Eintrag 3 festgeschrieben ist, wäre auch Eintrag 3 neu zu
berechnen --- und alle folgenden. Ohne gleichzeitige Änderung *aller*
späteren Einträge *und* der Zeitstempel-Einträge (siehe Abschnitt 5) wird jede
Manipulation sofort erkannt.

== Hash-Berechnung

Der `entry_hash` wird als SHA-256 über eine kanonische Repräsentation aller
Eintragsfelder gebildet. Eingehende Bestandteile in fester Reihenfolge:

#set enum(numbering: "1.")
+ Zeitstempel (ISO 8601, UTC)
+ Benutzer-Identität (`user_id`)
+ Prozess-Bezeichnung
+ Transaktions-UUID
+ Entitätstyp
+ Entitäts-UUID
+ Operation (`action`)
+ Feldname
+ Alter Wert
+ Neuer Wert
+ Hash des vorangehenden Eintrags (`prev_hash`)

Die Verwendung von SHA-256 entspricht dem aktuellen Stand der Technik für
kryptografische Hash-Funktionen und gilt nach Einschätzung des BSI als
sicher.

== Interne Verifikation

Die Software stellt eine Funktion `verify_chain` bereit, die für eine Menge
von Audit-Einträgen prüft:

- ob jeder Eintrag den im Datensatz gespeicherten `entry_hash` trägt, der sich
  aus den obigen Feldern tatsächlich neu berechnen lässt, und
- ob die `prev_hash`-Werte eine lückenlose Kette bilden.

Die REST-Schnittstelle `GET /api/audit/verify` führt diese Prüfung über die
gesamte Kette aus und meldet etwaige "broken links" (fehlerhaft verkettete
oder manipulierte Einträge).

= Qualifizierter elektronischer Zeitstempel

== Zweck

Die Hash-Kette allein beweist, dass keine *innere* Inkonsistenz vorliegt.
Ein Angreifer mit vollständigem Datenbankzugriff könnte theoretisch die
gesamte Kette inklusive aller Hashes neu berechnen. Um dies wirksam zu
verhindern, wird der aktuelle Kettenend-Hash in regelmäßigen Abständen an
einen *externen Vertrauensdiensteanbieter* gebunden.

Hierfür nutzt die Software das *Time-Stamp Protocol* nach RFC 3161. Der
externe Dienst (Time Stamping Authority, kurz TSA) signiert den eingereichten
Hash zusammen mit seinem eigenen Zeitsignal. Das resultierende
*TSR-Token* ist durch das Zertifikat des TSA rechtlich gesichert.

Wird eine TSA aus der *EU Trust List* (LOTL) verwendet, die als
qualifizierter Vertrauensdiensteanbieter nach eIDAS gelistet ist, sind die
erzeugten Zeitstempel *qualifizierte elektronische Zeitstempel* im Sinne
von Art. 42 eIDAS-VO. Sie genießen damit die unionsrechtliche
Beweisqualität einer vermuteten Richtigkeit von Zeit und Integrität der
signierten Daten.

#v(0.4em)

#infobox(title: "Rolle der TSA")[
  Die TSA sieht *keine* vertraulichen Daten. Übertragen wird nur ein Hashwert.
  Rückschlüsse auf die zugrundeliegenden Audit-Einträge sind damit
  ausgeschlossen. Die Einbindung einer externen TSA verletzt damit keine
  Geheimhaltungspflichten.
]

== Ablauf

#align(center, block(width: 100%, inset: 8pt, stroke: 0.4pt + rule-color, radius: 3pt)[
  #set text(size: 9pt, font: ("DejaVu Sans Mono", "Liberation Mono"))
```
   Genossi-Worker                     TSA (extern)                 Ablage
   ──────────────                     ────────────                 ──────
      │
      │ 1. letzter entry_hash
      │    der audit_log-Kette
      │
      │ 2. SHA-256(hash) ────────────▶│
      │                               │ 3. signiert mit
      │                               │    TSA-Zertifikat
      │ ◀───────── TSR-Token ─────────│
      │                               │
      │ 4. TSR-Token + audit_hash     │
      │    + Eintragsanzahl     ───────────────────────────▶  audit_timestamp
      │                               │
      │ 5. (optional) WebDAV-Upload ─────────────────────────▶  externer
      │                                                         Aufbewahrungsort
```
])

#v(0.4em)

Wichtig: *An keinem Punkt verlassen Klartextdaten das System*. Der TSA erhält
lediglich einen Hashwert.

== Intervall

Der Worker wird in einem durch den Betreiber konfigurierbaren Intervall
ausgelöst. Standardwert ist eine Woche (168 Stunden). Hat sich der
Kettenend-Hash seit dem letzten Zeitstempel nicht geändert (z. B. weil keine
Änderungen stattfanden), unterbleibt ein erneuter Aufruf des TSA, und es
werden keine Kosten verursacht. Vgl. Abschnitt 9 zur Konfiguration.

== Auswahl der TSA

Die konkrete Wahl der TSA erfolgt durch den Betreiber. Für die
prüfungsrelevante Qualifikation als *qualifizierter Zeitstempel* ist ein
Anbieter aus der EU Trust List erforderlich. Eine aktuelle Liste findet sich
unter:

#align(center, link("https://eidas.ec.europa.eu/efda/tl-browser/"))

Die konkret eingesetzte TSA einer Installation ist in der
*Betreiber-Dokumentation* einzutragen (siehe separate Vorlage
`template-betreiber.de.pdf`).

= Rollen und Zugriff

Das Audit-System trennt zwischen Lese- und Schreibrechten:

#fieldtable(
  [*Normale Benutzer*],
  [Lösen durch ihre Geschäftsvorgänge Audit-Einträge aus, können die
   Audit-Einträge selbst aber nicht einsehen, ändern oder löschen.],

  [*Administratoren*],
  [Können den vollständigen Audit-Log über die REST-Schnittstelle abrufen
   sowie die Integrität der Kette und der Zeitstempel prüfen. Sie können
   *keine* Einträge ändern oder löschen --- die Datenstruktur sieht dafür
   keine Operation vor.],

  [*Technische Systemprozesse*],
  [Der Zeitstempel-Worker arbeitet unter der Identität `SYSTEM` und darf
   ausschließlich *neue* Zeitstempel-Einträge schreiben.],
)

#v(0.4em)

#notebox(title: "Keine API zum Löschen")[
  Die Software stellt weder REST-Endpunkte noch Service-Methoden bereit, um
  einen Audit-Eintrag zu ändern oder zu entfernen. Nur ein administrativer
  Eingriff in die zugrundeliegende Datenbank könnte einen Eintrag
  manipulieren --- ein solcher Eingriff bricht jedoch die Hash-Kette und
  ist damit anhand der internen und externen Verifikation nachweisbar.
]

= Aufbewahrung und Export

== Datenbank

Audit-Log-Einträge und Zeitstempel werden in derselben Datenbank wie die
Geschäftsdaten abgelegt. Für die Datensicherung gelten damit dieselben
Regeln wie für die übrigen Geschäftsdaten (Backup-Konzept des Betreibers).

== Externe Spiegelung (optional)

Die Software bietet die Möglichkeit, TSR-Token nach erfolgreicher Erzeugung
automatisch an einen externen Speicher (WebDAV) zu übertragen. Damit liegt
mindestens ein vollständiger Zeitstempel-Beleg außerhalb des primären
Systems. Dies entspricht dem Grundgedanken der GoBD, nach dem "Beweismittel
außerhalb des Buchführungssystems" zusätzlichen Schutz gegen Manipulation
bieten.

Die Tabelle `audit_timestamp` hält für jede Zeitstempel-Transaktion neben den
Basisdaten (Zeitpunkt, zugehöriger Hash, Anzahl der abgedeckten Einträge, Status)
auch das rohe TSR-Token als binäres Datenfeld vor. Dieses Token kann zur
externen Verifikation jederzeit exportiert werden.

= Verifikation durch einen Prüfer

Ein Prüfer hat drei voneinander unabhängige Möglichkeiten, die Integrität der
Audit-Kette zu überprüfen. Die Unabhängigkeit der Werkzeuge ist dabei ein
bewusster Teil des Kontrollkonzepts.

== 1. Interne Verifikation

Über die Administrations-Oberfläche kann die Prüfung der Hash-Kette
ausgelöst werden (Aufruf von `GET /api/audit/verify`). Das System gibt zurück,
ob die Kette konsistent ist. Diese Prüfung ist jederzeit möglich und
verursacht keine Kosten.

== 2. Sichtprüfung der Zeitstempel-Einträge

Jeder Eintrag der Tabelle `audit_timestamp` zeigt:

- den Zeitpunkt, zu dem der Zeitstempel angefordert wurde,
- den zu diesem Zeitpunkt aktuellen Kettenend-Hash,
- die Anzahl der bis dahin erfassten Audit-Einträge und
- den Status (`success` bei erfolgreichem TSA-Rücklauf).

Über die Admin-Oberfläche können diese Daten eingesehen und als Nachweis
abgedruckt werden.

== 3. Externe Verifikation des TSR-Tokens

Für eine vom System unabhängige kryptografische Prüfung kann ein TSR-Token
exportiert und mit einem externen Werkzeug verifiziert werden. Empfohlen wird
die offizielle *DSS Demo Webanwendung der Europäischen Kommission*:

#align(center, link("https://ec.europa.eu/digital-building-blocks/DSS/webapp-demo/"))

Die Webanwendung prüft das Token gegen die EU Trust List (LOTL), verifiziert
die Signatur des TSA-Zertifikats und liefert ein formales Prüfprotokoll als
PDF. Das Ergebnis kann einem Prüfbericht unmittelbar beigelegt werden.

#infobox(title: "Ablauf einer externen Verifikation")[
  1. Ein aktueller Zeitstempel-Eintrag wird über die Admin-Oberfläche gewählt.
  2. Das zugehörige TSR-Token wird als Datei exportiert.
  3. Die Datei wird auf der DSS-Webanwendung hochgeladen.
  4. Die Anwendung zeigt Signaturstatus, Zertifikatskette und
     Zeitangabe und bietet ein signiertes Prüfprotokoll zum Download an.
]

= Konfiguration durch den Betreiber

Die Software trifft bewusst keine Annahme darüber, welcher
Vertrauensdiensteanbieter verwendet wird. Der Betreiber konfiguriert:

#fieldtable(
  [`tsa_enabled`],         [Aktivierung der Zeitstempel-Funktion (`true`/`false`)],
  [`tsa_url`],             [URL des TSA-Endpunkts (RFC 3161)],
  [`tsa_user` (optional)], [Benutzername bei HTTP Basic Auth],
  [`tsa_pass` (optional)], [Passwort bei HTTP Basic Auth],
  [`tsa_interval_hours`],  [Intervall zwischen Zeitstempel-Anforderungen],
)

Der konkrete Wert dieser Einstellungen für eine bestimmte Installation ist
Bestandteil der *Betreiber-Dokumentation*. Eine Vorlage steht dafür zur
Verfügung.

= Grenzen der Implementierung

Zu den Grenzen der aktuellen Implementierung gehört, dass die softwareeigene
`verify`-Funktion die kryptografische *Signatur* des TSR-Tokens nicht
eigenständig gegen das TSA-Zertifikat prüft; sie verifiziert stattdessen die
*Kettenkonsistenz* sowie die Parsebarkeit des Tokens (Statuswert der
TSA-Antwort).

Die eigentliche qualifizierte kryptografische Verifikation erfolgt bewusst
durch einen externen, unabhängigen Prüfpfad (siehe Abschnitt 8.3). Dies
entspricht dem in der Informationssicherheit üblichen Prinzip der
*Trennung von Prüf- und Produktionswerkzeugen*: Ein möglicherweise
manipuliertes System darf nicht gleichzeitig Produzent *und*
alleiniger Prüfer seiner eigenen Beweise sein.

#notebox(title: "Hinweis")[
  Für Prüfer: Diese Grenze ist transparent dokumentiert und architektonisch
  beabsichtigt. Der Nachweis der qualifizierten Signatur und damit der
  Rechtswirkung nach Art. 42 eIDAS-VO erfolgt über die externe DSS-Webanwendung
  der EU-Kommission.
]

= Anhang A --- Begriffe

#fieldtable(
  [*SHA-256*],
  [Standardisierte Hash-Funktion (FIPS 180-4). Erzeugt aus beliebigem Input
   einen 256-Bit-Fingerabdruck. Kollisionsresistent nach heutigem Stand der Technik.],

  [*RFC 3161*],
  [Internet-Standard für Zeitstempel-Dienste. Definiert Anforderungs-Format
   ("TimeStampReq") und Antwort-Format ("TimeStampResp" mit TSR-Token).],

  [*TSA*],
  [Time Stamping Authority --- Vertrauensdiensteanbieter, der nach RFC 3161
   Zeitstempel ausstellt.],

  [*TSR-Token*],
  [Time-Stamp Response Token. Von der TSA signiertes Datenpaket, das den
   eingereichten Hash, einen Zeitstempel und die Signatur der TSA enthält.],

  [*eIDAS-VO*],
  [EU-Verordnung Nr. 910/2014 über elektronische Identifizierung und
   Vertrauensdienste. Regelt u. a. qualifizierte elektronische Zeitstempel
   (Art. 41--42).],

  [*EU Trust List (LOTL)*],
  [Offizielle Liste der von den EU-Mitgliedstaaten notifizierten
   qualifizierten Vertrauensdiensteanbieter.],

  [*GoBD*],
  [Grundsätze zur ordnungsmäßigen Führung und Aufbewahrung von Büchern,
   Aufzeichnungen und Unterlagen in elektronischer Form. BMF-Schreiben,
   letzte Fassung vom 28. November 2019.],

  [*DSS*],
  [Digital Signature Service. Open-Source-Werkzeug der EU-Kommission zur
   Verifikation elektronischer Signaturen und Zeitstempel gegen die EU Trust List.],

  [*Hash-Kette*],
  [Datenstruktur, bei der jeder Eintrag einen Hash des vorherigen Eintrags
   enthält. Änderungen am Anfang der Kette machen alle nachfolgenden Hashes
   ungültig und sind dadurch erkennbar.],

  [*Transaktions-UUID*],
  [Eindeutige Kennung, die alle zu *einer* Geschäftsoperation gehörenden
   Audit-Einträge zusammenfasst, auch wenn jede Feldänderung als eigene Zeile
   gespeichert wird.],
)

= Anhang B --- Referenzen

- BMF: *Grundsätze zur ordnungsmäßigen Führung und Aufbewahrung von Büchern,
  Aufzeichnungen und Unterlagen in elektronischer Form sowie zum Datenzugriff
  (GoBD)*, Stand 28. November 2019.
- Verordnung (EU) Nr. 910/2014 des Europäischen Parlaments und des Rates vom
  23. Juli 2014 über elektronische Identifizierung und Vertrauensdienste für
  elektronische Transaktionen im Binnenmarkt (*eIDAS-Verordnung*).
- IETF: *RFC 3161 --- Internet X.509 Public Key Infrastructure Time-Stamp
  Protocol (TSP)*, August 2001.
- NIST: *FIPS PUB 180-4 --- Secure Hash Standard*, August 2015.
- Europäische Kommission: *DSS Demo Webanwendung*,
  #link("https://ec.europa.eu/digital-building-blocks/DSS/webapp-demo/").
- Europäische Kommission: *EU Trust List Browser*,
  #link("https://eidas.ec.europa.eu/efda/tl-browser/").
- Bundesgesetzblatt: *Genossenschaftsgesetz (GenG)*, insbesondere § 53
  (Prüfungspflicht).
