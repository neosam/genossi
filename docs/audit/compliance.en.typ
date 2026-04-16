#import "style.typ": conf, infobox, notebox, fieldtable, accent, muted, rule-color, today-en

#show: doc => conf(
  title: "Audit Log Integrity and Compliance",
  subtitle: "Technical documentation for auditors and operators",
  version: "1.0",
  date: today-en(),
  author: "Genossi project",
  language: "en",
  doc,
)

= Purpose and Scope

This document describes the mechanisms by which the *Genossi* software ensures
that write-operations on business-relevant data are traceable and tamper-evident.
It is addressed to:

- *Auditors* (in particular cooperative auditors under the German § 53 GenG,
  but also compliance, data-protection and IT-security auditors) wishing to
  understand the control framework;
- *Operators* of the software wishing to provide evidence to third parties
  (audit association, supervisory board, external audit) of the controls in
  place.

The presentation is deliberately restricted to aspects that are relevant to
the audit. Detailed implementation guidance and developer notes can be found
in the project's source documentation.

== Reference frameworks

The mechanisms described below align with the following frameworks:

#fieldtable(
  [*GoBD*],
  [German principles for the orderly keeping and retention of books, records
   and documents in electronic form, as well as for data access (issued by
   the Federal Ministry of Finance)],

  [*§ 239 HGB*],
  [Requirements under the German Commercial Code concerning the keeping of
   commercial books (including immutability and traceability)],

  [*eIDAS Regulation*],
  [Regulation (EU) No 910/2014, in particular Articles 41 and 42 (qualified
   electronic time stamps)],

  [*RFC 3161*],
  [Internet X.509 Public Key Infrastructure Time-Stamp Protocol --- the
   technical protocol used for exchanges with time-stamping authorities],

  [*§ 53 GenG*],
  [Mandatory audit of registered cooperatives under the German Cooperatives
   Act],
)

#v(0.6em)

#infobox(title: "Summary")[
  Changes to protected entities are recorded in a cryptographically linked
  hash chain (SHA-256). At regular intervals the current end of the chain is
  bound to an external trust service by a *qualified electronic time stamp*
  under eIDAS. This combination makes subsequent tampering or back-dating of
  records provably detectable.
]

= Architecture Overview

The audit function is structured in several layers and uses established
standards exclusively (SHA-256, ISO 8601, RFC 3161):

#align(center, block(width: 100%, inset: 8pt, stroke: 0.4pt + rule-color, radius: 3pt)[
  #set text(size: 9pt, font: ("DejaVu Sans Mono", "Liberation Mono"))
```
                          Write operation on business data
                       (create / update / delete of a
                       member, application, document …)
                                     │
                                     ▼
              ┌────────────────────────────────────────────┐
              │  Audit macros                              │
              │  compute one entry per changed field,      │
              │  linked by SHA-256 hash to the preceding   │
              │  entry in the table.                       │
              └────────────────────┬───────────────────────┘
                                   │
                                   ▼
              ┌────────────────────────────────────────────┐
              │  Table audit_log                           │
              │  – one row per changed field –             │
              │  Transaction ID groups all field changes   │
              │  belonging to the same business operation. │
              └────────────────────┬───────────────────────┘
                                   │
                                   ▼ (periodically, by background worker)
              ┌────────────────────────────────────────────┐
              │  Qualified time stamp (RFC 3161)           │
              │  The current chain-end hash is submitted   │
              │  to an external TSA; the signed TSR token  │
              │  is stored.                                │
              └────────────────────┬───────────────────────┘
                                   │
                                   ▼
              ┌────────────────────────────────────────────┐
              │  Table audit_timestamp                     │
              │  Retains the time stamps incl. TSR token   │
              │  (optionally mirrored externally via       │
              │  WebDAV).                                  │
              └────────────────────────────────────────────┘
```
])

The separation between *internal integrity* (the hash chain) and
*external anchoring* (the qualified time stamp) is deliberate: the chain
detects any subsequent modification within the database, whereas the
time stamp prevents a tampered chain from being silently recomputed to a
seemingly consistent earlier state.

= Contents of the Audit Log

== Protected entity types

Write operations on business data are logged. The following entity types
are currently subject to audit logging:

- *Member* --- member records of the cooperative
- *MemberAction* --- actions performed on a member
- *MemberDocument* --- documents associated with a member
- *Application* --- membership applications

Additional entities can be added by implementing the internal `Auditable`
trait and using the corresponding audit macros. The mechanism is designed
for future extension.

== Logged operations

#fieldtable(
  [`create`], [Creation of a new record],
  [`update`], [Modification of existing field values],
  [`delete`], [Deletion (implemented as a soft delete via a deletion timestamp)],
  [`snapshot`], [Full representation of a record
                 (administrative retro-documentation)],
)

== Fields of an audit entry

Each row in the `audit_log` table corresponds to exactly one modified field
and contains:

#fieldtable(
  [`id`],             [Unique identifier of the audit entry (UUID)],
  [`timestamp`],      [Time of recording in UTC, ISO 8601],
  [`user_id`],        [Login identity of the triggering user
                       (for system actions: `SYSTEM`)],
  [`process`],        [Name of the triggering process
                       (e.g. `member-service`, `audit-snapshot`)],
  [`transaction_id`], [UUID grouping all field changes of a single business
                       operation],
  [`entity_type`],    [Type of the affected entity (e.g. `member`)],
  [`entity_id`],      [UUID of the affected entity],
  [`action`],         [`create` \/ `update` \/ `delete` \/ `snapshot`],
  [`field_name`],     [Name of the changed field],
  [`old_value`],      [Previous value (empty for `create`)],
  [`new_value`],      [New value (empty for `delete`)],
  [`prev_hash`],      [Hash value of the preceding audit entry],
  [`entry_hash`],     [Hash value of this entry (see section 4)],
)

#v(0.4em)

#infobox(title: "Granularity")[
  One *separate* row is written for each modified field. An operation that
  changes three fields of a member record produces three rows sharing the
  same `transaction_id`. This enables field-level reconstruction of the
  change history.
]

= Tamper-Evidence Through the Hash Chain

== Principle

Audit entries form a linear chain: the `entry_hash` of one row becomes the
`prev_hash` of the *next* row. Any subsequent modification to a previously
written entry destroys the consistency of all following hashes and is
therefore unambiguously detectable.

#v(0.4em)

#align(center, block(width: 100%, inset: 8pt, stroke: 0.4pt + rule-color, radius: 3pt)[
  #set text(size: 9pt, font: ("DejaVu Sans Mono", "Liberation Mono"))
```
  ┌────────────────┐      ┌────────────────┐      ┌────────────────┐
  │    Entry 1     │      │    Entry 2     │      │    Entry 3     │
  │ ──────────── 　│      │ ──────────── 　│      │ ──────────── 　│
  │ prev_hash  = ""│      │ prev_hash = h₁ │      │ prev_hash = h₂ │
  │ fields …       │      │ fields …       │      │ fields …       │
  │ entry_hash= h₁ │━━━━━▶│ entry_hash= h₂ │━━━━━▶│ entry_hash= h₃ │
  └────────────────┘      └────────────────┘      └────────────────┘
```
])

#v(0.4em)

Tampering with e.g. the `new_value` of Entry 2 would change `h₂`. Since `h₂`
is fixed as `prev_hash` in Entry 3, Entry 3 would also have to be recomputed,
and so on for every subsequent entry. Without simultaneously rewriting
*all* later entries *and* all affected time-stamp records (see section 5),
any tampering is immediately detectable.

== Hash computation

The `entry_hash` is computed as the SHA-256 digest over a canonical
representation of all entry fields. Inputs, in fixed order:

#set enum(numbering: "1.")
+ Timestamp (ISO 8601, UTC)
+ User identity (`user_id`)
+ Process name
+ Transaction UUID
+ Entity type
+ Entity UUID
+ Operation (`action`)
+ Field name
+ Previous value
+ New value
+ Hash of the preceding entry (`prev_hash`)

SHA-256 corresponds to the current state of the art for cryptographic hash
functions and is considered secure by the German BSI.

== Internal verification

The software provides a `verify_chain` function that, for a set of audit
entries, checks

- that each entry carries a stored `entry_hash` which can be recomputed from
  the listed fields, and
- that the `prev_hash` values form an unbroken chain.

The REST endpoint `GET /api/audit/verify` performs this check over the
entire chain and reports any "broken links" (incorrectly chained or
tampered entries).

= Qualified Electronic Time Stamp

== Purpose

The hash chain alone proves that no *internal* inconsistency exists. An
adversary with full database access could theoretically recompute the
entire chain along with all hashes. To prevent this in practice, the
current chain-end hash is periodically bound to an *external trust service*.

The software uses the *Time-Stamp Protocol* defined in RFC 3161. The
external service (Time Stamping Authority, TSA) signs the submitted hash
together with its own time signal. The resulting *TSR token* is legally
secured by the TSA's certificate.

Where a TSA from the *EU Trust List* (LOTL) is used that is listed as a
qualified trust service provider under eIDAS, the resulting time stamps are
*qualified electronic time stamps* within the meaning of Article 42 of the
eIDAS Regulation. They therefore enjoy the Union-law presumption of
accuracy of time and integrity of the signed data.

#v(0.4em)

#infobox(title: "Role of the TSA")[
  The TSA never sees confidential data. Only a hash value is transmitted.
  No conclusions about the underlying audit entries can be drawn from this
  hash. Using an external TSA therefore does not breach any confidentiality
  obligations.
]

== Workflow

#align(center, block(width: 100%, inset: 8pt, stroke: 0.4pt + rule-color, radius: 3pt)[
  #set text(size: 9pt, font: ("DejaVu Sans Mono", "Liberation Mono"))
```
   Genossi worker                    TSA (external)              Storage
   ──────────────                    ──────────────              ───────
      │
      │ 1. latest entry_hash of
      │    the audit_log chain
      │
      │ 2. SHA-256(hash) ────────────▶│
      │                               │ 3. signed with
      │                               │    TSA certificate
      │ ◀───────── TSR token ─────────│
      │                               │
      │ 4. TSR token + audit hash     │
      │    + entry count         ──────────────────────────▶  audit_timestamp
      │                               │
      │ 5. (optional) WebDAV upload ─────────────────────────▶  external
      │                                                         archive
```
])

#v(0.4em)

Importantly, *plain-text business data never leaves the system*. The TSA
receives nothing but a hash value.

== Interval

The worker runs at an interval configured by the operator. The default is
one week (168 hours). If the chain-end hash has not changed since the
previous time stamp (for example because no write operations have
occurred), no new TSA call is made and no costs are incurred. See section 9
for configuration details.

== Choice of TSA

The choice of TSA is made by the operator. In order for the resulting time
stamps to qualify as *qualified electronic time stamps* under eIDAS, the
TSA must be listed in the EU Trust List. An up-to-date listing is provided
at:

#align(center, link("https://eidas.ec.europa.eu/efda/tl-browser/"))

The TSA actually used by a given installation is to be recorded in the
*operator documentation* (see separate template
`template-betreiber.de.pdf`).

= Roles and Access

The audit system distinguishes between read and write privileges:

#fieldtable(
  [*Regular users*],
  [Trigger audit entries through their business transactions but cannot
   view, modify or delete audit entries themselves.],

  [*Administrators*],
  [Can retrieve the complete audit log through the REST API and run
   integrity checks on chain and time stamps. They *cannot* modify or
   delete audit entries --- the data model provides no such operation.],

  [*Technical system processes*],
  [The time-stamp worker runs as user `SYSTEM` and is only permitted to
   create *new* time-stamp records.],
)

#v(0.4em)

#notebox(title: "No deletion API")[
  The software exposes neither REST endpoints nor service methods to modify
  or remove audit entries. Only a direct administrative intervention on the
  underlying database could alter an entry --- any such intervention breaks
  the hash chain and is therefore detectable via the internal and external
  verification mechanisms.
]

= Retention and Export

== Database

Audit log entries and time stamps are stored in the same database as the
business data. Data retention is therefore subject to the same rules as
the remaining business data (operator's backup policy).

== External mirroring (optional)

The software optionally transfers TSR tokens to an external store (WebDAV)
after successful creation. This ensures that at least one complete
time-stamp record resides outside the primary system, in keeping with the
GoBD principle of holding "evidence outside the accounting system" for
additional tampering protection.

The `audit_timestamp` table holds, for each time-stamp transaction, the
basic metadata (time, associated hash, count of covered entries, status)
together with the raw TSR token as a binary field. This token can be
exported at any time for external verification.

= Verification by an Auditor

An auditor has three mutually independent ways to verify the integrity of
the audit chain. This independence is a deliberate feature of the control
framework.

== 1. Internal verification

Chain verification (calling `GET /api/audit/verify`) can be triggered from
the administration interface. The system reports whether the chain is
consistent. This check can be performed at any time and incurs no cost.

== 2. Inspection of time-stamp entries

Each row in the `audit_timestamp` table shows:

- the point in time at which the time stamp was requested,
- the chain-end hash current at that point,
- the number of audit entries covered, and
- the status (`success` for a successful TSA response).

These fields can be displayed through the administration interface and
printed as evidence.

== 3. External verification of the TSR token

For a cryptographic verification independent of the software, a TSR token
can be exported and verified with an external tool. The *DSS Demo web
application* of the European Commission is recommended:

#align(center, link("https://ec.europa.eu/digital-building-blocks/DSS/webapp-demo/"))

The application validates the token against the EU Trust List (LOTL),
verifies the TSA certificate signature and produces a formal validation
report as PDF, which can be attached to the audit report.

#infobox(title: "External verification steps")[
  1. Select a current time-stamp entry through the administration interface.
  2. Export the associated TSR token as a file.
  3. Upload the file to the DSS web application.
  4. The application displays signature status, certificate chain and
     time and offers a signed validation report for download.
]

= Operator Configuration

The software deliberately makes no assumption about the specific trust
service provider in use. The operator configures:

#fieldtable(
  [`tsa_enabled`],         [Activation of the time-stamp function (`true`/`false`)],
  [`tsa_url`],             [URL of the TSA endpoint (RFC 3161)],
  [`tsa_user` (optional)], [HTTP Basic Auth username],
  [`tsa_pass` (optional)], [HTTP Basic Auth password],
  [`tsa_interval_hours`],  [Interval between time-stamp requests],
)

The concrete values in a specific installation form part of the
*operator documentation*, for which a template is provided.

= Implementation Boundaries

A boundary of the current implementation is that the software's own
`verify` function does not independently check the cryptographic
*signature* of the TSR token against the TSA certificate; instead it
verifies *chain consistency* and parseability of the token (TSA response
status code).

The actual qualified cryptographic verification is deliberately delegated
to an external, independent verification path (see section 8.3). This
follows the information-security principle of *separating verification
tools from production tools*: a potentially compromised system must not
be both the producer *and* the sole verifier of its own evidence.

#notebox(title: "Note")[
  For auditors: this boundary is transparently documented and
  architecturally intentional. Proof of the qualified signature --- and
  therefore of the legal effect under Article 42 eIDAS --- is provided
  through the external DSS web application of the European Commission.
]

= Appendix A --- Glossary

#fieldtable(
  [*SHA-256*],
  [Standardised hash function (FIPS 180-4). Produces a 256-bit fingerprint
   of any input. Collision-resistant according to the current state of the art.],

  [*RFC 3161*],
  [Internet standard for time-stamp services. Defines the request format
   ("TimeStampReq") and response format ("TimeStampResp" containing the TSR
   token).],

  [*TSA*],
  [Time Stamping Authority --- trust service provider issuing time stamps
   according to RFC 3161.],

  [*TSR token*],
  [Time-Stamp Response token. Data packet signed by the TSA containing the
   submitted hash, a time stamp and the TSA signature.],

  [*eIDAS Regulation*],
  [EU Regulation No 910/2014 on electronic identification and trust
   services. Regulates, inter alia, qualified electronic time stamps
   (Articles 41--42).],

  [*EU Trust List (LOTL)*],
  [Official list of qualified trust service providers notified by EU
   member states.],

  [*GoBD*],
  [German principles for the orderly keeping and retention of books,
   records and documents in electronic form. BMF circular, most recent
   version 28 November 2019.],

  [*DSS*],
  [Digital Signature Service. Open-source European Commission tool for
   verifying electronic signatures and time stamps against the EU Trust List.],

  [*Hash chain*],
  [Data structure in which each entry contains a hash of the previous
   entry. Changes near the start of the chain invalidate all subsequent
   hashes and are therefore detectable.],

  [*Transaction UUID*],
  [Unique identifier grouping all audit entries belonging to *one*
   business operation, even though each field change is stored in a
   separate row.],
)

= Appendix B --- References

- German Federal Ministry of Finance (BMF): *Principles for the orderly
  keeping and retention of books, records and documents in electronic form
  (GoBD)*, version of 28 November 2019.
- Regulation (EU) No 910/2014 of the European Parliament and of the Council
  of 23 July 2014 on electronic identification and trust services for
  electronic transactions in the internal market (*eIDAS Regulation*).
- IETF: *RFC 3161 --- Internet X.509 Public Key Infrastructure Time-Stamp
  Protocol (TSP)*, August 2001.
- NIST: *FIPS PUB 180-4 --- Secure Hash Standard*, August 2015.
- European Commission: *DSS Demo Web Application*,
  #link("https://ec.europa.eu/digital-building-blocks/DSS/webapp-demo/").
- European Commission: *EU Trust List Browser*,
  #link("https://eidas.ec.europa.eu/efda/tl-browser/").
- Federal Law Gazette: *German Cooperatives Act (GenG)*, in particular
  § 53 (mandatory audit).
