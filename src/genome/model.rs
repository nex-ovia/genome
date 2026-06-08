// The canonical in-memory shape of a genome. One schema that every adapter converges
// on. Step 1's renderer resolves at the value level for byte-fidelity; these typed
// views are the contract used by the rest of the tool (validate, ingest, …).
#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub by: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub domain: String,
    pub schema: String,
    pub summary: String,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub order: i64,
    pub label: String,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub label: String,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    #[serde(default)]
    pub statement: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub enforcement: Option<String>,
}
