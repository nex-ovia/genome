// Argument parsing and subcommand dispatch. Owns no business logic — wires modules.
// (clap arrives when the surface grows.)
use crate::{facet, ingest, render, validate};
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
        Some("ask") => {
            let (mut pos, mut role, mut anchor) = (Vec::new(), "eng".to_string(), default_anchor());
            while let Some(a) = args.next() {
                match a.as_str() {
                    "--role" => role = need(&mut args, "--role")?,
                    "--anchor" => anchor = need(&mut args, "--anchor")?.into(),
                    _ => pos.push(a),
                }
            }
            if pos.len() < 2 {
                return Err("usage: genome ask <target> <text> [--role exec|arch|eng] [--anchor nexovia.toml]".into());
            }
            let id = facet::ask(&anchor, &pos[0], &role, &pos[1..].join(" "))?;
            eprintln!("asked {id} on {}", pos[0]);
            Ok(())
        }
        Some("approve") => {
            let (mut target, mut role, mut decision, mut note, mut anchor) =
                (None, "arch".to_string(), None, None, default_anchor());
            while let Some(a) = args.next() {
                match a.as_str() {
                    "--role" => role = need(&mut args, "--role")?,
                    "--decision" => decision = Some(need(&mut args, "--decision")?),
                    "--note" => note = Some(need(&mut args, "--note")?),
                    "--anchor" => anchor = need(&mut args, "--anchor")?.into(),
                    _ => {
                        if target.is_none() {
                            target = Some(a);
                        }
                    }
                }
            }
            let target = target
                .ok_or("usage: genome approve <target> --decision <pending|approved|rejected> [--role ...] [--note ...]")?;
            let decision = decision.ok_or("genome approve needs --decision")?;
            facet::approve(&anchor, &target, &role, &decision, note.as_deref())?;
            eprintln!("recorded {decision} on {target}");
            Ok(())
        }
        Some("enrich") => {
            let path: PathBuf = args
                .next()
                .ok_or("usage: genome enrich <genome.toml>")?
                .into();
            #[cfg(feature = "enrich")]
            {
                crate::enrich::llm::enrich(&path)
            }
            #[cfg(not(feature = "enrich"))]
            {
                let _ = path;
                Err("enrich is a build-time feature — rebuild with `--features enrich`".into())
            }
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
                Err(format!(
                    "invalid: {errors} error(s), {} warning(s)",
                    issues.len() - errors
                )
                .into())
            } else {
                eprintln!("valid ({} warning(s))", issues.len());
                Ok(())
            }
        }
        Some(other) => Err(format!("unknown subcommand: {other}").into()),
        None => Err("usage: genome <from-doc|render|validate|ask|approve> …".into()),
    }
}

fn default_anchor() -> PathBuf {
    PathBuf::from("nexovia.toml")
}

fn need<I: Iterator<Item = String>>(
    args: &mut I,
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| format!("{flag} needs a value").into())
}
