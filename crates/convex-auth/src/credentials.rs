//! Credentials extracted by a transport adapter; no header or cookie parsing here.
use convex_core::{Error, Result};

#[derive(Clone, Default)]
pub struct Credentials {
    pub session: Option<String>,
    pub browser: Option<String>,
    pub method_proof: Option<String>,
}

impl Credentials {
    pub fn session_token(&self) -> Result<&str> {
        self.session.as_deref().ok_or_else(Error::unauthorized)
    }
}

pub struct SessionUpdate<T> {
    pub token: String,
    pub value: T,
    pub clear_method_proof: bool,
}

impl<T> SessionUpdate<T> {
    pub fn new(token: &str, value: T) -> Self {
        Self {
            token: token.into(),
            value,
            clear_method_proof: false,
        }
    }
}

pub struct Authorization {
    pub authorization_url: String,
    pub browser_token: String,
}

pub enum OidcCompletion {
    Login(String),
    Linked(String),
    Proof(String),
}
