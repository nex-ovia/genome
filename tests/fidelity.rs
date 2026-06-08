// Render fidelity is the Phase-1 acceptance gate: `genome render nexovia.toml`
// must byte-match the hand-built golden mock/report.html.
use std::path::Path;

#[test]
fn render_byte_matches_mock() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = nexovia::render::html::render(&root.join("nexovia.toml")).expect("render");
    let mock = std::fs::read_to_string(root.join("mock/report.html")).expect("read mock");
    assert_eq!(out, mock, "render must byte-match mock/report.html");
}
