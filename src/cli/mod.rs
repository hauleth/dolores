use structopt::StructOpt;

use std::sync::Mutex;

use slog::Drain;

mod run;
mod serve;
mod status;

#[derive(structopt::StructOpt, Debug)]
pub struct App {
    #[structopt(short, long)]
    debug: bool,

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
    pub fn run() -> anyhow::Result<()> {
        let opts = Self::from_args();
        println!("{:?}", opts);

        let drain = slog_term::term_full();
        let drain = Mutex::new(drain);
        // let drain = if opts.debug {
        //     slog::LevelFilter::new(drain, slog::Level::Debug)
        // } else {
        //     slog::LevelFilter::new(drain, slog::Level::Info)
        // };
        let root = slog::Logger::root(drain.fuse(), o! {});

        opts.command.run(&opts.socket_path, &root)
    }
}

#[derive(structopt::StructOpt, Debug)]
enum Command {
    Run(run::Command),
    Serve(serve::Command),
    Status(status::Command),
}

impl Command {
    fn run(self, path: &std::path::Path, logger: &slog::Logger) -> anyhow::Result<()> {
        match self {
            Command::Run(cmd) => cmd.run(path, logger),
            Command::Serve(cmd) => cmd.run(path, logger),
            Command::Status(cmd) => cmd.run(path, logger),
        }
    }
}
