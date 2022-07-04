use color_eyre::eyre::Result;
use clap::Parser;

mod run;
mod serve;
mod status;
mod gen_cert;

#[derive(clap::Parser, Debug)]
pub struct App {
    #[structopt(short, long)]
    pub debug: bool,

    #[structopt(subcommand)]
    command: Command,

    #[structopt(
        long = "socket",
        env = "DOLORES_SOCKET",
        default_value = "/var/run/dolores.sock"
    )]
    socket_path: std::path::PathBuf,
}

impl App {
    pub fn new() -> Self { Parser::parse() }

    pub fn run(self) -> Result<()> {
        tracing::debug!(?self);

        self.command.run(&self.socket_path)
    }
}

#[derive(Parser, Debug)]
enum Command {
    Run(run::Command),
    Serve(serve::Command),
    Status(status::Command),
    GenCert(gen_cert::Command),
}

impl Command {
    fn run(self, path: &std::path::Path) -> Result<()> {
        match self {
            Command::Run(cmd) => cmd.run(path),
            Command::Serve(cmd) => cmd.run(path),
            Command::Status(cmd) => cmd.run(path),
            Command::GenCert(cmd) => cmd.run(),
        }
    }
}
