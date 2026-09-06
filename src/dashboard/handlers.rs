use askama::Template;
use color_eyre::eyre::Result;
use hyper::{body::Incoming, Request, Response};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Home;

#[derive(Template)]
#[allow(dead_code)]
#[template(path = "hello.html")]
struct HomeTemplate<'a> {
    req: Request<Incoming>,
    registry: &'a HashMap<String, crate::service::Service>,
}

#[async_trait]
impl super::Handler for Home {
    async fn handle(
        self: Arc<Self>,
        req: Request<Incoming>,
        ctx: super::Context,
    ) -> Result<Response<String>> {
        let registry = ctx.registry.read().await;

        let view = HomeTemplate {
            req,
            registry: &registry,
        };

        Ok(Response::builder()
            .header("content-type", "text/html")
            .body(view.render()?)?)
    }
}

pub struct Health;

#[async_trait]
impl super::Handler for Health {
    async fn handle(
        self: Arc<Self>,
        _req: Request<Incoming>,
        _ctx: super::Context,
    ) -> Result<Response<String>> {
        Ok(Response::builder().body("Ok\n".into())?)
    }
}

mod filters {
    #![allow(dead_code)]

    use hyper::Request;

    use std::borrow::Cow;

    pub fn debug(val: impl std::fmt::Debug) -> askama::Result<String> {
        Ok(format!("{val:?}"))
    }

    #[askama::filter_fn]
    pub fn domain_url<B>(
        domain: &str,
        _env: &dyn askama::Values,
        req: &Request<B>,
    ) -> askama::Result<String> {
        let port: Cow<str> = match req.uri().port_u16() {
            Some(p) if p != 443 => format!(":{p}").into(),
            _ => "".into(),
        };
        Ok(format!("https://{domain}{port}"))
    }
}
