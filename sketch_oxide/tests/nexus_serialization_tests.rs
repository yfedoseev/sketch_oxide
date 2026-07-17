//! Serialization round-trip coverage for every sketch the nexus_oxide
//! workspace consumes (fable5 W6.1): MinHash, HyperLogLog/UltraLogLog, KLL,
//! SpaceSaving heavy hitters, Bloom/BinaryFuse, and RaBitQ. Graph-module
//! round-trips live in `graph_integration_tests.rs`.

use sketch_oxide::cardinality::{HyperLogLog, UltraLogLog};
use sketch_oxide::common::{Serializable, Sketch};
use sketch_oxide::frequency::SpaceSaving;
use sketch_oxide::membership::{BinaryFuseFilter, BloomFilter};
use sketch_oxide::quantiles::KllSketch;
use sketch_oxide::similarity::MinHash;
use sketch_oxide::vector::{RaBitQ, RaBitQCode};

#[test]
fn minhash_round_trip() {
    let mut mh = MinHash::new(128).unwrap();
    for i in 0..1000u64 {
        mh.update(&i);
    }
    let restored = MinHash::from_bytes(&mh.to_bytes().unwrap()).unwrap();
    assert!((restored.jaccard_similarity(&mh).unwrap() - 1.0).abs() < 1e-12);
}

#[test]
fn hyperloglog_round_trip() {
    let mut hll = HyperLogLog::new(12).unwrap();
    for i in 0..10_000u64 {
        hll.update(&i);
    }
    let restored = HyperLogLog::from_bytes(&hll.to_bytes()).unwrap();
    assert_eq!(restored.estimate(), hll.estimate());
}

#[test]
fn ultraloglog_round_trip() {
    let mut ull = UltraLogLog::new(10).unwrap();
    for i in 0..5_000u64 {
        ull.update(&i);
    }
    let restored = UltraLogLog::from_bytes(&ull.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.estimate(), ull.estimate());
}

#[test]
fn kll_round_trip() {
    let mut kll = KllSketch::new(200).unwrap();
    for i in 0..10_000 {
        kll.update(i as f64);
    }
    // Explicit inherent call: the `Serializable` trait's `&self` method would
    // otherwise shadow the `&mut self` inherent one in method-call syntax.
    let bytes = KllSketch::to_bytes(&mut kll);
    let mut restored = KllSketch::from_bytes(&bytes).unwrap();
    for q in [0.1, 0.5, 0.9, 0.99] {
        assert_eq!(restored.quantile(q).unwrap(), kll.quantile(q).unwrap());
    }
}

#[test]
fn space_saving_u64_round_trip_with_populated_counters() {
    let mut ss: SpaceSaving<u64> = SpaceSaving::new(0.05).unwrap();
    for i in 0..1000u64 {
        // Skewed stream: item 7 is a heavy hitter.
        ss.update(if i % 3 == 0 { 7 } else { i });
    }
    let bytes = ss.to_bytes().expect("populated u64 counters must encode");
    let restored: SpaceSaving<u64> = SpaceSaving::from_bytes(&bytes).unwrap();
    assert_eq!(restored.num_items(), ss.num_items());
    assert_eq!(restored.stream_length(), ss.stream_length());
    assert_eq!(restored.estimate(&7), ss.estimate(&7));
    let mut a = restored.heavy_hitters(0.2);
    let mut b = ss.heavy_hitters(0.2);
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn space_saving_string_round_trip_with_populated_counters() {
    let mut ss: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap();
    for _ in 0..50 {
        ss.update("label:person".to_string());
    }
    ss.update("label:city".to_string());
    let bytes = ss
        .to_bytes()
        .expect("populated String counters must encode");
    let restored: SpaceSaving<String> = SpaceSaving::from_bytes(&bytes).unwrap();
    assert_eq!(
        restored.estimate(&"label:person".to_string()),
        ss.estimate(&"label:person".to_string())
    );
    assert_eq!(restored.num_items(), ss.num_items());
}

#[test]
fn space_saving_rejects_malformed_bytes() {
    let mut ss: SpaceSaving<u64> = SpaceSaving::new(0.1).unwrap();
    ss.update(1);
    let bytes = ss.to_bytes().unwrap();
    assert!(SpaceSaving::<u64>::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert!(SpaceSaving::<u64>::from_bytes(&[]).is_err());
}

#[test]
fn bloom_round_trip() {
    let mut bloom = BloomFilter::new(1000, 0.01);
    for i in 0..500u64 {
        bloom.insert(&i.to_le_bytes());
    }
    let restored = BloomFilter::from_bytes(&bloom.to_bytes()).unwrap();
    for i in 0..500u64 {
        assert!(restored.contains(&i.to_le_bytes()), "lost key {i}");
    }
}

#[test]
fn binary_fuse_round_trip() {
    let keys: Vec<u64> = (0..2000).collect();
    let filter = BinaryFuseFilter::from_items(keys.iter().copied(), 9).unwrap();
    let restored = BinaryFuseFilter::deserialize(&filter.serialize()).unwrap();
    for &k in &keys {
        assert!(restored.contains(&k), "lost key {k}");
    }
}

#[test]
fn rabitq_quantizer_and_code_round_trip() {
    let dim = 64;
    let q = RaBitQ::new(dim, 42).unwrap();
    let v: Vec<f64> = (0..dim).map(|i| ((i * 37 % 19) as f64) - 9.0).collect();
    let query: Vec<f64> = (0..dim).map(|i| ((i * 13 % 23) as f64) - 11.0).collect();
    let code = q.encode(&v).unwrap();

    // Quantizer round-trip: same rotation, so identical codes and estimates.
    let q2 = RaBitQ::from_bytes(&q.to_bytes().unwrap()).unwrap();
    assert_eq!(q2.dim(), dim);
    let code2 = q2.encode(&v).unwrap();
    assert_eq!(code2, code);

    // Code round-trip: identical estimate against the same query.
    let code3 = RaBitQCode::from_bytes(&code.to_bytes().unwrap()).unwrap();
    assert_eq!(code3, code);
    let est_orig = q.estimate_inner_product(&code, &query).unwrap();
    let est_restored = q2.estimate_inner_product(&code3, &query).unwrap();
    assert!((est_orig - est_restored).abs() < 1e-12);
}

#[test]
fn rabitq_rejects_malformed_bytes() {
    let q = RaBitQ::new(16, 7).unwrap();
    let bytes = q.to_bytes().unwrap();
    assert!(RaBitQ::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert!(RaBitQ::from_bytes(&[]).is_err());

    let code = q.encode(&vec![1.0; 16]).unwrap();
    let cb = code.to_bytes().unwrap();
    assert!(RaBitQCode::from_bytes(&cb[..cb.len() - 1]).is_err());
}
