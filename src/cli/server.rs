/// Start master process listening for connections
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    #[structopt(short, long, default_value = "localhost")]
    domain: String,
}

impl Command {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        Ok(())
    }
}
