// Document → design-first genome. Markdown headings become nodes (often path-less,
// since the code may not exist yet); the first paragraph under each becomes a
// scan-extracted summary, left for enrichment to turn into plain-language why.
// This is how a spec or an idea enters the system before a line of code is written.
use crate::genome::io;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde_json::{json, Map, Value as J};
use std::path::Path;

type R<T> = Result<T, Box<dyn std::error::Error>>;

struct Section {
    level: HeadingLevel,
    title: String,
    summary: Option<String>,
}

/// Ingest a markdown document into a self-contained genome value.
pub fn from_doc(path: &Path) -> R<J> {
    let text = std::fs::read_to_string(path)?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    Ok(build_genome(stem, sections(&text)))
}

/// Convenience: ingest and serialize to TOML.
pub fn from_doc_to_toml(path: &Path) -> R<String> {
    io::to_toml_string(&from_doc(path)?)
}

/// Walk the markdown, collecting (heading, first-paragraph) pairs in order.
fn sections(text: &str) -> Vec<Section> {
    let (mut out, mut buf) = (Vec::<Section>::new(), String::new());
    let (mut in_head, mut in_para, mut level) = (false, false, HeadingLevel::H1);
    for ev in Parser::new(text) {
        match ev {
            Event::Start(Tag::Heading { level: l, .. }) => {
                in_head = true;
                buf.clear();
                level = l;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_head = false;
                out.push(Section {
                    level,
                    title: buf.trim().to_string(),
                    summary: None,
                });
                buf.clear();
            }
            Event::Start(Tag::Paragraph) => {
                in_para = true;
                buf.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if in_para {
                    if let Some(last) = out.last_mut() {
                        let s = buf.trim();
                        if last.summary.is_none() && !s.is_empty() {
                            last.summary = Some(s.to_string());
                        }
                    }
                }
                in_para = false;
                buf.clear();
            }
            Event::Text(t) | Event::Code(t) => {
                if in_head || in_para {
                    buf.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak if (in_head || in_para) => {
                buf.push(' ');
            }
            _ => {}
        }
    }
    out
}

/// Map sections to a genome: H1 = the project; H2 = layers (when H3s exist) or
/// components (when they don't); H3+ = components under their layer.
fn build_genome(stem: &str, mut secs: Vec<Section>) -> J {
    // The first H1 names the project; its paragraph is the project summary.
    let (name, summary) = match secs.iter().position(|s| s.level == HeadingLevel::H1) {
        Some(i) => {
            let h1 = secs.remove(i);
            (h1.title, h1.summary.unwrap_or_default())
        }
        None => (titlecase(stem), String::new()),
    };

    let has_h3 = secs.iter().any(|s| s.level >= HeadingLevel::H3);
    let mut slugs = SlugSet::default();
    let mut layers = Map::new();
    let mut comps = Map::new();
    let mut order = 0i64;
    let mut cur_layer = String::new();

    if !has_h3 {
        layers.insert(
            "outline".into(),
            json!({ "order": 1, "label": "Outline", "role": "The document's sections." }),
        );
        cur_layer = "outline".into();
    }
    for s in &secs {
        if s.level == HeadingLevel::H2 && has_h3 {
            order += 1;
            let id = slugs.make(&s.title);
            let mut layer = Map::new();
            layer.insert("order".into(), json!(order));
            layer.insert("label".into(), J::String(s.title.clone()));
            if let Some(r) = &s.summary {
                layer.insert("role".into(), J::String(r.clone()));
            }
            cur_layer = id.clone();
            layers.insert(id, J::Object(layer));
        } else {
            comps.insert(slugs.make(&s.title), component(s, &cur_layer));
        }
    }

    let mut project = Map::new();
    project.insert("name".into(), J::String(name));
    project.insert("kind".into(), J::String("document".into()));
    project.insert("domain".into(), J::String("document".into()));
    project.insert(
        "summary".into(),
        J::String(if summary.is_empty() {
            "—".into()
        } else {
            summary
        }),
    );
    project.insert("source".into(), J::String("document".into()));
    project.insert("schema".into(), J::String("nexovia/2".into()));
    project.insert("status".into(), J::String("draft".into()));

    json!({
        "project": J::Object(project),
        "architecture": { "pattern": "outline", "description": "Sections of the document, as an outline.", "layers": J::Object(layers) },
        "components": J::Object(comps),
    })
}

/// One section → one component node. The extracted summary is by=scan (a
/// deterministic lift from the text); `why` is left empty for enrichment.
fn component(s: &Section, layer: &str) -> J {
    let mut c = Map::new();
    c.insert("label".into(), J::String(s.title.clone()));
    c.insert("layer".into(), J::String(layer.to_string()));
    c.insert("status".into(), J::String("draft".into()));
    if let Some(sum) = &s.summary {
        c.insert("summary".into(), J::String(sum.clone()));
        c.insert(
            "provenance".into(),
            json!({ "summary": { "by": "scan", "confidence": 1.0 } }),
        );
    }
    J::Object(c)
}

#[derive(Default)]
struct SlugSet(std::collections::HashSet<String>);
impl SlugSet {
    fn make(&mut self, title: &str) -> String {
        let base: String = title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let base = base.trim_matches('_').replace("__", "_");
        let base = if base.is_empty() {
            "section".into()
        } else {
            base
        };
        let mut id = base.clone();
        let mut n = 2;
        while !self.0.insert(id.clone()) {
            id = format!("{base}_{n}");
            n += 1;
        }
        id
    }
}

fn titlecase(s: &str) -> String {
    s.split(['-', '_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut ch = w.chars();
            ch.next()
                .map(|f| f.to_uppercase().collect::<String>() + ch.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingests_headings_and_summaries() {
        // soft line breaks must not glue words together.
        let md = "# Title\n\nFirst para about\nthe project.\n\n## Problem\n\nIt is\nslow.\n\n## Approach\n\nDo it.\n";
        let g = build_genome("doc", sections(md));
        assert_eq!(g["project"]["name"], "Title");
        assert_eq!(g["project"]["summary"], "First para about the project.");
        let comps = g["components"].as_object().unwrap();
        assert!(comps.contains_key("problem") && comps.contains_key("approach"));
        assert_eq!(comps["problem"]["summary"], "It is slow.");
        assert_eq!(comps["problem"]["provenance"]["summary"]["by"], "scan");
    }

    #[test]
    fn h2_become_layers_when_h3_present() {
        let md = "# T\n\n## Part\n\n### Section\n\nBody text.\n";
        let g = build_genome("doc", sections(md));
        assert!(g["architecture"]["layers"]
            .as_object()
            .unwrap()
            .contains_key("part"));
        assert_eq!(g["components"]["section"]["layer"], "part");
    }
}
