//! Reusable Postgres component and configuration.
//!
//! Enable this module with the `postgres` crate feature.

mod config;

pub use config::{
    Config, ConfigBuilder, ConfigBuilderError, DEFAULT_ACQUIRE_TIMEOUT, DEFAULT_IDLE_TIMEOUT,
    DEFAULT_MAX_CONNECTIONS, DEFAULT_MAX_LIFETIME, DEFAULT_MIN_CONNECTIONS,
    DEFAULT_STATEMENT_CACHE_CAPACITY, Password,
};
