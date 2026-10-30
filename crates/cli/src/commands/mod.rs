pub mod init;
pub mod reseal;
pub mod seal;
pub mod unseal_source;

pub fn not_yet(name: &str) -> anyhow::Result<()> {
    anyhow::bail!("`advent {name}` is not implemented yet")
}
