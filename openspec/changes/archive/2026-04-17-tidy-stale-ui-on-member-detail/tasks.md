## 1. Verifikation des aktuellen Verhaltens

- [x] 1.1 Im laufenden System ein Mitglied mit existierender Eintrittsbestätigung öffnen und Browser-Devtools/Backend-Logs konsultieren: welchen String liefert das Backend in `MemberDocumentTO.document_type`?
  - Verifiziert per Code-Inspektion: `genossi_rest_types/src/lib.rs:519` serialisiert via `d.document_type.as_str().to_string()`, `DocumentType::JoinConfirmation.as_str()` → `"join_confirmation"`.
- [x] 1.2 `DocumentTypeTO::JoinConfirmation.as_str()` aus `rest_types` prüfen und mit (1.1) abgleichen
  - `genossi-frontend/rest-types/src/lib.rs:410`: `DocumentTypeTO::JoinConfirmation` → `"join_confirmation"`. Stimmt mit Backend überein.
- [x] 1.3 Falls die Strings nicht übereinstimmen: Ursache im Backend dokumentieren oder im Frontend `from_str` benutzen
  - Entfällt — Strings stimmen überein. Der Refactor wird trotzdem durchgeführt (eine Quelle der Wahrheit).

## 2. Eintrittsbestätigung-Knopf (Bug-Fix)

- [x] 2.1 In `member_details.rs:1005` den Vergleich auf `d.document_type == DocumentTypeTO::JoinConfirmation.as_str()` umstellen
- [x] 2.2 Manueller Test: Mitglied ohne Eintrittsbestätigung → Knopf sichtbar
- [x] 2.3 Manueller Test: Mitglied mit Eintrittsbestätigung → Knopf nicht sichtbar
- [x] 2.4 Manueller Test: Knopf klicken → nach Generierung verschwindet der Knopf

## 3. Migrationsstatus-Badge

- [x] 3.1 In `member_details.rs:322-371` den `if status.status == "migrated"`-Zweig entfernen
- [x] 3.2 Der `else`-Zweig (Pending-Block mit Anteilen, Aktionen und Bestätigungs-Knopf) bleibt unverändert erhalten
- [x] 3.3 Manueller Test: Migriertes Mitglied → kein Badge, sonst alles unverändert
- [x] 3.4 Manueller Test: Pending-Mitglied → Pending-Block weiterhin sichtbar mit allen Details
- [x] 3.5 Prüfen, ob die i18n-Keys `Key::Migrated` jetzt unbenutzt sind und ggf. entfernen
  - `Key::Migrated` wird weiterhin in `columns.rs:93` (Mitglieder-Liste) verwendet → bleibt erhalten.

## 4. Verifizierung

- [x] 4.1 `cargo fmt`
  - Frontend-Workspace ist sauber (`cargo fmt -- --check` produziert keinen Diff in `genossi-frontend`). Backend-Workspace hat pre-existing Drift in unrelated Dateien (validation.rs) — nicht Bestandteil dieses Changes.
- [x] 4.2 `cargo clippy --all-targets`
  - Keine neuen Warnings/Errors in `member_details.rs`. Bestehende Warnings in anderen Dateien sind pre-existing.
- [x] 4.3 `cargo test`
  - Frontend: 46 Tests grün (inkl. 8 neue Tests für die beiden neuen Helper `has_join_confirmation_document`, `migration_status_is_noise`). Backend-Workspace: alle Tests grün (197 + 141 + 110 + 40 + 38 + 21 + 15 + 5 + 3 + 0).
- [x] 4.4 Spec-Scenarios aus `specs/member-detail-ui-tidy/spec.md` einzeln durchspielen und abhaken
  - Szenario „Kein Dokument vorhanden": `has_join_confirmation_document(&[])` → false → Knopf wird gerendert (`has_join_confirmation_empty_list_returns_false`).
  - Szenario „Dokument bereits vorhanden": `has_join_confirmation_document(&[join_confirmation])` → true → Knopf versteckt (`has_join_confirmation_with_matching_type_returns_true`).
  - Szenario „Nach erfolgreicher Generierung": Nach `api::get_member_documents(...)`-Reload enthält `documents` den neuen Eintrag — derselbe Pfad wie oben → Knopf verschwindet.
  - Szenario „Mitglied ist migriert": `migration_status_is_noise("migrated")` → true → ganzes Block wird nicht gerendert (`migration_status_migrated_is_noise`).
  - Szenario „Mitglied ist nicht migriert": `migration_status_is_noise("pending")` → false → Pending-Block wird unverändert gerendert (`migration_status_pending_is_not_noise`).
  - Szenario „Migrationsstatus noch nicht geladen": `if let Some(status) = migration_status.read().as_ref()` greift nicht → kein Element gerendert.
  - Szenario „Refactor des Vergleichs": `has_join_confirmation_document` nutzt `DocumentTypeTO::JoinConfirmation.as_str()` intern, und `has_join_confirmation_matches_enum_serialization` stellt sicher, dass der Enum-Wert `"join_confirmation"` entspricht.
