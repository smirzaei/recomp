//! Reusable Postgres component and configuration.
//!
//! Enable this module with the `postgres` crate feature.

mod component;
mod config;
mod options;

pub use ::sqlx::postgres::PgPool;
pub use component::{Postgres, PostgresAccessError, PostgresHealthError, PostgresRunError};
pub use config::{
    Config, ConfigBuilder, ConfigBuilderError, DEFAULT_ACQUIRE_TIMEOUT, DEFAULT_IDLE_TIMEOUT,
    DEFAULT_MAX_CONNECTIONS, DEFAULT_MAX_LIFETIME, DEFAULT_MIN_CONNECTIONS, Password,
};
