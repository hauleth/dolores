use std::net;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Service {
    pub domain: String,
    pub addr: net::SocketAddr,
    pub proxy: Arc<crate::proxy::TcpProxy>,
}

impl Service {
    pub fn new(domain: &str, addr: net::SocketAddr, proxy: crate::proxy::Type) -> Self {
        Service {
            domain: domain.into(),
            addr,
            proxy: proxy.build(domain),
        }
    }
}

pub fn parse_handshake(connection: &mut rustls::ServerConnection, mut data: &[u8]) -> Option<String> {
    connection.read_tls(&mut data).ok()?;
    let _ = connection.process_new_packets();
    connection.sni_hostname().and_then(|sni| {
        let mut parts = sni.split('.');
        let tld = parts.nth_back(0)?;
        let name = parts.nth_back(0)?;
        Some(format!("{}.{}", name, tld))
    })
}

impl net::ToSocketAddrs for Service {
    type Iter = std::option::IntoIter<net::SocketAddr>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        Ok(Some(self.addr).into_iter())
    }
}
