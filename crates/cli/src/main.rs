use clap::Parser;

/// Write, seal and publish puzzle advent calendars.
#[derive(Parser)]
#[command(name = "advent", version, about)]
struct Cli {}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
