//! Round-trip the optional `~~~graph` section through
//! `import::cyb_format::write_model_file` → `run::format::read_model_file`.

use std::path::PathBuf;

fn tmp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("import-graph-roundtrip-{}-{name}.model", std::process::id()));
    p
}

#[test]
fn write_and_read_graph_section() {
    let path = tmp_path("present");
    let weights = b"\x01\x02\x03\x04";
    import::cyb_format::write_model_file(
        &path,
        "test",
        "card",
        "model_type = \"llama\"\n",
        "",
        "rs",
        Some("deadbeef"),
        "",
        "",
        "",
        weights,
    )
    .expect("write");

    let model = run::format::read_model_file(&path).expect("read");
    assert_eq!(model.graph.as_deref(), Some(&[0xde, 0xad, 0xbe, 0xef][..]));
    assert_eq!(model.weights, weights);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn write_without_graph_omits_section() {
    let path = tmp_path("absent");
    import::cyb_format::write_model_file(
        &path,
        "test",
        "card",
        "model_type = \"llama\"\n",
        "",
        "rs",
        None,
        "",
        "",
        "",
        b"",
    )
    .expect("write");

    let model = run::format::read_model_file(&path).expect("read");
    assert!(model.graph.is_none());
    let _ = std::fs::remove_file(&path);
}
