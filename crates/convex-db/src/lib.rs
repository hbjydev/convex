//! PostgreSQL setup and transaction storage. Domain repositories supply the queries.

#[cfg(feature = "postgres")]
pub mod postgres;
