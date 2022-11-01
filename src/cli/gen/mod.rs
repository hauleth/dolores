use color_eyre::eyre::Result;

mod cert;
mod completion;
mod man;

/// Utilities for generating multiple files useful for working with Dolores
#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Generator
}

#[derive(clap::Subcommand, Debug)]
enum Generator {
    Cert(cert::Command),
    Completion(completion::Command),
    Man(man::Command),
}

impl Command {
    pub(crate) fn run(self) -> Result<()> {
        match self.command {
            Generator::Cert(cmd) => cmd.run(),
            Generator::Completion(cmd) => cmd.run(),
            Generator::Man(cmd) => cmd.run(),
        }
    }
}
