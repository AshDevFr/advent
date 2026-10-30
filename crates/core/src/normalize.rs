//! Answer normalization, exactly as the crypto spec defines it: Unicode NFD,
//! strip combining marks, lowercase, keep only `[a-z0-9]`. The browser runs
//! the same steps, and `test-vectors.json` holds both to identical output, so
//! no step may be added or changed here without a schema change.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Normalize an answer or a word.
///
/// Characters with no canonical decomposition to ASCII (`ß`, `æ`, CJK,
/// fullwidth digits) are dropped, so the result can be empty; callers that
/// need a usable answer must reject an empty result.
pub fn normalize(input: &str) -> String {
    input
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .filter(|c| matches!(c, 'a'..='z' | '0'..='9'))
        .collect()
}

/// Whether `word` is already in normalized form and non-empty.
///
/// Words are joined to build the final key, so a word that differs from its
/// normalization (case, accents, spaces) would make the certificate
/// unreachable for every player.
pub fn is_normalized_word(word: &str) -> bool {
    !word.is_empty() && normalize(word) == word
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_accents_punctuation_and_spaces() {
        assert_eq!(normalize("  Café-Crème! "), "cafecreme");
    }

    #[test]
    fn normalize_word_check_accepts_only_normalized_words() {
        assert!(is_normalized_word("light"));
        assert!(is_normalized_word("2014"));
        for bad in ["Light", "light ", "lumière", ""] {
            assert!(!is_normalized_word(bad), "{bad:?} must be rejected");
        }
    }
}
