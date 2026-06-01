---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
reviewed: 2026-06-02T00:00:00Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - .gitignore
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/repayment_letter_e2e.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/component/repayment_entry_list.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/page/repayment_phase_details.rs
  - genossi_rest/src/lib.rs
  - genossi_rest/src/repayment_letter.rs
  - genossi_rest/src/test_server.rs
  - genossi_service_impl/src/lib.rs
  - genossi_service_impl/src/pdf_generation.rs
  - genossi_service_impl/src/repayment_context.rs
  - genossi_service_impl/src/repayment_letter.rs
  - genossi_service_impl/src/template_storage.rs
  - genossi_service/src/lib.rs
  - genossi_service/src/member_document.rs
  - genossi_service/src/repayment_context.rs
  - genossi_service/src/repayment_letter.rs
  - templates/defaults/auszahlungs_anschreiben_bundle.typ
  - templates/defaults/auszahlungs_anschreiben.typ
findings:
  critical: 2
  warning: 6
  info: 4
  total: 12
status: issues_found
---

# Phase 13: Code Review Report — RepaymentLetter Bulk-Anschreiben

**Reviewed:** 2026-06-02
**Depth:** standard
**Files Reviewed:** 23
**Status:** issues_found

## Summary

Die Phase-13-Implementierung des RepaymentLetter-Bulk-Anschreiben-Pfads ist insgesamt strukturell solide aufgesetzt: Service-Layer-Funnel (load → admin → status), pure-fn Resolver-Aggregation, Read-Tx-vor-Render-Reihenfolge (Pitfall #2), explizite `MAX_ENTRY_IDS_PER_REQUEST`-DoS-Begrenzung, Audit-Macros statt Direkt-DAO-Calls, und Browser-`Url::revoke_object_url`-Pattern sind sauber umgesetzt. Die Test-Coverage ist ungewöhnlich gründlich (12 Unit-Tests im Service-Layer + 8 E2E-Tests).

Trotzdem habe ich **zwei BLOCKER** identifiziert, die vor dem Ship behoben werden müssen:

1. **Dioxus-Hook in async `spawn`-Closure** (`repayment_phase_details.rs:305`) — `use_i18n()` wird innerhalb eines spawnen async-Tasks aufgerufen. Dioxus-Hooks dürfen ausschließlich auf Component-Top-Level laufen; im Async-Task kann der Hook außerhalb des Render-Scopes panicen oder undefined behaviour auslösen, je nach Dioxus-Version.
2. **Verwaiste PDF-Dateien bei Schreibe-Tx-Rollback** (`repayment_letter.rs:296–354`) — file_save passiert pro Recipient VOR dem `commit(write_tx)` am Loop-Ende. Wenn der letzte audited_create fehlschlägt, werden alle vorherigen Files NICHT zurückgerollt, aber die DAO-Schreibungen schon. Das schafft Orphan-Files OHNE korrespondierendes MemberDocument — Inkonsistenz zwischen Storage und DB, die der Operator manuell aufräumen muss.

Daneben sechs WARNINGs (Cleanup nach Bundle-Render-Fehler nicht abgedeckt, Bulk-Limit-Diskrepanz Service-vs-Frontend, X-Document-Count-Header wird vom Browser via CORS standardmäßig nicht exponiert, Status-Code-Mismatch im 403-Test-Skeleton, `pub`-Felder erlauben Test-Bypass der Domain-Invarianten, Idempotenz-Verhalten erzeugt N Orphan-PDFs bei Re-Generate).

## Critical Issues

### CR-01: Dioxus-Hook im async `spawn`-Block (Hook-Rules-Verletzung)

**File:** `genossi-frontend/src/page/repayment_phase_details.rs:305`
**Issue:** Innerhalb des `on_letter_request`-Handlers wird `use_i18n()` in der async `spawn`-Closure aufgerufen, nachdem die Letter-Response zurückgekommen ist:

```rust
spawn(async move {
    let cfg = CONFIG.read().clone();
    match api::generate_repayment_letters(...).await {
        Ok(result) => {
            // ... browser-save logic ...
            let i18n_for_toast = use_i18n();  // <-- HOOK in async-Closure
            let toast_msg = if result.document_count == 1 { ... };
            show_toast(&mut toast_messages, &mut toast_counter, toast_msg);
        }
```

`use_i18n()` ist zwar nur ein GlobalSignal-Read (`I18N.read().clone()`), aber:

- Dioxus-Hooks dürfen lt. API-Vertrag NUR auf Component-Top-Level aufgerufen werden. Außerhalb des Render-Scopes kann `GlobalSignal::read()` je nach Dioxus-Version panicen oder einen Subscription-Track in der falschen Scope-Hierarchie anlegen.
- Selbst wenn aktuell keine Runtime-Panic auftritt: jeder zukünftige Refactor des i18n-Systems (z.B. zu Locale-Context statt globalem Signal) bricht diese Stelle silent.
- Die existierenden BasicsTab/ExportTab-Components haben dasselbe Pattern korrekt umgesetzt (`let i18n = use_i18n();` am Component-Top, dann via Move-Capture in die Closure).

**Fix:** Den i18n-Handle bereits am Component-Top-Level lesen (vor `rsx!`-Block) und per move-capture in die spawn-Closure übergeben:

```rust
// Direkt nach Zeile 123 (`let i18n = use_i18n();`):
let i18n_singular = i18n.t(Key::RepaymentLetterToastSingular).to_string();
let i18n_plural_template = i18n.t(Key::RepaymentLetterToastPlural).to_string();

// Dann in der spawn-Closure:
spawn(async move {
    // ... 
    Ok(result) => {
        // ... browser-save ...
        let toast_msg = if result.document_count == 1 {
            i18n_singular
        } else {
            i18n_plural_template.replace("{count}", &result.document_count.to_string())
        };
        show_toast(&mut toast_messages, &mut toast_counter, toast_msg);
    }
});
```

### CR-02: Verwaiste PDF-Dateien bei Audit-Create-Fehler im Schreibe-Tx-Loop

**File:** `genossi_service_impl/src/repayment_letter.rs:296-354`
**Issue:** Der Schreibe-Tx-Loop speichert pro Recipient zuerst das PDF auf dem Filesystem, dann persistiert das `MemberDocument` via `audited_create!`:

```rust
let write_tx = self.transaction_dao.use_transaction(None).await?;
for ((member, _ctx), (_mid, pdf_bytes)) in recipients.iter().zip(single_pdfs.iter()) {
    // ...
    self.document_storage.save(&relative_path, pdf_bytes).await.map_err(...)?;
    // ...
    crate::audited_create!(self, self.member_document_dao, ..., write_tx);
    document_ids.push(doc_id);
}
self.transaction_dao.commit(write_tx).await?;
```

Wenn `audited_create!` für Recipient #N fehlschlägt (z.B. UNIQUE-Constraint, Disk-Full, oder DB-Connection-Loss):

1. `?` propagiert den Fehler → Funktion returnt mit Err
2. `write_tx` wird gedroppt → SQLite rollbackt ALLE bisherigen INSERTs (#1..#N-1)
3. Aber die N-1 PDF-Files sind bereits via `document_storage.save()` ASYNC auf dem Filesystem gelandet — diese werden NIE rückgängig gemacht.

Ergebnis: N-1 orphan PDFs in `documents/` ohne korrespondierende `member_document`-Row und ohne Audit-Eintrag. Bei produktiven Re-Tries entsteht ein wachsender Orphan-Pool. Das ist:
- Inkonsistenz zwischen Storage und DB (Tip #1 Datenintegrität)
- DSGVO-relevant (PDFs enthalten Name, Adresse, IBAN — orphan files = nicht-getrackte personenbezogene Daten)
- Der inline-Kommentar erkennt das Problem (`"verwaiste Files bei DAO-Erfolg muss Operator ggf. aufraeumen"`), löst es aber nicht.

Selbst der „Best-Effort"-Cleanup nach DAO-Fehler fehlt komplett.

**Fix:** Files erst NACH erfolgreichem audited_create speichern, oder bei DAO-Fehler die bereits geschriebenen Files explizit löschen. Empfehlung Variante A (in-memory bis Tx-Commit):

```rust
// Sammeln statt direkt speichern
let mut planned_saves: Vec<(String, &[u8])> = Vec::new();
for ((member, _ctx), (_mid, pdf_bytes)) in recipients.iter().zip(single_pdfs.iter()) {
    let doc_id = self.uuid_service.new_v4().await;
    let relative_path = format!("{}.pdf", doc_id);
    
    // ... build new_doc with relative_path ...
    let doc_entity: MemberDocumentEntity = (&new_doc).into();
    crate::audited_create!(self, self.member_document_dao, &doc_entity, REPAYMENT_LETTER_PROCESS, &user_id, write_tx);
    
    planned_saves.push((relative_path, pdf_bytes));
    document_ids.push(doc_id);
}

self.transaction_dao.commit(write_tx).await?;

// NACH Commit: Files schreiben (orphan-DB-Rows bei File-Fehler tolerabler als orphan-Files)
for (path, bytes) in &planned_saves {
    self.document_storage.save(path, bytes).await.map_err(...)?;
}
```

Alternative Variante B mit Cleanup-on-Failure:

```rust
let mut written_paths: Vec<String> = Vec::new();
let result: Result<Vec<Uuid>, ServiceError> = async {
    let mut ids = Vec::with_capacity(recipients.len());
    for ... {
        self.document_storage.save(&relative_path, pdf_bytes).await.map_err(...)?;
        written_paths.push(relative_path.clone());
        // ... audited_create ...
        ids.push(doc_id);
    }
    Ok(ids)
}.await;

match result {
    Ok(ids) => {
        self.transaction_dao.commit(write_tx).await?;
        document_ids = ids;
    }
    Err(e) => {
        // Tx rolls back automatically on drop; clean up files
        for path in &written_paths {
            let _ = self.document_storage.delete(path).await; // best-effort
        }
        return Err(e);
    }
}
```

Hinweis: das Service-Test `test_generate_sequential_audited_create_pitfall_4` deckt nur den Happy-Path ab — es gibt KEINEN Test für „audited_create fails midway → cleanup".

## Warnings

### WR-01: Bundle-Render-Fehler nach Read-Tx-Commit lässt user_id-Resolution-Fehler unklar

**File:** `genossi_service_impl/src/repayment_letter.rs:268-293`
**Issue:** Reihenfolge:
1. `commit(read_tx)` (Zeile 268)
2. `resolve_user_id_or_deny(&context).await?` (Zeile 271)
3. Render Single-PDFs (Zeile 274-284)
4. Render Bundle-PDF (Zeile 288-293)

Wenn `resolve_user_id_or_deny` zwischen Read-Tx-Commit und Render fehlschlägt (z.B. transient OIDC-Glitch), wurde der Read-Tx schon committed (kein Datenproblem), aber ein 403 wird zurückgegeben. Das verbraucht im Fail-Fall trotzdem die Read-Tx-Ressource. Geringer Impact, aber der Pre-Validation-Comment auf Zeile 0 sagt explizit "vor jeder DB-Touche" — `resolve_user_id_or_deny` könnte VOR `use_transaction(None)` aufgerufen werden (bessere Defense-in-Depth).

**Fix:** `resolve_user_id_or_deny` an den Anfang von `generate` verschieben — vor `read_tx`-Acquire (Zeile 211). Das spart eine Tx-Round-Trip im 403-Failure-Pfad und hält die Reihenfolge der Validierungen konsistent (Auth zuerst, dann DB).

### WR-02: Bulk-Limit 200 vs. Frontend ohne clientseitige Begrenzung

**File:** `genossi_service_impl/src/repayment_letter.rs:62` (`MAX_ENTRY_IDS_PER_REQUEST: usize = 200`) und `genossi-frontend/src/component/repayment_entry_list.rs:259-282` (Letter-Button)
**Issue:** Das Backend lehnt Requests mit > 200 entry_ids mit `ValidationError`/400 ab — sinnvoll als DoS-Schutz. Aber das Frontend kennt diese Grenze NICHT:

- Der Bulk-Letter-Button hat KEINE Selection-Cap → Vorstand kann (theoretisch) 500 Einträge selektieren und auf "Anschreiben erzeugen" klicken
- Die Fehlermeldung kommt erst nach Round-Trip + ToastError mit unspezifischer 400-Message ("Ungültige Anfrage" im `status_to_message`)
- Es gibt KEINE i18n-Translation für die spezifische Bulk-Limit-Meldung

In der Praxis ist das selten, weil eine GV typischerweise <100 Austritte hat. Aber es ist eine UX-Erosion: User bekommt unklare Fehlermeldung statt einer disabled-Button-mit-Hinweis-UI.

**Fix:** Entweder:
- Backend-Grenze auf Frontend-Konstante teilen (z.B. via `rest_types::MAX_LETTER_BULK = 200`) und Button mit Selection-Counter-Tooltip "Maximal 200 Einträge pro Bulk-Anfrage" bei `selected_count > 200` disablen, ODER
- Backend gibt einen strukturierten 400-Body mit `{"error": "bulk_limit_exceeded", "max": 200}` zurück, den das Frontend in `parse_close_conflict`-Style parsed und übersetzt.

### WR-03: `X-Document-Count`-Header wird vom Browser CORS-bedingt NICHT exponiert

**File:** `genossi_rest/src/lib.rs:415` (CORS-Layer) und `genossi_rest/src/repayment_letter.rs:138` (Header-Setzung)
**Issue:** Der Backend setzt `X-Document-Count: N` und das Frontend liest ihn in `api.rs:2018-2024`:

```rust
let document_count: usize = resp
    .headers()
    .get("X-Document-Count")
    .ok()
    .flatten()
    .and_then(|s| s.parse::<usize>().ok())
    .unwrap_or(entry_ids_len);
```

Aber: nach W3C/Fetch-Spec macht der Browser benutzerdefinierte Response-Header NUR sichtbar, wenn sie via `Access-Control-Expose-Headers` im CORS-Response gelistet sind. Der `build_cors_layer` (`genossi_rest/src/lib.rs:377-416`) setzt KEIN `expose_headers` — er konfiguriert nur `allow_methods` und `allow_headers` (für Request-Header).

Konsequenz: In **Cross-Origin-Deployments** (z.B. Frontend auf `localhost:8080`, Backend auf `localhost:3000`) ist `resp.headers().get("X-Document-Count")` immer `None` — der Fallback `entry_ids_len` wird verwendet. Das BRICHT die D-13-04-Aggregation-Toast-Anzeige genau in dem Szenario, das die Header eigentlich lösen sollten: Bei 3 Entries für 2 Members wäre die Toast-Message "3 Briefe erzeugt" statt "2 Briefe erzeugt".

Same-Origin-Deployments (production behind Reverse Proxy) wären nicht betroffen. Die E2E-Tests in `repayment_letter_e2e.rs` testen direkt via `reqwest` ohne Browser-CORS-Enforcement, daher zeigen sie das Problem NICHT.

**Fix:** In `build_cors_layer`:

```rust
use tower_http::cors::ExposeHeaders;

CorsLayer::new()
    .allow_origin(AllowOrigin::list(origins))
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
    .expose_headers([
        http::HeaderName::from_static("x-document-count"),
        http::HeaderName::from_static("content-disposition"),  // gleicher Bug latent
    ])
```

Plus: explicit unit/integration-Test im Frontend-API-Layer, der das CORS-Header-Verhalten in echter Browser-Umgebung mockt.

### WR-04: `test_letter_helper_auth_returns_403` akzeptiert auch 5xx-Folge-Fehler

**File:** `genossi_bin/tests/repayment_letter_e2e.rs:570-577`
**Issue:** Der ignorierte Test enthält folgende Assertion:

```rust
let status = resp.status();
assert!(
    status == StatusCode::FORBIDDEN || status.is_client_error(),
    "Permission-Gate sollte 4xx (idealerweise 403) liefern; got {}",
    status
);
```

Selbst wenn der Test irgendwann reaktiviert wird (oder versehentlich, etwa via `cargo test --ignored`), würde er **JEDEN** 4xx-Statuscode akzeptieren — inklusive 401 (was bedeutet "Session ungültig") oder 400 (was bedeutet "Validation-Error"). Das ist KEINE 403-Verifikation; das ist eine "not-5xx"-Verifikation. Das ist gefährlich, weil der Test bei einer echten Regression (Server gibt jetzt 401 statt 403) keinen Alarm schlägt.

Der ganze Test ist außerdem mit unrealistischen Daten konstruiert (`entry_ids: [Uuid::new_v4()]` — Random-UUID nicht in Phase), sodass der Server vor dem Permission-Gate schon mit `entry_phase_mismatch`-400 antworten könnte (wenn `mock_auth` doch Admin ist, was der Test-Comment selbst zugibt).

**Fix:** Entweder den Test komplett entfernen oder zumindest die Assertion auf EXACT `StatusCode::FORBIDDEN` schärfen:

```rust
assert_eq!(
    status,
    StatusCode::FORBIDDEN,
    "Permission-Gate muss EXAKT 403 liefern (nicht 401, nicht 400); got {}",
    status
);
```

Wenn `mock_auth` keinen non-admin-Pfad bietet, ist der Test ohnehin nutzlos — dann lieber löschen mit Kommentar (nicht als `#[ignore]` skeleton, der False-Positive-Sicherheit suggeriert).

### WR-05: `RepaymentLetterServiceImpl`-Felder sind `pub` — bypassed DI-Constructor

**File:** `genossi_service_impl/src/repayment_letter.rs:92-105`
**Issue:** Alle Service-Felder sind `pub`:

```rust
pub struct RepaymentLetterServiceImpl<Deps: RepaymentLetterServiceDeps> {
    pub repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
    pub repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
    pub member_dao: Arc<Deps::MemberDao>,
    pub member_document_dao: Arc<Deps::MemberDocumentDao>,
    pub audit_log_dao: Arc<Deps::AuditLogDao>,
    pub permission_service: Arc<Deps::PermissionService>,
    pub transaction_dao: Arc<Deps::TransactionDao>,
    pub uuid_service: Arc<Deps::UuidService>,
    pub repayment_context_resolver: Arc<Deps::RepaymentContextResolver>,
    pub document_storage: Arc<Deps::DocumentStorage>,
    pub pdf_generator: Arc<PdfGenerator>,
    pub template_base: Arc<PathBuf>,
}
```

Das ist konsistent mit dem Codebase-Pattern (RepaymentExportServiceImpl, AttendanceServiceImpl etc. machen es genauso). Aber: Es gibt keinen Constructor, der invarianten enforced (z.B. "template_base muss existieren", "audit_log_dao muss der globale single-Arc sein"). Damit kann jeder External-Caller Felder per `..` Mutation oder durch Struct-Literal-Instantiation in Tests inkonsistent setzen — wie es z.B. `build_service_with_templates` (Zeile 988-1002) tatsächlich macht.

Konkrete Konsequenz: Wenn das Test-Setup `Arc::new(MockTestStorage)` als `document_storage` injiziert, aber die Production-Wiring `Arc::new(FilesystemDocumentStorage::from_env())` nutzt, gibt es keine Compile-Zeit-Garantie, dass die beiden Implementationen denselben Persistenz-Vertrag erfüllen. Bei Test-Pass ≠ Production-Pass.

Außerdem: Die Tests `build_service()` (Zeile 832) und `build_service_with_templates()` (Zeile 975) sind nahezu identisch — Pattern-Duplikation, die einen Refactor zu einem Constructor `RepaymentLetterServiceImpl::new(...)` rechtfertigen würde.

**Fix:**

1. Felder auf `pub(crate)` reduzieren (Production-Wiring in `genossi_bin/src/lib.rs` ist im selben Workspace, also `pub(crate)` reicht nicht — `pub(super)` oder echter `new()` Constructor)
2. ODER: Konsistenten Constructor anbieten:

```rust
impl<Deps: RepaymentLetterServiceDeps> RepaymentLetterServiceImpl<Deps> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
        // ... all 12 fields ...
    ) -> Self {
        // Optional: validate invariants (e.g. template_base.exists())
        Self { ... }
    }
}
```

3. Test-Helper `build_service` und `build_service_with_templates` zu einem einzigen mit `template_base: Option<Arc<PathBuf>>`-Param konsolidieren.

### WR-06: Idempotente Re-Generierung erzeugt N Storage-Files OHNE Cleanup-Strategie

**File:** `genossi_service_impl/src/repayment_letter.rs:295-352` (Schreibe-Tx-Loop) und `genossi_service/src/member_document.rs:87-94` (`is_singleton == false`)
**Issue:** D-13-08 erlaubt Re-Generierung explizit — jeder Bulk-Letter-Call erzeugt N neue MemberDocuments pro Member, jedes mit eigenem `id.pdf` im Storage. Der E2E-Test `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` (`repayment_letter_e2e.rs:782-866`) bestätigt: nach 2 Calls existieren 2 MemberDocuments + 2 PDFs für denselben Member.

Konsequenzen:

- Der Filesystem-Storage wächst linear mit der Anzahl der Re-Generierungen. Bei 50 Mitgliedern, 5 Korrekturen pro Phase = 250 PDFs pro Phase. Über mehrere Jahre summiert das zu signifikantem Storage-Wachstum.
- Keine Audit-Trail-Spur, dass die früheren Briefe „abgelöst" wurden — sie bleiben als gleichberechtigte Versionen sichtbar im UI.
- Frontend zeigt vermutlich ALLE im MemberDocument-List-Endpoint, sodass der Vorstand bei jeder Re-Korrektur N+1 fast-identische PDFs sieht und manuell entscheiden muss, welcher der "aktuelle" ist.
- DSGVO-Löschpflicht: bei Mitglieder-Lösch-Anfrage müssen ALLE N PDFs aufgeräumt werden, nicht nur das letzte.

Das ist KEIN Bug per se — D-13-08 ist intentional — aber die Implementierung berücksichtigt die Konsequenzen nicht. Die `RepaymentMail`-Variante (Phase 10) hat dasselbe Pattern, daher konsistent, aber das macht es nicht besser.

**Fix:** Eine der drei Strategien dokumentieren oder implementieren:

1. **Soft-delete frühere RepaymentLetter-Dokumente** bei Re-Generate für denselben Member in derselben Phase. Sinngemäß: `RepaymentLetter` ist „per (member_id, phase_id) singleton mit History" statt „nicht-singleton". audited_delete macht den Vorgang nachvollziehbar.
2. **Storage-Cleanup-Worker** der orphan-PDFs nach 30 Tagen aufräumt (löscht File aber behält DB-Row für Audit-Spur, oder umgekehrt).
3. **Frontend zeigt nur das jüngste RepaymentLetter pro Phase** und die anderen unter „Verlauf" — UI-only fix, ohne Backend-Änderung.

In jedem Fall: eine Komponenten-Test-Verifikation, dass beim Member-Delete-Cascade alle RepaymentLetter-Files mit aufgeräumt werden.

## Info

### IN-01: TempDir-Leak in Test-Helper `provision_template_base`

**File:** `genossi_service_impl/src/repayment_letter.rs:946-972`
**Issue:** Der Test-Helper leakt absichtlich das `TempDir`-Handle:

```rust
fn provision_template_base() -> Arc<PathBuf> {
    // ...
    std::mem::forget(dir);
    Arc::new(base)
}
```

Der Kommentar erkennt das („TempDir muss am Leben bleiben") — aber Cargo-Test-Runs in CI sammeln tempfiles in `/tmp`. Über viele Test-Iterationen (z.B. CI mit Hunderten von Runs/Tag) füllt sich `/tmp` mit `tempdir-*`-Resten. Auf manchen CI-Systemen ist `/tmp` durch tmpfs limitiert; das ist eine schleichende OOM-Quelle.

**Fix:** Statt `std::mem::forget(dir)`: TempDir als globale `static` mit `OnceLock<TempDir>` shared zwischen Tests (sodass nur EIN TempDir pro Testlauf entsteht und beim Process-Exit korrekt aufgeräumt wird):

```rust
use std::sync::OnceLock;
static TEMPLATE_BASE: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();

fn provision_template_base() -> Arc<PathBuf> {
    let (_, base) = TEMPLATE_BASE.get_or_init(|| {
        let dir = tempfile::tempdir().expect("tempdir");
        // ... write templates + logo ...
        let base = dir.path().to_path_buf();
        (dir, base)
    });
    Arc::new(base.clone())
}
```

### IN-02: Unused `_phase` Parameter in `build_inputs_repayment_letter`

**File:** `genossi_service_impl/src/pdf_generation.rs:976`
**Issue:** Die Funktion akzeptiert `_phase: &RepaymentPhaseEntity` und ignoriert es:

```rust
fn build_inputs_repayment_letter(
    _phase: &RepaymentPhaseEntity,  // unused
    member: &MemberEntity,
    ctx: &RepaymentContext,
) -> Dict {
```

Der Kommentar sagt "wird mitgereicht fuer kuenftige Erweiterungen". Aber: der Funktions-Signatur fehlt der `#[allow(dead_code)]`-Hint, und Konsumenten müssen das Phase-Argument konstruieren, obwohl es im Output nicht erscheint.

**Fix:** Entweder Parameter entfernen (YAGNI — Erweiterung ist nicht Phase-13-Scope) oder es im Output sichtbar machen (z.B. `phase_id` in den `member`-JSON-Block einbetten für künftige phase-spezifische Template-Anpassungen). Da das Bundle-Template `phase.fiscal_year` und `phase.id` schon nutzt (`build_inputs_repayment_letters_bundle:1052-1056`), wäre Symmetrie sinnvoll.

### IN-03: Inkonsistente Bezeichnung „Anschreiben Auszahlung GJ N" vs. Filename-Pattern

**File:** `genossi_service_impl/src/repayment_letter.rs:322-331`
**Issue:** Im Service:

```rust
description: Some(Arc::from(
    format!("Anschreiben Auszahlung GJ {}", phase.fiscal_year).as_str(),
)),
file_name: Arc::from(
    format!(
        "auszahlungs_anschreiben_{}_GJ_{}.pdf",
        member.member_number, phase.fiscal_year
    )
    .as_str(),
),
```

Das Bundle-PDF-Filename auf der Response (`auszahlungs_anschreiben_GJ_{fy}.pdf` ohne member_number) und das per-Member-MemberDocument-Filename (`auszahlungs_anschreiben_{nr}_GJ_{fy}.pdf` mit member_number) sind nahezu identisch, aber nicht dokumentiert als unterschiedlich. Bei Code-Suche nach „auszahlungs_anschreiben_GJ_" findet man nur die Bundle-Variante; per-Member-Filename ist genug anders, dass Operator-Skripte (regex über documents/) falsch matchen könnten.

**Fix:** In den Doc-Kommentar von `RepaymentLetterBundle::filename` (in `genossi_service/src/repayment_letter.rs:23`) explizit ergänzen, dass per-Member-Filenames ein anderes Pattern haben (`auszahlungs_anschreiben_{member_nr}_GJ_{fy}.pdf`). Optional: eine pure-fn `letter_filename_pattern(member_number, fiscal_year) -> String` exportieren, sodass Frontend-Filter und Backend-Generation dieselbe Wahrheits-Quelle haben.

### IN-04: Fallback-Pfad in `from_env`-Konstruktoren ist unsicher für Tests

**File:** `genossi_service_impl/src/template_storage.rs:61-64` und `genossi_service_impl/src/pdf_generation.rs:62-64`
**Issue:** Sowohl `TemplateStorage::from_env()` als auch `PackageCache::new()` lesen Env-Variablen mit String-Default:

```rust
let path = std::env::var("TEMPLATE_PATH").unwrap_or_else(|_| "./templates".into());
```

und

```rust
.unwrap_or_else(|_| PathBuf::from("./typst-packages"));
```

`./templates` und `./typst-packages` sind relativ zum CWD. Bei parallelen Cargo-Tests (default behavior) kann das CWD undefiniert sein, und mehrere Test-Threads konkurrieren um denselben Pfad. Der E2E-Test umgeht das mit explizitem `provision_defaults()` + `template_storage.base_path()`, aber Unit-Tests, die direkt `TemplateStorage::from_env()` ohne `TEMPLATE_PATH=...`-Env aufrufen, würden conflicten.

**Fix:** In Tests `TemplateStorage::new(temp_dir.path().to_path_buf())` direkt verwenden (existiert bereits in `test_storage()`) und `from_env()` nur in Production-Wiring (`genossi_bin`) aufrufen. Optional: `from_env()` Doc-Kommentar warnen vor Test-Use.

---

_Reviewed: 2026-06-02_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
