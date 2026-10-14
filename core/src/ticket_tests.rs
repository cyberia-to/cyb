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

/// The SpMV relation `y = A·x` as one CCS: row `r` is
/// `(Σ_{(r,c,w)} w·x_c − y_r) · 1 = 0`, `x` pinned (public), `y` committed.
fn spmv_relation(g: &TicketGraph, x: &[F]) -> (zheng::types::CCSInstance, Vec<F>, Vec<(usize, F)>) {
    use zheng::types::{CCSInstance, SparseMatrix};
    let n = g.n;
    let rows = n.max(2).next_power_of_two();
    let cols = (2 * n + 1).max(64).next_power_of_two();
    let mut a = SparseMatrix::new(rows, cols);
    let mut b = SparseMatrix::new(rows, cols);
    for r in 0..n {
        a.entries[r].push((1 + n + r, -F::ONE));
        b.entries[r].push((0, F::ONE));
    }
    for &(r, c, w) in &g.edges {
        a.entries[r].push((1 + c, w));
    }
    let instance = CCSInstance {
        matrices: vec![a, b, SparseMatrix::new(rows, cols)],
        multisets: vec![vec![0, 1], vec![2]],
        coeffs: vec![F::ONE, -F::ONE],
        num_rows: rows,
        num_cols: cols,
    };
    let y = g.spmv(x).unwrap();
    let mut z = vec![F::ZERO; cols];
    z[0] = F::ONE;
    z[1..=n].copy_from_slice(x);
    z[1 + n..=2 * n].copy_from_slice(&y);
    let pins = std::iter::once((0, F::ONE)).chain((0..n).map(|i| (1 + i, x[i]))).collect();
    (instance, z, pins)
}

/// Would a profile-1 proof (zheng's user-device profile) be worth carrying
/// for the subsidy instead of recomputation? A lower bound on its cost:
/// the SpMV relation alone, `y` committed, `digest(y)` NOT bound in the
/// circuit (binding it is ~2 hemera permutations per element of `y`, which
/// exceeds zheng's 2^20 relation cap above a few hundred nodes). Its
/// verifier still rebuilds the relation from the graph — it needs the
/// graph, as recomputation does. Run with `--ignored --nocapture`;
/// `ZHENG_BACKEND` is not read here (CPU). Recorded in
/// `audit/body-ticket-profile-2026-10.md`.
#[test]
#[ignore = "measurement"]
fn ticket_profile_one_lower_bound() {
    use zheng::execution::succinct::{Whir, committed_vars, device_params, prove_relation, verify_relation};
    for count in [255u64, 10_000] {
        let g = TicketGraph::from_axons(&axons(count));
        let c = committed(&g);
        let x = challenge(42, 1, g.n);
        let reps = 5;
        let t0 = std::time::Instant::now();
        for k in 0..reps {
            assert!(prove(&c, 42, k).is_some());
        }
        let recompute_ms = t0.elapsed().as_secs_f64() * 1e3 / reps as f64;
        let (instance, z, pins) = spmv_relation(&g, &x);
        assert!(instance.is_satisfied_by(&zheng::types::CCSWitness { z: z.clone() }));
        let vars = committed_vars(&instance, &pins).unwrap();
        let params = device_params(vars);
        let mut prove_ms = vec![];
        let mut verify_ms = vec![];
        let mut bytes = 0;
        for _ in 0..reps {
            let t0 = std::time::Instant::now();
            let p = prove_relation::<Whir>(&params, &instance, &z, &pins, b"cyb-ticket-p1-bound").unwrap();
            prove_ms.push(t0.elapsed().as_secs_f64() * 1e3);
            let t0 = std::time::Instant::now();
            verify_relation::<Whir>(&instance, &pins, b"cyb-ticket-p1-bound", &p).unwrap();
            verify_ms.push(t0.elapsed().as_secs_f64() * 1e3);
            bytes = zheng::envelope::succinct_proof_bytes(&p.into()).len();
        }
        prove_ms.sort_by(f64::total_cmp);
        verify_ms.sort_by(f64::total_cmp);
        eprintln!(
            "ticket n={} nnz={}: recompute {recompute_ms:.2} ms, 80 B | profile 1 (relation only, 2^{vars}) \
             prove {:.1} ms, verify {:.1} ms (+ the relation rebuild from the graph), {bytes} B",
            g.n,
            g.edges.len(),
            prove_ms[reps as usize / 2],
            verify_ms[reps as usize / 2],
        );
    }
}
