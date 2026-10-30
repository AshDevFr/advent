//! Core of the advent engine: answer normalization, sealing primitives,
//! sealed formats, the content model and the release schedule.
//!
//! This crate must build for `wasm32-unknown-unknown`, so it does no I/O:
//! callers pass bytes in and get bytes out.

pub mod format;
mod guessable;
mod normalize;
pub mod primitives;
pub mod wrap;

pub use guessable::{GuessableReason, guessable};
pub use normalize::{is_normalized_word, normalize};

/// Version of this crate, recorded in sealed files as `sealed_by` by callers.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
