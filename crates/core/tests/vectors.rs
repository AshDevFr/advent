//! Checks advent-core against the shared `test-vectors.json`, which the
//! TypeScript frontend is held to as well.

use serde::Deserialize;

#[derive(Deserialize)]
struct Vectors {
    normalize: Vec<NormalizeCase>,
}

#[derive(Deserialize)]
struct NormalizeCase {
    input: String,
    normalized: String,
    note: String,
}

fn vectors() -> Vectors {
    let raw = include_str!("../../../test-vectors.json");
    serde_json::from_str(raw).expect("test-vectors.json parses")
}

#[test]
fn normalize_vectors() {
    let cases = vectors().normalize;
    assert!(cases.len() >= 10, "need at least 10 normalization cases");
    for case in cases {
        assert_eq!(
            advent_core::normalize(&case.input),
            case.normalized,
            "input {:?} ({})",
            case.input,
            case.note
        );
    }
}
