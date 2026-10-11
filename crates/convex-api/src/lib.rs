//! HTTP adapters, request/response DTOs, cookies, and the public OpenAPI contract.
pub mod accounts;
#[cfg(feature = "embed-web")]
mod frontend;
pub mod http;
pub mod issues;
pub mod methods;
pub mod oidc;
pub mod security;
pub mod workspaces;

pub use http::{app, openapi, router};

#[cfg(test)]
mod test_support;
