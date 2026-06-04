#!/usr/bin/env python3
"""Render the non-code (document) example with the SAME renderer as nexovia.

Proves domain-generality: a self-contained `domain="document"` genome is resolved
and embedded into a copy of mock/report.html. Commits + drift are illustrative
(domain-appropriate), the way the real tool would derive them from git + a scan.
"""
import tomllib, json, re
from pathlib import Path

root = Path(__file__).parent
g = tomllib.load(open(root / "examples/strategy-doc/genome.toml", "rb"))

arch = g["architecture"]
layers = {name: {**L, "_ov": [], "_src": "local"} for name, L in arch["layers"].items()}
architecture = {k: v for k, v in arch.items() if k != "layers"}
architecture["layers"] = layers

proj = dict(g["project"]); proj["intent"] = " ".join(proj["intent"].split())

embed = {
    "project": proj, "architecture": architecture,
    "policies": {}, "policy_includes": [],
    "components": g["components"], "links": g["links"],
    "lifecycle": g.get("lifecycle", []),
    "commits": [
        {"hash": "7c1a3f", "date": "2026-05-28", "msg": "Draft executive summary", "components": ["exec_summary"], "fns": []},
        {"hash": "9b2f0d", "date": "2026-05-30", "msg": "Finalize market sizing with sources", "components": ["market_sizing"], "fns": []},
        {"hash": "a4d3e1", "date": "2026-06-01", "msg": "Competitive landscape (in review)", "components": ["competition"], "fns": []},
        {"hash": "e6c8b2", "date": "2026-06-02", "msg": "Positioning + wedge draft", "components": ["positioning"], "fns": []},
    ],
    "drift": {"pr": 17, "rows": [
        {"sev": "bad",  "ic": "⛔", "msg": "Claim without a source", "sub": "“~15% CAGR” in Market sizing has no source link", "tag": "unsourced"},
        {"sev": "warn", "ic": "❓", "msg": "Reference points to a missing section", "sub": "Pricing references a “discount policy” that is not in the outline", "tag": "dangling reference"},
        {"sev": "ok",   "ic": "○", "msg": "4 sections still in draft or planned", "sub": "pricing · channels · timeline · risks — not yet final", "tag": "unfinished"},
    ]},
}

js = json.dumps(embed, ensure_ascii=False, separators=(",", ":"))
assert "\n" not in js, "embed must not contain newlines"

tmpl = (root / "mock/report.html").read_text()          # same renderer as nexovia
out, n = re.subn(r"const GENOME = \{.*?\};\n", "const GENOME = " + js + ";\n", tmpl, count=1, flags=re.S)
assert n == 1, f"GENOME literal not replaced (n={n})"
(root / "examples/strategy-doc/report.html").write_text(out)

done = sum(1 for c in g["components"].values() if c.get("status") in ("final", "live", "done", "shipped", "published"))
print(f"rendered document example: {len(g['components'])} sections, {len(g['links'])} references, "
      f"{done} final → built {round(done/len(g['components'])*100)}% ({len(out)} chars)")
