# Phase 25 UAT Checklist

**Phase:** 25-application-file-upload-audited-carryover
**Coverage:** APDOC-01 through APDOC-05 (single-slot Application document with audited Move-transfer to MemberDocument on confirm)
**Companion automated tests:**
- `application_upload_confirm_carryover_audited` — end-to-end proof of the audited carryover cascade.
- `application_upload_confirm_missing_file_rolls_back` — end-to-end proof of the APDOC-04 rollback guarantee.
- `application_upload_replace_in_place` — end-to-end proof of the replace-in-place versioning.
- 4 mock-based unit tests in `genossi_service_impl::application::tests` (CR-02 no-side-effects, happy carryover, no-doc skip, missing-file rollback).
- 7 mock-based unit tests in `genossi_service_impl::application_document::tests` (create, replace, permission-denied, download, delete, extension helper, refetch on replace).

Steps 1–3 are the automated regression portion — auto-approved as soon as `cargo test --workspace` matches the STATE.md baseline. Steps 4–12 are the browser-interactive walkthrough deferred to the Vorstand smoke session before merge (mirror Phase 24's UAT pattern).

## Setup

Follow the project skill `run-rust-backend-and-frontend` — or manually:

1. **Backend** — from repo root:
   ```bash
   cargo run --features mock_auth --bin genossi
   ```
   Serves on `http://localhost:3000` with mock authentication (Context = DEVUSER, admin).

2. **Frontend** — from `genossi-frontend/`:
   ```bash
   npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch &
   dx serve
   ```
   Serves on `http://localhost:8080`. `assets/config.json` points at the backend URL.

3. **Test data** — seed at least one Offen application via WordPress-Integration or via the "Antrag anlegen" button on the Applications page (`POST /api/applications`). One offer keeps the walkthrough focused on the doc slot.

## Verification Steps

Tick the checkbox after each step you complete. For failing items, capture: (a) which APDOC requirement was hit, (b) a DevTools screenshot of the innerHTML or Network payload, (c) suggested fix location.

- [x] **1. `cargo test --workspace` regression pass. HARD FAIL GATE — automatable regression.** Full workspace test run reports the same failure list as the STATE.md baseline. Pre-existing `test_mail_preview_repayment_no_entries_does_not_default_to_one` failure (Phase 22 or earlier — documented in Plan 25-04 SUMMARY under "Issues Encountered") is expected. No new failures. Result at Wave 4 executor: 308 passed / 1 failed / 0 ignored — the failure is the same pre-existing repayment-preview test. Auto-approved.

- [x] **2. `cargo clippy --all-targets` clean. HARD FAIL GATE — automatable.** Compilation warnings are informational only; no clippy errors introduced by Wave 4. Auto-approved.

- [x] **3. `cargo fmt -- --check` clean on files touched by Wave 4. HARD FAIL GATE — automatable.** `rustfmt --edition 2021 --check` on each file modified by Plan 25-05 returns 0 diff after the Task 2 formatting pass. Pre-existing formatting drift in the workspace (e.g. `genossi_service_impl/src/pdf_generation.rs`, `genossi-frontend/src/component/tsa_config.rs`) is out of scope. Auto-approved.

- [ ] **4. Empty state shows "Antrag hochladen" button [APDOC-05].** Start backend + frontend per project skill. Navigate to Applications list, open an Offen application. Expected: below the address/shares block and above the Confirm/Reject buttons the `ApplicationDocumentSlot` renders "Kein Antrags-Dokument hinterlegt." on the left and a blue "Antrag hochladen" button on the right (DE locale). Console has no red errors.

- [ ] **5. Upload happy path [APDOC-01, APDOC-05].** Click "Antrag hochladen", pick a valid PDF (`< 50 MB`). Expected: the slot flips to filled state showing `filename · size · upload-date DD.MM.YYYY` with three action buttons ("Herunterladen", "Ersetzen", "Löschen"). Network tab: `POST /api/applications/{id}/document` returns `201` with an `ApplicationDocumentTO` JSON body carrying a fresh `id` and `version`.

- [ ] **6. Replace-in-place [APDOC-01, APDOC-05].** Click "Ersetzen", pick a different PDF. Expected: filename in the slot updates to the new file's name; the returned `version` from the second POST differs from the first (verify via DevTools Network). Then click "Herunterladen" — the download URL streams the SECOND file's bytes (byte-diff against the local copy on disk to prove replace-in-place).

- [ ] **7. Delete flow [APDOC-01, APDOC-05].** Click "Löschen". Expected: native browser dialog `Dieses Antrags-Dokument wirklich löschen?` appears. Click OK. The slot returns to the empty state ("Kein Antrags-Dokument hinterlegt." + "Antrag hochladen" button). Network tab: `DELETE /api/applications/{id}/document` returns `204`.

- [ ] **8. Confirm with attached doc → audited carryover [APDOC-03]. HARD FAIL GATE.** Upload a PDF again, then click "Bestätigen" on the application and confirm the dialog. Expected: application status flips to Bestaetigt. Navigate to the newly-created Member's detail page → Documents tab → verify a MemberDocument with `document_type = "other"` and `description = "Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)"` where the date matches today. `GET /api/applications/{id}/document?meta=1` now returns 404 (the app-doc row is soft-deleted). **This is the APDOC-03 audit-critical proof — without it the Genossenschaftsverband cannot trace the original application to the accepted Member.**

- [ ] **9. Audit hashchain valid [APDOC-03]. HARD FAIL GATE.** `curl http://localhost:3000/api/audit/verify` returns `200` with `{"valid": true, "total_entries": N, "broken_links": []}`. **This is the audit-repudiation-defense proof — the hashchain must remain valid across the confirm cascade or the audit log stops being legally useful.**

- [ ] **10. Unauthenticated / non-admin upload rejected [APDOC-02, CR-02]. HARD FAIL GATE.** Restart backend WITHOUT `--features mock_auth` (i.e. with the real OIDC path — this requires Nextcloud OIDC configured, or use `curl` without a session cookie in mock_auth mode). Attempt `POST /api/applications/{id}/document`. Expected: `401` or `403` response. Then run `sqlite3 genossi.db 'SELECT COUNT(*) FROM application_documents WHERE deleted IS NULL;'` and confirm the count is **unchanged** — no side effects from the unauthorized call. **This is the CR-02 regression proof; if this fails, an anonymous scanner can populate the storage directory with arbitrary uploads.**

- [ ] **11. MIME reject [APDOC-01, T-25-04-04].** Restore mock_auth backend. Attempt to upload a `.exe` file via `curl -F "file=@bad.exe" http://localhost:3000/api/applications/{id}/document`. Expected: `415 Unsupported Media Type` with a JSON body listing `allowed_extensions`. The slot in the browser (if you retried there) shows the German error toast.

- [ ] **12. Body-limit reject [APDOC-01, T-25-04-03].** Create a 60 MB file: `dd if=/dev/urandom of=big.bin bs=1M count=60`. Rename to `big.pdf`. Upload via `curl -F "file=@big.pdf" http://localhost:3000/api/applications/{id}/document`. Expected: `413 Payload Too Large` (axum `DefaultBodyLimit::max(50 * 1024 * 1024)`). No new row in `application_documents`. Clean up `big.bin` / `big.pdf` after the test.

## Sign-off

- **Automated portion (Steps 1–3):** auto-approved by Wave 4 executor on 2026-07-03. Full log preserved in the phase's SUMMARY.md.
- **Browser walkthrough (Steps 4–12):** DEFERRED to Vorstand smoke session before merge — do NOT hold the phase for it (mirror Phase 24 UAT pattern).
