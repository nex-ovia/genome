// nexovia library crate — the modules the `genome` binary and tests share.
pub mod cli;
#[cfg(feature = "enrich")]
pub mod enrich;
pub mod facet;
pub mod genome;
pub mod ingest;
pub mod render;
pub mod validate;
