use std::borrow::Cow;
use std::io;
use std::sync::Arc;

mod tls_terminating;
mod transparent;

pub use tls_terminating::{CertificateAuthority, TlsTerminating};
pub use transparent::Transparent;

#[derive(
    Debug, Clone, Copy, PartialEq, Hash, serde::Serialize, serde::Deserialize, clap::ValueEnum,
)]
pub enum Type {
    Passthrough,
    Terminating,
}

impl Type {
    pub fn build<'a>(
        self,
        domain: impl Into<Domain<'a>>,
        authority: Option<&CertificateAuthority>,
    ) -> Result<Arc<TcpProxy>, rcgen::Error> {
        let domain = domain.into();

        match (self, authority) {
            (Type::Passthrough, _) => Ok(Arc::new(Transparent)),
            (Type::Terminating, Some(authority)) => {
                Ok(Arc::new(TlsTerminating::from_authority(domain, authority)?))
            }
            (Type::Terminating, None) => Ok(Arc::new(TlsTerminating::self_signed(domain))),
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

    async fn run(&self, up: Self::Up, down: Self::Down) -> io::Result<()>;
}

pub type TcpProxy = dyn Proxy<Up = tokio::net::TcpStream, Down = tokio::net::TcpStream>;

#[cfg(test)]
mod tests {
    use super::Domain;

    #[test]
    fn terminating_certificate_covers_service_domain_and_wildcard() {
        let names: Vec<String> = Domain::from("foo.localhost").into();

        assert_eq!(names, ["foo.localhost", "*.foo.localhost"]);
    }
}
