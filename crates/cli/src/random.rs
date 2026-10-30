//! Fresh randomness for keys, salts and IVs, straight from the OS.

pub fn bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).expect("the OS random number generator is unavailable");
    buf
}
