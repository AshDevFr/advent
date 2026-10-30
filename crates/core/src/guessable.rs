//! Warns authors when an answer is easy to brute-force offline. A word lock
//! is only as strong as its answer, so the warning is the only defence a
//! static site has; it never blocks sealing.

use crate::normalize::normalize;

/// Why an answer is considered guessable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessableReason {
    /// Only digits: a year or a PIN is a few thousand tries.
    DigitsOnly,
    /// Shorter than 5 characters after normalization.
    TooShort,
    /// A member of a small, well-known set (yes/no, weekdays, months, ...).
    ClosedSet,
}

const MIN_LENGTH: usize = 5;

/// Small sets a guesser would try first.
const CLOSED_SETS: &[&[&str]] = &[
    &["yes", "no", "true", "false"],
    &[
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ],
    &[
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ],
    &[
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ],
];

/// Reasons `answer` is guessable, empty when none apply. The answer is
/// normalized first, since that is what the word lock is derived from.
pub fn guessable(answer: &str) -> Vec<GuessableReason> {
    let normalized = normalize(answer);
    let mut reasons = Vec::new();
    if !normalized.is_empty() && normalized.bytes().all(|b| b.is_ascii_digit()) {
        reasons.push(GuessableReason::DigitsOnly);
    }
    if normalized.len() < MIN_LENGTH {
        reasons.push(GuessableReason::TooShort);
    }
    if CLOSED_SETS
        .iter()
        .any(|set| set.contains(&normalized.as_str()))
    {
        reasons.push(GuessableReason::ClosedSet);
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use GuessableReason::*;

    #[test]
    fn guessable_year_is_digits_and_short() {
        assert_eq!(guessable("2014"), vec![DigitsOnly, TooShort]);
    }

    #[test]
    fn guessable_closed_set_member() {
        assert!(guessable("Yes!").contains(&ClosedSet));
        assert!(guessable("December").contains(&ClosedSet));
    }

    #[test]
    fn guessable_combined_answer_passes() {
        assert!(guessable("lisbon2014").is_empty());
        assert!(guessable("2014 Lisbon").is_empty());
    }
}
