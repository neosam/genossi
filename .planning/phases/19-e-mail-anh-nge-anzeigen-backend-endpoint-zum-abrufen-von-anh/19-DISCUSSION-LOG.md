# Phase 19: E-Mail-Anhänge anzeigen - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-07
**Phase:** 19-e-mail-anhaenge-anzeigen
**Areas discussed:** Persistenz vs. On-Demand-IMAP, Migration für Bestands-Mails, Endpoint-Design + Sicherheits-Limits, Frontend-UX im Detail-Pane

---

## Area-Auswahl

| Option | Description | Selected |
|--------|-------------|----------|
| Persistenz vs. On-Demand-IMAP | Attachments persistent oder live aus IMAP holen | ✓ |
| Migration für Bestands-Mails | Umgang mit Bestands-Mails ohne Attachment-Bytes | ✓ |
| Endpoint-Design + Sicherheits-Limits | REST-API, Permission, Audit, Size-Limit, MIME-Whitelist | ✓ |
| Frontend-UX im Detail-Pane | Layout, Preview, Component-Struktur | ✓ |

**User's choice:** Alle vier Areas
**Notes:** Nutzer wählte alle Bereiche zur ausführlichen Diskussion.

---

## Area 1: Persistenz vs. On-Demand-IMAP

### Storage-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Persistent beim Polling speichern | Worker schreibt Bytes in DocumentStorage; DB-Row pro Attachment | ✓ |
| On-Demand: live aus IMAP | Bei Download IMAP-Refetch + parse + stream | |
| Hybrid: Cache nach erstem Zugriff | Lazy Storage beim ersten Download | |

**User's choice:** Persistent beim Polling speichern (Recommended)
**Notes:** Begründung im Preview gezeigt — Worker parst Attachments, ruft `storage.save(relative_path=inbound/<mail_id>/<idx>, bytes)`, schreibt `InboundMailAttachment`-Row. Detail-Endpoint instant aus Filesystem.

### Size-Limit pro Attachment

| Option | Description | Selected |
|--------|-------------|----------|
| 10 MB Hard-Limit pro Attachment | > 10 MB nur Metadaten + `oversized=true` | ✓ |
| Kein Limit — alles speichern | Mailbox-Quota limitiert ohnehin | |
| Konfigurierbar via ENV | Default 10 MB, überschreibbar | |

**User's choice:** 10 MB Hard-Limit (Recommended)
**Notes:** Schutz gegen Disk-DoS; oversized-Mails behalten Metadaten-Row, Frontend zeigt Hinweis statt Download.

### Anzeige-Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Alle inbound Mails | Auch nicht-zugeordnete Mails zeigen Attachments | ✓ |
| Nur Mails mit `assigned_member_id` | Nur zugeordnete Mails | |

**User's choice:** Alle inbound Mails (Recommended)
**Notes:** Spam-Filter greift beim Empfang, nicht beim Anzeigen.

---

## Area 2: Migration für Bestands-Mails

### Backfill-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Nur ab heute — alte Mails ohne | Migration legt nur leere Tabelle an | |
| Backfill-Worker: Bestand via IMAP nachholen | Einmaliger Job iteriert alle InboundMails mit has_attachments=true | ✓ |
| Backfill-CLI-Befehl, manuell auslösbar | Admin entscheidet, ob/wann | |

**User's choice:** Backfill-Worker: Bestand via IMAP nachholen
**Notes:** Vollständige Coverage gewünscht; UID-Validity-Drift wird durch Silent-Skip akzeptiert.

### Backfill-Trigger

| Option | Description | Selected |
|--------|-------------|----------|
| Automatisch beim Server-Start, einmalig | Worker im Hintergrund, Best-Effort | ✓ |
| Beim Server-Start mit Resume-Marker | Fortschritt persistiert, fortsetzbar | |
| Admin-Endpoint manuell triggern | POST /api/inbox/backfill-attachments | |

**User's choice:** Automatisch beim Server-Start (Recommended)
**Notes:** Vorstand muss nichts tun; Audit-Log-Eintrag pro erfolgreichem Refetch nicht nötig (analog D-10).

### Backfill-Fehlerbehandlung

| Option | Description | Selected |
|--------|-------------|----------|
| Silent skip + tracing::warn | Bei Fehler weiter, Frontend zeigt 'nicht verfügbar' | ✓ |
| Backfill bricht ab + Admin-Mail | Erster Fehler stoppt | |
| `attachment_unrecoverable=true`-Flag | Neues DB-Feld explizit | |

**User's choice:** Silent skip + tracing::warn (Recommended)
**Notes:** Pragmatisch; passt zur produktiv-erprobten Mail-Pipeline-Philosophie.

---

## Area 3: Endpoint-Design + Sicherheits-Limits

### REST-API-Schema

| Option | Description | Selected |
|--------|-------------|----------|
| Embedded in DetailTO + separater Download | `attachments`-Feld in Detail, Download via /attachments/{id} | ✓ |
| Getrennte Endpoints | Liste + Download separat | |
| Embedded mit `relative_path` exposed | Storage-Path direkt im TO | |

**User's choice:** Embedded in DetailTO + separater Download (Recommended)
**Notes:** Spart Round-Trip beim Detail-Öffnen; relative_path bleibt Backend-intern.

### Permission

| Option | Description | Selected |
|--------|-------------|----------|
| Wie bestehende Inbox-Endpoints | Vorstand-only via vorhandenem Auth-Pfad | ✓ |
| Strenger: separate Sub-Permission `inbox_attachment_read` | Granularere Rollen | |

**User's choice:** Wie bestehende Inbox-Endpoints (Recommended)
**Notes:** Konsistenz mit GET /api/inbox/{id}.

### Audit-Log

| Option | Description | Selected |
|--------|-------------|----------|
| Kein Audit-Log | Konsistent zu InboundMail-Pattern | ✓ |
| Audit auf Download | InboundMailAttachment implementiert Auditable | |

**User's choice:** Kein Audit-Log (Recommended)
**Notes:** InboundMail ist nicht auditiert; Attachments folgen demselben Pattern. Kein Auditable-Trait-Impl nötig.

### MIME-Whitelist

| Option | Description | Selected |
|--------|-------------|----------|
| Keine Whitelist — alles speichern | 10-MB-Limit als DoS-Schutz reicht | ✓ |
| Whitelist (PDF, Office, Images, Text) | Exotische MIMEs verworfen | |

**User's choice:** Keine Whitelist (Recommended)
**Notes:** Whitelist-Pflege würde mehr stören als helfen.

---

## Area 4: Frontend-UX im Detail-Pane

### Layout

| Option | Description | Selected |
|--------|-------------|----------|
| Section unter Body-Text | Header + Liste, klassisches Mail-Pattern | ✓ |
| Header-Bar oben (Gmail-Style) | Chips über Body | |
| Beides: Top-Hinweis + Section unten | Redundant aber sichtbar | |

**User's choice:** Section unter Body-Text (Recommended) — Preview-ASCII bestätigt
**Notes:** Section direkt nach Body, vor Assignment-Section. Ersetzt bestehenden „nicht anzeigbar im MVP"-Hinweis.

### Preview-Modus

| Option | Description | Selected |
|--------|-------------|----------|
| Nur Download (MVP) | Konsistent zu MemberDocument-Pattern | |
| Inline-Preview für Bilder + PDF | <img>/<embed> | ✓ |
| Klick → neuer Tab | Browser entscheidet | |

**User's choice:** Inline-Preview für Bilder + PDF
**Notes:** Erweitert Phase-Scope leicht, aber Nutzen für Vorstand bei Bild-Belegen rechtfertigt es.

### Component-Extraktion

| Option | Description | Selected |
|--------|-------------|----------|
| `InboxAttachmentList`-Component | Neue Component-Familie unter component/inbox/ | ✓ |
| Inline in inbox_page.rs | Direkt im Detail-Pane als RSX | |

**User's choice:** `InboxAttachmentList`-Component (Recommended)
**Notes:** Component-First-Prinzip aus genossi-frontend/CLAUDE.md ist verbindlich.

### Inline-Endpoint-Schema

| Option | Description | Selected |
|--------|-------------|----------|
| Query-Param `?disposition=inline` | Ein Endpoint, beide Modi | ✓ |
| Zwei getrennte Endpoints | /attachments/{id} + /attachments/{id}/inline | |

**User's choice:** Query-Param `?disposition=inline` (Recommended)
**Notes:** Pragmatisch; URL-Schema bleibt schlank.

### Inline-MIMEs

| Option | Description | Selected |
|--------|-------------|----------|
| Nur `image/*` und `application/pdf` | Bilder + PDF inline, Rest Download | ✓ |
| Auch `text/plain` + `text/html` | Plus Textformate (HTML braucht Sandbox) | |
| Alle MIMEs — Browser entscheidet | <embed> für alles | |

**User's choice:** Nur `image/*` und `application/pdf` (Recommended)
**Notes:** HTML-Inline wäre eigenes Sicherheits-Capitel — separate Phase.

---

## Claude's Discretion

- DB-Schema-Details, Index-Strategien und Migration-Filename: Planner entscheidet — Pattern in `dao_sqlite.rs:1130-1175` existiert.
- Konkrete UI-Größen, Tailwind-Klassen, Mobile-Layout: Planner/Executor.
- Test-Strategie (Unit/E2E): Planner — bestehende Patterns in `e2e_tests.rs` als Referenz.
- Konkrete Filename-Sanitization beim Content-Disposition-Inline-Modus — `http_util::content_disposition_attachment` existiert als Vorlage.

## Deferred Ideas

- Audit-Log für Attachment-Downloads → eigene Phase bei Compliance-Bedarf
- MIME-Type-Whitelist → reaktiv bei Spam-Welle
- Konfigurierbares Size-Limit via ENV → reaktiv bei produktivem Bedarf
- Inline-Preview für text/plain + text/html → eigene Security-Phase (iframe-Sandbox)
- Volltext-Suche in Attachments → neues Capability
- Virenscan beim IMAP-Polling → ClamAV-Integration, eigene Phase
- Bulk-Download-ZIP („Alle Anhänge dieser Mail") → v1.3-Backlog
- Reply-with-Forward (Anhang an Reply weiterleiten) → out of scope
