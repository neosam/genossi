# Pitfalls Research — v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres

**Domain:** Membership lifecycle adjustments coexisting with v1.1 RepaymentPhase/PaidOut-Cascade
**Researched:** 2026-06-04
**Confidence:** HIGH (codebase-grounded with file:line references)

---

## Kritische Invariante (Lest dies zuerst)

**v1.2 darf NICHT `MemberAction::Verkauf` erzeugen und NICHT `current_shares` reduzieren, wenn die Genossenschaft später Geld auszahlt. Das macht v1.1's PaidOut-Cascade** (`genossi_service_impl/src/repayment_entry.rs::mark_paid_out`, Z. 517–723).

v1.2 erzeugt nur **Intent-Datensätze:**
- **Kündigung** → `exit_date` am Member; KEINE MemberAction, KEIN RepaymentEntry direkt (v1.1's Auto-Fill picked beim nächsten Phase-Open)
- **Teil-Rückgabe** → `RepaymentEntry` in Ziel-Phase; KEINE MemberAction, KEINE `current_shares`-Reduktion
- **Übertrag** → 2 verlinkte MemberActions (neuer Action-Typ, NICHT `Verkauf`); `current_shares` atomar; KEIN RepaymentEntry
- **Aufstocken** → MemberAction (neuer Aufstockung-Action-Typ); `current_shares` sofort

---

## Kategorie 1: Doppelbuchung — Auto-Fill + v1.2-Trigger (KRITISCH)

### Risiko

v1.2-Kündigung setzt `exit_date` am Member. Beim nächsten `open_repayment_phase` filtert v1.1's Auto-Fill (`genossi_service_impl/src/repayment_phase.rs:319–395`) alle Member mit `exit_date IN [fy_start, fy_end]` und `current_shares > 0`. Wenn v1.2 dieselbe Kündigung zusätzlich als RepaymentEntry vor-greifend einfügt, entsteht ein Duplikat — **ENTR-03** hat bewusst KEINEN UNIQUE-Constraint auf `(member_id, phase_id)` (siehe PROJECT.md Key Decisions).

Komplexer Fall: Wenn ein Member im selben GJ **sowohl** Teil-Rückgabe (v1.2 erzeugt Entry) **als auch** später Kündigung mit Stichtag im selben fiscal_year hat, picked Auto-Fill ihn beim Phase-Open zusätzlich auf — Duplikat-Entry.

### Warning Signs

- `create_repayment_entry` (`genossi_service_impl/src/repayment_entry.rs:101–170`) validiert nur gegen `Phase.status == Open` (D-11.1 Z. 128–133), **nicht** gegen Duplikate auf `(member_id, phase_id)`.
- Audit-Log würde zwei separate `audited_create!`-Einträge zeigen (semantisch valid, business-logisch falsch).
- Helper-Funktion für Duplikat-Detection fehlt; nur `current_shares`-Range-Check existiert (D-11.3 Z. 143).

### Prevention Strategy

- **Service-Layer-Sum-Check** beim `create_repayment_entry`: vor Insert prüfen, dass `sum(share_count_to_pay_out) für (member_id, phase_id) WHERE status != PaidOut + new.share_count <= member.current_shares`.
- **Auto-Fill-Skip-Pattern** im `open_repayment_phase` — wenn der Member bereits Entries in der Phase hat, Auto-Fill überspringt ihn (statt zusätzlichen Entry erzeugen). Pattern-Anker: Phase-8-Auto-Fill-Loop bei Z. 360–395 erweitern.
- **Neue DAO-Query** `find_by_member_and_phase(member_id, phase_id, tx) -> Vec<Entry>` als Foundation für beide Strategies.

### In welcher Phase abzudecken

- **Spec-Phase:** Duplikat-Detection-Pflicht spezifizieren
- **Plan-Phase:** Service-Layer-Sum-Check + Auto-Fill-Skip + DAO-Query implementieren, E2E-Test schreiben für „Kündigung + Auto-Fill = nur 1 Entry"

---

## Kategorie 2: Auto-Anlegen-Ziel-Phase bei H2-Offset (MITTEL-HOCH)

### Risiko

v1.2-Teil-Rückgabe im H2 (z.B. November 2026) mit Stichtag 31.12. **folgendes** GJ (2027) braucht eine `RepaymentPhase` für FY 2027 im Status **Open**. Wenn diese Phase nicht existiert oder noch in `Vorbereitung` ist, schlägt `create_repayment_entry` mit `Phase.status != Open` (D-11.1 Z. 128–133) fehl.

Bei Kündigung ist das **nicht** akut — Auto-Fill picked den Member ja erst beim späteren `open_phase` automatisch auf, kein Direkt-Insert. Bei **Teil-Rückgabe** hingegen ist der RepaymentEntry der einzige Intent-Datensatz; ohne ihn geht die Information verloren.

### Warning Signs

- `create_repayment_entry` schlägt mit 409 fehl, wenn Phase nicht existiert oder in falschem Status — Vorstand-Workflow „Teilrückgabe eingeben" → Fehlermeldung → Workflow bricht ab.
- v1.1's `open_repayment_phase` lädt Phase via `repayment_phase_dao.find_by_id` und 404-t bei fehlender Phase.

### Prevention Strategy (3 Optionen — Discuss-Phase-Item)

**A) Auto-Create in `Vorbereitung` + D-11.1-Guard erweitern auf `Preparation | Open`** + Auto-Fill-Dedup beim späteren Open
- Risiko: D-11.1-Guard-Aufweichung berührt v1.1-Invarianten (Phase-8-Decisions); benötigt Audit-Story für Auto-Erzeugte Phasen
- Vorteil: Phase wird im richtigen Status angelegt (Vorstand kann später öffnen/abschliessen wie üblich)

**B) Auto-Create direkt in `Open` + Auto-Fill-Skip-Pattern (Kategorie 1)**
- Risiko: Auto-Fill iteriert über existing exit_date-Members → Müllentries entstehen, wenn Phase nur für 1 Teilrückgabe-Member angelegt
- Vorteil: D-11.1 bleibt unangetastet; Direkt-Insert funktioniert sofort
- Vorbedingung: Auto-Fill-Skip-Pattern aus Kategorie 1 muss zuverlässig sein

**C) Explicit Error + Helpful Message** (kein Auto-Create)
- v1.2-Dialog: „Phase für FY 2027 existiert nicht. Vorstand muss diese zuerst anlegen → [Link zur Phase-Anlegen-Seite]"
- Vorteil: minimaler Eingriff; explizite Vorstands-Aktion erzwingt Bewusstsein für GJ-Wechsel
- Nachteil: User-Experience-Bruch — Workflow muss neu gestartet werden

**D) MemberAction sofort, RepaymentEntry deferred bis Phase-Open** (Drittvariante)
- v1.2-Teilrückgabe erzeugt nur ein neues `PendingPartialReturn`-Marker (Member-Spalte oder leichte neue Entity); Phase-Open-Auto-Fill picked Marker auf
- Risiko: zusätzliche Entity mit eigenem Lifecycle, höhere Komplexität
- Nicht empfohlen für v1.2-Scope (Out-of-Scope-Kandidat)

**Empfehlung:** Variante **B** (Auto-Create in Open) abhängig von Kategorie-1-Skip-Lösung. Discuss-Phase muss zwischen A/B/C entscheiden.

### In welcher Phase abzudecken

- **Discuss-Phase:** A vs. B vs. C entscheiden
- **Plan-Phase:** je nach Entscheidung Service-Layer `ensure_repayment_phase` oder Error-Mapping implementieren

---

## Kategorie 3: Audit-Hashchain-Konsistenz bei verlinkt-atomaren Operationen (MITTEL)

### Risiko

Übertrag erzeugt 2 verlinkte MemberActions:
- `Übertragung-Aus (A: −n, transfer_member_id=B.id)`
- `Übertragung-Ein (B: +n, transfer_member_id=A.id)`

Wenn v1.2 nur eine der beiden in der Tx ausführt (Exception zwischen den zwei `audited_create!`-Calls), bleibt der `audit_log` mit einer verwaisten Action. Die Hash-Chain bleibt technisch valid (Hash ≠ vorherige + neue Hash), aber die Semantik bricht — Verlinkung fehlt.

Außerdem: Wenn beide Actions denselben `transaction_id` brauchen, damit Auditor sie als ein Vorgang erkennt, muss das im Service-Layer explizit gesetzt sein. v1.1's PaidOut-Cascade nutzt gemeinsamen `process="repayment-entry.mark-paid-out"`-String (`repayment_entry.rs:47`, `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT`).

### Warning Signs

- `MemberActionEntity` hat `transfer_member_id: Option<Uuid>` (`genossi_dao/src/member_action.rs:59`).
- Zwei Service-Layer-`audited_create!`-Calls für die zwei Actions, falls sie nicht in derselben Tx liegen → erste committed, zweite rollback → inkonsistenter State.
- Wenn unterschiedliche `process`-Strings vergeben werden, gruppiert `/api/audit/verify` + Process-Filter den Vorgang nicht.

### Prevention Strategy

- **Single-Tx-Anker:** v1.2-Übertrag-Implementation analog `mark_paid_out`-Cascade (12-Schritt-Pattern Phase 9) — beide `audited_create!`s in derselben Tx mit gemeinsamem `process="member-adjust.transfer"`.
- Beide Actions teilen `tx.clone()`; Exception im zweiten Schritt → ganze Tx rollback.
- **Test:** v1.2-Übertrag mit Mock-Exception nach erstem `audited_create!` → Tx-Rollback verifizieren, audit_log enthält keine der beiden Actions.
- **Audit-Verifikation** im E2E: `/api/audit/verify.valid==true` UND `/api/audit/member_action` mit Filter `process="member-adjust.transfer"` zeigt genau 2 Einträge pro Übertrag.

### In welcher Phase abzudecken

- **Plan-Phase:** Übertrag-Implementierung mit atomarer 2-Action-Tx + gemeinsamem `process`-String
- **Verify-Phase:** E2E-Audit-Verifikation analog Phase-9-Multi-Endpoint-Pattern

---

## Kategorie 4: H1/H2-Stichtagsregel-Edge-Cases (MITTEL)

### Risiko

- **Schaltjahr:** Willensbekundung am 30.06. (H1-Grenze) bzw. 01.07. (H2-Grenze) muss explizit gehandelt werden — die Definition `H1 = Monat 1–6, H2 = 7–12` ist im Design-Doc, aber im Code nirgendwo dokumentiert.
- **Willensbekundung am 31.12.:** Kündigung am 31.12.2026 → H2 → Stichtag 31.12.2027. Aber: Was ist das `MemberAction.date`-Feld? Willensbekundungs-Datum oder berechneter Stichtag? `compute_dates` in `genossi_service_impl/src/member_action.rs:155–177` nutzt `effective_date.unwrap_or(action.date)`.
- **Datepicker-Bounds:** „nur offenes GJ erlaubt" (Design-Doc) ist ambig, wenn H2-Wirksamkeit folgendes GJ erreicht — Datepicker muss aktuelles + nächstes GJ erlauben, sonst keine H2-Erfassung möglich.

### Warning Signs

- `member_action.rs:168` `effective_date.unwrap_or(a.date)` — implizit, nicht v1.2-aware.
- `member.rs:213–218` `join_date`, Z. 280–300 `exit_date` — keine H1/H2-Bezugskommentare.
- Audit-Log würde nicht zeigen, ob ein `exit_date` aus v1.2's Stichtagsregel kommt oder manuell gesetzt war.

### Prevention Strategy

- **Pure-Function** `compute_effective_date(willensbekundung_date: Date) -> (fiscal_year: i32, exit_date: Date)`:
  - H1 (Monat 1–6): `fiscal_year = year(willensbekundung)`, `exit_date = 31.12. year(willensbekundung)`
  - H2 (Monat 7–12): `fiscal_year = year(willensbekundung) + 1`, `exit_date = 31.12. year(willensbekundung)+1`
- Lokation: neuer Helper in `genossi_service_impl/src/membership_adjust.rs` oder `member_action.rs`. Unit-testbar mit Edge-Cases (30.06., 01.07., 31.12., 01.01., Schaltjahr-Februar).
- **Datepicker-Logik im Frontend:** erlaubt `today() ± span(aktuelles offenes GJ)`; Backend-Service validiert zusätzlich.
- **MemberAction.date-Konvention:** Wilensbekundungs-Datum geht in `MemberAction.date`; berechneter Stichtag (falls != Willensbekundung) geht in `effective_date`. Inline-Doc im Service-Code.

### In welcher Phase abzudecken

- **Discuss-Phase:** H1/H2-Grenze + Datepicker-Scope explizit fixieren
- **Plan-Phase:** Pure-Function `compute_effective_date` + Unit-Tests (mind. 6 Edge-Cases)

---

## Kategorie 5: ActionType-Enum-Erweiterung ohne PaidOut-Cascade-Seiteneffekt (MITTEL-HOCH)

### Risiko

v1.2 braucht neue `ActionType`-Varianten:
- **Übertragung-Aus** (transfer_member_id required, shares_change < 0)
- **Übertragung-Ein** (transfer_member_id required, shares_change > 0)
- **Aufstockung** (shares_change > 0, transfer_member_id = None)

`mark_paid_out` (`genossi_service_impl/src/repayment_entry.rs:600–627`) hardcodet `ActionType::Verkauf` (Z. 610). Wenn v1.2 fälschlich `ActionType::Verkauf` für Übertrag verwendet:
- `validate_action` (`member_action.rs:96–103`) erzwingt `shares_change < 0` für Verkauf → fängt einen Teil der Falsch-Verwendung
- Aber: Audit-Story wird verwirrt (Verkauf statt Übertrag in `/api/audit/member_action`)

Außerdem: PaidOut-Cascade triggert auf `RepaymentEntry`-Status-Toggle (nicht auf MemberAction-Type), also kein direkter Cascade-Seiteneffekt. **Risiko ist primär semantisch**, nicht buchhalterisch.

### Warning Signs

- `ActionType`-Enum-Variante muss in mehreren Stellen synchron gepflegt werden: DAO (`genossi_dao/src/member_action.rs:9–18`), Service-Validierung (`validate_action` Z. 76–153), REST-TO, Frontend-Translation.
- Wenn `validate_action` für die neuen Types keine Regel hat, akzeptiert sie alles → Datenqualitätsbug.

### Prevention Strategy

- **Enum-Erweiterung-Checkliste** (im Plan-Doc):
  1. `ActionType`-Enum in `genossi_dao/src/member_action.rs` (Migration falls Enum-as-String stored)
  2. `validate_action`-Regeln in `genossi_service_impl/src/member_action.rs`:
     - `Übertragung-Aus`: `shares_change < 0` AND `transfer_member_id.is_some()`
     - `Übertragung-Ein`: `shares_change > 0` AND `transfer_member_id.is_some()`
     - `Aufstockung`: `shares_change > 0` AND `transfer_member_id.is_none()`
  3. REST-TO + Frontend-Display-Strings (i18n DE/EN)
  4. Unit-Test pro neue Variante
- **Verkauf-Verteidigung:** Inline-Doc auf `mark_paid_out` (Z. 610): „ActionType::Verkauf ist EXKLUSIV für PaidOut-Cascade. Übertragung/Aufstockung haben eigene Types."
- **Grep-Gate** in der Plan-Phase: `grep -n "ActionType::Verkauf" --include="*.rs"` zeigt nur die eine Zeile in `mark_paid_out`. Falls v1.2-Code zusätzliche Zeilen erzeugt → fail.

### In welcher Phase abzudecken

- **Discuss-Phase:** Namen + Validierungsregeln finalisieren (insbesondere DE/EN-Naming-Convention)
- **Plan-Phase:** Enum-Erweiterung + validate_action-Tests + Grep-Gate

---

## Kategorie 6: current_shares-Race (Optimistic Locking) (MITTEL)

### Risiko

v1.1's `mark_paid_out` (`repayment_entry.rs:629–641`) aktualisiert `Member.current_shares -= N`. v1.2-Übertrag aktualisiert `current_shares` sofort (A: −n, B: +n). v1.2-Aufstockung aktualisiert sofort (+n). Wenn parallel:
- v1.2-Übertrag auf Member A
- v1.1-mark_paid_out auf Member A

optimistic-locking (`genossi_service_impl/src/member.rs:214–215` `if entity.version != update.version`) blockt eine der beiden mit 409. Frontend muss Re-Read durchführen (siehe Phase-7-Tech-Debt: Optimistic-Locking Stale-Retry-Pattern).

### Warning Signs

- Nach `audited_update!` wird Member-Entity zwar re-read (`member.rs:343–348`), aber NEUE `version` UUID wird im Service-Return mitgegeben — Frontend muss diese auch im Formular halten.
- Wenn zwei nebenläufige REST-Calls beide auf demselben Member landen, sieht der zweite die alte `version`.

### Prevention Strategy

- **Service-Layer-Fehler-Message** muss klar sein: „Member.version mismatch — Daten wurden parallel geändert. Bitte Seite neu laden und erneut versuchen."
- **Frontend:** nach 409 (Conflict) im v1.2-Dialog → expliziter Hinweis + auto-Refresh des Dialogs mit neuen Daten.
- **Discuss-Phase-Item:** Sollte v1.2 eine pessimistische Lock auf dem Member halten während des Dialogs? — empfohlen: **nein**, das ist v2-Architektur.

### In welcher Phase abzudecken

- **Plan-Phase:** Service-Layer-Fehler-Message + Frontend-Re-Read-Pattern für 409

---

## Kategorie 7: Empfänger-Search bei Übertrag — Soft-Delete + Self-Transfer (MITTEL)

### Risiko

v1.2-Übertrag braucht ein Search-Feld für „Empfänger aktives Mitglied". `member.rs` `all()` (genossi_service/src/member.rs) lädt alle Member; der DAO-`dump_all` filtert nur `deleted IS NULL`, **nicht** auf `status` oder `exit_date`. Eine soft-deleted oder gekündigte Member könnte im Search auftauchen.

Zusätzlich: Member darf nicht **sich selbst** als Empfänger wählen (Self-Transfer ist kein valider Geschäftsvorfall).

### Warning Signs

- `member_dao.all()` und `find_by_id()` filtern `deleted IS NULL` per Default — aber `exit_date IS NOT NULL` heißt der Member ist gekündigt, nicht soft-deleted.
- `member.rs:309–311` `update()` hat keinen Status-Mutation-Guard.

### Prevention Strategy

- **Neue Service-Methode** `list_transfer_recipients(exclude_member_id: Uuid) -> Vec<Member>`:
  - Filter: `deleted IS NULL` (default) AND `exit_date IS NULL` (aktives Mitglied) AND `id != exclude_member_id`
- **Neuer REST-Endpoint** `GET /api/members/transfer-recipients?exclude_self={uuid}` mit Permission-Check (admin-only).
- **Frontend-Search:** ausschließlich diesen Endpoint nutzen, nicht den allgemeinen `GET /api/members`.
- **Service-Layer-Guard** beim Übertrag-Create: zusätzlich validieren `from_member_id != to_member_id`.

### In welcher Phase abzudecken

- **Plan-Phase:** `list_transfer_recipients` DAO/Service-Methode + REST-Endpoint + Frontend-Search

---

## Kategorie 8: Permission-Edge-Case — Vorstand kündigt sich selbst (MITTEL)

### Risiko

v1.2-Kündigung ist `admin`-only. Ein Mitglied der Genossenschaft kann auch Vorstand sein. Wenn Vorstand sich selbst kündigt:
- `exit_date` wird gesetzt
- bei Voll-Kündigung wird später (via PaidOut-Cascade) `current_shares = 0`
- Eventuell: Vorstand verliert noch in der UI-Session selbst die Berechtigung, was zu unklarem UX führt

### Warning Signs

- Permission-Check global auf `ADMIN_PRIVILEGE`, nicht auf „darf ich auf mich selbst operieren?".
- Audit-Log zeigt `actor_id == subject_id` für die Action — semantisch ok, aber ungewöhnlich.

### Prevention Strategy

- **Frontend-Dialog:** Wenn `current_user_id == member_id`, extra Warn-Modal: „Sie sind dabei, Ihre eigene Mitgliedschaft zu beenden. Das ist unwiderruflich. Fortfahren?"
- **Kein Service-Layer-Guard** — Vorstand darf sich selbst kündigen, das ist verbandsrechtlich legitim (z.B. Vorstand muss aus persönlichen Gründen austreten).
- Optional: Im Audit-Log expliziter Flag „self-action" für leichteren Audit-Trail.

### In welcher Phase abzudecken

- **Plan-Phase:** Frontend-Dialog-Text + Visual-Warning

---

## Kategorie 9: SQLITE_BUSY-Race in v1.2-Cascade-Tests (NIEDRIG)

### Risiko

v1.1-Phase-9-E2E-Tests akzeptieren `[200, 409|500]` als Race-Outcome (Phase-9-Tech-Debt; siehe `milestones/v1.1-MILESTONE-AUDIT.md`). v1.2-Übertrag-Cascade (2 verlinkte MemberActions + 2 Member-Updates in einer Tx) ist ähnlich strukturiert; SQLITE_BUSY-Race-Path im Memory-Pool-Test ist erwartbar.

### Prevention Strategy

- **E2E-Test-Pool-Setup:** `busy_timeout(5000)` im In-Memory-Pool setzen (analog v1.1 Phase 9 Pool-Setup).
- Tests akzeptieren `[200, 409|500]` mit Negativ-Constraint `!(status_a == 200 && status_b == 200)` (Pattern Phase-9-Plan-04).
- DAO-Layer-Mapping `SQLITE_BUSY → ConflictError` ist Rule-4-Change und bleibt Tech-Debt für v1.3+.

### In welcher Phase abzudecken

- **Verify-Phase:** E2E-Pool mit `busy_timeout` + sortierte Status-Assertion

---

## Kategorie 10: recalc_migrated-Konsistenz bei v1.2-Operationen (NIEDRIG–MITTEL)

### Risiko

v1.1's `mark_paid_out` ruft `recalc_migrated` auf (`repayment_entry.rs:692–718`), um den `Member.migrated`-Flag zu aktualisieren. `compute_migration_status` in `member.rs:74–82` zählt MemberActions zur Bestimmung.

v1.2-Operationen erzeugen MemberActions:
- **Übertrag** (2× MemberAction) → muss `recalc_migrated` aufrufen für beide Members
- **Aufstockung** (1× MemberAction) → muss `recalc_migrated` aufrufen
- **Kündigung** (KEINE MemberAction direkt; setzt nur `exit_date`) → KEIN `recalc_migrated` nötig
- **Teil-Rückgabe** (nur RepaymentEntry, keine MemberAction) → KEIN `recalc_migrated` nötig

### Warning Signs

- `recalc_migrated` ist Service-internal in `genossi_service_impl/src/member.rs`; wenn v1.2-Service nicht denselben Helper teilt, könnte er vergessen werden.
- Bestehende `MemberServiceImpl` ruft `recalc_migrated` nach `MemberAction`-Create automatisch (über `MemberActionService`-Interaktion). Wenn v1.2 einen eigenen Service-Pfad nimmt, muss er den Helper explizit aufrufen.

### Prevention Strategy

- **Service-Code-Konvention:** Nach jedem `audited_create!(MemberAction)` in v1.2-Code muss `recalc_migrated` für die betroffenen Member-IDs aufgerufen werden (in derselben Tx).
- **Grep-Gate** in Plan-Phase: jede `audited_create!(...member_action_dao, ...)`-Stelle in v1.2-Code muss in derselben Funktion einen `recalc_migrated`-Aufruf haben (oder explizit dokumentierte Ausnahme).
- **Test:** v1.2-Übertrag erzeugt 2 MemberActions → `Member.migrated`-Flag beider Members nach Übertrag korrekt gesetzt.

### In welcher Phase abzudecken

- **Plan-Phase:** Helper-Aufruf-Konvention in der Service-Code-Skizze festhalten; Unit-Test pro Operation

---

## Zusammenfassung — Priorisierung

| # | Kategorie | Severity | Phase-Coverage |
|---|-----------|----------|----------------|
| 1 | Doppelbuchung Auto-Fill + v1.2 | **KRITISCH** | Spec + Plan + Verify |
| 2 | Ziel-Phase nicht existent (H2→folgendes GJ) | **MITTEL-HOCH** | Discuss + Plan |
| 3 | Audit-Verlinkung Übertrag-Action | MITTEL | Plan + Verify |
| 4 | H1/H2-Stichtag Edge-Cases | MITTEL | Discuss + Plan |
| 5 | Neue ActionTypes statt Verkauf | **MITTEL-HOCH** | Discuss + Plan |
| 6 | current_shares-Race (Optimistic-Lock) | MITTEL | Plan |
| 7 | Empfänger-Search Soft-Delete + Self | MITTEL | Plan |
| 8 | Vorstand-Self-Kündigung | MITTEL | Plan |
| 9 | SQLITE_BUSY in v1.2-E2E | NIEDRIG | Verify |
| 10 | recalc_migrated-Konsistenz | NIEDRIG–MITTEL | Plan |

---

## Top-Empfehlungen für Discuss-Phase v1.2

1. **Auto-Anlegen-Phase-Strategie:** A vs. B vs. C aus Kategorie 2 fixieren — bestimmt die ganze Teilrückgabe-Pipeline.
2. **H1/H2-Grenze:** im Code-Kommentar fixieren (Monat 1–6 / 7–12), nicht implizit.
3. **ActionType-Naming:** Deutsch (Übertragung-Aus/Ein, Aufstockung) oder English (TransferOut/In, Increase)? Bestehende Enum-Werte sind gemischt — Konvention vor Plan-Phase fixieren.
4. **Datepicker-Bounds:** nur aktuelles GJ ODER auch nächstes (für H2-Wirksamkeit)?
5. **Sub-Choice-Form:** 4 Buttons flat vs. 3 mit Nesting (Reduzieren → Genossenschaft/Mitglied) vs. Kündigungs-Quickpath — Design-Doc-offene-Frage.

---

*Pitfalls research for: Genossi v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres*
*Researched: 2026-06-04*
