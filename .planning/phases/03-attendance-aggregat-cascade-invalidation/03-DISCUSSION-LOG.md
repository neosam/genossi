# Phase 3: Attendance-Aggregat + Cascade-Invalidation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-03
**Phase:** 3-attendance-aggregat-cascade-invalidation
**Areas discussed:** Attendance-Tabellen-Schema, Cascade-Invalidation-Strategie, Permission-Branch für AuthContext::Helper, Toggle-Endpoint-Design + Service-Layout

---

## Attendance-Tabellen-Schema

### Frage 1: Welches Schema-Modell für die Anwesenheits-Tabelle?

| Option | Description | Selected |
|--------|-------------|----------|
| Leichtgewichtig + Soft-Delete-Toggle | assembly_id+member_id+marked_at+marked_by_user_id, KEIN id/version, deleted für Toggle-Off; Toggle-On überschreibt deleted=NULL | ✓ |
| Leichtgewichtig + Hard-Delete-Row | Wie oben, aber Toggle-Off macht DELETE; UNIQUE-Index ohne WHERE; "wer hat ausgetragen" geht verloren | |
| Vollwertiges Aggregat (id/version/audited) | Wie assembly.rs mit id-UUID, version, audited-Macros — bricht ATTN-05 und Overkill | |

**User's choice:** Leichtgewichtig + Soft-Delete-Toggle
**Notes:** Erlaubt UPSERT-Reuse beim Re-Toggle und bewahrt "wer hat zuletzt eingetragen" für Vorstand-Inspektion.

### Frage 2: Wie wird marked_by_user_id gesetzt — und braucht es ein zusätzliches Feld für 'unmarked_by'?

| Option | Description | Selected |
|--------|-------------|----------|
| Nur marked_by_user_id, überschreiben beim Toggle | Toggle-On überschreibt; kein unmarked_by; minimal footprint | ✓ |
| marked_by + unmarked_by (zwei Spalten) | Beide Spalten persistieren — vollständige Bewegungs-Historie | |
| Kein marked_by | Nur assembly_id + member_id + marked_at + deleted; bei Bug-Hunt kein Anhaltspunkt | |

**User's choice:** Nur marked_by_user_id, überschreiben beim Toggle
**Notes:** Wer-hat-ausgetragen ist Edge-Case ohne Verband-Anforderung; Vorstand kann bei Bedarf später nachziehen.

### Frage 3: Wie implementiert das DAO den idempotenten Toggle-On?

| Option | Description | Selected |
|--------|-------------|----------|
| INSERT ... ON CONFLICT DO UPDATE (UPSERT) | Atomar, ein SQL-Statement, race-frei; SQLite ≥ 3.24 | ✓ |
| SELECT-then-INSERT/UPDATE im Service | Zwei Roundtrips, theoretisches Race-Fenster mit UNIQUE-Constraint | |
| INSERT ... OR IGNORE | Funktioniert nicht für Re-Toggle (deleted!=NULL kommt nicht zurück) | |

**User's choice:** INSERT ... ON CONFLICT DO UPDATE (UPSERT)
**Notes:** Erfüllt ATTN-03-Idempotenz und SYNC-02-Race-Sicherheit in einem.

---

## Cascade-Invalidation-Strategie

### Frage 4: Wie wird die aktive Cascade-Invalidation in close_assembly verdrahtet?

| Option | Description | Selected |
|--------|-------------|----------|
| Via helper_token.session_id-Lookup | DAO-Method liefert session_ids, close_assembly-Loop ruft delete_session; nutzt FK aus Phase 2 D-01 | ✓ |
| Neue DAO-Method delete_sessions_by_assembly_id mit JSON-LIKE | json_extract auf claims; ein SQL-Statement, aber PermissionDao bekommt helper-spezifische Method | |
| Via synthetic-User-Loop | helper_token.id → user-id "helper:<id>" → delete_sessions_for_user; doppelt indirekt | |

**User's choice:** Via helper_token.session_id-Lookup
**Notes:** Saubere Schicht-Trennung; Phase-2-FK wird für seinen Zweck verwendet.

### Frage 5: Wo lebt der Cascade-Code: neue HelperTokenDao-Method oder direkt in AssemblyServiceImpl::close_assembly?

| Option | Description | Selected |
|--------|-------------|----------|
| Neue HelperTokenDao::list_session_ids_for_assembly | DAO liefert Vec<Arc<str>>; AssemblyServiceImpl orchestriert | ✓ |
| Direkt SQL in close_assembly via PermissionDao::delete_sessions_by_assembly_id | Ein Roundtrip; PermissionDao kennt jetzt helper_token (Coupling) | |
| Neuer HelperSessionService-Wrapper-Service | Eigener Service mit invalidate_sessions_for_assembly() — Overengineering | |

**User's choice:** Neue HelperTokenDao::list_session_ids_for_assembly
**Notes:** O(N) Aufrufe sind bei realistischen GVs trivial; Permission-Schicht bleibt domain-frei.

### Frage 6: Bleibt der Phase-2-D-18-Status-Check (Verify-Pfad) als Defense-in-Depth oder wird er entfernt?

| Option | Description | Selected |
|--------|-------------|----------|
| Bleibt als Defense-in-Depth | verify_user_session prüft weiterhin status==Open; Belt-and-Suspenders zur aktiven Cascade | ✓ |
| Entfernen, nur noch aktive Cascade | Spart 1 DB-Roundtrip pro Helfer-Request; Phase-2-Test müsste angepasst werden | |
| Beides, mit Anti-Pattern-Doc | Wie Defense-in-Depth, aber explizit "do NOT remove" dokumentiert | |

**User's choice:** Bleibt als Defense-in-Depth
**Notes:** Schützt gegen Race zwischen Cascade-DELETE und gleichzeitigem Helfer-Request; Phase-2-Test bleibt unverändert.

---

## Permission-Branch für AuthContext::Helper

### Frage 7: Wie wird die positive Branch für Helper-Access elegant verdrahtet?

| Option | Description | Selected |
|--------|-------------|----------|
| Neue Method check_assembly_access(assembly_id, ctx) auf PermissionService | Trait-Method mit explizitem Domain-Vertrag | ✓ (initiale Auswahl, später verfeinert zu D-17 in CONTEXT) |
| Erweitern check_permission um Helper-Match (Special-Case 'attendance.access') | Bricht den Vertrag (privilege-string trägt assembly_id nicht) | |
| Helper-Access vollständig im AttendanceService selbst | Permission-Logik leakt in Domain-Service; Code-Duplikation | |

**User's choice:** Neue Method check_assembly_access (zunächst auf PermissionService gedacht; nach Folge-Frage 9 in AttendanceService verlagert)

### Frage 8: Welche Bedingung gilt für Vorstand (OIDC) im check_assembly_access?

| Option | Description | Selected |
|--------|-------------|----------|
| admin-Privilege reicht | Konsistent mit Phase 1+2; ATTN-06 erfüllt | ✓ |
| Neue dedizierte Permission attendance.access | Feinere Granularität, aber Migration nötig und nicht in REQUIREMENTS | |
| Nur user-Privilege | Bricht Genossi-Sicherheits-Konvention | |

**User's choice:** admin-Privilege reicht

### Frage 9: Prüft check_assembly_access auch assembly.status==Open für Helper?

| Option | Description | Selected |
|--------|-------------|----------|
| Helfer: assembly.status==Open zwingend | Belt-and-Suspenders zu D-18; Vorstand-Branch (admin) ohne Status-Check für Post-Close-Edit | ✓ |
| Status-Check nur in einzelnen Handlern | Code-Duplikation; Risiko vergessen | |
| Kein Status-Check hier — D-18 reicht | Vorstand-Edit auf nicht-existenter assembly würde durchgehen | |

**User's choice:** Helfer: assembly.status==Open zwingend

### Frage 10: Wo lebt check_assembly_access — PermissionService oder AttendanceService?

| Option | Description | Selected |
|--------|-------------|----------|
| AttendanceService | Schon AssemblyDao-Dep für stats; Assembly-Lookup wird wiederverwendet; PermissionService bleibt domain-frei | ✓ |
| PermissionService bekommt AssemblyDao-Dep | Schicht-Verletzung; PermissionService würde Über-Dependency | |
| Reine Match-Helper-Function, Caller lädt assembly | Gleichwertig zu AttendanceService — nur Modul-Frage | |

**User's choice:** AttendanceService
**Notes:** Final geltend für CONTEXT-D-17; Frage 7 wird damit konkretisiert (lebt in AttendanceServiceImpl, nicht PermissionService).

---

## Toggle-Endpoint-Design + Service-Layout

### Frage 11: Wie wird der Toggle-Endpoint REST-l geschnitten?

| Option | Description | Selected |
|--------|-------------|----------|
| PUT für On + DELETE für Off | REST-konform; Idempotenz pro Verb klar; Body leer | ✓ |
| Ein PUT mit Body { present: bool } | Weniger REST-idiomatisch | |
| PATCH mit Body { present: bool } | Bricht PATCH-Semantik (kein vorhandener Resource-State garantiert) | |

**User's choice:** PUT für On + DELETE für Off

### Frage 12: Wie wird der Vorstand-Post-Close-Edit (ASSY-06) endpoint-l aufgesetzt?

| Option | Description | Selected |
|--------|-------------|----------|
| Selbe Endpoints, AttendanceService entscheidet via Status | Helper bekommt 403 nach Schluss (D-18-Branch); admin nicht; ASSY-06 SC#9 automatisch | ✓ |
| Dedizierter Post-Close-Endpoint POST /api/assembly/{aid}/post-close-edit | Zwei Code-Pfade; UI muss differenzieren | |
| Ein PUT/DELETE — aber Audit-Macros für Post-Close-Edits | Würde Verband-Argument stärken; bricht ATTN-05 nicht; aber zusätzliche Komplexität | |

**User's choice:** Selbe Endpoints, AttendanceService entscheidet via Status

### Frage 13: Wo lebt der neue Code: eigener AttendanceService oder erweiterter AssemblyService?

| Option | Description | Selected |
|--------|-------------|----------|
| Neuer AttendanceService | Eigenes Modul; AssemblyService bleibt fokussiert auf Lifecycle | ✓ |
| AssemblyService erweitern | AssemblyService würde zu groß; Permission-Mix im selben File | |
| Mehrere kleine Services (Attendance + Stats + MemberLookup) | Overengineering | |

**User's choice:** Neuer AttendanceService

### Frage 14: Wo lebt GET /api/assembly/{id}/stats?

| Option | Description | Selected |
|--------|-------------|----------|
| AttendanceService::stats | Live-Counter ist Anwesenheits-View; check_assembly_access wiederverwendet | ✓ |
| AssemblyService::stats | AssemblyService bräuchte AttendanceDao-Dep + duplizierte Permission-Logik | |
| Eigener StatsService | Overengineering für eine Method | |

**User's choice:** AttendanceService::stats
**Notes:** Endpoint-URL bleibt `/api/assembly/{id}/stats` (semantisch ein Assembly-Aspekt), Implementation aber im AttendanceService.

---

## Claude's Discretion

Folgende Detail-Entscheidungen wurden bewusst dem Plan/Researcher überlassen (siehe CONTEXT-Decisions-Block "Claude's Discretion"):

- **UNIQUE-Index-WHERE-Clause** (D-04): plain `UNIQUE` vs. partial `UNIQUE WHERE deleted IS NULL` — SQLite-UPSERT-Anforderung entscheidet.
- **FK-ON-DELETE-Verhalten** für `attendance.assembly_id`/`attendance.member_id`: vermutlich `RESTRICT`, Plan finalisiert.
- **Search-Min-Length, Pagination**: nicht in Phase 3 nötig; Plan-Researcher kann nachziehen wenn Performance-Sorgen auftauchen.
- **Stats-Polling-Rate-Limit**: Frontend-Phase-4-Detail; aktuelle `tower_governor`-Konfig vermutlich okay.
- **Reihenfolge der Filter im Substring-LIKE** (D-25): Plan wählt aus Performance-Sicht.
- **Test-Strategie für UPSERT-Race** (D-05): Race-Test mit `tokio::join!`; E2E-Researcher detailliert.
- **`stats`-Permission-Branch für Helper**: identisch zu Toggle-Endpoints (greift `check_assembly_access`).
- **Error-Strategie bei `delete_session`-Fehler in Cascade** (D-15): Rollback vs. Continue-on-Error; Plan entscheidet.

## Deferred Ideas

Siehe CONTEXT.md `<deferred>`-Block für vollständige Liste. Kurz:

- **Phase 4 (Frontend)**: AttendanceRow/AttendanceSearch/LiveCounter-Components, Polling, Connection-Banner, Manual-Code-UI.
- **Phase 5 (Operations)**: Stats-Polling unter realer Last bei Generalprobe verifizieren.
- **v2 / Out of Scope**: Bulk-Mark-Endpoint, Pagination, eigene attendance.access-Privilege, Audit-Log für Post-Close-Edit, unmarked_by-Feld, Stats für historische GVs (gehört zu EXPO-01/02), Pro-IP-Rate-Limit für Stats.
