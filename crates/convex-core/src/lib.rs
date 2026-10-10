//! Shared operation errors. Domain crates never select HTTP status codes.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("Sign in with valid credentials to continue.")]
    Unauthorized,
    #[error("You do not have permission to perform this action.")]
    Forbidden,
    #[error("This resource does not exist or is not available to you.")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("Too many attempts. Wait 15 minutes, then try again.")]
    RateLimited,
    #[error(transparent)]
    Infrastructure(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn validation(message: &str) -> Self {
        Self::Validation(message.into())
    }
    pub fn unauthorized() -> Self {
        Self::Unauthorized
    }
    pub fn forbidden() -> Self {
        Self::Forbidden
    }
    pub fn not_found() -> Self {
        Self::NotFound
    }
    pub fn conflict(message: &str) -> Self {
        Self::Conflict(message.into())
    }
    pub fn rate_limited() -> Self {
        Self::RateLimited
    }
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Infrastructure(anyhow::anyhow!(error.to_string()))
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        if error.as_database_error().and_then(|e| e.constraint()) == Some("workspaces_slug_key") {
            Self::conflict("This workspace URL is already in use. Choose another slug.")
        } else if error
            .as_database_error()
            .is_some_and(|e| e.is_unique_violation())
        {
            Self::conflict(
                "This email or team key is already in use. For an existing account, sign in before accepting an invitation.",
            )
        } else {
            Self::Infrastructure(error.into())
        }
    }
}

pub mod validation;
