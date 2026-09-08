---
id: 260908-cjo
type: quick
status: complete
description: "Backfill rekonstruiert abgeschlossene Repayment-Phasen: PaidOut-Eintraege einbeziehen, fiscal_year verfuegbar machen"
date: 2026-09-08
depends_on: 260908-9ud
tasks_completed: 3
tasks_total: 3
commits:
  - 13e67a3
  - ee56e04
  - 8a9953b
  - 2533028
files_modified:
  - genossi_service_impl/src/repayment_context.rs
  - Cargo.lock
  - genossi_bin/src/lib.rs
  - genossi_bin/Cargo.toml
  - genossi_bin/tests/backfill_historic_repayment.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/component/mail_recipient_rendered_content.rs
---

# Quick Task 260908-cjo: Backfill rekonstruiert abgeschlossene Repayment-Phasen

Der Startup-Backfill bekommt einen zweiten, status-agnostischen
`RepaymentContextResolver`, sodass Mails aus abgeschlossenen Repayment-Phasen
(alle Eintraege `PaidOut`) wieder rendern — der Live-Versandpfad behaelt den
strikten Open/Contacted-Filter unveraendert, und `genossi_mail` bekommt keine
Zeile Diff.

## Was gebaut wurde

### Task 1 — Historische Aggregation + zweiter Resolver (`13e67a3`)

`genossi_service_impl/src/repayment_context.rs`:

- `context_from_relevant(phase, relevant)` — der Summen- und
  Euro-Formatierungskern, verhaltensneutral aus `aggregate_for_member`
  herausgezogen. `aggregate_for_member` behaelt Signatur, Doc-Kommentar und
  Filterausdruck woertlich; die zehn bestehenden Unit-Tests pinnen das.
- `HistoricAggregate { context, entry_count, stale_status_count,
  diverges_from_live }` — macht die Divergenz zur strikten Aggregation
  testbar, ohne einen `tracing`-Subscriber aufsetzen zu muessen.
- `aggregate_for_member_historic(...)` — filtert nur noch
  `deleted.is_none() && member_id == X`, **ohne** Status-Praedikat (D-02).
- `load_phase_and_entries(...)` — geteilte DAO-Ladelogik; beide `resolve`-Impls
  rufen sie auf, damit sie nicht driftet.
- `HistoricRepaymentContextResolverImpl<Deps>` — dieselbe Struct-Form und
  dieselben `RepaymentContextResolverDeps` wie die strikte Impl. `aggregate`
  loggt bei `diverges_from_live` ein `warn!` mit `phase_id`, `member_id`,
  `entry_count`, `stale_status_count` — Ids und Anzahlen, **keine** Geldwerte
  und keine Anteilszahlen (D-06).

10 neue Tests, darunter das Live-Pfad-Regressionsgate
`strict_trait_aggregate_still_errs_for_paid_out_only` und der Wertbeweis
`historic_aggregate_includes_paid_out_only` (5 PaidOut-Anteile à 120,00 EUR
⇒ `share_count 5`, `payout_amount "600,00"`, `fiscal_year 2025`).

### Task 2 — Injektion ausschliesslich im Backfill (`ee56e04`)

`genossi_bin/src/lib.rs`:

- Neuer privater Alias `HistoricRepaymentContextResolver` mit demselben
  `RepaymentContextResolverDependencies`-Marker wie der strikte Alias.
- `start_rendered_backfill_worker` konstruiert ihn **lokal** aus exakt denselben
  zwei DAO-`Arc`s, aus denen `RestStateImpl::new()` den strikten Resolver baut
  (Single-Arc-per-Process). Er wird in **kein** Struct-Feld geschrieben.
- `start_mail_worker` liest weiterhin `self.repayment_context_resolver` — der
  strikte Resolver — und traegt jetzt einen Kommentar, dass hier nie der
  historische eingesetzt werden darf.
- `genossi_bin/Cargo.toml`: `mockall` als dev-dependency (bereits
  Workspace-Dependency und in `Cargo.lock`, kein neues Crate).

`genossi_bin/tests/backfill_historic_repayment.rs` (neu, 4 Tests): ruft die
**echte** `genossi_mail::render::resolve_rendered_content` mit den **echten**
Resolver-Impls; nur die DAOs ringsherum sind Mocks, kein SQLite-Pool.

- Pflichttest (a) `backfill_resolver_renders_closed_phase_with_paid_out_entries_only`
  ⇒ `"Geschaeftsjahr 2024: 5 Anteile zu je 120,00 EUR = 600,00 EUR"`.
- Pflichttest (b) `live_resolver_still_fails_on_closed_phase_with_paid_out_entries_only`
  ⇒ `Err`, Meldung beginnt mit `"Template render error (body): "`, Diagnose
  enthaelt `"fiscal_year"` — exakt das Produktionssymptom.
- `live_resolver_still_fails_for_mixed_open_and_paid_out_amounts` ⇒ Live rendert
  `2`, Backfill rendert `5`. Zeigt die D-02-Divergenz explizit.
- `live_and_backfill_resolver_types_are_distinct` — TypeId-Gate.

### Task 3 — Ehrlichkeits-Hinweis am Rekonstruktions-Badge (`8a9953b`)

Neuer i18n-Key `MailRenderedReconstructedHint` (DE + EN); das Amber-Badge in
`mail_recipient_rendered_content.rs` traegt ihn als `title`-Attribut. Kein
zweites DB-Feld, keine Migration, kein neuer Prop, keine Layout-Aenderung.
Der Test pinnt nicht-leere, voneinander verschiedene DE/EN-Strings **und** die
Abgrenzung zum kurzen Badge-Label.

## Verifikation

| Gate | Ergebnis |
|------|----------|
| `rustfmt --check` auf alle 7 geaenderten Dateien | clean |
| `cargo clippy -p genossi_service_impl -p genossi_bin --all-targets` | keine Warnung in den geaenderten Dateien |
| `cargo test -p genossi_service_impl repayment_context::` | 25/25 |
| `cargo test -p genossi_bin --test backfill_historic_repayment` | 4/4 |
| `cargo test -p genossi_mail` | 325/325 |
| `cargo test --workspace` | exit 0, alle Targets gruen |
| `cargo test --manifest-path genossi-frontend/Cargo.toml i18n::` | 9/9 |
| `git diff -- genossi_mail/` | leer (D-04 belegt) |

Vorbestehende Clippy-Warnungen (`is_multiple_of`, `sort_by_key`, collapsible
`if`) in `genossi_service`, `genossi_mail` und `genossi_service_impl` bleiben
unberuehrt — ausserhalb des Scopes dieses Quicks.

## Abweichungen vom Plan

**1. TypeId-Test vergleicht die Impl-Typen statt der `genossi_bin`-Aliase**

- **Gefunden bei:** Task 2, beim Schreiben von
  `live_and_backfill_resolver_types_are_distinct`.
- **Sache:** Der Plan schreibt einen `TypeId`-Vergleich "der beiden
  `genossi_bin`-Aliase" vor. Beide Aliase (`RepaymentContextResolver`,
  `HistoricRepaymentContextResolver`) sind in `genossi_bin/src/lib.rs` jedoch
  **privat** (`type ...`, kein `pub type`) und aus einem Integrationstest in
  `tests/` nicht erreichbar. Die Aliase `pub` zu machen, waere eine
  Sichtbarkeits-Ausweitung allein zugunsten eines Tests gewesen.
- **Umgesetzt:** Der Test vergleicht `TypeId::of::<RepaymentContextResolverImpl<TestDeps>>()`
  gegen `TypeId::of::<HistoricRepaymentContextResolverImpl<TestDeps>>()` — also
  dieselben generischen Impls, auf die die Aliase zeigen, nur mit dem
  Test-`Deps`-Marker. Der Zweck des Gates bleibt erhalten: faellt ein spaeterer
  Refactor die beiden Impls auf denselben Typ zusammen, schlaegt der Test fehl.
  Was der Test in dieser Form **nicht** faengt: ein Refactor, der die beiden
  `genossi_bin`-Aliase auf **dieselbe** Impl zeigen laesst, ohne die Impls
  selbst zu vereinigen. Diese Restluecke ist bewusst in Kauf genommen; sie wird
  von den Pflichttests (a) und (b) auf Verhaltensebene abgedeckt.
- **Commit:** `ee56e04`

Sonst keine Abweichungen. `genossi_mail` wurde nicht angefasst (D-04),
`fiscal_year` nicht aus dem Aggregat herausgeloest (D-03), kein zweites DB-Feld
(D-05), keine Zeitschranke (D-02).

## Beobachtungen

**Die zwei vorbestehend roten `/api/mail/preview`-e2e-Failures sind gruen.**
Der Plan fuehrt sie unter "Vorbestehend rot (nicht durch diesen Quick
verursacht)". `cargo test --workspace` laeuft jetzt mit exit 0 durch, die
e2e-Suite meldet 326/326. Sie wurden also zwischenzeitlich anderweitig
behoben — nicht von diesem Quick, der `genossi_rest` und `genossi_mail` nicht
anfasst. Erwaehnt, damit die Notiz in STATE.md nicht stehen bleibt und die
naechste Ausfuehrung sich nicht auf eine falsche Baseline verlaesst.

**`genossi-frontend/Cargo.lock` haengt eine Version hinterher** —
der Lock fuehrt `genossi-frontend` mit `2026.233.2-dev`, die `Cargo.toml` steht
seit Commit `7bbd18e` auf `2026.251.1-dev`. Der Frontend-Test-Lauf zieht den Lock
mechanisch nach und laesst den Arbeitsbaum dirty zurueck. Nicht von diesem Quick
verursacht, deshalb nicht mitcommittet — festgehalten in `deferred-items.md`.
Gehoert in die Version-Bump-Logik, die den Root-Lock aktualisiert, den des aus
dem Workspace exkludierten Frontend-Crates aber nicht.

**Nachtrags-Commit `2533028`** — der `mockall`-Eintrag fuer `genossi_bin` in der
Root-`Cargo.lock` fiel beim Stagen von Task 2 durch (die Lock-Aktualisierung
passiert erst beim naechsten `cargo`-Aufruf). Separat committet statt `ee56e04`
zu amenden, weil dessen Hash bereits in SUMMARY und STATE.md steht.

**Keine Auto-Fixes noetig.** Alle Aenderungen in `repayment_context.rs` sind
additiv (neue `pub`-Items) plus das verhaltensneutrale Herausziehen des
Summenkerns; keine bestehende Signatur und kein bestehendes Struct-Feld hat
sich geaendert. Die im Plan befuerchteten fremden Test-Helper-Brueche sind
ausgeblieben.

## Grenzen dieser Rekonstruktion

Unveraendert wie im Plan beschrieben — die rekonstruierten Werte werden mit dem
**heutigen** `share_count_to_pay_out` und `phase.share_value` berechnet, nicht
mit den Werten vom Versandzeitpunkt. Unsichtbar bleiben: nachtraeglich
korrigierter `share_value`, nachtraeglich geaenderter
`share_count_to_pay_out`, und Eintraege, die nach dem Versand angelegt wurden
oder schon vorher `PaidOut` waren. Es gibt keine Historisierung der
Betragsfelder, aus der sich das ableiten liesse.

Was stattdessen geliefert wird: das Amber-Badge kennzeichnet die Zeile, der neue
Tooltip benennt die Grenze der Aussagekraft, und das Startup-Log nennt jede
Zeile, deren Rekonstruktion ueber nicht-mehr-offene Eintraege zustande kam.

## Manuelle Abnahme (offen, nach dem naechsten Serverstart)

Im Log erwartet: `rendered backfill: 11 von 11 befuellt, 0 uebersprungen` sowie
pro betroffenem Mitglied die neue `warn`-Zeile ("historische Rekonstruktion aus
nicht mehr offenen Repayment-Eintraegen"). Bleiben Zeilen uebrig, nennt die
Diagnose aus Quick 260908-9ud die verbleibende Ursache — von diesem Quick
bewusst nicht abgedeckt sind Mitglieder ohne jeden nicht-geloeschten Eintrag in
der Phase sowie geloeschte/nicht auffindbare Phasen.

## Known Stubs

Keine.

## Threat Flags

Keine. Es entsteht keine neue Netzwerk-, Auth- oder Dateizugriffs-Flaeche; kein
Schema-Change; kein neues Crate. Die einzige sicherheitsrelevante Aenderung ist
die gelockerte Aggregations-Semantik, und die ist per Typ auf den Backfill
begrenzt und durch Pflichttest (b) plus
`strict_trait_aggregate_still_errs_for_paid_out_only` gegen ein Lecken in den
Live-Versandpfad abgesichert.

## Self-Check: PASSED

Alle 8 geaenderten/neuen Artefakte auf Disk vorhanden, alle 3 Task-Commits in
`git log` auffindbar. Keine Datei-Loeschungen in den Commits
(`git diff --diff-filter=D HEAD~1 HEAD` je Commit leer).
