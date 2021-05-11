#[macro_use]
extern crate slog;
#[macro_use]
extern crate async_trait;

pub mod cli;
pub mod proxy;
pub mod registry;
pub mod service;

pub use registry::Client;
pub use service::Service;
