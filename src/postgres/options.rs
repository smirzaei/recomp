use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::Config;

impl From<&Config> for PgConnectOptions {
    fn from(config: &Config) -> Self {
        let mut options = Self::new();

        if let Some(host) = config.host() {
            options = options.host(host);
        }

        if let Some(port) = config.port() {
            options = options.port(port);
        }

        if let Some(username) = config.username() {
            options = options.username(username);
        }

        if let Some(password) = config.password() {
            options = options.password(password.as_str());
        }

        if let Some(database) = config.database() {
            options = options.database(database);
        }

        if let Some(application_name) = config.application_name() {
            options = options.application_name(application_name);
        }

        if let Some(capacity) = config.statement_cache_capacity() {
            options = options.statement_cache_capacity(capacity);
        }

        options
    }
}

impl From<&Config> for PgPoolOptions {
    fn from(config: &Config) -> Self {
        Self::new()
            .acquire_timeout(config.acquire_timeout())
            .idle_timeout(config.idle_timeout())
            .max_lifetime(config.max_lifetime())
            .max_connections(config.max_connections().get())
            .min_connections(config.min_connections())
    }
}
