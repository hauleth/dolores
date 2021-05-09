use structopt::StructOpt;

mod run;
mod server;

#[derive(structopt::StructOpt, Debug)]
pub struct App {
    #[structopt(short, long)]
    debug: bool,

    #[structopt(subcommand)]
    command: Command,
}

impl App {
    pub fn run() -> anyhow::Result<()> {
        let opts = Self::from_args();
        println!("{:?}", opts);

        opts.command.run()
    }
}

#[derive(structopt::StructOpt, Debug)]
enum Command {
    Run(run::Command),
    Server(server::Command),
}

impl Command {
    fn run(self) -> anyhow::Result<()> {
        match self {
            Command::Run(cmd) => cmd.run(),
            Command::Server(cmd) => cmd.run(),
        }
    }
}
