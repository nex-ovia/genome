// Persistence (optional, feature `enrich`): the only place hybrid intelligence
// lives. An embedded GGUF "flash" model fills empty semantic fields fully offline,
// stamping each by=llm — kept off the default path so the core stays small.
pub mod llm;
