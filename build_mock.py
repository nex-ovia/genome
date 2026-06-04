#!/usr/bin/env python3
"""Resolve the composed genome and embed it into the mock.

Mirrors what Rust genome_io/render will do in Phase 1:
  1. load the anchor (nexovia.toml): project + architecture + [include],
  2. resolve [architecture].extends -> shared base topology, apply overrides,
  3. resolve [include].nodes -> modules + edges,
  4. resolve [include].policies -> shared bases (org/*) under local overrides,
  5. resolve [include].lifecycle -> features / enhancements / bugs,
each marked with provenance (inherited / override / local). The TOML on disk is
the SOURCE; the embed is the RESOLVED view, exactly what `nexovia render` computes.
"""
import tomllib, json, re
from pathlib import Path

root = Path(__file__).parent
load = lambda f: tomllib.load(open(root / f, "rb"))
genome = load("nexovia.toml")
inc = genome.get("include", {})

# --- architecture: extends base ⊕ local overrides --------------------------
arch = genome["architecture"]
base = load(arch["extends"])
overrides = arch.get("layers", {})
layers = {}
for name, L in base["layers"].items():
    merged, ov = dict(L), []
    for k, v in overrides.get(name, {}).items():
        if merged.get(k) != v:
            ov.append(k)
        merged[k] = v
    merged["_ov"] = ov
    merged["_src"] = "override" if ov else "inherited"
    layers[name] = merged
for name, L in overrides.items():
    if name not in layers:
        layers[name] = {**L, "_ov": list(L.keys()), "_src": "local", "_added": True}
architecture = {
    "pattern": base["pattern"], "basis": base["basis"],
    "dependency_rule": base["dependency_rule"], "description": base["description"],
    "extends": arch["extends"], "fluid": arch.get("fluid", False), "layers": layers,
}

# --- nodes + edges (from include.nodes) ------------------------------------
nodes, edges = {}, []
for f in inc.get("components", []):
    d = load(f)
    nodes.update(d.get("components", {}))
    edges.extend(d.get("links", []))

# --- policies: shared bases (org/*) under local overrides ------------------
pol_tables = lambda d: {k: v for k, v in d.items() if isinstance(v, dict)}
base_pol, local_pol = {}, {}
for f in inc.get("policies", []):
    (base_pol if f.startswith("org/") else local_pol).update(pol_tables(load(f)))
policies = {}
for name, body in base_pol.items():
    policies[name] = {**body, "_src": "inherited", "_ov": []}
for name, body in local_pol.items():
    if name in policies:
        ov = [k for k, v in body.items() if policies[name].get(k) != v]
        policies[name] = {**policies[name], **body, "_src": "override", "_ov": ov}
    else:
        policies[name] = {**body, "_src": "local", "_ov": []}

# --- lifecycle: features / enhancements / bugs -----------------------------
lifecycle = []
for f in inc.get("lifecycle", []):
    lifecycle.extend(load(f).get("item", []))

# --- planning facets: delivery / deployment / quality ----------------------
delivery = {}
if inc.get("delivery"):
    d = load(inc["delivery"][0]); delivery = {**d.get("delivery", {}), "estimates": d.get("estimate", [])}
deployment = load(inc["deployment"][0]).get("deployment", {}) if inc.get("deployment") else {}
quality = {}
if inc.get("quality"):
    q = load(inc["quality"][0]); quality = {**q.get("quality", {}), "budgets": q.get("budget", []), "gates": q.get("gate", [])}

embed = {
    "project": dict(genome["project"]), "architecture": architecture,
    "policies": policies, "policy_includes": [f for f in inc.get("policies", []) if f.startswith("org/")],
    "components": nodes, "links": edges, "lifecycle": lifecycle,
    "delivery": delivery, "deployment": deployment, "quality": quality,
}
embed["project"]["intent"] = " ".join(embed["project"]["intent"].split())  # formatter-safe (no \n)

js = json.dumps(embed, ensure_ascii=False, separators=(",", ":"))
assert "\n" not in js, "embed must not contain raw or escaped newlines"

mock_path = root / "mock/report.html"
mock2, n = re.subn(r"const GENOME = \{.*?\};\n", "const GENOME = " + js + ";\n",
                   mock_path.read_text(), count=1, flags=re.S)
assert n == 1, f"GENOME literal not found/replaced (n={n})"
mock_path.write_text(mock2)

ov = {k: v["_ov"] for k, v in {**layers, **policies}.items() if v.get("_ov")}
print(f"resolved: {len(nodes)} components, {len(edges)} links, {len(layers)} layers, "
      f"{len(policies)} policies, {len(lifecycle)} lifecycle items")
print(f"overrides: {ov or 'none'}")
print(f"embedded into mock ({len(mock2)} chars)")
