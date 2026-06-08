// Resolve the composed genome and inject it into the offline HTML template.
// This ports build_mock.py: resolve `extends`/`include`, tag provenance (_ov/_src),
// embed the compact JSON the page reads. Output must byte-match mock/report.html.
use crate::genome::io;
use serde_json::{json, Map, Value as J};
use std::path::Path;

type R<T> = Result<T, Box<dyn std::error::Error>>;

const TEMPLATE: &str = include_str!("../../assets/report.template.html");

/// Render a genome anchor (`nexovia.toml`) to the self-contained HTML document.
pub fn render(anchor: &Path) -> R<String> {
    let embed = resolve(anchor)?;
    let js = serde_json::to_string(&embed)?;
    debug_assert!(!js.contains('\n'), "embed must not contain newlines");
    Ok(TEMPLATE.replace("__GENOME_JSON__", &js))
}

/// The resolved view: base ⊕ overrides for architecture + policies, plus all
/// included fragments composed in. Mirrors build_mock.py exactly.
pub fn resolve(anchor: &Path) -> R<J> {
    let root = anchor.parent().unwrap_or(Path::new("."));
    let genome = io::load_path(anchor)?;
    let inc = obj(&genome, "include");

    // --- architecture: composed (extends ⊕ overrides) or self-contained ----
    let arch = genome.get("architecture").cloned().unwrap_or(J::Object(Map::new()));
    let architecture = if arch.get("extends").and_then(J::as_str).is_some() {
        resolve_arch_extends(root, &arch)?
    } else {
        resolve_arch_inline(&arch)
    };

    // --- nodes + edges (inline first, then include.components) --------------
    let mut nodes = Map::new();
    let mut edges: Vec<J> = vec![];
    if let Some(c) = genome.get("components").and_then(J::as_object) {
        for (k, v) in c {
            nodes.insert(k.clone(), v.clone());
        }
    }
    if let Some(l) = genome.get("links").and_then(J::as_array) {
        edges.extend(l.iter().cloned());
    }
    for f in arr(&inc, "components") {
        let d = io::load_rel(root, &f)?;
        if let Some(c) = d.get("components").and_then(J::as_object) {
            for (k, v) in c {
                nodes.insert(k.clone(), v.clone());
            }
        }
        if let Some(l) = d.get("links").and_then(J::as_array) {
            edges.extend(l.iter().cloned());
        }
    }

    // --- policies: shared bases (org/*) under local overrides ---------------
    let (mut base_pol, mut local_pol) = (Map::new(), Map::new());
    for f in arr(&inc, "policies") {
        let d = io::load_rel(root, &f)?;
        let target = if f.starts_with("org/") { &mut base_pol } else { &mut local_pol };
        if let Some(t) = d.as_object() {
            for (k, v) in t {
                if v.is_object() {
                    target.insert(k.clone(), v.clone());
                }
            }
        }
    }
    let mut policies = Map::new();
    for (name, body) in &base_pol {
        let mut m = body.as_object().cloned().unwrap_or_default();
        m.insert("_src".into(), J::String("inherited".into()));
        m.insert("_ov".into(), J::Array(vec![]));
        policies.insert(name.clone(), J::Object(m));
    }
    for (name, body) in &local_pol {
        let body_obj = body.as_object().cloned().unwrap_or_default();
        if let Some(existing) = policies.get(name).and_then(J::as_object).cloned() {
            let ov: Vec<J> = body_obj
                .iter()
                .filter(|(k, v)| existing.get(*k) != Some(v))
                .map(|(k, _)| J::String(k.clone()))
                .collect();
            let mut m = existing;
            for (k, v) in &body_obj {
                m.insert(k.clone(), v.clone());
            }
            m.insert("_src".into(), J::String("override".into()));
            m.insert("_ov".into(), J::Array(ov));
            policies.insert(name.clone(), J::Object(m));
        } else {
            let mut m = body_obj;
            m.insert("_src".into(), J::String("local".into()));
            m.insert("_ov".into(), J::Array(vec![]));
            policies.insert(name.clone(), J::Object(m));
        }
    }

    // --- lifecycle (inline [[lifecycle]] first, then include.lifecycle) -----
    let mut lifecycle: Vec<J> = vec![];
    if let Some(items) = genome.get("lifecycle").and_then(J::as_array) {
        lifecycle.extend(items.iter().cloned());
    }
    for f in arr(&inc, "lifecycle") {
        let d = io::load_rel(root, &f)?;
        if let Some(items) = d.get("item").and_then(J::as_array) {
            lifecycle.extend(items.iter().cloned());
        }
    }

    // --- planning facets: delivery / deployment / quality -------------------
    let mut delivery = J::Object(Map::new());
    if let Some(f) = arr(&inc, "delivery").first() {
        let d = io::load_rel(root, f)?;
        let mut m = d.get("delivery").and_then(J::as_object).cloned().unwrap_or_default();
        m.insert("estimates".into(), d.get("estimate").cloned().unwrap_or(J::Array(vec![])));
        delivery = J::Object(m);
    }
    let mut deployment = J::Object(Map::new());
    if let Some(f) = arr(&inc, "deployment").first() {
        let d = io::load_rel(root, f)?;
        deployment = d.get("deployment").cloned().unwrap_or(J::Object(Map::new()));
    }
    let mut quality = J::Object(Map::new());
    if let Some(f) = arr(&inc, "quality").first() {
        let q = io::load_rel(root, f)?;
        let mut m = q.get("quality").and_then(J::as_object).cloned().unwrap_or_default();
        m.insert("budgets".into(), q.get("budget").cloned().unwrap_or(J::Array(vec![])));
        m.insert("gates".into(), q.get("gate").cloned().unwrap_or(J::Array(vec![])));
        quality = J::Object(m);
    }

    // --- control plane: questions + approvals (TOML is the database) --------
    let mut questions: Vec<J> = vec![];
    for f in arr(&inc, "questions") {
        let d = io::load_rel(root, &f)?;
        if let Some(q) = d.get("question").and_then(J::as_array) {
            questions.extend(q.iter().cloned());
        }
    }
    let mut approvals: Vec<J> = vec![];
    for f in arr(&inc, "approvals") {
        let d = io::load_rel(root, &f)?;
        if let Some(a) = d.get("approval").and_then(J::as_array) {
            approvals.extend(a.iter().cloned());
        }
    }

    // --- assemble the embed (key order is part of the contract) -------------
    let mut project = genome.get("project").and_then(J::as_object).cloned().unwrap_or_default();
    if let Some(J::String(intent)) = project.get("intent") {
        let collapsed = intent.split_whitespace().collect::<Vec<_>>().join(" ");
        project.insert("intent".into(), J::String(collapsed));
    }
    let policy_includes: Vec<J> = arr(&inc, "policies")
        .into_iter()
        .filter(|f| f.starts_with("org/"))
        .map(J::String)
        .collect();

    let mut embed = Map::new();
    embed.insert("project".into(), J::Object(project));
    embed.insert("architecture".into(), architecture);
    embed.insert("policies".into(), J::Object(policies));
    embed.insert("policy_includes".into(), J::Array(policy_includes));
    embed.insert("components".into(), J::Object(nodes));
    embed.insert("links".into(), J::Array(edges));
    embed.insert("lifecycle".into(), J::Array(lifecycle));
    embed.insert("delivery".into(), delivery);
    embed.insert("deployment".into(), deployment);
    embed.insert("quality".into(), quality);
    embed.insert("questions".into(), J::Array(questions));
    embed.insert("approvals".into(), J::Array(approvals));
    Ok(J::Object(embed))
}

// --- architecture resolution -----------------------------------------------
/// Composed architecture: inherit a base topology via `extends`, override only
/// what this genome changes; each layer tagged inherited / override / local.
fn resolve_arch_extends(root: &Path, arch: &J) -> R<J> {
    let base = io::load_rel(root, arch.get("extends").and_then(J::as_str).unwrap_or(""))?;
    let overrides = arch.get("layers").and_then(J::as_object).cloned().unwrap_or_default();
    let mut layers = Map::new();
    for (name, l) in base.get("layers").and_then(J::as_object).cloned().unwrap_or_default() {
        let mut merged = l.as_object().cloned().unwrap_or_default();
        let mut ov: Vec<J> = vec![];
        if let Some(o) = overrides.get(&name).and_then(J::as_object) {
            for (k, v) in o {
                if merged.get(k) != Some(v) {
                    ov.push(J::String(k.clone()));
                }
                merged.insert(k.clone(), v.clone());
            }
        }
        let src = if ov.is_empty() { "inherited" } else { "override" };
        merged.insert("_ov".into(), J::Array(ov));
        merged.insert("_src".into(), J::String(src.into()));
        layers.insert(name, J::Object(merged));
    }
    for (name, l) in &overrides {
        if !layers.contains_key(name) {
            let mut m = l.as_object().cloned().unwrap_or_default();
            let keys: Vec<J> = m.keys().map(|k| J::String(k.clone())).collect();
            m.insert("_ov".into(), J::Array(keys));
            m.insert("_src".into(), J::String("local".into()));
            m.insert("_added".into(), J::Bool(true));
            layers.insert(name.clone(), J::Object(m));
        }
    }
    let mut out = Map::new();
    for k in ["pattern", "basis", "dependency_rule", "description"] {
        out.insert(k.into(), base.get(k).cloned().unwrap_or(J::Null));
    }
    out.insert("extends".into(), arch.get("extends").cloned().unwrap_or(J::Null));
    out.insert("fluid".into(), arch.get("fluid").cloned().unwrap_or(J::Bool(false)));
    out.insert("layers".into(), J::Object(layers));
    Ok(J::Object(out))
}

/// Self-contained architecture (e.g. a document outline): no base, every layer
/// is local. Preserves the genome's own field order, layers last.
fn resolve_arch_inline(arch: &J) -> J {
    let mut layers = Map::new();
    for (name, l) in arch.get("layers").and_then(J::as_object).cloned().unwrap_or_default() {
        let mut m = l.as_object().cloned().unwrap_or_default();
        m.insert("_ov".into(), J::Array(vec![]));
        m.insert("_src".into(), J::String("local".into()));
        layers.insert(name, J::Object(m));
    }
    let mut out = Map::new();
    for (k, v) in arch.as_object().cloned().unwrap_or_default() {
        if k != "layers" {
            out.insert(k, v);
        }
    }
    out.insert("layers".into(), J::Object(layers));
    J::Object(out)
}

// --- small helpers ---------------------------------------------------------
fn obj(v: &J, key: &str) -> J {
    v.get(key).cloned().unwrap_or(json!({}))
}
fn arr(v: &J, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(J::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}
