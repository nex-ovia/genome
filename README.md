# genome

The **nexovia project genome** — one editable, version-controlled source of truth that
describes this project end to end, and (because nexovia describes *itself*) the
design-first spec for building the nexovia tool. There is no separate spec document:
the genome is the plan.

## Layout

```
nexovia.toml            the anchor — project identity + architecture spine + [include]
project/                this project's volatile, generated-then-refined fragments
  components.toml         modules + links (the structural reality)
  policies.toml           local policies (override/extend the org bases)
  lifecycle.toml          the phased roadmap — features, enhancements, bugs
  delivery.toml           estimates, resourcing, cost
  deployment.toml         how it ships (binary / container / saas …)
  quality.toml            performance budgets + QA gates
org/                    SHARED, centrally-owned standards — inherited, never copied
  nxs-topology.toml       the design pattern (hexagonal + 12-factor), extended by the anchor
  coding-standards.toml   org-wide policies (readability, security, testing)
schema/
  nexovia-2.toml          the formal grammar `validate` enforces (resolves schema = "nexovia/2")
examples/strategy-doc/  a NON-code (document) genome — proves domain-generality
mock/report.html        the rendered human view (the render target / golden fixture)
build_mock.py           reference resolver for this genome  → mock/report.html
build_example.py        reference resolver for the example  → examples/strategy-doc/report.html
```

## What's generated vs. shared

The tool **generates** `nexovia.toml` + `project/*` by ingesting a one-pager, a repo, or a
prompt. It **references** (never regenerates) the shared `org/*` standards and the
tool-shipped `schema/`. Humans rarely edit — only to set a guardrail, which is sticky
(`by=human`) and never overwritten by an agent or a re-scan.

## The phased plan lives in the genome

`project/lifecycle.toml` is the phased roadmap (each item linked to the components it
touches); `project/delivery.toml` carries the estimates and cost; `quality.toml` carries
the QA gates. Read them rendered in `mock/report.html` — Executive / Architect / Engineer
views, plus completion and drift.

## Render

```
python3 build_mock.py       # → mock/report.html            (this project)
python3 build_example.py    # → examples/strategy-doc/report.html   (the document example)
```

Open either HTML in any browser — self-contained, offline, no dependencies.
