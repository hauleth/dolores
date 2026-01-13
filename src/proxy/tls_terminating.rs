use std::io;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{
    rustls,
    TlsAcceptor
};

/// TLS terminating proxy
///
/// This proxy will terminate TLS on the boundary and will pass raw TCP communication downstream.
/// It supports:
///
/// - Self-signed certificates generated on demand
/// - Generated certificates that are signed by the given CA (WIP)
/// - Passed certificate (TODO)
#[derive(Clone)]
pub struct TlsTerminating {
    acceptor: TlsAcceptor,
}

impl TlsTerminating {
    pub fn self_signed(domain: super::Domain) -> Self {
        let cert = rcgen::generate_simple_self_signed(domain).unwrap();
        let certs = vec![cert.cert.der().clone()];
        let pk_der: rustls::pki_types::PrivatePkcs8KeyDer = cert.signing_key.serialize_der().into();
        let priv_key = pk_der.into();

        Self::build(certs, priv_key)
    }

    //pub fn from_ca(domain: super::Domain, ca_cert: &rcgen::Certificate) -> Self {
    //    let keypair = rcgen::KeyPair::generate();
    //    let params = rcgen::CertificateParams::new(domain).unwrap()
    //        .signed_by(&keypair, &ca_cert.cert, &ca_cert.key_pair)
    //        .unwrap();
    //    let cert = rcgen::generate_simple_self_signed(domain).unwrap();
    //    let certs = vec![
    //        cert.serialize_der_with_signer(ca_cert).unwrap().into(),
    //    ];
    //    let pk_der: rustls::pki_types::PrivatePkcs8KeyDer = cert.serialize_private_key_der().into();
    //    let priv_key = pk_der.into();
    //
    //    Self::build(certs, priv_key)
    //}

    fn build(certs: Vec<rustls::pki_types::CertificateDer<'static>>, priv_key: rustls::pki_types::PrivateKeyDer<'static>) -> Self {
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, priv_key)
            .expect("Bad certificate/key");

        let acceptor = TlsAcceptor::from(Arc::new(config));

        TlsTerminating { acceptor }
    }
}

#[async_trait]
impl super::Proxy for TlsTerminating {
    type Up = tokio::net::TcpStream;
    type Down = tokio::net::TcpStream;

    async fn run(&self, up: Self::Up, mut down: Self::Down) -> io::Result<()> {
        tracing::debug!("Proxy started");
        let up_addr = up.local_addr().unwrap();
        let down_addr = down.peer_addr().unwrap();
        let mut up_buf = [0; 4 * 1024];
        let mut down_buf = [0; 4 * 1024];
        let mut up = self.acceptor.accept(up).await?;

        loop {
            // Read from any connection and write to the another one
            let finished = tokio::select! {
                result = up.read(&mut up_buf) => {
                    tracing::trace!("{} -> {}", up_addr, down_addr);
                    copy(result, &up_buf, &mut down).await?
                }
                result = down.read(&mut down_buf) => {
                    tracing::trace!("{} <- {}", up_addr, down_addr);
                    copy(result, &down_buf, &mut up).await?
                }
            };

            if finished {
                return Ok(());
            }
        }
    }
}

async fn copy(
    result: io::Result<usize>,
    buf: &[u8],
    out: &mut (impl AsyncWriteExt + Unpin),
) -> io::Result<bool> {
    match result {
        Ok(0) => {
            tracing::trace!("EOF");
            Ok(true)
        }
        Ok(len) => {
            let data = std::str::from_utf8(&buf[..len]);
            tracing::trace!(?data, "Received");
            out.write(&buf[..len]).await?;

            Ok(false)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            tracing::trace!("Would block");
            Ok(false)
        }
        Err(err) => {
            tracing::error!(?err, "Error");
            Err(err)
        }
    }
}

impl std::fmt::Debug for TlsTerminating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str("TlsTerminating")
    }
}
