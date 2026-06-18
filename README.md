# nexovia · `genome`

[![CI](https://github.com/nex-ovia/genome/actions/workflows/ci.yml/badge.svg)](https://github.com/nex-ovia/genome/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/nex-ovia/genome?include_prereleases&label=release)](https://github.com/nex-ovia/genome/releases)
[![License: BSL 1.1](https://img.shields.io/badge/license-BSL%201.1-blue.svg)](LICENSE)

**One file is your project's genome** — a single, version-controlled source of truth that lets
non-developers understand a system precisely enough to **ask the right question and approve it**,
without reading code.

> Brand: **nexovia** · Binary: **`genome`** · Config: **`nexovia.toml`** · Schema: `nexovia/2`

> ⚠️ **Experimental.** This is an early MVP, released as a pre-release for validation and feedback.
> The genome schema, CLI surface, and HTML output may change without notice — not yet stable.
> Download the latest experimental build from the [**Releases**](https://github.com/nex-ovia/genome/releases) page.

---

## The problem

Architects, implementation engineers, forward-deployed engineers, and executives constantly need to
understand a system — its architecture, what each part does, and *why* — in order to make a call:
*Is this right? What should I ask? Can I approve it?* Today that means reading code, chasing authors,
or trusting a stale wiki. The understanding drifts from the system the moment it's written down.

## The idea

`genome` keeps one editable **`nexovia.toml`** that describes a project end to end — its architecture,
every module with its functions and libraries, the rationale behind each part, and the policies that
keep it honest. The TOML files are the **database / control plane**. The HTML is a **static,
read-only window** onto them:

```
   document ──▶ genome from-doc ──▶ nexovia.toml  (design-first nodes)
                                          │
                                          ▼  genome enrich   (offline, embedded GGUF model)
                              plain-language summary / why per node   (by=llm + confidence)
                                          │
   genome ask <node|*> "question"  ──┐    │    ┌──  genome approve <node|*> --decision …
   (writes questions facet)          ▼    ▼    ▼    (writes approvals facet)
                              ┌────────────────────────┐
                              │      genome render       │ ──▶ report.html  (static, offline)
                              └────────────────────────┘       Exec / Arch / Eng lenses, each node:
                                                                summary · why · status · questions · approvals
```

Re-render to refresh. The HTML is a pure projection of the TOML — there is no hidden state.

## What makes it trustworthy

- **Honest by construction.** Every semantic field carries provenance (`by` = human / scan / llm) and
  a confidence. Machine guesses never overwrite a human's edit — provenance is sticky.
- **Offline and network-free by default.** The default binary has no HTTP client and no inference
  engine in its dependency graph. Comprehension stays local.
- **Optional embedded intelligence.** The `enrich` feature fills empty `summary` / `why` fields using
  an in-process GGUF "flash" model — fetched once, then fully offline. Each result is stamped
  `by=llm` with a calibrated confidence, so you always know what was inferred.
- **The genome is the plan.** This project is built with itself: its own roadmap, components, and
  policies live in the genome TOML and render to the same HTML.

## Quickstart

```sh
# build the default (offline, inference-free) binary
cargo build --release

# render the genome to a self-contained, read-only HTML map
genome render nexovia.toml > report.html

# validate the genome against the schema + honesty + integrity rules
genome validate nexovia.toml

# turn a one-pager / spec into a design-first genome
genome from-doc strategy.md nexovia.toml

# ask a question or record an approval (written back into the TOML)
genome ask render_html "why is this on the surface layer?" --role arch
genome approve "*" --role arch --decision approved --note "MVP scope looks right"

# optional: fill blank why/summary fields with the embedded offline model
cargo build --release --features enrich
genome enrich nexovia.toml
```

Open `report.html` in any browser — self-contained, offline, no dependencies.

## Repository layout

```
nexovia.toml            the anchor — project identity + architecture spine + [include]
project/                this project's generated-then-refined fragments
  components.toml         modules + links (the structural reality)
  policies.toml           local policies (override/extend the org bases)
  lifecycle.toml          the phased roadmap — features, enhancements, bugs
  questions.toml          questions raised against nodes or the whole snapshot
  approvals.toml          approval / rejection decisions, per node + per snapshot
  delivery.toml           estimates, resourcing, cost
  deployment.toml         how it ships (binary / container / saas …)
  quality.toml            performance budgets + QA gates
org/                    SHARED, centrally-owned standards — inherited, never copied
  nxs-topology.toml       the design pattern (hexagonal + 12-factor), extended by the anchor
  coding-standards.toml   org-wide policies (readability, security, testing)
schema/
  nexovia-2.toml          the formal grammar `validate` enforces (resolves schema = "nexovia/2")
src/                    the Rust workspace (genome contract, ingest, enrich, render, validate, cli)
examples/               worked genomes (a non-code document genome proves domain-generality)
mock/report.html        the rendered human view — regenerated by `genome render`
```

Humans rarely edit the TOML directly — only to set a guardrail, which is sticky (`by=human`) and
never overwritten by an agent or a re-scan.

## Status

This is an early MVP that proves the loop — **understand → ask → approve** — fully offline:

- ✅ Genome contract + byte-stable TOML I/O
- ✅ `genome render` → self-contained HTML (Exec / Arch / Eng lenses)
- ✅ `genome validate` → schema + honesty + integrity gates
- ✅ `genome from-doc` → document to design-first nodes
- ✅ `genome enrich` → offline embedded-GGUF enrichment (`enrich` feature)
- ✅ `genome ask` / `genome approve` → questions + approvals as TOML facets

**Designed for, built later:** codebase ingest (tree-sitter), LSP surface, skeleton scaffolding,
freeze + drift tracking, an MCP server for agents, and optional cloud enrichment.

## License

Source-available under the **Business Source License 1.1** (see [`LICENSE`](LICENSE)). You may use
`genome` internally on your own projects for free; offering it to third parties as a hosted or
managed service requires a commercial license. On the Change Date (2030-06-17) the code converts to
the **Apache License 2.0**. For commercial arrangements, contact amitkumar.srivastava42@gmail.com.
