use super::*;

fn committed(g: &TicketGraph) -> CommittedGraph {
    CommittedGraph::new(g.clone())
}

fn axons(count: u64) -> Vec<([u8; 32], [u8; 32], u64)> {
    (0..count)
        .map(|i| {
            let mut a = [0u8; 32];
            let mut b = [0u8; 32];
            a[..8].copy_from_slice(&splitmix64(i).to_le_bytes());
            b[..8].copy_from_slice(&splitmix64(i * 7 + 1).to_le_bytes());
            (a, b, i + 1)
        })
        .collect()
}

#[test]
fn honest_ticket_verifies_and_is_deterministic() {
    let g = TicketGraph::from_axons(&axons(255));
    assert!(g.n > 100 && g.edges.len() == 255);
    let t = prove(&committed(&g), 0xabc, 7).unwrap();
    assert!(verify(&t, &committed(&g)));
    assert_eq!(prove(&committed(&g), 0xabc, 7), Some(t));
    assert_eq!(t.to_bytes().len(), TICKET_BYTES);
}

#[test]
fn wrong_output_seed_number_or_graph_is_rejected() {
    let g = TicketGraph::from_axons(&axons(64));
    let t = prove(&committed(&g), 1, 2).unwrap();
    let mut bad = t;
    bad.output[0] ^= 1;
    assert!(!verify(&bad, &committed(&g)), "output");
    assert!(!verify(&Ticket { seed: 3, ..t }, &committed(&g)), "seed");
    assert!(
        !verify(&Ticket { number: 3, ..t }, &committed(&g)),
        "number"
    );
    let mut heavier = g.clone();
    heavier.edges[5].2 += F::ONE;
    assert!(!verify(&t, &committed(&heavier)), "weight");
    let mut more = g.clone();
    more.edges.push((0, 1, F::ONE));
    assert!(!verify(&t, &committed(&more)), "extra edge");
    // a ticket honestly made over another graph does not transfer
    let other = prove(&committed(&heavier), 1, 2).unwrap();
    assert!(!verify(
        &Ticket {
            graph: t.graph,
            ..other
        },
        &committed(&g)
    ));
}

#[test]
fn the_output_is_the_spmv_of_the_challenge() {
    // y = A·x by hand on a three-node path: 0 -w1→ 1 -w2→ 2
    let g = TicketGraph {
        n: 3,
        edges: vec![(0, 1, F::new(5)), (1, 2, F::new(7))],
    };
    let x = challenge(9, 4, 3);
    let y = g.spmv(&x).unwrap();
    assert_eq!(y, vec![F::new(5) * x[1], F::new(7) * x[2], F::ZERO]);
    assert_eq!(
        prove(&committed(&g), 9, 4).unwrap().output,
        output_digest(&y)
    );
}

#[test]
fn malformed_graphs_neither_prove_nor_verify() {
    let g = TicketGraph::from_axons(&axons(8));
    let t = prove(&committed(&g), 0, 0).unwrap();
    let mut out_of_range = g.clone();
    out_of_range.edges[0].0 = g.n;
    assert!(prove(&committed(&out_of_range), 0, 0).is_none());
    assert!(!verify(&t, &committed(&out_of_range)));
    let empty = TicketGraph {
        n: 0,
        edges: vec![],
    };
    assert!(prove(&committed(&empty), 0, 0).is_none());
}

#[test]
fn empty_graph_falls_back_to_a_ring() {
    let g = TicketGraph::from_axons(&[]);
    assert_eq!((g.n, g.edges.len()), (2, 2));
    let c = committed(&g);
    assert!(verify(&prove(&c, 5, 6).unwrap(), &c));
}

/// Ticket cost by graph size. Run with --nocapture to read the numbers.
#[test]
fn ticket_cost_by_graph_size() {
    for count in [255u64, 10_000, 100_000] {
        let g = TicketGraph::from_axons(&axons(count));
        let t0 = std::time::Instant::now();
        let c = committed(&g);
        let commit_us = t0.elapsed().as_secs_f64() * 1e6;
        let rounds = (200_000 / count).max(3) as u32;
        let t0 = std::time::Instant::now();
        let mut ticket = None;
        for k in 0..rounds {
            ticket = prove(&c, 42, k as u64);
        }
        let prove_us = t0.elapsed().as_secs_f64() * 1e6 / rounds as f64;
        let ticket = ticket.unwrap();
        let t0 = std::time::Instant::now();
        for _ in 0..rounds {
            assert!(verify(&ticket, &c));
        }
        let verify_us = t0.elapsed().as_secs_f64() * 1e6 / rounds as f64;
        eprintln!(
            "ticket n={} nnz={}: {TICKET_BYTES} bytes, graph commitment {commit_us:.1} us once, \
             prove {prove_us:.1} us, verify {verify_us:.1} us",
            g.n,
            g.edges.len(),
        );
    }
}

#[test]
fn about_one_ticket_in_difficulty_counts() {
    let graph = CommittedGraph::new(TicketGraph::from_axons(&[]));
    let total = 170 * 200;
    let counted = (0..total)
        .filter(|&n| counts(&prove(&graph, 7, n).unwrap()))
        .count() as f64;
    let expected = total as f64 / TICKET_DIFFICULTY as f64;
    // 200 expected wins; ±5 sigma
    assert!((counted - expected).abs() < 5.0 * expected.sqrt(), "{counted} vs {expected}");
}
