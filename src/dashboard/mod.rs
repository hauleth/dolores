use color_eyre::eyre::Result;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, StatusCode};

use std::sync::Arc;
use std::collections::HashMap;

use crate::registry::RegistryStore;

mod handlers;

#[async_trait]
trait Handler: Send + Sync {
    async fn handle(self: Arc<Self>, req: Request<Body>, ctx: Context) -> Result<Response<Body>>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Data {
    registry: HashMap<String, crate::service::Service>
}

#[derive(Clone)]
pub struct Context {
    registry: RegistryStore,
}

pub struct Server {
    acceptor: tokio_rustls::TlsAcceptor,
    router: Arc<matchit::Router<Arc<dyn Handler>>>,
    registry: RegistryStore,
}

impl Server {
    pub fn new(registry: RegistryStore, tls_config: Arc<rustls::ServerConfig>) -> Self {
        let acceptor = tokio_rustls::TlsAcceptor::from(tls_config);
        let mut router = matchit::Router::<Arc<dyn Handler>>::new();

        router.insert("/", Arc::new(handlers::Home)).unwrap();
        router.insert("/health", Arc::new(handlers::Health)).unwrap();

        Server { acceptor, registry, router: Arc::new(router) }
    }

    pub async fn handle(&self, stream: tokio::net::TcpStream) -> std::io::Result<()> {
        let tls_stream = self.acceptor.accept(stream).await?;

        let service_fn = service_fn(move |req| {
            let req = add_host(req);
            tracing::info!(?req);
            let ctx = Context {
                registry: self.registry.clone(),
            };
            let router = self.router.clone();
            let route = router.at(req.uri().path()).unwrap();
            Handler::handle(route.value.clone(), req, ctx)
        });

        if let Err(http_err) = Http::new()
            .serve_connection(tls_stream, service_fn)
            .await
        {
            tracing::error!("Error while serving HTTP connection: {}", http_err);
        }

        Ok(())
    }
}

/// Add details to URI from `Host` header
fn add_host(mut req: Request<Body>) -> Request<Body> {
    let host = req.headers().get("host").cloned();

    tracing::info!(?host);

    let uri = req.uri_mut();
    let mut parts = uri.clone().into_parts();
    // We know that we are handling HTTPS connection
    parts.scheme = Some(http::uri::Scheme::HTTPS);
    parts.authority = host.and_then(|host| http::uri::Authority::from_maybe_shared(host).ok());

    *uri = hyper::Uri::from_parts(parts).unwrap();

    req
}
