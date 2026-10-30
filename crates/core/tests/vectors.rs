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

mod primitives {
    use advent_core::primitives::{
        FINAL_ITERATIONS, WORD_ITERATIONS, WordLock, open_bundle, open_final, open_hint, open_word,
        seal_bundle, seal_final, seal_hint, seal_word,
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Vectors {
        word: Vec<WordCase>,
        bundle: Vec<KeyedCase>,
        hint: Vec<KeyedCase>,
        #[serde(rename = "final")]
        final_: Vec<FinalCase>,
    }

    #[derive(Deserialize)]
    struct WordCase {
        answer: String,
        word: String,
        salt: String,
        iv: String,
        ct: String,
    }

    #[derive(Deserialize)]
    struct KeyedCase {
        key: String,
        iv: String,
        plaintext: String,
        ct: String,
    }

    #[derive(Deserialize)]
    struct FinalCase {
        words: Vec<String>,
        salt: String,
        iv: String,
        plaintext: String,
        ct: String,
    }

    fn vectors() -> Vectors {
        serde_json::from_str(include_str!("../../../test-vectors.json")).unwrap()
    }

    fn bytes<const N: usize>(b64: &str) -> [u8; N] {
        STANDARD.decode(b64).unwrap().try_into().unwrap()
    }

    #[test]
    fn primitives_word_vectors() {
        let cases = vectors().word;
        assert!(cases.len() >= 3);
        for c in cases {
            let lock = seal_word(&c.answer, &c.word, bytes(&c.salt), bytes(&c.iv)).unwrap();
            assert_eq!(STANDARD.encode(&lock.ct), c.ct, "answer {:?}", c.answer);
            assert_eq!(open_word(&c.answer, &lock).unwrap(), c.word);
        }
    }

    #[test]
    fn primitives_bundle_vectors() {
        let cases = vectors().bundle;
        assert!(cases.len() >= 3);
        for c in cases {
            let ct = seal_bundle(&bytes(&c.key), bytes(&c.iv), c.plaintext.as_bytes());
            assert_eq!(STANDARD.encode(&ct), c.ct);
            let opened = open_bundle(&bytes(&c.key), bytes(&c.iv), &ct).unwrap();
            assert_eq!(opened, c.plaintext.as_bytes());
        }
    }

    #[test]
    fn primitives_hint_vectors() {
        let cases = vectors().hint;
        assert!(cases.len() >= 3);
        for c in cases {
            let ct = seal_hint(&bytes(&c.key), bytes(&c.iv), &c.plaintext);
            assert_eq!(STANDARD.encode(&ct), c.ct);
            assert_eq!(
                open_hint(&bytes(&c.key), bytes(&c.iv), &ct).unwrap(),
                c.plaintext
            );
        }
    }

    #[test]
    fn primitives_final_vectors() {
        let cases = vectors().final_;
        assert!(cases.len() >= 3);
        for c in cases {
            let words: Vec<&str> = c.words.iter().map(String::as_str).collect();
            let ct = seal_final(&words, bytes(&c.salt), bytes(&c.iv), c.plaintext.as_bytes());
            assert_eq!(STANDARD.encode(&ct), c.ct);
            let opened = open_final(&words, bytes(&c.salt), bytes(&c.iv), &ct).unwrap();
            assert_eq!(opened, c.plaintext.as_bytes());
        }
    }

    #[test]
    fn primitives_wrong_answer_fails() {
        let c = &vectors().word[0];
        let lock = WordLock {
            salt: bytes(&c.salt),
            iv: bytes(&c.iv),
            ct: STANDARD.decode(&c.ct).unwrap(),
        };
        assert!(open_word("london", &lock).is_err());
    }

    #[test]
    fn primitives_iteration_counts_match_spec() {
        assert_eq!(WORD_ITERATIONS, 150_000);
        assert_eq!(FINAL_ITERATIONS, 200_000);
    }
}
