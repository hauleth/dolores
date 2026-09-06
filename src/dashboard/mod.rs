use color_eyre::eyre::Result;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as Http,
};

use tokio_rustls::{rustls, TlsAcceptor};

use std::sync::Arc;

use crate::registry::RegistryStore;

mod handlers;

#[async_trait]
trait Handler: Send + Sync {
    async fn handle(
        self: Arc<Self>,
        req: Request<Incoming>,
        ctx: Context,
    ) -> Result<Response<String>>;
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
        let acceptor = TlsAcceptor::from(tls_config);
        let mut router = matchit::Router::<Arc<dyn Handler>>::new();

        router.insert("/", Arc::new(handlers::Home)).unwrap();
        router
            .insert("/health", Arc::new(handlers::Health))
            .unwrap();

        Server {
            acceptor,
            registry,
            router: Arc::new(router),
        }
    }

    pub async fn handle(&self, stream: tokio::net::TcpStream) -> std::io::Result<()> {
        let tls_stream = self.acceptor.accept(stream).await?;
        let io_stream = TokioIo::new(tls_stream);

        let service_fn = service_fn(move |req| {
            let req = add_host(req);
            tracing::info!(?req, "Incoming request");
            let ctx = Context {
                registry: self.registry.clone(),
            };
            let router = self.router.clone();
            let route = router.at(req.uri().path()).unwrap();
            Handler::handle(route.value.clone(), req, ctx)
        });

        if let Err(http_err) = Http::new(TokioExecutor::new())
            .serve_connection(io_stream, service_fn)
            .await
        {
            tracing::error!("Error while serving HTTP connection: {}", http_err);
        }

        Ok(())
    }
}

/// Add details to URI from `Host` header
fn add_host(mut req: Request<Incoming>) -> Request<Incoming> {
    let host = req.headers().get("host").cloned();

    tracing::info!(?host);

    let uri = req.uri_mut();
    let mut parts = uri.clone().into_parts();
    // We know that we are handling HTTPS connection
    parts.scheme = Some(hyper::http::uri::Scheme::HTTPS);
    parts.authority =
        host.and_then(|host| hyper::http::uri::Authority::from_maybe_shared(host).ok());

    *uri = hyper::Uri::from_parts(parts).unwrap();

    req
}
