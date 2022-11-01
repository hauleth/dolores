use color_eyre::eyre::Result;
use hyper::{Body, Request, Response, StatusCode};
use askama::Template;

use std::sync::Arc;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Home;

#[derive(Template)]
#[allow(dead_code)]
#[template(path = "hello.html")]
struct HomeTemplate<'a> {
    req: Request<Body>,
    registry: &'a HashMap<String, crate::service::Service>
}

#[async_trait]
impl super::Handler for Home {
    async fn handle(
        self: Arc<Self>,
        req: Request<Body>,
        ctx: super::Context,
    ) -> Result<Response<Body>> {
        let registry = ctx.registry.read().await;

        let view = HomeTemplate { req, registry: &*registry };

        Ok(Response::builder()
            .header("content-type", "text/html")
            .body(Body::from(view.render()?))?)
    }
}

pub struct Health;

#[async_trait]
impl super::Handler for Health {
    async fn handle(
        self: Arc<Self>,
        _req: Request<Body>,
        _ctx: super::Context,
    ) -> Result<Response<Body>> {
        Ok(Response::new(Body::from("Ok\n")))
    }
}

mod filters {
    #![allow(dead_code)]

    use hyper::Request;

    pub fn debug(val: impl std::fmt::Debug) -> askama::Result<String> {
        Ok(format!("{val:?}"))
    }

    pub fn domain_url<B>(domain: &str, req: &Request<B>) -> askama::Result<String> {
        let port = match req.uri().port_u16() {
            Some(p) if p != 443 => format!(":{p}"),
            _ => "".into()
        };
        Ok(format!("https://{domain}{port}"))
    }
}
