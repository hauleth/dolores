use color_eyre::eyre::Result;

/// Return status of the registered services
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    #[structopt()]
    name: Option<String>,
}

impl Command {
    pub(crate) fn run(self, path: &std::path::Path) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let span = tracing::span!(tracing::Level::DEBUG, "status");
        let _guard = span.enter();

        runtime.block_on(async {
            tracing::debug!("Query status");
            let client = crate::registry::Client::open(path)?;
            tracing::debug!("Client started");
            let resp = client
                .call(crate::registry::Command::Status { name: self.name })
                .await?;
            tracing::info!(?resp);
            Ok(())
        })
    }
}
