use std::io;
use std::net;
use std::os::unix::process::CommandExt;
use std::process;

use nix::sys::socket::{self, socket};
use nix::unistd::{dup2, fork, ForkResult, Pid};

/// Run given command and pass sockets to listen on incoming connections
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    #[structopt(short, long)]
    name: Option<String>,

    #[structopt(long, default_value = "terminating")]
    proxy: crate::proxy::Type,

    #[structopt(name = "PROG")]
    prog_name: String,

    #[structopt(name = "ARGS")]
    prog_args: Vec<String>,
}

const FD_START: i32 = 3;

// TODO: Support more socket types and allow using other socket types, not only TCP
fn open_socket() -> io::Result<net::SocketAddr> {
    let addr = socket::InetAddr::new(socket::IpAddr::new_v6(0, 0, 0, 0, 0, 0, 0, 1), 0);

    let fd = socket(
        socket::AddressFamily::Inet6,
        socket::SockType::Stream,
        socket::SockFlag::empty(),
        None,
    )
    .map_err(|e| e.as_errno().unwrap())?;
    socket::bind(fd, &socket::SockAddr::new_inet(addr)).map_err(|e| e.as_errno().unwrap())?;
    socket::listen(fd, 10).map_err(|e| e.as_errno().unwrap())?;

    dup2(fd, FD_START as i32).map_err(|e| e.as_errno().unwrap())?;

    match socket::getsockname(fd).map_err(|e| e.as_errno().unwrap())? {
        socket::SockAddr::Inet(addr) => Ok(addr.to_std()),
        _ => unreachable!(),
    }
}

impl Command {
    pub(crate) fn run(self, path: &std::path::Path, logger: &slog::Logger) -> anyhow::Result<()> {
        let name = self.name.as_ref().unwrap_or(&self.prog_name);
        let logger = logger.new(o![
            "command" => "run",
            "name" => name.to_owned(),
            "pid" => Pid::this().as_raw(),
        ]);

        debug!(logger, "Starting");

        let addr = open_socket()?;

        match unsafe { fork() }? {
            ForkResult::Child => {
                let error = process::Command::new(&self.prog_name)
                    .args(&self.prog_args)
                    // Use systemd-like interface to pass the sockets to the new process
                    .env("LISTEN_FDS", "1")
                    .env("LISTEN_PID", Pid::this().to_string())
                    .env("LISTEN_FDNAMES", "http")
                    .exec();

                // If we reach that, then `exec` above failed, so we just return error directly
                Err(error.into())
            }
            ForkResult::Parent { child, .. } => {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                let logger = logger.new(o!["child" => child.as_raw()]);

                runtime
                    .block_on(async {
                        use crate::registry;
                        let client = registry::Client::open(path)?;
                        let mut watcher =
                            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child())?;

                        client
                            .send(registry::Command::Register {
                                name: name.into(),
                                addr,
                                proxy: self.proxy,
                            })
                            .await?;

                        debug!(logger, "Registered {}", addr);
                        loop {
                            tokio::select! {
                                _ = tokio::signal::ctrl_c() =>
                                    nix::sys::signal::kill(child, nix::sys::signal::SIGINT)?,
                                _ = watcher.recv() => break,
                            }
                        }
                        debug!(logger, "Shutting down");

                        client
                            .send(registry::Command::Deregister { name: name.into() })
                            .await?;

                        Ok(())
                    })
                    .or_else(|err| {
                        nix::sys::signal::kill(child, nix::sys::signal::SIGTERM)?;
                        Err(err)
                    })
            }
        }
    }
}
