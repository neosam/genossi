# Deferred Items — Phase 28

Out-of-scope Funde während der Ausführung. **Nicht** in Phase 28 gefixt (Scope Boundary:
nur Probleme fixen, die direkt durch die Änderungen des aktuellen Tasks entstanden sind).

## 1. Vorbestehender Testfehlschlag: `preview_body_html_round_trips_to_response`

- **Gefunden in:** Plan 28-01, Task 2
- **Ort:** `genossi_bin/tests/e2e_tests.rs:14961:5`
- **Meldung:** `assertion \`left == right\` failed: plain body must render member first_name` — `left: "Hallo **Max**"`, `right: "Hallo Max"`
- **Ursache:** Quick `260718-html-to-plain-derivation` leitet `PreviewResponse.body` per
  `crate::render::plain_from_html` aus dem gerenderten HTML ab, sobald `body_html` gesetzt ist.
  `plain_from_html("<p>Hallo <b>Max</b></p>")` liefert `"Hallo **Max**"`. Die Assertion des
  Tests stammt aus Phase 24 Plan 04 und wurde beim Quick nicht nachgezogen.
- **Nicht-Regression bewiesen:** `rest.rs` wurde temporär auf `HEAD~1` (Stand VOR dem
  Phase-28-Commit `f51ab45`) zurückgesetzt; der Test schlägt dort mit byte-identischer
  Meldung an derselben Zeile `14961:5` fehl.
- **Warum nicht gefixt:** Plan 28-01 Task 2 verbietet ausdrücklich jede Änderung an
  bestehenden Tests (`git diff --numstat` muss 0 gelöschte Zeilen zeigen).
- **Empfohlene Auflösung:** eigener Quick-Task — Assertion auf `"Hallo **Max**"` korrigieren
  ODER `plain_from_html` bewusst als Contract festschreiben und den Test entsprechend umbauen.

## 2. Vorbestehender Testfehlschlag: `test_mail_preview_repayment_no_entries_does_not_default_to_one`

- **Gefunden in:** Plan 28-01, Baseline-Lauf vor jeder Änderung
- **Ort:** `genossi_bin/tests/e2e_tests.rs:14628:44`
- **Meldung:** `errors must be array`
- **Status:** bereits in `STATE.md` als vorbestehender Fehlschlag aus Phase 22 dokumentiert.
  Baseline vor und nach Phase 28 identisch (Meldung + Zeile unverändert).

## 3. `cargo fmt`-Drift in `genossi_bin/tests/membership_adjust_e2e.rs`

- **Gefunden in:** Plan 28-01, Task 2 (`cargo fmt -p genossi_bin -- --check`)
- **Umfang:** 16 Fundstellen, ausschließlich in `membership_adjust_e2e.rs`
  (Zeilen 214, 887, 920, 1342, 1349, 1357, 1418, 1426, 1438, 1503, 1613, 1646, 1810, 1885, 1946, 1971)
- **Warum nicht gefixt:** unberührte Datei, keine Phase-28-Verbindung. `genossi_bin/tests/e2e_tests.rs`
  selbst ist fmt-sauber (0 Fundstellen).

## 4. `cargo fmt`-Drift in `genossi_mail/src/sanitize.rs`

- **Gefunden in:** Plan 28-01, Task 1
- **Umfang:** ein `rm_tag_attributes`-Aufruf (Zeile 49) wird von `cargo fmt` umgebrochen.
- **Warum nicht gefixt:** Plan 28-01 fordert `sanitize.rs` explizit als unverändert
  (Acceptance Criteria Task 1). Die von `cargo fmt -p genossi_mail` erzeugte Änderung wurde
  per `git checkout HEAD -- genossi_mail/src/sanitize.rs` zurückgenommen.

## 5. `cargo fmt`-Drift in `genossi-frontend/src/api.rs`

- **Gefunden in:** Plan 28-02, Task 1 (`cd genossi-frontend && cargo fmt -- --check`)
- **Umfang:** eine Fundstelle, `src/api.rs:405` — die Signatur von `upload_mail_asset`
  überschreitet die Zeilenlänge und würde von `cargo fmt` dreizeilig umgebrochen.
- **Vorbestehend:** ja. Die Drift existiert vor jedem Commit dieses Plans; `api.rs` wurde
  von Plan 28-02 nicht angefasst.
- **Warum nicht gefixt:** unberührte Datei ohne Phase-28-Bezug. Der repo-spezifische
  Git-Protokoll-Hinweis verbietet zudem ausdrücklich, mit crate-weitem `cargo fmt`
  unbeteiligte Dateien in den Diff zu ziehen. Die neu angelegte
  `mail_preview_frame.rs` wurde stattdessen gezielt mit
  `rustfmt --edition 2021 <datei>` formatiert und ist fmt-sauber.
- **Empfohlene Auflösung:** eigener Quick-Task, der `cargo fmt` einmal crate-weit über
  `genossi-frontend` laufen lässt.

## 6. Nicht genutzter Re-Export `MailPreviewFrame` in `mail_compose/mod.rs`

- **Gefunden in:** Plan 28-02, Task 2
- **Meldung:** `warning: unused import: mail_preview_frame::MailPreviewFrame`
- **Warum nicht gefixt:** Der Re-Export ist ein Pflicht-Artefakt dieses Plans (Plan 28-03
  importiert die Component darüber). `component/mod.rs` trägt bereits acht identische
  Warnungen für denselben Vorgriffs-Fall — das ist die etablierte Repo-Konvention.
  Die Warnung verschwindet mit der Verkabelung in Plan 28-03.
