use std::sync::Arc;

use color_eyre::eyre::{eyre, Result, WrapErr};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls;

/// Start master process listening for connections
#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    /// TLD that will be used for handling the applications
    #[arg(short, long, default_value = "localhost")]
    domain: String,

    /// Address which Dolores should listen at
    #[arg(short, long, default_value = "0.0.0.0:443")]
    listen: std::net::SocketAddr,

    /// Path to the PEM-encoded Certificate Authority certificate
    #[arg(long, requires("ca_key"))]
    ca_cert: Option<std::path::PathBuf>,

    /// Path to the PEM-encoded Certificate Authority private key
    #[arg(long, requires("ca_cert"))]
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

    fn certificate_authority(&self) -> Result<Option<Arc<crate::proxy::CertificateAuthority>>> {
        match (&self.ca_cert, &self.ca_key) {
            (None, None) => Ok(None),
            (Some(certificate_path), Some(private_key_path)) => {
                let certificate =
                    std::fs::read_to_string(certificate_path).wrap_err_with(|| {
                        format!(
                            "cannot read certificate authority certificate from {}",
                            certificate_path.display()
                        )
                    })?;
                let private_key =
                    std::fs::read_to_string(private_key_path).wrap_err_with(|| {
                        format!(
                            "cannot read certificate authority private key from {}",
                            private_key_path.display()
                        )
                    })?;
                let authority =
                    crate::proxy::CertificateAuthority::from_pem(&certificate, &private_key)
                        .wrap_err("cannot load certificate authority")?;

                Ok(Some(Arc::new(authority)))
            }
            _ => Err(eyre!("--ca-cert and --ca-key must be used together")),
        }
    }

    async fn serve(&self, path: &std::path::Path) -> Result<()> {
        let certificate_authority = self.certificate_authority()?;
        let listener = TcpListener::bind(self.listen).await?;
        let registry =
            crate::registry::Registry::open(path, &self.domain, certificate_authority.clone())?;

        let config = match certificate_authority.as_deref() {
            Some(authority) => authority
                .server_config("localhost")
                .wrap_err("cannot create dashboard certificate")?,
            None => {
                let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
                let certificates = vec![certificate.cert.der().clone()];
                let private_key: rustls::pki_types::PrivatePkcs8KeyDer =
                    certificate.signing_key.serialize_der().into();

                rustls::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certificates, private_key.into())?
            }
        };

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
                result = registry.handle() => result?,
                // Frontend socket
                result = listener.accept() => {
                    let (stream, addr) = result?;

                    let span = tracing::span!(tracing::Level::DEBUG, "Connection", addr = %addr);
                    let _guard = span.enter();

                    let connection = rustls::ServerConnection::new(config.clone())?;
                    let services = registry.services.clone();
                    let dashboard = dashboard.clone();

                    tokio::spawn(async move {
                        if let Err(error) =
                            handle_request(services, stream, connection, dashboard, addr).await
                        {
                            tracing::error!(peer = %addr, error = ?error, "Request failed");
                        }
                    });
                }
                result = tokio::signal::ctrl_c() => {
                    result?;
                    return Ok(());
                },
            }
        }
    }
}

async fn handle_request(
    services: crate::registry::RegistryStore,
    up: TcpStream,
    mut connection: rustls::ServerConnection,
    dashboard: Arc<crate::dashboard::Server>,
    peer_addr: std::net::SocketAddr,
) -> Result<()> {
    let mut buf = [0; 1024];
    // Peek into the 1 MiB of the data and try to check if there is SNI information
    let len = up
        .peek(&mut buf)
        .await
        .wrap_err_with(|| format!("cannot inspect request from {peer_addr}"))?;
    if buf.starts_with(&b"GET "[..]) {
        tracing::error!(peer = %peer_addr, "HTTP request, HTTPS expected");
    } else if let Some(sni) = crate::service::parse_handshake(&mut connection, &buf[..len]) {
        tracing::info!(peer = %peer_addr, %sni, "Request");

        let service = match services.read().await.get(&*sni) {
            Some(service) => service.clone(),
            None => {
                // TODO: Redirect to page for service selection
                tracing::warn!(peer = %peer_addr, %sni, "Unknown service");
                return Ok(());
            }
        };

        tracing::debug!(peer = %peer_addr, %sni, backend = %service.addr);

        let down = TcpStream::connect(service.addr).await.wrap_err_with(|| {
            format!(
                "cannot connect request from {peer_addr} for {sni} to {}",
                service.addr
            )
        })?;

        let proxy = service.proxy.clone();
        proxy.run(up, down).await.wrap_err_with(|| {
            format!(
                "cannot proxy request from {peer_addr} for {sni} to {}",
                service.addr
            )
        })?;
    } else {
        tracing::info!(peer = %peer_addr, "Dashboard");
        dashboard
            .handle(up)
            .await
            .wrap_err_with(|| format!("cannot serve dashboard request from {peer_addr}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{handle_request, Command};
    use crate::proxy::Type;
    use crate::registry::RegistryStore;
    use crate::service::Service;
    use std::sync::Arc;
    use tokio::io::AsyncWriteExt;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::RwLock;
    use tokio_rustls::rustls;

    async fn tcp_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let connection = TcpStream::connect(listener.local_addr().unwrap());
        let accepted = listener.accept();
        let (connection, accepted) = tokio::join!(connection, accepted);

        (accepted.unwrap().0, connection.unwrap())
    }

    fn server_config() -> Arc<rustls::ServerConfig> {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificates = vec![certificate.cert.der().clone()];
        let private_key: rustls::pki_types::PrivatePkcs8KeyDer =
            certificate.signing_key.serialize_der().into();

        Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certificates, private_key.into())
                .unwrap(),
        )
    }

    fn client_hello(server_name: &str) -> Vec<u8> {
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        let server_name = rustls::pki_types::ServerName::try_from(server_name.to_owned()).unwrap();
        let mut connection = rustls::ClientConnection::new(Arc::new(config), server_name).unwrap();
        let mut output = Vec::new();
        connection.write_tls(&mut output).unwrap();
        output
    }

    #[test]
    fn loads_configured_certificate_authority() {
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
        let signing_key = rcgen::KeyPair::generate().unwrap();
        let certificate = params.self_signed(&signing_key).unwrap();
        let directory =
            std::env::temp_dir().join(format!("dolores-ca-{:x}", rand::random::<u64>()));
        let certificate_path = directory.with_extension("crt");
        let private_key_path = directory.with_extension("key");
        std::fs::write(&certificate_path, certificate.pem()).unwrap();
        std::fs::write(&private_key_path, signing_key.serialize_pem()).unwrap();

        let command = Command {
            domain: "localhost".into(),
            listen: "127.0.0.1:443".parse().unwrap(),
            ca_cert: Some(certificate_path.clone()),
            ca_key: Some(private_key_path.clone()),
        };

        assert!(command.certificate_authority().unwrap().is_some());

        std::fs::remove_file(certificate_path).unwrap();
        std::fs::remove_file(private_key_path).unwrap();
    }

    #[tokio::test]
    async fn returns_backend_connection_errors_with_request_context() {
        let unavailable = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = unavailable.local_addr().unwrap();
        drop(unavailable);

        let services: RegistryStore = Arc::new(RwLock::new(Default::default()));
        services.write().await.insert(
            "foo.localhost".to_owned(),
            Service::new("foo.localhost", backend_addr, Type::Passthrough, None).unwrap(),
        );

        let config = server_config();
        let dashboard = Arc::new(crate::dashboard::Server::new(
            services.clone(),
            config.clone(),
        ));
        let connection = rustls::ServerConnection::new(config).unwrap();
        let (up, mut peer) = tcp_pair().await;
        let peer_addr = up.peer_addr().unwrap();
        peer.write_all(&client_hello("foo.localhost"))
            .await
            .unwrap();

        let error = handle_request(services, up, connection, dashboard, peer_addr)
            .await
            .unwrap_err();
        let message = error.to_string();

        assert!(message.contains(&peer_addr.to_string()));
        assert!(message.contains("foo.localhost"));
        assert!(message.contains(&backend_addr.to_string()));
    }
}
