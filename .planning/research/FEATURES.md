# Feature Research

**Domain:** GV-Anwesenheits-Erfassung in Genossenschafts-Mitgliederverwaltung (Helfer-QR-Login, reduzierte Liste, Suche, Anwesend-Markieren, Live-Counter)
**Researched:** 2026-05-01
**Confidence:** MEDIUM (Rechtsrahmen § 47 GenG HIGH; konkrete Verbands-Praxis MEDIUM; Wettbewerber-Feature-Beobachtungen MEDIUM, da kein direkter Test-Zugang)

## Scope-Klärung

Dies ist ein **Subsequent Milestone**. Member-Verwaltung, Audit-Hashchain, OIDC, Excel-Import, PDF/Typst, Email-Pipeline existieren bereits. Diese Recherche bewertet ausschließlich die **GV-Anwesenheits-Erfassung als Papierlisten-Ablöse**. Generische Mitgliederverwaltungs-Features sind absichtlich ausgeklammert.

Bezugsquellen für die Soll-Funktion:
- § 47 GenG (Niederschrift): Protokoll muss Anzahl der erschienenen oder vertretenen Mitglieder, Vorsitzender, Schriftführer und Stimmzähler benennen; eine Anwesenheits-/Teilnehmerliste ist als Anlage zu führen.
- Verbandspraxis (Genoverband, DGRV): Prüfungsverband nimmt beratend teil und kann die Niederschrift einsehen; die Anwesenheitsliste ist Teil der zu erhaltenden Versammlungsdokumente.
- Vergleichbare Software: easyQuorum (Wolters Kluwer), SEWOBE VereinsMANAGER, easyVerein, campai, DigitalCheckIn, LiteLog.

## Feature Landscape

### Table Stakes (Verbandskonformer Ersatz für Papier — ohne diese kein Go)

Features, ohne die das System nicht als Excel-/Papier-Ersatz für die GV-Anwesenheit gelten kann.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| GV als persistente Entität (Datum, Titel, Status: geplant/laufend/geschlossen) | § 47 GenG verlangt Datum, Ort, Form der Versammlung im Protokoll. Ohne eigene Entität gibt es kein nachvollziehbares Anker-Objekt für die Anwesenheitsliste pro Versammlung. | LOW | Standard-Entity nach Genossi-Pattern (id/created/deleted/version + Felder). Status-Lifecycle: planned → open → closed. Closed muss Sessions invalidieren und Liste einfrieren. |
| Vollständige Mitgliederliste pro GV (Soll-Teilnehmer-Universe) | Für Aussage „X von Y anwesend" muss Y eindeutig sein und zur GV gehören; Mitglieder, die zwischen GVs ein-/austreten, dürfen die Historie alter GVs nicht verändern. | MEDIUM | Snapshot der berechtigten Mitglieder zum Zeitpunkt GV-Eröffnung empfohlen, sonst verändern Member-Updates nachträglich Y. Alternative: berechnetes Y zum GV-Datum aus Member-Lifecycle. |
| Anwesend-Markieren / Anwesend-Zurücknehmen pro Mitglied | Die elementare Operation. „Zurücknehmen" ist nicht optional — Helfer hakt versehentlich falsche Person ab; ohne Korrektur wird die Liste falsch. | LOW | Idempotente Toggle-Operation; (member_id, assembly_id) ist natürlicher Unique-Key der Anwesenheits-Tabelle. |
| Suche in der Helfer-Liste (Name, Mitgliedsnummer) | Bei 200–2000 Mitgliedern ist Scrollen unbrauchbar. Helfer brauchen Sub-Sekunden-Filter. | LOW | Client-seitig im WASM-Frontend ausreichend; Liste lädt einmal beim Session-Start. Diakritik-/Umlaut-tolerant. |
| Reduzierte Datensicht für Helfer (nur Mitgliedsnummer, Name, Titel, Anrede) | Datenschutz/DSGVO: Helfer sind Externe oder Nicht-Vorstand, dürfen Adressen, Geburtstage, Anteilsdaten nicht sehen. Im Audit der Verbandsprüfung würde unkontrollierter Datenzugriff durch Helfer kritisiert. | LOW | Eigene reduzierte API-Route für Helfer-Sessions, nicht der bestehende Member-GET-Endpunkt. |
| Anzahl Anwesende im Protokoll-Export (mindestens als Zahl, idealerweise als Liste) | § 47 GenG: „Zahl der erschienenen oder vertretenen Mitglieder" gehört in die Niederschrift. Ohne maschinell exportierbare Zahl muss der Vorstand sie händisch aus der UI abschreiben — Fehlerquelle. | LOW | Reicht initial als JSON/CSV-Export oder als Datenfeld im bestehenden Typst-Protokoll-Template. |
| Persistenz nach GV-Schluss | Anwesenheits-Liste ist Teil der GV-Niederschrift und muss aufbewahrt werden (für Pflichtprüfung Genossenschaftsverband). | LOW | Soft-Delete-Pattern reicht; closed-Status verhindert weitere Schreibvorgänge. |
| Helfer-QR-Code Erzeugen (One-Time-Use, mit Memo-Name) | Kern der „Helfer ohne Account"-Idee aus Active Requirements. One-Time-Use ist explizite Constraint aus PROJECT.md. | MEDIUM | Token-Generierung kryptografisch sicher (>=128 bit); separate Tabelle `helper_invite` mit `consumed_at`. Memo-Name ist Freitext, kein Identitäts-Anker (vgl. Decision-Log). |
| Helfer-Session, gebunden an genau eine GV | Ohne Bindung an die GV ist nicht klar, wann die Session endet. PROJECT.md fordert explizit Auto-Invalidierung beim Schließen der GV. | MEDIUM | Bestehendes tower-sessions-Layer wiederverwenden, aber separater Session-Typ mit eingeschränkten Permissions; Key zu `assembly_id`. |
| Vorstand-direkt-Zugriff auf Helfer-View ohne QR | PROJECT.md Active: Vorstand kann „aus seiner regulären Anmeldung heraus öffnen". Sonst doppelt-UI / Fallback bei Ausfall. | LOW | UI-Route `/assemblies/:id/check-in` ist für authentifizierte Vorstand-Permission UND für Helfer-Session zugänglich. |
| Auto-Invalidierung aller Helfer-Sessions beim GV-Schluss | Sonst lebt Daten-Zugriff der Helfer nach der GV unkontrolliert weiter — Verband-Audit würde das mit Recht beanstanden. | LOW | DB-Cascade oder Session-Validation prüft `assembly.status == open`. |

### Differentiators (wo Genossi sich vom Status quo abhebt)

Features, die echten Mehrwert gegenüber Excel-Papier-Liste und älteren Tools bieten — passend zum Core Value „verbandskonform, weniger manuelle Arbeit".

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Live-Counter „X von Y anwesend" für Vorstand | Dashboard-Wert, der Vorständen während laufender GV einen Blick auf den Saal-Stand gibt; auf Papier nur durch nachträgliches Auszählen möglich. Direkte Erleichterung am GV-Tag. | LOW | Einfacher GET-Endpunkt; Refresh-on-poll im Frontend reicht (PROJECT.md schließt Live-Push explizit aus). |
| Parallele Helfer ohne zentrale Master-Liste | An einem großen GV-Eingang können 2-4 Helfer gleichzeitig abhaken, ohne dass Doppelabhaken-Probleme entstehen — weil jede Markierung idempotent ist (anwesend ist anwesend). Das löst ein echtes Papier-Problem (zwei Helfer auf einer Liste = chaotisch). | LOW | Idempotenz auf DB-Ebene durch UNIQUE(member_id, assembly_id); kein Locking, keine Konflikt-Behandlung nötig. PROJECT.md hat das bewusst so designt. |
| Helfer-Onboarding ohne Account in <30s | Vorstand druckt QR-Codes aus, gibt sie aus, Helfer scannt, ist drin. Kein Passwort, keine Mailadresse, keine Schulung. Bei einem 1x/Jahr-Event mit wechselnden Helfern ist das ein erheblicher Effizienz-Gewinn. | MEDIUM | Memo-Name am QR-Code ist die Genossi-spezifische UX-Verfeinerung: Vorstand weiß beim Drucken, welcher QR für wen ist, ohne dass dadurch Identität an die Session gebunden wird. |
| Audit-feste Mitglieder-Datenbasis (durch bestehendes Member-Audit-Log) | Der Verband prüft im Streitfall: war Person X am Stichtag wirklich Mitglied? Genossi hat bereits Hashchain auf Member; kombiniert mit dem GV-Snapshot ergibt das eine durchgehend belastbare Beweis-Kette für die GV-Niederschrift. | LOW (vorhanden) | Kein neues Feature, aber explizit nutzen: Snapshot referenziert Member-Versions; Audit-Log dokumentiert bestehende Member-Status-Änderungen. |
| Protokoll-Anhang automatisch generierbar (Anwesenheitsliste als PDF via Typst) | Bestehende Typst-Pipeline produziert druckbare Anlage zum GV-Protokoll. Direkter Verbands-Compliance-Wert: Niederschrift + Anwesenheits-Anlage in einem Schritt. | LOW | Neues Typst-Template; nutzt vorhandenen Generator. Bonus für v1, keine Pflicht (CSV-Export reicht zunächst). |

### Anti-Features (bewusst NICHT in v1 — mit Begründung)

Features, die auf den ersten Blick attraktiv wirken, aber Scope-Verletzung, Audit-Risiko oder DSGVO-Probleme bedeuten.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Stimmrechte / Stimmgewichte / Beschlussfähigkeit (Quorum) berechnen | „Wenn wir Anwesende zählen, können wir auch gleich Quorum prüfen." Tools wie easyQuorum tun das. | Genossi-Genossenschaft hat Stimmrecht 1 Mitglied = 1 Stimme (typisch), aber Quorum-Regeln stehen in der Satzung jeder Genossenschaft anders; Mehrfach-Anteile, Mehrstimmrechte für Genossenschafts-Mitglieder mit Investitions-Anteilen, Satzungsquorum vs. gesetzliches Quorum. Das wird ein eigenes Feature mit Satzungs-Modellierung — sprengt v1-Scope. PROJECT.md schließt das explizit aus. | Reine Anwesenheits-Zahl im Protokoll exportieren; Vorstand prüft Quorum manuell gegen Satzung. v2-Kandidat. |
| Vollmachts-Erfassung (X vertritt Y) | § 47 GenG kennt „erschienene oder vertretene" Mitglieder; Vollmacht ist genossenschafts-typisch. | Vollmacht erfordert: Vollmachts-Dokument-Upload, Gültigkeits-Prüfung, Ausschluss-Regeln (max. N Vollmachten pro Vertreter), Rechtsform der Vollmacht (schriftlich, elektronisch). Komplexität nicht unter 1–2 Phasen Aufwand. PROJECT.md schließt explizit aus. | Vorstand erfasst Vollmachten weiter auf Papier neben dem digitalen System; v2-Feature mit eigenem Milestone. |
| Identitäts-Verifikation per QR-Code | „Mitglied scannt QR und ist identifiziert" als Self-Service-Variante. | QR-Code als Identitäts-Beweis ist NICHT verbandskonform — er ist nur ein Bearer-Token, keine Authentifizierung des Mitglieds. Wer den QR hat, ist „angeblich" das Mitglied. Bei einer GV erwartet der Verband den Helfer-Sichtkontakt zum Mitglied (Personalausweis/Bekanntheit). | Helfer hakt selbst ab, nachdem er das Mitglied physisch identifiziert hat. Das ist Genossi-Scope. |
| Live-Push / WebSockets / SSE zwischen Helfern | „Live ist immer besser." | PROJECT.md Decision-Log: Doppelabhaken ist akzeptables Risiko (idempotent); SSE/WS bedeutet Connection-Management, Re-Connect-Logik, Out-of-Order-Events, deutlich höhere Komplexität ohne entsprechenden Nutzen für 1x/Jahr-Event. | Periodischer Refresh oder Refresh-on-Search; reicht für die Praxis vollkommen. |
| Audit-Hashchain auf jede Anwesenheits-Markierung | Genossi hat Hashchain für Member; Konsistenz-Argument „dann auch hier". | PROJECT.md Decision-Log: Verband fordert nur Anwesenheits-ZAHL im Protokoll, nicht den protokollierten Vorgang des Abhakens. Hashchain auf jeden Toggle würde die Größe und Komplexität der Anwesenheits-Tabelle ohne Verbands-Wert vervielfachen. | Anwesenheits-Liste ist transient zwischen Helfern, finalisiert beim GV-Schluss; Persistenz ab da, aber kein per-Field-Audit. |
| Handschriftliche elektronische Unterschrift pro Mitglied | „Papier-Unterschrift war Gewohnheit, also auch digital signieren." | Elektronische Signatur (qualifiziert nach § 126a BGB) erfordert Vertrauensdienstanbieter und ist deutlich überdimensioniert. Ein digitaler Check-in (Helfer markiert Anwesenheit) ersetzt rechtssicher die Unterschrift bei digitaler Erfassung. | Helfer-markiert-anwesend ist ausreichend; Verband akzeptiert digitalen Check-in als Anwesenheits-Beleg, sofern die Liste der erschienenen Mitglieder vollständig ist. |
| Online-Voting / virtuelle GV-Beschlussfassung | Trend-Thema seit Corona; § 43b GenG erlaubt virtuelle/hybride GV. | Voting ist eine eigene Software-Domäne (Stimmgeheimnis, Manipulationsschutz, Wahlleitung); POLYAS, easyQuorum etc. sind dafür spezialisiert. Genossi-Scope ist GV-Anwesenheit, nicht Beschlussfassung. | Externes Voting-Tool integrieren (oder physische Abstimmung beibehalten); Anwesenheit aus Genossi exportieren als Voter-Universe. |
| Native Mobile App für Helfer | „App-only" wirkt moderner. | PROJECT.md Out of Scope: Web-First. Native App = zwei zusätzliche Plattformen + App-Store-Reviews. Browser auf Tablet/Handy reicht für Scan-und-Liste-bedienen. | Responsive Web-UI auf Tablet/Handy, QR-Scanner via getUserMedia (BarcodeDetector API). |
| Helfer-zu-Helfer-Live-Chat / Notizfunktion | „Helfer wollen sich abstimmen." | Kommunikation läuft im Saal; Chat ist ein Software-Feature mit Moderation, Speicher-/DSGVO-Pflichten; nicht Genossi-Kern. | Helfer reden mündlich; Notizen pro Mitglied in v2 falls echter Bedarf entsteht. |
| Offline-Modus mit Sync | „Was wenn das WLAN ausfällt?" | PROJECT.md Out of Scope: Offline = Conflict-Resolution = große Komplexität. Bei einem 1-Tages-Event mit kontrollierter Saal-Infrastruktur kalkulierbares Risiko. | Vorstand sorgt für stabile Verbindung am Veranstaltungsort; Papier-Backup-Liste als Fallback im Worst Case (außerhalb des Tools). |

## Feature Dependencies

```
Assembly-Entity (CRUD)
    └──requires──> Status-Lifecycle (planned/open/closed)
                        └──requires──> Auto-Invalidate-Helper-Sessions-on-Close

Helper-Invite (One-Time-Use QR)
    └──requires──> Assembly-Entity (Invite ist an GV gebunden)
    └──requires──> Helper-Session-Type (Invite konsumieren erzeugt Session)
                        └──requires──> Reduzierte-Member-API (Helfer-Permission-Scope)

Anwesend-Markieren
    └──requires──> Assembly-Entity (status == open)
    └──requires──> Helper-Session ODER Vorstand-Auth
    └──requires──> Member-Universe-Snapshot (für stabiles Y)

Live-Counter "X von Y"
    └──requires──> Anwesend-Markieren (X)
    └──requires──> Member-Universe-Snapshot (Y)
    └──enhances──> Vorstand-UI (eigener Endpunkt)

Suche im Helfer-UI
    └──requires──> Reduzierte-Member-API
    └──enhances──> Anwesend-Markieren (Bedienbarkeit)

Protokoll-Export (Anzahl Anwesende)
    └──requires──> Persistenz nach GV-Schluss
    └──requires──> Anwesend-Markieren (vollständig)
    └──enhances──> bestehende Typst-Pipeline (kein neuer Stack)

Vorstand-Direkt-Zugriff (ohne QR)
    └──requires──> bestehender OIDC-Auth
    └──conflicts (semantisch nicht) ── ist UI-Route, die zwei Auth-Wege akzeptiert

[Stimmrechte/Vollmacht] ──conflicts──> [v1-Scope]
[Hashchain pro Markierung] ──conflicts──> [PROJECT-Decision: keine Audit-Pflicht für Anwesenheit]
```

### Dependency Notes

- **Assembly-Entity ist Wurzel:** Ohne sie gibt es keinen Scope für Helfer-Invites, Sessions oder Anwesenheits-Datensätze. Muss in der ersten Phase fertig sein.
- **Member-Universe-Snapshot ist nicht trivial:** Entweder Snapshot beim Öffnen der GV (zusätzliche Tabelle/View) oder On-the-Fly-Berechnung aus Member-Lifecycle zum GV-Datum. Tradeoff: Snapshot = Speicher + klare Historie; berechnet = weniger Daten, aber Member-Lifecycle muss stabil sein. Empfehlung: Snapshot-Liste, weil Vorstand am GV-Tag die Liste „einfrieren" möchte.
- **Helper-Session und Helper-Invite sind getrennt:** Invite = One-Time-Use-Token, lebt bis konsumiert oder GV geschlossen. Session = beim Konsum erzeugt, lebt bis GV geschlossen. Trennung, weil: Invite kann vor GV-Eröffnung erzeugt werden, Session erst beim Scan.
- **Live-Counter und Suche sind unabhängig** und können in unterschiedlichen Phasen gebaut werden, beide dependen aber auf der zugrundeliegenden Anwesenheits-Tabelle.
- **Protokoll-Export ist v1-Pflicht (mindestens Zahl)**, PDF-Anlage v1.x.

## MVP Definition

### Launch With (v1)

Minimum, um die Papier-/Excel-Liste auf einer GV vollständig zu ersetzen:

- [ ] **Assembly-Entität CRUD + Status-Lifecycle** — ohne kein verankerter Scope für Anwesenheit
- [ ] **Member-Universe-Snapshot beim GV-Öffnen** — für stabiles Y im Counter und im Protokoll
- [ ] **Helper-Invite mit Memo-Name + One-Time-Use** — Kern-UX für Helfer-Onboarding
- [ ] **Helper-Session, gebunden an Assembly, auto-invalidate beim Schließen** — Datenschutz-Pflicht
- [ ] **Reduzierte Helfer-Member-API (Mitgliedsnummer, Name, Titel, Anrede)** — DSGVO-Pflicht
- [ ] **Suche + Anwesend-Toggle in Helfer-UI** — die elementare Operation
- [ ] **Vorstand-Direkt-Zugriff ohne QR** — explizit in Active Requirements
- [ ] **Live-Counter „X von Y" für Vorstand** — Differentiator gegenüber Papier
- [ ] **Persistenz nach GV-Schluss** — Pflicht für Verbands-Niederschrift
- [ ] **Anzahl Anwesende und Anwesenheits-Liste exportierbar (CSV oder JSON)** — minimum für Protokoll

### Add After Validation (v1.x)

- [ ] **Anwesenheits-Liste als PDF-Anlage via Typst** — Auslöser: Vorstand verlangt druckbare Anlage zum Niederschrifts-PDF
- [ ] **Bulk-QR-Drucken (z. B. 20 QR-Codes auf eine Seite)** — Auslöser: Vorstand findet Einzeldruck umständlich
- [ ] **Re-Open einer GV (von closed zurück zu open)** — Auslöser: nachträgliche Korrektur in der Niederschrifts-Phase notwendig
- [ ] **Helper-Activity-Log pro Helfer-Session** — Auslöser: Vorstand möchte sehen, welcher Helfer wie viele markiert hat (NICHT im Verbands-Audit, rein UX)

### Future Consideration (v2+)

- [ ] **Vollmachts-Erfassung** — defer: eigener Komplexitäts-Block mit Dokument-Upload und Vertretungs-Regeln; eigener Milestone wert
- [ ] **Stimmrechte / Quorum-Berechnung** — defer: erfordert Satzungs-Modellierung, ist eigene Feature-Säule
- [ ] **Online-Voting** — defer: eigene Software-Domäne; ggf. Integration mit POLYAS oder ähnlich statt Eigenbau
- [ ] **Hybride/virtuelle GV** — defer: § 43b GenG erlaubt es, aber Streaming, Identifikations-Prüfung, virtuelle Wortmeldung sind weit jenseits Scope
- [ ] **Self-Check-in durch Mitglied per persönlichem QR-Code** — defer: erfordert Identitäts-Verifikation und ist anti-feature-nah, weil QR allein nicht authentifiziert

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Assembly-Entität + Status-Lifecycle | HIGH | LOW | P1 |
| Member-Universe-Snapshot | HIGH | MEDIUM | P1 |
| Helper-Invite (One-Time-Use, Memo) | HIGH | MEDIUM | P1 |
| Helper-Session + Auto-Invalidate | HIGH | MEDIUM | P1 |
| Reduzierte Helfer-Member-API | HIGH | LOW | P1 |
| Suche im Helfer-UI | HIGH | LOW | P1 |
| Anwesend-Toggle | HIGH | LOW | P1 |
| Vorstand-Direkt-Zugriff (ohne QR) | MEDIUM | LOW | P1 |
| Live-Counter „X von Y" | HIGH | LOW | P1 |
| Persistenz nach GV-Schluss | HIGH | LOW | P1 |
| CSV/JSON-Export der Anwesenheit | HIGH | LOW | P1 |
| Anwesenheits-PDF-Anlage (Typst) | MEDIUM | LOW | P2 |
| Bulk-QR-Drucken | MEDIUM | LOW | P2 |
| Re-Open einer GV | LOW | MEDIUM | P2 |
| Helper-Activity-Log | LOW | MEDIUM | P3 |
| Vollmachten | HIGH | HIGH | P3 (eigener Milestone) |
| Stimmrechte/Quorum | HIGH | HIGH | P3 (eigener Milestone) |
| Online-Voting | MEDIUM | HIGH | P3 (extern integrieren) |

**Priority key:**
- P1: Pflicht für v1-Launch — verbandskonformer Papier-Ersatz
- P2: Sollte folgen, sobald v1 stabil
- P3: Eigener Milestone oder bewusst extern

## Competitor Feature Analysis

| Feature | easyQuorum (Wolters Kluwer) | SEWOBE VereinsMANAGER | easyVerein | campai | Genossi-Ansatz |
|---------|---------|---------|---------|---------|--------------|
| QR-Check-in | Tablet + QR-Reader, hybrid-fähig | QR auf Eintrittskarte, Self-Check-in via App | QR auf digitalem Mitgliedsausweis | QR auf digitalem Mitgliedsausweis | One-Time-Use-QR pro Helfer (nicht pro Mitglied — anderes Mental-Modell, datensparsamer) |
| Helfer-Onboarding | Account-basiert | Account-basiert | Account-basiert | Account-basiert | Account-frei, Memo-Name am QR (UX-Differentiator für 1x/Jahr-Event) |
| Live-Counter / Quorum-Anzeige | Ja (mit Quorum-Berechnung) | Listen/Auswertungen | Live-Doku während Versammlung | Real-time „wie viele anwesend, Quorum erreicht" | Live-Counter „X von Y" — bewusst OHNE Quorum-Logik in v1 |
| Vollmacht-Verwaltung | Ja, mit elektronischer Vollmachts-Workflow | (nicht klar dokumentiert) | (nicht klar dokumentiert) | (nicht klar dokumentiert) | NICHT in v1 — eigener Milestone |
| Online-Voting | Ja, integriert | Nein im Kern, externe Tools | Nein im Kern | Nein im Kern | NICHT in v1 — Genossi ist Anwesenheits-Tool |
| Stimmgewichts-Anzeige für Helfer | Ja (in Voting-Kontext) | Nein | Nein | Nein | NICHT für Helfer (DSGVO + bewusst minimal) |
| DSGVO-reduzierte Helfer-Sicht | (nicht prominent) | (nicht prominent) | (nicht prominent) | „strukturiert und datenschutzkonform" | Explizit reduziert auf Mitgliedsnummer/Name/Titel/Anrede |
| Audit-Trail auf Markierungen | (nicht prominent) | (nicht prominent) | (nicht prominent) | (nicht prominent) | Bewusst NICHT — Verband fordert nur Zahl, nicht Markierungs-Vorgang |
| Parallele Helfer | (nicht prominent) | Mehrere Geräte/Scanner möglich | (nicht prominent) | (nicht prominent) | Parallele Helfer ohne Konflikt durch idempotente Toggles — explizites Design-Ziel |
| Protokoll-Anlage automatisch | Ja | „Anwesenheitsliste zur Unterschrift" | Live-Doku | „automatisch archiviert ans Protokoll" | Via bestehende Typst-Pipeline (v1.x) |

**Differenzierungs-Linie:** Genossi positioniert sich nicht gegen easyQuorum (das ist ein 360°-GV-Tool mit Voting), sondern als das, was eine kleine bis mittlere Genossenschaft tatsächlich braucht: Ablösung der Excel-Liste mit verbandskonformem Output, datensparsam, ohne Helfer-Account-Ballast, ohne Voting-Über-Engineering. Die anderen Tools sind entweder zu groß (easyQuorum) oder nicht genossenschafts-spezifisch (easyVerein, campai gehen vom Verein-Modell aus, GenG-Spezifika fehlen oft).

## Verbands-Perspektive (was prüft die Pflichtprüfung?)

Die genossenschaftliche Pflichtprüfung nach § 53 GenG prüft Wirtschaftslage und ordnungsmäßige Geschäftsführung mindestens alle zwei Jahre. Im Kontext der GV-Niederschrift achtet der Prüfungsverband auf:

1. **Vollständigkeit der Niederschrift nach § 47 GenG:** Datum, Ort, Versammlungsform, Vorsitzender, Schriftführer, Stimmzähler, Tagesordnung, Beschlüsse, **Zahl der erschienenen oder vertretenen Mitglieder**.
2. **Aufbewahrung:** Niederschrift und Anlagen (insbesondere Anwesenheitsliste) müssen erhalten und nachvollziehbar sein.
3. **Ordnungsgemäße Einberufung und Durchführung:** ist nachweisbar, dass nur berechtigte Mitglieder teilgenommen haben.
4. **Beratende Teilnahme:** Verbandsvertreter dürfen an jeder GV teilnehmen — sie sehen das System de facto live.

**Was Genossi v1 dafür liefert:** Anzahl Anwesende + Liste der anwesenden Mitglieder mit Mitgliedsnummer und Name, exportierbar; Member-Universe-Snapshot zum GV-Datum (so dass „berechtigt" nachvollziehbar bleibt); Persistenz nach GV-Schluss.

**Was Genossi v1 NICHT liefert (und das ist OK):** Vollmachts-Dokumentation, Stimmrechts-Tabelle, qualifiziertes Signatur-Verfahren auf der Liste. Diese Themen werden weiter über Vorstands-Verfahren neben Genossi gehandhabt oder in späteren Milestones adressiert.

**Mögliche Audit-Beanstandung, die wir vermeiden:** „Helfer hatten Zugriff auf Mitgliederdaten über die GV hinaus" (durch Auto-Invalidate gelöst); „Anwesenheits-Liste wurde nach der GV verändert" (durch closed-Status gelöst); „Y in ‚X von Y' ist nicht definiert" (durch Snapshot gelöst).

## Sources

- [§ 47 GenG — Niederschrift (dejure.org)](https://dejure.org/gesetze/GenG/47.html) — HIGH confidence, Originaltext
- [§ 47 GenG — gesetze-im-internet.de](https://www.gesetze-im-internet.de/geng/__47.html) — HIGH confidence
- [§ 53 GenG — Pflichtprüfung (dejure.org)](https://dejure.org/gesetze/GenG/53.html) — HIGH confidence
- [§ 43b GenG — Formen der Generalversammlung (Haufe)](https://www.haufe.de/id/norm/genossenschaftsgesetz-43b-formen-der-generalversammlung-HI768063_p43b.html) — HIGH confidence
- [Genoverband — Wie funktioniert eine Genossenschaft? Teil 2 (Dokumente)](https://www.genoverband.de/site/assets/files/30670/wie_funktioniert_eine_genossenschaft_teil_2.pdf) — MEDIUM confidence, Verbandspublikation
- [DGRV — Virtuelle General- und Vertreterversammlungen rechtssicher verankert](https://www.dgrv.de/news/virtuelle-general-und-vertreterversammlungen-rechtssicher-verankert/) — MEDIUM confidence
- [easyQuorum (Wolters Kluwer) — Cooperative-Lösung](https://www.wolterskluwer.com/de-de/solutions/easyquorum/cooperative) — MEDIUM confidence, Anbieter-Marketing
- [SEWOBE Veranstaltungsmanagement (QR-Anwesenheit)](https://www.sewobe.de/vereinssoftware/veranstaltungsmanagement/) — MEDIUM confidence
- [SEWOBE Gremienverwaltung](https://www.sewobe.de/vereinssoftware/gremienverwaltung/) — MEDIUM confidence
- [easyVerein — Mitgliederversammlungen](https://easyverein.com/vereinssoftware/mitgliederverwaltung/) — MEDIUM confidence
- [campai Anwesenheitsmodul](https://www.campai.com/funktionen/anwesenheitslisten) — MEDIUM confidence
- [campai-Akademie — Anwesenheitsliste im Verein (rechtssichere Umsetzung)](https://www.campai.com/de/akademie/anwesenheitsliste-im-verein) — MEDIUM confidence, Anbieter-Ratgeber, deckt sich aber mit Verbandspraxis
- [DigitalCheckIn — Funktionen](https://digitalcheckin.de/funktionen/) — LOW-MEDIUM confidence, Marketing
- [LiteLog — Digitale Anwesenheitsliste](https://litelog.de/de/features/digitale-anwesenheitsliste-app) — LOW-MEDIUM confidence
- [Genossenschaften.digital — Tools](https://genossenschaften.digital/ressourcen/digitalisierung-von-genossenschaften/tools) — MEDIUM confidence
- PROJECT.md (Genossi) — Active Requirements und Decision-Log; HIGH confidence (interne Quelle)

---
*Feature research for: GV-Anwesenheits-Erfassung als Papierlisten-Ablöse in Genossenschafts-Mitgliederverwaltung*
*Researched: 2026-05-01*
