use std::collections::HashMap;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
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
        let perms = Permissions::from_mode(0o777);
        std::fs::set_permissions(&rx, perms)?;
        match socket.connect(path) {
            Ok(_) => Ok(Client { socket }),
            Err(err) => {
                std::fs::remove_file(rx.as_path())?;
                Err(err)
            }
        }
    }

    /// Send message and ignore any response
    pub async fn send(&self, cmd: Command<'_>) -> io::Result<()> {
        self.socket
            .send(&bincode::serialize(&cmd).unwrap())
            .await
            .map(|_| ())
    }

    /// Send message and await for response
    pub async fn call(&self, cmd: Command<'_>) -> io::Result<String> {
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

pub type RegistryStore = Arc<RwLock<HashMap<String, crate::service::Service>>>;

#[derive(Debug)]
pub struct Registry {
    domain: String,
    socket: UnixDatagram,
    pub services: RegistryStore,
}

impl Registry {
    pub fn open<P: AsRef<Path>>(path: P, domain: &str) -> io::Result<Self> {
        let socket = UnixDatagram::bind(&path)?;
        let perms = Permissions::from_mode(0o777);
        std::fs::set_permissions(&path, perms)?;

        Ok(Registry {
            domain: domain.into(),
            socket,
            services: Arc::new(Default::default()),
        })
    }

    pub async fn handle(&self) -> io::Result<()> {
        let mut buf = [0; 1024];
        let (len, from) = self.socket.recv_from(&mut buf).await?;
        let cmd = bincode::deserialize::<Command>(&buf[..len]).unwrap();
        tracing::debug!(?cmd);
        Self::handle_command(
            self.services.clone(),
            cmd,
            &self.socket,
            from.as_pathname().unwrap(),
            &self.domain,
        )
        .await
    }

    async fn handle_command(
        services: RegistryStore,
        command: Command<'_>,
        sock: &UnixDatagram,
        to: &std::path::Path,
        domain: &str,
    ) -> std::io::Result<()> {
        use Command::*;

        match command {
            Status { name, .. } => {
                tracing::info!(name = %name.as_deref().unwrap_or("(all)"), "Status");
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
                let domain = format!("{}.{}", name, domain);
                tracing::info!(%name, %domain, "Register");
                services
                    .write()
                    .await
                    .insert(domain, crate::service::Service::new(&name, addr, proxy));
            }
            Deregister { name, .. } => {
                let domain = format!("{}.{}", name, domain);
                let mut services = services.write().await;
                services.remove(&*domain).unwrap();
                tracing::info!(%name, %domain, "Deregistered");
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
