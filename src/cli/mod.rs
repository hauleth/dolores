use structopt::StructOpt;
use color_eyre::eyre::Result;

mod run;
mod serve;
mod status;

#[derive(structopt::StructOpt, Debug)]
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
    pub fn new() -> Self { Self::from_args() }

    pub fn run(self) -> Result<()> {
        tracing::debug!(?self);

        self.command.run(&self.socket_path)
    }
}

#[derive(structopt::StructOpt, Debug)]
enum Command {
    Run(run::Command),
    Serve(serve::Command),
    Status(status::Command),
}

impl Command {
    fn run(self, path: &std::path::Path) -> Result<()> {
        match self {
            Command::Run(cmd) => cmd.run(path),
            Command::Serve(cmd) => cmd.run(path),
            Command::Status(cmd) => cmd.run(path),
        }
    }
}
