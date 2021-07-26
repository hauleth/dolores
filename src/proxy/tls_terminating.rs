use std::io;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;

#[derive(Clone)]
pub struct TlsTerminating {
    acceptor: TlsAcceptor,
}

impl TlsTerminating {
    pub fn self_signed(domain: super::Domain) -> Self {
        let cert = rcgen::generate_simple_self_signed(domain).unwrap();
        let mut config = rustls::ServerConfig::new(Arc::new(rustls::NoClientAuth));

        let certs = vec![rustls::Certificate(cert.serialize_der().unwrap())];
        let priv_key = rustls::PrivateKey(cert.serialize_private_key_der());

        config.set_single_cert(certs, priv_key).unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(config));

        TlsTerminating { acceptor }
    }
}

#[async_trait]
impl super::Proxy for TlsTerminating {
    type Up = tokio::net::TcpStream;
    type Down = tokio::net::TcpStream;

    async fn run(
        &self,
        up: Self::Up,
        mut down: Self::Down,
    ) -> io::Result<()> {
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
