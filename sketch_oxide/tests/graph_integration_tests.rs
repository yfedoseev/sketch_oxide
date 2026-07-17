//! Integration tests for the `graph` module: every graph sketch exercised
//! through its public API from outside the crate, plus `Serializable`
//! round-trips for each type (fable5 W6.1 — the graph module previously had
//! inline tests only and unproven serialization).

use sketch_oxide::common::Serializable;
use sketch_oxide::graph::{
    AgmConnectivity, Doulion, Fleet, GssSketch, HyperAnf, Mascot, TcmSketch, ThinkD, Triest,
};

/// Undirected path 0-1-...-(n-1).
fn path_edges(n: u64) -> Vec<(u64, u64)> {
    (0..n - 1).map(|i| (i, i + 1)).collect()
}

/// All edges of a clique on `n` vertices.
fn clique_edges(n: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for u in 0..n {
        for v in (u + 1)..n {
            out.push((u, v));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// HyperANF
// ---------------------------------------------------------------------------

#[test]
fn hyperanf_round_trip_preserves_estimates() {
    let mut anf = HyperAnf::new(12).unwrap();
    for (u, v) in path_edges(80) {
        anf.add_edge(u, v);
    }
    let bytes = anf.to_bytes().unwrap();
    let restored = HyperAnf::from_bytes(&bytes).unwrap();

    assert_eq!(restored.num_vertices(), anf.num_vertices());
    let nf_a = anf.neighborhood_function(4);
    let nf_b = restored.neighborhood_function(4);
    for (a, b) in nf_a.iter().zip(&nf_b) {
        assert!((a - b).abs() < 1e-6 * a.abs().max(1.0), "{a} vs {b}");
    }
    let ball_a = anf.ball_size(40, 5);
    let ball_b = restored.ball_size(40, 5);
    assert!((ball_a - ball_b).abs() < 1e-9, "{ball_a} vs {ball_b}");
}

#[test]
fn hyperanf_empty_round_trip() {
    let anf = HyperAnf::new(10).unwrap();
    let restored = HyperAnf::from_bytes(&anf.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.num_vertices(), 0);
}

#[test]
fn hyperanf_rejects_truncated_and_foreign_bytes() {
    let mut anf = HyperAnf::new(12).unwrap();
    anf.add_edge(1, 2);
    let bytes = anf.to_bytes().unwrap();
    assert!(HyperAnf::from_bytes(&bytes[..bytes.len() - 3]).is_err());
    assert!(HyperAnf::from_bytes(&[]).is_err());
    // Bytes framed for a different sketch must be rejected.
    let other = TcmSketch::new(2, 8).unwrap().to_bytes().unwrap();
    assert!(HyperAnf::from_bytes(&other).is_err());
}

// ---------------------------------------------------------------------------
// TRIEST
// ---------------------------------------------------------------------------

#[test]
fn triest_round_trip_preserves_state() {
    let mut t = Triest::with_seed(20, 42).unwrap();
    for (u, v) in clique_edges(8) {
        t.add_edge(u, v);
    }
    let restored = Triest::from_bytes(&t.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.edges_seen(), t.edges_seen());
    assert_eq!(restored.sample_size(), t.sample_size());
    assert!((restored.estimate() - t.estimate()).abs() < 1e-12);
}

#[test]
fn triest_restored_sketch_keeps_estimating() {
    // Exact regime: reservoir holds everything, so estimates stay exact after
    // a serialize/deserialize cycle even as more edges stream in.
    let mut t = Triest::with_seed(100, 7).unwrap();
    t.add_edge(0, 1);
    t.add_edge(1, 2);
    let mut restored = Triest::from_bytes(&t.to_bytes().unwrap()).unwrap();
    restored.add_edge(0, 2); // closes the triangle
    assert!((restored.estimate() - 1.0).abs() < 1e-9);
}

#[test]
fn triest_rejects_malformed_bytes() {
    assert!(Triest::from_bytes(&[]).is_err());
    let mut t = Triest::with_seed(10, 1).unwrap();
    t.add_edge(0, 1);
    let bytes = t.to_bytes().unwrap();
    assert!(Triest::from_bytes(&bytes[..bytes.len() - 1]).is_err());
}

// ---------------------------------------------------------------------------
// DOULION
// ---------------------------------------------------------------------------

#[test]
fn doulion_round_trip_preserves_estimate() {
    let mut d = Doulion::new(0.7, 12345).unwrap();
    for (u, v) in clique_edges(20) {
        d.add_edge(u, v);
    }
    let restored = Doulion::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.kept_edges(), d.kept_edges());
    assert!((restored.estimate_triangles() - d.estimate_triangles()).abs() < 1e-9);
    assert!((restored.keep_prob() - d.keep_prob()).abs() < 1e-12);
}

#[test]
fn doulion_round_trip_exact_mode() {
    let mut d = Doulion::new(1.0, 1).unwrap();
    for (u, v) in clique_edges(10) {
        d.add_edge(u, v);
    }
    let restored = Doulion::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.estimate_triangles(), 120.0); // C(10,3)
}

// ---------------------------------------------------------------------------
// MASCOT
// ---------------------------------------------------------------------------

#[test]
fn mascot_round_trip_preserves_estimate() {
    let mut m = Mascot::with_seed(0.8, 99).unwrap();
    for (u, v) in clique_edges(12) {
        m.add_edge(u, v);
    }
    let restored = Mascot::from_bytes(&m.to_bytes().unwrap()).unwrap();
    assert!((restored.estimate() - m.estimate()).abs() < 1e-9);
    assert_eq!(restored.sampled_edges(), m.sampled_edges());
}

// ---------------------------------------------------------------------------
// ThinkD
// ---------------------------------------------------------------------------

#[test]
fn thinkd_round_trip_preserves_global_and_local_counts() {
    let mut t = ThinkD::with_seed(0.9, 5).unwrap();
    for (u, v) in clique_edges(10) {
        t.add_edge(u, v);
    }
    let restored = ThinkD::from_bytes(&t.to_bytes().unwrap()).unwrap();
    assert!((restored.global_count() - t.global_count()).abs() < 1e-9);
    for node in 0..10u64 {
        assert!(
            (restored.local_count(node) - t.local_count(node)).abs() < 1e-9,
            "local count of {node} drifted"
        );
    }
    assert_eq!(restored.sampled_edges(), t.sampled_edges());
}

#[test]
fn thinkd_restored_sketch_supports_deletion() {
    let mut t = ThinkD::with_seed(1.0, 5).unwrap(); // r = 1: exact
    for &(u, v) in &[(0, 1), (1, 2), (0, 2), (1, 3), (2, 3)] {
        t.add_edge(u, v);
    }
    let mut restored = ThinkD::from_bytes(&t.to_bytes().unwrap()).unwrap();
    restored.remove_edge(1, 2); // destroys both triangles containing (1,2)
    assert!((restored.global_count() - 0.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// FLEET
// ---------------------------------------------------------------------------

#[test]
fn fleet_round_trip_preserves_butterfly_estimate() {
    let mut f = Fleet::new(64, 0.5, 7).unwrap();
    for l in 0..4u64 {
        for r in 100..104u64 {
            f.add_edge(l, r);
        }
    }
    let restored = Fleet::from_bytes(&f.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.reservoir_size(), f.reservoir_size());
    assert!((restored.estimate() - f.estimate()).abs() < 1e-9);
    assert!((restored.sampling_probability() - f.sampling_probability()).abs() < 1e-12);
    // K(4,4): C(4,2)^2 = 36 butterflies, exact while the reservoir holds all edges.
    assert_eq!(restored.estimate(), 36.0);
}

// ---------------------------------------------------------------------------
// GSS
// ---------------------------------------------------------------------------

#[test]
fn gss_round_trip_preserves_queries() {
    let mut g = GssSketch::new(64, 2).unwrap();
    g.add_edge(b"alice", b"bob", 3);
    g.add_edge(b"alice", b"carol", 2);
    g.add_edge(b"dave", b"bob", 7);
    // Force some overflow into the buffer with many edges through a small matrix.
    for i in 0..500u64 {
        g.add_edge(&i.to_le_bytes(), &(i + 1).to_le_bytes(), 1);
    }
    let restored = GssSketch::from_bytes(&g.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.side(), g.side());
    assert_eq!(restored.room(), g.room());
    assert_eq!(restored.buffer_len(), g.buffer_len());
    assert_eq!(
        restored.edge_weight(b"alice", b"bob"),
        g.edge_weight(b"alice", b"bob")
    );
    assert_eq!(restored.out_degree(b"alice"), g.out_degree(b"alice"));
    assert_eq!(restored.in_degree(b"bob"), g.in_degree(b"bob"));
    for i in 0..500u64 {
        assert_eq!(
            restored.edge_weight(&i.to_le_bytes(), &(i + 1).to_le_bytes()),
            g.edge_weight(&i.to_le_bytes(), &(i + 1).to_le_bytes()),
            "edge {i} drifted"
        );
    }
}

#[test]
fn gss_rejects_malformed_bytes() {
    let g = GssSketch::new(8, 2).unwrap();
    let bytes = g.to_bytes().unwrap();
    assert!(GssSketch::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert!(GssSketch::from_bytes(&[]).is_err());
}

// ---------------------------------------------------------------------------
// TCM
// ---------------------------------------------------------------------------

#[test]
fn tcm_round_trip_via_serializable() {
    let mut g = TcmSketch::new(4, 64).unwrap();
    g.add_edge(b"a", b"b", 9);
    g.add_edge(b"c", b"d", 4);
    let restored = TcmSketch::from_bytes(&g.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.edge_weight(b"a", b"b"), g.edge_weight(b"a", b"b"));
    assert_eq!(restored.out_degree(b"a"), g.out_degree(b"a"));
    assert_eq!(restored.in_degree(b"d"), g.in_degree(b"d"));
}

// ---------------------------------------------------------------------------
// AGM connectivity
// ---------------------------------------------------------------------------

#[test]
fn agm_round_trip_preserves_components() {
    let mut g = AgmConnectivity::new(10, 1).unwrap();
    // Two components: {0..4} path, {5..9} path.
    for i in 0..4 {
        g.add_edge(i, i + 1);
    }
    for i in 5..9 {
        g.add_edge(i, i + 1);
    }
    let restored = AgmConnectivity::from_bytes(&g.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.num_components(), g.num_components());
    assert_eq!(restored.num_components(), 2);
    assert!(restored.connected(0, 4));
    assert!(restored.connected(5, 9));
    assert!(!restored.connected(0, 5));
}

#[test]
fn agm_rejects_malformed_bytes() {
    let g = AgmConnectivity::new(4, 1).unwrap();
    let bytes = g.to_bytes().unwrap();
    assert!(AgmConnectivity::from_bytes(&bytes[..bytes.len() - 5]).is_err());
    assert!(AgmConnectivity::from_bytes(&[]).is_err());
}

// ---------------------------------------------------------------------------
// Cross-algorithm integration: one edge stream feeding several sketches
// ---------------------------------------------------------------------------

#[test]
fn one_stream_many_graph_sketches() {
    // A clique K10 (45 edges, 120 triangles, diameter 1) streamed once into
    // four different summaries; each answers its own question sensibly.
    let edges = clique_edges(10);

    let mut anf = HyperAnf::new(12).unwrap();
    let mut triest = Triest::with_seed(200, 3).unwrap();
    let mut doulion = Doulion::new(1.0, 3).unwrap();
    let mut agm = AgmConnectivity::new(10, 3).unwrap();

    for &(u, v) in &edges {
        anf.add_edge(u, v);
        triest.add_edge(u, v);
        doulion.add_edge(u, v);
        agm.add_edge(u as usize, v as usize);
    }

    // Everyone reaches everyone within 1 hop: N(1) ~ 100 ordered pairs.
    let nf = anf.neighborhood_function(1);
    assert!((nf[1] - 100.0).abs() < 10.0, "N(1) = {}", nf[1]);
    // Reservoir 200 > 45 edges: exact triangle count.
    assert!((triest.estimate() - 120.0).abs() < 1e-9);
    // p = 1: exact.
    assert_eq!(doulion.estimate_triangles(), 120.0);
    // One connected component.
    assert_eq!(agm.num_components(), 1);
}
