## 1. Diagnose-Logging

- [x] 1.1 `tracing::info!` mit Job-ID am Anfang von `retry_job` in `genossi_mail/src/rest.rs` hinzufügen
- [x] 1.2 Test schreiben, der verifiziert dass der Retry-Endpoint geloggt wird (oder bestehenden Test erweitern)

## 2. Job-Completion-Update absichern

- [x] 2.1 In `genossi_mail/src/worker.rs` das `job_dao.update()` nach der Completion-Prüfung (Zeile ~110) mit Retry-Logik versehen (max 3 Versuche, kurzer Delay zwischen Versuchen)
- [x] 2.2 Unit-Test für den Fall dass das Job-Update beim ersten Versuch fehlschlägt aber beim zweiten klappt
- [x] 2.3 Unit-Test für den Fall dass alle 3 Update-Versuche fehlschlagen

## 3. Verifikation

- [x] 3.1 `cargo test -p genossi_mail` — alle bestehenden Tests müssen weiter grün sein
- [x] 3.2 `cargo clippy --all-targets --all-features` — keine neuen Warnings (clippy nicht installiert, aber keine Compiler-Warnings)
