// Reads genome TOML fragments. Parsing is order-preserving (toml `preserve_order`),
// and we convert into an order-preserving serde_json::Value so the resolved embed
// serializes byte-stably — the render-fidelity contract depends on this.
use serde_json::Value as J;
use std::path::Path;

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// Load a TOML file at `path` into an order-preserving JSON value.
pub fn load_path(path: &Path) -> R<J> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("reading {}: {e}", path.display()))?;
    let tv: toml::Value = toml::from_str(&text)
        .map_err(|e| format!("parsing {}: {e}", path.display()))?;
    Ok(toml_to_json(&tv))
}

/// Load a fragment referenced relative to the genome root (e.g. `org/foo.toml`).
pub fn load_rel(root: &Path, rel: &str) -> R<J> {
    load_path(&root.join(rel))
}

/// Serialize a genome value to TOML text (stable key order; nulls dropped).
/// The inverse of `load_*` — used when the tool writes a genome (e.g. from-doc).
pub fn to_toml_string(v: &J) -> R<String> {
    Ok(toml::to_string(&json_to_toml(v))?)
}

fn json_to_toml(v: &J) -> toml::Value {
    match v {
        J::Null => toml::Value::String(String::new()),
        J::Bool(b) => toml::Value::Boolean(*b),
        J::Number(n) => n
            .as_i64()
            .map(toml::Value::Integer)
            .unwrap_or_else(|| toml::Value::Float(n.as_f64().unwrap_or(0.0))),
        J::String(s) => toml::Value::String(s.clone()),
        J::Array(a) => toml::Value::Array(a.iter().map(json_to_toml).collect()),
        J::Object(o) => {
            let mut t = toml::value::Table::new();
            for (k, val) in o {
                if !val.is_null() {
                    t.insert(k.clone(), json_to_toml(val));
                }
            }
            toml::Value::Table(t)
        }
    }
}

/// Faithful TOML→JSON, preserving key order and the int/float distinction.
fn toml_to_json(v: &toml::Value) -> J {
    match v {
        toml::Value::String(s) => J::String(s.clone()),
        toml::Value::Integer(i) => J::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(J::Number)
            .unwrap_or(J::Null),
        toml::Value::Boolean(b) => J::Bool(*b),
        toml::Value::Datetime(dt) => J::String(dt.to_string()),
        toml::Value::Array(a) => J::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            J::Object(t.iter().map(|(k, v)| (k.clone(), toml_to_json(v))).collect())
        }
    }
}
