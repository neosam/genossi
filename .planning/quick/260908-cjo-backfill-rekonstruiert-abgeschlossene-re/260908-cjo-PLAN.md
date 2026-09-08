---
id: 260908-cjo
type: quick
description: "Backfill rekonstruiert abgeschlossene Repayment-Phasen: PaidOut-Eintraege einbeziehen, fiscal_year verfuegbar machen"
status: planned
date: 2026-09-08
depends_on: 260908-9ud
files_modified:
  - genossi_service_impl/src/repayment_context.rs
  - genossi_bin/src/lib.rs
  - genossi_bin/Cargo.toml
  - genossi_bin/tests/backfill_historic_repayment.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/component/mail_recipient_rendered_content.rs
---

# Quick Task 260908-cjo: Backfill rekonstruiert abgeschlossene Repayment-Phasen

## Problem

Der Startup-Backfill scheitert auf Produktion an 11 Alt-Mails. Die Diagnose aus
Quick 260908-9ud nennt die Ursache namentlich: `expr=fiscal_year` — eine
undefinierte Repayment-Variable im strict-Render.

Vom Nutzer verifiziert: die RepaymentPhase existiert, ist aber **abgeschlossen**;
alle Eintraege stehen auf `PaidOut`.

Verifizierte Kette (Zeilennummern am aktuellen Stand):

1. `aggregate_for_member` (`genossi_service_impl/src/repayment_context.rs:55-65`)
   filtert `matches!(e.status, Open | Contacted)`. `RepaymentEntryStatus` hat
   genau drei Varianten (`genossi_dao/src/repayment_entry.rs:16-21`).
2. Abgeschlossene Phase ⇒ alle Eintraege `PaidOut` ⇒ `relevant.is_empty()` ⇒
   `None` (Zeile 67-69).
3. `aggregate` macht daraus `ServiceError::EntityNotFound(member_id)` (Zeile 123).
4. `render.rs` laesst den Kontext daraufhin bewusst UNMERGED
   (`genossi_mail/src/render.rs:368-405`, `Err(ServiceError::EntityNotFound(_))`-Arm)
   ⇒ keine der vier Repayment-Variablen ist definiert ⇒ strict-Render bricht bei
   `fiscal_year` ab.

Die Daten sind vollstaendig da: `phase.fiscal_year`, `phase.share_value`,
`entry.share_count_to_pay_out`. Nur der Statusfilter blendet sie aus.

## Verifizierte Fakten

**F-1 — `find_by_phase_id` filtert NICHT nach Status.**
Die Default-Impl (`genossi_dao/src/repayment_entry.rs:138-150`) filtert
ausschliesslich `e.phase_id == phase_id && e.deleted.is_none()`. Die
SQLite-Impl (`genossi_dao_impl_sqlite/src/repayment_entry.rs`) ueberschreibt
`find_by_phase_id` **nicht** (grep: kein `async fn find_by_phase_id` ausserhalb
des Testmoduls). Die `PaidOut`-Eintraege erreichen `aggregate` also bereits
heute — sie werden erst dort verworfen. Es ist keine DAO-Aenderung noetig.

**F-2 — Der Trait ist die richtige Naht.**
`resolve_rendered_content` (`genossi_mail/src/render.rs:240-260`) nimmt
`repayment_context_resolver: &RCR` mit `RCR: RepaymentContextResolver<Transaction = RE::Transaction>`
generisch entgegen und ruft daraus **ausschliesslich** `aggregate`
(Zeile 368) — `resolve` wird auf diesem Pfad nie benutzt. Die gesamte
abweichende Semantik liegt damit hinter genau einer Trait-Methode.
`run_rendered_backfill` (`genossi_mail/src/backfill.rs:26-51`) ist ueber
dasselbe `RCR` generisch. Beide nehmen jede Implementierung entgegen: die
geteilte Render-Funktion bleibt **unveraendert**, es gibt keinen
Modus-Parameter, kein Flag und keinen zweiten Render-Pfad.

Damit ist der Ansatz aus dem Design-Constraint bestaetigt — mit einer
Praezisierung, siehe D-02 (die Semantik ist status-agnostisch, nicht
"PaidOut zusaetzlich").

**F-3 — `start_mail_worker` behaelt den strikten Resolver strukturell.**
`start_mail_worker` (`genossi_bin/src/lib.rs:1685`) liest
`self.repayment_context_resolver`. Das Feld ist auf
`Arc<RepaymentContextResolver>` typisiert (`:775`), also auf den strikten
Alias `RepaymentContextResolverImpl<RepaymentContextResolverDependencies>`
(`:386-389`). Dasselbe Feld speist auch
`RepaymentLetterServiceDependencies::RepaymentContextResolver` (`:416`) und
wird bei `RestStateImpl::new()` (`:1207-1219`) genau einmal gebaut. Der
historische Resolver wird **nie in ein Feld geschrieben**, sondern
ausschliesslich als lokale Variable in `start_rendered_backfill_worker`
konstruiert. Ein Austausch am Live-Pfad waere kein stiller Verhaltensdrift,
sondern eine Typaenderung an einem Feld, an dem zwei weitere Konsumenten
haengen.

**F-4 — Der Live-Pfad und der Backfill-Pfad teilen sich sonst alles.**
Beide rufen `resolve_rendered_content`; nur das 7. Argument unterscheidet sich
nach diesem Quick. `genossi_mail` wird von diesem Quick **nicht angefasst**
(keine Zeile). Das ist die staerkste Form der Zusicherung "der Live-Versandpfad
aendert sein Verhalten nicht", die hier erreichbar ist.

**F-5 — Der Rekonstruktions-Marker existiert bereits.**
`backfill.rs:101` setzt `rendered_reconstructed = true`; das Feld laeuft ueber
`genossi-frontend/src/api.rs:1018` in
`component/mail_recipient_rendered_content.rs:38` und rendert dort ein
Amber-Badge mit `Key::MailRenderedReconstructed`
("Nachträglich rekonstruiert" / "Reconstructed afterwards").

## Entscheidungen

**D-01 — Zweite Trait-Implementierung, injiziert nur im Backfill.**

Neu in `genossi_service_impl/src/repayment_context.rs`:
`HistoricRepaymentContextResolverImpl<Deps>` — dieselbe Struct-Form wie die
bestehende Impl (zwei DAO-`Arc`s, dieselbe `RepaymentContextResolverDeps`),
aber mit historischer Aggregations-Semantik. Injiziert wird sie ausschliesslich
in `start_rendered_backfill_worker`.

Begruendung gegen die Alternativen:
- **Modus-Parameter durch `resolve_rendered_content`:** wuerde die bewusst DRY
  gehaltene Funktion (Quick 260614-b1t) mit einer Fallunterscheidung
  durchziehen, die es genau einmal, ganz am Ende, im `aggregate`-Aufruf gibt.
  Jeder kuenftige Leser muesste den Modus mitdenken.
- **Flag auf `RepaymentContextResolverImpl`:** aendert die Struct-Literale in
  `genossi_bin` und damit auch die Konstruktion fuer den Letter-Service — der
  Live-Pfad bekaeme ein Feld, das ihn nichts angeht, und die Absicherung waere
  ein Default-Wert statt eines Typs.
- **Filter global lockern:** genau der gefaehrliche Fix aus dem
  Design-Constraint. Ausgeschlossen.

Der Preis: `resolve` muss in beiden Impls existieren. Damit die DAO-Ladelogik
nicht driftet, wandert sie in eine geteilte private `async fn`
`load_phase_and_entries(...)`, die beide `resolve`-Impls aufrufen. Nur die
anschliessende Aggregation unterscheidet sich.

**D-02 — Historische Semantik ist STATUS-AGNOSTISCH, nicht "PaidOut zusaetzlich".**

`aggregate_for_member_historic` summiert **alle nicht-geloeschten Eintraege des
Mitglieds in der Phase**, unabhaengig vom Status. Die uebrigen Filter
(`deleted.is_none()`, `member_id`) bleiben unveraendert.

Warum nicht "nur wenn kein Open/Contacted existiert, dann auch PaidOut":
Diese minimal-invasive Variante wuerde bei gemischten Eintraegen
(z.B. `Open(2)` + `PaidOut(3)`) weiterhin nur `2` liefern — obwohl die
urspruengliche Mail mit hoher Wahrscheinlichkeit `5` genannt hat, weil
Eintraege monoton `Open → Contacted → PaidOut` wandern und zum Versandzeitpunkt
noch nicht ausgezahlt waren. Die Regel waere ausserdem unstetig: 1 Open + 5
PaidOut ergaebe `1`, 0 Open + 5 PaidOut ergaebe `5`. Status-agnostisch ist die
einzige Regel, die "der Eintrags-Satz des Mitglieds in dieser Phase" sauber
ausdrueckt.

Restrisiko, das damit bewusst eingegangen wird: wurde ein Eintrag **nach** dem
Mailversand angelegt oder war er **schon vor** dem Versand `PaidOut`, zaehlt er
jetzt mit, obwohl er in der Originalmail nicht stand. Das ist nicht
rekonstruierbar — der Statuswechsel-Zeitpunkt wird nirgends gespeichert
(`RepaymentEntryEntity` hat nur `created`), und `share_count_to_pay_out`-
Aenderungen sind ebenfalls nicht historisiert. Siehe Ehrlichkeits-Abschnitt.

Eine Zeitschranke `entry.created <= job.created` wurde geprueft und verworfen:
sie wuerde einen Zeitstempel durch die `aggregate`-Trait-Signatur schleusen
(und damit auch den Letter-Service-Caller anfassen), waere aber trotzdem nur
eine Teilloesung, weil der Statuswechsel — nicht die Erzeugung — die
entscheidende, unbekannte Groesse ist.

**D-03 — `fiscal_year` wird NICHT aus dem Aggregat herausgeloest.**

Der im Auftrag benannte zweite Designfehler ist real, aber sein Fix ist hier
ohne Nutzen und mit einem Risiko verbunden:

- **Ohne Nutzen:** die historische Aggregation liefert `share_count`,
  `payout_amount` und `fiscal_year` in einem Rutsch zurueck; `share_value`
  kommt ohnehin aus `phase.share_value` direkt in `render.rs:363`. Nach D-01
  sind alle vier Variablen im Backfill definiert. Ein separater
  `fiscal_year`-Pfad waere eine zweite Quelle fuer denselben Wert.
- **Mit Risiko:** `fiscal_year` unabhaengig vom Aggregat zu mergen wuerde auch
  den LIVE-Pfad aendern. Ein Template, das heute beim Live-Versand an
  `fiscal_year` scheitert (und den Empfaenger korrekt als failed markiert),
  wuerde danach stillschweigend durchgehen und eine Mail ohne Betrag
  verschicken. Das verletzt die harte Grenze.

Der Umbau gehoert — wenn ueberhaupt — in einen eigenen Vorgang mit eigener
Live-Pfad-Bewertung. Hier: bewusst weggelassen.

**D-04 — `genossi_mail` wird nicht angefasst.**

Kein Diff in `render.rs`, `backfill.rs`, `worker.rs`, `template.rs`. Die
Verhaltensaenderung entsteht ausschliesslich durch das Argument, das
`genossi_bin` uebergibt. Alle bestehenden `genossi_mail`-Tests (inklusive der
drei Quick-260908-9ud-Regressionstests) bleiben unveraendert gruen.

**D-05 — Kennzeichnung: `rendered_reconstructed` bleibt der Marker, bekommt
aber eine Erklaerung.**

Kein zweites DB-Feld, keine Migration. Begruendung:

- `rendered_reconstructed = true` sagt bereits die richtige Sache: "das ist
  nicht der byte-genaue Originaltext des Versands". Fuer **jede** vom Backfill
  gefuellte Zeile — auch die aus offenen Phasen — gilt, dass die Betraege aus
  dem heutigen Datenstand neu berechnet wurden. Ein zweiter Marker wuerde eine
  Unterscheidung suggerieren, die es auf der Ebene "wie verlaesslich ist die
  Zahl?" nicht gibt.
- Was fehlt, ist kein staerkerer Marker, sondern eine **Erklaerung**. Das
  Badge-Label "Nachträglich rekonstruiert" liest sich heute wie "aus einem
  Archiv wiederhergestellt". Es bekommt deshalb einen Tooltip
  (`title`-Attribut, ein neuer i18n-Key), der explizit sagt, dass Betraege und
  Anteilszahlen aus dem heutigen Datenstand stammen und vom versendeten Text
  abweichen koennen. Kosten: ein Key, zwei Locale-Arme, ein Attribut.
- Zusaetzlich meldet der historische Resolver beim Backfill pro betroffenem
  Mitglied ein `tracing::warn!`, wenn mindestens ein summierter Eintrag nicht
  mehr `Open`/`Contacted` ist (`diverges_from_live`). Damit steht im
  Startup-Log nachvollziehbar, **welche** Zeilen aus einer abgeschlossenen
  Phase rekonstruiert wurden — die Information, die in der DB bewusst nicht
  landet.

**D-06 — Datenschutz: das neue Log enthaelt keine Betraege.**

Das `warn!` aus D-05 loggt `phase_id`, `member_id`, `entry_count` und
`stale_status_count` — Ids und Anzahlen, keine Geldwerte. `payout_amount` und
`share_count` bleiben draussen, exakt wie in Quick 260908-9ud D-02 festgelegt
(Server-Logs sind weder zugriffsbeschraenkt noch loeschfristen-verwaltet).
`entry_count` wird an derselben Stelle bereits heute geloggt
(`render.rs:389`), ist also keine neue Kategorie.

## Ehrlichkeit: was diese Rekonstruktion NICHT kann

Die rekonstruierten Werte werden mit dem **heutigen** `share_count_to_pay_out`
und dem heutigen `phase.share_value` berechnet, nicht mit den Werten vom
Versandzeitpunkt. Solange sich daran nichts geaendert hat, kommt die
urspruengliche Zahl heraus — sonst nicht, und man sieht es der Mail nicht an.

Konkret unsichtbar bleiben drei Faelle:

1. `share_value` der Phase wurde nach dem Versand korrigiert → `payout_amount`
   weicht ab.
2. `share_count_to_pay_out` eines Eintrags wurde nach dem Versand geaendert →
   `share_count` und `payout_amount` weichen ab.
3. Ein Eintrag wurde nach dem Versand angelegt, oder war zum Versandzeitpunkt
   schon `PaidOut` → er zaehlt jetzt mit, obwohl er in der Originalmail nicht
   stand (D-02-Restrisiko).

Keiner der drei Faelle ist aus den vorhandenen Daten erkennbar; es gibt keine
Historisierung der Betragsfelder. Was wir liefern koennen und liefern, ist:

- Das Amber-Badge kennzeichnet die Zeile weiterhin als Rekonstruktion (F-5).
- Der Badge-Tooltip sagt jetzt ausdruecklich, dass die Zahlen aus dem heutigen
  Datenstand kommen (D-05).
- Das Startup-Log nennt jede Zeile, deren Rekonstruktion nur ueber
  nicht-mehr-offene Eintraege zustande kam (D-05/D-06).

Was wir bewusst NICHT tun: so tun, als waere die Zahl belegbar identisch mit
der versendeten. Sie ist die bestmoegliche Rekonstruktion, nicht das Original.

## Sicherheit / Datenschutz

| Risiko | Bewertung | Massnahme |
|--------|-----------|-----------|
| Auszahlungsmail geht an bereits ausgezahltes Mitglied | kritisch | Live-Pfad unveraendert (D-04, F-3, F-4); Regressionstest im Live-Pfad (Task 2) und auf Aggregations-Ebene (Task 1) |
| Historischer Resolver leckt in den Live-Pfad | hoch | Neuer Typ wird nie in ein `RestStateImpl`-Feld geschrieben; TypeId-Test pinnt die Trennung (Task 2) |
| Mitglieds-Betraege im Server-Log | mittel | D-06: nur Ids + Anzahlen |
| Neue Supply-Chain-Flaeche | keine | Kein neues Crate. `mockall` (dev-dep `genossi_bin`) ist bereits Workspace-Dependency und in `Cargo.lock` |

## Tasks

### Task 1 — Historische Aggregation + zweiter Resolver

**files:** `genossi_service_impl/src/repayment_context.rs`

**action:**

- Gemeinsamen Summenkern herausziehen, damit die Euro-Formatierung nur an
  einer Stelle lebt: private
  `fn context_from_relevant(phase: &RepaymentPhaseEntity, relevant: &[&RepaymentEntryEntity]) -> RepaymentContext`
  mit dem heutigen Inhalt von Zeile 71-80 (SUM `share_count_to_pay_out`,
  `cents = share_count as i64 * phase.share_value`,
  `format!("{},{:02}", cents / 100, cents % 100)`, `phase.fiscal_year`).
- `aggregate_for_member` (Zeile 50) behaelt Signatur, Doc-Kommentar und
  Filterausdruck **woertlich** und ruft am Ende nur noch
  `context_from_relevant`. Kein Verhaltensdiff — die zehn bestehenden
  Unit-Tests (Zeile 188-289) pinnen das.
- Neuer oeffentlicher Rueckgabetyp, damit die Abweichung testbar ist ohne
  `tracing-subscriber`-dev-dep:

  `pub struct HistoricAggregate { pub context: RepaymentContext, pub entry_count: usize, pub stale_status_count: usize, pub diverges_from_live: bool }`

  `diverges_from_live` ist `stale_status_count > 0`, also: mindestens ein
  summierter Eintrag ist nicht mehr `Open`/`Contacted`, die strikte
  Aggregation haette also eine andere (kleinere) Zahl oder `None` geliefert.
- Neue Funktion
  `pub fn aggregate_for_member_historic(phase, entries, member_id) -> Option<HistoricAggregate>`:
  filtert `e.deleted.is_none() && e.member_id == member_id` — **ohne**
  Status-Praedikat (D-02); leer ⇒ `None`; sonst `context_from_relevant` plus
  die drei Zaehler.
- Geteilte DAO-Ladelogik: private
  `async fn load_phase_and_entries<PD, ED, T>(phase_dao: &PD, entry_dao: &ED, phase_id: Uuid, tx: T) -> Result<(RepaymentPhaseEntity, Vec<RepaymentEntryEntity>), ServiceError>`
  mit `T: Transaction`, `PD: RepaymentPhaseDao<Transaction = T>`,
  `ED: RepaymentEntryDao<Transaction = T>`. Inhalt = heutige `resolve`-Schritte
  1 und 2 (Zeile 95-110), inklusive
  `.ok_or(ServiceError::EntityNotFound(phase_id))`.
  Das bestehende `resolve` (Zeile 89) wird darauf umgestellt und delegiert
  danach unveraendert an `self.aggregate`.
- Neue Struct
  `pub struct HistoricRepaymentContextResolverImpl<Deps: RepaymentContextResolverDeps>`
  mit denselben zwei `pub`-Feldern wie die bestehende Impl (gleiche
  `Deps`-Bounds, damit `genossi_bin` denselben
  `RepaymentContextResolverDependencies`-Typ wiederverwenden kann).
  Modul-Doc-Kommentar an der Struct: nur fuer den Startup-Backfill
  (`run_rendered_backfill`), NIE fuer `start_mail_worker` oder den
  Letter-Service; mit einer Zeile Begruendung (der Open/Contacted-Filter
  verhindert im Live-Versand die Mail an bereits Ausgezahlte).
- `impl RepaymentContextResolver for HistoricRepaymentContextResolverImpl`:
  - `resolve`: `load_phase_and_entries(...)` → `self.aggregate(...)`.
  - `aggregate`: `aggregate_for_member_historic(...)`; bei `None` →
    `Err(ServiceError::EntityNotFound(member_id))` (identische
    Fehlerkonvention wie strikt); bei `Some(h)` → wenn
    `h.diverges_from_live`, ein `tracing::warn!` mit den Feldern
    `phase_id = %phase.id`, `member_id = %member_id`,
    `entry_count = h.entry_count`, `stale_status_count = h.stale_status_count`
    und einer Meldung, die sagt: historische Rekonstruktion aus nicht mehr
    offenen Eintraegen, Betraege stammen aus dem heutigen Datenstand und
    koennen vom versendeten Text abweichen. **Keine** Betragsfelder (D-06).
    Rueckgabe `Ok(h.context)`.

**Tests** (im vorhandenen `mod tests` derselben Datei, Fixtures
`sample_phase`/`sample_entry` ab Zeile 142 wiederverwenden):

- `historic_aggregate_includes_paid_out_only` — ein `PaidOut`-Eintrag,
  5 Anteile, `share_value` 12000: `Some`, `share_count == 5`,
  `payout_amount == "600,00"`, `fiscal_year == 2025`,
  `diverges_from_live == true`, `stale_status_count == 1`.
  **Das ist der Wertbeweis dieses Quicks auf Aggregations-Ebene.**
- `historic_aggregate_mixed_statuses_sums_all` — `Open(2)` + `PaidOut(3)` ⇒
  `share_count == 5`, `diverges_from_live == true`. Steht bewusst direkt neben
  dem bestehenden `test_aggregate_filters_paid_out` (Zeile 211), das fuer
  denselben Input `3` behauptet — die beiden Tests dokumentieren die
  Semantik-Differenz als Absicht (D-02).
- `historic_aggregate_open_only_equals_strict` — nur `Open(3)`: der
  `context` ist `assert_eq!`-gleich zum Ergebnis von `aggregate_for_member`,
  und `diverges_from_live == false`.
- `historic_aggregate_still_filters_soft_deleted` — soft-deleted
  `PaidOut`-Eintrag ⇒ `None`.
- `historic_aggregate_cross_member_isolation` — Eintrag eines anderen
  Mitglieds ⇒ `None`.
- `historic_aggregate_empty_returns_none`.
- `historic_trait_aggregate_ok_for_paid_out_only` — ueber
  `HistoricRepaymentContextResolverImpl::aggregate` (mit
  `MockTestPhaseDao`/`MockTestEntryDao` ohne Erwartungen, die DAOs werden von
  `aggregate` nicht angefasst): `Ok`.
- `strict_trait_aggregate_still_errs_for_paid_out_only` — identischer Input
  ueber `RepaymentContextResolverImpl::aggregate`:
  `Err(ServiceError::EntityNotFound(member_id))`. Regressionsgate fuer den
  Live-Pfad auf Aggregations-Ebene.
- `historic_resolve_loads_and_aggregates` — `#[tokio::test]` nach dem Muster
  von `test_resolve_happy_path` (Zeile 423): Phase gefunden, nur
  `PaidOut`-Eintraege ⇒ `Ok`, `fiscal_year` gesetzt.
- `historic_resolve_phase_not_found` — Phase-DAO liefert `None` ⇒
  `Err(EntityNotFound(phase_id))`, Entry-DAO wird nicht aufgerufen (keine
  Erwartung gesetzt). Belegt, dass `load_phase_and_entries` fuer beide Impls
  identisch fehlerbehaftet ist.

**verify:** `nix develop --command cargo test -p genossi_service_impl repayment_context::`

**done:** Eine abgeschlossene Phase mit ausschliesslich `PaidOut`-Eintraegen
liefert ueber den historischen Resolver einen vollstaendigen
`RepaymentContext`; die strikte Impl liefert fuer denselben Input weiterhin
`EntityNotFound`.

### Task 2 — Historischen Resolver ausschliesslich im Backfill injizieren

**files:** `genossi_bin/src/lib.rs`, `genossi_bin/Cargo.toml`,
`genossi_bin/tests/backfill_historic_repayment.rs` (neu)

**action:**

- Direkt unter dem bestehenden Alias (`genossi_bin/src/lib.rs:386-389`) einen
  zweiten Alias ergaenzen:

  `type HistoricRepaymentContextResolver = genossi_service_impl::repayment_context::HistoricRepaymentContextResolverImpl<RepaymentContextResolverDependencies>;`

  Derselbe `Deps`-Typ wie der strikte Alias — kein zweiter
  `...Dependencies`-Marker noetig. Kommentar: ausschliesslich fuer
  `start_rendered_backfill_worker`; kein `RestStateImpl`-Feld (F-3).
- In `start_rendered_backfill_worker` (`:1724`) die Zeile
  `let repayment_context_resolver = self.repayment_context_resolver.clone();`
  ersetzen durch die lokale Konstruktion:

  `let historic_repayment_context_resolver = Arc::new(HistoricRepaymentContextResolver { repayment_phase_dao: self.repayment_phase_dao.clone(), repayment_entry_dao: self.repayment_entry_dao.clone() });`

  Es sind exakt dieselben zwei `Arc`s, aus denen `RestStateImpl::new()` den
  strikten Resolver baut (`:1207-1219`) — kein neuer DAO-Konstruktor
  (Single-Arc-per-Process). Das Binding wird als 7. Positionsargument an
  `run_rendered_backfill` uebergeben; alle uebrigen acht Argumente bleiben
  unveraendert.
- Kommentarblock ueber der Konstruktion mit der harten Grenze: der
  Open/Contacted-Filter ist beim Live-Versand fachlich richtig und bleibt dort;
  der Backfill rekonstruiert historische Mails und muss `PaidOut` einbeziehen;
  die Trennung laeuft ueber den Trait, nicht ueber ein Flag (D-01/F-2);
  Verweis auf diesen Quick.
- Einzeiliger Kommentar in `start_mail_worker` an
  `let repayment_context_resolver = self.repayment_context_resolver.clone();`
  (`:1685`): muss der strikte Resolver bleiben; hier NIE den historischen
  einsetzen, mit Verweis auf diesen Quick.
- `genossi_bin/Cargo.toml`: `mockall = { workspace = true }` in
  `[dev-dependencies]`. Bereits Workspace-Dependency und in `Cargo.lock` — kein
  neues Crate.

**Tests** (neue Datei `genossi_bin/tests/backfill_historic_repayment.rs`):

Aufbau: der Test ruft die **echte** geteilte Render-Funktion
`genossi_mail::render::resolve_rendered_content` mit den **echten**
Resolver-Impls; nur die DAOs/Resolver-Traits ringsherum sind Mocks. Kein
SQLite-Pool noetig, weil `aggregate` keine DB anfasst.

Fixtures (analog `genossi_mail/src/render.rs:653-742` — dort privat, hier neu
angelegt):
- `make_member()`, `make_recipient(Some(member.id))`,
  `make_job(subject, body, Some(phase_id))` mit
  `genossi_mail::dao::{MailJob, MailRecipient}` (beide `pub`).
- `make_closed_phase(fiscal_year, share_value)` mit
  `RepaymentPhaseStatus::Closed` — die Produktionslage.
- `paid_out_entry(phase_id, member_id, share_count)`.
- `clonable_tx()`-Helfer nach dem Muster
  `genossi_mail/src/render.rs:1414-1420` (rekursives `expect_clone`).
- Lokale `struct TestDeps;` mit
  `impl genossi_service_impl::repayment_context::RepaymentContextResolverDeps`:
  `Transaction = genossi_dao::MockTransaction`,
  `RepaymentPhaseDao = genossi_dao::repayment_phase::MockRepaymentPhaseDao`,
  `RepaymentEntryDao = genossi_dao::repayment_entry::MockRepaymentEntryDao`.
  Die Resolver-Instanzen bekommen frische Mock-DAOs ohne Erwartungen — ihre
  DAO-Felder werden auf dem `aggregate`-Pfad nicht benutzt; Phase und Entries
  laedt `render.rs` selbst ueber die separat gemockten DAOs.
- Template-Body exakt in Produktions-Form, `fiscal_year` **zuerst**, damit der
  strict-Render an derselben Variable scheitert wie auf Produktion:
  `"Geschaeftsjahr {{ fiscal_year }}: {{ share_count }} Anteile zu je {{ share_value }} EUR = {{ payout_amount }} EUR"`.

Tests:
- `backfill_resolver_renders_closed_phase_with_paid_out_entries_only` —
  `HistoricRepaymentContextResolverImpl::<TestDeps>`, Phase `Closed`,
  `fiscal_year = 2024`, `share_value = 12000`, ein `PaidOut`-Eintrag mit
  5 Anteilen. Erwartung: `Ok`, und
  `rendered.body == "Geschaeftsjahr 2024: 5 Anteile zu je 120,00 EUR = 600,00 EUR"`.
  **Pflichttest (a): abgeschlossene Phase, nur PaidOut, Backfill-Pfad liefert
  vollstaendigen Kontext und das Template rendert.**
- `live_resolver_still_fails_on_closed_phase_with_paid_out_entries_only` —
  identische Fixtures, nur `RepaymentContextResolverImpl::<TestDeps>` statt der
  historischen Impl. Erwartung: `Err`, `failure.message` beginnt mit
  `"Template render error (body): "`, und `failure.diagnosis` (Quick
  260908-9ud) enthaelt `"fiscal_year"` — also exakt das Produktionssymptom.
  **Pflichttest (b): derselbe Fall scheitert im Live-Pfad weiterhin.**
- `live_resolver_still_fails_for_mixed_open_and_paid_out_amounts` — Phase
  `Closed`, `Open(2)` + `PaidOut(3)`, Template nur `{{ share_count }}`.
  Erwartung: Live-Resolver rendert `2`, historischer Resolver rendert `5`.
  Pinnt D-02 an der Stelle, an der es weh tut, und zeigt die Divergenz
  explizit statt sie zu verstecken.
- `live_and_backfill_resolver_types_are_distinct` — `std::any::TypeId`-
  Vergleich der beiden `genossi_bin`-Aliase. Billiges Gate dagegen, dass ein
  spaeterer Refactor die Aliase auf denselben Typ zusammenfallen laesst.

**verify:** `nix develop --command cargo test -p genossi_bin --test backfill_historic_repayment`

**done:** Der Backfill-Pfad rendert die Produktionslage vollstaendig; der
Live-Pfad scheitert an derselben Lage unveraendert mit derselben Meldung.

### Task 3 — Ehrlichkeits-Hinweis am Rekonstruktions-Badge

**files:** `genossi-frontend/src/i18n/mod.rs`, `genossi-frontend/src/i18n/de.rs`,
`genossi-frontend/src/i18n/en.rs`,
`genossi-frontend/src/component/mail_recipient_rendered_content.rs`

**action:**

- Neuer Key `MailRenderedReconstructedHint` im `Key`-Enum
  (`i18n/mod.rs`, direkt neben `MailRenderedReconstructed`, Zeile 247).
- `de.rs` (neben Zeile 188): sinngemaess "Nachträglich aus dem heutigen
  Datenstand rekonstruiert — Beträge und Anteilszahlen können vom ursprünglich
  versendeten Text abweichen."
- `en.rs` (neben Zeile 188): sinngemaess "Reconstructed afterwards from today's
  data — amounts and share counts may differ from the text originally sent."
- `mail_recipient_rendered_content.rs`: den Hint neben
  `reconstructed_label` (Zeile 30) aufloesen und am Badge-`span` (Zeile 39-42)
  als `title: "{reconstructed_hint}"` setzen. Keine Layout-Aenderung, keine
  neue Komponente, kein neuer Prop — der Hinweis gilt fuer jede
  rekonstruierte Zeile (D-05).

**Tests** (`i18n/mod.rs`, `mod tests`, nach dem Muster von
`phase_18_keys_have_distinct_de_en_translations`, Zeile 1056):
- `mail_rendered_reconstructed_hint_has_distinct_de_en_translations` — beide
  Locales liefern nicht-leere und voneinander verschiedene Strings fuer
  `Key::MailRenderedReconstructedHint`.
- Im selben Test zusaetzlich pruefen, dass der Hint sich vom kurzen
  Badge-Label `Key::MailRenderedReconstructed` unterscheidet (faengt den
  Copy-Paste-Fehler, der den Tooltip zur Wiederholung des Labels macht).

**verify:** `nix develop --command cargo test --manifest-path genossi-frontend/Cargo.toml i18n::`

**done:** Das Amber-Badge traegt einen Tooltip, der die Grenze der
Aussagekraft benennt; beide Locales sind gepflegt und getestet.

**Hinweis zur Reihenfolge:** Task 3 ist vom Backend entkoppelt und bekommt
einen eigenen Commit. Sollte der native Build von `genossi-frontend` aus
unabhaengigen Gruenden rot sein (Crate ist aus dem Workspace exkludiert, siehe
Root-`Cargo.toml:15`), blockiert das die Tasks 1 und 2 nicht — der Nutzer
braucht die Mails.

## Verification

```
nix develop --command rustfmt --edition 2021 \
  genossi_service_impl/src/repayment_context.rs \
  genossi_bin/src/lib.rs \
  genossi_bin/tests/backfill_historic_repayment.rs

nix develop --command cargo clippy -p genossi_service_impl -p genossi_bin --all-targets

nix develop --command cargo test -p genossi_service_impl repayment_context::
nix develop --command cargo test -p genossi_bin --test backfill_historic_repayment
nix develop --command cargo test -p genossi_mail

nix develop --command cargo test --workspace

# Task 3 (Frontend, aus dem Workspace exkludiert)
nix develop --command rustfmt --edition 2021 \
  genossi-frontend/src/i18n/mod.rs genossi-frontend/src/i18n/de.rs \
  genossi-frontend/src/i18n/en.rs \
  genossi-frontend/src/component/mail_recipient_rendered_content.rs
nix develop --command cargo test --manifest-path genossi-frontend/Cargo.toml i18n::
```

`rustfmt` wird gezielt auf die geaenderten Dateien angewendet — ein
workspace-weites `cargo fmt` reformatiert ~24 fremde Dateien (Memory-Notiz).

`cargo test -p genossi_mail` steht explizit in der Liste, obwohl der Quick
`genossi_mail` nicht anfasst (D-04): es ist der Beweis, dass die geteilte
Render-Funktion und ihre Quick-260908-9ud-Regressionstests unberuehrt gruen
bleiben.

Die volle Workspace-Suite ist das Schluss-Gate, weil `genossi_service_impl`
angefasst wird und Aenderungen dort in diesem Repo schon fremde Test-Helper
gebrochen haben (Memory-Notiz). Erwartete Bruchstellen: keine — alle
Aenderungen in `repayment_context.rs` sind additiv (neue `pub`-Items) plus ein
verhaltensneutrales Herausziehen des Summenkerns; keine bestehende Signatur und
kein bestehendes Struct-Feld aendert sich.

Vorbestehend rot (nicht durch diesen Quick verursacht, siehe STATE.md
"Decisions (Phase 29 Plan 02)"): zwei `/api/mail/preview`-e2e-Failures.

**Manuelle Abnahme nach dem naechsten Serverstart:** im Log erwartet wird
`rendered backfill: 11 von 11 befuellt, 0 uebersprungen` sowie pro betroffenem
Mitglied die neue `warn`-Zeile aus D-05. Bleiben Zeilen uebrig, nennt die
Quick-260908-9ud-Diagnose die verbleibende Ursache (z.B. geloeschte Eintraege
oder geloeschte Phase — beides von diesem Quick bewusst nicht abgedeckt).

## must_haves

**truths:**
- Eine abgeschlossene Phase mit ausschliesslich `PaidOut`-Eintraegen liefert im
  Backfill-Pfad einen vollstaendigen Repayment-Kontext, und das Template
  rendert.
- Derselbe Fall scheitert im Live-Versandpfad weiterhin mit unveraenderter
  Meldung; der Empfaenger wird weiterhin als failed markiert und die Mail
  bleibt aus.
- Die geteilte Render-Funktion `resolve_rendered_content` hat genau eine
  Fassung und kennt keinen Modus-Parameter.
- Kein Server-Log enthaelt Auszahlungsbetraege oder Anteilszahlen.
- Jede vom Backfill gefuellte Zeile ist im Frontend als Rekonstruktion
  gekennzeichnet, und die Kennzeichnung erklaert, dass die Zahlen aus dem
  heutigen Datenstand stammen.

**artifacts:**
- `genossi_service_impl/src/repayment_context.rs` (geaendert, mit Tests)
- `genossi_bin/src/lib.rs` (geaendert)
- `genossi_bin/Cargo.toml` (dev-dependency `mockall`)
- `genossi_bin/tests/backfill_historic_repayment.rs` (neu)
- `genossi-frontend/src/i18n/{mod,de,en}.rs` (geaendert, mit Test)
- `genossi-frontend/src/component/mail_recipient_rendered_content.rs` (geaendert)

**key_links:**
- `genossi_service_impl/src/repayment_context.rs:55-65` — der Statusfilter, der
  die Ursache ist und im Live-Pfad exakt so bleibt
- `genossi_mail/src/render.rs:368` — der einzige `aggregate`-Aufruf; die Naht,
  ueber die die Semantik hereinkommt (F-2)
- `genossi_mail/src/render.rs:389` — der Quick-260908-9ud-`warn!`, der im
  Backfill nach diesem Quick nicht mehr feuern soll
- `genossi_bin/src/lib.rs:1685` — `start_mail_worker`, muss den strikten
  Resolver behalten (F-3)
- `genossi_bin/src/lib.rs:1731` — `start_rendered_backfill_worker`, die einzige
  Injektionsstelle des historischen Resolvers
- `genossi_bin/src/lib.rs:775` / `:416` — Feld + Letter-Service-Deps, die den
  strikten Alias mit-verankern
- `genossi_dao/src/repayment_entry.rs:138-150` — `find_by_phase_id` ohne
  Statusfilter (F-1)
- `genossi_mail/src/backfill.rs:101` — `rendered_reconstructed = true` (F-5)

## Nicht in diesem Quick

- **Keine Aenderung am Live-Versandpfad.** `genossi_mail` bekommt keine Zeile
  Diff (D-04).
- **Kein Herausloesen von `fiscal_year`** aus dem Aggregat (D-03, begruendet).
- **Kein zweites DB-Feld** zur Kennzeichnung, keine Migration (D-05,
  begruendet).
- **Keine Zeitschranke** `entry.created <= job.created` (D-02, geprueft und
  verworfen).
- **Backup-Sync-Bug** (leerer `relative_path` bei
  `repayment_mail`-MemberDocuments ⇒ "Failed to sync document : Is a
  directory") — separat vorgemerkt.
- **Nextcloud-`j.body`-Bug** — separat vorgemerkt.
- **Nicht abgedeckte Restfaelle des Backfills:** Mitglied ohne jeden
  nicht-geloeschten Eintrag in der Phase, sowie geloeschte/nicht auffindbare
  Phase. Beide scheitern weiterhin und werden von der 260908-9ud-Diagnose im
  Log benannt.
