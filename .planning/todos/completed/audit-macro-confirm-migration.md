---
title: "Audit-Lücke: confirm_migration schreibt Member direkt via member_dao.update statt Audit-Macro"
date: 2026-06-14
priority: high
source: Code-Audit 2026-06-14 (Design-Pattern-Compliance)
blocked_by: keiner
---

# Audit-Macro in confirm_migration() verwenden

## Was

`genossi_service_impl/src/member_action.rs:528` (`confirm_migration()`) schreibt eine auditpflichtige `MemberEntity` direkt:

```rust
let mut updated_member = member.clone();
updated_member.action_count = new_action_count;
self.member_dao
    .update(&updated_member, "confirm-migration", tx.clone())
    .await?;
```

`action_count` ist ein Daten-/Auditfeld. Die Änderung erscheint **nicht** in der Audit-Hashchain, und es wird keine Actor-`user_id` ermittelt.

## Warum

Verstoß gegen die Audit-Pflicht (CLAUDE.md): Member/MemberAction/MemberDocument/Application MÜSSEN `audited_*!`-Macros statt direkter DAO-Calls nutzen. Der `MemberActionServiceImpl` hat `audit_log_dao` bereits als Dependency und nutzt die Macros andernorts korrekt — hier wurde der Call übersehen. Lücke in der Nachvollziehbarkeit der Hashchain.

## Fix

1. `member_dao.update(...)` durch `audited_update!(self, MEMBER_SERVICE_PROCESS, &user_id, member_dao, updated_member, tx)` ersetzen.
2. Actor-`user_id` aus dem Authentication-Context beziehen.
3. Test: nach `confirm_migration` existiert ein Audit-Log-Eintrag für `action_count`-Änderung; Hashchain-Verify (`/api/audit/verify`) bleibt grün.

## Akzeptanz

- `confirm_migration` erzeugt einen Audit-Eintrag mit korrektem Actor
- Workspace-Tests grün, clippy clean

## Routing

`/gsd-quick` — mechanischer, lokaler Fix.
