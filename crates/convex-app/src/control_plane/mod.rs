use crate::{UnitOfWork, UnitOfWorkFactory};
use convex_auth::{repository::RateLimiter, security::SecurityConfig};
use std::sync::Arc;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct ControlPlane {
    pub transactions: Arc<dyn UnitOfWorkFactory>,
    pub security: SecurityConfig,
    pub rate_limiter: Arc<dyn RateLimiter>,
}

/// Publish values only after commit. Failures explicitly roll back; cancellation
/// still drops the PostgreSQL transaction without committing.
pub(crate) async fn finish<T>(
    work: Box<dyn UnitOfWork>,
    result: convex_core::Result<T>,
) -> convex_core::Result<T> {
    match result {
        Ok(value) => {
            work.commit().await?;
            Ok(value)
        }
        Err(error) => {
            if let Err(rollback_error) = work.rollback().await {
                tracing::error!(%rollback_error, "Transaction rollback failed");
            }
            Err(error)
        }
    }
}
