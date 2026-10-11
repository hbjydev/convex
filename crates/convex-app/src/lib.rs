//! Application operations coordinate domain repositories within one unit of work.

pub mod error;
pub mod issues;
#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod unit_of_work;

pub use error::ApplicationError;
pub use issues::{IssueApplication, OperationResult, UpdateIssueResult};
pub use unit_of_work::{UnitOfWork, UnitOfWorkFactory};

pub mod accounts;
pub mod services;
pub mod workspaces;

pub mod control_plane;

pub mod account_workflows;
pub mod workspace_workflows;

#[cfg(test)]
mod test_support;
