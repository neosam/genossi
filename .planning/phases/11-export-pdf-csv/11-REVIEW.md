---
phase: 11-export-pdf-csv
reviewed: 2026-06-01T00:00:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
  - genossi_rest/src/lib.rs
  - genossi_rest/src/repayment_export.rs
  - genossi_rest/src/test_server.rs
  - genossi_service_impl/src/lib.rs
  - genossi_service_impl/src/pdf_generation.rs
  - genossi_service_impl/src/repayment_export.rs
  - genossi_service_impl/src/template_storage.rs
  - genossi_service/src/lib.rs
  - genossi_service/src/repayment_export.rs
  - templates/defaults/auszahlungsliste.typ
findings:
  blocker: 0
  warning: 6
  info: 4
  total: 10
status: issues_found
---

# Phase 11: Code Review Report — RepaymentExport PDF

**Reviewed:** 2026-06-01
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Phase 11 liefert einen funktionierenden, gut getesteten PDF-Export-Endpunkt fuer `RepaymentPhase`. Der Permission-Funnel (`load -> admin -> status`) ist korrekt implementiert, der `tx.commit()` erfolgt vor dem synchronen Typst-Render (Pitfall #8 mitigiert), die Audit-Macro-Freiheit (EXPO-05) ist sowohl via Source-Grep-Test als auch durch die Audit-Chain-E2E-Verifikation abgesichert, und das Verwendungszweck-Schema mit Original-Umlauten ist runtime + render verifiziert.

Es wurden **keine Blocker** gefunden — keine Crashes, Datenverlust-Risiken oder Security-Bypasses. Allerdings gibt es mehrere **Warnings** zu silent-data-loss-Szenarien, Information Disclosure und Datenkonsistenz, sowie kleinere Quality/Convention-Mängel.

## Warnings

### WR-01: Soft-deleted Member fuehrt zu unbemerktem Skip einer offenen Auszahlung

**File:** `genossi_service_impl/src/repayment_export.rs:216-224`
**Issue:** Wenn ein `RepaymentEntry` mit `status = Open` existiert, aber der referenzierte `Member` soft-deleted ist, wird der Entry **kommentarlos** uebersprungen — kein `tracing::warn!`, keine Sammlung in einer Diagnostik-Liste. Der Vorstand sieht die offene Auszahlung nicht im PDF und nimmt an "alle offenen Auszahlungen sind enthalten". D-02 nennt dieses Verhalten explizit, aber das ist eine Annahme ueber Sauberkeit der Datenpflege, die in der Praxis verletzt werden kann (Mitgliedsloeschung mit Action-Cascade-Bug, oder manuelle DB-Korrektur).
**Fix:**
```rust
if let Some(member) = member_opt {
    entry_member_pairs.push((entry, member));
} else {
    tracing::warn!(
        target: EXPORT_TARGET,
        entry_id = %entry.id,
        member_id = %entry.member_id,
        phase_id = %phase_id,
        "repayment_entry references soft-deleted member — skipping in export"
    );
}
```
Idealerweise zusaetzlich: count in den `tracing::info!`-Eintrag der Endlogzeile aufnehmen (`skipped_orphans = N`).

### WR-02: 404-vs-403-Differenz leakt Existenz von Phase-IDs an Non-Admins

**File:** `genossi_service_impl/src/repayment_export.rs:83-98`
**Issue:** Permission-Funnel macht `find_by_id` (404 wenn fehlend) BEFORE Admin-Gate (403 wenn nicht Admin). Ein authentifizierter Non-Admin kann durch UUID-Enumeration zwischen "Phase existiert nicht" (404) und "Phase existiert, aber du darfst nicht" (403) unterscheiden. Der Code-Kommentar spricht explizit von "Status-Information-Leak verhindern" (zu Pitfall #2), schweigt aber zur **Existenz**-Leak. Bei einem Vorstands-internen System mit Rate-Limiting ist die Severity niedrig — aber der Plan benennt es nicht als bewusste Trade-Off-Entscheidung.
**Fix:** Entweder dokumentieren als bewusst-akzeptiertes Tradeoff, oder Reihenfolge auf `admin-check -> find_by_id` umstellen (Phase 6 attendance_export hat dasselbe Pattern, also ggf. konsistent updaten).

### WR-03: Tx-`commit` schluckt fehlende Reads-Commit-Garantie still

**File:** `genossi_service_impl/src/repayment_export.rs:187-228` + `genossi_dao_impl_sqlite/src/transaction.rs:33-45`
**Issue:** `TransactionImpl::commit` ist ein No-Op, wenn `Arc::strong_count(&self.tx) != 1`. Im Export-Pfad existieren bis zu N Clones (`tx.clone()` in `check_admin_and_phase_status`, `find_by_phase_id`, sowie N x `find_by_id` im Loop). Wenn ein Clone aus irgendeinem Grund nicht gedropt wird (z.B. Future-Cancellation mitten im Loop), bleibt `commit()` silent success ohne tatsaechlichen DB-Commit. Bei reinen Reads ist das Schadensausmass null, aber das Fehlen einer `Result::Err`-Signalisierung versteckt einen potenziellen Bug-Vektor fuer zukuenftige Writes im Export-Pfad. Die Test-Helper `tx_dao_no_commit()` testet daher nur eine schwache Bedingung (`times(0..=1)`).
**Fix:** Im Service den Tx nicht klonen — stattdessen Tx by-value in DAO-Methoden geben und am Ende mit `transaction_dao.commit(tx).await?` committen. Wenn der Clone-Workflow nicht aufgegeben werden kann: `TransactionImpl::commit` sollte einen Error werfen, wenn `strong_count != 1` (statt silently success), damit zukuenftige Writes nicht im selben silent-skip-Pfad landen. Dies ist ein **codebase-weites Pattern** — Issue duplicate mit attendance_export und repayment_phase.

### WR-04: `share_value` hat kein DB-CHECK-Constraint > 0

**File:** `migrations/sqlite/*create_repayment_phase*.sql` + `genossi_service_impl/src/pdf_generation.rs:779-783`
**Issue:** Der Service-Layer validiert `share_value > 0` (`repayment_phase.rs:77`), aber die DB-Migration fehlt das `CHECK(share_value > 0)`. Wenn jemand `share_value = 0` oder `share_value = -1` direkt per SQL/migration einspielt, kann `build_inputs_repayment` einen `total_amount_str` wie `"-1,-50"` (Format-Bug bei negativen Cents wegen Rust's toward-zero division/modulo) ausgeben. Das ist defensive-coding-Schwaeche, nicht primaer ein Phase-11-Bug — aber Phase 11 baut auf der Annahme auf, dass `share_value > 0`.
**Fix:** Migration mit `CHECK(share_value > 0)` ergaenzen (analog zu `share_count_to_pay_out > 0` in `repayment_entry`). Sekundaer: `build_inputs_repayment`/`filter_and_enrich_rows` mit `i64::max(0, ...)` defensiv absichern oder explizit `debug_assert!(amount_cents >= 0)`.

### WR-05: PII-Leak via `#[instrument]` ohne Skip von `context` in OIDC-Build

**File:** `genossi_rest/src/repayment_export.rs:115-121`
**Issue:** `#[instrument(skip(rest_state))]` skipt `rest_state`, aber `context: Extension<Context>` wird via Debug geloggt. Im `oidc`-Build ist `Context = Option<AuthenticatedContext>`, und `AuthenticatedContext` enthaelt `claims: Option<Arc<str>>` (JSON-Session-Claims mit potenziell E-Mail, Username, Rollen). Bei Log-Level INFO/DEBUG leakt das PII in zentrale Log-Aggregation. Auch andere Handler (member.rs, attendance_export.rs) haben dasselbe Pattern — die Issue ist codebase-weit, aber Phase 11 erbt das.
**Fix:**
```rust
#[instrument(skip(rest_state, context))]
```
(Idealerweise `current_user_id` ueber `PermissionService` ziehen und als getrenntes Span-Field annotieren, falls Korrelation noetig ist.)

### WR-06: Pure-Function `filter_and_enrich_rows` macht keine Defense-in-Depth gegen negative cents

**File:** `genossi_service_impl/src/repayment_export.rs:142-171`
**Issue:** Der Kommentar erklaert, warum kein `.abs()` verwendet wird ("Domain-Constraint garantiert non-negative"), aber wenn diese Garantie irgendwo durchbricht (Mock-Test mit `share_count_to_pay_out = -1`, oder Future-Migration ohne CHECK), kann `amount_str = "-1,-50"` entstehen — und im PDF rendern. Es gibt keinen Unit-Test mit negativen Werten, der das aufdecken wuerde. Verbunden mit WR-04 (fehlende DB-Constraint auf `share_value`).
**Fix:** Entweder `debug_assert!(amount_cents >= 0, "negative cents — domain invariant violated")` einfuegen, oder ein Test-Case `test_negative_cents_panic_or_returns_error` hinzufuegen, der das Fehlverhalten dokumentiert.

## Info

### IN-01: Lokaler `Authentication::Full` Match-Arm im Service ist redundant

**File:** `genossi_service_impl/src/repayment_export.rs:91-98`
**Issue:** Der `match &context` macht einen frühen Pass fuer `Authentication::Full`, aber `PermissionService::check_permission` ist in `genossi_service_impl/src/permission.rs:33-34` bereits implementiert, dass `Authentication::Full => Ok(())` direkt zurueckgegeben wird. Der lokale Match dupliziert nur Logik, ohne Gewinn (eine async-call wird vermieden, der aber sofort `Ok(())` returnt). Macht den Code laenger und beschreibt einen "Trick", der in Wahrheit ein Codebase-Pattern ist.
**Fix:** `self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;` ohne Match-Arm. Falls Performance ein Argument waere (was es nicht ist, weil `check_permission` mit `Full` keinen DAO-Call macht), den Match dokumentieren.

### IN-02: ASCII-Schreibweise "Geschaeftsjahr" inkonsistent mit D-04/D-05-Spirit

**File:** `genossi_service_impl/src/pdf_generation.rs:786` + `templates/defaults/auszahlungsliste.typ:36`
**Issue:** Die UI-Strings im Title und Template-Header verwenden "Geschaeftsjahr" statt "Geschäftsjahr". D-04/D-05 fordern explizit "ORIGINAL-Umlauten — KEINE ASCII-Sanitization" — auch wenn diese Direktiven sich primaer auf den Verwendungszweck beziehen, ist die Inkonsistenz fuer ein deutsches Vorstandsdokument auffaellig. Der Vorstand bekommt PDFs, wo das deutsche Wort "Geschäftsjahr" als "Geschaeftsjahr" gedruckt wird, was unprofessionell wirkt.
**Fix:** Sowohl in `pdf_generation.rs:786` als auch `auszahlungsliste.typ:36` den Umlaut nutzen:
```rust
"title": format!("Auszahlungsliste Geschäftsjahr {}", phase.fiscal_year),
```
```typst
*Geschäftsjahr #meta.fiscal_year — #meta.row_count Auszahlung(en)*
```

### IN-03: `format_str` Path-Parameter ohne Laengen-Limit

**File:** `genossi_rest/src/repayment_export.rs:127-135`
**Issue:** `format_str: String` wird ohne Laengen-Validierung in `format!("unknown export format: {}", other)` interpoliert und in Response-Body + Logs geschrieben. Bei extremen Werten (z.B. 1MB URL-Path-Segment, wenn Axum/Hyper das durchlaesst) entstaeht log-bloat. Realistisch klein, weil Server-/Proxy-Limits greifen, aber das Pattern ist nicht eindeutig defensiv.
**Fix:**
```rust
"pdf" => ExportFormat::Pdf,
other if other.len() <= 16 => {
    return Err(RestError::BadRequest(format!("unknown export format: {}", other)))
}
_ => return Err(RestError::BadRequest("unknown export format".to_string())),
```

### IN-04: Doppelter `import std::path::PathBuf` im Test-Modul

**File:** `genossi_service_impl/src/repayment_export.rs:18, 289`
**Issue:** `use std::path::PathBuf` ist sowohl auf Modul-Ebene (Z. 18) als auch im Test-Modul (Z. 289) importiert. Der Test-Import ist redundant, weil `use super::*` (Z. 284) bereits den outer-Scope-Import erbt.
**Fix:** Den Test-Modul-Import auf Z. 289 entfernen.

## Notes auf Items, die **nicht** als Findings gelten

- **N+1-Loop in `find_by_id` per Entry** (`repayment_export.rs:216-219`): Performance-Pathologie ist OUT OF SCOPE fuer v1; zusaetzlich nutzt der Code die default-DAO-Impl, die `dump_all` aufruft — N x O(|members|) pro Export. Bereits in RESEARCH Q5 als "Discretion-Choice" dokumentiert.
- **Audit-Chain bleibt valid nach Export**: E2E-Test `test_export_repayment_does_not_break_audit_chain` + Grep-Gate-Test `no_audit_macros_used` verifizieren EXPO-05 sowohl runtime als auch compile-time. Solide.
- **Format-Whitelist (D-12)**: `pdf` ist die einzige Variante; `csv`, `xlsx`, `json`, `html` werden mit 400 abgewiesen — Test `test_export_repayment_unknown_format_returns_400` enumeriert vier Negative-Cases. Klean.
- **Verwendungszweck-Umlauten**: Sowohl Pure-Function-Unit-Test (`test_purpose_string_preserves_umlaut_per_d04`) als auch render-test (`test_render_repayment_list_with_two_rows`) verifizieren `'ü'` end-to-end. Negative-Assertion gegen ASCII-Variante per Laufzeit-`format!` — der Grep-Gate ist deterministisch erfuellbar.
- **IBAN leerer String (D-06/D-07)**: `m.bank_account.as_ref().map(|s| s.to_string()).unwrap_or_default()` korrekt; E2E-Test `test_export_repayment_empty_iban_renders_empty_column` verifiziert end-to-end.

---

_Reviewed: 2026-06-01_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
