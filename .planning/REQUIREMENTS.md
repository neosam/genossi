# Requirements: Genossi — v1.5 Editor-Vervollständigung, Bild-Support & Vorschau

**Defined:** 2026-07-17
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit.

**Milestone-Goal:** Der WYSIWYG-Editor bekommt vollen Formatierungs-Umfang (Listen, Überschriften), Vorstand kann Inline-Bilder direkt im Editor hochladen und in HTML-Mails einbetten, und das gerenderte HTML lässt sich in Desktop-/Mobile-Vorschau prüfen bevor die Mail versendet wird.

## v1.5 Requirements

### Editor-Formatierung (Phase 26)

- [ ] **EDIT-06**: Vorstand kann im WYSIWYG-Editor **ungeordnete Listen** (`<ul><li>`) via Toolbar-Button einfügen; die Struktur überlebt Save/Reload und ammonia-Sanitization.
- [ ] **EDIT-07**: Vorstand kann im WYSIWYG-Editor **geordnete Listen** (`<ol><li>`) via Toolbar-Button einfügen; die Struktur überlebt Save/Reload und ammonia-Sanitization.
- [ ] **EDIT-08**: Vorstand kann im WYSIWYG-Editor **Überschriften H2/H3** via Toolbar-Button (Dropdown oder zwei Buttons) einfügen; die Struktur überlebt Save/Reload und ammonia-Sanitization.
- [x] **EDIT-09**: Toolbar-Buttons für Listen und Überschriften nutzen `document.execCommand` konsistent mit dem bestehenden Bold-Pattern (`styleWithCSS=false`); zusätzlicher Grep-Gate analog EDIT-01/02 aus v1.4.
- [ ] **EDIT-10**: v1.4 Phase 24 UAT-Checklist wird im Zuge dieser Phase mit-abgehakt (Bold + Paste-Plain + Modal-Link-Dialog + neue Formatierungen als kombinierter Vorstand-Smoke-Test).

### Bild-Support (Phase 27)

- [x] **IMG-01**: Neue `mail_asset`-Entität (SQLite BLOB-Storage: `id, created, deleted, version, filename, mime_type, size_bytes, bytes, uploaded_by`) mit DAO/Service/REST — **kein Audit-Log** (analog Application-Doc-Pattern für Nicht-Kern-Entitäten).
- [x] **IMG-02**: `POST /api/mail/assets` akzeptiert `multipart/form-data` mit PNG/JPEG/GIF, max 5 MB/Bild, gibt `mail_asset.id` zurück; nur für Vorstand (`admin`-Rolle).
- [x] **IMG-03**: Vorstand kann im WYSIWYG-Editor Bilder per **Drag&Drop** ODER Toolbar-Button einfügen; Editor fügt `<img data-genossi-asset-id="…" src="/api/mail/assets/{id}/bytes">` in den Body ein.
- [x] **IMG-04**: `GET /api/mail/assets/{id}/bytes` liefert die Bytes für Editor-Preview; nur für Vorstand (kein Public-Access, kein CID-Bypass).
- [x] **IMG-05**: `sanitize.rs` `<img>`-Regel härten — erlaubt ausschließlich `data-genossi-asset-id` als Attribut-Referenz; `src` und andere Attribute werden gestrippt bzw. server-seitig injiziert (kein externes HTTP, kein `data:`-URI).
- [x] **IMG-06**: Renderer transformiert `<img data-genossi-asset-id="X">` zu `<img src="cid:asset-X@genossi">` und hängt die Bytes als `multipart/related` inline-Part mit passender `Content-ID` an; Mail-Struktur wird `multipart/mixed → multipart/related → multipart/alternative`.
- [x] **IMG-07**: Test-Mail-Versand (bestehender Endpoint) unterstützt Bilder identisch — Vorstand sieht die Bilder in der Test-Mail.
- [x] **IMG-08**: Gesamtmailgröße wird beim Rendern gegen 25 MB Limit geprüft; Überschreitung liefert klaren Fehler (kein SMTP-Reject später).
- [x] **IMG-09**: Backward-Compat — bestehende Templates ohne Bilder (v1.4) senden weiterhin ohne `multipart/related`-Wrapper.

### Desktop/Mobile-Vorschau (Phase 28)

- [ ] **PREV-01**: Vorstand kann im WYSIWYG-Editor zwischen **Bearbeiten**, **Desktop-Vorschau** (~640px) und **Mobile-Vorschau** (~360px) umschalten.
- [ ] **PREV-02**: Vorschau rendert die tatsächlich ammonia-sanitisierte HTML-Fassung des Bodys (nicht `contenteditable`-Roh-DOM); Diskrepanzen zwischen Editor und Empfänger sind damit sichtbar.
- [ ] **PREV-03**: Vorschau löst `data-genossi-asset-id` zu `/api/mail/assets/{id}/bytes`-Referenzen auf, sodass Bilder korrekt dargestellt werden.
- [ ] **PREV-04**: Vorschau ist visuell klar von „Bearbeiten"-Modus abgegrenzt (z. B. Frame-Border simuliert Device-Rahmen); Vorstand versteht sofort, dass Klicks/Tippen im Vorschau-Modus nicht editieren.
- [ ] **PREV-05**: Vorschau nutzt sandboxed `<iframe>` mit fester Breite; kein Preview-CSS bleedet in die Editor-Umgebung und umgekehrt.

## Future Requirements

Deferred zu späteren Milestones — sinnvoll, aber nicht kritisch für v1.5.

### Bild-Support Erweiterung

- **IMG-FUT-01**: Server-seitige Bild-Verkleinerung/Kompression bei Upload (spart Mail-Größe)
- **IMG-FUT-02**: Orphan-GC-Job für `mail_asset`-Rows, die in keinem Template/Job mehr referenziert werden — Backlog 999.6

### Preview Erweiterung

- **PREV-FUT-01**: Externe Mail-Client-Simulation via Litmus/Email-on-Acid — falls Kompatibilitäts-Bugs im Betrieb auftreten

## Out of Scope

Explizit ausgeschlossen — dokumentiert, um Scope-Creep zu verhindern.

| Feature | Grund |
|---|---|
| SVG-Bilder | XSS-Risiko (inline-Script-Vektoren, SVG-`<use>`-Referenzen), nicht wert für den Use-Case |
| WebP-Bilder | Outlook-Kompatibilität patchy; PNG/JPEG/GIF reichen für Vorstands-Mails |
| Externe Bild-URLs im Editor (`http(s):`-src) | Ammonia-Regel schließt das explizit aus — Tracking-Pixel-Risiko, DSGVO |
| Data-URI-Bilder (`data:image/…;base64,…`) | Gmail strippt sie; Angriffsfläche via SVG-data-URI; CID ist der Standard |
| Externe Preview-Services (Litmus, Email-on-Acid) | Frame-Preview + Test-Mail-Versand reichen; externe Services out-of-scope |
| Client-Side-Bild-Editor (Cropping, Rotation) im Frontend | Vorstand bearbeitet Bilder vor Upload; nicht Kern-Value der Software |
| Automatische Bild-Alt-Texte via AI | Vorstand kann alt-text manuell setzen; automatisierte Beschreibungen nicht im Scope |
| Bild-Historie/Versionierung pro `mail_asset` | Bilder sind unveränderlich nach Upload; Ersetzen = neues Asset |

## Traceability

Gefüllt vom `gsd-roadmapper` beim Roadmap-Bau am 2026-07-17.

| Requirement | Phase | Phase Name | Status |
|-------------|-------|------------|--------|
| EDIT-06 | Phase 26 | Editor-Formatierung vervollständigen | Pending |
| EDIT-07 | Phase 26 | Editor-Formatierung vervollständigen | Pending |
| EDIT-08 | Phase 26 | Editor-Formatierung vervollständigen | Pending |
| EDIT-09 | Phase 26 | Editor-Formatierung vervollständigen | Complete |
| EDIT-10 | Phase 26 | Editor-Formatierung vervollständigen | Pending |
| IMG-01 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-02 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-03 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-04 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-05 | Phase 27 | Bild-Support Backend + Editor-Upload | Pending |
| IMG-06 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-07 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-08 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| IMG-09 | Phase 27 | Bild-Support Backend + Editor-Upload | Complete |
| PREV-01 | Phase 28 | Desktop/Mobile-Vorschau | Pending |
| PREV-02 | Phase 28 | Desktop/Mobile-Vorschau | Pending |
| PREV-03 | Phase 28 | Desktop/Mobile-Vorschau | Pending |
| PREV-04 | Phase 28 | Desktop/Mobile-Vorschau | Pending |
| PREV-05 | Phase 28 | Desktop/Mobile-Vorschau | Pending |

**Coverage:** 19/19 (100%) v1.5 Requirements mapped, keine Orphans, keine Duplikate.
