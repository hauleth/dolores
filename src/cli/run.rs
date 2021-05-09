use std::os::unix::process::CommandExt;
use std::process;

use nix::sys::socket::{self, socket};
use nix::unistd::{dup2,getpid};

/// Run given command and pass sockets to listen on incoming connections
#[derive(structopt::StructOpt, Debug)]
pub(crate) struct Command {
    #[structopt(short, long)]
    name: Option<String>,

    #[structopt(long, default_value = "https")]
    fd_names: Vec<String>,

    #[structopt(name = "PROG")]
    prog_name: String,

    #[structopt(name = "ARGS")]
    prog_args: Vec<String>,
}

const FD_START: i32 = 3;

fn open_sockets<'a>(ports: &[impl AsRef<str>]) -> anyhow::Result<()> {
    let addr = socket::InetAddr::new(socket::IpAddr::new_v6(0, 0, 0, 0, 0, 0, 0, 1), 0);

    for (n, name) in ports.into_iter().enumerate() {
        let fd = socket(
            socket::AddressFamily::Inet6,
            socket::SockType::Stream,
            socket::SockFlag::empty(),
            None,
        )
        .expect("Cannot open socket");
        socket::bind(fd, &socket::SockAddr::new_inet(addr)).expect("Cannot bind");
        socket::listen(fd, 10).expect("Cannot listen");

        println!("{}: {}", name.as_ref(), socket::getsockname(fd)?.to_str());

        dup2(fd, FD_START + n as i32).expect("Cannot duplicate FD");
    }

    Ok(())
}

impl Command {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let name = self.name.as_ref().unwrap_or(&self.prog_name);
        let ports = &self.fd_names;
        let fd_count = ports.len();

        open_sockets(ports)?;

        println!("Starting {}", name);

        let error = process::Command::new(&self.prog_name)
            .args(&self.prog_args)
            // Use systemd-like interface to pass the sockets to the new process
            .env("LISTEN_FDS", format!("{}", fd_count))
            .env("LISTEN_PID", format!("{}", getpid()))
            .env("LISTEN_FDNAMES", ports.join(":"))
            .exec();

        // If we reach that, then `exec` above failed, so we just return error directly
        Err(error)?
    }
}
