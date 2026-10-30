pub mod init;
pub mod seal;

pub fn not_yet(name: &str) -> anyhow::Result<()> {
    anyhow::bail!("`advent {name}` is not implemented yet")
}
