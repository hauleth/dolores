use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::service::Service;

type Registry = HashMap<String, Service>;

/// Start master process listening for connections
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    /// TLD that will be used for handling the applications
    #[structopt(short, long, default_value = "localhost")]
    domain: String,

    #[structopt(short, long, default_value = "[::1]:443")]
    listen: std::net::SocketAddr,

    #[structopt(long, requires("ca-key"))]
    ca_cert: Option<std::path::PathBuf>,

    #[structopt(long, requires("ca-cert"))]
    ca_key: Option<std::path::PathBuf>,
}

impl Command {
    pub(crate) fn run(self, path: &std::path::Path, logger: &slog::Logger) -> anyhow::Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;
        let logger = logger.new(o!["command" => "serve"]);

        let result = runtime.block_on(self.serve(path, &logger));

        info!(logger, "Shutting down");

        result
    }

    async fn serve(&self, path: &std::path::Path, logger: &slog::Logger) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.listen).await?;
        let registry = crate::registry::Registry::open(path, &self.domain)?;

        let config = Arc::new(rustls::ServerConfig::new(Arc::new(rustls::NoClientAuth)));

        info!(logger, "TCP requests: {}", self.listen);
        info!(logger, "Controller: {:?}", &path);

        loop {
            tokio::select! {
                // Control socket
                _ = registry.handle(&logger) => (),
                // Frontend socket
                Ok((stream, addr)) = listener.accept() => {
                    let logger = logger.new(o!["addr" => addr]);
                    debug!(logger, "New connection");
                    let session = rustls::ServerSession::new(&config);
                    let services = registry.services.clone();

                    let handler = Self::handle_request(services, stream, session, logger);

                    tokio::spawn(handler);
                }
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }

    async fn handle_request(
        services: Arc<RwLock<Registry>>,
        up: TcpStream,
        mut session: rustls::ServerSession,
        logger: slog::Logger,
    ) {
        let mut buf = [0; 1024];
        let services = services.read();
        // Peek into the 1 MiB of the data and try to check if there is SNI information
        let len = up.peek(&mut buf).await.unwrap();
        if let Some(sni) = crate::service::parse_handshake(&mut session, &buf[..len]) {
            let logger = logger.new(o![
                "sni" => sni.clone(),
            ]);

            info!(logger, "Request");

            let service = match services.await.get(&*sni) {
                Some(service) => service.clone(),
                None => {
                    // TODO: Redirect to page for service selection
                    warn!(logger, "Request for unknown service {}", sni);
                    return;
                }
            };

            debug!(logger, "Downstream {}", service.addr);

            let down = TcpStream::connect(service.addr).await.unwrap();

            let proxy = service.proxy.clone();
            proxy.run(up, down, &logger).await.unwrap();
        } else {
            warn!(logger, "Cannot find SNI");
        }
    }
}
