//! Smoke test: synthesize a tiny .graph in memory, write it to a temp file,
//! open it with mc::Graph, verify round-trip of frontmatter + cyberlink records.

use std::io::Write;

use mc::graph::record::RECORD_SIZE;
use mc::Graph;

fn synth_record(neuron: u8, from: u8, to: u8, amount: u128, valence: i8, block: u64) -> [u8; RECORD_SIZE] {
    let mut r = [0u8; RECORD_SIZE];
    r[0..32].fill(neuron);
    r[32..64].fill(from);
    r[64..96].fill(to);
    r[96..100].copy_from_slice(&1u32.to_le_bytes());
    r[100..116].copy_from_slice(&amount.to_le_bytes());
    r[116] = valence as u8;
    r[117..125].copy_from_slice(&block.to_le_bytes());
    r
}

#[test]
fn round_trip_minimal_graph() {
    let r1 = synth_record(0xAA, 0x01, 0x02, 1_000_000, 1, 100);
    let r2 = synth_record(0xBB, 0x02, 0x03, 2_000_000, 0, 101);
    let cyberlinks_size = (RECORD_SIZE * 2) as u64;

    let frontmatter = format!(
        "[cyb]\n\
         types = [\"graph\"]\n\
         name = \"smoke-test\"\n\
         \n\
         [[files]]\n\
         name = \"config\"\n\
         format = \"toml\"\n\
         \n\
         [[files]]\n\
         name = \"cyberlinks\"\n\
         format = \"records\"\n\
         size = {cyberlinks_size}\n"
    );
    let config = "chain_id = \"smoke\"\nblock = 1\n";

    let tmp = std::env::temp_dir().join("mc-smoke.graph");
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(frontmatter.as_bytes()).unwrap();
    write!(f, "~~~config\n{config}").unwrap();
    f.write_all(b"~~~cyberlinks\n").unwrap();
    f.write_all(&r1).unwrap();
    f.write_all(&r2).unwrap();
    drop(f);

    let g = Graph::open(&tmp).unwrap();
    assert_eq!(g.name(), "smoke-test");
    assert!(g.config_raw().unwrap().contains("chain_id"));

    let links: Vec<_> = g.cyberlinks().unwrap().collect();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].amount, 1_000_000);
    assert_eq!(links[0].valence, 1);
    assert_eq!(links[0].block, 100);
    assert_eq!(links[1].amount, 2_000_000);
    assert_eq!(links[1].valence, 0);
    assert_eq!(links[1].block, 101);

    std::fs::remove_file(&tmp).ok();
}
