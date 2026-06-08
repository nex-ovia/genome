// The control-plane write path: `genome ask` / `genome approve` append a question
// or sign-off to the genome's TOML (the database). Append-only text keeps the
// human-authored headers/comments intact; the HTML is a read-only window on it.
use crate::genome::io;
use crate::render::html;
use serde_json::Value as J;
use std::io::Write;
use std::path::{Path, PathBuf};

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// Append a question targeting `target` (a component id or "*"). Returns its id.
pub fn ask(anchor: &Path, target: &str, role: &str, text: &str) -> R<String> {
    check_target(anchor, target)?;
    let file = facet_file(anchor, "questions")?;
    let id = format!("q{}", count(&file, "[[question]]")? + 1);
    let block = format!(
        "\n[[question]]\nid     = \"{}\"\ntarget = \"{}\"\nrole   = \"{}\"\ntext   = \"{}\"\nstatus = \"open\"\nby     = \"human\"\nat     = \"{}\"\n",
        id, esc(target), esc(role), esc(text), today()
    );
    append(&file, &block)?;
    Ok(id)
}

/// Append an approval/decision targeting `target` (a component id or "*").
pub fn approve(anchor: &Path, target: &str, role: &str, decision: &str, note: Option<&str>) -> R<()> {
    check_target(anchor, target)?;
    let file = facet_file(anchor, "approvals")?;
    let mut block = format!(
        "\n[[approval]]\ntarget   = \"{}\"\nrole     = \"{}\"\ndecision = \"{}\"\n",
        esc(target), esc(role), esc(decision)
    );
    if let Some(n) = note {
        block += &format!("note     = \"{}\"\n", esc(n));
    }
    block += &format!("by       = \"human\"\nat       = \"{}\"\n", today());
    append(&file, &block)
}

// --- helpers ---------------------------------------------------------------

/// Resolve the facet file (first path under [include].<facet>) relative to root.
fn facet_file(anchor: &Path, facet: &str) -> R<PathBuf> {
    let root = anchor.parent().unwrap_or_else(|| Path::new("."));
    let genome = io::load_path(anchor)?;
    let rel = genome
        .get("include")
        .and_then(|i| i.get(facet))
        .and_then(J::as_array)
        .and_then(|a| a.first())
        .and_then(J::as_str)
        .ok_or_else(|| format!("{anchor:?} has no [include].{facet} file to append to"))?;
    Ok(root.join(rel))
}

/// A target must be "*" (the snapshot) or an existing component.
fn check_target(anchor: &Path, target: &str) -> R<()> {
    if target == "*" {
        return Ok(());
    }
    let embed = html::resolve(anchor)?;
    let ok = embed
        .get("components")
        .and_then(J::as_object)
        .is_some_and(|c| c.contains_key(target));
    if ok {
        Ok(())
    } else {
        Err(format!("target \"{target}\" is not a component (use \"*\" for the whole snapshot)").into())
    }
}

fn count(file: &Path, marker: &str) -> R<usize> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    Ok(text.lines().filter(|l| l.trim() == marker).count())
}

fn append(file: &Path, block: &str) -> R<()> {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file)?;
    f.write_all(block.as_bytes())?;
    Ok(())
}

/// Escape a TOML basic-string value; collapse newlines so entries stay one line.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace(['\n', '\r', '\t'], " ")
}

/// Today as YYYY-MM-DD (civil date from the system clock; no deps).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let z = (secs / 86400) as i64 + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}
