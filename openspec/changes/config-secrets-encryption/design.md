## Context

Config-Werte werden in der SQLite-Tabelle `config_entries` als `(key, value, value_type)` gespeichert (`genossi_config/src/dao_sqlite.rs:63-76`). Der `value_type` markiert Secrets bereits (`"secret"`), und die REST-Response maskiert deren `value` zu `"***"` (`genossi_config/src/rest.rs:39-40`). Die Persistenz selbst bleibt Klartext — d. h. `backup_webdav_password`, `public_api_key` u. a. liegen in Klartext in der DB-Datei.

**Bedrohungsmodell:** Leser der Datei (verlegtes/geleaktes Backup, kompromittierte Server-Platte, Offline-Analyse) erhält direkt verwertbare Zugangsdaten. Die gültige Laufzeit hat über `CONFIG_ENCRYPTION_KEY` (Env) Zugang zum Klartext, das ist akzeptiert — kein HSM-Ersatz.

**Constraints:**
- SQLite-Schema: `value` ist `TEXT NOT NULL` — verschlüsselte Bytes müssen String-kodiert werden (Base64).
- Ein einzelner Worker-Prozess liest & schreibt; keine Multi-Node-Koordination nötig.
- Dev-Setups ohne Key dürfen nicht brechen (Onboarding, Tests).
- Keine API-Vertragsänderung (REST antwortet weiterhin `"***"`).

## Goals / Non-Goals

**Goals:**
- Secret-Values (Rows mit `value_type = "secret"`) liegen at-rest in der DB verschlüsselt.
- Authenticated Encryption (AEAD) mit per-Row-Nonce — kein deterministisches Encryption-Leak.
- Abwärtskompatibel: ohne gesetzten Key läuft das System wie heute (Klartext in DB).
- Einmalige Migration bestehender Klartext-Secrets, wenn Key beim Start gesetzt ist.
- Verschlüsselung ist auf Secret-Typen beschränkt — Non-Secret-Config bleibt Klartext (einfachere Diagnose, kein Overhead).

**Non-Goals:**
- Key-Rotation zur Laufzeit (manueller Migrationspfad via Re-Import genügt für jetzt).
- Key-Management-Service-Integration (HSM, Vault, AWS KMS) — env var reicht für Single-Node-Deployment.
- Verschlüsselung anderer sensibler Daten (Member-Felder, Mail-Bodies) — separater Scope.
- Schutz vor Memory-Dumps des laufenden Prozesses.
- Per-Tenant/per-User-Keys — eine System-weite Key-Identity.

## Decisions

### D1: Krypto-Primitive — `chacha20poly1305` (XChaCha20-Poly1305)

**Gewählt:** `chacha20poly1305` crate, Variante `XChaCha20Poly1305` mit 24-Byte-Nonce.

**Warum:**
- AEAD (Authenticated Encryption with Associated Data) — erkennt Tampering, kein Padding-Oracle-Risiko.
- 24-Byte-Nonce erlaubt sicheres Zufalls-Sampling pro Write ohne Counter-State (im Gegensatz zu AES-GCM's 12-Byte-Nonce, wo Random-Nonces bei vielen Writes statistisch kollidieren können — relevant, wenn dieselbe Config häufig überschrieben wird).
- Reine Rust-Implementierung, keine nativen Krypto-Abhängigkeiten (OpenSSL-frei → simpleres Build, konsistent mit `nix`-Toolchain).
- Seitenkanalresistent ohne spezielle CPU-Features (ChaCha läuft auf jedem Target ohne AES-NI-Abhängigkeit).

**Alternativen:**
- `aes-gcm`: ebenfalls AEAD, aber 12-Byte-Nonce → Random-Nonce-Kollisionsrisiko bei ~2^32 Writes mit selbem Key. Bei einer Config-Tabelle eher theoretisch, aber XChaCha20 eliminiert die Sorge komplett.
- `age` (high-level): zu file-orientiert, zieht unnötige Features (Recipients, Identity-Management).
- Selbstbau mit `ring`: kein Vorteil gegenüber `chacha20poly1305` bei mehr Boilerplate.

### D2: Key-Bereitstellung — Env-Variable `CONFIG_ENCRYPTION_KEY` (Base64, 32 Byte)

**Gewählt:** 32-Byte-Key als Base64-String in `CONFIG_ENCRYPTION_KEY`. Beim Start einmalig dekodiert, im `Arc<Key>` gehalten, an `ConfigDaoSqlite` per Konstruktor injiziert.

**Warum:**
- Konsistent mit bestehenden Env-Vars (`DATABASE_URL`, `SERVER_ADDRESS`).
- Base64 hält den Key copy-paste-freundlich in systemd-Unit-Files / `.env`.
- 32 Byte = 256 Bit matched XChaCha20-Keysize.
- Kein Key-on-Disk-Fußabdruck im Projekt — Operator wählt, wie der Env-Wert dorthin kommt (systemd `EnvironmentFile`, `pass`, etc.).

**Alternativen:**
- Key aus Datei (`CONFIG_ENCRYPTION_KEY_FILE`): könnte als zweite Option ergänzt werden, aber KISS — Env-Var reicht, Operator kann via `$(cat /etc/...)` selbst mappen.
- Key in DB (verschlüsselt mit Master-Key): rekursives Problem, kein Sicherheitsgewinn.
- Key aus Passphrase ableiten (Argon2): nur sinnvoll wenn User interaktiv entsperrt — hier nicht der Fall (Server startet unattended).

### D3: Ciphertext-Format — `v1:<base64(nonce || ciphertext || tag)>`

**Gewählt:** Versionspräfix + Base64-Payload, z. B. `v1:eF8NfK...`. Entscheidung auf Read-Seite: Präfix vorhanden → entschlüsseln; nicht vorhanden → als Klartext interpretieren.

**Warum:**
- Selbst-beschreibend — Migration kann ungescheute Secrets (ohne Präfix) erkennen und beim nächsten Write verschlüsseln.
- Version-Präfix erlaubt zukünftige Algorithmus-Wechsel ohne Schema-Migration.
- Kein Schema-Change: `value` bleibt `TEXT NOT NULL`.

**Alternativen:**
- Separate Spalte `value_encrypted BLOB`: saubere Trennung, aber Schema-Migration nötig und doppelte Code-Pfade beim Read.
- Flag-Spalte `is_encrypted BOOL`: zweite Wahrheitsquelle → Drift-Risiko. Das Präfix macht den Zustand direkt im Wert sichtbar.

### D4: Encryption-Boundary — DAO-Layer (`ConfigDaoSqlite`)

**Gewählt:** Ver-/Entschlüsselung findet in `ConfigDaoSqlite::set/get/all` statt. Service-Layer sieht immer Klartext-`ConfigEntry`.

**Warum:**
- DAO ist die einzige Quelle für DB-Zugriffe — zentraler Chokepoint, keine vergessenen Pfade.
- Service-Layer bleibt agnostisch → Tests dort brauchen keinen Key.
- Matcht das bestehende DAO-Pattern (Transformation DB↔Domain ist DAO-Verantwortung).

**Alternativen:**
- Service-Layer-Encryption: würde DAO unverändert lassen, aber jeder DAO-Konsument müsste selbst verschlüsseln. Verletzt DRY, erhöht Fehleranfälligkeit.
- SQLite-Encryption (SQLCipher): verschlüsselt die ganze DB-Datei, nicht nur Secrets. Overhead, zusätzliche Build-Abhängigkeit, verhindert Debugging non-secret rows mit `sqlite3`.

### D5: Migrations-Strategie — Lazy + Startup-Sweep

**Gewählt:** Beim Serverstart (wenn Key gesetzt) einmaliger Sweep über `config_entries` WHERE `value_type='secret'`: jeder Row ohne `v1:`-Präfix wird gelesen, verschlüsselt zurückgeschrieben. Keine separate SQL-Migration.

**Warum:**
- SQLx-Migrations kennen den Key nicht — Krypto gehört nicht ins SQL.
- Idempotent: bereits verschlüsselte Rows werden am Präfix erkannt und übersprungen.
- Läuft bei jedem Start (billig) — fängt auch Edge-Cases wie manuelle DB-Edits auf.
- Kein Schema-Change nötig.

**Alternativen:**
- Eager SQL-Migration: scheitert daran, dass SQL kein XChaCha20 kann. Hybrid-Ansatz (Rust-Code in Migration aufrufen) ist unüblich und fragil.
- Nur lazy (beim nächsten Write verschlüsseln): alte Rows blieben ggf. ewig Klartext.

### D6: No-Key-Mode — Klartext-Fallback explizit

**Gewählt:** Fehlt `CONFIG_ENCRYPTION_KEY`, arbeitet der DAO ohne Verschlüsselung. Secrets werden mit Warn-Log (`tracing::warn!`) gespeichert; Präfix-behaftete Rows werden trotzdem abgelehnt (Startup-Error), weil man sonst verschlüsselten Content nicht lesen kann.

**Warum:**
- Entwickler-Experience: Onboarding, Tests, CI ohne Key-Setup.
- Fail-loud bei Fehlkonfiguration: verschlüsselte DB + fehlender Key → klarer Fehler beim Start, nicht stiller Garbage-Return.

**Alternativen:**
- Hart erzwingen (kein Key → kein Start): bricht bestehende Setups, vermeidet aber "vergessen, Key zu setzen"-Risiko. Aufgeschoben — Warn-Log reicht in Phase 1, Erzwingung kann später per separatem Feature-Flag kommen.

## Risks / Trade-offs

- **Key-Loss = Daten-Loss der Secrets** → Mitigation: Doku im README + Ops-Checkliste. Operator verantwortet Backup des Keys außerhalb der DB (password manager, `pass`-Store). Recovery-Pfad: Secrets manuell via REST neu setzen.
- **Key-Exposure in Env-Dumps / `/proc/<pid>/environ`** → Mitigation: systemd `EnvironmentFile=` mit `0600`-Permissions; Doku warnt vor unbedachten Prozess-Dumps. Alternative Key-Quellen (Datei) in späterer Iteration möglich.
- **Performance: AEAD-Kosten pro Read** → Messbar, aber vernachlässigbar (Config-Table hat < 50 Rows, selten gelesen, kein Hotpath).
- **Deterministic-Key-Rotation nicht gelöst** → Key wechseln erfordert heute manuelles Dump → Decrypt → Set neuer Key → Re-Encrypt. Für jetzt akzeptiert; Rotation-Change kann später kommen.
- **Test-Setup komplexer** → Mitigation: DAO-Konstruktor akzeptiert `Option<Key>`. Bestehende Tests ohne Key laufen unverändert; neue Tests können Key injizieren.
- **Präfix-Kollision** → Key-Wert `v1:...` in Klartext würde fälschlich als Ciphertext interpretiert. Mitigation: Klartext-Secrets dürfen nicht mit `v1:` beginnen (validieren bei `set()` im No-Key-Mode). Praktisch irrelevant, aber sauber dokumentieren.

## Migration Plan

1. **Rollout:**
   - Deployment mit neuem Binary + gesetztem `CONFIG_ENCRYPTION_KEY`.
   - Startup-Sweep verschlüsselt bestehende Klartext-Secrets beim ersten Boot.
   - REST-API unverändert, Clients merken nichts.

2. **Rollback:**
   - Key verfügbar halten.
   - Downgrade-Binary (ohne Encryption-Support) würde verschlüsselte Rows als unleserlichen String behandeln.
   - Recovery: altes Binary + Migration-Skript (Rust-CLI oder ein-shot `cargo run --bin decrypt-configs`), das Rows mit `v1:`-Präfix entschlüsselt und zurückschreibt.

3. **Key-Generierung** (einmalig, pro Deployment):
   ```
   openssl rand -base64 32
   ```
   Ergebnis in systemd-EnvironmentFile, `0600 root:root`.

## Open Questions

- **Sollen non-secret Values (z. B. `value_type="string"`) ebenfalls optional verschlüsselt werden können?** — Aktuell nein, aber der Präfix-Mechanismus lässt es jederzeit nachrüsten.
- **Wie umgehen, wenn ein Secret per SQL-Migration (seed data) hart eingefügt wird?** — Aktuell: Startup-Sweep fängt es. Falls Seed-Daten aber Klartext-Defaults enthalten dürfen (z. B. "change me"), ist das akzeptabel.
- **Brauchen wir einen `/api/config/rotate-key`-Endpoint?** — Nicht in diesem Scope. Rotation wird zum separaten Change, wenn operationaler Bedarf entsteht.
