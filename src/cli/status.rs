/// Run given command and pass sockets to listen on incoming connections
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    #[structopt()]
    name: Option<String>,
}

impl Command {
    pub(crate) fn run(self, path: &std::path::Path, logger: &slog::Logger) -> anyhow::Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let logger = logger.new(o!["command" => "status"]);

        runtime.block_on(async {
            debug!(logger, "Query status");
            let client = crate::registry::Client::open(path)?;
            debug!(logger, "Client started");
            let resp = client
                .call(crate::registry::Command::Status { name: self.name })
                .await?;
            info!(logger, "{:?}", resp);
            Ok(())
        })
    }
}
