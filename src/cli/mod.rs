use color_eyre::eyre::Result;

mod run;
mod serve;
mod status;
mod gen;

#[derive(clap::Parser, Debug)]
#[command(version, author, about)]
pub struct App {
    /// Enable debugging logs.
    #[arg(short, long)]
    pub debug: bool,

    #[command(subcommand)]
    command: Command,

    /// Path for UNIX socket used for communicating with Dolores server
    #[arg(
        long = "socket",
        env = "DOLORES_SOCKET",
        default_value = "/var/run/dolores.sock"
    )]
    socket_path: std::path::PathBuf,
}

impl App {
    pub fn new() -> Self { clap::Parser::parse() }

    pub fn run(self) -> Result<()> {
        tracing::debug!(?self);

        self.command.run(&self.socket_path)
    }
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    Run(run::Command),
    Serve(serve::Command),
    Status(status::Command),
    Gen(gen::Command),
}

impl Command {
    fn run(self, path: &std::path::Path) -> Result<()> {
        match self {
            Command::Run(cmd) => cmd.run(path),
            Command::Serve(cmd) => cmd.run(path),
            Command::Status(cmd) => cmd.run(path),
            Command::Gen(cmd) => cmd.run(),
        }
    }
}
