use std::borrow::Cow;
use std::io;
use std::sync::Arc;

mod tls_terminating;
mod transparent;

pub use tls_terminating::TlsTerminating;
pub use transparent::Transparent;

#[derive(
    Debug, Clone, Copy, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::EnumString,
)]
#[strum(serialize_all = "snake_case")]
pub enum Type {
    Passthrough,
    Terminating,
}

impl Type {
    pub fn build<'a>(self, domain: impl Into<Domain<'a>>) -> Arc<TcpProxy> {
        match self {
            Type::Passthrough => Arc::new(Transparent),
            Type::Terminating => Arc::new(TlsTerminating::self_signed(domain.into())),
        }
    }
}

pub struct Domain<'a>(Cow<'a, str>);

impl<'a, S> From<S> for Domain<'a>
where
    S: Into<Cow<'a, str>>,
{
    fn from(input: S) -> Self {
        Domain(input.into())
    }
}

impl From<Domain<'_>> for Vec<String> {
    fn from(Domain(ref domain): Domain) -> Vec<String> {
        vec![domain.to_string(), format!("*.{}", domain)]
    }
}

#[async_trait]
pub trait Proxy: Send + Sync + core::fmt::Debug {
    type Up;
    type Down;

    async fn run(&self, up: Self::Up, down: Self::Down, logger: &slog::Logger) -> io::Result<()>;
}

pub type TcpProxy = dyn Proxy<Up = tokio::net::TcpStream, Down = tokio::net::TcpStream>;
