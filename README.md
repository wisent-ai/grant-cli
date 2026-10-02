<!-- wisent-banner:start -->
<p align="center">
  <img src="assets/readme-banner.webp" alt="grant-cli by Wisent" width="100%">
</p>
<!-- wisent-banner:end -->

<!-- wisent-readme-signals:start -->
[![Source](https://img.shields.io/badge/GitHub-Source-181717?logo=github)](https://github.com/wisent-ai/grant-cli) [![Issues](https://img.shields.io/badge/GitHub-Issues-181717?logo=github)](https://github.com/wisent-ai/grant-cli/issues) [![Wisent](https://img.shields.io/badge/Wisent-Website-0B0B0B)](https://wisent.com) [![Discord](https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white)](https://discord.gg/qRjpkthq54) [![LinkedIn](https://img.shields.io/badge/LinkedIn-Follow-0A66C2?logo=linkedin&logoColor=white)](https://www.linkedin.com/company/wisent-ai/) [![X](https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white)](https://x.com/wisentai) [![Enterprise](https://img.shields.io/badge/Enterprise-Book%20a%20call-0B0B0B?logo=calendly)](https://calendly.com/lbartoszcze)
<!-- wisent-readme-signals:end -->

# Grant CLI

Automated AI Harness for Grant Funding.

You are leaving money on the table. The governments of the world want to support
your project with non-dilutive funding. Money you could use to expand your team,
do more research and overtake your competitors is stuck in a labyrinth of
regulations, illegible interfaces and opaque rules. Our AI keeps track of all
opportunities in the EU, USA, Australia and Singapore to identify opportunities
for funding, extra partnerships and client outreach opportunities. Grant CLI
keeps track of deadlines, drafts proposals based on published criteria and
actually fills in the interfaces — all you have to do is sign off. All texts are
humanised to bypass AI-detection scanners.

Get more money for your business without diluting yourself.

Grant data lives in the Wisent fleet database `grant-cli`, which Stado
provisions and names; every machine in the fleet reads and writes the same
record. Managed collaboration and opportunity intelligence are separate,
fail-closed capabilities.

[Quick start](#quick-start) · [Command surface](#primary-interfaces) ·
[Canonical repository](https://github.com/wisent-ai/grant-cli)

Current boundary: version `0.1.0` is development source. It helps prepare and
review applications; it does not guarantee eligibility, award, legal compliance,
or acceptance by a funder.

## Problem and intended users

Grant work combines changing source material, eligibility rules, organization
facts, application fields, budgets, documents, reviewer comments, and deadlines.
Spreadsheets and copied documents lose provenance and make it difficult to show
which claim is supported by which source.

Grant CLI serves:

- **grant researchers** collecting official opportunity sources and deadlines;
- **applicants and proposal writers** maintaining organization facts,
  eligibility, claims, budgets, and documents;
- **reviewers** checking completeness, evidence, consistency, and export state;
- **teams** that may later use managed collaboration without surrendering the
  local application record.

## Product boundaries

### Included

- the fleet database `grant-cli` as the system of record, and fetched source
  objects and exports in the directory `GRANT_HOME` names;
- source cataloguing and retrieval;
- opportunity discovery, search, qualification, and deadline records;
- organization profiles and eligibility checks;
- applications, tasks, documents, guides, patterns, reviewer comments, fields,
  claims, and budgets;
- source-backed knowledge and authoring assistance;
- deterministic review, analytics, outcome tracking, and application export;
- JSON output for machine consumers.

### Explicit non-goals

- Grant CLI does not provide legal, tax, accounting, or funding advice.
- It does not guarantee that an applicant is eligible or that a funder will
  accept or award an application.
- It does not submit an application to a funder unless a separately documented,
  explicitly authorized delivery integration exists.
- It does not make scraped or model-generated text authoritative; official
  sources and applicant-approved facts remain required.
- It does not invent organization facts, citations, budget values, or evidence.
- The CLI does not require a paid organization entitlement.
- Managed-service failure must not block access to the workspace record and its
  evidence.

### Supported environment and current capability

| Surface | Requirement | Current state |
|---|---|---|
| CLI | Rust build supported by `Cargo.lock` | Implemented |
| Fleet database state | Stado with `grant-cli` declared, and the `grant-cli-database-client` bearer | Implemented |
| Source and document ingestion | supported HTTP/PDF/XML/ZIP inputs | Implemented |
| JSON automation | `--json` | Implemented |
| Managed organization collaboration | platform entitlement | Contract declared; hosted availability separate |
| Managed opportunity intelligence | platform entitlement | Contract declared; hosted availability separate |
| Stable hosted service | — | Not published |

## Core use cases

### Build a source-backed opportunity record

- **Actor:** a grant researcher.
- **Initial state:** the researcher has an official source URL or document and a
  local workspace.
- **Outcome:** Grant CLI retains the source, extracts an opportunity record, and
  keeps the source relationship available for review.
- **Boundary:** extracted fields are not silently promoted above the official
  source and uncertain values remain review work.

### Qualify an organization

- **Actor:** an applicant or proposal lead.
- **Initial state:** organization facts and opportunity eligibility criteria are
  present.
- **Outcome:** the workspace records eligibility results and missing information
  before authoring proceeds.
- **Boundary:** the result is a preparation aid, not a funder's binding decision.

### Author and review an application

- **Actor:** a proposal writer and reviewer.
- **Initial state:** an application links the intended opportunity,
  organization, tasks, documents, claims, and budget.
- **Outcome:** the writer maintains the application while review identifies
  incomplete, inconsistent, or unsupported fields.
- **Boundary:** generated or suggested language cannot replace applicant-approved
  facts and retained evidence.

### Export a submission package

- **Actor:** an authorized application owner.
- **Initial state:** review is complete enough for the owner's workflow and the
  output path is explicit.
- **Outcome:** Grant CLI exports the selected application package for human
  inspection or an authorized downstream system.
- **Boundary:** export is not submission, funder acceptance, or proof that every
  jurisdictional requirement was met.

## How Grant CLI works

```text
official sources + organization facts
                 │
                 ▼
     fleet database `grant-cli` (evidence record)
                 │
   ┌─────────────┼──────────────┐
   ▼             ▼              ▼
opportunity   eligibility   application authoring
   │             │              │
   └─────────────┴──────────────┘
                 ▼
       deterministic review and export
                 │
                 ▼
       human-approved submission package
```

The fleet database is authoritative for the workspace record. External sources
remain authoritative for funder rules. Applicant-approved organization facts
remain authoritative for the applicant. Managed intelligence may assist, but it
must fail closed and must not rewrite local evidence as fact.

## Quick start

This path installs the built-in source catalog and knowledge patterns into the
fleet database. It does not contact a funder or submit an application.

### Prerequisites

- Git;
- the Rust toolchain compatible with `Cargo.lock`;
- Stado installed at `~/.stado/bin/stado`, answering
  `stado database resolve grant-cli --consumer grant-cli --json`;
- the Skarbiec bearer of consumer `grant-cli-database-client` in
  `~/.stado/grant-cli-database-client-skarbiec-token`, which may read
  `grant-cli-database#pooler_url` and `grant-cli-database#ca_certificate`.

```bash
git clone https://github.com/wisent-ai/grant-cli.git
cd grant-cli
cargo build --locked
cargo run --locked -- --json init
```

Expected JSON contains `home`, `sources`, and `patterns`. The first command
creates grant-cli's tables in the fleet database if they are missing.

### Where the data lives and how a failure reads

Every command connects in four steps, and a failure names the step:

1. `stado database resolve grant-cli --consumer grant-cli --json` names the
   Skarbiec item that holds the address (`grant-cli-database`).
2. `stado service directory connect skarbiec --consumer grant-cli --json`
   gives the Skarbiec route.
3. `stado secrets get grant-cli-database --field pooler_url` and
   `--field ca_certificate`, as consumer `grant-cli-database-client`, give the
   pooler URL and the provider's root certificate.
4. grant-cli connects over TLS verified against that certificate and creates
   any missing table.

`Stado is not installed at …` means step 1 cannot start. `stado … exited …`
quotes Stado's own refusal of steps 1 to 3. `… is not a PEM certificate` or
`… is not a Postgres connection URL` means the Skarbiec item holds a malformed
field. `connecting to the fleet database grant-cli … failed` carries the
Postgres or TLS error of step 4. A data error after that reads
`the fleet database refused: …`.

Inspect the current command surface:

```bash
cargo run --locked -- --help
cargo run --locked -- opportunity --help
cargo run --locked -- application --help
```

`grant organization delete <slug>` removes an organization and its evidence;
the database refuses while an application still names it.

Real source retrieval may make network requests and real documents may contain
confidential applicant data.

## Primary interfaces

- **CLI:** installed binary `grant`; command families include `source`,
  `opportunity`, `organization`, `eligibility`, `application`, `task`,
  `document`, `guide`, `pattern`, `comment`, `field`, `claim`, `budget`,
  `review`, `outcome`, `analytics`, and `export`.
  A completed task and a resolved comment can be taken back: `grant task
  reopen <task>` (refused while a completed task depends on it) and `grant
  comment reopen <comment>` (clears the resolution); both are refused on a
  task or comment that is not done or resolved, and both are kept in the
  application's activity log. What is added can be taken out the same way:
  `grant opportunity unwatch <opportunity>` (refused when it is not
  watched), `grant eligibility rule-remove <rule>` and `grant organization
  evidence-remove <evidence>`; later assessments no longer see what was
  removed. `grant source disable <source>` stops syncing a source and keeps
  its snapshots and opportunities; `grant source enable <source>` resumes it.
  Links come apart the same way they were made: `grant field
  unlink-requirement|unlink-criterion <application> <field> <id>`, `grant
  claim unlink <link>` (a claim left with no evidence is `unverified`
  again, also when the evidence itself is removed), and `grant budget
  line-remove <application> <line>`; each refuses a link or line that does
  not exist. `grant pattern remove <pattern>` drops a pattern and keeps its
  examples, detached; `grant pattern example-remove <example>` drops one
  example.
- **Machine output:** global `--json` returns structured command results.
- **Workspace:** the fleet database `grant-cli` holds the record; `GRANT_HOME`
  or `--home` selects the directory for fetched source objects and exports.
- **Platform entitlement:** `grant.local` remains community capability;
  `grant.organization` and `grant.opportunity-intelligence` are managed
  capabilities.

## Operational model

- **Configuration:** explicit `GRANT_HOME` plus command arguments; managed
  services require separate platform identity and entitlement.
- **State:** the fleet database `grant-cli`, reached through the shared
  `stado-database` crate as a SeaORM connection (the account's home is
  `GRANT_FLEET_HOME` when set, else `HOME`); fetched objects and exported
  files under the selected local directory. A refusal names the step that
  failed: `resolve database`, `resolve Skarbiec route`, `read credential
  field`, `read pooler_url` or `connect`.
- **Credentials:** any private source or managed-service credentials remain
  outside application content and must not be exported into a submission.
- **Observability:** JSON results, review output, analytics, and retained source
  relationships distinguish missing data from failed external retrieval.
- **Recovery:** the fleet database is Supabase Postgres provisioned by
  `stado database create grant-cli`; its backups follow that project.
- **Cost:** the local workspace has no hosted entitlement requirement. Managed
  collaboration or intelligence pricing is not published in this repository.

## Project status and support

- **Maturity:** public development source, version `0.1.0`.
- **Local contract:** implemented local workspace and CLI.
- **Managed contract:** declared by `platform-entitlements.json`; availability
  and pricing require separate approved service operation.
- **Issues:** [`wisent-ai/grant-cli`](https://github.com/wisent-ai/grant-cli/issues).
- **Security and privacy:** report vulnerabilities privately; never attach
  applicant records, budgets, personal data, credentials, or unpublished
  applications to a public issue.
- **License:** Apache License 2.0; see [`LICENSE`](LICENSE).