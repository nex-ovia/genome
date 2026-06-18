//! Offline embedded GGUF enrichment (feature `enrich`). Fills empty `why` fields
//! with an in-process llama.cpp model — fully offline at runtime (the model is
//! fetched once and cached). Greedy decoding for reproducibility. Stamps each
//! result by=llm + confidence and only ever fills blanks, so human/scan fields
//! are never overwritten (provenance stays sticky).
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use std::path::{Path, PathBuf};

type R<T> = Result<T, Box<dyn std::error::Error>>;

const MODEL_FILE: &str = "qwen2.5-0.5b-instruct-q4_k_m.gguf";
const MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf?download=true";
const CONFIDENCE: f64 = 0.6;

/// Fill empty `why` fields in the genome at `anchor`, in place.
pub fn enrich(anchor: &Path) -> R<()> {
    let text = std::fs::read_to_string(anchor)?;
    let mut doc = text.parse::<toml_edit::DocumentMut>()?;
    let targets = empty_why(&doc);
    if targets.is_empty() {
        println!("enrich: nothing to fill — every component already has a why");
        return Ok(());
    }
    let project = doc
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("the project")
        .to_string();

    let model_path = ensure_model()?;
    eprintln!("enrich: loading model {}", model_path.display());
    let backend = LlamaBackend::init()?;
    let model = LlamaModel::load_from_file(&backend, &model_path, &LlamaModelParams::default())?;

    let mut filled = 0;
    for (id, label, summary) in &targets {
        let why = generate(&backend, &model, &project, label, summary)?;
        if why.is_empty() {
            continue;
        }
        set_why(&mut doc, id, &why);
        filled += 1;
        eprintln!("  · {id}: {why}");
    }
    std::fs::write(anchor, doc.to_string())?;
    println!(
        "enrich: filled {filled} why field(s) by=llm in {}",
        anchor.display()
    );
    Ok(())
}

/// Components that have a summary but no why — the blanks enrichment may fill.
fn empty_why(doc: &toml_edit::DocumentMut) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if let Some(comps) = doc.get("components").and_then(|c| c.as_table()) {
        for (id, item) in comps {
            let Some(t) = item.as_table() else { continue };
            let summary = t.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            let why = t.get("why").and_then(|v| v.as_str()).unwrap_or("");
            if !summary.is_empty() && why.is_empty() {
                let label = t.get("label").and_then(|v| v.as_str()).unwrap_or(id);
                out.push((id.to_string(), label.to_string(), summary.to_string()));
            }
        }
    }
    out
}

/// Set `why` + a by=llm provenance entry on a component (the provenance table
/// already exists from ingest, so we only add the `why` key).
fn set_why(doc: &mut toml_edit::DocumentMut, id: &str, why: &str) {
    doc["components"][id]["why"] = toml_edit::value(why);
    let mut prov = toml_edit::InlineTable::new();
    prov.insert("by", "llm".into());
    prov.insert("confidence", toml_edit::Value::from(CONFIDENCE));
    doc["components"][id]["provenance"]["why"] = toml_edit::value(prov);
}

/// One concise sentence, greedily decoded (deterministic) from the GGUF model.
fn generate(
    backend: &LlamaBackend,
    model: &LlamaModel,
    project: &str,
    label: &str,
    summary: &str,
) -> R<String> {
    let prompt = format!(
        "<|im_start|>system\nYou explain why a part of a system matters, in exactly one concise sentence. Start directly with the reason — never restate the project or part name, never add a preamble.<|im_end|>\n<|im_start|>user\nProject: {project}\nPart: {label}\nWhat it is: {summary}\n\nWhy does this part matter? Answer in one sentence, starting with a capital letter.<|im_end|>\n<|im_start|>assistant\n"
    );
    let mut ctx = model.new_context(backend, LlamaContextParams::default())?;
    let tokens = model.str_to_token(&prompt, AddBos::Always)?;
    let mut batch = LlamaBatch::new(2048, 1);
    let last = tokens.len() as i32 - 1;
    for (i, t) in tokens.iter().enumerate() {
        batch.add(*t, i as i32, &[0], i as i32 == last)?;
    }
    ctx.decode(&mut batch)?;

    let mut sampler = LlamaSampler::greedy();
    let mut n_cur = batch.n_tokens();
    let limit = n_cur + 90;
    let mut out = String::new();
    while n_cur < limit {
        let tok = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(tok);
        if model.is_eog_token(tok) {
            break;
        }
        out.push_str(&String::from_utf8_lossy(
            &model.token_to_piece_bytes(tok, 64, false, None)?,
        ));
        if out.contains("<|im_end|>") {
            break;
        }
        batch.clear();
        batch.add(tok, n_cur, &[0], true)?;
        n_cur += 1;
        ctx.decode(&mut batch)?;
    }
    Ok(clean(&out))
}

/// Trim the model output to one clean sentence, dropping any prompt echo
/// (a small flash model sometimes restates the `Project:`/`Part:` context).
fn clean(s: &str) -> String {
    let s = s.split("<|im_end|>").next().unwrap_or(s).trim();
    let mut s = s.split('\n').next().unwrap_or(s).trim();
    for prefix in ["Project:", "Part:", "Why:", "Answer:"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest
                .trim_start_matches(|c: char| c != '.' && !c.is_alphabetic())
                .trim();
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Locate the GGUF model, fetching it once into the cache if absent.
fn ensure_model() -> R<PathBuf> {
    if let Ok(p) = std::env::var("GENOME_MODEL") {
        return Ok(PathBuf::from(p));
    }
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|_| PathBuf::from(".cache"));
    let dir = base.join("genome").join("models");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(MODEL_FILE);
    if path.exists() {
        return Ok(path);
    }
    let url = std::env::var("GENOME_MODEL_URL").unwrap_or_else(|_| MODEL_URL.to_string());
    eprintln!("enrich: fetching model once → {}", path.display());
    let resp = ureq::get(&url).call()?;
    let tmp = dir.join(format!("{MODEL_FILE}.part"));
    let mut reader = resp.into_reader();
    let mut f = std::fs::File::create(&tmp)?;
    std::io::copy(&mut reader, &mut f)?;
    drop(f);
    std::fs::rename(&tmp, &path)?;
    eprintln!(
        "enrich: cached model ({} MB)",
        path.metadata().map(|m| m.len() / 1_048_576).unwrap_or(0)
    );
    Ok(path)
}
