use std::sync::Arc;

use color_eyre::eyre::Result;
use tokio::net::{TcpListener, TcpStream};

/// Start master process listening for connections
#[derive(clap::Parser, Debug)]
pub(crate) struct Command {
    /// TLD that will be used for handling the applications
    #[structopt(short, long, default_value = "localhost")]
    domain: String,

    /// Address which Dolores should listen at
    #[structopt(short, long, default_value = "0.0.0.0:443")]
    listen: std::net::SocketAddr,

    /// Path to the PEM encoded Certificate Authority key
    #[structopt(long, requires("ca-key"))]
    ca_cert: Option<std::path::PathBuf>,

    /// Path to the PEM encoded Certificate Authority private certificate
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

        // Use self signed certificate to make the `rustls` happy (it is not really used right
        // now). In future it may be used for https://localhost or other pages to show list of the
        // currently registered apps, metrics, etc.
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certs = vec![rustls::Certificate(cert.serialize_der().unwrap())];
        let priv_key = rustls::PrivateKey(cert.serialize_private_key_der());

        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, priv_key)
            .expect("Bad certificate/key");

        let config = Arc::new(config);

        let dashboard = Arc::new(crate::dashboard::Server::new(
            registry.services.clone(),
            config.clone(),
        ));

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

                    let handler = handle_request(services, stream, connection, dashboard.clone());

                    tokio::spawn(handler);
                }
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }
}

async fn handle_request(
    services: crate::registry::RegistryStore,
    up: TcpStream,
    mut connection: rustls::ServerConnection,
    dashboard: Arc<crate::dashboard::Server>,
) {
    let mut buf = [0; 1024];
    // Peek into the 1 MiB of the data and try to check if there is SNI information
    let len = up.peek(&mut buf).await.unwrap();
    if let Some(sni) = crate::service::parse_handshake(&mut connection, &buf[..len]) {
        let span = tracing::span!(tracing::Level::DEBUG, "Request", sni = %sni);
        let _guard = span.enter();

        tracing::info!("Request");

        let service = match services.read().await.get(&*sni) {
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
        tracing::info!("Dashboard");
        if let Err(err) = dashboard.handle(up).await {
            tracing::error!(%err);
        }
    }
}
