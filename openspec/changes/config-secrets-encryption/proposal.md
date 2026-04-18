## Meta
- **Priority:** medium
- **Category:** security

## Why

Config-Werte vom Typ `secret` (z.B. `backup_webdav_password`, `public_api_key`) werden zwar in der REST-API korrekt mit `***` maskiert (`genossi_config/src/rest.rs:39-40`), liegen aber als Klartext in der SQLite-Datenbank. Wer Zugriff auf die DB-Datei bekommt (Backup-Leak, Server-Kompromittierung), sieht alle Passwörter direkt.

## What Changes

- Config-Werte mit `value_type = "secret"` werden vor dem Speichern in der DB verschlüsselt und beim Lesen entschlüsselt.
- Verschlüsselungsschlüssel wird über eine Umgebungsvariable (z.B. `CONFIG_ENCRYPTION_KEY`) bereitgestellt.
- Bestehende Klartext-Secrets werden bei der Migration einmalig verschlüsselt.
- Ohne gesetzten Key verhält sich das System wie bisher (Klartext) — abwärtskompatibel für Entwicklung.

## Capabilities

### New Capabilities

- `config-secret-encryption`: Verschlüsselung von Secret-Config-Werten at rest in der SQLite-Datenbank

### Modified Capabilities

_(keine — die REST-API maskiert bereits korrekt)_

## Impact

**Code:**
- `genossi_config/src/dao_sqlite.rs` — Encrypt/Decrypt-Logik bei `set()` und `get_all()`/`get()`
- `genossi_config/src/service.rs` — ggf. Key-Injection
- `genossi_bin/src/main.rs` — Env-Variable `CONFIG_ENCRYPTION_KEY` lesen

**Datenbank:**
- Migration: Bestehende Secrets einmalig verschlüsseln

**Dependencies:**
- Krypto-Crate evaluieren (z.B. `aes-gcm` oder `chacha20poly1305`)
