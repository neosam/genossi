---
slug: kuendigung-repayment-missing
status: resolved
trigger: |
  <<DATA_START>>
  Bug: Bei Mitgliedschaft anpassen werden gekündigte Mitglieder nicht in die Repayment-Periode eingefügt.
  <<DATA_END>>
created: 2026-06-08T07:33:11Z
updated: 2026-06-08T12:17:06Z
---

# Debug Session: kuendigung-repayment-missing

## Symptoms

**Expected behavior:**
Wenn ein Mitglied als "gekündigt" markiert wird, sollte (analog zur Teilrückgabe) ein Eintrag in der aktuellen offenen Repayment-Periode erzeugt werden — und ggf. die Periode automatisch geöffnet werden.

**Actual behavior:**
- Markierung als "gekündigt" → es wird nur eine MemberAction erzeugt, aber KEIN Eintrag in der Repayment-Periode.
- Markierung als "Teilrückgabe" → Eintrag in Repayment-Periode wird erzeugt, Periode ggf. automatisch eröffnet/geöffnet.

**Error messages:**
Keine Fehlermeldungen — stilles Fehlverhalten (fehlender Eintrag).

**Timeline:**
Unbekannt — User hat das beim Bedienen entdeckt. Frage des Users: "Aber dann werden die gekündigten Mitglieder nie in die Repayment Periode eingefügt, oder?" (impliziert: User vermutet das Verhalten, ist sich aber nicht sicher → muss erst verifiziert werden, ob es tatsächlich ein Bug ist).

**Reproduction:**
1. Mitglied auswählen
2. "Mitgliedschaft anpassen" / Status auf "gekündigt" setzen
3. Erwartet: Eintrag in offener Repayment-Periode (analog Teilrückgabe-Pfad)
4. Beobachtet: Nur MemberAction wird angelegt, kein Repayment-Eintrag

## Vermutete Stellen (Hinweis vom User)

- Member-Service (Status-Übergang zu gekündigt) im `genossi_service_impl`
- Logik für Repayment-Periode (vermutlich beim Teilrückgabe-Pfad implementiert, fehlt beim Kündigungs-Pfad)
- REST-Handler / Frontend-Action "Mitgliedschaft anpassen"

## Current Focus

- hypothesis: BESTÄTIGT — `cancel_membership` erzeugt KEINEN RepaymentEntry. Cancelled members landen nur dann in einer Phase, wenn die Phase NACH der Kündigung von Preparation → Open transitioniert wird. Ist die Phase für das relevante fiscal_year bereits `Open`, wird der gekündigte Member NIE in dieser Phase erscheinen.
- test: ABGESCHLOSSEN — Code-Pfade `cancel_membership` vs. `partial_repayment` vs. `open_repayment_phase` Auto-Fill verglichen.
- expecting: ERFÜLLT — Beim Teilrückgabe-Pfad gibt es Auto-Phase-Create + RepaymentEntry-Create; beim Kündigungs-Pfad fehlt das.
- next_action: Fix planen — Option A: in `cancel_membership` denselben Phase-Resolve+RepaymentEntry-Create-Block einfügen wie in `partial_repayment` (mit `share_count_to_pay_out = member.current_shares`). Option B: in `open_repayment_phase` zusätzlich beim Update einer bereits `Open`-Phase einen Auto-Fill-Pass ausführen (weniger sauber). User-Entscheidung erforderlich.

## Evidence

- timestamp: 2026-06-08T07:50:00Z
  source: code-inspection
  file: `genossi_service_impl/src/membership_adjust.rs`
  lines: 83-170 (`cancel_membership`)
  finding: |
    `cancel_membership` führt aus:
    1. Permission/Datum-Validierung.
    2. Member existence + Already-Cancelled-Check.
    3. `compute_effective_date(willensbekundung_date)` berechnet H1/H2-Stichtag → liefert `fiscal_year` + `effective_date` (31.12. dieses Jahres).
    4. `audited_create!` für `MemberAction { action_type: Austritt, effective_date: Some(effective.effective_date), shares_change: 0 }`.
    5. `recalc_dates(member_id)` setzt `Member.exit_date`.
    6. Tx-Commit.

    KEIN Aufruf an `repayment_phase_dao` oder `repayment_entry_dao`. Kein `RepaymentEntry` wird erzeugt.

- timestamp: 2026-06-08T07:50:00Z
  source: code-inspection
  file: `genossi_service_impl/src/membership_adjust.rs`
  lines: 288-472 (`partial_repayment`)
  finding: |
    `partial_repayment` HAT die fehlende Logik:
    - Step 9 (Z. 349-411): `ensure_repayment_phase` — sucht existierende Phase per fiscal_year, oder legt sie inline mit Status `Open` an (`audited_create!` auf `repayment_phase_dao` mit Process-String `"repayment-phase.create"`).
    - Step 12 (Z. 438-457): `audited_create!` auf `repayment_entry_dao` mit Status `Open` und `share_count_to_pay_out = shares`.
    - Step 5 (Z. 318-328): explizite Blockade von gekündigten Members mit Conflict-Error und Kommentar:
        `// PART hat explizit eigene Semantik: gekuendigte Members gehen via v1.1-PaidOut-Cascade in die naechste Auszahlungsphase`
    Die Design-Intention war also: cancelled-Members werden NICHT über `partial_repayment` behandelt, sondern über einen anderen Mechanismus (siehe nächste Evidence).

- timestamp: 2026-06-08T07:50:00Z
  source: code-inspection
  file: `genossi_service_impl/src/repayment_phase.rs`
  lines: 319-423 (`open_repayment_phase` Auto-Fill-Block)
  finding: |
    Der "andere Mechanismus" für cancelled-Members ist der Auto-Fill in `open_repayment_phase`:
    - Filter (Z. 358): `m.exit_date.is_some_and(|d| d >= fy_start && d <= fy_end)` AND `m.current_shares > 0`.
    - Für jeden Treffer: `audited_create!` auf `repayment_entry_dao` mit `share_count_to_pay_out = member.current_shares`.
    - Skip-Pattern (Z. 389-395): überspringt Members, die bereits einen Entry in der Phase haben (PaidOut/Open/Contacted).

    **KRITISCH:** Dieser Auto-Fill läuft NUR beim State-Transition Preparation → Open (Z. 297-302). Ist die Phase bereits `Open` (z.B. weil sie zuvor durch `partial_repayment` auto-angelegt wurde oder ein Admin sie bereits geöffnet hat), wird der Auto-Fill NICHT erneut ausgelöst — auch nicht beim Update.

- timestamp: 2026-06-08T07:50:00Z
  source: code-inspection
  file: `genossi_service_impl/src/membership_adjust.rs`
  lines: 493-694 (`transfer_shares` Voll-Übertrag)
  finding: |
    Der Voll-Übertrag-Branch (Z. 629-657) erzeugt bei `will_become_zero` ebenfalls nur eine `MemberAction::Austritt` mit `effective_date = Some(transfer_date)` und ruft `recalc_dates` auf — KEIN RepaymentEntry. Gleicher Bug-Pfad wie `cancel_membership`.

- timestamp: 2026-06-08T07:50:00Z
  source: code-inspection
  file: `genossi_bin/tests/membership_adjust_e2e.rs`
  finding: |
    Kein einziger E2E-Test deckt das Szenario "Phase bereits Open → dann cancel_membership → erwarte neuen RepaymentEntry" ab. Auch in `genossi_service_impl/src/membership_adjust.rs` (service_tests) keine Coverage. Das Problem ist deshalb bisher in CI nicht aufgefallen.

## Root Cause

**Strukturell:** Es gibt **drei** Code-Pfade, die `MemberAction::Austritt` + `exit_date` erzeugen können:
1. `cancel_membership` — manuelle Kündigung.
2. `transfer_shares` (Voll-Übertrag-Branch) — Mitglied wird durch Übertrag auf 0 Anteile reduziert.
3. (potenziell) reguläre `MemberAction`-Creates über `MemberActionService::create` mit `action_type = Austritt`.

Repayment-Entries für ein gekündigtes Mitglied werden NUR durch den **Auto-Fill in `open_repayment_phase`** angelegt. Dieser läuft genau einmal beim `Preparation → Open`-Übergang.

**Konkrete Lücke:** Wenn eine `RepaymentPhase` für das durch `compute_effective_date(willensbekundung_date)` ermittelte `fiscal_year` zum Zeitpunkt der Kündigung bereits `Open` ist, gibt es **keinen** Mechanismus, der den nun gekündigten Member nachträglich in diese Phase einträgt. Der Member bleibt in dieser Phase unsichtbar.

Da `partial_repayment` Phasen mit Status `Open` per Auto-Create erzeugt (Z. 388), tritt diese Konstellation in der Praxis sofort auf, sobald nach einer Teil-Rückgabe (oder einem manuellen Phase-Open) ein anderes Mitglied im selben fiscal_year gekündigt wird.

**Bestätigt:** Die User-Vermutung ist korrekt — gekündigte Mitglieder landen NICHT zuverlässig in der Repayment-Periode.

## Eliminated

- ❌ Frontend-Bug: Wir haben verifiziert, dass der REST-Handler `cancel_membership` (Z. 52-81 in `genossi_rest/src/membership_adjust.rs`) korrekt den Service ruft. Service ist der Ursachen-Ort.
- ❌ "Cancelled wird absichtlich blockiert auf Repayment-Path und kein anderer Mechanismus übernimmt": Es GIBT einen anderen Mechanismus (`open_repayment_phase`-Auto-Fill), aber er deckt nicht alle Reihenfolgen ab.

## Specialist Hint

`rust` — Backend-Service-Layer-Fix mit Audit-Macro-Konventionen, ggf. neue Service-Test-Cases.

## Vorgeschlagener Fix (empfohlen: Option A)

**Option A — Recommended (analog zu `partial_repayment` Step 9/12):**

In `cancel_membership` (nach `recalc_dates`, vor `commit`) den Phase-Resolve+RepaymentEntry-Create-Block einfügen:

```rust
// Nach recalc_dates(member_id) und vor self.transaction_dao.commit(tx):

// Skip wenn Member tatsächlich 0 Anteile hat (Edge-Case: bereits leerer Member,
// z.B. nach Voll-Übertrag — kein Auszahlungs-Bedarf).
if updated_entity.current_shares > 0 {
    // Phase für effective.fiscal_year auflösen oder anlegen (analog
    // partial_repayment Step 9; gleiche Inlining-Strategy D-16-04).
    let all_phases = self.repayment_phase_dao.all(tx.clone()).await?;
    let target_phase_existing = all_phases
        .iter()
        .find(|p| p.fiscal_year == effective.fiscal_year)
        .cloned();

    // Closed-Phase = harter Fehler (D-11.1 Status-Guard, analog
    // partial_repayment).
    if let Some(ref existing) = target_phase_existing {
        if existing.status == RepaymentPhaseStatus::Closed {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "RepaymentPhase fiscal_year={} ist Closed — Kuendigung kann nicht eingetragen werden",
                effective.fiscal_year
            ))));
        }
    }

    let target_phase = match target_phase_existing {
        Some(p) => p,
        None => {
            let share_value = all_phases
                .first()
                .map(|p| p.share_value)
                .unwrap_or(DEFAULT_SHARE_VALUE_CENT);
            let auto_phase = RepaymentPhaseEntity {
                id: self.uuid_service.new_v4().await,
                fiscal_year: effective.fiscal_year,
                share_value,
                status: RepaymentPhaseStatus::Open,
                opened_at: Some(time::PrimitiveDateTime::new(now.date(), now.time())),
                closed_at: None,
                created: time::PrimitiveDateTime::new(now.date(), now.time()),
                deleted: None,
                version: self.uuid_service.new_v4().await,
            };
            crate::audited_create!(
                self,
                self.repayment_phase_dao,
                &auto_phase,
                REPAYMENT_PHASE_CREATE_PROCESS, // bereits in dieser Datei deklariert
                &user_id,
                tx
            );
            auto_phase
        }
    };

    // Skip-Pattern: wenn der Member bereits einen Entry in dieser Phase hat
    // (z.B. von einer vorherigen partial_repayment), keinen zweiten anlegen.
    // Gleiches Pattern wie open_repayment_phase Z. 389-395.
    let existing = self
        .repayment_entry_dao
        .find_by_member_and_phase(member_id, target_phase.id, tx.clone())
        .await?;
    if existing.is_empty() {
        let new_entry = RepaymentEntryEntity {
            id: self.uuid_service.new_v4().await,
            member_id,
            phase_id: target_phase.id,
            share_count_to_pay_out: updated_entity.current_shares,
            status: RepaymentEntryStatus::Open,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };
        crate::audited_create!(
            self,
            self.repayment_entry_dao,
            &new_entry,
            CANCEL_PROCESS,
            &user_id,
            tx
        );
    }
}
```

**Konsequenzen:**
- Idempotenz: Das Skip-Pattern verhindert Doppelung mit nachträglichem `open_repayment_phase` Auto-Fill.
- Audit-Trail: Process-String `member-adjust.cancel` macht klar, dass der Entry durch die Kündigung erzeugt wurde (nicht durch Auto-Fill `repayment-phase.open`).
- Symmetrie: `transfer_shares` Voll-Übertrag-Branch sollte denselben Block ebenfalls einfügen (Z. 629-657), sonst bleibt der Bug dort bestehen.

**Test-Coverage zu ergänzen:**
1. E2E: Phase in `Open` → `cancel_membership` → Phase muss einen neuen RepaymentEntry für den Member enthalten.
2. E2E: Keine Phase vorhanden → `cancel_membership` → Phase muss in `Open` angelegt werden + Entry vorhanden.
3. E2E: Phase in `Closed` → `cancel_membership` → 409 Conflict.
4. E2E: Member hat 0 Anteile (z.B. nach Voll-Übertrag, dann separat cancel) → kein Entry.
5. E2E: Member hat bereits Entry in Phase (durch früheren `partial_repayment`) → keine Doppelung.
6. Service-Unit-Tests analog.
7. Symmetrischer Test für `transfer_shares` Voll-Übertrag.

**Option B (nicht empfohlen):** Den Auto-Fill in `open_repayment_phase` auch beim Update auf bereits-Open-Phasen laufen lassen. Verändert eingespielte Semantik und Audit-Trail-Annahmen.

## Resolution

- timestamp: 2026-06-08T12:17:06Z
  resolved_by: Quick 260608-jb1
  commits:
    - 0e81e066 (test RED — 4 e2e tests für cancel_membership)
    - 66612231 (fix GREEN — cancel_membership creates RepaymentEntry)
    - 5070af33 (test RED — 1 e2e test für transfer_shares full)
    - e856ff4b (fix GREEN — transfer_shares full-transfer creates RepaymentEntry)
  summary: |
    Symmetrischer Fix in `cancel_membership` (genossi_service_impl/src/membership_adjust.rs Z. 83-260)
    und `transfer_shares` Voll-Uebertrag-Branch (Z. 685-790). Beide Code-Pfade fuehren jetzt analog
    `partial_repayment` Step 9+12 einen Phase-Resolve+Entry-Create-Block aus.

    Neue Audit-Process-Strings:
      - `member-adjust.cancel.repayment` fuer Cancel-erzeugte Entries
      - `member-adjust.transfer-full.repayment` fuer Voll-Uebertrag-erzeugte Entries

    Idempotenz ueber `find_by_member_and_phase().is_empty()`-Skip-Pattern (gleich wie
    open_repayment_phase Auto-Fill Z. 389-395). Closed-Phase -> 409 Conflict.
    5 neue E2E-Tests decken alle Pfade ab (Phase Open, Phase Auto-Create, Closed, Skip,
    transfer-full).

    Update am Step-5-Kommentar in `partial_repayment` (Z. 318-328): die alte "v1.1-PaidOut-Cascade"-
    Erlaeuterung war irrefuehrend; tatsaechlich landen cancelled-Members jetzt direkt im Entry, nicht
    ueber eine spaetere PaidOut-Cascade.
