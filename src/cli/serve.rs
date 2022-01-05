use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use color_eyre::eyre::Result;

use crate::service::Service;

type Registry = HashMap<String, Service>;

/// Start master process listening for connections
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    /// TLD that will be used for handling the applications
    #[structopt(short, long, default_value = "localhost")]
    domain: String,

    #[structopt(short, long, default_value = "0.0.0.0:443")]
    listen: std::net::SocketAddr,

    #[structopt(long, requires("ca-key"))]
    ca_cert: Option<std::path::PathBuf>,

    #[structopt(long, requires("ca-cert"))]
    ca_key: Option<std::path::PathBuf>,
}

impl Command {
    pub(crate) fn run(self, path: &std::path::Path) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;

        let span = tracing::span!(tracing::Level::DEBUG, "serve");
        let _guard = span.enter();

        let result = runtime.block_on(self.serve(path));

        tracing::info!("Shutting down");

        result
    }

    async fn serve(&self, path: &std::path::Path) -> Result<()> {
        let listener = TcpListener::bind(self.listen).await?;
        let registry = crate::registry::Registry::open(path, &self.domain)?;

        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certs = vec![rustls::Certificate(cert.serialize_der().unwrap())];
        let priv_key = rustls::PrivateKey(cert.serialize_private_key_der());

        // let config = Arc::new(rustls::ServerConfig::new(Arc::new(rustls::NoClientAuth)));
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, priv_key)
            .expect("Bad certificate/key");

        let config = Arc::new(config);

        tracing::info!(%self.listen, "TCP request");
        tracing::info!(?path, "Controller");

        loop {
            tokio::select! {
                // Control socket
                _ = registry.handle() => (),
                // Frontend socket
                Ok((stream, addr)) = listener.accept() => {
                    let span = tracing::span!(tracing::Level::DEBUG, "Connection", addr = %addr);
                    let _guard = span.enter();

                    let connection = rustls::ServerConnection::new(config.clone())?;
                    let services = registry.services.clone();

                    let handler = Self::handle_request(services, stream, connection);

                    tokio::spawn(handler);
                }
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }

    async fn handle_request(
        services: Arc<RwLock<Registry>>,
        up: TcpStream,
        mut connection: rustls::ServerConnection,
    ) {
        let mut buf = [0; 1024];
        let services = services.read();
        // Peek into the 1 MiB of the data and try to check if there is SNI information
        let len = up.peek(&mut buf).await.unwrap();
        if let Some(sni) = crate::service::parse_handshake(&mut connection, &buf[..len]) {
            let span = tracing::span!(tracing::Level::DEBUG, "Request", sni = %sni);
            let _guard = span.enter();

            tracing::info!("Request");

            let service = match services.await.get(&*sni) {
                Some(service) => service.clone(),
                None => {
                    // TODO: Redirect to page for service selection
                    tracing::warn!("Unknown service");
                    return;
                }
            };

            tracing::debug!(%service.addr);

            let down = TcpStream::connect(service.addr).await.unwrap();

            let proxy = service.proxy.clone();
            proxy.run(up, down).await.unwrap();
        } else {
            tracing::warn!("Cannot find SNI");
        }
    }
}
