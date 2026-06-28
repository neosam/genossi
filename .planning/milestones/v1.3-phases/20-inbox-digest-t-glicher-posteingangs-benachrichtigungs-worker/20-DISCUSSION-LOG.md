# Phase 20: Inbox-Digest — täglicher Posteingangs-Benachrichtigungs-Worker - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-26
**Phase:** 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
**Areas discussed:** Scheduling & Tages-Garantie, Empfänger-Format & Versand-Art, Digest-Mail Inhalt & Format, Config-Seite UI & Validierung

---

## Scheduling & Tages-Garantie

### Verpasstes Zeitfenster
| Option | Description | Selected |
|--------|-------------|----------|
| Nachholen am selben Tag | Beim nächsten Worker-Lauf nachholen, wenn Uhrzeit überschritten und noch nicht gesendet | ✓ |
| Überspringen | Nur exakt im Zeitfenster senden; verpasst = bis morgen warten | |

### Zeitzone
| Option | Description | Selected |
|--------|-------------|----------|
| Server-Lokalzeit | Uhrzeit in lokaler Server-TZ | ✓ |
| Fest Europe/Berlin | Immer deutsche Zeit, braucht tz-Handling | |
| UTC | Uhrzeit in UTC | |

### Dedup-State
| Option | Description | Selected |
|--------|-------------|----------|
| last-sent-date in Config-KV | Versanddatum als Config-Entry | |
| Neue DB-Spalte/Tabelle | Eigene Persistenz, Migration + DAO | ✓ |

### Poll-Strategie
| Option | Description | Selected |
|--------|-------------|----------|
| Periodisch pollen (~60s) | Worker wacht regelmäßig auf, prüft Zeit + last-sent-date | ✓ |
| Sleep bis nächste Uhrzeit | Worker berechnet Dauer und schläft exakt | |

**User's choice:** Nachholen am selben Tag · Server-Lokalzeit · Neue DB-Tabelle · Periodisch pollen
**Notes:** Bewusste Wahl der eigenen Tabelle statt KV-Store für den Dedup-State (mehr Boilerplate, aber sauber getrennt). Worker-Loop nach Vorbild `timestamp_worker.rs`.

---

## Empfänger-Format & Versand-Art

### Empfänger-Format
| Option | Description | Selected |
|--------|-------------|----------|
| Ein Komma-Feld | Alle Adressen komma-getrennt in einem Feld | ✓ |
| Dynamische Adress-Liste | Add/Remove-Zeilen pro Adresse | |

### Versandart (Datenschutz)
| Option | Description | Selected |
|--------|-------------|----------|
| Einzelmail pro Empfänger | Pro Empfänger eine Mail, To: nur dieser | ✓ |
| Eine Mail, alle im Bcc | Sammelmail, Empfänger im Bcc | |
| Eine Mail, alle im To | Empfänger sehen sich gegenseitig | |

### Fehlerverhalten
| Option | Description | Selected |
|--------|-------------|----------|
| Loggen & weitermachen | Fehler tracen, andere bedienen, last-sent-date trotzdem setzen | ✓ |
| Tag als nicht-gesendet markieren | last-sent-date bei Fehler nicht setzen → Retry-Risiko | |

**User's choice:** Ein Komma-Feld · Einzelmail pro Empfänger · Loggen & weitermachen
**Notes:** Einzelmails = Datenschutz + Isolation fehlerhafter Empfänger. Kein Retry, um Mehrfachversand zu vermeiden.

---

## Digest-Mail Inhalt & Format

### Format
| Option | Description | Selected |
|--------|-------------|----------|
| Plain-Text | Reiner Text, Link ausgeschrieben | ✓ |
| HTML | Formatierte HTML-Mail mit Button | |

### Rendering
| Option | Description | Selected |
|--------|-------------|----------|
| Hardcodiert im Worker | Body via format! im Rust-Code | ✓ |
| minijinja-Template | Wie bestehende Mail-Templates | |

### Sortierung & Betreff
| Option | Description | Selected |
|--------|-------------|----------|
| Neueste zuerst, Anzahl im Betreff | Absteigend nach Eingangszeit; Betreff mit Anzahl | ✓ |
| Älteste zuerst (FIFO) | Aufsteigend, Workqueue-Gedanke | |
| Du entscheidest | Claude wählt Defaults | |

### Deep-Link-Basis
| Option | Description | Selected |
|--------|-------------|----------|
| APP_URL env (wie helper_token) | {APP_URL}/inbox, Fallback localhost | ✓ |
| Eigener Config-Key | Separater Config-Eintrag für Basis-URL | |

**User's choice:** Plain-Text · Hardcodiert · Neueste zuerst + Anzahl im Betreff · APP_URL env
**Notes:** Bewusst minimal gehalten — interne Benachrichtigung, kein Template-Overhead.

---

## Config-Seite UI & Validierung

### UI-Einbindung
| Option | Description | Selected |
|--------|-------------|----------|
| Eigener Abschnitt analog SMTP/IMAP | Neuer Block „Posteingangs-Benachrichtigung" | ✓ |
| In den IMAP-Block integrieren | Felder zum bestehenden Posteingang-Abschnitt | |

### Validierung
| Option | Description | Selected |
|--------|-------------|----------|
| Adressen & Uhrzeit validieren | E-Mail-Format je Adresse + HH:MM | ✓ |
| Nur Uhrzeit-Format | Nur HH:MM prüfen | |
| Keine Validierung | Roh speichern | |

### Deaktivierung (DIGEST-07)
| Option | Description | Selected |
|--------|-------------|----------|
| Leeres Empfänger-Feld = aus | Keine Adressen → kein Versand | ✓ |
| Zusätzlicher Enabled-Toggle | Expliziter An/Aus-Schalter | |

**User's choice:** Eigener Abschnitt · Adressen & Uhrzeit validieren · Leeres Feld = aus
**Notes:** Leeres Feld deckt DIGEST-07 ohne Zusatzschalter ab.

---

## Claude's Discretion

- Konkrete Config-Key-Namen, Tabellen-/Spaltennamen der State-Tabelle, exakter Poll-Intervall-Wert,
  genaue Betreff-/Body-Formulierung, exakte E-Mail-Format-Prüfung.

## Deferred Ideas

- Reply-Komfort / Antwort-Modal → Phase 21 (REPLY-01..04)
- Feineres Versand-Intervall als täglich → DIGEST-F2, bewusst verworfen
- Digest nur über neu eingegangene Mails → DIGEST-F1, bewusst verworfen
- HTML-Mail / minijinja-Template für den Digest → später, falls gewünscht
- Expliziter Enabled-Toggle / eigener Config-Key für Basis-URL → bewusst verworfen

### Reviewed Todos (nicht eingefoldet)
- `backend-pre-flight-check-attach-repayment-letter.md` — False-Positive, nicht relevant
- `frontend-bulk-no-repayment-letter-action.md` — False-Positive, nicht relevant
