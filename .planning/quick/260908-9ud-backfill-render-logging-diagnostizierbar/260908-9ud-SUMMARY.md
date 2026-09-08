---
id: 260908-9ud
type: quick
description: "Backfill-/Render-Logging diagnostizierbar machen"
subsystem: infra
tags: [tracing, logging, observability, minijinja, dsgvo, genossi_mail]

requires:
  - phase: quick-260614-b1t
    provides: "resolve_rendered_content als einzige Render-Quelle für Worker + Backfill"
provides:
  - "TemplateError.diagnosis / RenderFailure.diagnosis — log-only Feld mit dem Namen der fehlenden Template-Variable"
  - "strict_env()/html_env() mit explizitem set_debug(false) in allen Build-Profilen"
  - "Strukturierte Backfill-Skip-Warnung mit recipient/job/member/application/template/repayment-phase-Ids"
  - "Log-Zeilen für die beiden bisher stillen Repayment-Merge-Zweige (EntityNotFound, phase_opt==None)"
affects: [mail-worker, mail-backfill, repayment-mails, template-authoring]

tech-stack:
  added: []
  patterns:
    - "PII-Grenze im Fehlerpfad: Variablen-NAMEN aus der (vorstands-autorierten) Template-Quelle ja, Variablen-WERTE nein"
    - "Persistierte `message` und log-only `diagnosis` sind getrennte Felder"

key-files:
  created: []
  modified:
    - genossi_mail/src/template.rs
    - genossi_mail/src/render.rs
    - genossi_mail/src/backfill.rs
    - genossi_mail/Cargo.toml
    - Cargo.lock

key-decisions:
  - "D-01: set_debug(false) explizit in beiden Envs — nicht true, nicht unberührt"
  - "D-02: Diagnose enthält Variablen-Namen, nie Variablen-Werte; kein Opt-in-Schalter"
  - "D-03: Diagnose in eigenem Feld, `message` bleibt byte-identisch (wird persistiert)"
  - "D-04: reine Observability — Backfill füllt/überspringt exakt dieselben Zeilen"
  - "D-05: Slicen ausschließlich über str::get(range), nie über den Index-Operator"

patterns-established:
  - "describe_minijinja_error(): einzeilige, PII-freie Fehlerdiagnose aus kind/line/expr/detail/source-Kette"
  - "Log-Capture in Unit-Tests via MakeWriter + tracing::subscriber::with_default um rt.block_on"

requirements-completed: []

duration: 45min
completed: 2026-09-08
status: complete
---

# Quick Task 260908-9ud: Backfill-/Render-Logging diagnostizierbar machen Summary

**Die Backfill-Skip-Warnung nennt jetzt den Namen der fehlenden Template-Variable plus alle beteiligten Ids — ohne dass ein einziger Mitglieds-Wert im Log landet und ohne dass sich das Füll-/Skip-Verhalten ändert.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-09-08T05:10Z (ca.)
- **Completed:** 2026-09-08T05:53Z
- **Tasks:** 3 / 3
- **Files modified:** 5 (3 Rust-Quellen, 1 Cargo.toml, Cargo.lock)

## Accomplishments

- **Der Wertbeweis:** die produktive Log-Zeile sieht jetzt so aus (real aus dem Testlauf abgegriffen):
  ```
  WARN genossi_mail::backfill: rendered backfill: skip recipient — Template render error (body): Template render error: undefined value (in <string>:1)
    recipient_id=cdebcbfa-… mail_job_id=005babf8-… member_id=Some(fbbb2aff-…) application_id=None
    template_id=None repayment_phase_id=None diagnosis=Some("kind=undefined value; line=1; expr=payout_amount")
  ```
  Vorher stand da nur `skip recipient <uuid> — Template render error (body): … undefined value (in <string>:3)`. Die SQL-Query gegen die Produktions-DB entfällt.
- **Der wahrscheinlichste Ursachen-Zweig ist nicht mehr stumm:** der `Err(ServiceError::EntityNotFound(_))`-Arm im Repayment-Merge loggt jetzt `phase_id`, `member_id`, `entry_count` und sagt explizit, dass der Kontext UNMERGED bleibt und ein ungeguardetes Template deshalb am Strict-Render scheitert. Derselbe Log-Punkt kam für den `phase_opt == None`-Fall dazu.
- **Datenschutz strukturell abgesichert statt nur diszipliniert:** `set_debug(false)` in beiden minijinja-Envs sorgt dafür, dass `debug_info` gar nicht erst entsteht — `referenced_locals` (unmaskierte IBAN, E-Mail, Kontostand) kann in keinem Build-Profil ins Error-Objekt geraten. Ein Test prüft das an der rohen `minijinja::Error`-Instanz unter `{:#}`.
- **14 neue Tests**, davon 4 explizite Gates (D-01-Pin, zwei DSGVO-Gates, D-03-Kurzform-Gate) und 2 D-04-Verhaltenspins.

## Task Commits

1. **Task 1: `TemplateError.diagnosis` einführen, Debug-Pfad abschalten** — `62ca326` (feat)
2. **Task 2: `diagnosis` durch `RenderFailure` reichen, Repayment-Merge-Zweige loggen** — `1159b21` (feat)
3. **Task 3: Backfill-Warnungen strukturieren** — `e00bda0` (feat)

## Files Created/Modified

- `genossi_mail/src/template.rs` — `set_debug(false)` in `strict_env()`/`html_env()`; `TemplateError.diagnosis: Option<String>`; `TemplateError::from_minijinja()`; `describe_minijinja_error()`, `failing_expression()`, `shorten_for_diagnosis()`; die sechs Struct-Literale umgestellt; 9 neue Tests.
- `genossi_mail/src/render.rs` — `RenderFailure.diagnosis: Option<String>` + `RenderFailure::with_diagnosis()`; sechs `map_err`-Stellen reichen die Diagnose durch; drei Log-Punkte im Repayment-Zweig (warn/warn/debug); 3 neue Tests.
- `genossi_mail/src/backfill.rs` — drei Fehlerzweige auf `tracing`-Felder umgestellt; Abschluss-`info!` mit `filled`/`total`/`skipped`; 2 neue Tests inkl. Log-Capture-Scaffolding.
- `genossi_mail/Cargo.toml` — `[dev-dependencies] tracing-subscriber = { workspace = true }`.
- `Cargo.lock` — genau eine Zeile: `tracing-subscriber` als Kante von `genossi_mail`. Kein neues Paket, keine Versionsänderung.

## Decisions Made

Alle fünf Plan-Entscheidungen (D-01 … D-05) wurden wörtlich umgesetzt. Zwei Umsetzungsdetails, die der Plan offen ließ:

- **`describe_minijinja_error` gibt eine `String` zurück, kein strukturiertes Objekt.** Sie ist ausschließlich Log-Input; ein Typ hätte nur Boilerplate erzeugt. Format: `kind=…; line=…; expr=…; detail=…; caused_by=…`, Felder entfallen wenn leer.
- **Der `caused_by`-Walk nutzt `Error::source()` über die Trait-Methode**, dafür kam `use std::error::Error as _;` in den Modul-Scope von `template.rs` (wie im Plan vorgesehen). In der Praxis ist die Kette bei Strict-Undefined-Fehlern leer — das Feld erscheint dann gar nicht.
- **Log-Capture-Tests sind `#[test]` mit eigenem current_thread-Runtime**, nicht `#[tokio::test]`. Grund: `tracing::subscriber::with_default` nimmt einen synchronen Closure; um es (wie vom Plan gefordert) statt der Guard-Variante zu verwenden, muss `rt.block_on(..)` innerhalb des Closures laufen. Thread-lokal und damit parallel-test-sicher wie geplant.

## Deviations from Plan

**Keine funktionalen Abweichungen.** Drei Beobachtungen, die der Plan anders erwartet hatte:

**1. [Beobachtung] Die als „vorbestehend rot" angekündigten `/api/mail/preview`-e2e-Tests sind grün.**
- **Gefunden bei:** Schluss-Gate (`cargo test --workspace`)
- **Erwartung laut Plan/STATE.md:** zwei rote `/api/mail/preview`-e2e-Failures
- **Befund:** `cargo test --test e2e_tests preview` liefert **8 passed, 0 failed**; `cargo test --workspace` ist über alle 25 Test-Targets hinweg **0 failed**. Es gibt in `genossi_bin/tests/e2e_tests.rs` auch kein einziges `#[ignore]`. Die Notiz in STATE.md („Decisions (Phase 29 Plan 02)") ist offenbar veraltet.
- **Konsequenz:** nichts zu tun — es wurde nichts repariert und nichts unterdrückt. Nur der Plan-Erwartungswert stimmt nicht mehr.

**2. [Scope-Grenze] Vorbestehende Clippy-Warnung in `worker.rs` nicht angefasst.**
- `genossi_mail/src/worker.rs:105` — `consider using sort_by_key` für `matches.sort_by(|a, b| b.created.cmp(&a.created))`. Existiert vor diesem Quick, liegt außerhalb der drei geänderten Dateien, wurde nach der Scope-Boundary-Regel bewusst stehen gelassen. (Ebenso `is_multiple_of` in `genossi_service`.) Nach dem Quick sind es exakt dieselben zwei Warnungen wie davor — keine neue hinzugekommen.

**3. [Ergänzung innerhalb D-04] Der `phase_opt == None`-Zweig bekam ein `else`.**
- Der Plan nennt diesen Fall unter Task 2 ausdrücklich („Der `if let Some(phase) = phase_opt`-Block bekommt einen `else`-Zweig"), er ist also geplant — hier nur zur Klarheit festgehalten, weil er der einzige Punkt ist, an dem *neuer Kontrollfluss-Text* (ein `else`-Block) entstand. Der Block enthält ausschließlich ein `tracing::warn!`; der Ablauf danach ist unverändert.

---

**Total deviations:** 0 auto-fixes (Regeln 1–4 kamen nicht zum Einsatz)
**Impact on plan:** Kein Scope Creep. Der Plan war vollständig; die einzige Reibung war eine veraltete Erwartung über rote Tests.

## Issues Encountered

- **Mockall-Generics:** die Hilfsfunktion `entity_not_found_mocks()` in `render.rs` wurde zunächst mit generischen Rückgabetypen (`MockTransactionDao<MockTransaction>`) geschrieben. Die Mocks entstehen im Repo per `#[automock(type Transaction = …;)]` und sind deshalb **nicht** generisch. Auf die nackten Typnamen korrigiert.
- Sonst nichts — die drei Verify-Kommandos des Plans liefen beim ersten vollständigen Durchlauf grün.

## Verification

Alle vier Plan-Kommandos ausgeführt, alle grün:

| Kommando | Ergebnis |
|---|---|
| `nix develop --command rustfmt --edition 2021 <die drei Dateien>` | angewendet; `--check` danach ohne Ausgabe |
| `nix develop --command cargo clippy -p genossi_mail --all-targets` | 0 Fehler; 2 vorbestehende Warnungen (worker.rs, genossi_service), keine neue |
| `nix develop --command cargo test -p genossi_mail` | **325 passed, 0 failed** |
| `nix develop --command cargo test --workspace` | **0 failed** über alle Targets (u.a. 445 + 326 + 325 + 105 + 88 …) |

Per-Task-Gates: `template::` 76 passed · `render::` 26 passed · `backfill::` 5 passed (davon die 3 bestehenden unverändert, D-04).

**must_haves gegengeprüft:**

| Truth | Status | Beleg |
|---|---|---|
| Strict-Undefined-Fehler nennt den Variablennamen, auch im Release-Build | erfüllt | `render_error_diagnosis_names_the_undefined_variable` + `diagnosis_works_without_runtime_debug_flag` (baut den Produktions-Env-Zustand explizit nach) |
| Skip-Warnung nennt Recipient-, Job-, Member-/Application-, Template-, Repayment-Phase-Id | erfüllt | `backfill_skip_log_carries_ids_and_variable_name` |
| `EntityNotFound`-Fall erzeugt eine Log-Zeile | erfüllt | `tracing::warn!` in `render.rs`; `repayment_entity_not_found_leaves_context_unmerged_and_names_the_variable` durchläuft den Zweig |
| Weder Error-Objekt noch Diagnose noch Log enthalten Mitglieder-Werte, in jedem Profil | erfüllt | `error_carries_no_member_values_even_under_alternate_format` (roher Error unter `{:#}`), `render_error_diagnosis_omits_member_values`, `backfill_skip_log_omits_member_values`, `strict_env_and_html_env_disable_runtime_debug` |
| Backfill überspringt exakt dieselben Zeilen wie vorher | erfüllt | die 3 bestehenden Backfill-Tests unverändert grün; `repayment_entity_not_found_with_guarded_template_still_renders` pinnt die Ok-Seite |

## Known Stubs

Keine.

## Threat Flags

Keine neue Angriffsfläche: keine neuen Endpoints, keine neuen Auth-Pfade, keine Schema-Änderung, keine neue Dependency (nur eine dev-only Kante auf ein bereits gelocktes Workspace-Crate).

Der Quick *verkleinert* eine bestehende Fläche: `set_debug(false)` verhindert strukturell, dass Mitglieds-Werte in `minijinja::Error`-Objekte materialisiert werden — vorher war das im Debug-/Test-Profil per Crate-Default (`cfg!(debug_assertions)`) der Fall.

## Next Steps

Wie im Plan unter „Nicht in diesem Quick" festgehalten: die 11 Produktions-Zeilen selbst sind **nicht** gefixt. Nach dem nächsten Serverstart zeigt das Log, welche Variable in welchem Template/Job fehlt. Erst dann fällt die Entscheidung, ob die betroffenen Templates `{% if … is defined %}`-Guards bekommen oder der Backfill diese Zeilen dauerhaft überspringen darf.

---
*Quick: 260908-9ud*
*Completed: 2026-09-08*

## Self-Check: PASSED

Alle in dieser SUMMARY genannten Dateien existieren auf der Platte (`genossi_mail/src/{template,render,backfill}.rs`, `genossi_mail/Cargo.toml`, `Cargo.lock`), alle drei Task-Commits (`62ca326`, `1159b21`, `e00bda0`) sind in `git log` auffindbar, und die zentralen neuen Symbole sind im Code nachweisbar (`set_debug(false)` an beiden Env-Stellen, `diagnosis` in allen drei Quellen).
