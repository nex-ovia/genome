// Argument parsing and subcommand dispatch. Owns no business logic — wires modules.
// (clap arrives when the surface grows.)
use crate::{ingest, render, validate};
use std::path::PathBuf;

type R = Result<(), Box<dyn std::error::Error>>;

pub fn run(mut args: impl Iterator<Item = String>) -> R {
    match args.next().as_deref() {
        Some("from-doc") => {
            let src: PathBuf = args
                .next()
                .ok_or("usage: genome from-doc <file.md> [out.toml]")?
                .into();
            let toml = ingest::doc::from_doc_to_toml(&src)?;
            match args.next() {
                Some(out) => {
                    std::fs::write(&out, toml)?;
                    eprintln!("wrote {out}");
                }
                None => print!("{toml}"),
            }
            Ok(())
        }
        Some("render") => {
            let path: PathBuf = args
                .next()
                .ok_or("usage: genome render <nexovia.toml>")?
                .into();
            let html = render::html::render(&path)?;
            print!("{html}");
            Ok(())
        }
        Some("validate") => {
            let path: PathBuf = args
                .next()
                .ok_or("usage: genome validate <nexovia.toml>")?
                .into();
            let issues = validate::validate(&path)?;
            let errors = issues
                .iter()
                .filter(|i| i.severity == validate::Severity::Error)
                .count();
            for i in &issues {
                eprintln!("{i}");
            }
            if errors > 0 {
                Err(format!("invalid: {errors} error(s), {} warning(s)", issues.len() - errors).into())
            } else {
                eprintln!("valid ({} warning(s))", issues.len());
                Ok(())
            }
        }
        Some(other) => Err(format!("unknown subcommand: {other}").into()),
        None => Err("usage: genome <from-doc|render|validate> …".into()),
    }
}
