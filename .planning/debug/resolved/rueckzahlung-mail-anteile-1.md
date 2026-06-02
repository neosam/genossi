---
slug: rueckzahlung-mail-anteile-1
status: resolved
trigger: |
  DATA_START
  Bug: Beim Schreiben einer Mail im Rückzahlungs-Kontext wird die Anzahl Anteile
  immer als 1 angezeigt, unabhängig vom tatsächlichen Wert beim Mitglied.
  DATA_END
created: 2026-06-02T06:33:16Z
updated: 2026-06-02T06:56:00Z
resolved_via: quick-260602-c19
goal: find_and_fix
tdd_mode: false
---

# Debug Session: rueckzahlung-mail-anteile-1

## Symptoms

**Expected behavior**
DATA_START
Die Mail-Vorschau zeigt die tatsächliche Anzahl Anteile, die das ausgewählte
Mitglied in der DB hat (z.B. 3 oder 5 Anteile).
DATA_END

**Actual behavior**
DATA_START
Die Mail-Vorschau zeigt immer "1 Anteil" — egal welches Mitglied selektiert
wurde, egal wie viele Anteile das Mitglied tatsächlich besitzt.
DATA_END

**Reproduction**
DATA_START
1. Aufruf von http://localhost:8080/repayment-phases/a4448599-b697-4f24-aedb-02078fb3a613
2. Ein oder mehrere Mitglieder in der Liste markieren
3. Auf "Mail senden" klicken → Mail-Dialog öffnet sich
4. Vorschau anstoßen
5. Beobachtung: Anzahl Anteile = 1 (statt z.B. 3)
DATA_END

**DB-Stand**
DATA_START
Das/die selektierte/n Mitglied/er haben in der DB nachweislich mehr als 1 Anteil.
Der Bug ist nicht datengetrieben — es ist eine Code-/Template-/Mapping-Frage.
DATA_END

**Timeline**
DATA_START
Unbekannt seit wann der Bug existiert. Nutzer hat es heute beim Testen
bemerkt; ob es früher schon kaputt war, ist unklar.
DATA_END

**Error messages**
DATA_START
Keine Fehler/Exceptions — die Mail wird erfolgreich gerendert, nur mit
falschem Anteils-Wert (1 statt N).
DATA_END

## Current Focus

- hypothesis: Preview-Endpoint `POST /api/mail/preview` ruft `merge_repayment_context` mit hartkodierten Dummy-Werten `"60,00"`, `1`, `2026` auf (genossi_mail/src/rest.rs:505). Die Var `{{ share_count }}` rendert deshalb in der Vorschau IMMER mit 1, unabhängig vom Mitglied. Im echten Versand-Pfad (Worker) wird `share_count` korrekt aus den RepaymentEntries des Mitglieds aggregiert (genossi_mail/src/worker.rs:349-360).
- test: Vorschau für einen Member mit Repayment-Phase rendern → erwartet Anteils-Summe aus den Open/Contacted RepaymentEntries; tatsächlich kommt `1`.
- expecting: Symptom verschwindet, sobald der Preview-Endpoint die echten share_count_to_pay_out-Werte aus repayment_entry_dao auf Basis der `phase_id` + `member_id` summiert (analog Worker-Logik).
- next_action: Fix wählen (Option A: echte Aggregation im Preview / Option B: Sentinel-Wert im UI deutlich machen).
- reasoning_checkpoint: confirmed
- tdd_checkpoint: (n/a — TDD mode off)

## Evidence

- timestamp: 2026-06-02T cycle 1
  source: genossi-frontend/src/page/repayment_phase_details.rs:247-261
  finding: `on_mail_request` ruft `build_mail_redirect_url(phase_id, &ids)` → Redirect zu `/mail?from=repayment&phase_id=…&members=…`. Nur Member-IDs werden übergeben, keine Entry-IDs oder Anteils-Werte. Korrekt — die Anteile sollen serverseitig aus dem Repayment-Kontext aufgelöst werden.

- timestamp: 2026-06-02T cycle 1
  source: genossi-frontend/src/page/mail_page.rs:55-78, 438
  finding: MailPage parst Query-Params synchron, setzt `repayment_phase_id` und reicht es an `TemplatePreview { repayment_phase_id: *repayment_phase_id.read() }` durch.

- timestamp: 2026-06-02T cycle 1
  source: genossi-frontend/src/component/mail_compose/template_preview.rs:25-38
  finding: TemplatePreview ruft `api::preview_mail(&config, &subj, &b, &mid_str, repayment_phase_id)` → POST /api/mail/preview mit `{ subject, body, member_id, repayment_phase_id }`.

- timestamp: 2026-06-02T cycle 1
  source: genossi-frontend/src/component/mail_compose/template_var_buttons.rs:31-35
  finding: Orange "Anteile"-Button im Repayment-Flow fügt `{{ share_count }}` ein (NICHT `{{ current_shares }}`). Das ist die Repayment-Variable, die im Preview falsch befüllt wird.

- timestamp: 2026-06-02T cycle 2 — ROOT CAUSE
  source: genossi_mail/src/rest.rs:494-508
  finding: Preview-Handler ruft `merge_repayment_context(base_ctx, "60,00", 1, 2026)` mit hartkodierten Dummy-Werten auf. Kommentar (rest.rs:152-153) bestätigt: `share_count` (`1`) und `fiscal_year` (Phase.fiscal_year, dummy `2026`). Diese Dummies waren ursprünglich als Workaround für "render darf nicht crashen" gedacht (UAT-Defekt #6), wurden aber NIE durch echte Per-Member-Aggregation ersetzt.

- timestamp: 2026-06-02T cycle 2 — VERIFICATION
  source: genossi_mail/src/worker.rs:332-361
  finding: Der echte Send-Worker macht es richtig: Lädt RepaymentEntries via `repayment_entry_dao.find_by_phase_id(phase_id, tx)`, filtert auf `member_id == member.id && status IN (Open|Contacted) && deleted IS NULL`, summiert `share_count_to_pay_out`, berechnet `cents = share_count * phase.share_value` und ruft `merge_repayment_context(ctx, &payout_amount, share_count, phase.fiscal_year)`. Die Preview hat keinen vergleichbaren Code-Pfad.

- timestamp: 2026-06-02T cycle 2 — SCOPE-Bestätigung
  source: rg -n "merge_repayment_context" --type rust
  finding: Es gibt nur EINE produktive Aufrufstelle ausserhalb von Tests mit Dummy-Werten: rest.rs:505. Der Worker (worker.rs:355) übergibt echte Werte. → Bug ist sauber auf den Preview-Pfad eingegrenzt.

- timestamp: 2026-06-02T cycle 2 — STATE-Lücke
  source: genossi_mail/src/rest.rs:25-48, genossi_bin/src/lib.rs:1399-1506
  finding: `MailRestState`-Trait hat KEINE Methoden für RepaymentPhase- oder RepaymentEntry-Lookup. Um den Preview korrekt zu fixen, muss das Trait um (z.B.) `resolve_repayment_share_count(phase_id, member_id) -> Option<(share_count, payout_amount, fiscal_year)>` erweitert werden, oder der Aggregations-Helper aus dem Worker wird extrahiert und im genossi_bin-Wiring beim Preview aufgerufen.

## Eliminated Hypotheses

- "Frontend hardcodet 1": ✗ Frontend reicht nur member_ids + phase_id durch. Kein hardcoded 1 im Frontend gefunden.
- "Member.current_shares wird ignoriert": ✗ `current_shares` ist als eigene Template-Var verfügbar (template.rs:32) und liefert den DB-Wert. Aber der orange "Anteile"-Button im Repayment-Kontext fügt `{{ share_count }}` ein, nicht `{{ current_shares }}`. Das Symptom liegt also nicht an `current_shares` sondern an der Repayment-spezifischen `share_count`-Var.
- "Send-Pfad ist auch betroffen": ✗ Worker-Code (worker.rs:349-360) summiert korrekt aus RepaymentEntries. Bug ist isoliert auf den Preview-Endpoint.

## Resolution

- root_cause: |
  Der Mail-Preview-Endpoint (`POST /api/mail/preview` in `genossi_mail/src/rest.rs:494-508`) ruft
  `merge_repayment_context(base_ctx, "60,00", 1, 2026)` mit hartkodierten Dummy-Werten auf,
  wenn `repayment_phase_id` im PreviewRequest gesetzt ist. Die Repayment-Template-Variable
  `{{ share_count }}` (eingefügt vom orangen "Anteile"-Button im Repayment-Flow) rendert
  deshalb in der Vorschau IMMER mit `1`, unabhängig vom selektierten Mitglied.

  Im echten Versand-Pfad (`genossi_mail/src/worker.rs:332-361`) berechnet der Worker
  `share_count` dagegen korrekt: Er lädt die RepaymentEntries der Phase, filtert auf
  `member_id` + `status IN (Open|Contacted)`, summiert `share_count_to_pay_out` und
  übergibt das Ergebnis an `merge_repayment_context`. Der Preview-Endpoint dupliziert
  diese Logik NICHT — er nutzt seit UAT-Defekt #6 nur "Sentinel-Werte" damit das Rendern
  nicht crasht.

- fix: (pending — Option-Wahl durch User offen)

  Empfohlener Fix: **Option A — Preview an Worker-Logik angleichen**

  1. `MailRestState`-Trait in `genossi_mail/src/rest.rs` um eine neue Methode erweitern:
     ```rust
     fn resolve_repayment_context(
         &self,
         phase_id: uuid::Uuid,
         member_id: uuid::Uuid,
     ) -> Pin<Box<dyn Future<Output = Option<(String /*payout_amount*/, i32 /*share_count*/, i32 /*fiscal_year*/)>> + Send + '_>>;
     ```
     Liefert `None`, falls Phase nicht gefunden oder keine Open/Contacted-Entries existieren.

  2. Im Preview-Handler (rest.rs:494-508) statt der hartkodierten Dummies:
     ```rust
     let ctx = if let Some(phase_id_str) = body.repayment_phase_id.as_deref() {
         if !phase_id_str.is_empty() {
             let phase_id = uuid::Uuid::parse_str(phase_id_str)
                 .map_err(|_| MailServiceError::BadRequest(Arc::from("Invalid repayment_phase_id")))?;
             match state.resolve_repayment_context(phase_id, member_id).await {
                 Some((payout, count, year)) => merge_repayment_context(base_ctx, &payout, count, year),
                 // D-05-Symmetrie: falls keine relevanten Entries → kein merge, Template
                 // muss `{% if share_count is defined %}` verwenden. Alternativ als Sentinel
                 // `merge_repayment_context(base_ctx, "0,00", 0, phase.fiscal_year)` rendern.
                 None => base_ctx,
             }
         } else { base_ctx }
     } else { base_ctx };
     ```

  3. In `genossi_bin/src/lib.rs` (ab Zeile 1399 `impl MailRestState for RestStateImpl`)
     die Methode implementieren — analog Worker-Logik:
     ```rust
     fn resolve_repayment_context(
         &self,
         phase_id: UuidType,
         member_id: UuidType,
     ) -> Pin<Box<dyn Future<Output = Option<(String, i32, i32)>> + Send + '_>> {
         let pool = self.pool.clone();
         Box::pin(async move {
             use genossi_dao::TransactionDao as _;
             use genossi_dao::repayment_phase::RepaymentPhaseDao as _;
             use genossi_dao::repayment_entry::RepaymentEntryDao as _;
             let transaction_dao = TransactionDaoImpl::new(pool.clone());
             let phase_dao = RepaymentPhaseDaoImpl::new(pool.clone());
             let entry_dao = RepaymentEntryDaoImpl::new(pool);
             let tx = transaction_dao.transaction().await.ok()?;
             let phase = phase_dao.find_by_id(phase_id, tx.clone()).await.ok()??;
             let entries = entry_dao.find_by_phase_id(phase_id, tx).await.ok()?;
             let share_count: i32 = entries.iter()
                 .filter(|e| e.deleted.is_none()
                     && e.member_id == member_id
                     && matches!(e.status, RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted))
                 .map(|e| e.share_count_to_pay_out)
                 .sum();
             if share_count == 0 { return None; }
             let cents = (share_count as i64) * phase.share_value;
             let payout = format!("{},{:02}", cents / 100, cents % 100);
             Some((payout, share_count, phase.fiscal_year))
         })
     }
     ```

  4. Bonus-Refactor (optional, getrennter Commit): Aggregations-Logik aus `worker.rs:332-361`
     in einen wiederverwendbaren Helper extrahieren, damit Worker + Preview garantiert
     identisch rechnen (DRY → keine Drift mehr).

  5. **Tests**: In `genossi_mail/src/template.rs` (oder eigenem Modul) Unit-Test, der
     `merge_repayment_context` mit echten Werten durchprobt. E2E-Test in
     `genossi_bin/tests/`, der einen Member mit 3 Anteilen + RepaymentPhase + Entries
     anlegt und den `/api/mail/preview`-Response asserted (`body` enthält "3" und nicht "1").

  **Alternative Optionen** (weniger empfohlen):
  - **Option B — Sentinel im UI**: Die hartkodierten Dummies bleiben, aber TemplatePreview
    zeigt einen Hinweis: "Vorschau-Werte: 1 Anteil, 60,00 EUR (Dummy)". Spart Backend-Arbeit,
    bricht aber den Nutzererwartungswert "Vorschau zeigt mein konkretes Mitglied".
  - **Option C — Frontend rechnet vor**: Frontend lädt RepaymentEntries per separater API,
    aggregiert, und übergibt `share_count` als Override-Param an `/preview`. Erhöht
    API-Surface unnötig und dupliziert Aggregations-Logik im Client.

- fix: |
  Implementiert via quick-260602-c19 (Option A — Preview an Worker-Logik angleichen):
  1. `MailRestState`-Trait erweitert um `resolve_repayment_context(phase_id, member_id) -> Option<(payout, share_count, fiscal_year)>` (`genossi_mail/src/rest.rs:58-65`).
  2. Preview-Handler (`genossi_mail/src/rest.rs:524-540`) ruft die neue Methode statt der hartkodierten Dummies; bei `None` bleibt `base_ctx` unverändert (D-05-Symmetrie).
  3. `RestStateImpl::resolve_repayment_context` (`genossi_bin/src/lib.rs:1399ff`) dupliziert die Worker-Aggregation 1:1: Filter `deleted IS NULL && member_id == X && status IN (Open|Contacted)`, summiert `share_count_to_pay_out`, `cents = share_count * phase.share_value`, German-Format `format!("{},{:02}", cents/100, cents%100)`.

- verification: |
  ✓ Worker-Parity per grep verifiziert: `RepaymentEntryStatus::(Open|Contacted)` + `format!("{},{:02}"` in beiden Dateien (worker.rs und lib.rs) present.
  ✓ 0 hits für alte Dummy-Werte (`"60,00"`, `merge_repayment_context(.., 1, 2026)`) in rest.rs.
  ✓ cargo build/clippy clean für genossi_mail + genossi_bin.
  ✓ cargo test -p genossi_mail: 128/128 grün.
  ✓ cargo test -p genossi_bin: 294 e2e + 7 repayment_letter, alle grün.
  ✓ Zwei neue E2E-Regression-Tests in genossi_bin/tests/e2e_tests.rs:
    - Positiv: Member mit 3 Anteilen → Preview rendert "3" + "180,00" + "2026"
    - Negativ (D-05): Member ohne Open/Contacted-Entries → kein Fallback auf 1

  Commits:
  - d627e96: fix(mail): correct repayment share_count in preview endpoint (#260602-c19)
  - 1e48b2f: test(mail): add E2E regression tests for repayment preview (#260602-c19)

  Manueller UI-Re-Test (lokales `dx serve`): noch durch User durchzuführen.

- files_changed:
  - genossi_mail/src/rest.rs (Trait-Erweiterung + Preview-Handler)
  - genossi_bin/src/lib.rs (Impl der Trait-Methode in RestStateImpl)
  - genossi_bin/tests/e2e_tests.rs (+2 Regression-Tests)
  - .planning/quick/260602-c19-fix-mail-preview-repayment-kontext-share/ (PLAN.md, SUMMARY.md)
