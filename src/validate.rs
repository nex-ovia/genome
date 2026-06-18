// Governance: check a genome against the grammar it declares (schema/nexovia-2.toml
// [rules]). Integrity (no dangling links / unknown enums), honesty (no semantic
// field stamped without provenance), the architecture dependency rule, and that
// question/approval targets resolve. The genome describes its own quality bar;
// this module makes that bar binding.
use crate::genome::io;
use crate::render::html;
use serde_json::Value as J;
use std::collections::HashSet;
use std::fmt;
use std::path::Path;

type R<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Severity {
    Error,
    Warn,
}

#[derive(Debug)]
pub struct Issue {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
}
impl Issue {
    fn error(rule: &str, message: String) -> Self {
        Issue {
            rule: rule.into(),
            severity: Severity::Error,
            message,
        }
    }
    fn warn(rule: &str, message: String) -> Self {
        Issue {
            rule: rule.into(),
            severity: Severity::Warn,
            message,
        }
    }
}
impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.severity {
            Severity::Error => "error",
            Severity::Warn => "warn",
        };
        write!(f, "{tag} [{}] {}", self.rule, self.message)
    }
}

/// Validate the genome at `anchor`. Returns every issue found (empty = clean).
pub fn validate(anchor: &Path) -> R<Vec<Issue>> {
    let root = anchor.parent().unwrap_or_else(|| Path::new("."));
    let embed = html::resolve(anchor)?;
    let mut issues = Vec::new();

    issues.extend(check_links(&embed));
    issues.extend(check_honesty(&embed));
    issues.extend(check_dependency_rule(&embed));
    if let Ok(schema) = io::load_rel(root, "schema/nexovia-2.toml") {
        if let Some(enums) = schema.get("enums") {
            issues.extend(check_enums(&embed, enums));
        }
    }

    // question / approval targets must resolve to a component id or "*".
    let comp_ids: HashSet<String> = component_ids(&embed);
    let genome = io::load_path(anchor)?;
    for (facet, item_key) in [("questions", "question"), ("approvals", "approval")] {
        for rel in includes(&genome, facet) {
            if let Ok(file) = io::load_rel(root, &rel) {
                if let Some(items) = file.get(item_key).and_then(J::as_array) {
                    issues.extend(check_targets(items, item_key, &comp_ids));
                }
            }
        }
    }
    Ok(issues)
}

// --- pure checks (unit-testable on a resolved embed) ------------------------

/// Every link's `from`/`to` must resolve to a known component.
pub fn check_links(embed: &J) -> Vec<Issue> {
    let ids = component_ids(embed);
    let mut out = Vec::new();
    for link in embed
        .get("links")
        .and_then(J::as_array)
        .into_iter()
        .flatten()
    {
        for end in ["from", "to"] {
            if let Some(id) = link.get(end).and_then(J::as_str) {
                if !ids.contains(id) {
                    out.push(Issue::error(
                        "forbid_dangling_links",
                        format!("link {end}=\"{id}\" does not resolve to a component"),
                    ));
                }
            }
        }
    }
    out
}

/// A semantic field that is present must carry a provenance entry — but only
/// where the node already declares provenance (machine authorship is tracked).
pub fn check_honesty(embed: &J) -> Vec<Issue> {
    let mut out = Vec::new();
    for (id, c) in components(embed) {
        let prov = match c.get("provenance").and_then(J::as_object) {
            Some(p) => p,
            None => {
                // No provenance block at all (e.g. a human-authored doc section):
                // not an error, but worth surfacing rather than silently trusting.
                for field in ["summary", "why"] {
                    if c.get(field)
                        .and_then(J::as_str)
                        .is_some_and(|s| !s.is_empty())
                    {
                        out.push(Issue::warn(
                            "require_provenance_on_semantic",
                            format!("component \"{id}\": \"{field}\" present but the node declares no provenance"),
                        ));
                    }
                }
                continue;
            }
        };
        for field in ["summary", "why"] {
            let present = c
                .get(field)
                .and_then(J::as_str)
                .is_some_and(|s| !s.is_empty());
            if present && !prov.contains_key(field) {
                out.push(Issue::error(
                    "require_provenance_on_semantic",
                    format!("component \"{id}\": semantic field \"{field}\" has no provenance"),
                ));
            }
        }
    }
    out
}

/// core-independent: the execution core must not depend outward (on persistence
/// or surface). Only enforced when the genome declares that dependency rule.
pub fn check_dependency_rule(embed: &J) -> Vec<Issue> {
    let arch = embed.get("architecture");
    let rule = arch
        .and_then(|a| a.get("dependency_rule"))
        .and_then(J::as_str);
    if rule != Some("core-independent") {
        return Vec::new();
    }
    let layer_of: std::collections::HashMap<String, String> = components(embed)
        .filter_map(|(id, c)| {
            c.get("layer")
                .and_then(J::as_str)
                .map(|l| (id.to_string(), l.to_string()))
        })
        .collect();
    let mut out = Vec::new();
    for link in embed
        .get("links")
        .and_then(J::as_array)
        .into_iter()
        .flatten()
    {
        let rel = link.get("relation").and_then(J::as_str).unwrap_or("");
        if rel != "depends_on" && rel != "calls" {
            continue;
        }
        let (from, to) = (
            link.get("from").and_then(J::as_str),
            link.get("to").and_then(J::as_str),
        );
        if let (Some(f), Some(t)) = (from, to) {
            let fl = layer_of.get(f).map(String::as_str).unwrap_or("");
            let tl = layer_of.get(t).map(String::as_str).unwrap_or("");
            if fl == "execution" && (tl == "persistence" || tl == "surface") {
                out.push(Issue::error(
                    "enforce_layer_dependency_rule",
                    format!("\"{f}\" (execution) must not {rel} \"{t}\" ({tl}) — core stays independent"),
                ));
            }
        }
    }
    out
}

/// Enumerated fields must use known values (status / relation / by / kind / …).
pub fn check_enums(embed: &J, enums: &J) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut check = |val: Option<&str>, en: &str, ctx: &str| {
        if let Some(v) = val {
            let ok = enums
                .get(en)
                .and_then(J::as_array)
                .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(v)));
            if !ok {
                out.push(Issue::error(
                    "require_known_enums",
                    format!("{ctx}: unknown {en} \"{v}\""),
                ));
            }
        }
    };
    let p = embed.get("project");
    check(
        p.and_then(|p| p.get("domain")).and_then(J::as_str),
        "domain",
        "project.domain",
    );
    check(
        p.and_then(|p| p.get("status")).and_then(J::as_str),
        "status",
        "project.status",
    );
    check(
        p.and_then(|p| p.get("source")).and_then(J::as_str),
        "source",
        "project.source",
    );
    for (id, c) in components(embed) {
        check(
            c.get("status").and_then(J::as_str),
            "status",
            &format!("component \"{id}\""),
        );
    }
    for link in embed
        .get("links")
        .and_then(J::as_array)
        .into_iter()
        .flatten()
    {
        check(
            link.get("relation").and_then(J::as_str),
            "relation",
            "link.relation",
        );
        check(
            link.get("provenance")
                .and_then(|p| p.get("by"))
                .and_then(J::as_str),
            "by",
            "link.provenance.by",
        );
    }
    for item in embed
        .get("lifecycle")
        .and_then(J::as_array)
        .into_iter()
        .flatten()
    {
        check(
            item.get("kind").and_then(J::as_str),
            "item_kind",
            "lifecycle.kind",
        );
        check(
            item.get("status").and_then(J::as_str),
            "status",
            "lifecycle.status",
        );
    }
    out
}

/// A question/approval `target` must be "*" (the snapshot) or a known component.
pub fn check_targets(items: &[J], kind: &str, comp_ids: &HashSet<String>) -> Vec<Issue> {
    let mut out = Vec::new();
    for item in items {
        if let Some(t) = item.get("target").and_then(J::as_str) {
            if t != "*" && !comp_ids.contains(t) {
                let id = item.get("id").and_then(J::as_str).unwrap_or("?");
                out.push(Issue::error(
                    "resolve_facet_targets",
                    format!("{kind} \"{id}\": target \"{t}\" does not resolve to a component"),
                ));
            }
        }
    }
    out
}

// --- helpers ---------------------------------------------------------------
fn components(embed: &J) -> impl Iterator<Item = (&String, &J)> {
    embed
        .get("components")
        .and_then(J::as_object)
        .into_iter()
        .flat_map(|o| o.iter())
}
fn component_ids(embed: &J) -> HashSet<String> {
    components(embed).map(|(k, _)| k.clone()).collect()
}
fn includes(genome: &J, facet: &str) -> Vec<String> {
    genome
        .get("include")
        .and_then(|i| i.get(facet))
        .and_then(J::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flags_dangling_link() {
        let e = json!({"components":{"a":{"label":"A"}},
                       "links":[{"from":"a","to":"ghost","relation":"calls"}]});
        assert!(check_links(&e).iter().any(|i| i.message.contains("ghost")));
    }

    #[test]
    fn flags_unknown_enum() {
        let enums = json!({"status":["planned","active"]});
        let e = json!({"components":{"a":{"label":"A","status":"planed"}}});
        assert!(check_enums(&e, &enums)
            .iter()
            .any(|i| i.message.contains("planed")));
    }

    #[test]
    fn flags_missing_provenance() {
        let e = json!({"components":{"a":{"summary":"x","provenance":{}}}});
        assert!(check_honesty(&e)
            .iter()
            .any(|i| i.rule == "require_provenance_on_semantic"));
    }

    #[test]
    fn flags_core_depends_outward() {
        let e = json!({"architecture":{"dependency_rule":"core-independent"},
            "components":{"core":{"layer":"execution"},"db":{"layer":"persistence"}},
            "links":[{"from":"core","to":"db","relation":"depends_on"}]});
        assert_eq!(check_dependency_rule(&e).len(), 1);
    }

    #[test]
    fn targets_must_resolve() {
        let ids: HashSet<String> = ["a".to_string()].into_iter().collect();
        let items = vec![
            json!({"id":"q1","target":"a"}),
            json!({"id":"q2","target":"ghost"}),
            json!({"id":"q3","target":"*"}),
        ];
        let out = check_targets(&items, "question", &ids);
        assert_eq!(out.len(), 1);
        assert!(out[0].message.contains("ghost"));
    }

    #[test]
    fn real_genome_has_no_errors() {
        let anchor = Path::new(env!("CARGO_MANIFEST_DIR")).join("nexovia.toml");
        let issues = validate(&anchor).expect("validate");
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }
}
