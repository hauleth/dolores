use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use std::borrow::Cow;

use tokio::io;
use tokio::net::UnixDatagram;
use tokio::sync::RwLock;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Command<'a> {
    Register {
        name: Cow<'a, str>,
        addr: std::net::SocketAddr,
        proxy: crate::proxy::Type,
    },
    Deregister {
        name: Cow<'a, str>,
    },
    Status {
        name: Option<String>,
    },
}

#[derive(Debug)]
pub struct Client {
    socket: UnixDatagram,
}

impl Client {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let id = rand::random::<u64>();
        let mut rx = std::env::temp_dir();
        rx.push(format!("dolores-{:x}-client.sock", id));
        let socket = UnixDatagram::bind(rx.as_path())?;
        match socket.connect(path) {
            Ok(_) => Ok(Client { socket }),
            Err(err) => {
                std::fs::remove_file(rx.as_path())?;
                Err(err.into())
            }
        }
    }

    pub async fn send<'a>(&self, cmd: Command<'a>) -> io::Result<()> {
        self.socket
            .send(&bincode::serialize(&cmd).unwrap())
            .await
            .map(|_| ())
    }

    pub async fn call<'a>(&self, cmd: Command<'a>) -> io::Result<String> {
        self.send(cmd).await?;
        let mut buf = [0; 1024];
        let len = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.socket.recv(&mut buf),
        )
        .await??;
        String::from_utf8(buf[..len].into())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}

impl std::str::FromStr for Client {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Client::open(s)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let addr = self.socket.local_addr().unwrap();
        let path = addr.as_pathname().unwrap();

        std::fs::remove_file(path).unwrap();
    }
}

type RegistryStore = Arc<RwLock<HashMap<String, crate::service::Service>>>;

#[derive(Debug)]
pub struct Registry {
    domain: String,
    socket: UnixDatagram,
    pub services: RegistryStore,
}

impl Registry {
    pub fn open<P: AsRef<Path>>(path: P, domain: &str) -> io::Result<Self> {
        let socket = UnixDatagram::bind(path.as_ref())?;

        Ok(Registry {
            domain: domain.into(),
            socket,
            services: Arc::new(Default::default()),
        })
    }

    pub async fn handle(&self, logger: &slog::Logger) -> io::Result<()> {
        let mut buf = [0; 1024];
        let (len, from) = self.socket.recv_from(&mut buf).await?;
        let cmd = bincode::deserialize::<Command>(&buf[..len]).unwrap();
        debug!(logger, "{:?}", &cmd);
        Self::handle_command(
            self.services.clone(),
            cmd,
            logger,
            &self.socket,
            from.as_pathname().unwrap(),
            &self.domain,
        )
        .await
    }

    async fn handle_command<'a>(
        services: RegistryStore,
        command: Command<'a>,
        logger: &slog::Logger,
        sock: &UnixDatagram,
        to: &std::path::Path,
        domain: &str,
    ) -> std::io::Result<()> {
        use Command::*;

        match command {
            Status { name, .. } => {
                info!(logger, "Status {}", name.as_deref().unwrap_or("(all)"));
                match name {
                    Some(ref name) => {
                        let services = services.read().await;
                        let service = services.get(&*name);
                        sock.send_to(
                            format!("ok {:?}", service.map(|s| &s.domain)).as_bytes(),
                            to,
                        )
                        .await?;
                    }
                    None => {
                        let mut out = String::new();
                        for (name, service) in &*services.read().await {
                            out.push_str(&format!("{} -> {}", name, service.addr));
                        }

                        sock.send_to(out.as_bytes(), to).await?;
                    }
                }
            }
            Register { name, addr, proxy } => {
                info!(logger, "Register {}", name);
                let domain = format!("{}.{}", name, domain);
                services
                    .write()
                    .await
                    .insert(domain, crate::service::Service::new(&name, addr, proxy));
            }
            Deregister { name, .. } => {
                info!(logger, "Deregister {}", name);
                services.write().await.remove(&*name);
            }
        };

        Ok(())
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        let addr = self.socket.local_addr().unwrap();
        let path = addr.as_pathname().unwrap();

        std::fs::remove_file(path).unwrap();
    }
}
