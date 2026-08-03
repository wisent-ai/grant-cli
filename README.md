# Grant CLI

**Grant CLI is a local-first workspace for discovering grant opportunities,
checking eligibility, authoring source-backed applications, retaining evidence,
and exporting a reviewable submission package.**

The local SQLite workspace remains usable without a hosted Wisent service.
Managed collaboration and opportunity intelligence are separate, fail-closed
capabilities.

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

- a local SQLite system of record under an explicit `GRANT_HOME`;
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
- The local product does not require a paid organization entitlement.
- Managed-service failure must not block access to the local workspace and its
  evidence.

### Supported environment and current capability

| Surface | Requirement | Current state |
|---|---|---|
| Local CLI and workspace | Rust build supported by `Cargo.lock` | Implemented |
| Local SQLite state | writable `GRANT_HOME` | Implemented |
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
       local SQLite evidence workspace
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

The local database is authoritative for the workspace record. External sources
remain authoritative for funder rules. Applicant-approved organization facts
remain authoritative for the applicant. Managed intelligence may assist, but it
must fail closed and must not rewrite local evidence as fact.

## Quick start

This safe path creates an isolated local workspace and installs the built-in
source catalog and knowledge patterns. It does not contact a funder or submit an
application.

### Prerequisites

- Git;
- the Rust toolchain compatible with `Cargo.lock`;
- a writable temporary directory.

```bash
git clone https://github.com/wisent-ai/grant-cli.git
cd grant-cli
cargo build --locked
GRANT_HOME="${TMPDIR:-/tmp}/grant-cli-quickstart" \
  cargo run --locked -- --json init
```

Expected JSON contains `home`, `sources`, and `patterns`. The command creates
local SQLite state under the selected directory. Remove only that disposable
quick-start directory when it is no longer needed.

Inspect the current command surface:

```bash
cargo run --locked -- --help
cargo run --locked -- opportunity --help
cargo run --locked -- application --help
```

Real source retrieval may make network requests and real documents may contain
confidential applicant data. Use an approved workspace path before importing
non-public material.

## Primary interfaces

- **CLI:** installed binary `grant`; command families include `source`,
  `opportunity`, `organization`, `eligibility`, `application`, `task`,
  `document`, `guide`, `pattern`, `comment`, `field`, `claim`, `budget`,
  `review`, `outcome`, `analytics`, and `export`.
- **Machine output:** global `--json` returns structured command results.
- **Workspace:** `GRANT_HOME` or `--home` selects the local database and retained
  evidence.
- **Platform entitlement:** `grant.local` remains community capability;
  `grant.organization` and `grant.opportunity-intelligence` are managed
  capabilities.

## Operational model

- **Configuration:** explicit `GRANT_HOME` plus command arguments; managed
  services require separate platform identity and entitlement.
- **State:** SQLite and exported files under the selected local workspace.
- **Credentials:** any private source or managed-service credentials remain
  outside application content and must not be exported into a submission.
- **Observability:** JSON results, review output, analytics, and retained source
  relationships distinguish missing data from failed external retrieval.
- **Recovery:** preserve and back up the selected workspace before migration;
  managed outage must leave local records available.
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
