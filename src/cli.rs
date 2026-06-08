// Argument parsing and subcommand dispatch. Owns no business logic — wires modules.
// (clap arrives when the surface grows; Step 1 needs only `render`.)
use crate::render;
use std::path::PathBuf;

type R = Result<(), Box<dyn std::error::Error>>;

pub fn run(mut args: impl Iterator<Item = String>) -> R {
    match args.next().as_deref() {
        Some("render") => {
            let path: PathBuf = args
                .next()
                .ok_or("usage: genome render <nexovia.toml>")?
                .into();
            let html = render::html::render(&path)?;
            print!("{html}");
            Ok(())
        }
        Some(other) => Err(format!("unknown subcommand: {other}").into()),
        None => Err("usage: genome <render> <nexovia.toml>".into()),
    }
}
