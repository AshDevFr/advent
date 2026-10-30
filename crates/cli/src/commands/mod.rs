pub mod init;

pub fn not_yet(name: &str) -> anyhow::Result<()> {
    anyhow::bail!("`advent {name}` is not implemented yet")
}
